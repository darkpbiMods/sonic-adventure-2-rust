//! Upright flat-ground force bridge. Host owns collision and position integration.
use crate::{disc::Result,sonic::Physics,original_math::OriginalMath,original_contact::Memory,original_control::ControlContext,original_ground_camera::CameraGroundContext,original_ground_accel::{GroundContext,accelerate},original_movement::{integrate_velocity,reconcile_velocity}};
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn vector(b:&mut[u8],i:usize,v:[f32;3]){for j in 0..3{put(b,i+j*4,v[j].to_bits());}}
fn angle(yaw:f32)->u32{((std::f32::consts::FRAC_PI_2-yaw)*(65536./std::f32::consts::TAU))as i32 as u32&0xffff}
#[derive(Clone)]
pub struct GroundMotion{entity:[u8;0x40],motion:[u8;0x40],character:[u8;0x3a0],memory:[u8;0x84],cache:[u32;4],surface:Option<[f32;3]>}
pub struct GroundOutput{pub velocity:[f32;3],pub yaw:f32}
pub struct TerrainOutput{
 pub position:[f32;3],pub velocity:[f32;3],pub yaw:f32,pub grounded:bool,
 /// Original query normal, including source numerical error; not normalized.
 pub source_normal:[f32;3],pub response:u32,
}
pub struct LocomotionAnimation{pub selected:u16,pub forward_speed:f32,pub adjusted_speed:f32,pub lean:u32}
impl GroundMotion{
 /// Source80080780, after action70 landing/braking selection.
 pub fn moving_landing(&mut self,historical_yaw:Option<u32>,math:&OriginalMath)->Result<()>{
  let mut next=self.clone();
  reconcile_velocity(&mut next.entity,&mut next.motion,&mut next.character,[0.,-1.,0.],0,math)?;
  crate::original_landing_selection::moving_records(&mut next.entity,&next.motion,&mut next.character,historical_yaw,math)?;
  *self=next;Ok(())
 }
 /// Source80080780, after action70 landing/braking selection.
 pub fn finish_bounce_landing(&mut self){
  let flags=u16::from_be_bytes(self.entity[4..6].try_into().unwrap())&!0x500;
  self.entity[4..6].copy_from_slice(&flags.to_be_bytes());
 }
 pub fn reconciled_terrain_velocity(&mut self,math:&OriginalMath)->Result<[f32;3]>{
  let mut next=self.clone();
  reconcile_velocity(&mut next.entity,&mut next.motion,&mut next.character,[0.,-1.,0.],0,math)?;
  let local=std::array::from_fn(|i|f32::from_be_bytes(next.character[0x64+i*4..0x68+i*4].try_into().unwrap()));
  let world=crate::original_ground::rotate_vector(&next.entity,local,true,math)?.map(|v|v*6.);
  *self=next;Ok(world)
 }
 /// Resume a source collision state without resetting action, heading or timers.
 pub fn from_collision_state(state:&crate::original_position_a::PositionState)->Result<Self>{
  if state.entity.len()!=0x40||state.motion.len()!=0x40||state.character.len()!=0x3a0||state.workspace.len()!=0x84{return Err("invalid collision controller records".into());}
  let mut next=Self::new(&state.character[0xc0..0x144])?;
  next.entity.copy_from_slice(&state.entity);next.motion.copy_from_slice(&state.motion);next.character.copy_from_slice(&state.character);next.memory.copy_from_slice(&state.workspace);next.cache=state.control_cache;
  Ok(next)
 }
 /// Run after source force integration, before host position integration.
 /// The world and persistent collision storage are supplied by the host.
 /// Both controller and collision state remain unchanged on any failure.
 /// Moving-platform callbacks require a separate host adapter and are rejected.
 pub fn resolve_source_terrain(&mut self,collision:&mut crate::original_position_a::PositionState,world:&crate::original_terrain_candidates::CandidateWorld,ctx:&crate::original_position_b::PositionBContext,math:&OriginalMath)->Result<TerrainOutput>{
  self.resolve_source_terrain_with_platform(collision,world,ctx,math,|_,_,_,_,_|Err("moving-platform callback requires host integration".into()))
 }
 /// Supply the host's implementation of an original platform callback.
 /// Arguments are callback address, player task, entity address, slot address,
 /// and transactional guest memory. Mutate only that memory: external callback
 /// side effects cannot be rolled back if a later collision operation fails.
 /// Controller and collision records commit only after the whole step succeeds.
 pub fn resolve_source_terrain_with_platform<F>(&mut self,collision:&mut crate::original_position_a::PositionState,world:&crate::original_terrain_candidates::CandidateWorld,ctx:&crate::original_position_b::PositionBContext,math:&OriginalMath,callback:F)->Result<TerrainOutput>
 where F:FnMut(u32,u32,u32,u32,&mut crate::original_platform::PlatformMemory)->Result<()> {

  let mut next=collision.clone();
  next.entity=self.entity.to_vec();next.motion=self.motion.to_vec();next.character=self.character.to_vec();next.workspace=self.memory.to_vec();next.control_cache=self.cache;
  let mut special=0;
  let response=crate::original_position_b::update(&mut next,&mut special,world,ctx,math,callback)?;
  let read=|b:&[u8],o:usize|->f32{f32::from_be_bytes(b[o..o+4].try_into().unwrap())};
  let position=std::array::from_fn(|i|read(&next.entity,0x14+i*4)/10.);
  let velocity=std::array::from_fn(|i|read(&next.motion,i*4)*6.);
  let source_normal=std::array::from_fn(|i|read(&next.character,0x7c+i*4));
  let yaw=std::f32::consts::FRAC_PI_2-u32::from_be_bytes(next.entity[12..16].try_into().unwrap())as f32*(std::f32::consts::TAU/65536.);
  if !position.iter().chain(velocity.iter()).chain(source_normal.iter()).all(|v:&f32|v.is_finite())||!yaw.is_finite(){return Err("nonfinite terrain host output".into());}
  let grounded=next.entity[5]&1!=0;
  self.entity.copy_from_slice(&next.entity);self.motion.copy_from_slice(&next.motion);self.character.copy_from_slice(&next.character);self.memory.copy_from_slice(&next.workspace);self.cache=next.control_cache;
  *collision=next;
  Ok(TerrainOutput{position,velocity,yaw,grounded,source_normal,response})
 }
 /// Transfer ordinary airborne records after host-confirmed support contact.
 pub fn from_landed_collision(state:&crate::original_position_a::PositionState)->Result<Self>{
  if state.entity.len()!=0x40||state.entity[5]&1==0{return Err("landing requires source support".into());}
  let mut next=Self::from_collision_state(state)?;
  next.entity[0]=1;
  Ok(next)
 }
 /// Transfer ordinary airborne records after host-confirmed support contact.
 /// The caller still supplies resolved contact velocity/normal before forces.
 /// Synthetic workspace is rebound here; original collision discovery is external.
 pub fn from_air_records(entity:&[u8],motion:&[u8],character:&[u8])->Result<Self>{
  if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0{return Err("invalid landing records".into());}
  let mut s=Self::new(&character[0xc0..0x144])?;
  s.entity.copy_from_slice(entity);s.motion.copy_from_slice(motion);s.character.copy_from_slice(character);
  s.entity[0]=1;s.entity[5]=(s.entity[5]&!3)|1;
  put(&mut s.character,0x90,0x80210000);
  Ok(s)
 }
 /// Decide ordinary stationary landing from retained collision-time records.
 /// Motion is deliberately not replaced with the host's zeroed vertical speed.
 /// Host must supply records appropriate to its contact response; this does not
 /// implement terrain collision, reverse-input braking or special action exits.
 pub fn stationary_landing_selection(&mut self,magnitudes:[f32;2],math:&OriginalMath)->Result<Option<u16>>{
  let mut next=self.clone();
  reconcile_velocity(&mut next.entity,&mut next.motion,&mut next.character,[0.,-1.,0.],0,math)?;
  if !crate::original_landing_selection::ordinary_stop(&mut next.entity,&mut next.character,magnitudes)?{return Ok(None);}
  let velocity=std::array::from_fn(|i|f32::from_be_bytes(next.motion[i*4..i*4+4].try_into().unwrap()));
  let selected=crate::original_landing_selection::select(velocity,math)?;
  next.set_animation_selection(selected);*self=next;Ok(Some(selected))
 }
 /// Stationary ordinary landing with host-owned idle timing and random state.
 pub fn stationary_landing_with_context(&mut self,magnitudes:[f32;2],frame:u32,seed:&mut u32,context_draw:bool,math:&OriginalMath)->Result<Option<u16>>{
  let mut next=self.clone();let mut next_seed=*seed;
  reconcile_velocity(&mut next.entity,&mut next.motion,&mut next.character,[0.,-1.,0.],0,math)?;
  let selected=crate::original_landing_selection::stationary_records(&mut next.entity,&next.motion,&mut next.character,magnitudes,[0.,-1.,0.],frame,&mut next_seed,context_draw,math)?;
  if selected.is_some(){*self=next;*seed=next_seed;}
  Ok(selected)
 }
 /// Original8007fc6c..fca8 landing branch, before stationary selection.
 pub fn rebound_landing_with_context(&mut self,magnitudes:[f32;2],frame:u32,seed:&mut u32,context_draw:bool,math:&OriginalMath)->Result<Option<u16>>{
  let mut next=self.clone();let mut next_seed=*seed;
  reconcile_velocity(&mut next.entity,&mut next.motion,&mut next.character,[0.,-1.,0.],0,math)?;
  let selected=crate::original_landing_selection::rebound_records(&mut next.entity,&next.motion,&mut next.character,magnitudes,[0.,-1.,0.],frame,&mut next_seed,context_draw,math)?;
  if selected.is_some(){*self=next;*seed=next_seed;}
  Ok(selected)
 }
 /// Original8007fc6c..fca8 landing branch, before stationary selection.
 /// No ground-force step is performed by this decision.
 pub fn landing_brake(&mut self,stick:[f32;2],math:&OriginalMath)->Result<bool>{
  if !stick.iter().all(|v|v.is_finite()){return Err("nonfinite landing stick".into());}
  let mut next=self.clone();
  reconcile_velocity(&mut next.entity,&mut next.motion,&mut next.character,[0.,-1.,0.],0,math)?;
  let control=ControlContext{directions:[angle(stick[0].atan2(stick[1])),0],magnitudes:[stick[0].hypot(stick[1]).min(1.),0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0};
  let memory=Memory{base:0x80210000,bytes:&next.memory};
  let reverse=crate::original_reverse_input::check(&next.entity,&next.character,&control,&memory,&mut next.cache,math)?;
  let speed=f32::from_be_bytes(next.character[0x64..0x68].try_into().unwrap());
  if !reverse||speed<=0.{return Ok(false);}
  next.entity[0]=12;next.set_animation_selection(18);next.character[0x17c..0x17e].fill(0);
  *self=next;Ok(true)
 }
 /// Normal-surface Sonic idle selection after final contact reconciliation.
 /// Caller owns global frame/RNG and must exclude slope/context-specific branches.
 /// Returns whether the host should emit the contextual idle-entry output.
 pub fn reset_idle_counter(&mut self){self.character[0x18..0x1a].fill(0);}
 pub fn idle_animation(&mut self,frame:u32,seed:&mut u32,context_draw:bool)->bool{
  let mut idle=crate::original_idle_selection::IdleState{
   selected:u16::from_be_bytes(self.character[0x176..0x178].try_into().unwrap()),
   counter:i16::from_be_bytes(self.character[0x18..0x1a].try_into().unwrap())};
  let entered=crate::original_idle_selection::advance(&mut idle,seed,frame,context_draw);
  self.character[0x176..0x178].copy_from_slice(&idle.selected.to_be_bytes());
  self.character[0x18..0x1a].copy_from_slice(&idle.counter.to_be_bytes());
  if entered{self.character[0x17c..0x17e].fill(0);}
  entered
 }
 /// Surface-aware idle update; refresh support without advancing alignment twice.
 pub fn idle_surface_animation(&mut self,frame:u32,seed:&mut u32,context_draw:bool,math:&OriginalMath)->Result<bool>{
  let mut next=self.clone();let mut next_seed=*seed;
  if let Some(normal)=next.surface{vector(&mut next.character,0x7c,normal);}
  let entered=crate::original_idle_slope::advance(&next.entity,&next.motion,&mut next.character,[0.,-1.,0.],frame,&mut next_seed,context_draw,math)?;
  *self=next;*seed=next_seed;Ok(entered)
 }
 /// The animation updater can change the requested ID on direct completion.
 pub fn set_animation_selection(&mut self,selected:u16){self.character[0x176..0x178].copy_from_slice(&selected.to_be_bytes());}
 /// Final host collision result -> source reconciliation -> animation selection.
 /// No forces or integration run here. Errors leave persistent records intact.
 pub fn finish_locomotion_contact(&mut self,position:[f32;3],velocity:[f32;3],yaw:f32,historical_yaw:Option<u32>,math:&OriginalMath)->Result<LocomotionAnimation>{
  if position.iter().chain(velocity.iter()).any(|v|!v.is_finite())||!yaw.is_finite(){return Err("nonfinite final contact".into());}
  let mut next=self.clone();
  vector(&mut next.entity,0x14,position.map(|v|v*10.));put(&mut next.entity,12,angle(yaw));
  vector(&mut next.motion,0,velocity.map(|v|v/6.));
  if let Some(normal)=next.surface{vector(&mut next.character,0x7c,normal);vector(&mut next.memory,0x1c,normal);}
  // Source845f0 reconciles after collision; it does not perform a second
  // terrain-alignment update. Repeating alignment changes slope orientation.
  reconcile_velocity(&mut next.entity,&mut next.motion,&mut next.character,[0.,-1.,0.],0,math)?;
  let output=next.locomotion_animation(historical_yaw,math)?;*self=next;Ok(output)
 }
 /// Execute after final contact reconciliation, before animation update/history.
 /// Rates remain in source units; do not replace them with host world magnitude.
 pub fn locomotion_animation(&mut self,historical_yaw:Option<u32>,math:&OriginalMath)->Result<LocomotionAnimation>{
  crate::original_locomotion_selection::advance(&self.entity,&mut self.character,historical_yaw,math)?;
  let word=|i:usize|u32::from_be_bytes(self.character[i..i+4].try_into().unwrap());
  Ok(LocomotionAnimation{selected:u16::from_be_bytes(self.character[0x176..0x178].try_into().unwrap()),forward_speed:f32::from_bits(word(0x64)),adjusted_speed:f32::from_bits(word(0x44)),lean:word(0x2c)})
 }
 pub fn into_fall(self)->Result<crate::air_motion_host::AirMotion>{
  crate::air_motion_host::AirMotion::from_fall_records(&self.entity,&self.motion,&self.character)
 }
 /// Columns mapping the baked demo model's axes into source world space.
 pub fn model_basis(&self,math:&OriginalMath)->Result<[[f32;3];3]>{
  let rotate=|v|crate::original_ground::rotate_vector(&self.entity,v,true,math);
  Ok([rotate([0.,0.,-1.])?,rotate([0.,1.,0.])?,rotate([1.,0.,0.])?])
 }
 /// Experimental settled-plane contact adapter; host supplies an upward unit
 /// normal. This does not reproduce collision classification or contact search.
 pub fn set_surface_normal(&mut self,normal:[f32;3])->Result<()>{
  if normal[1]<=0.{return Err("ground contact requires upward unit normal".into());}
  self.set_experimental_surface_normal(normal)
 }
 /// Diagnostic full-sphere plane adapter. Host must separately establish support
 /// and implement loss of adhesion; accepting a normal does not establish either.
 /// The playable source-terrain host uses retained collision normals here.
 pub fn set_experimental_surface_normal(&mut self,normal:[f32;3])->Result<()>{
  let length=normal.iter().map(|v|v*v).sum::<f32>();
  if !normal.iter().all(|v|v.is_finite())||(length-1.).abs()>0.0001{return Err("ground contact requires finite unit normal".into());}
  self.surface=Some(normal);Ok(())
 }
 /// Refresh host-editable parameters without discarding persistent force state.
 pub fn configure(&mut self,physics:&[u8],ready:bool)->Result<()>{
  Physics::decode(physics)?;
  self.character[0xc0..0x144].copy_from_slice(physics);
  //80076474..78 changes a force parameter, not instantaneous velocity.
  // Copying the host table above restores its ordinary value on ready exit.
  self.entity[4]=if ready{2}else{0};
  if ready{put(&mut self.character,0xd0,0.5_f32.to_bits());}
  Ok(())
 }
 /// Read-only original records for differential diagnostics; not host collision state.
 pub fn source_records(&self)->(&[u8],&[u8],&[u8]){(&self.entity,&self.motion,&self.character)}
 pub fn source_workspace(&self)->&[u8]{&self.memory}
 pub fn new(physics:&[u8])->Result<Self>{
  Physics::decode(physics)?;
  let mut s=Self{entity:[0;0x40],motion:[0;0x40],character:[0;0x3a0],memory:[0;0x84],cache:[0;4],surface:None};
  s.entity[0]=1;s.entity[5]=1;
  s.character[0xc0..0x144].copy_from_slice(physics);
  put(&mut s.character,0x34,1f32.to_bits());put(&mut s.character,0x40,1f32.to_bits());
  put(&mut s.character,0x90,0x80210000);vector(&mut s.memory,0x1c,[0.,1.,0.]);vector(&mut s.character,0x7c,[0.,1.,0.]);
  Ok(s)
 }
 pub fn new_at_yaw(physics:&[u8],yaw:f32)->Result<Self>{
  if !yaw.is_finite(){return Err("nonfinite initial ground heading".into());}
  let mut s=Self::new(physics)?;put(&mut s.entity,12,angle(yaw));put(&mut s.motion,0x1c,angle(yaw));Ok(s)
 }
 fn prepare_contact(&mut self,position:[f32;3],velocity:[f32;3],yaw:f32,math:&OriginalMath)->Result<()>{
  if position.iter().chain(velocity.iter()).any(|v|!v.is_finite())||!yaw.is_finite(){return Err("nonfinite ground host input".into());}
  vector(&mut self.entity,0x14,position.map(|v|v*10.));put(&mut self.entity,12,angle(yaw));
  let world=velocity.map(|v|v/6.);vector(&mut self.motion,0,world);
  if let Some(normal)=self.surface{
   let [pitch,roll]=math.surface_angles(normal)?;
   let mut workspace=[0;0x60];workspace[0x1c..0x28].copy_from_slice(&self.memory[0x1c..0x28]);
   vector(&mut workspace,0x48,normal);put(&mut workspace,0x34,pitch);put(&mut workspace,0x38,roll);
   //8003f184..f1c8: settled contact can retain orientation rather than
   // approaching the surface again. With fixed downward gravity the dot
   // comparison reduces to new Y <= previous Y; vertical speed is irrelevant.
   let previous_y=f32::from_be_bytes(self.memory[0x20..0x24].try_into().unwrap());
   let settled=world[0]==0.&&world[2]==0.&&self.character[0x1d]&1!=0&&normal[1]<=previous_y;
   if !settled{crate::original_terrain_align::align(&mut self.entity,&self.character,&workspace,0,[0.,-1.,0.],math)?;}
   // Host initialization specialization: a new controller snaps to its plane.
   // Full original collision first copies entity flags into character status;
   // equivalence of these initial conditions is not established here.
   self.character[0x1d]=1;
   put(&mut self.motion,0x18,pitch);put(&mut self.motion,0x20,roll);
   vector(&mut self.character,0x7c,normal);vector(&mut self.memory,0x1c,normal);
  }
  // Host velocity is already collision-resolved. The original finish step also
  // clears grounded local Y, suppresses tiny speeds and updates slope/gravity.
  reconcile_velocity(&mut self.entity,&mut self.motion,&mut self.character,[0.,-1.,0.],0,math)?;
  Ok(())
 }
 /// Consume the current contact controller without running another ground-force
 /// step. Returned air state has already executed its takeoff physics once.
 pub fn jump(self,position:[f32;3],velocity:[f32;3],yaw:f32,stick:[f32;2],math:&OriginalMath)->Result<crate::air_motion_host::AirMotion>{
  self.jump_impl(position,velocity,yaw,stick,None,math)
 }
 /// Attached counterpart of jump. The memory supplies source task/object/slot
 /// records; takeoff physics is executed once and retains the launch impulse.
 pub fn jump_with_platform(self,position:[f32;3],velocity:[f32;3],yaw:f32,stick:[f32;2],memory:&crate::original_contact::Memory,math:&OriginalMath)->Result<crate::air_motion_host::AirMotion>{
  self.jump_impl(position,velocity,yaw,stick,Some(memory),math)
 }
 fn jump_impl(mut self,position:[f32;3],velocity:[f32;3],yaw:f32,stick:[f32;2],memory:Option<&crate::original_contact::Memory>,math:&OriginalMath)->Result<crate::air_motion_host::AirMotion>{
  if !stick.iter().all(|v|v.is_finite()){return Err("nonfinite jump stick".into());}
  self.prepare_contact(position,velocity,yaw,math)?;
  let context=crate::original_movement::AirContext{directions:[angle(stick[0].atan2(stick[1])),0],magnitudes:[stick[0].hypot(stick[1]).min(1.),0.],gravity:[0.,-1.,0.],level:0,control_mode:0,has_direction_surface:false};
  match memory {
   Some(memory)=>crate::air_motion_host::AirMotion::from_ground_with_platform(&self.entity,&self.motion,&self.character,memory,&context,math),
   None=>crate::air_motion_host::AirMotion::from_ground(&self.entity,&self.motion,&self.character,&context,math),
  }
 }
 /// Experimental rolling force composition. The caller supplies the original
 /// attack flag explicitly; action entry/exit and collision remain outside.
 pub fn tick_roll(&mut self,position:[f32;3],velocity:[f32;3],yaw:f32,stick:[f32;2],attack:bool,math:&OriginalMath)->Result<GroundOutput>{
  if !stick.iter().all(|v|v.is_finite()){return Err("nonfinite roll stick".into());}
  self.entity[0]=4;self.entity[4]=if attack{1}else{0};
  self.prepare_contact(position,velocity,yaw,math)?;
  self.rolling_forces(stick,math)
 }
 /// Original8007f2ac..8007f2c8 button-driven uncurl; no impulse is added.
 pub fn cancel_roll(&mut self){self.entity[0]=1;self.character[0x176..0x178].copy_from_slice(&26_u16.to_be_bytes());let flags=u16::from_be_bytes(self.entity[4..6].try_into().unwrap())&!0x500;self.entity[4..6].copy_from_slice(&flags.to_be_bytes());}
 /// Demo adapter for source somersault entry and force phases. Contextual
 /// target steering (4d7fc) is host policy; this adapter holds its entry heading.
 pub fn somersault_frame(&mut self,position:[f32;3],velocity:[f32;3],yaw:f32,entry:bool,recovering:bool,stage:u8,advance:bool,replay:bool,math:&OriginalMath)->Result<GroundOutput>{
  let mut next=self.clone();
  next.prepare_contact(position,velocity,yaw,math)?;
  if entry{
   crate::original_somersault_entry::enter(&mut next.entity,&mut next.character)?;
  }
  if replay{next.set_animation_selection(96);put(&mut next.character,0x64,math.constant(0x803af680).to_bits());}
  if !recovering&&advance{
   //77c64/783fc: queued action62/63 retains the animation continuation.
   if !(2..=3).contains(&stage){return Err("invalid somersault combo stage".into());}
   let fast=matches!(next.entity[0],66..=68);
   next.entity[0]=if fast{65+stage}else{60+stage};next.character[0x366..0x368].fill(0);
   let impulse=if fast{if stage==2{0x803af684}else{0x803af5f8}}else{0x803af640};
   put(&mut next.character,0x64,math.constant(impulse).to_bits());
  }
  if recovering{
   next.entity[0]=64;next.entity[4]&=!4;
   crate::original_brake_force::accelerate(&mut next.entity,&next.motion,&mut next.character,[0.,-1.,0.],[0.;2],math)?;
  }else{
   //80048724: external force plus gravity, rotated to local coordinates.
   crate::original_somersault_force::accelerate(&next.entity,&next.motion,&mut next.character,[0.,-1.,0.],math)?;
  }
  integrate_velocity(&next.entity,&mut next.motion,&mut next.character,[0.;2],math)?;
  let velocity=std::array::from_fn(|i|f32::from_be_bytes(next.motion[i*4..i*4+4].try_into().unwrap())*6.);
  let yaw=std::f32::consts::FRAC_PI_2-u32::from_be_bytes(next.entity[12..16].try_into().unwrap())as f32*std::f32::consts::TAU/65536.;
  *self=next;Ok(GroundOutput{velocity,yaw})
 }
 pub fn roll_frame(&mut self,position:[f32;3],velocity:[f32;3],yaw:f32,stick:[f32;2],math:&OriginalMath)->Result<(GroundOutput,bool)>{
  if !stick.iter().all(|v|v.is_finite()){return Err("nonfinite roll stick".into());}
  self.entity[0]=4;self.entity[4]=5;
  self.prepare_contact(position,velocity,yaw,math)?;
  let exited=crate::original_roll_exit::advance(&mut self.entity,&mut self.character)?;
  let out=if exited{self.ordinary_forces(stick,math)?}else{self.rolling_forces(stick,math)?};
  Ok((out,exited))
 }
 fn rolling_forces(&mut self,stick:[f32;2],math:&OriginalMath)->Result<GroundOutput>{
  let magnitude=stick[0].hypot(stick[1]).min(1.);
  let control=ControlContext{directions:[angle(stick[0].atan2(stick[1])),0],magnitudes:[magnitude,0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0};
  let camera=CameraGroundContext{control:&control,stick_x:[0;2],cameras:[0;2],incoming_f28:0.,path_scratch:[0;0x44]};
  let direction=crate::original_target_support::DirectionContext{camera:&camera,incoming_f30:0.};
  crate::original_target_support::direction(&mut self.entity,&mut self.motion,&mut self.character,&direction,&Memory{base:0x80210000,bytes:&self.memory},&mut self.cache,math)?;
  crate::original_target_support::accelerate(&self.entity,&self.motion,&mut self.character,[0.,-1.,0.],0x80210000,&mut self.memory,math)?;
  integrate_velocity(&self.entity,&mut self.motion,&mut self.character,control.magnitudes,math)?;
  let velocity=std::array::from_fn(|j|f32::from_be_bytes(self.motion[j*4..j*4+4].try_into().unwrap())*6.);
  let source_yaw=u32::from_be_bytes(self.entity[12..16].try_into().unwrap())as f32*std::f32::consts::TAU/65536.;
  Ok(GroundOutput{velocity,yaw:std::f32::consts::FRAC_PI_2-source_yaw})
 }
 /// Action3 force phase only; caller owns readiness decisions and collision.
 /// Original84844: direction48490, brake48ec4, integration46de4.
 pub fn charge_frame(&mut self,position:[f32;3],velocity:[f32;3],yaw:f32,stick:[f32;2],math:&OriginalMath)->Result<GroundOutput>{
  if !stick.iter().all(|v|v.is_finite()){return Err("nonfinite charge stick".into());}
  let mut next=self.clone();next.entity[0]=3;next.entity[4]|=5;
  next.prepare_contact(position,velocity,yaw,math)?;
  let magnitude=stick[0].hypot(stick[1]).min(1.);
  let control=ControlContext{directions:[angle(stick[0].atan2(stick[1])),0],magnitudes:[magnitude,0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0};
  let camera=CameraGroundContext{control:&control,stick_x:[0;2],cameras:[0;2],incoming_f28:0.,path_scratch:[0;0x44]};
  let direction=crate::original_target_support::DirectionContext{camera:&camera,incoming_f30:0.};
  crate::original_target_support::direction(&mut next.entity,&mut next.motion,&mut next.character,&direction,&Memory{base:0x80210000,bytes:&next.memory},&mut next.cache,math)?;
  crate::original_brake_force::accelerate(&mut next.entity,&next.motion,&mut next.character,[0.,-1.,0.],control.magnitudes,math)?;
  integrate_velocity(&next.entity,&mut next.motion,&mut next.character,control.magnitudes,math)?;
  let velocity=std::array::from_fn(|j|f32::from_be_bytes(next.motion[j*4..j*4+4].try_into().unwrap())*6.);
  let yaw=std::f32::consts::FRAC_PI_2-u32::from_be_bytes(next.entity[12..16].try_into().unwrap())as f32*std::f32::consts::TAU/65536.;
  *self=next;Ok(GroundOutput{velocity,yaw})
 }
 /// Charge decision selects action4 before this same frame's rolling physics.
 /// Audio requests remain the host's responsibility.
 pub fn release_spin(&mut self,position:[f32;3],velocity:[f32;3],yaw:f32,stick:[f32;2],charge:f32,math:&OriginalMath)->Result<GroundOutput>{
  if !charge.is_finite()||!stick.iter().all(|v|v.is_finite()){return Err("nonfinite spin release".into());}
  self.entity[4]=5;
  self.prepare_contact(position,velocity,yaw,math)?;
  self.entity[0]=4;
  self.character[0x176..0x178].copy_from_slice(&12_u16.to_be_bytes());
  put(&mut self.character,0x64,charge.to_bits());
  self.character[0x360..0x362].fill(0);put(&mut self.character,0x44,0_f32.to_bits());
  self.rolling_forces(stick,math)
 }
 /// Y-up resolved contact position/velocity and world X/Z stick.
 /// Prototype excludes animation advancement and animation-driven idle exit.
 pub fn run_brake_frame(&mut self,position:[f32;3],velocity:[f32;3],yaw:f32,stick:[f32;2],math:&OriginalMath)->Result<(GroundOutput,bool)>{
  self.locomotion_frame(position,velocity,yaw,stick,false,false,math)
 }
 /// Animation completion comes from the host presentation clock. Idle variant
 /// selection and full animation blending remain outside this adapter.
 pub fn locomotion_frame(&mut self,position:[f32;3],velocity:[f32;3],yaw:f32,stick:[f32;2],brake_finished:bool,turn_finished:bool,math:&OriginalMath)->Result<(GroundOutput,bool)>{
  if !stick.iter().all(|v|v.is_finite()){return Err("nonfinite locomotion stick".into());}
  self.prepare_contact(position,velocity,yaw,math)?;
  let control=ControlContext{directions:[angle(stick[0].atan2(stick[1])),0],magnitudes:[stick[0].hypot(stick[1]).min(1.),0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0};
  let camera=CameraGroundContext{control:&control,stick_x:[0;2],cameras:[0;2],incoming_f28:0.,path_scratch:[0;0x44]};
  let direction=crate::original_target_support::DirectionContext{camera:&camera,incoming_f30:0.};
  let memory=Memory{base:0x80210000,bytes:&self.memory};
  let mut turned=false;
  if self.entity[0]==0{
   let mut input=crate::original_control::InputOutputs{direction:None,magnitude:None};
   if crate::original_control::read(&self.entity,&self.character,&control,&memory,&mut self.cache,&mut input,math)?{self.entity[0]=1;self.character[0x176..0x178].copy_from_slice(&6_u16.to_be_bytes());}
  }
  if self.entity[0]==12{
   turned=crate::original_brake_turn::advance(&mut self.entity,&mut self.motion,&mut self.character,&direction,&memory,&mut self.cache,math)?;
   if !turned&&brake_finished{self.entity[0]=0;self.character[0x18..0x1a].fill(0);self.character[0x176..0x178].fill(0);}
  }else if crate::original_reverse_input::check(&self.entity,&self.character,&control,&memory,&mut self.cache,math)?{
   let speed=f32::from_be_bytes(self.character[0x64..0x68].try_into().unwrap());
   let jog=f32::from_be_bytes(self.character[0xe4..0xe8].try_into().unwrap());
   if speed>jog{
    self.entity[0]=12;self.character[0x176..0x178].copy_from_slice(&18_u16.to_be_bytes());self.character[0x17c..0x17e].fill(0);
   }
  }
  if self.entity[0]!=12{
   if turn_finished{
    self.character[0x178..0x17a].fill(0);self.character[0x17a..0x17c].copy_from_slice(&27_u16.to_be_bytes());
    crate::original_run_animation_boost::advance(&mut self.character)?;
    self.character[0x17a..0x17c].fill(0);
   }
   if self.entity[0]==0{crate::original_target_support::direction(&mut self.entity,&mut self.motion,&mut self.character,&direction,&memory,&mut self.cache,math)?;}
   Ok((self.ordinary_forces(stick,math)?,turned))
  }else{
  crate::original_brake_force::accelerate(&mut self.entity,&self.motion,&mut self.character,[0.,-1.,0.],control.magnitudes,math)?;
  integrate_velocity(&self.entity,&mut self.motion,&mut self.character,control.magnitudes,math)?;
  let velocity=std::array::from_fn(|j|f32::from_be_bytes(self.motion[j*4..j*4+4].try_into().unwrap())*6.);
  let yaw=std::f32::consts::FRAC_PI_2-u32::from_be_bytes(self.entity[12..16].try_into().unwrap())as f32*std::f32::consts::TAU/65536.;
  Ok((GroundOutput{velocity,yaw},turned))
  }
 }
 /// Y-up resolved contact position/velocity and world X/Z stick.
 pub fn tick_brake(&mut self,position:[f32;3],velocity:[f32;3],yaw:f32,stick:[f32;2],math:&OriginalMath)->Result<GroundOutput>{
  if !stick.iter().all(|v|v.is_finite()){return Err("nonfinite brake stick".into());}
  if self.entity[0]!=12{put(&mut self.motion,0x1c,angle(yaw));}
  self.entity[0]=12;self.entity[4]=0;
  self.prepare_contact(position,velocity,yaw,math)?;
  let magnitudes=[stick[0].hypot(stick[1]).min(1.),0.];
  crate::original_brake_force::accelerate(&mut self.entity,&self.motion,&mut self.character,[0.,-1.,0.],magnitudes,math)?;
  integrate_velocity(&self.entity,&mut self.motion,&mut self.character,magnitudes,math)?;
  let velocity=std::array::from_fn(|j|f32::from_be_bytes(self.motion[j*4..j*4+4].try_into().unwrap())*6.);
  let yaw=std::f32::consts::FRAC_PI_2-u32::from_be_bytes(self.entity[12..16].try_into().unwrap())as f32*std::f32::consts::TAU/65536.;
  Ok(GroundOutput{velocity,yaw})
 }
 /// Y-up resolved contact position/velocity and world X/Z stick.
 pub fn tick(&mut self,position:[f32;3],velocity:[f32;3],yaw:f32,stick:[f32;2],math:&OriginalMath)->Result<GroundOutput>{
  if !stick.iter().all(|v|v.is_finite()){return Err("nonfinite ground stick".into());}
  self.prepare_contact(position,velocity,yaw,math)?;
  self.ordinary_forces(stick,math)
 }
 fn ordinary_forces(&mut self,stick:[f32;2],math:&OriginalMath)->Result<GroundOutput>{
  let magnitude=stick[0].hypot(stick[1]).min(1.);
  let control=ControlContext{directions:[angle(stick[0].atan2(stick[1])),0],magnitudes:[magnitude,0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0};
  let camera=CameraGroundContext{control:&control,stick_x:[0;2],cameras:[0;2],incoming_f28:0.,path_scratch:[0;0x44]};
  let context=GroundContext{camera,ordinary_path_scratch:[0;0x44]};let memory=Memory{base:0x80210000,bytes:&self.memory};
  accelerate(&mut self.entity,&mut self.motion,&mut self.character,&context,&memory,&mut self.cache,math)?;
  integrate_velocity(&self.entity,&mut self.motion,&mut self.character,control.magnitudes,math)?;
  let velocity=std::array::from_fn(|j|f32::from_be_bytes(self.motion[j*4..j*4+4].try_into().unwrap())*6.);
  let source_yaw=u32::from_be_bytes(self.entity[12..16].try_into().unwrap())as f32*std::f32::consts::TAU/65536.;
  Ok(GroundOutput{velocity,yaw:std::f32::consts::FRAC_PI_2-source_yaw})
 }
}

#[cfg(test)]
mod settled_contact_tests {
 use super::*;
 #[test]
 fn fresh_contact_uses_full_surface_angles() {
  let math=OriginalMath::from_data_bundle(include_bytes!("../../playable/assets/original-math-data.bin").to_vec()).unwrap();
  for normal in [[0.,0.8,-0.6],[0.6,0.8,0.]] {
   let mut g=GroundMotion::new(include_bytes!("../../playable/assets/physics.bin")).unwrap();
   g.set_surface_normal(normal).unwrap();
   let [pitch,roll]=math.surface_angles(normal).unwrap();
   g.prepare_contact([0.;3],[0.;3],0.,&math).unwrap();
   assert_eq!(&g.entity[8..12],&(pitch&0xffff).to_be_bytes());
   assert_eq!(&g.entity[16..20],&(roll&0xffff).to_be_bytes());
  }
 }
 #[test]
 fn stationary_contact_retains_orientation_only_under_source_guard() {
  let math=OriginalMath::from_data_bundle(include_bytes!("../../playable/assets/original-math-data.bin").to_vec()).unwrap();
  for (velocity,grounded,old_y,new_normal,retained) in [
   ([0.,0.,0.],true,0.8,[0.6,0.8,0.],true),
   ([0.,-6.,0.],true,0.8,[0.6,0.8,0.],true),
   ([0.,0.,0.],true,1.,[0.6,0.8,0.],true),
   ([1.,0.,0.],true,0.8,[0.6,0.8,0.],false),
   ([0.,0.,1.],true,0.8,[0.6,0.8,0.],false),
   ([0.,0.,0.],false,0.8,[0.6,0.8,0.],false),
   ([0.,0.,0.],true,0.8,[0.,1.,0.],false),
  ] {
   let mut g=GroundMotion::new(include_bytes!("../../playable/assets/physics.bin")).unwrap();
   put(&mut g.entity,8,0x1234);put(&mut g.entity,0x10,0x2345);
   g.character[0x1d]=u8::from(grounded);
   vector(&mut g.memory,0x1c,[0.6,old_y,0.]);
   g.set_surface_normal(new_normal).unwrap();
   let before=(g.entity[8..12].to_vec(),g.entity[16..20].to_vec());
   g.prepare_contact([0.;3],velocity,0.,&math).unwrap();
   let after=(g.entity[8..12].to_vec(),g.entity[16..20].to_vec());
   assert_eq!(before==after,retained,"velocity={velocity:?} grounded={grounded} old_y={old_y} normal={new_normal:?}");
  }
 }
}
