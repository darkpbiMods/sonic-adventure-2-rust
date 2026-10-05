//! Persistent action59 physics phase. Decision/input and presentation are external.
use crate::{disc::Result,original_math::OriginalMath,original_position_a::PositionState,original_position_b::PositionBContext,original_terrain_candidates::CandidateWorld,original_platform::PlatformMemory,original_ring_dash::{self,RingCandidate}};
#[derive(Clone)]pub struct RingMotion{state:PositionState,special_mode:u32}
pub struct RingPhysicsOutput{pub selected:Option<usize>,pub collision_response:u32,pub stop_sound:bool,pub reset_requested:bool}
/// Caller builds the current offered-action list and resolves higher-priority interrupts first.
/// Other chosen actions are returned for their dispatcher; only action59 enters here.
pub fn offer_and_activate(state:&mut PositionState,center:[f32;3],rings:&[RingCandidate],pressed:bool,held:bool,math:&OriginalMath)->Result<crate::original_action_choice::Choice>{
 let mut next=state.clone();
 original_ring_dash::offer(&next.entity,&mut next.character,center,rings,math)?;
 let choice=crate::original_action_choice::choose(&mut next.character,pressed,held)?;
 if choice==crate::original_action_choice::Choice::Offered(59){original_ring_dash::enter(&mut next.entity,&mut next.character,math)?;}
 *state=next;Ok(choice)
}
impl RingMotion{
 /// Retain an already-entered state without resetting timers or pickup history.
 pub fn from_active(state:PositionState)->Result<Self>{
  if state.entity.len()!=0x40||state.motion.len()!=0x40||state.character.len()!=0x3a0||state.entity[0]!=59{return Err("ring host requires entered action59 records".into());}
  Ok(Self{state,special_mode:0})
 }

 pub fn enter(mut state:PositionState,math:&OriginalMath)->Result<Self>{original_ring_dash::enter(&mut state.entity,&mut state.character,math)?;Ok(Self{state,special_mode:0})}
 /// Timer phase after higher-priority interrupt handling; true requests sound stop.
 /// Caller supplies the original input-direction class used by expiry.
 pub fn decide_timeout(&mut self,direction_class:i32,math:&OriginalMath)->Result<bool>{
  if self.state.entity.first()!=Some(&59){return Err("ring timer requires live action59".into());}
  let mut next=self.clone();let expired=original_ring_dash::advance_timer(&mut next.state.character)?;
  if expired{original_ring_dash::expire(&mut next.state.entity,&mut next.state.character,direction_class,math)?;}
  *self=next;Ok(expired)
 }
 /// Timer phase with original controller classification, queried only on expiry.
 /// A rejected input/memory query leaves both timers and cached records unchanged.
 pub fn decide_timeout_with_control(&mut self,ctx:&crate::original_control::ControlContext,math:&OriginalMath)->Result<bool>{
  if self.state.entity.first()!=Some(&59){return Err("ring timer requires live action59".into());}
  let mut next=self.clone();let s=&mut next.state;
  let expired=original_ring_dash::advance_timer(&mut s.character)?;
  if expired{
   let memory=crate::original_contact::Memory{base:s.memory.base,bytes:&s.memory.bytes};
   let class=crate::original_direction_class::classify(&s.entity,&s.character,ctx,&memory,&mut s.control_cache,math)?;
   original_ring_dash::expire(&mut s.entity,&mut s.character,class,math)?;
  }
  *self=next;Ok(expired)
 }
 /// Resolved decision-phase contact; host owns overlap discovery and pickup events.
 pub fn contact(&mut self,category:u8,position:[f32;3],math:&OriginalMath)->Result<bool>{
  if self.state.entity.first()!=Some(&59){return Err("ring contact requires live action59".into());}
  let mut next=self.clone();
  let stop=original_ring_dash::contact(&mut next.state.entity,&mut next.state.character,category,position,math)?;
  *self=next;Ok(stop)
 }
 pub fn records(&self)->&PositionState{&self.state}
 pub fn into_records(self)->PositionState{self.state}
 /// Source80084930 physics phase only. Caller runs action decisions first.
 /// External side effects in callbacks cannot be rolled back; mutate memory only.
 pub fn physics<P,F>(&mut self,center:[f32;3],rings:&[RingCandidate],fallback_speed:f32,world:&CandidateWorld,ctx:&PositionBContext,math:&OriginalMath,prepare:P,callback:F)->Result<RingPhysicsOutput>
 where P:FnOnce(&mut PlatformMemory)->Result<()>,F:FnMut(u32,u32,u32,u32,&mut PlatformMemory)->Result<()>{
  if self.state.entity.first()!=Some(&59){return Err("ring physics requires live action59".into());}
  let mut next=self.clone();prepare(&mut next.state.memory)?;
  let s=&mut next.state;let control=&ctx.common.control;
  let selected=original_ring_dash::advance_motion(&mut s.entity,&mut s.motion,&mut s.character,&mut s.workspace,ctx.common.workspace_identity,center,rings,control.gravity,control.magnitudes,fallback_speed,math)?;
  let response=crate::original_collision_loop::update(s,&mut next.special_mode,world,ctx,math,callback)?;
  let stop_sound=original_ring_dash::finish_collision(&mut s.entity,&mut s.motion,&mut s.character,response,control.gravity,control.level,fallback_speed,math)?;
  *self=next;Ok(RingPhysicsOutput{selected,collision_response:response,stop_sound,reset_requested:response==2})
 }
}
