//! Original position parent 8003eafc..8003f760. Not a complete gameplay frame.
use crate::{disc::Result,original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES,angle_distance},
 original_math::{OriginalMath,IDENTITY,transform},original_geometry::{signed_angle,point_plane},
 original_terrain_candidates::{CandidateState,CandidateWorld,QueryNode,collect_triangles,sweep_objects},
 original_terrain_sweep::{SweepState,sweep},original_terrain_selection::{Selection,SelectionContext,select},
 original_terrain_response::{ResponseContext,respond},original_contact_motion::{PositionContact,position},
 original_control::{ControlContext,InputOutputs},original_contact::Memory,
 original_platform::{PlatformMemory,PlatformCall}};
#[derive(Clone)]pub struct PositionState{
 pub entity:Vec<u8>,pub motion:Vec<u8>,pub character:Vec<u8>,pub workspace:Vec<u8>,
 pub candidates:CandidateState,pub nodes:Vec<QueryNode>,pub sweep:SweepState,
 pub collision_flag:u32,pub control_cache:[u32;4],pub memory:PlatformMemory,
}
pub struct PositionContext{
 pub control:ControlContext,pub incoming_f26:f64,
 pub entity_identity:u32,pub motion_identity:u32,pub character_identity:u32,pub workspace_identity:u32,
 /// Host scratch cell used only while dispatching a platform callback.
 pub platform_offset_identity:u32,
}
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn short(b:&[u8],i:usize)->u16{u16::from_be_bytes(b[i..i+2].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn vector(b:&[u8],i:usize)->[f32;3]{std::array::from_fn(|j|float(b,i+j*4))}
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn half(b:&mut[u8],i:usize,v:u16){b[i..i+2].copy_from_slice(&v.to_be_bytes());}
fn putv(b:&mut[u8],i:usize,v:[f32;3]){for j in 0..3{put(b,i+j*4,v[j].to_bits());}}
fn add(a:[f32;3],b:[f32;3])->[f32;3]{std::array::from_fn(|i|a[i]+b[i])}
fn sub(a:[f32;3],b:[f32;3])->[f32;3]{std::array::from_fn(|i|a[i]-b[i])}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],a[0]*b[0]))}
fn environment(s:&mut PositionState,flags:u32,ctx:&PositionContext,math:&OriginalMath)->Result<()>{
 crate::original_environment::update(&s.entity,&mut s.character,flags,ctx.control.level,math)
}
fn offsets(s:&mut PositionState,ctx:&PositionContext,math:&OriginalMath)->Result<([f32;3],[f32;3])>{
 let mut matrix=math.rotate(&IDENTITY,b'z',word(&s.entity,0x10)as i32)?;
 matrix=math.rotate(&matrix,b'x',word(&s.entity,8)as i32)?;
 matrix=math.rotate(&matrix,b'y',(word(&s.entity,12)as i32).wrapping_neg())?;
 let k=|o:u32|math.constant(0x803b6520-o);let zero=k(0x75f8);
 put(&mut s.workspace,0x34,word(&s.entity,8));put(&mut s.workspace,0x38,word(&s.entity,0x10));
 putv(&mut s.workspace,0x1c,transform(&matrix,[zero,k(0x75c8),zero]));
 let halfheight=transform(&matrix,[zero,k(0x75cc)*float(&s.character,0x134),zero]);
 let mut local=[zero,k(0x75cc)*float(&s.character,0x134),zero];crate::original_offset::update(&s.motion,&s.character,vector(&s.workspace,0x1c),ctx.control.gravity,&mut local,math)?;
 Ok((transform(&matrix,local),halfheight))
}
fn write_record(memory:&mut PlatformMemory,address:u32,bytes:&[u8])->Result<()>{
 for (i,b)in bytes.chunks_exact(4).enumerate(){memory.put_word(address.wrapping_add(i as u32*4),u32::from_be_bytes(b.try_into().unwrap()))?;}Ok(())
}
fn read_record(memory:&PlatformMemory,address:u32,bytes:&mut[u8])->Result<()>{
 for (i,b)in bytes.chunks_exact_mut(4).enumerate(){b.copy_from_slice(&memory.word(address.wrapping_add(i as u32*4))?.to_be_bytes());}Ok(())
}
pub(crate) fn platform<F>(s:&mut PositionState,offset:[f32;3],ctx:&PositionContext,math:&OriginalMath,callback:&mut F)->Result<bool>
where F:FnMut(u32,u32,u32,u32,&mut PlatformMemory)->Result<()>{
 let mut scratch=[0;3];for i in 0..3{scratch[i]=s.memory.word(ctx.platform_offset_identity+i as u32*4)?;}
 write_record(&mut s.memory,ctx.entity_identity,&s.entity)?;write_record(&mut s.memory,ctx.motion_identity,&s.motion)?;
 write_record(&mut s.memory,ctx.character_identity,&s.character)?;write_record(&mut s.memory,ctx.workspace_identity,&s.workspace)?;
 for i in 0..3{s.memory.put_float(ctx.platform_offset_identity+i as u32*4,offset[i])?;}
 let call=PlatformCall{entity:ctx.entity_identity,character:ctx.character_identity,workspace:ctx.workspace_identity,offset:ctx.platform_offset_identity,player_tasks:ctx.control.player_tasks};
 let moved=crate::original_platform::apply(&mut s.memory,call,math,callback)?;
 read_record(&s.memory,ctx.entity_identity,&mut s.entity)?;read_record(&s.memory,ctx.motion_identity,&mut s.motion)?;
 read_record(&s.memory,ctx.character_identity,&mut s.character)?;read_record(&s.memory,ctx.workspace_identity,&mut s.workspace)?;
 for i in 0..3{s.memory.put_word(ctx.platform_offset_identity+i as u32*4,scratch[i])?;}Ok(moved)
}
/// Transactional host adapter. Unknown platform callbacks must return errors.
pub fn update<F>(state:&mut PositionState,world:&CandidateWorld,ctx:&PositionContext,math:&OriginalMath,mut callback:F)->Result<u32>
where F:FnMut(u32,u32,u32,u32,&mut PlatformMemory)->Result<()>{
 if state.entity.len()!=ENTITY_BYTES||state.character.len()!=CHARACTER_BYTES||state.motion.len()!=0x40||state.workspace.len()!=0x84{return Err("invalid position A state extent".into());}
 let mut next=state.clone();let result=inner(&mut next,world,ctx,math,&mut callback)?;*state=next;Ok(result)
}
fn inner<F>(s:&mut PositionState,world:&CandidateWorld,ctx:&PositionContext,math:&OriginalMath,callback:&mut F)->Result<u32>
where F:FnMut(u32,u32,u32,u32,&mut PlatformMemory)->Result<()>{
 let k=|o:u32|math.constant(0x803b6520-o);let gravity=ctx.control.gravity;
 let previous_status=short(&s.character,0x1c)&3;
 half(&mut s.character,0x1c,short(&s.entity,4));let status=short(&s.entity,4);half(&mut s.entity,4,status&!1);
 let (offset,mut halfheight)=offsets(s,ctx,math)?;
 putv(&mut s.workspace,4,add(vector(&s.entity,0x14),offset));
 put(&mut s.workspace,0,(k(0x75cc)*float(&s.character,0x134)).to_bits());
 putv(&mut s.workspace,0x10,add(vector(&s.motion,0),vector(&s.character,0x4c)));
 let previous_position=vector(&s.entity,0x14);
 collect_triangles(&s.workspace,world,&mut s.nodes,&mut s.candidates,false,math)?;
 let objects=sweep_objects(&s.nodes,world,&s.candidates)?;
 let mut list=Vec::<[u32;3]>::new();let mut chosen=None;
 for record in s.candidates.objects[..s.candidates.object_count as usize*12].chunks_exact(12){
  let record=[word(record,0),word(record,4),word(record,8)];let [flags,id,attached]=record;
  if flags&0x2002!=0{continue;}
  if attached!=0{
   let entity=s.memory.word(attached.wrapping_add(0x34))?;
   let flags=(s.memory.word(entity.wrapping_add(4))?>>16)as u16;if flags&0x100==0{continue;}
  }
  if sweep(&mut s.workspace,&objects,id,0,false,&mut s.sweep,math)?{
   let kind=if short(&s.character,0x1c)&3!=0{Selection::Primary}else{Selection::Limit{incoming_f26:ctx.incoming_f26}};
   let context=SelectionContext{gravity,surface_flags:flags,descriptor_attached:attached!=0};
   if select(kind,&s.entity,&mut s.motion,&s.character,&mut s.workspace,&s.sweep.contacts,&mut s.collision_flag,&context,math)?==1{chosen=Some(list.len());}
  }
  list.push(record);if list.len()==31{break;}
 }
 if list.is_empty(){
  let moved=add(add(vector(&s.entity,0x14),vector(&s.character,0x4c)),vector(&s.motion,0));putv(&mut s.entity,0x14,moved);
  environment(s,0,ctx,math)?;put(&mut s.character,0xa0,0);put(&mut s.character,0xb0,0);return Ok(0);
 }
 // Source prioritizes the selected contact and puts the previous object last.
 let mut ordered=vec![None;list.len()];let mut cursor=0;
 if let Some(index)=chosen{ordered[0]=Some(list[index]);cursor=1;}
 let indexes:Vec<_>=if word(&s.character,0xa0)!=0{(0..list.len()).collect()}else{(0..list.len()).rev().collect()};
 for i in indexes{
  if Some(i)==chosen{continue;}
  if list[i][1]==word(&s.character,0xb0){let last=ordered.len()-1;ordered[last]=Some(list[i]);}
  else{if cursor>=ordered.len(){return Err("invalid source candidate ordering".into());}ordered[cursor]=Some(list[i]);cursor+=1;}
 }
 let mut ordered:Vec<[u32;3]>=ordered.into_iter().collect::<Option<Vec<_>>>().ok_or("source ordering leaves uninitialized stack descriptors")?;
 putv(&mut s.workspace,4,add(vector(&s.entity,0x14),offset));
 putv(&mut s.workspace,0x10,add(vector(&s.motion,0),vector(&s.character,0x4c)));
 put(&mut s.character,0xa0,0);let mut result=0;let mut landed=false;let mut last_id=0;let mut index=0;let mut iterations=0;
 while index<ordered.len(){
  iterations+=1;if iterations>4096{return Err("position A contact restart limit".into());}
  let [flags,id,mut attached]=ordered[index];
  if !sweep(&mut s.workspace,&objects,id,0,false,&mut s.sweep,math)?{index+=1;continue;}
  let ground=vector(&s.workspace,0x1c);let normal=vector(&s.workspace,0x48);
  let angle=signed_angle(ground,normal,None,math)? as i32;
  let old_status=short(&s.character,0x1c);let vertical=float(&s.character,0x68);
  let contact_branch=short(&s.entity,4)&2!=0 || flags&0x401000!=0 ||
   (angle>0x2c00 && dot(gravity,ground)<=dot(gravity,normal)) ||
   (previous_status!=0 && old_status&3==0) || (dot(gravity,normal)>k(0x75f8) && old_status&3==0) ||
   (vertical>k(0x7540) && (vertical<=k(0x75f8)||old_status&3!=0));
  if contact_branch{
   let contact=PositionContact{previous_position,offset:halfheight,previous_result:result,flags,stop_mode:s.collision_flag};
   result=position(&mut s.entity,&mut s.character,&mut s.workspace,&mut s.motion,&contact,math)?;
   index+=1;continue;
  }
  landed=true;last_id=id;
  let response=respond(false,&mut s.entity,&mut s.motion,&mut s.character,&mut s.workspace,&s.sweep.contacts,&ResponseContext{gravity,surface_flags:flags,motion_identity:ctx.motion_identity},math)?;
  if response!=0 && (result==0||response==2){result=response;}
  let angles=math.surface_angles(vector(&s.workspace,0x48))?;
  put(&mut s.workspace,0x34,angles[0]);put(&mut s.workspace,0x38,angles[1]);
  let unchanged=float(&s.motion,0)==k(0x75f8)&&float(&s.motion,8)==k(0x75f8)&&short(&s.character,0x1c)&1!=0&&dot(gravity,vector(&s.workspace,0x48))>=dot(gravity,vector(&s.workspace,0x1c));
  if !unchanged{
   if angle_distance(word(&s.entity,0x10),angles[1])>0x4000{put(&mut s.workspace,0x38,angles[1].wrapping_sub(0x8000));}
   if float(&s.workspace,0x4c)<=k(0x75f8)&&angle_distance(word(&s.workspace,0x38),0)<0x4000{put(&mut s.workspace,0x34,0x8000_u32.wrapping_sub(angles[0]));}
   crate::original_terrain_align::align(&mut s.entity,&s.character,&s.workspace,flags,gravity,math)?;
  }
  if result==0{halfheight=offsets(s,ctx,math)?.1;}
  put(&mut s.motion,0x18,word(&s.workspace,0x34));put(&mut s.motion,0x20,word(&s.workspace,0x38));
  putv(&mut s.character,0x7c,vector(&s.workspace,0x48));
  let displacement=vector(&s.workspace,0x6c);let (length,unit)=math.estimated_vector(displacement)?;
  let restart=length>k(0x753c)as f64;
  if restart{
   let unit=unit.ok_or("undefined contact displacement direction")?;
   let remaining=(math.estimated_vector(vector(&s.workspace,0x10))?.0-k(0x753c)as f64)as f32;
   putv(&mut s.workspace,0x10,unit.map(|v|v*remaining));
   putv(&mut s.workspace,0x6c,unit.map(|v|v*k(0x753c)));
   let last=ordered.len()-1;ordered.swap(index,last);
  }else{putv(&mut s.workspace,0x10,[k(0x75f8);3]);}
  let point=add(vector(&s.workspace,0x60),vector(&s.workspace,0x6c));
  let record=short(&s.workspace,0x2a)as usize;
  if record>=128{return Err("position A selected contact outside storage".into());}
  let p=vector(&s.sweep.contacts,record*0x58+0x20);let n=vector(&s.workspace,0x1c);
  let projected=point_plane(point,[p[0],p[1],p[2],n[0],n[1],n[2]],true,math)?.1.ok_or("missing plane projection")?;
  putv(&mut s.workspace,4,add(halfheight,projected));
  if attached!=0{put(&mut s.character,0xa0,attached);if !platform(s,halfheight,ctx,math,callback)?{attached=0;}}
  else{put(&mut s.character,0xa0,0);}
  let new_position=sub(vector(&s.workspace,4),halfheight);
  let distance=crate::original_terrain_contacts::distance(vector(&s.entity,0x14),new_position,math)?;
  let stationary=attached==0&&float(&s.character,0x64)==k(0x75f8)&&float(&s.character,0x6c)==k(0x75f8);
  if (!stationary||distance>k(0x75c8))&&distance>k(0x7538){putv(&mut s.entity,0x14,new_position);}
  let status=short(&s.entity,4);half(&mut s.entity,4,status|1);
  // The source uses r28 after swapping descriptors, so read current flags again.
  environment(s,ordered[index][0],ctx,math)?;
  index=if restart{0}else{index+1};
 }
 if !landed{
  let status=short(&s.character,0x1c);let mut retreat=false;
  if status&1!=0&&status&0x4000==0{
   let mut outputs=InputOutputs{direction:None,magnitude:Some(k(0x75f8))};
   let active=crate::original_control::read(&s.entity,&s.character,&ctx.control,&Memory{base:s.memory.base,bytes:&s.memory.bytes},&mut s.control_cache,&mut outputs,math)?;
   retreat=(!active || (outputs.magnitude.unwrap()<=k(0x7548)&&short(&s.character,0x14)==0))&&
    float(&s.character,0x68)<=k(0x75f8)&&float(&s.character,0x64)<=float(&s.character,0xe8)&&float(&s.character,0x34)>k(0x7534);
  }
  if retreat{
   let old=vector(&s.entity,0x14);let velocity=vector(&s.motion,0);
   putv(&mut s.entity,0x14,std::array::from_fn(|i|-k(0x75cc).mul_add(velocity[i],-old[i])));
   putv(&mut s.motion,0,[k(0x75f8);3]);
  }else{let moved=add(vector(&s.entity,0x14),vector(&s.workspace,0x10));putv(&mut s.entity,0x14,moved);}
  environment(s,0,ctx,math)?;
 }else{
  put(&mut s.character,0xb0,last_id);let moved=add(vector(&s.entity,0x14),vector(&s.workspace,0x10));putv(&mut s.entity,0x14,moved);
 }
 Ok(result)
}
