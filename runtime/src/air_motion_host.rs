//! Persistent ordinary jump/flight records; host owns collision and action decisions.
use crate::{disc::Result,original_math::OriginalMath,original_movement::{AirContext,reconcile_velocity,align_airborne,air_acceleration,integrate_velocity},original_jump_takeoff};
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn vector(b:&mut[u8],i:usize,v:[f32;3]){for j in 0..3{put(b,i+j*4,v[j].to_bits());}}
fn read_vector(b:&[u8],i:usize)->[f32;3]{std::array::from_fn(|j|f32::from_be_bytes(b[i+j*4..i+j*4+4].try_into().unwrap()))}
pub struct AirMotion{entity:Vec<u8>,motion:Vec<u8>,character:Vec<u8>}
pub struct AirOutput{pub position:[f32;3],pub velocity:[f32;3],pub yaw:f32}
impl AirMotion{
 /// Full source terrain response after flight forces. Keep impact velocity
 /// in the source records so landing selection can inspect it afterward.
 pub fn resolve_source_terrain(&mut self,collision:&mut crate::original_position_a::PositionState,world:&crate::original_terrain_candidates::CandidateWorld,ctx:&crate::original_position_b::PositionBContext,math:&OriginalMath)->Result<bool>{
  self.resolve_source_terrain_with_platform(collision,world,ctx,math,|_,_,_,_,_|Err("moving-platform callback requires host integration".into()))
 }
 /// Supply the host's implementation of an original platform callback.
 /// Arguments are callback address, player task, entity address, slot address,
 /// and transactional guest memory. Mutate only that memory: external callback
 /// side effects cannot be rolled back if a later collision operation fails.
 /// Controller and collision records commit only after the whole step succeeds.
 pub fn resolve_source_terrain_with_platform<F>(&mut self,collision:&mut crate::original_position_a::PositionState,world:&crate::original_terrain_candidates::CandidateWorld,ctx:&crate::original_position_b::PositionBContext,math:&OriginalMath,callback:F)->Result<bool>
 where F:FnMut(u32,u32,u32,u32,&mut crate::original_platform::PlatformMemory)->Result<()> {

  let mut next=collision.clone();next.entity=self.entity.clone();next.motion=self.motion.clone();next.character=self.character.clone();
  let mut special=0;
  crate::original_position_b::update(&mut next,&mut special,world,ctx,math,callback)?;
  if next.entity[0x14..0x20].chunks_exact(4).chain(next.motion[..12].chunks_exact(4)).any(|b|!f32::from_be_bytes(b.try_into().unwrap()).is_finite()){return Err("nonfinite airborne collision output".into());}
  let grounded=next.entity[5]&1!=0;
  self.entity=next.entity.clone();self.motion=next.motion.clone();self.character=next.character.clone();*collision=next;Ok(grounded)
 }
 /// Retain a completed action10 collision, including support flags, until the
 /// next decision consumes it. Unlike support loss, this must not clear contact.
 pub fn from_completed_fall_contact(entity:&[u8],motion:&[u8],character:&[u8])->Result<Self>{
  if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0||entity[0]!=10{return Err("invalid completed fall contact records".into());}
  Ok(Self{entity:entity.to_vec(),motion:motion.to_vec(),character:character.to_vec()})
 }
 /// Host detected loss of ordinary support. No takeoff impulse or physics step.
 pub fn from_fall_records(entity:&[u8],motion:&[u8],character:&[u8])->Result<Self>{
  if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0{return Err("invalid fall records".into());}
  let mut s=Self{entity:entity.to_vec(),motion:motion.to_vec(),character:character.to_vec()};
  s.entity[0]=10;s.entity[5]&=!3;Ok(s)
 }
 /// Preserve a completed homing rebound without applying a jump impulse.
 pub fn from_homing_contact(entity:&[u8],motion:&[u8],character:&[u8])->Result<Self>{
  if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0||entity[0]!=6{return Err("invalid homing rebound records".into());}
  let clip=u16::from_be_bytes(character[0x176..0x178].try_into().unwrap());
  if !(101..=103).contains(&clip){return Err("invalid homing rebound animation".into());}
  Ok(Self{entity:entity.to_vec(),motion:motion.to_vec(),character:character.to_vec()})
 }
 pub fn configure(&mut self,physics:&[u8])->Result<()>{
  crate::sonic::Physics::decode(physics)?;self.character[0xc0..0x144].copy_from_slice(physics);Ok(())
 }
 pub fn model_basis(&self,math:&OriginalMath)->Result<[[f32;3];3]>{
  let rotate=|v|crate::original_ground::rotate_vector(&self.entity,v,true,math);
  Ok([rotate([0.,0.,-1.])?,rotate([0.,1.,0.])?,rotate([1.,0.,0.])?])
 }
 pub fn from_ground(entity:&[u8],motion:&[u8],character:&[u8],context:&AirContext,math:&OriginalMath)->Result<Self>{
  let mut s=Self{entity:entity.to_vec(),motion:motion.to_vec(),character:character.to_vec()};
  original_jump_takeoff::advance(&mut s.entity,&mut s.motion,&mut s.character,context,math)?;Ok(s)
 }
 /// Attached takeoff; host maps current player to these records and supplies
 /// source attachment memory. First physics is included, collision is not.
 pub fn from_ground_with_platform(entity:&[u8],motion:&[u8],character:&[u8],memory:&crate::original_contact::Memory,context:&AirContext,math:&OriginalMath)->Result<Self>{
  let mut s=Self{entity:entity.to_vec(),motion:motion.to_vec(),character:character.to_vec()};
  original_jump_takeoff::advance_with_platform(&mut s.entity,&mut s.motion,&mut s.character,memory,context,math)?;Ok(s)
 }
 /// Call after collision and other frame consumers, before the next tick.
 /// Source collision storage is refreshed from this controller next resolve.
 /// Clears forces only; does not implement other source frame housekeeping.
 pub fn finish_frame_forces(&mut self)->Result<()>{original_jump_takeoff::clear_frame_forces(&mut self.motion,&mut self.character)}
 pub fn source_records(&self)->(&[u8],&[u8],&[u8]){(&self.entity,&self.motion,&self.character)}
 pub fn output(&self)->AirOutput{
  let yaw=u32::from_be_bytes(self.entity[12..16].try_into().unwrap())as f32*std::f32::consts::TAU/65536.;
  AirOutput{position:read_vector(&self.entity,0x14).map(|v|v*0.1),velocity:read_vector(&self.motion,0).map(|v|v*6.),yaw:std::f32::consts::FRAC_PI_2-yaw}
 }
 /// Ordinary airborne frame. `held` is already filtered by the host action
 /// decision; callers stop this bridge on landing, bounce or homing transition.
 pub fn tick(&mut self,position:[f32;3],velocity:[f32;3],held:bool,context:&AirContext,math:&OriginalMath)->Result<AirOutput>{
  if position.iter().chain(velocity.iter()).any(|v|!v.is_finite()){return Err("nonfinite air host input".into());}
  vector(&mut self.entity,0x14,position.map(|v|v*10.));vector(&mut self.motion,0,velocity.map(|v|v/6.));
  self.entity[5]&=!3;
  reconcile_velocity(&mut self.entity,&mut self.motion,&mut self.character,context.gravity,context.level,math)?;
  if held && self.entity[4]&0x40==0{
   let prior=i16::from_be_bytes(self.character[0x12..0x14].try_into().unwrap());
   self.character[0x12..0x14].copy_from_slice(&prior.wrapping_add(1).to_be_bytes());
   let budget=i32::from_be_bytes(self.character[0xc0..0xc4].try_into().unwrap());
   if (prior as i32)<budget{
    let local=read_vector(&self.character,0x64);
    let add=f32::from_be_bytes(self.character[0xf8..0xfc].try_into().unwrap());
    put(&mut self.character,0x68,0.8_f32.mul_add(add,local[1]).to_bits());
   }
  }
  align_airborne(&mut self.entity,&mut self.character,0,0,math)?;
  air_acceleration(&mut self.entity,&mut self.motion,&mut self.character,context,math)?;
  integrate_velocity(&self.entity,&mut self.motion,&mut self.character,context.magnitudes,math)?;
  Ok(self.output())
 }
}
