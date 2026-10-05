//! Isolated original pose sampler for normal Sonic and the bounce model.
//! Character-side attachment/effect context is intentionally not simulated here.
use serde::Deserialize;
use sa2_runtime::{animated_vertex_host::{self,Input,Output},original_math::OriginalMath,disc::digest};
#[derive(Deserialize)]struct Audit {base:u32,memory_sha256:String,bundle:Bundle}
#[derive(Deserialize)]struct Bundle {entries:Vec<Entry>}
#[derive(Deserialize)]struct Entry {index:u16,motion:u16,frames:Option<u32>,model:u32,model_pointer:u32,motion_pointer:u32}
pub struct SourcePose {base:u32,asset:Vec<u8>,entries:Vec<Entry>,math:std::sync::Arc<OriginalMath>}
pub struct PreparedTransition{base:u32,math:std::sync::Arc<OriginalMath>,memory:Vec<u8>,root:u32,motion:u32}
impl PreparedTransition{
 pub fn sample(&self,blend:f32,action:u8,lean:u32)->Result<Output,String>{
  if !blend.is_finite() || !(0.0..=1.0).contains(&blend){return Err("Invalid transition fraction".into());}
  let mut m=self.memory.clone();let base=self.base;let work=base+0x200000;
  m[0x20012c..0x200130].copy_from_slice(&lean.to_be_bytes());m[0x202000]=action;
  animated_vertex_host::sample(&mut m,&Input{base,root:self.root,motion:self.motion,frame:blend,models:base+0x1240,character:work+0x100,scratch:work+0x1000,players:work+0x1800,stack:work+0x4000,vertices:work+0x10000,vertex_bytes:0x4000},&self.math).map_err(|e|e.to_string())
 }
}
impl SourcePose {
 pub fn sample_transition(&self,old:u16,old_frame:f32,new:u16,new_frame:f32,blend:f32,action:u8,lean:u32)->Result<Output,String>{
  self.prepare_transition(old,old_frame,new,new_frame)?.sample(blend,action,lean)
 }
 pub fn prepare_transition(&self,old:u16,old_frame:f32,new:u16,new_frame:f32)->Result<PreparedTransition,String>{
  use sa2_runtime::{original_heap::{self,HeapContext},original_animation_transition};
  let entry=|id|self.entries.iter().find(|e|e.index==id).ok_or("Unknown transition animation");
  let a=entry(old)?;let b=entry(new)?;
  if a.model!=b.model || !matches!(b.model,0|6){return Err("Unsupported transition pair or fraction".into());}
  for (e,f) in [(a,old_frame),(b,new_frame)] {if e.motion_pointer==0 || !f.is_finite() || f<0. || f>=e.frames.unwrap_or(0)as f32{return Err("Invalid transition frame".into());}}
  let mut m=self.asset.clone();m.resize(0x226000,0);let base=self.base;let work=base+0x200000;
  let put=|m:&mut[u8],o:usize,v:u32|m[o..o+4].copy_from_slice(&v.to_be_bytes());
  for offset in [0x20103c,0x204000] {for (i,v) in [1f32,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.].iter().enumerate(){put(&mut m,offset+i*4,v.to_bits());}}
  put(&mut m,0x201800,work+0x2000);put(&mut m,0x201804,work+0x2040);put(&mut m,0x20012c,0);m[0x202000]=0;
  for e in [a,b] {let at=0x214000+e.index as usize*16;m[at..at+2].copy_from_slice(&e.motion.to_be_bytes());m[at+2..at+4].copy_from_slice(&(e.model as u16).to_be_bytes());}
  m[0x215002..0x215004].copy_from_slice(&new.to_be_bytes());m[0x215004..0x215006].copy_from_slice(&old.to_be_bytes());
  put(&mut m,0x215010,old_frame.to_bits());put(&mut m,0x215018,new_frame.to_bits());put(&mut m,0x215024,base+0x214000);
  put(&mut m,0x215100,u32::MAX);let heap=HeapContext{table:base+0x215100,count:1};
  let index=original_heap::create(base,&mut m,&heap,base+0x216000,base+0x226000).map_err(|e|e.to_string())?;
  let header=original_heap::allocate(base,&mut m,&heap,index,12).map_err(|e|e.to_string())?;
  let data=original_heap::allocate(base,&mut m,&heap,index,111*24).map_err(|e|e.to_string())?;
  if header==0 || data==0{return Err("Transition heap exhausted".into());}
  put(&mut m,(header-base)as usize,data);put(&mut m,(header-base)as usize+4,2);put(&mut m,0x215028,header);
  original_animation_transition::build(base+0x215000,base,&mut m,base+0x800,&heap,index).map_err(|e|e.to_string())?;
  Ok(PreparedTransition{base:self.base,math:self.math.clone(),memory:m,root:b.model_pointer,motion:header})
 }
 pub fn model_and_last_frame(&self,id:u16)->Result<(u32,f32),String>{let e=self.entries.iter().find(|e|e.index==id).ok_or("Unknown source animation")?;Ok((e.model,e.frames.ok_or("Missing source motion")?.saturating_sub(1)as f32))}
 pub fn new(audit:&[u8],asset:Vec<u8>,dol:Vec<u8>)->Result<Self,String>{
  Self::with_math(audit,asset,OriginalMath::from_dol(dol).map_err(|e|e.to_string())?)
 }
 /// Portable runtime input: extracted constants and tables, no game executable.
 pub fn from_math_bundle(audit:&[u8],asset:Vec<u8>,math:Vec<u8>)->Result<Self,String>{
  Self::with_math(audit,asset,OriginalMath::from_data_bundle(math).map_err(|e|e.to_string())?)
 }
 fn with_math(audit:&[u8],asset:Vec<u8>,math:OriginalMath)->Result<Self,String>{
  let audit:Audit=serde_json::from_slice(audit).map_err(|e|e.to_string())?;
  if asset.len()!=0x200000 || digest(&asset)!=audit.memory_sha256 || audit.base.checked_add(0x226000).is_none(){return Err("Invalid source pose bundle".into());}
  let mut ids=std::collections::BTreeSet::new();
  for entry in &audit.bundle.entries {
   if entry.index>=203 || !ids.insert(entry.index) {return Err("Duplicate source animation ID".into());}
   if entry.motion_pointer!=0 && (entry.frames.unwrap_or(0)==0 || entry.motion_pointer<audit.base || entry.motion_pointer>=audit.base+0x200000 || entry.model_pointer<audit.base || entry.model_pointer>=audit.base+0x200000){return Err("Invalid source animation entry".into());}
  }
  Ok(Self{base:audit.base,asset,entries:audit.bundle.entries,math:std::sync::Arc::new(math)})
 }
 pub fn sample(&self,animation:u16,frame:f32,action:u8,lean:u32)->Result<Output,String>{
  let entry=self.entries.iter().find(|e|e.index==animation).ok_or("Unknown source animation")?;
  if !matches!(entry.model,0|6) || entry.motion_pointer==0 || !frame.is_finite() || frame<0. || frame>=entry.frames.unwrap_or(0)as f32 {return Err("Unsupported source pose request".into());}
  self.sample_entry(entry,frame,action,lean)
 }
 /// Original display override retains the active animation's frame, even when
 /// it exceeds the substituted motion's nominal period. It does not wrap it.
 pub fn sample_spin_override(&self,active:u16,frame:f32,action:u8,lean:u32)->Result<Output,String>{
  let active=self.entries.iter().find(|e|e.index==active).ok_or("Unknown active animation")?;
  if active.motion_pointer==0 || !frame.is_finite() || frame<0. || frame>=active.frames.unwrap_or(0)as f32{return Err("Invalid active animation frame".into());}
  let entry=self.entries.iter().find(|e|e.index==30).ok_or("Missing spin override")?;
  if entry.model!=6 || entry.motion_pointer==0{return Err("Invalid spin override".into());}
  self.sample_entry(entry,frame,action,lean)
 }
 fn sample_entry(&self,entry:&Entry,frame:f32,action:u8,lean:u32)->Result<Output,String>{
  let mut memory=self.asset.clone();memory.resize(0x214000,0);let work=self.base+0x200000;
  let mut word=|offset:usize,value:u32|memory[offset..offset+4].copy_from_slice(&value.to_be_bytes());
  for offset in [0x20103c,0x204000] {for (i,value) in [1f32,0.,0.,0.,0.,1.,0.,0.,0.,0.,1.,0.].iter().enumerate(){word(offset+i*4,value.to_bits());}}
  word(0x201800,work+0x2000);word(0x201804,work+0x2040);word(0x20012c,lean);
  memory[0x202000]=action;
  animated_vertex_host::sample(&mut memory,&Input{base:self.base,root:entry.model_pointer,motion:entry.motion_pointer,frame,models:self.base+0x1240,character:work+0x100,scratch:work+0x1000,players:work+0x1800,stack:work+0x4000,vertices:work+0x10000,vertex_bytes:0x4000},&self.math).map_err(|e|e.to_string())
 }
}
