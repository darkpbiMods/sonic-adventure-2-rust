//! Timing-only host for the complete recovered animation updater.
//! Zero sampled channels retain allocator/transition timing without pretending
//! to produce blended poses. Callers supply original table entries and lengths.
use crate::{disc::Result,original_heap::{self,HeapContext},original_math::OriginalMath,original_animation_update};
const BASE:u32=0x80200000;
const STATE:usize=0x100;
const TABLE:usize=0x1000;
const REGISTRY:usize=0x3000;
#[derive(Clone)]
pub struct AnimationSpec {pub id:u16,pub entry:[u8;16],pub frames:u32}
#[derive(Debug,Clone,Copy)]
pub struct ClockOutput {pub mode:u16,pub selected:u16,pub current:u16,pub previous:u16,pub flags:u16,pub frame:f32}
pub struct AnimationClock {blend:Option<original_animation_update::BlendEndpoints>,memory:Vec<u8>,ids:Vec<u16>,heap:HeapContext,index:i32}
fn half(m:&[u8],at:usize)->u16{u16::from_be_bytes(m[at..at+2].try_into().unwrap())}
fn put(m:&mut[u8],at:usize,v:u32){m[at..at+4].copy_from_slice(&v.to_be_bytes());}
impl AnimationClock {
 pub fn new(specs:&[AnimationSpec],initial:u16)->Result<Self>{
  let mut memory=vec![0;0xa000];let mut ids=Vec::new();
  for s in specs{
   if s.id>=128||ids.contains(&s.id)||s.frames<2{return Err("invalid or duplicate clock animation".into());}
   let blend=f32::from_be_bytes(s.entry[8..12].try_into().unwrap());
   let rate=f32::from_be_bytes(s.entry[12..16].try_into().unwrap());
   if !blend.is_finite()||blend<=0.||!rate.is_finite()||!matches!(half(&s.entry,2),0|6)||half(&s.entry,4)>13{return Err("unsupported clock metadata".into());}
   ids.push(s.id);
   let entry=TABLE+s.id as usize*16;memory[entry..entry+16].copy_from_slice(&s.entry);
   // Remap motion storage by animation ID; source motion IDs need not be dense.
   memory[entry..entry+2].copy_from_slice(&s.id.to_be_bytes());
   let header=0x4000+s.id as usize*16;
   put(&mut memory,REGISTRY+0xe4+s.id as usize*8,BASE+header as u32);
   put(&mut memory,header+4,s.frames); // zero nodes/channels to sample
  }
  if !ids.contains(&initial)||specs.iter().any(|s|!ids.contains(&(half(&s.entry,6)&255))){return Err("clock references absent animation".into());}
  // Distinct model roots preserve the source's pointer identity comparison.
  for model in [0usize,6]{put(&mut memory,REGISTRY+0xa44+model*8,BASE+0x5000+(model as u32)*64);}
  put(&mut memory,STATE+0x24,BASE+TABLE as u32);
  put(&mut memory,STATE+0x1c,BASE+0x600);put(&mut memory,STATE+0x20,BASE+0x604);
  put(&mut memory,0x600,1f32.to_bits());put(&mut memory,0x604,1f32.to_bits());
  let mode=half(&memory,TABLE+initial as usize*16+4);
  memory[STATE..STATE+2].copy_from_slice(&mode.to_be_bytes());
  for offset in [2,4,6]{memory[STATE+offset..STATE+offset+2].copy_from_slice(&initial.to_be_bytes());}
  let next=half(&memory,TABLE+initial as usize*16+6)&255;
  memory[STATE+8..STATE+10].copy_from_slice(&next.to_be_bytes());
  put(&mut memory,0x5800,u32::MAX);
  let heap=HeapContext{table:BASE+0x5800,count:1};
  let index=original_heap::create(BASE,&mut memory,&heap,BASE+0x6000,BASE+0xa000)?;
  let header=original_heap::allocate(BASE,&mut memory,&heap,index,12)?;
  let data=original_heap::allocate(BASE,&mut memory,&heap,index,111*24)?;
  if header==0||data==0{return Err("clock heap exhausted".into());}
  put(&mut memory,(header-BASE)as usize,data);put(&mut memory,(header-BASE)as usize+4,2);
  put(&mut memory,(header-BASE)as usize+8,0x00030002);put(&mut memory,STATE+0x28,header);
  Ok(Self{blend:None,memory,ids,heap,index})
 }
 pub fn tick(&mut self,selected:u16,speed:f32,secondary_speed:f32,math:&OriginalMath)->Result<ClockOutput>{
  if !self.ids.contains(&selected)||!speed.is_finite()||!secondary_speed.is_finite(){return Err("invalid clock input".into());}
  // Stage caller writes too, so rejected updater inputs cannot partially commit.
  let mut memory=self.memory.clone();memory[STATE+2..STATE+4].copy_from_slice(&selected.to_be_bytes());
  put(&mut memory,0x600,speed.to_bits());put(&mut memory,0x604,secondary_speed.to_bits());
  let started=original_animation_update::update_with_transition(BASE+STATE as u32,BASE,&mut memory,BASE+REGISTRY as u32,&self.heap,self.index,math)?;
  self.blend=if half(&memory,STATE)==2{started.or(self.blend)}else{None};
  self.memory=memory;Ok(self.output())
 }
 /// Fixed endpoints of the currently active blend; queued selections do not retarget it.
 pub fn blend(&self)->Option<original_animation_update::BlendEndpoints>{self.blend}
 pub fn output(&self)->ClockOutput{
  let m=&self.memory;
  ClockOutput{mode:half(m,STATE),selected:half(m,STATE+2),current:half(m,STATE+4),previous:half(m,STATE+6),flags:half(m,STATE+12),frame:f32::from_be_bytes(m[STATE+16..STATE+20].try_into().unwrap())}
 }
}
