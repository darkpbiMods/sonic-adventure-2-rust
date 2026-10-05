//! Persistent bounce records. Host resolves higher-priority actions before tick
//! and ordinary landing after Landed; this bridge never invents world-Y impulses.
use crate::{disc::Result,original_math::OriginalMath,original_position_a::PositionState,
 original_position_b::PositionBContext,original_terrain_candidates::CandidateWorld};
#[derive(Clone)]
pub struct BounceMotion{state:PositionState,extension:Vec<u8>,special_mode:u32}
#[derive(Debug,PartialEq,Eq)]
pub enum BounceStep{Flying,Rebounded,Repeated,Landed}
impl BounceMotion{
 pub fn enter(mut state:PositionState,math:&OriginalMath)->Result<Self>{
  let mut extension=vec![0;0x368];
  crate::original_bounce_contact::enter(&mut state.entity,&mut state.character,&mut extension,false,math)?;
  Ok(Self{state,extension,special_mode:0})
 }
 pub fn records(&self)->&PositionState{&self.state}
 pub fn into_records(self)->PositionState{self.state}
 pub fn tick(&mut self,repeat_pressed:bool,world:&CandidateWorld,ctx:&PositionBContext,math:&OriginalMath)->Result<BounceStep>{
  self.tick_with_platform(repeat_pressed,world,ctx,math,|_|Ok(()),|_,_,_,_,_|Err("bounce moving-platform adapter required".into()))
 }
 /// Memory preparation and callback edits commit only on successful bounce step.
 /// Keep external side effects outside both closures; those cannot be rolled back.
 pub fn tick_with_platform<P,F>(&mut self,repeat_pressed:bool,world:&CandidateWorld,ctx:&PositionBContext,math:&OriginalMath,prepare:P,callback:F)->Result<BounceStep>
 where P:FnOnce(&mut crate::original_platform::PlatformMemory)->Result<()>,
 F:FnMut(u32,u32,u32,u32,&mut crate::original_platform::PlatformMemory)->Result<()> {

  let mut next=self.clone();
  let supported=u16::from_be_bytes(next.state.entity[4..6].try_into().unwrap())&3!=0;
  // Action70 resolves landing before its later repeated-bounce input branch.
  if next.state.entity[0]==70&&supported{return Ok(BounceStep::Landed);}
  let mut outcome=BounceStep::Flying;
  if next.state.entity[0]==69{
   if crate::original_bounce_contact::advance(&mut next.state.entity,&next.state.motion,&mut next.state.character,&mut next.extension,math)?{outcome=BounceStep::Rebounded;}
  }else if next.state.entity[0]==70&&repeat_pressed{
   crate::original_bounce_contact::enter(&mut next.state.entity,&mut next.state.character,&mut next.extension,true,math)?;
   outcome=BounceStep::Repeated;
  }
  prepare(&mut next.state.memory)?;
  crate::original_bounce_physics::advance(&mut next.state,&mut next.special_mode,world,ctx,math,callback)?;
  *self=next;Ok(outcome)
 }
}
