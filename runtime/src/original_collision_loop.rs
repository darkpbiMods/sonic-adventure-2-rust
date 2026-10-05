//! Original 80045880..8004614c. Object contacts, terrain movement and landing.
//! Synthetic graph adapters are explicit; unknown platform callbacks are errors.
use crate::{disc::Result, original_contact::{Memory,select_contact},
 original_gameplay::{angle_delta,angle_distance,ENTITY_BYTES,CHARACTER_BYTES},
 original_math::{OriginalMath,IDENTITY,transform},original_ground::rotate_vector,
 original_position_a::{PositionState,PositionContext},original_position_b::PositionBContext,
 original_platform::PlatformMemory,original_terrain_candidates::CandidateWorld,
 original_wall::{WallContact,respond}};
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn half(b:&[u8],i:usize)->u16{u16::from_be_bytes(b[i..i+2].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn vec(b:&[u8],i:usize)->[f32;3]{std::array::from_fn(|j|float(b,i+j*4))}
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn putv(b:&mut[u8],i:usize,v:[f32;3]){for j in 0..3{put(b,i+j*4,v[j].to_bits());}}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],a[0]*b[0]))}
fn unit(v:[f32;3],math:&OriginalMath)->Result<[f32;3]>{Ok(math.estimated_vector(v)?.1.unwrap_or([0.;3]))}
fn view(s:&PositionState)->Memory<'_>{Memory{base:s.memory.base,bytes:&s.memory.bytes}}
fn sync(s:&mut PositionState,ctx:&PositionContext)->Result<()>{
 for (address,bytes)in [(ctx.entity_identity,&s.entity),(ctx.motion_identity,&s.motion),(ctx.character_identity,&s.character),(ctx.workspace_identity,&s.workspace)]{
  for (i,b)in bytes.chunks_exact(4).enumerate(){s.memory.put_word(address.wrapping_add(i as u32*4),u32::from_be_bytes(b.try_into().unwrap()))?;}
 }Ok(())
}
fn contact(s:&PositionState,collision:u32)->Result<WallContact>{
 let m=view(s);let flags=m.half(collision.wrapping_add(4))?;
 let mut vector=[0.;3];if flags&4!=0{for i in 0..3{vector[i]=f32::from_bits(m.word(collision.wrapping_add(0x90+i as u32*4))?);}}
 Ok(WallContact{flags,vector})
}
fn special_wall(s:&PositionState,collision:u32)->Result<bool>{
 if collision==0{return Ok(false);}let m=view(s);let status=half(&s.entity,4);
 if (m.half(collision.wrapping_add(2))?as i16)<=0 || status&4!=0 || m.half(collision.wrapping_add(4))?&0x14!=0x14{return Ok(false);}
 let index=m.half(collision.wrapping_add(0xa2))?as i16 as i32;
 let other=m.word(collision.wrapping_add(0xa4))?;
 let shape=m.word(other.wrapping_add(0xc))?.wrapping_add(index.wrapping_mul(0x30)as u32);
 Ok(m.word(shape.wrapping_add(4))?&0x2000==0 || status&0x800!=0)
}
fn stop(s:&mut PositionState){putv(&mut s.character,0x64,[0.;3]);putv(&mut s.motion,0,[0.;3]);}
fn stop_horizontal(s:&mut PositionState){put(&mut s.motion,0,0);put(&mut s.motion,8,0);}
fn object_response(s:&mut PositionState,ctx:&PositionContext,math:&OriginalMath)->Result<u32>{
 sync(s,ctx)?;let collision=word(&s.entity,0x2c);let selected=select_contact(&view(s),ctx.entity_identity)?;
 if selected==0{
  let m=view(s);
  if m.half(collision.wrapping_add(4))?&1==0{return Ok(0);}
  let other=m.word(collision.wrapping_add(0xa4))?;if other==0{return Ok(0);}
  let task=m.word(other.wrapping_add(0x9c))?;if task==0{return Ok(0);}
  let entity=m.word(task.wrapping_add(0x34))?;if entity==0 || m.byte(entity.wrapping_add(2))?!=0x12{return Ok(0);}
  let status=half(&s.entity,4);
  if status&1==0 || float(&s.character,0x68)>0. {return Ok(0);}
  if !(float(&s.character,0x64)>=float(&s.character,0xe8)) && status&0x100==0{return Ok(0);}
  if angle_distance(0x4000_u32.wrapping_sub(word(&s.entity,12)),m.word(entity.wrapping_add(12))?)<0x3000{stop(s);return Ok(3);}
  return Ok(0);
 }
 let m=view(s);let own_shape=m.word(collision.wrapping_add(0xc))?;
 let other_entity=m.word(selected.wrapping_add(0x34))?;let other=m.word(other_entity.wrapping_add(0x2c))?;
 let index=m.half(other.wrapping_add(0xa2))?as i16 as i32;
 let shape=m.word(other.wrapping_add(0xc))?.wrapping_add(index.wrapping_mul(0x30)as u32);
 let kind=m.byte(shape)?;let shape_mode=m.byte(shape.wrapping_add(1))?;
 let own_radius=f32::from_bits(m.word(own_shape.wrapping_add(0x14))?);
 let other_radius=f32::from_bits(m.word(shape.wrapping_add(0x14))?);
 let rounded=(shape_mode==0||shape_mode==1)&&other_radius>own_radius&&m.half(other)?==4&&
  (m.byte(shape.wrapping_add(3))?&3)<=((m.byte(own_shape.wrapping_add(3))?>>2)&3);
 let wall=contact(s,collision)?;let mut normal=wall.vector;if ctx.control.level==0x5a{normal[1]=0.;}
 normal=unit(normal,math)?;putv(&mut s.character,0x70,normal);
 let mut direction=unit(vec(&s.motion,0),math)?;let alignment=dot(direction,normal);
 let speed=float(&s.character,0x64);let k=|o:u32|math.constant(0x803b6520-o);
 if speed>=k(0x74e0)&&kind!=0x10{stop(s);return Ok(2);}
 if speed>=float(&s.character,0xf0)&&alignment<k(0x74b4)&&kind!=0x10{stop_horizontal(s);return Ok(2);}
 if speed>=float(&s.character,0xe8)&&alignment<0.{
  if rounded{
   let squared=direction[0]*direction[0]+direction[2]*direction[2];
   let length=math.sqrt_refined(squared);let ratio=length/(own_radius+other_radius);
   let integer=|v:f64|if v.is_nan(){i32::MIN}else{v as i32};
   let scale=math.double_constant(0x803b6520-0x75e8);
   let mut angle=integer(scale*math.acos(ratio)as f64)as u32;
   let facing=(integer(scale*math.atan2(normal[2],normal[0])as f64)as u32).wrapping_add(0x8000);
   let movement=integer(scale*math.atan2(direction[2],direction[0])as f64)as u32;
   if angle_delta(facing,movement)<0{angle=0x10000_u32.wrapping_sub(angle);}
   let matrix=math.rotate(&IDENTITY,b'y',(angle as i32).wrapping_neg())?;
   let turned=transform(&matrix,vec(&s.motion,0));let yaw=word(&s.entity,12).wrapping_add(angle);
   put(&mut s.entity,12,yaw);put(&mut s.motion,0,turned[0].to_bits());put(&mut s.motion,8,turned[2].to_bits());
  }else if shape_mode.wrapping_sub(3)<=2{
   if special_wall(s,collision)?{stop_horizontal(s);}else{respond(&mut s.entity,&mut s.motion,&mut s.character,&wall,ctx.control.level,math)?;}
  }
  return Ok(1);
 }
 // Original unordered compare follows the positive branch for NaN; finite
 // vector-domain guards above reject nonfinite inputs before this point.
 if alignment<=0.{return Ok(1);}
 sync(s,ctx)?;
 let memory=Memory{base:s.memory.base,bytes:&s.memory.bytes};
 let response=crate::original_collision_select::dispatch(&memory,ctx.entity_identity,ctx.character_identity,ctx.control.player_tasks,&mut s.control_cache,math)?;
 put(&mut s.character,0x9c,response.cached_task);
 if response.flags&0x400!=0{
  put(&mut s.motion,0,(-speed*normal[0]).to_bits());put(&mut s.motion,8,(-speed*normal[2]).to_bits());
 }else if special_wall(s,collision)?{
  direction[1]=0.;normal[1]=0.;
  if math.estimated_vector(direction)?.0==0.||math.estimated_vector(normal)?.0==0.{stop_horizontal(s);}else{
   direction=unit(direction,math)?;normal=unit(normal,math)?;
   let sum=std::array::from_fn(|i|normal[i]+direction[i]);
   if math.estimated_vector(sum)?.0==0.{stop_horizontal(s);}else{
    let result=unit(sum,math)?;put(&mut s.motion,0,(speed*result[0]).to_bits());put(&mut s.motion,8,(speed*result[2]).to_bits());
   }
  }
 }else{respond(&mut s.entity,&mut s.motion,&mut s.character,&wall,ctx.control.level,math)?;}
 Ok(1)
}
/// Atomic host state commit, with the source's return-priority and dispatch.
pub fn update<F>(state:&mut PositionState,special_mode:&mut u32,world:&CandidateWorld,ctx:&PositionBContext,math:&OriginalMath,mut callback:F)->Result<u32>
where F:FnMut(u32,u32,u32,u32,&mut PlatformMemory)->Result<()>{
 if state.entity.len()!=ENTITY_BYTES||state.character.len()!=CHARACTER_BYTES||state.motion.len()!=0x40||state.workspace.len()!=0x84{return Err("invalid collision loop state extent".into());}
 let mut s=state.clone();let mut mode=*special_mode;
 putv(&mut s.character,0x70,[0.;3]);let previous=word(&s.character,0x88);put(&mut s.character,0x8c,previous);put(&mut s.character,0x88,0);
 let object_result=object_response(&mut s,ctx.common,math)?;
 sync(&mut s,ctx.common)?;
 let status=half(&s.entity,4);let correction=status&2!=0;
 // The apparent (status&1)|2 condition in the executable is always nonzero.
 let terrain_result=if status&0x100!=0||s.entity[0]==0x34{
  crate::original_position_a::update(&mut s,world,ctx.common,math,&mut callback)?
 }else{crate::original_position_b::update(&mut s,&mut mode,world,ctx,math,&mut callback)?};
 let result=if object_result==0{terrain_result}else{object_result};
 if correction&&half(&s.entity,4)&1==0{
  let normal=rotate_vector(&s.entity,[0.,math.constant(0x803b6520-0x75c8),0.],true,math)?;
  putv(&mut s.character,0x7c,normal);
  let adjustment=rotate_vector(&s.entity,[0.,math.constant(0x803b6520-0x74b0),0.],true,math)?;
  let y=float(&s.entity,0x18)+adjustment[1];put(&mut s.entity,0x18,y.to_bits());
  if float(&s.character,0x68)<=0.{
   let mut local=rotate_vector(&s.entity,vec(&s.motion,0),false,math)?;local[1]=0.;putv(&mut s.character,0x64,local);
   let world_velocity=rotate_vector(&s.entity,local,true,math)?;putv(&mut s.motion,0,world_velocity);
  }else{let status=half(&s.entity,4)&!2;s.entity[4..6].copy_from_slice(&status.to_be_bytes());}
 }
 sync(&mut s,ctx.common)?;*state=s;*special_mode=mode;Ok(result)
}
