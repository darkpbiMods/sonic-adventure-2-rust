//! Upright airborne adapter for recovered fallback forces and optional source terrain.
//! Host owns action decisions; object contacts and moving platforms require another adapter.
use crate::{disc::Result,original_math::OriginalMath,original_ground::rotate_vector,original_contact::Memory,original_control::ControlContext,original_ground_camera::CameraGroundContext,original_ground_accel::GroundContext,original_homing_fallback,sonic::Physics};
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn vec_put(b:&mut[u8],i:usize,v:[f32;3]){for j in 0..3{put(b,i+j*4,v[j].to_bits());}}
fn vec_get(b:&[u8],i:usize)->[f32;3]{std::array::from_fn(|j|f32::from_be_bytes(b[i+j*4..i+j*4+4].try_into().unwrap()))}
fn angle(yaw:f32)->u32{((std::f32::consts::FRAC_PI_2-yaw)*(65536./std::f32::consts::TAU))as i32 as u32&0xffff}
pub struct AirFallback{entity:[u8;0x40],motion:[u8;0x40],character:[u8;0x3a0],memory:[u8;0x40],cache:[u32;4],first:bool,done:bool,skip_decision:bool}
pub struct AirFallbackOutput{pub velocity:[f32;3],pub yaw:f32,pub expired:bool,pub counter:i16}
pub struct AirFallbackContact{pub position:[f32;3],pub velocity:[f32;3],pub grounded:bool}
impl AirFallback{
 /// Continue an existing action13 using its collision and motion history.
 /// This imports records; the caller separately specifies the selection outcome.
 pub fn from_collision(state:&crate::original_position_a::PositionState)->Result<Self>{
  if state.entity.len()!=0x40||state.motion.len()!=0x40||state.character.len()!=0x3a0||state.workspace.len()!=0x84||state.entity[0]!=13{return Err("invalid fallback continuation records".into());}
  // The force adapter exposes this workspace at the same identity each tick.
  if u32::from_be_bytes(state.character[0x90..0x94].try_into().unwrap())!=0x80210000{return Err("unsupported fallback workspace identity".into());}
  Physics::decode(&state.character[0xc0..0x144])?;
  Ok(Self{entity:state.entity.as_slice().try_into().unwrap(),motion:state.motion.as_slice().try_into().unwrap(),character:state.character.as_slice().try_into().unwrap(),memory:state.workspace[..0x40].try_into().unwrap(),cache:state.control_cache,first:false,done:false,skip_decision:false})
 }
 pub fn source_records(&self)->(&[u8],&[u8],&[u8]){(&self.entity,&self.motion,&self.character)}
 /// Contact after fallback forces, with source flags/counters retained.
 pub fn resolve_source_terrain(&mut self,state:&mut crate::original_position_a::PositionState,world:&crate::original_terrain_candidates::CandidateWorld,ctx:&crate::original_position_b::PositionBContext,math:&OriginalMath)->Result<AirFallbackContact>{
  self.resolve_source_terrain_with_platform(state,world,ctx,math,|_,_,_,_,_|Err("fallback moving-platform adapter required".into()))
 }
 pub fn resolve_source_terrain_with_platform<F>(&mut self,state:&mut crate::original_position_a::PositionState,world:&crate::original_terrain_candidates::CandidateWorld,ctx:&crate::original_position_b::PositionBContext,math:&OriginalMath,callback:F)->Result<AirFallbackContact>
 where F:FnMut(u32,u32,u32,u32,&mut crate::original_platform::PlatformMemory)->Result<()> {

  let mut next=state.clone();next.entity=self.entity.to_vec();next.motion=self.motion.to_vec();next.character=self.character.to_vec();next.control_cache=self.cache;
  // Position B consumes the previous collision's support status before replacing
  // it with this frame's flags. A fresh action adapter must not erase takeoff
  // history, or its first upward dash can be classified as a new floor contact.
  if state.character.len()!=0x3a0{return Err("invalid fallback prior collision records".into());}
  next.character[0x1c..0x1e].copy_from_slice(&state.character[0x1c..0x1e]);
  // The demo adapter has no object contacts; a real empty collider graph is
  // required by45880 even when only terrain geometry is supplied.
  let collider=0x80213000_u32;
  let offset=collider.checked_sub(next.memory.base).ok_or("fallback collider below memory")?as usize;
  let end=offset.checked_add(0xb0).filter(|&end|end<=next.memory.bytes.len()).ok_or("fallback collider outside memory")?;
  next.memory.bytes[offset..end].fill(0);next.memory.bytes[offset+0x11]=255;
  put(&mut next.entity,0x2c,collider);
  crate::original_attack_contact::advance(&mut next,&mut 0,world,ctx,math,callback)?;
  let position=vec_get(&next.entity,0x14).map(|v|v*0.1);let velocity=vec_get(&next.motion,0).map(|v|v*6.);
  if position.iter().chain(velocity.iter()).any(|v|!v.is_finite()){return Err("nonfinite fallback terrain output".into());}
  let grounded=u16::from_be_bytes(next.entity[4..6].try_into().unwrap())&3!=0;
  self.entity.copy_from_slice(&next.entity);self.motion.copy_from_slice(&next.motion);self.character.copy_from_slice(&next.character);self.memory.copy_from_slice(&next.workspace[..0x40]);self.cache=next.control_cache;
  *state=next;Ok(AirFallbackContact{position,velocity,grounded})
 }
 pub fn new(physics:&[u8],position:[f32;3],velocity:[f32;3],yaw:f32,math:&OriginalMath)->Result<Self>{
  Physics::decode(physics)?;
  if position.iter().chain(velocity.iter()).any(|v|!v.is_finite())||!yaw.is_finite(){return Err("nonfinite airborne host state".into());}
  let mut s=Self{entity:[0;0x40],motion:[0;0x40],character:[0;0x3a0],memory:[0;0x40],cache:[0;4],first:true,done:false,skip_decision:false};
  s.entity[0]=13;put(&mut s.entity,12,angle(yaw));vec_put(&mut s.entity,0x14,position.map(|v|v*10.));
  s.character[0xc0..0x144].copy_from_slice(physics);put(&mut s.character,0x34,1f32.to_bits());put(&mut s.character,0x40,1f32.to_bits());
  put(&mut s.character,0x90,0x80210000);vec_put(&mut s.memory,0x1c,[0.,1.,0.]);vec_put(&mut s.character,0x7c,[0.,1.,0.]);
  let world=velocity.map(|v|v/6.);vec_put(&mut s.motion,0,world);vec_put(&mut s.character,0x64,rotate_vector(&s.entity,world,false,math)?);
  s.character[0x364..0x366].copy_from_slice(&(-1i16).to_be_bytes());Ok(s)
 }
 /// Existing pursuit loses its candidate list; source364was0 on successful pursuit.
 pub fn after_target_loss(mut self,empty_list:bool,total:u16)->Self{
  self.first=!empty_list;
  self.character[0x364..0x366].copy_from_slice(&0i16.to_be_bytes());
  self.character[0x366..0x368].copy_from_slice(&(total as i16).to_be_bytes());self
 }
 /// Startup already armed counter1; same-frame physics has no new decision pass.
 pub fn after_upward_start(self)->Self{self.after_air_dash_start(false)}
 /// Source8007b9a0..c8 / b9ec..f4: override only local forward speed.
 /// Preserve world motion until the ordinary force/integrator phase, as source does.
 pub fn after_air_dash_start(mut self,alternate:bool)->Self{
  self.first=false;self.skip_decision=true;
  put(&mut self.character,0x64,if alternate{8f32}else{5f32}.to_bits());
  self.character[0x364..0x366].copy_from_slice(&1i16.to_be_bytes());self
 }
 pub fn tick(&mut self,position:[f32;3],stick:[f32;2],math:&OriginalMath)->Result<AirFallbackOutput>{
  if position.iter().chain(stick.iter()).any(|v|!v.is_finite()){return Err("nonfinite airborne host input".into());}
  self.decide_no_contact()?;
  self.tick_physics(position,stick,math)
 }
 /// Decision phase, before landing selection or any current-frame forces.
 pub fn timeout_due(&self)->bool{
  !self.skip_decision&&(self.done||i16::from_be_bytes(self.character[0x364..0x366].try_into().unwrap())>=15||i16::from_be_bytes(self.character[0x366..0x368].try_into().unwrap())>=360)
 }
 /// Decision phase, before landing selection or any current-frame forces.
 /// Call exactly once per frame; startup consumes its deliberate skip here.
 pub fn decide_no_contact(&mut self)->Result<bool>{
  let expired=if self.skip_decision{self.skip_decision=false;false}else{self.done||original_homing_fallback::expire_no_contact(&mut self.entity,&mut self.character)?};
  self.done=expired;Ok(expired)
 }
 /// Physics phase after the caller has chosen to remain in action13.
 /// Does not advance decision counters; an expired adapter produces no forces.
 pub fn tick_physics(&mut self,position:[f32;3],stick:[f32;2],math:&OriginalMath)->Result<AirFallbackOutput>{
  if position.iter().chain(stick.iter()).any(|v|!v.is_finite()){return Err("nonfinite airborne host input".into());}
  // Keep source precision when the host returns our own projected position.
  // Repeated source->host->source scaling otherwise introduces rounding drift.
  // A genuinely changed host position still represents a teleport/correction.
  let projected=vec_get(&self.entity,0x14).map(|v|v*0.1);
  if position.map(f32::to_bits)!=projected.map(f32::to_bits){vec_put(&mut self.entity,0x14,position.map(|v|v*10.));}
  let expired=self.done;
  if !expired{
   let magnitude=stick[0].hypot(stick[1]).min(1.);
   let direction=angle(stick[0].atan2(stick[1]));
   let control=ControlContext{directions:[direction,0],magnitudes:[magnitude,0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0};
   let camera=CameraGroundContext{control:&control,stick_x:[0;2],cameras:[0;2],incoming_f28:0.,path_scratch:[0;0x44]};
   let ctx=GroundContext{camera,ordinary_path_scratch:[0;0x44]};let memory=Memory{base:0x80210000,bytes:&self.memory};
   if self.first{original_homing_fallback::begin_selection_failure(&mut self.entity,&mut self.motion,&mut self.character,&ctx,&memory,&mut self.cache,math)?;self.first=false;}
   else{original_homing_fallback::advance(&mut self.entity,&mut self.motion,&mut self.character,&ctx,&memory,&mut self.cache,math)?;}
  }
  let yaw=u32::from_be_bytes(self.entity[12..16].try_into().unwrap())as f32*std::f32::consts::TAU/65536.;
  Ok(AirFallbackOutput{velocity:vec_get(&self.motion,0).map(|v|v*6.),yaw:std::f32::consts::FRAC_PI_2-yaw,expired,counter:i16::from_be_bytes(self.character[0x364..0x366].try_into().unwrap())})
 }
}
