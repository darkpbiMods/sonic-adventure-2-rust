//! Test-world adapter using the original build's parameters. This controller and
//! collision solver are new Rust code, NOT a translated SA2 action dispatcher.
//! All simulation is 60 Hz; source speeds are units/frame at 0.1 world units/unit.
use glam::{Vec2, Vec3, vec2, vec3};
use sa2_runtime::{sonic::Physics,air_fallback_host::AirFallback,original_math::OriginalMath};
fn fallback_math()->&'static OriginalMath{static M:std::sync::OnceLock<OriginalMath>=std::sync::OnceLock::new();M.get_or_init(||OriginalMath::from_data_bundle(include_bytes!("../assets/original-math-data.bin").to_vec()).expect("verified original math data"))}

pub const STEP:f64=1.0/60.0;
pub const RADIUS:f32=0.28;
pub const HEIGHT:f32=1.05;
#[derive(Clone,Copy,Default)]
pub struct Input {pub direction:Vec2,pub jump_pressed:bool,pub jump_held:bool,pub charge:bool,pub bounce_pressed:bool,pub action_held:bool}
#[derive(Clone,Copy,Debug,PartialEq)]
pub enum State {Idle,Run,Brake,Jump,Fall,Charge,Roll,Somersault,Land,BounceDown,BounceUp,LightAttack,Homing,RingDash}
impl State {pub fn label(self)->&'static str {match self {Self::Idle=>"IDLE",Self::Run=>"RUN",Self::Brake=>"BRAKE",Self::Jump=>"JUMP",Self::Fall=>"FALL",Self::Charge=>"CHARGE",Self::Roll=>"ROLL",Self::Somersault=>"SOMERSAULT",Self::Land=>"LAND",Self::BounceDown=>"BOUNCE DOWN",Self::BounceUp=>"REBOUND",Self::LightAttack=>"LIGHT ATTACK",Self::Homing=>"HOMING",Self::RingDash=>"LIGHT DASH"}}}
pub const FULL_CHARGE:f32=2.0;
#[derive(Clone,Copy,PartialEq)]
enum JumpMotion {Intro,Spin,Rebound,FallIntro,FallLoop}
pub fn targets()->Vec<Vec3>{vec![vec3(-6.,0.65,0.),vec3(-8.,1.,4.),vec3(-6.,1.8,8.),vec3(-10.,1.,12.)]}
/// Visibility for the prototype attack; sample more finely than the thinnest wall.
pub fn clear_attack_path(from:Vec3,to:Vec3)->bool{
 let steps=(from.distance(to)/0.08).ceil().max(1.)as usize;
 for i in 1..=steps{let p=from.lerp(to,i as f32/steps as f32);
  if blocks().iter().any(|b|p.x>b.min.x-RADIUS&&p.x<b.max.x+RADIUS&&p.z>b.min.z-RADIUS&&p.z<b.max.z+RADIUS&&p.y+HEIGHT>b.min.y&&p.y<b.max.y){return false;}
  if ramp_height(vec2(p.x,p.z)).is_some_and(|h|p.y<h-0.01){return false;}
 }true
}
#[derive(Clone,Copy)]
pub struct Block {pub min:Vec3,pub max:Vec3}
pub fn blocks()->Vec<Block> {vec![
 Block{min:vec3(-8.,0.,35.),max:vec3(8.,4.,48.)},
 Block{min:vec3(13.,0.,-1.),max:vec3(14.,5.,21.)},
 Block{min:vec3(-18.,0.,78.),max:vec3(18.,4.,79.)},
 Block{min:vec3(20.,0.,25.),max:vec3(28.,2.,35.)},
 Block{min:vec3(20.,0.,40.),max:vec3(28.,4.,50.)},
 Block{min:vec3(-25.,0.,12.),max:vec3(-19.,1.,18.)},
 Block{min:vec3(-25.,0.,22.),max:vec3(-19.,2.,28.)},
 Block{min:vec3(-25.,0.,32.),max:vec3(-19.,3.,38.)},
]}
pub fn ramp_height(p:Vec2)->Option<f32>{if p.x>=-8. && p.x<=8. && p.y>=15. && p.y<=35. {Some((p.y-15.)*0.2)}else{None}}
fn in_rect(p:Vec2,b:Block,margin:f32)->bool{p.x>b.min.x-margin && p.x<b.max.x+margin && p.y>b.min.z-margin && p.y<b.max.z+margin}
pub fn support(p:Vec3,ceiling:f32)->Option<f32>{
 let mut h:Option<f32>=if p.x.abs()<=45. && p.z>=-25. && p.z<=100. && ceiling>=-0.001 {Some(0.)}else{None};
 if let Some(r)=ramp_height(vec2(p.x,p.z)){if r<=ceiling+0.001{h=Some(h.unwrap_or(-100.).max(r));}}
 for b in blocks(){if in_rect(vec2(p.x,p.z),b,0.) && b.max.y<=ceiling+0.001{h=Some(h.unwrap_or(-100.).max(b.max.y));}}
 if let Some((y,_))=crate::slope_arena::SlopeArena::shared().support(p,ceiling){h=Some(h.map_or(y,|v|v.max(y)));}
 h
}
pub fn rings()->Vec<Vec3>{
 let mut r=Vec::new();
 for i in 0..15{let z=3.+i as f32*3.;r.push(vec3(0.,support(vec3(0.,0.,z),100.).unwrap()+0.7,z));}
 for i in 0..9{r.push(vec3(-22.,0.75, -10.+i as f32*2.));}
 for i in 0..8{r.push(vec3(24.,if i<4{2.7}else{4.7},27.+(i%4)as f32*2.+if i<4{0.}else{15.}));}
 r
}
/// Host-owned world queries. Target/ring slots must retain identity for a player's lifetime.
/// Removed targets return None; do not compact or reuse slots until reset.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum ActionEvent {Jump,ChargeStart,ChargeReady,SpinDash,HomingStart,LightAttackStart,BounceStart,BounceImpact,TargetHit(usize),RingCollected(usize),Respawn}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum HomingContactResponse {DemoUpward,SourceCategory0Upright,SourceCategory2,SourceCategory3}
#[derive(Clone,Copy,Debug)]
pub struct BodyContactProfile{pub category:u16,pub descriptor:[u8;0x30]}
impl Default for BodyContactProfile{
 fn default()->Self{let mut descriptor=[0;0x30];descriptor[4..8].copy_from_slice(&0x400u32.to_be_bytes());Self{category:2,descriptor}}
}
pub trait World {
 /// Optional source-unit terrain snapshot. Keep triangle ordering/identifiers
 /// stable across snapshots and return the same snapshot throughout a tick.
 fn source_terrain(&self)->Option<&sa2_runtime::static_terrain_host::StaticTerrain>{None}
 /// Populate host-owned task/object/slot memory before ground or air contact.
 fn prepare_platform_memory(&self,_memory:&mut sa2_runtime::original_platform::PlatformMemory)->sa2_runtime::disc::Result<()>{Ok(())}
 /// Callback side effects must stay in supplied transactional guest memory.
 fn platform_callback(&self,_address:u32,_player:u32,_entity:u32,_slot:u32,_memory:&mut sa2_runtime::original_platform::PlatformMemory)->sa2_runtime::disc::Result<()>{Err("host platform callback not implemented".into())}
 /// Host must identify original contact semantics; generic targets keep demo behavior.
 fn homing_contact_response(&self,_index:usize)->HomingContactResponse{HomingContactResponse::DemoUpward}
 /// Called exactly once after a complete fixed tick, for host presentation snapshots.
 fn after_tick(&self,_player:&Player){}
 /// Synchronous events; hosts may enqueue them for audio/gameplay after the tick.
 fn event(&self,_event:ActionEvent,_position:Vec3){}
 /// Original feedback request; host owns enable settings and motor mapping.
 fn vibration(&self,_request:sa2_runtime::original_vibration::Request,_position:Vec3){}
 fn target_count(&self)->usize;
 fn target(&self,index:usize)->Option<Vec3>;
 /// Entity origin for source candidate distance; target() is the world aim center.
 fn target_origin(&self,index:usize)->Option<Vec3>{self.target(index)}
 /// Original task type; type17 belongs to the separate ring candidate list.
 fn homing_target_kind(&self,_index:usize)->u8{0}
 /// Bit0x40 enables insertion into original player proximity lists.
 fn homing_target_flags(&self,index:usize)->u16{if self.target_attackable(index){0x40}else{0}}
 /// Target sphere radius in host units. Default point targets still contact Sonic's body.
 fn target_radius(&self,_index:usize)->f32{0.}
 /// Host translates its collision categories, immunity and attack filters here.
 fn target_attackable(&self,_index:usize)->bool{true}
 /// Authored default: category2, defense0, receives source hit marking.
 /// Applies to body and targeted attack hits; target health policy remains separate.
 fn target_body_contact_profile(&self,_index:usize)->BodyContactProfile{BodyContactProfile::default()}
 /// Explicit host adapter: generic attackable targets map to source category2.
 fn light_target_category(&self,_index:usize)->u16{2}
 /// Original list supplies cached squared distance in SA units; override to reproduce its producer.
 fn light_target_distance(&self,index:usize,origin:Vec3)->Option<f32>{
  self.target(index).map(|p|((p-origin)*10.).length_squared())
 }
 fn ring_count(&self)->usize;
 fn ring(&self,index:usize)->Option<Vec3>;
 fn attack_path_clear(&self,from:Vec3,to:Vec3)->bool;
 /// Resolve one bounded movement step, including grounded/bounce/landing state.
 fn move_player(&self,player:&mut Player,dt:f32);
 /// Upward contact normal at the resolved grounded position, in world space.
 fn ground_normal(&self,_position:Vec3)->Vec3{Vec3::Y}
 fn spawn(&self)->Vec3;
 fn should_respawn(&self,position:Vec3)->bool;
}
pub struct DemoWorld;
pub fn demo_source_terrain()->&'static sa2_runtime::static_terrain_host::StaticTerrain{
 static TERRAIN:std::sync::OnceLock<sa2_runtime::static_terrain_host::StaticTerrain>=std::sync::OnceLock::new();
 TERRAIN.get_or_init(||{
  let mut triangles=Vec::new();
  let mut quad=|a:Vec3,b:Vec3,c:Vec3,d:Vec3,normal:Vec3|{
   for mut t in [[a,b,c],[a,c,d]]{if (t[1]-t[0]).cross(t[2]-t[0]).dot(normal)<0.{t.swap(1,2);}triangles.push(t.map(|v|(v*10.).to_array()));}
  };
  quad(vec3(-45.,0.,-25.),vec3(-45.,0.,100.),vec3(45.,0.,100.),vec3(45.,0.,-25.),Vec3::Y);
  quad(vec3(-8.,0.,15.),vec3(-8.,4.,35.),vec3(8.,4.,35.),vec3(8.,0.,15.),vec3(0.,1.,-0.2));
  for b in blocks(){let a=b.min;let z=b.max;
   quad(vec3(a.x,z.y,a.z),vec3(a.x,z.y,z.z),z,vec3(z.x,z.y,a.z),Vec3::Y);
   quad(a,vec3(a.x,z.y,a.z),vec3(z.x,z.y,a.z),vec3(z.x,a.y,a.z),-Vec3::Z);
   quad(vec3(a.x,a.y,z.z),vec3(a.x,z.y,z.z),z,vec3(z.x,a.y,z.z),Vec3::Z);
   quad(a,vec3(a.x,a.y,z.z),vec3(a.x,z.y,z.z),vec3(a.x,z.y,a.z),-Vec3::X);
   quad(vec3(z.x,a.y,a.z),vec3(z.x,a.y,z.z),z,vec3(z.x,z.y,a.z),Vec3::X);
   quad(a,vec3(a.x,a.y,z.z),vec3(z.x,a.y,z.z),vec3(z.x,a.y,a.z),-Vec3::Y);
  }
  let arena=crate::slope_arena::SlopeArena::shared();let tube_start=triangles.len()+arena.bowl.len();
  triangles.extend(arena.source_vertices());
  sa2_runtime::static_terrain_host::StaticTerrain::with_mesh_ranges(&triangles,&[(tube_start..triangles.len(),0)]).expect("demo static triangles")
 })
}
impl World for DemoWorld {
 fn source_terrain(&self)->Option<&sa2_runtime::static_terrain_host::StaticTerrain>{Some(demo_source_terrain())}
 fn target_count(&self)->usize{targets().len()}
 fn target(&self,i:usize)->Option<Vec3>{targets().get(i).copied()}
 fn target_radius(&self,_i:usize)->f32{0.42}
 fn ring_count(&self)->usize{rings().len()}
 fn ring(&self,i:usize)->Option<Vec3>{rings().get(i).copied()}
 fn attack_path_clear(&self,a:Vec3,b:Vec3)->bool{clear_attack_path(a,b)}
 fn move_player(&self,p:&mut Player,dt:f32){p.move_substep(dt)}
 fn ground_normal(&self,p:Vec3)->Vec3{
  if let Some((h,n))=crate::slope_arena::SlopeArena::shared().support(p,p.y+0.02){if (h-p.y).abs()<0.02{return n;}}
  if ramp_height(vec2(p.x,p.z)).is_some_and(|h|(h-p.y).abs()<0.02){vec3(0.,1.,-0.2).normalize()}else{Vec3::Y}
 }
 fn spawn(&self)->Vec3{vec3(0.,0.,-6.)}
 fn should_respawn(&self,p:Vec3)->bool{p.y< -50.}
}
// DOL801b762c entries0/18/27/65..68; source lengths from the asset manifest.
fn locomotion_animation_clock()->sa2_runtime::animation_clock_host::AnimationClock {
 use sa2_runtime::animation_clock_host::{AnimationClock,AnimationSpec};
 let specs:Vec<_>=[(0u16,3u16,0.0625f32,0.1f32,8),(18,9,0.25,0.3,6),(27,4,1.,0.5,8),(65,4,0.25,1.4,8),(66,3,0.25,1.,17),(67,4,0.125,0.4,4),(68,3,0.125,0.4,4),(1,3,0.25,0.1,8),(26,4,0.0625,1.,8),(15,3,0.125,0.4,4),(16,4,1.,0.2,4),(17,4,1.,0.4,32),(4,3,0.03125,0.1,9),(5,3,0.03125,0.1,8),(87,3,1.,0.5,4),(88,3,0.03125,0.3,18),(89,3,0.03125,0.1,39),(90,9,0.03125,0.25,16),(91,4,0.25,1.,16),(92,4,0.25,1.,16),(93,4,0.25,0.2,4),(94,9,0.25,0.3,6),(96,4,0.25,1.,16),(97,4,0.25,1.,16),(98,4,0.25,0.2,4),(99,4,0.25,1.,4),(30,3,0.5,1.,8),(11,11,0.25,0.2,8),(12,10,0.25,0.7,8),(100,3,1.,3.,17),(101,6,0.25,0.3,24),(102,6,0.25,0.3,24),(103,6,0.25,0.3,24),(6,11,0.25,3.4,31),(7,11,0.25,1.4,31),(8,11,0.25,0.8,31),(9,11,0.25,0.6,31),(10,10,0.5,0.4,31)].into_iter().map(|(id,mode,blend,rate,frames)|{
  let mut entry=[0u8;16];if id==30{entry[2..4].copy_from_slice(&6u16.to_be_bytes());}entry[0..2].copy_from_slice(&id.to_be_bytes());entry[4..6].copy_from_slice(&mode.to_be_bytes());entry[8..12].copy_from_slice(&blend.to_be_bytes());entry[12..16].copy_from_slice(&rate.to_be_bytes());
  let next:u16=match id{91=>92,92=>93,93=>16,96=>97,97=>98,98=>99,99=>16,65|66=>66,67|68=>68,1|26=>1,15=>15,30=>30,100=>100,101..=103=>67,4..=12|87|88|89=>id,_=>0};entry[6..8].copy_from_slice(&next.to_be_bytes());
  AnimationSpec{id,entry,frames}
 }).collect();
 AnimationClock::new(&specs,0).expect("original locomotion metadata")
}
pub struct Player {
 fire_spawn:Option<crate::fire_effect::Spawn>,
 pre_movement_effect_center:Option<Vec3>,
 homing_launch_center:Option<Vec3>,
 draw_counter:u16,homing_draw_status:u16,
 homing_candidates:Option<Vec<(usize,f32)>>,
 heading_history:sa2_runtime::heading_history::HeadingHistory,source_locomotion:Option<sa2_runtime::ground_motion_host::LocomotionAnimation>,
 locomotion_clock:Option<sa2_runtime::animation_clock_host::AnimationClock>,brake_finish_pending:bool,
 turning:bool,turn_finish_pending:bool,idle_active:bool,idle_seed:u32,
 action_hold_ticks:u16,shared_charge:bool,
 somersault_stage:u8,somersault_queued:bool,somersault_fast:bool,somersault_timer:i16,somersault_replay:bool,
 somersault_recovery:bool,
 ring_motion:Option<sa2_runtime::ring_motion_host::RingMotion>,
 ground_motion:Option<sa2_runtime::ground_motion_host::GroundMotion>,
 terrain_collision:Option<sa2_runtime::original_position_a::PositionState>,
 air_motion:Option<sa2_runtime::air_motion_host::AirMotion>,
 bounce_motion:Option<sa2_runtime::bounce_motion_host::BounceMotion>,
 pub position:Vec3,pub velocity:Vec3,pub yaw:f32,pub grounded:bool,pub ground_normal:Vec3,pub state:State,
 pub charge:f32,pub spin_charge_speed:f32,pub ticks:u64,pub air_dash_used:bool,pub collected:Vec<bool>,pub respawns:u32,
 pub wall_hits:u32,pub jumps:u32,pub landings:u32,pub max_height:f32,pub max_speed:f32,
 pub animation_time:f32,pub animation_frame:f64,pub physics:Physics,landing_animation:Option<u16>,light_landing_pending:bool,jump_timer:u8,previous_charge:bool,charge_ticks:u16,jump_motion:JumpMotion,rebound_cycle:u8,rebound_variant:u8,homing_seed:u32,homing_fallback:Option<AirFallback>,bounce_repeat:bool,bounce_fall_loop:bool,bounce_fall_started:bool,
 pub bounced:u32,pub defeated:Vec<bool>,pub chain_hits:u32,pub attack_target:Option<usize>,pub attack_timer:u16,pub light_steering_counter:i16,pub last_attack_hits:u32,
}
impl Default for Player {fn default()->Self{Self::new()}}
impl Player {
 pub fn new()->Self {Self {fire_spawn:None,homing_launch_center:None,pre_movement_effect_center:None,draw_counter:0,homing_draw_status:0,heading_history:sa2_runtime::heading_history::HeadingHistory::zeroed(0),source_locomotion:None,locomotion_clock:None,brake_finish_pending:false,turning:false,turn_finish_pending:false,idle_active:false,idle_seed:0,action_hold_ticks:0,shared_charge:false,somersault_stage:0,somersault_queued:false,somersault_fast:false,somersault_timer:0,somersault_replay:false,somersault_recovery:false,ring_motion:None,ground_motion:None,terrain_collision:None,air_motion:None,bounce_motion:None,position:vec3(0.,0.,-6.),velocity:Vec3::ZERO,yaw:0.,grounded:true,ground_normal:Vec3::Y,state:State::Idle,
 charge:0.,spin_charge_speed:0.,ticks:0,air_dash_used:false,collected:vec![false;rings().len()],respawns:0,wall_hits:0,jumps:0,landings:0,max_height:0.,max_speed:0.,animation_time:0.,animation_frame:0.,
 physics:Physics::decode(include_bytes!("../assets/physics.bin")).expect("verified Sonic physics"),landing_animation:None,light_landing_pending:false,jump_timer:0,previous_charge:false,charge_ticks:0,jump_motion:JumpMotion::Intro,rebound_cycle:0,rebound_variant:0,homing_seed:0,homing_fallback:None,bounce_repeat:false,bounce_fall_loop:false,bounce_fall_started:false,
 bounced:0,defeated:vec![false;targets().len()],chain_hits:0,attack_target:None,attack_timer:0,light_steering_counter:0,last_attack_hits:0,homing_candidates:None}}
 pub fn in_world(world:&dyn World)->Self{
  let mut p=Self::new();p.position=world.spawn();p.collected=vec![false;world.ring_count()];p.defeated=vec![false;world.target_count()];p.update_ground_normal(world);p
 }
 pub fn reset_in(&mut self,world:&dyn World){let n=self.respawns+1;*self=Self::in_world(world);self.respawns=n;}
 pub fn reset(&mut self){let n=self.respawns+1;*self=Self::new();self.respawns=n;}
 pub fn speed(&self)->f32{vec2(self.velocity.x,self.velocity.z).length()}
 /// Recovered charge/roll/Light Speed Attack and homing draw phase.
 pub fn source_draw_selection(&self)->sa2_runtime::original_sonic_draw::Selection{
  let Some(clock)=self.source_animation_clock() else{return sa2_runtime::original_sonic_draw::Selection::Current(0);};
  let o=clock.output();
  let status=self.source_draw_status();
  sa2_runtime::original_sonic_draw::select(o.mode as i16,o.current,status,0,self.draw_counter)
 }
 fn source_draw_status(&self)->u16{
  if self.state==State::Homing{self.homing_draw_status}
  else if matches!(self.state,State::Charge|State::Roll|State::LightAttack|State::BounceDown)
   || (self.state==State::BounceUp && self.clip()=="bounce"){0x100}else{0}
 }
 pub fn source_lean(&self)->u32{self.source_locomotion.as_ref().map_or(0,|a|a.lean)}
 pub fn model_rotation(&self)->glam::Mat3{
  if let Some(ring)=&self.ring_motion{
   let e=&ring.records().entity;
   let rotate=|v|Vec3::from_array(sa2_runtime::original_ground::rotate_vector(e,v,true,fallback_math()).expect("ring model orientation"));
   return glam::Mat3::from_cols(rotate([0.,0.,-1.]),rotate([0.,1.,0.]),rotate([1.,0.,0.]));
  }

  if self.state==State::Homing{if let Some(fallback)=&self.homing_fallback{
   let e=fallback.source_records().0;
   let rotate=|v|Vec3::from_array(sa2_runtime::original_ground::rotate_vector(e,v,true,fallback_math()).expect("fallback model orientation"));
   return glam::Mat3::from_cols(rotate([0.,0.,-1.]),rotate([0.,1.,0.]),rotate([1.,0.,0.]));
  }}
  if matches!(self.state,State::Homing|State::LightAttack){
   let action=if self.state==State::Homing{13}else{60};
   if let Some(records)=self.terrain_collision.as_ref().filter(|r|r.entity[0]==action){
    let rotate=|v|Vec3::from_array(sa2_runtime::original_ground::rotate_vector(&records.entity,v,true,fallback_math()).expect("pursuit model orientation"));
    return glam::Mat3::from_cols(rotate([0.,0.,-1.]),rotate([0.,1.,0.]),rotate([1.,0.,0.]));
   }
  }
  if let Some(bounce)=&self.bounce_motion{
   let e=&bounce.records().entity;
   let rotate=|v|Vec3::from_array(sa2_runtime::original_ground::rotate_vector(e,v,true,fallback_math()).expect("bounce model orientation"));
   return glam::Mat3::from_cols(rotate([0.,0.,-1.]),rotate([0.,1.,0.]),rotate([1.,0.,0.]));
  }
  if !self.grounded{if let Some(air)=&self.air_motion{
   let basis=air.model_basis(fallback_math()).expect("finite source air orientation");
   return glam::Mat3::from_cols(Vec3::from_array(basis[0]),Vec3::from_array(basis[1]),Vec3::from_array(basis[2]));
  }}
  if self.grounded{if let Some(ground)=&self.ground_motion{
   let basis=ground.model_basis(fallback_math()).expect("finite source orientation");
   return glam::Mat3::from_cols(Vec3::from_array(basis[0]),Vec3::from_array(basis[1]),Vec3::from_array(basis[2]));
  }}
  let slope=if self.grounded{glam::Quat::from_rotation_arc(Vec3::Y,self.ground_normal)}else{glam::Quat::IDENTITY};
  glam::Mat3::from_quat(slope*glam::Quat::from_rotation_y(self.yaw))
 }
 pub fn collected_count(&self)->usize{self.collected.iter().filter(|&&x|x).count()}
 fn p(&self,i:usize)->f32{self.physics.parameter(i).unwrap()}
 pub fn clip(&self)->&'static str{if self.turning&&self.grounded&&self.state==State::Run{return "turn";}if self.state==State::Run{if let Some(a)=&self.source_locomotion{match a.selected{6=>return "walk",7=>return "speedwalk",8=>return "jog",9=>return "run",10=>return "sprint",_=>{}}}}match self.state{
 State::Jump=>if self.air_dash_used{"jump_dash"}else{match self.jump_motion{JumpMotion::Intro=>"jump",JumpMotion::Spin=>"spin_jump",JumpMotion::Rebound=>match self.rebound_variant{1=>"rebound2",2=>"rebound3",_=>"rebound"},JumpMotion::FallIntro=>"bounce_fall_start",JumpMotion::FallLoop=>"bounce_fall"}},
 State::Homing=>"jump_dash",State::RingDash=>"light_dash",
 State::BounceDown=>"bounce",State::BounceUp=>if self.bounce_motion.as_ref().map_or(self.velocity.y>0., |_|!self.bounce_fall_started){"bounce"}else if self.bounce_fall_loop{"bounce_fall"}else{"bounce_fall_start"},
 State::Somersault=>"somersault",State::Roll=>"roll",State::Charge=>if self.charge>=FULL_CHARGE{"ready"}else{"spin"},State::LightAttack=>"roll",State::Fall=>match self.jump_motion{JumpMotion::FallIntro=>"bounce_fall_start",JumpMotion::FallLoop=>"bounce_fall",_=>"fall"},State::Land=>"land",State::Brake=>"brake",
 State::Idle=>if self.charge>=FULL_CHARGE{"ready"}else{"idle"},State::Run=>if self.speed()<self.p(9)*6.{"walk"}else if self.speed()<self.p(10)*6.{"jog"}else if self.speed()<self.p(3)*6.{"run"}else{"sprint"}}}
 pub fn animation_rate(&self)->f64{
  if self.state==State::Run&&!self.turning{if let Some(a)=&self.source_locomotion{
   let increment=match a.selected{6=>a.adjusted_speed*3.4,7=>a.adjusted_speed*1.4,8=>a.adjusted_speed*0.8,9=>a.adjusted_speed*0.6,10=>a.forward_speed*0.4,_=>0.};
   return increment as f64*60.;
  }}
  if self.clip()=="idle"{return 0.1_f32 as f64*60.;}
  if self.clip()=="fall"{return 0.4_f32 as f64*60.;}
  if self.clip()=="land"{return 0.2_f32 as f64*60.;}
  if self.clip()=="turn"{return 30.;}
  if self.clip()=="brake"{return 0.3_f32 as f64*60.;}
  if self.clip()=="ready"{return 60.;}
  if self.state==State::Charge{return (self.spin_charge_speed*0.2) as f64*60.;}
  if matches!(self.state,State::Roll|State::LightAttack){return (self.roll_animation_speed()*0.7) as f64*60.;}
  // Original table rates are frame increments per 60Hz update.
  match self.clip(){"walk"|"jog"|"run"|"sprint"=>(self.speed()*3.).clamp(16.,65.) as f64,"spin"|"spin_jump"|"bounce"=>60.,"jump"=>1.4_f32 as f64*60.,"jump_dash"=>180.,"rebound"|"rebound2"|"rebound3"=>0.3_f32 as f64*60.,"bounce_fall_start"|"bounce_fall"=>0.4_f32 as f64*60.,_=>30.}
 }
 /// Recovered ground/air transition clock. Presentation reads never advance it.
 fn roll_animation_speed(&self)->f32{
  self.ground_motion.as_ref().map(|g|f32::from_be_bytes(g.source_records().2[0x64..0x68].try_into().unwrap())).unwrap_or((self.velocity.x*self.yaw.sin()+self.velocity.z*self.yaw.cos())/6.)
 }
 pub fn source_animation_clock(&self)->Option<&sa2_runtime::animation_clock_host::AnimationClock>{
  if self.state==State::RingDash{return self.locomotion_clock.as_ref();}
  if matches!(self.state,State::Charge|State::Roll|State::LightAttack|State::Somersault){return self.locomotion_clock.as_ref();}
  match self.clip(){"brake"|"turn"|"walk"|"speedwalk"|"jog"|"run"|"sprint"=>self.locomotion_clock.as_ref(),"ready"|"idle"|"fall"|"land"|"bounce"|"jump_dash"|"jump"|"spin_jump"|"bounce_fall_start"|"bounce_fall"|"rebound"|"rebound2"|"rebound3"=>self.locomotion_clock.as_ref(),_=>None}
 }
 pub fn animation_blend(&self)->Option<f32>{
  let clock=self.source_animation_clock();
  clock.map(|clock|{
   let output=clock.output();
   if output.mode==2{output.frame.clamp(0.,1.)}else{1.}
  })
 }
 pub fn tick(&mut self,input:Input){self.tick_in(input,&DemoWorld);}
 fn update_ground_normal(&mut self,world:&dyn World){
  let source=world.source_terrain().and_then(|_|self.terrain_collision.as_ref()).filter(|r|r.entity[5]&1!=0)
   .map(|r|Vec3::from_array(std::array::from_fn(|i|f32::from_be_bytes(r.character[0x7c+i*4..0x80+i*4].try_into().unwrap()))));
  self.ground_normal=if self.grounded{source.unwrap_or_else(||world.ground_normal(self.position)).try_normalize().filter(|n|n.is_finite()).unwrap_or(Vec3::Y)}else{Vec3::Y};
 }
 fn resolve_air_landing(&mut self,input:Input,world:&dyn World){
  if self.grounded {
   if let Some(air)=self.air_motion.take(){
    let(e,m,c)=air.source_records();
    let mut ground=if let Some(collision)=self.terrain_collision.as_ref().filter(|s|s.entity==e&&s.motion==m&&s.character==c&&s.entity[5]&1!=0){
     sa2_runtime::ground_motion_host::GroundMotion::from_landed_collision(collision).expect("source landing records")
    }else{sa2_runtime::ground_motion_host::GroundMotion::from_air_records(e,m,c).expect("ordinary landing records")};
    if !input.charge {
     if ground.landing_brake(input.direction.to_array(),fallback_math()).expect("landing brake decision") {
      self.landing_animation=None;self.state=State::Brake;
     }else{
     let selected=ground.stationary_landing_with_context([input.direction.length().min(1.),0.],self.ticks as u32,&mut self.idle_seed,true,fallback_math()).expect("ordinary landing decision");
     self.landing_animation=selected;
     if selected==Some(17){world.vibration(sa2_runtime::original_vibration::request(6,15,0),self.position);}
     if selected.is_none(){ground.moving_landing(Some(self.heading_history.delayed_angles(3)[1]),fallback_math()).expect("moving landing records");}
     self.state=if selected.is_some(){State::Land}else if self.speed()>0.1||input.direction.length_squared()>0.{State::Run}else{State::Idle};
     }
    }
    self.ground_motion=Some(ground);
   }
  }else if !matches!(self.state,State::Jump|State::Fall){self.air_motion=None;}
 }
 pub fn tick_in(&mut self,input:Input,world:&dyn World){
  self.fire_spawn=None;
  self.homing_launch_center=None;
  self.pre_movement_effect_center=Some(self.effect_center());
  let clip=self.clip();let respawns=self.respawns;let ready_before=self.charge>=FULL_CHARGE;
  self.update_ground_normal(world);
  if self.light_landing_pending{
   self.light_landing_pending=false;
   self.resolve_air_landing(input,world);
  }
  let draw_state_before=self.state;
  self.tick_state(input,world);
  if matches!(self.state,State::Charge|State::Roll|State::LightAttack){
   if self.state!=draw_state_before{self.draw_counter=0;}
   self.draw_counter=sa2_runtime::original_sonic_draw::advance_counter(0x100,self.draw_counter);
  }
  if self.state==State::Homing{self.draw_counter=sa2_runtime::original_sonic_draw::advance_counter(self.homing_draw_status,self.draw_counter);}
  // Entry/repeat/contact preserve360. Rebound's apex decision clears100
  // when selecting67; the source bounce clip tracks that pre-physics decision.
  if matches!(self.state,State::BounceDown|State::BounceUp){
   self.draw_counter=sa2_runtime::original_sonic_draw::advance_counter(self.source_draw_status(),self.draw_counter);
  }
  if !self.grounded || input.charge || input.direction.length_squared()>0. {
   self.landing_animation=None;
  }
  if !self.grounded && matches!(self.state,State::Idle|State::Run|State::Brake|State::Land|State::Roll|State::Somersault){
   if let Some(ground)=self.ground_motion.take(){
    self.air_motion=Some(ground.into_fall().expect("ordinary support loss"));self.state=State::Fall;
   }
  }
  if !self.light_landing_pending{self.resolve_air_landing(input,world);}
  if !self.grounded || matches!(self.state,State::Homing|State::LightAttack){self.ground_motion=None;}
  if self.grounded{self.jump_motion=JumpMotion::Intro;self.rebound_cycle=0;}
  self.update_ground_normal(world);
  self.source_locomotion=None;
  if self.grounded&&self.state==State::Run&&!self.turning{
   if let Some(ground)=self.ground_motion.as_mut(){
    if self.charge>=FULL_CHARGE{
     ground.set_animation_selection(if !ready_before{26}else{self.locomotion_clock.as_ref().map_or(1,|c|c.output().selected)});
    }
    ground.set_experimental_surface_normal(self.ground_normal.to_array()).expect("final contact normal");
    self.source_locomotion=Some(ground.finish_locomotion_contact(self.position.to_array(),self.velocity.to_array(),self.yaw,Some(self.heading_history.delayed_angles(3)[1]),fallback_math()).expect("final locomotion contact"));
   }
  }
  let ordinary_idle=self.grounded&&self.state==State::Idle&&self.charge<FULL_CHARGE;
  let mut idle_selected=0;let mut idle_rate=1.;
  if ordinary_idle {
   if let Some(ground)=self.ground_motion.as_mut(){
    if !self.idle_active{ground.reset_idle_counter();}
    ground.set_animation_selection(self.locomotion_clock.as_ref().map_or(0,|c|c.output().selected));
    // Demo-local RNG/frame context; full game-global effect/audio RNG is external.
    ground.set_experimental_surface_normal(self.ground_normal.to_array()).expect("idle support normal");
    ground.idle_surface_animation(self.ticks as u32,&mut self.idle_seed,true,fallback_math()).expect("surface idle selection");
    idle_rate=f32::from_be_bytes(ground.source_records().2[0x44..0x48].try_into().unwrap());
    idle_selected=u16::from_be_bytes(ground.source_records().2[0x176..0x178].try_into().unwrap());
   }
  }
  self.idle_active=ordinary_idle;
  // Presentation phase follows the selected animation, not unrelated action labels.
  // Entry27's blend=1 completes immediately, then reset_time sets frame0.
  // Verified against the complete original updater; it does not retain frame1.
  if self.respawns!=respawns || self.clip()!=clip{self.animation_time=0.;self.animation_frame=0.;}
  else{self.animation_time+=STEP as f32;self.animation_frame+=STEP*self.animation_rate();}
  let air_selected=match self.clip(){"bounce"=>Some(30),"jump_dash"=>Some(100),"jump"=>Some(65),"spin_jump"=>Some(66),"bounce_fall_start"=>Some(67),"bounce_fall"=>Some(68),"rebound"=>Some(101),"rebound2"=>Some(102),"rebound3"=>Some(103),_=>None};
  if self.state==State::Somersault{
   let first=if self.somersault_fast{96}else{91};
   let selected=if self.somersault_recovery{94}else if self.somersault_replay{96}else{self.locomotion_clock.as_ref().map_or(first,|c|if (first..=if self.somersault_fast{99}else{93}).contains(&c.output().selected){c.output().selected}else{first})};
   let clock=self.locomotion_clock.get_or_insert_with(locomotion_animation_clock);
   let output=clock.tick(selected,1.,1.,fallback_math()).expect("somersault animation clock");
   self.animation_frame=if output.mode==2{0.}else{output.frame as f64};
  }else if self.state==State::RingDash{
   let clock=self.locomotion_clock.get_or_insert_with(locomotion_animation_clock);
   let output=clock.tick(87,1.,1.,fallback_math()).expect("ring animation clock");
   self.animation_frame=if output.mode==2{0.}else{output.frame as f64};
  }else if matches!(self.state,State::Charge|State::Roll|State::LightAttack){
   let selected=if self.state==State::Charge{11}else{12};
   let forward=self.roll_animation_speed();
   let clock=self.locomotion_clock.get_or_insert_with(locomotion_animation_clock);
   let output=clock.tick(selected,forward,self.spin_charge_speed,fallback_math()).expect("verified spin animation clock");
   self.animation_frame=if output.mode==2{0.}else{output.frame as f64};
   self.brake_finish_pending=false;
  }else if let Some(selected)=air_selected{
   let clock=self.locomotion_clock.get_or_insert_with(locomotion_animation_clock);
   let output=clock.tick(selected,1.,1.,fallback_math()).expect("verified air animation clock");
   self.animation_frame=if output.mode==2{0.}else{output.frame as f64};
   if selected==65&&output.current==66{self.jump_motion=JumpMotion::Spin;self.animation_time=0.;}
   if selected==67&&output.current==68{
    if self.state==State::BounceUp{self.bounce_fall_loop=true;}else{self.jump_motion=JumpMotion::FallLoop;}
    self.animation_time=0.;
   }
  }else if matches!(self.clip(),"brake"|"turn")||self.source_locomotion.is_some(){
   let selected=if self.clip()=="brake"{18}else if self.clip()=="turn"{27}else{self.source_locomotion.as_ref().unwrap().selected};
   let speeds=self.source_locomotion.as_ref().map(|a|(a.forward_speed,a.adjusted_speed)).unwrap_or((1.,1.));
   let clock=self.locomotion_clock.get_or_insert_with(locomotion_animation_clock);
   let output=clock.tick(selected,speeds.0,speeds.1,fallback_math()).expect("verified locomotion animation clock");
   // During source mode2, frame is blend progress, not a motion sample frame.
   self.animation_frame=if output.mode==2{0.}else{output.frame as f64};
   if let Some(ground)=self.ground_motion.as_mut(){ground.set_animation_selection(output.selected);}
   self.brake_finish_pending=selected==18&&output.flags&1!=0;
   if output.previous==27&&output.current!=27{self.turning=false;self.turn_finish_pending=true;self.animation_frame=0.;self.animation_time=0.;}
  }else if matches!(self.clip(),"ready"|"idle"|"fall"|"land"){
   let selected=match self.clip(){"ready"=>if !ready_before{26}else{self.locomotion_clock.as_ref().map_or(1,|c|if c.output().selected==26{26}else{1})},"fall"=>15,"land"=>self.landing_animation.unwrap_or(16),_=>idle_selected};
   let clock=self.locomotion_clock.get_or_insert_with(locomotion_animation_clock);
   let output=if self.respawns!=respawns{clock.output()}else{clock.tick(selected,1.,if ordinary_idle{idle_rate}else{1.},fallback_math()).expect("verified idle/fall/landing clock")};
   if self.state==State::Land && ((output.selected==0 && output.current==0) || (selected==1&&output.current==1)) {
    self.landing_animation=None;self.state=State::Idle;
   }
   self.animation_frame=if output.mode==2{0.}else{output.frame as f64};
   self.brake_finish_pending=false;
  }else{self.locomotion_clock=None;self.brake_finish_pending=false;}
  let heading=((std::f32::consts::FRAC_PI_2-self.yaw)*(65536./std::f32::consts::TAU))as i32 as u32&0xffff;
  self.heading_history.record(self.position.to_array().map(|v|(v*10.).to_bits()),[0,heading,0]);
  world.after_tick(self);
  // Observers consume launch-frame records before original external-force clear.
  if let Some(air)=self.air_motion.as_mut(){
   if air.source_records().1[12..24].iter().any(|&b|b!=0){
    air.finish_frame_forces().expect("air frame force clear");
    if let Some(records)=self.terrain_collision.as_mut(){let(_,m,c)=air.source_records();records.motion.copy_from_slice(m);records.character.copy_from_slice(c);}
   }
  }
  self.refresh_homing_candidates(world);
 }
 fn normalize_shared_action(&mut self,input:&mut Input){
  if self.shared_charge{
   input.charge|=input.action_held;
   input.bounce_pressed=false;
   if !input.action_held{self.shared_charge=false;}
   return;
  }
  if !self.grounded||input.jump_pressed||!matches!(self.state,State::Idle|State::Run|State::Brake|State::Land){
   self.action_hold_ticks=0;return;
  }
  if input.action_held{
   if input.bounce_pressed{self.action_hold_ticks=1;}
   else if self.action_hold_ticks>0{self.action_hold_ticks=self.action_hold_ticks.saturating_add(1);}
   input.bounce_pressed=false;
   if self.action_hold_ticks>24{input.charge=true;self.shared_charge=true;self.action_hold_ticks=0;}
  }else if self.action_hold_ticks>0{
   input.bounce_pressed=self.action_hold_ticks>2;self.action_hold_ticks=0;
  }
 }
 fn tick_state(&mut self,mut input:Input,world:&dyn World){
 self.defeated.resize(world.target_count(),false);self.collected.resize(world.ring_count(),false);
 if input.bounce_pressed&&!input.jump_pressed&&!self.shared_charge&&self.charge<FULL_CHARGE&&self.begin_ring_dash(world).expect("contextual ring activation"){
  self.action_hold_ticks=0;input.bounce_pressed=false;
 }
 self.normalize_shared_action(&mut input);
 self.ticks+=1;
 if self.state==State::RingDash{self.tick_ring_dash(input,world);return;}
  if !self.grounded||!matches!(self.state,State::Run|State::Brake){self.turning=false;self.turn_finish_pending=false;}
  if self.grounded || !matches!(self.state,State::Jump|State::Fall) || input.bounce_pressed || (input.jump_pressed&&!self.air_dash_used){self.air_motion=None;}
  if !self.grounded || matches!(self.state,State::Homing|State::LightAttack){self.ground_motion=None;}
  if self.state==State::Homing&&self.homing_fallback.as_ref().is_some_and(|f|f.timeout_due()){
   let fallback=self.homing_fallback.as_mut().unwrap();assert!(fallback.decide_no_contact().expect("source fallback expiry"));
   let(e,m,c)=fallback.source_records();
   self.air_motion=Some(sa2_runtime::air_motion_host::AirMotion::from_fall_records(e,m,c).expect("expired fallback fall records"));
   if let Some(records)=self.terrain_collision.as_mut(){records.entity=e.to_vec();records.motion=m.to_vec();records.character=c.to_vec();}
   self.end_homing();self.jump_motion=JumpMotion::Intro;
   input.bounce_pressed=false;input.jump_pressed=false;
  }
  if self.state==State::Homing&&self.homing_fallback.is_none()&&self.attack_timer>=360&&world.source_terrain().is_some(){
   let contact=self.attack_target.and_then(|i|world.target(i).filter(|p|p.is_finite()&&self.defeated.get(i)==Some(&false)&&self.valid_attack_target(world,i)));
   if let Some(point)=contact{if self.finish_homing_contact(world,point){return;}}
   if let Some(records)=self.terrain_collision.as_mut().filter(|s|s.entity[0]==13){
    records.character[0x366..0x368].copy_from_slice(&self.attack_timer.to_be_bytes());
    assert!(sa2_runtime::original_homing_fallback::expire_no_contact(&mut records.entity,&mut records.character).expect("active homing expiry"));
    self.air_motion=Some(sa2_runtime::air_motion_host::AirMotion::from_fall_records(&records.entity,&records.motion,&records.character).expect("expired pursuit fall records"));
    self.end_homing();self.jump_motion=JumpMotion::Intro;
    input.bounce_pressed=false;input.jump_pressed=false;
   }
  }
  if self.state==State::Homing && input.bounce_pressed{
   // Source action13 resolves existing contact before its late action-button
   // cancellation. The demo host has no earlier contextual offered actions.
   let contact=self.attack_target.and_then(|i|world.target(i).filter(|p|p.is_finite()&&self.defeated.get(i)==Some(&false)&&self.valid_attack_target(world,i)));
   if let Some(point)=contact{if self.finish_homing_contact(world,point){return;}}
   let cancelled=self.homing_fallback.as_ref().map(|f|{let(e,m,c)=f.source_records();(e.to_vec(),m.to_vec(),c.to_vec())})
    .or_else(||self.terrain_collision.as_ref().filter(|s|s.entity[0]==13).map(|s|(s.entity.clone(),s.motion.clone(),s.character.clone())));
   if let Some((mut e,m,mut c))=cancelled{
    sa2_runtime::original_homing_fallback::cancel_action(&mut e,&mut c).expect("source homing cancellation");
    self.air_motion=Some(sa2_runtime::air_motion_host::AirMotion::from_fall_records(&e,&m,&c).expect("cancelled homing fall records"));
    if let Some(records)=self.terrain_collision.as_mut(){records.entity=e;records.motion=m;records.character=c;}
   }
   self.end_homing();self.jump_motion=JumpMotion::Intro;
   // Continue the new state's physics once; this press must not also bounce.
   input.bounce_pressed=false;input.jump_pressed=false;
  }
  if self.grounded&&self.state==State::Roll&&input.bounce_pressed&&!input.jump_pressed{
   if let Some(ground)=self.ground_motion.as_mut(){ground.cancel_roll();}
   self.state=State::Run;input.bounce_pressed=false;
  }
  if self.tick_somersault(input,world){self.finish_motion(input,world);return;}
  if self.decide_supported_homing(input,world){return;}
  if self.state==State::Homing {self.tick_homing(input,world);return;}
  if self.state==State::LightAttack {self.tick_light_attack(input,world);return;}
  // Bounce startup8052c..8056c/807f4..80834 clears100 for fallback; active acquisition
  // restores500 in initialize_homing. Capture before releasing bounce records.
  let incoming_draw_status=if matches!(self.state,State::BounceDown|State::BounceUp){self.source_draw_status()&!0x100}else{self.source_draw_status()};
  if self.tick_source_bounce(input,world){self.finish_motion(input,world);return;}
  let direction=input.direction.clamp_length_max(1.);let moving=direction.length_squared()>0.01;
  let desired=direction.normalize_or_zero();let mut horizontal=vec2(self.velocity.x,self.velocity.z);
  let source_jump=self.grounded&&input.jump_pressed;
  let source_ground=self.grounded&&(!input.charge||self.charge>=FULL_CHARGE);
  if !self.grounded{self.ground_motion=None;}
  let mut source_ground_applied=false;
  let mut source_air_applied=false;
  let releasing=self.previous_charge&&!input.charge&&(self.state==State::Charge||self.charge>=FULL_CHARGE);
  // Full readiness is held across takeoff. Resolve its release before ordinary
  // airborne physics, which otherwise consumes the branch and hides the edge.
  if releasing&&self.charge>=FULL_CHARGE&&!self.grounded{
   self.charge=0.;self.spin_charge_speed=0.;self.previous_charge=false;
   if let Some(target)=self.nearest_target_in(world){
    world.event(ActionEvent::LightAttackStart,self.position);self.state=State::LightAttack;self.attack_target=Some(target);self.attack_timer=0;self.light_steering_counter=0;self.velocity.y=16.*6.;self.last_attack_hits=0;
    self.air_motion=None;self.bounce_motion=None;self.homing_fallback=None;
    self.tick_light_attack(input,world);return;
   }
  }
  if source_jump{
   let mut ground=self.ground_motion.take().unwrap_or_else(||sa2_runtime::ground_motion_host::GroundMotion::new(&self.physics.encode()).expect("finite jump physics"));
   ground.configure(&self.physics.encode(),false).expect("finite jump physics");
   ground.set_experimental_surface_normal(self.ground_normal.to_array()).expect("source jump contact");
   let attached=ground.source_records().2[0xa0..0xa4]!=[0;4];
   let air=if attached {
    let records=self.terrain_collision.as_ref().expect("attached contact memory");
    let memory=sa2_runtime::original_contact::Memory{base:records.memory.base,bytes:&records.memory.bytes};
    ground.jump_with_platform(self.position.to_array(),self.velocity.to_array(),self.yaw,direction.to_array(),&memory,fallback_math())
   }else{ground.jump(self.position.to_array(),self.velocity.to_array(),self.yaw,direction.to_array(),fallback_math())}.expect("source jump handoff");
   let out=air.output();self.air_motion=Some(air);
   world.event(ActionEvent::Jump,self.position);
   self.position=Vec3::from_array(out.position);self.velocity=Vec3::from_array(out.velocity);self.yaw=out.yaw;horizontal=vec2(out.velocity[0],out.velocity[2]);
   self.grounded=false;self.state=State::Jump;self.jump_timer=0;self.jump_motion=JumpMotion::Intro;self.jumps+=1;if self.charge<FULL_CHARGE{self.charge=0.;}self.air_dash_used=false;
   input.jump_pressed=false;input.bounce_pressed=false;source_air_applied=true;
  }else if let Some(air)=&mut self.air_motion{
   air.configure(&self.physics.encode()).expect("finite air physics");
   let angle=((std::f32::consts::FRAC_PI_2-direction.x.atan2(direction.y))*(65536./std::f32::consts::TAU))as i32 as u32&0xffff;
   let context=sa2_runtime::original_movement::AirContext{directions:[angle,0],magnitudes:[direction.length(),0.],gravity:[0.,-1.,0.],level:0,control_mode:0,has_direction_surface:false};
   // Presentation Fall can still be original action6. Only action10 walkoff/
   // cancellation must suppress the held-jump budget, not ordinary descent.
   let source_jump_action=air.source_records().0[0]==6&&self.jump_motion!=JumpMotion::Rebound;
   let out=air.tick(self.position.to_array(),self.velocity.to_array(),input.jump_held&&source_jump_action,&context,fallback_math()).expect("ordinary air frame");
   self.position=Vec3::from_array(out.position);self.velocity=Vec3::from_array(out.velocity);self.yaw=out.yaw;horizontal=vec2(out.velocity[0],out.velocity[2]);source_air_applied=true;
  }else if self.grounded && input.charge && !input.jump_pressed && self.charge<FULL_CHARGE {
   if self.state!=State::Charge{
    world.event(ActionEvent::ChargeStart,self.position);self.charge_ticks=0;
    //80077b90/80077e7c: charge uses local forward speed or the jog minimum.
    self.spin_charge_speed=(horizontal.dot(vec2(self.yaw.sin(),self.yaw.cos()))/6.).max(self.p(8));
   }else if self.spin_charge_speed<10.{
    //80076630..48 checks before adding, so the final increment may exceed10.
    self.spin_charge_speed+=0.4;
   }
   let was_ready=self.charge>=FULL_CHARGE;
   self.charge_ticks=self.charge_ticks.saturating_add(1).min(120);
   self.state=State::Charge;self.charge=self.charge_ticks as f32/60.;
   if !was_ready&&self.charge>=FULL_CHARGE{world.event(ActionEvent::ChargeReady,self.position);self.state=if self.speed()>0.1{State::Run}else{State::Idle};}
   let bridge=self.ground_motion.get_or_insert_with(||sa2_runtime::ground_motion_host::GroundMotion::new_at_yaw(&self.physics.encode(),self.yaw).expect("charge physics"));
   bridge.configure(&self.physics.encode(),self.charge>=FULL_CHARGE).expect("charge physics");
   bridge.set_experimental_surface_normal(self.ground_normal.to_array()).expect("charge contact");
   let out=if self.state==State::Charge{
    bridge.charge_frame(self.position.to_array(),self.velocity.to_array(),self.yaw,direction.to_array(),fallback_math()).expect("charge force frame")
   }else{
    bridge.locomotion_frame(self.position.to_array(),self.velocity.to_array(),self.yaw,direction.to_array(),false,false,fallback_math()).expect("ready force frame").0
   };
   horizontal=vec2(out.velocity[0],out.velocity[2]);self.velocity.y=out.velocity[1];self.yaw=out.yaw;source_ground_applied=true;
  }else if releasing {
   if self.charge>=FULL_CHARGE-0.001 {
    if let Some(target)=self.nearest_target_in(world){world.event(ActionEvent::LightAttackStart,self.position);self.state=State::LightAttack;self.attack_target=Some(target);self.attack_timer=0;self.light_steering_counter=0;self.velocity.y=16.*6.;self.last_attack_hits=0;self.charge=0.;self.previous_charge=false;self.grounded=false;self.tick_light_attack(input,world);return;}
    // Ready release with no offered target clears readiness; it is not a spin launch.
    self.state=if self.speed()>0.1{State::Run}else{State::Idle};
    let bridge=self.ground_motion.get_or_insert_with(||sa2_runtime::ground_motion_host::GroundMotion::new_at_yaw(&self.physics.encode(),self.yaw).expect("ready release physics"));
    bridge.configure(&self.physics.encode(),false).expect("ready release physics");
    bridge.set_experimental_surface_normal(self.ground_normal.to_array()).expect("ready release contact");
    let out=bridge.locomotion_frame(self.position.to_array(),self.velocity.to_array(),self.yaw,direction.to_array(),false,false,fallback_math()).expect("ready release locomotion").0;
    horizontal=vec2(out.velocity[0],out.velocity[2]);self.velocity.y=out.velocity[1];self.yaw=out.yaw;source_ground_applied=true;
   }else{
    let bridge=self.ground_motion.get_or_insert_with(||sa2_runtime::ground_motion_host::GroundMotion::new(&self.physics.encode()).expect("release physics"));
    bridge.configure(&self.physics.encode(),false).expect("release physics");
    bridge.set_experimental_surface_normal(self.ground_normal.to_array()).expect("release contact");
    let out=bridge.release_spin(self.position.to_array(),self.velocity.to_array(),self.yaw,direction.to_array(),self.spin_charge_speed,fallback_math()).expect("spin release frame");
    horizontal=vec2(out.velocity[0],out.velocity[2]);self.velocity.y=out.velocity[1];self.yaw=out.yaw;
    world.event(ActionEvent::SpinDash,self.position);self.state=State::Roll;source_ground_applied=true;
   }
   self.charge=0.;self.spin_charge_speed=0.;
  }else{
   if self.grounded {
    if self.state==State::Roll {
     let bridge=self.ground_motion.get_or_insert_with(||sa2_runtime::ground_motion_host::GroundMotion::new(&self.physics.encode()).expect("roll physics"));
     bridge.configure(&self.physics.encode(),false).expect("roll physics");
     bridge.set_experimental_surface_normal(self.ground_normal.to_array()).expect("roll contact");
     let (out,exited)=bridge.roll_frame(self.position.to_array(),self.velocity.to_array(),self.yaw,direction.to_array(),fallback_math()).expect("rolling frame");
     horizontal=vec2(out.velocity[0],out.velocity[2]);self.velocity.y=out.velocity[1];self.yaw=out.yaw;source_ground_applied=true;
     if exited{self.state=if moving{State::Run}else if horizontal.length()>0.1{State::Brake}else{State::Idle};}
    }else if source_ground{
     let bridge=self.ground_motion.get_or_insert_with(||sa2_runtime::ground_motion_host::GroundMotion::new_at_yaw(&self.physics.encode(),self.yaw).expect("verified Sonic physics"));
     bridge.configure(&self.physics.encode(),self.charge>=FULL_CHARGE).expect("finite host physics");
     bridge.set_experimental_surface_normal(self.ground_normal.to_array()).expect("validated contact normal");
     let brake_finished=self.state==State::Brake&&self.brake_finish_pending;
     let (out,turned)=bridge.locomotion_frame(self.position.to_array(),self.velocity.to_array(),self.yaw,direction.to_array(),brake_finished,self.turn_finish_pending,fallback_math()).expect("ordinary locomotion frame");
     self.turn_finish_pending=false;
     let braking=bridge.source_records().0[0]==12;
     horizontal=vec2(out.velocity[0],out.velocity[2]);self.velocity.y=out.velocity[1];self.yaw=out.yaw;
     self.state=if braking{State::Brake}else if moving||horizontal.length()>0.1{State::Run}else{State::Idle};
     if turned{self.turning=true;}
     if self.state!=State::Run{self.turning=false;}
     source_ground_applied=true;
    }else if moving{
     let braking=horizontal.dot(desired)< -0.2;
     let limit=if self.charge>=FULL_CHARGE{0.5}else{self.p(3)};
     horizontal=move_towards(horizontal,desired*limit*6.*direction.length(),if braking{self.p(17).abs()*6.}else{self.p(14)*6.});
     self.state=if braking{State::Brake}else{State::Run};
    }else{
     //8004c734 ordinary ground parent: no-input forward drag above jog speed.
     // Keep this separate from rolling resistance and constant ground friction.
     let forward=vec2(self.yaw.sin(),self.yaw.cos());
     let speed=horizontal.dot(forward)/6.;
     if speed>self.p(9){horizontal+=forward*(self.p(20)*speed*6.);}
     horizontal=move_towards(horizontal,Vec2::ZERO,self.p(16).abs()*6.);
     self.state=if horizontal.length()>0.1{State::Brake}else{State::Idle};
    }
   }else if moving{
    let target=desired*horizontal.length().max(self.p(3)*6.);
    horizontal=move_towards(horizontal,target,self.p(15)*6.);
   }else{horizontal*=1.+self.p(20);}
   if !source_ground_applied && horizontal.length_squared()>0.02 && self.state!=State::Brake {
    let target=horizontal.x.atan2(horizontal.y);let difference=(target-self.yaw+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;
    self.yaw+=difference.clamp(-0.18,0.18);
   }
  }
  self.previous_charge=input.charge;
  let started_jump=source_jump || (input.jump_pressed && self.grounded);
  // The demo labels jump descent Fall, but original action6 remains active.
  let jump_descent=self.state==State::Fall&&matches!(self.jump_motion,JumpMotion::FallIntro|JumpMotion::FallLoop);
  let priority_homing=(matches!(self.state,State::Jump|State::BounceUp)||jump_descent)&&input.jump_pressed&&!self.air_dash_used;
  if input.bounce_pressed && !self.grounded && self.state!=State::BounceDown && !priority_homing {
   self.bounce_repeat=self.state==State::BounceUp;
   self.bounce_fall_loop=false;
   world.event(ActionEvent::BounceStart,self.position);self.state=State::BounceDown;
   // Source action6/70 enter69 at -5/-7 units per frame.
   self.velocity.y=if self.bounce_repeat{-7.*6.}else{-5.*6.};self.jump_timer=30;self.air_dash_used=false;
  }else if input.jump_pressed && self.grounded {
   world.event(ActionEvent::Jump,self.position);self.velocity.y=self.p(5)*6.;self.grounded=false;self.state=State::Jump;self.jump_timer=0;self.jump_motion=JumpMotion::Intro;self.jumps+=1;if self.charge<FULL_CHARGE{self.charge=0.;}
  }else if input.jump_pressed && !self.air_dash_used && !self.grounded {
   self.homing_draw_status=incoming_draw_status;
   if let Some(target)=self.nearest_homing_target_in(world){
    self.homing_launch_center=self.pre_movement_effect_center;
    world.event(ActionEvent::HomingStart,self.position);self.state=State::Homing;self.attack_target=Some(target);self.attack_timer=0;
    self.air_dash_used=true;self.initialize_homing(world,target);self.tick_homing(input,world);return;
   }
   self.homing_launch_center=self.pre_movement_effect_center;
   world.event(ActionEvent::HomingStart,self.position);
   self.homing_fallback=AirFallback::new(&self.physics.encode(),self.position.to_array(),self.velocity.to_array(),self.yaw,fallback_math()).ok().map(|f|f.after_air_dash_start(false));
   self.air_dash_used=true;self.state=State::Homing;self.attack_target=None;self.attack_timer=0;
   if self.homing_fallback.is_some(){self.tick_homing_fallback(input,world);}else{self.end_homing();}
   return;
  }
  if !self.grounded {
   let ordinary_jump=!self.air_dash_used && ((self.state==State::Jump && matches!(self.jump_motion,JumpMotion::Intro|JumpMotion::Spin)) || (self.state==State::Fall && matches!(self.jump_motion,JumpMotion::FallIntro|JumpMotion::FallLoop)));
   if !source_air_applied && ordinary_jump && input.jump_held && !started_jump{
    let held=self.jump_timer;self.jump_timer=self.jump_timer.saturating_add(1);
    if (held as i32)<self.physics.hang_time{self.velocity.y=(0.8_f32.mul_add(self.p(13),self.velocity.y/6.))*6.;}
   }
   if !source_air_applied{if ordinary_jump || matches!(self.state,State::BounceDown|State::BounceUp){
    // Upright, no external force specialization of8004b330 then80046de4.
    let source_speed=self.velocity.y/6.;
    self.velocity.y=(source_speed+self.p(21).mul_add(source_speed,-self.p(29)))*6.;
   }else{self.velocity.y-=self.p(29)*6.;}
   self.velocity.y=self.velocity.y.max(-self.p(2)*6.);
   }
   if self.velocity.y<0. && !matches!(self.state,State::BounceDown|State::BounceUp){
    if self.state==State::Jump && !self.air_dash_used && matches!(self.jump_motion,JumpMotion::Intro|JumpMotion::Spin){
     //80080374..ec: animation66 must pass80% of its17-frame motion.
     if self.jump_motion==JumpMotion::Intro || self.animation_frame.rem_euclid(16.)>0.8_f32 as f64*17.{
      self.jump_motion=JumpMotion::FallIntro;self.state=State::Fall;
     }
    }else if !(self.state==State::Jump && self.jump_motion==JumpMotion::Rebound && !self.air_dash_used){self.state=State::Fall;}
   }
  }
  if !source_air_applied{horizontal=horizontal.clamp_length_max(self.p(1)*6.);}
  self.velocity.x=horizontal.x;self.velocity.z=horizontal.y;
  let steps=((self.velocity.length()/60.)/(RADIUS*0.45)).ceil().max(1.)as usize;
  let source_contact=if source_ground_applied{self.move_source_terrain(world,input)}else if source_air_applied&&matches!(self.state,State::Jump|State::Fall){self.move_source_air_terrain(world,input)}else{false};
  if !source_contact{for _ in 0..steps{let before=self.bounced;world.move_player(self,1./60./steps as f32);if self.bounced>before{world.event(ActionEvent::BounceImpact,self.position);world.vibration(sa2_runtime::original_vibration::request(7,15,0),self.position);}}}
  // Contact can remove the normal component that triggered Run above. Decide
  // stationary presentation from resolved motion, not pressure into a slope.
  if self.grounded&&self.state==State::Run&&!moving&&self.speed()<=0.1 {
   self.state=State::Idle;self.turning=false;
  }
  self.finish_motion(input,world);
 }
 // Direct B is the demo input adapter; original shared-button/context selection
 // is intentionally separate from X charging. Source entry and recovery records
 // plus source force, contact, animation and attack classification are reused.
 fn tick_somersault(&mut self,input:Input,world:&dyn World)->bool{
  self.somersault_replay=false;
  let active=self.state==State::Somersault;
  let entry=!active&&self.grounded&&matches!(self.state,State::Idle|State::Run|State::Brake|State::Land)&&input.bounce_pressed&&!input.jump_pressed&&!input.charge&&self.charge==0.;
  if !entry&&!active{return false;}
  if active&&(!self.grounded||input.jump_pressed||input.charge){
   if let Some(g)=self.ground_motion.as_mut(){g.cancel_roll();}
   self.state=if self.grounded{State::Run}else{State::Fall};return false;
  }
  let mut advance=false;
  if entry{self.somersault_timer=0;self.somersault_fast=false;self.somersault_stage=1;self.somersault_queued=false;self.somersault_recovery=false;self.landing_animation=None;self.turning=false;}
  if active&&!self.somersault_recovery&&self.somersault_stage<3&&input.bounce_pressed{self.somersault_queued=true;}
  if active{
   self.somersault_timer=self.somersault_timer.wrapping_add(1);
   if let Some(clock)=self.locomotion_clock.as_ref(){
    let o=clock.output();
    if !self.somersault_recovery&&o.flags&(if self.somersault_stage==1{1}else{2})!=0&&(!self.somersault_fast||self.somersault_stage!=3||o.selected==99){
     if self.somersault_fast&&self.somersault_stage==1&&self.somersault_timer<=41{
      //80077dec: replay96, timer40 and continuation64 before normal dispatch.
      self.somersault_timer=40;self.somersault_queued=false;
      self.somersault_replay=true;
     }else if self.somersault_queued&&self.somersault_stage<3{
      self.somersault_stage+=1;self.somersault_queued=false;self.somersault_timer=0;advance=true;
     }else if self.somersault_stage==3{
      if let Some(g)=self.ground_motion.as_mut(){g.cancel_roll();}
      self.state=State::Run;return false;
     }else{self.somersault_recovery=true;}
    }
    if self.somersault_recovery&&(o.current==94||o.previous==94)&&o.flags&1!=0{
     if let Some(g)=self.ground_motion.as_mut(){g.cancel_roll();}
     self.state=State::Run;return false;
    }
   }
  }
  self.state=State::Somersault;self.previous_charge=false;
  let g=self.ground_motion.get_or_insert_with(||sa2_runtime::ground_motion_host::GroundMotion::new_at_yaw(&self.physics.encode(),self.yaw).expect("somersault records"));
  g.set_experimental_surface_normal(self.ground_normal.to_array()).expect("somersault normal");
  let out=g.somersault_frame(self.position.to_array(),self.velocity.to_array(),self.yaw,entry,self.somersault_recovery,self.somersault_stage,advance,self.somersault_replay,fallback_math()).expect("somersault forces");
  if entry{self.somersault_fast=g.source_records().0[0]==66;}
  self.velocity=Vec3::from_array(out.velocity);self.yaw=out.yaw;
  if !self.move_source_terrain(world,input){
   let steps=((self.velocity.length()/60.)/(RADIUS*0.45)).ceil().max(1.)as usize;
   for _ in 0..steps{world.move_player(self,1./60./steps as f32);}
  }
  if !self.somersault_recovery{
   self.fire_spawn=Some(crate::fire_effect::Spawn{player:0,center:(self.pre_movement_effect_center.unwrap_or_else(||self.effect_center())*10.).to_array(),velocity:(self.velocity/6.).to_array()});
  }
  true
 }
 fn finish_motion(&mut self,input:Input,world:&dyn World){
  if matches!(self.state,State::Roll|State::Somersault|State::BounceDown|State::BounceUp){
   for i in 0..world.target_count(){
    if let Some(t)=world.target(i){
     let profile=world.target_body_contact_profile(i);
     let eligible=match self.source_body_contact(&profile.descriptor,profile.category){
      Ok(Some(c))=>c.marks_receiver,
      Ok(None) if world.source_terrain().is_none()=>{
       // Host-physics compatibility path has no source records. Map its
       // supported move labels explicitly; do not pretend records were recovered.
       let classification=if self.state==State::BounceDown{0xe6}else{0xe5};
       sa2_runtime::original_attack_pair::marks_hit(classification,profile.descriptor[3],u32::from_be_bytes(profile.descriptor[4..8].try_into().unwrap()),profile.category).unwrap_or(false)
      },_=>false,
     };
     if !self.defeated[i]&&eligible&&self.attack_contact(world,i,t){self.defeated[i]=true;world.event(ActionEvent::TargetHit(i),self.position);}
    }
   }
  }
  if self.landing_animation.is_some() && self.grounded && self.speed()<0.1 && !input.charge && input.direction.length_squared()==0.{self.state=State::Land;}else{self.landing_animation=None;}
  let center=self.position+Vec3::Y*0.65;
  for i in 0..world.ring_count(){if let Some(r)=world.ring(i){if !self.collected[i]&&center.distance(r)<0.8{self.collected[i]=true;world.event(ActionEvent::RingCollected(i),self.position);}}}
  self.max_height=self.max_height.max(self.position.y);self.max_speed=self.max_speed.max(self.speed());
  if world.should_respawn(self.position) || !self.position.is_finite(){self.reset_in(world);world.event(ActionEvent::Respawn,self.position);}
 }
 /// Retained source airborne action for presentation; grounded/stale records
 /// must not continue emitting airborne effects.
 pub fn source_air_action(&self)->Option<u8>{
  if self.grounded||!matches!(self.state,State::Jump|State::Fall){None}else{self.air_motion.as_ref().map(|a|a.source_records().0[0])}
 }
 /// Source common-update center rotates the local height before translation.
 /// Visual attachment only; the host attack-volume contract is separate.
 pub fn fire_spawn(&self)->Option<crate::fire_effect::Spawn>{self.fire_spawn}
 pub fn effect_center(&self)->Vec3{self.position+self.model_rotation().y_axis*(self.p(31)*0.1)}
 /// Action6 creates its aura before the common collision-center refresh.
 /// Captured once per fixed tick; reset clears it. Other effects retain their
 /// supplied center until their own scheduling has been verified.
 pub fn jump_aura_center(&self,current:Vec3)->Vec3{
  if self.source_air_action()==Some(6){self.pre_movement_effect_center.unwrap_or(current)}else{current}
 }
 pub fn homing_launch_center(&self)->Option<Vec3>{self.homing_launch_center}
 /// Jump and bounce-down factories both run before the common center refresh.
 pub fn aura_snapshot_center(&self,current:Vec3)->Vec3{
  if self.state==State::BounceDown{
   self.pre_movement_effect_center.unwrap_or(current)
  }else{self.jump_aura_center(current)}
 }
 pub fn attack_center(&self)->Vec3{
  let entity=if matches!(self.state,State::Homing|State::LightAttack){
   let action=if self.state==State::Homing{13}else{60};
   self.terrain_collision.as_ref().filter(|r|r.entity[0]==action).map(|r|r.entity.as_slice())
  }else if matches!(self.state,State::BounceDown|State::BounceUp){
   self.bounce_motion.as_ref().map(|b|b.records().entity.as_slice())
  }else if !self.grounded && matches!(self.state,State::Jump|State::Fall){
   self.air_motion.as_ref().map(|a|a.source_records().0)
  }else if self.grounded{self.ground_motion.as_ref().map(|g|g.source_records().0)}else{None};
  match entity{
   Some(e)=>source_attack_center(e,self.position,self.p(31)),
   None=>self.position+Vec3::Y*(self.p(31)*0.1),
  }
 }
 /// Classify an already-overlapping receiver using retained attack records.
 /// None means this move has no supported live-record bridge; no state is invented.
 pub fn source_body_contact(&self,receiver:&[u8],category:u16)->Result<Option<sa2_runtime::original_attack_pair::ClassifiedContact>,String>{
  let records=if matches!(self.state,State::BounceDown|State::BounceUp){
   self.bounce_motion.as_ref().map(|b|{let r=b.records();(r.entity.as_slice(),r.character.as_slice())})
  }else if matches!(self.state,State::Roll|State::Somersault){
   self.ground_motion.as_ref().map(|g|{let(e,_,c)=g.source_records();(e,c)})
  }else if matches!(self.state,State::Homing|State::LightAttack){
   let action=if self.state==State::Homing{13}else{60};
   self.terrain_collision.as_ref().filter(|r|r.entity[0]==action)
    .map(|r|(r.entity.as_slice(),r.character.as_slice()))
  }else{None};
  let Some((entity,character))=records else{return Ok(None)};
  // Primary Sonic descriptor801bb18c; only header fields are consumed here.
  let mut descriptor=[0u8;0x30];descriptor[..8].copy_from_slice(&[0,7,7,0xe0,0,0,4,0x20]);
  sa2_runtime::original_attack_pair::classify_sonic_contact(entity,character,&descriptor,receiver,category).map(Some)
 }
 fn targeted_hit_eligible(&self,world:&dyn World,i:usize)->bool{
  let profile=world.target_body_contact_profile(i);
  match self.source_body_contact(&profile.descriptor,profile.category){
   Ok(Some(contact))=>contact.marks_receiver,
   Ok(None)=>{
    // Early contact can precede source movement. Explicit ordinary-Sonic
    // adapter for absent records and host physics; no powerups are inferred.
    let classification=match self.state{State::Homing=>0xe5,State::LightAttack=>0xee,_=>return false};
    sa2_runtime::original_attack_pair::marks_hit(classification,profile.descriptor[3],u32::from_be_bytes(profile.descriptor[4..8].try_into().unwrap()),profile.category).unwrap_or(false)
   },Err(_)=>false,
  }
 }
 fn valid_attack_target(&self,world:&dyn World,i:usize)->bool{
  let radius=world.target_radius(i);
  world.target_attackable(i)&&radius.is_finite()&&radius>=0.
 }
 fn attack_contact(&self,world:&dyn World,i:usize,point:Vec3)->bool{
  if !self.valid_attack_target(world,i){return false;}
  // The first overlap can precede pursuit movement and its retained records.
  // Prepare a local copy so that query timing cannot select an upright volume.
  let action=match self.state{State::Homing=>Some(13),State::LightAttack=>Some(60),_=>None};
  let prepared=action.filter(|action|self.terrain_collision.as_ref().is_none_or(|r|r.entity[0]!=*action))
   .and_then(|action|self.prepare_source_attack(world,action));
  let center=prepared.as_ref().map_or_else(||self.attack_center(),|r|source_attack_center(&r.entity,self.position,self.p(31)));
  sphere_overlap(center,self.p(27)*0.1,point,world.target_radius(i))
 }
 pub fn nearest_target(&self)->Option<usize>{self.nearest_target_in(&DemoWorld)}
 pub fn nearest_target_in(&self,world:&dyn World)->Option<usize>{self.select_target(world,false)}
 pub fn select_light_target(&self,world:&dyn World)->Option<usize>{
  let mut best=160000.;let mut selected=None;
  for i in 0..world.target_count(){
   if self.defeated.get(i).copied().unwrap_or(false)||!self.valid_attack_target(world,i)||!matches!(world.light_target_category(i),2|3){continue;}
   let Some(point)=world.target(i).filter(|p|p.is_finite())else{continue;};
   if !world.attack_path_clear(self.position,point){continue;}
   let Some(distance)=world.light_target_distance(i,self.position)else{continue;};
   if distance.is_finite()&&distance>=0.&&distance<best{best=distance;selected=Some(i);}
  }
  selected
 }
 pub fn nearest_homing_target_in(&self,world:&dyn World)->Option<usize>{
  use sa2_runtime::original_gameplay::{HomingCandidate,select_homing_candidate};
  let initial;
  let snapshot=if let Some(cached)=self.homing_candidates.as_ref(){cached}else{initial=self.build_homing_candidates(world);&initial};
  let mut candidates=Vec::new();let mut slots=Vec::new();
  for &(i,metric) in snapshot{
   if i>=world.target_count()||self.defeated.get(i).copied().unwrap_or(false){continue;}
   let Some(center)=world.target(i).filter(|v|v.is_finite())else{continue;};
   let eligible=self.valid_attack_target(world,i)&&world.attack_path_clear(self.position,center);
   candidates.push(HomingCandidate{kind:world.homing_target_kind(i),center:eligible.then_some((center*10.).to_array()),position:[0.;3],world_center:true,metric});
   slots.push(i);
  }
  let scale=65536./std::f32::consts::TAU;
  let heading=0x4000u32.wrapping_sub((self.yaw*scale)as i32 as u32)&0xffff;
  select_homing_candidate(&candidates,(self.position*10.).to_array(),heading,-1.,0,false,fallback_math()).ok().flatten().map(|i|slots[i])
 }
 fn build_homing_candidates(&self,world:&dyn World)->Vec<(usize,f32)>{
  let mut entries=Vec::new();
  for i in 0..world.target_count(){
   if self.defeated.get(i).copied().unwrap_or(false)||world.homing_target_kind(i)==17{continue;}
   let Some(origin)=world.target_origin(i).filter(|v|v.is_finite())else{continue;};
   if world.target(i).is_none(){continue;}
   if let Ok(Some(metric))=sa2_runtime::original_gameplay::homing_list_metric((self.position*10.).to_array(),(origin*10.).to_array(),world.homing_target_flags(i),entries.len()){entries.push((i,metric));}
  }
  entries
 }
 /// Refresh after host collision processing or an out-of-band teleport. tick_in
 /// also refreshes after after_tick; reset clears the snapshot for bootstrap.
 pub fn refresh_homing_candidates(&mut self,world:&dyn World){self.homing_candidates=Some(self.build_homing_candidates(world));}
 fn select_target(&self,world:&dyn World,homing:bool)->Option<usize>{
  (0..world.target_count()).filter(|i|self.valid_attack_target(world,*i)).filter_map(|i|world.target(i).filter(|t|t.is_finite()).map(|t|(i,t)))
   .filter(|(_,t)|!homing||{
    let delta=*t-self.position;let scale=65536./std::f32::consts::TAU;
    let from=(self.yaw*scale)as i32 as u32;let to=(delta.x.atan2(delta.z)*scale)as i32 as u32;
    sa2_runtime::original_gameplay::angle_distance(from,to)<=0x5000
   })
   .filter(|(i,t)|!self.defeated.get(*i).copied().unwrap_or(false)&&self.position.distance(*t-Vec3::Y*(self.p(31)*0.1))<12.&&world.attack_path_clear(self.position,*t-Vec3::Y*(self.p(31)*0.1)))
   .min_by(|(_,a),(_,b)|self.position.distance_squared(*a).total_cmp(&self.position.distance_squared(*b))).map(|(i,_)|i)
 }
 /// Recovered aim/steering/projection; target selection remains a host adaptation.
 fn homing_direction(&mut self,delta:Vec3)->Option<Vec3>{
  let angle_scale=65536./std::f32::consts::TAU;
  let current=0x4000u32.wrapping_sub((self.yaw*angle_scale)as i32 as u32)&0xffff;
  let source_delta=delta*10.;
  let target_angle=sa2_runtime::original_gameplay::homing_aim(source_delta.x,source_delta.z,-1.,fallback_math()).ok()?;
  let source_heading=sa2_runtime::original_gameplay::homing_heading(current,target_angle,self.attack_timer as i16,false,0);
  let direction=sa2_runtime::original_gameplay::homing_direction(source_delta.to_array(),source_heading,-1.,fallback_math()).ok()?;
  let heading=0x4000u32.wrapping_sub(source_heading)&0xffff;
  self.yaw=heading as f32/angle_scale;
  Some(Vec3::from_array(direction))
 }
 fn initialize_homing(&mut self,world:&dyn World,index:usize){
  let Some(point)=world.target(index).filter(|p|p.is_finite())else{return;};
  let mut delta=point-self.position;
  let (vertical,counter)=sa2_runtime::original_homing_fallback::initial_vertical(delta.y*10.,-1.,-1,false).expect("finite target delta");
  if counter<=0{self.homing_draw_status|=0x500;}
  delta.y=vertical*0.1;
  // Coincident points are resolved by the contact decision, not normalization.
  if let Some(direction)=self.homing_direction(delta){self.velocity=direction*30.;}
  if counter>0{
   self.homing_fallback=AirFallback::new(&self.physics.encode(),self.position.to_array(),self.velocity.to_array(),self.yaw,fallback_math()).ok().map(AirFallback::after_upward_start);
   self.attack_target=None;
  }
 }
 fn tick_homing(&mut self,input:Input,world:&dyn World){
  if self.homing_fallback.is_some(){self.tick_homing_fallback(input,world);return;}
  self.attack_timer=self.attack_timer.saturating_add(1);
  if self.attack_timer>360{self.end_homing();return;}
  let previous=self.attack_target;
  // Invalid/disabled host collision volumes still cancel the attack. This is
  // the host validation contract, distinct from an absent source candidate.
  if previous.is_some_and(|i|world.target(i).is_some_and(|p|!p.is_finite()||!self.valid_attack_target(world,i))){self.end_homing();return;}
  if let Some(point)=previous.and_then(|i|world.target(i).filter(|t|t.is_finite()&&self.defeated.get(i)==Some(&false)&&self.valid_attack_target(world,i))){
   // Retain the host's explicit obstruction policy and prior-contact decision.
   if !world.attack_path_clear(self.position,point){self.end_homing();return;}
   if self.finish_homing_contact(world,point){return;}
  }
  self.attack_target=self.nearest_homing_target_in(world);
  let point=self.attack_target.and_then(|i|world.target(i).filter(|t|t.is_finite()&&self.defeated.get(i)==Some(&false)&&self.valid_attack_target(world,i)));
  let Some(point)=point else{
   if previous.is_some(){
    let initial;
    let snapshot=if let Some(cache)=self.homing_candidates.as_ref(){cache}else{initial=self.build_homing_candidates(world);&initial};
    let empty=!snapshot.iter().any(|&(i,_)|world.target(i).is_some()&&!self.defeated.get(i).copied().unwrap_or(true));
    let fallback=if world.source_terrain().is_some(){
     self.terrain_collision.as_ref().filter(|s|s.entity[0]==13).map(AirFallback::from_collision)
    }else{None};
    self.homing_fallback=fallback.unwrap_or_else(||AirFallback::new(&self.physics.encode(),self.position.to_array(),self.velocity.to_array(),self.yaw,fallback_math())).ok().map(|f|f.after_target_loss(empty,self.attack_timer.saturating_sub(1)));
    self.attack_target=None;
    if self.homing_fallback.is_some(){self.tick_homing_fallback(input,world);return;}
   }
   self.end_homing();return;
  };
  let target=point; // Original aim is target collider center minus entity position.
  if self.attack_timer>360 || !world.attack_path_clear(self.position,target){self.end_homing();return;}
  if self.finish_homing_contact(world,point){return;}
  let delta=target-self.position;
  let Some(direction)=self.homing_direction(delta)else{self.end_homing();return;};
  // Original8007b7f0: ordinary Sonic uses5SAunits/tick (host scale0.1).
  // Original8007b7f4..834; private host RNG stream, not global SA2 scheduling.
  let speed=if self.attack_timer>180{
   let random=sa2_runtime::original_effect_state::random(&mut self.homing_seed)as f32/32768.;
   30.*0.1_f32.mul_add(random,0.7)
  }else{30.};
  self.velocity=direction*speed;
  if !self.move_attack(world,direction*(speed/60_f32).min(delta.length()),input){self.end_homing();return;}
  self.finish_homing_contact(world,point);
 }
 fn finish_homing_contact(&mut self,world:&dyn World,point:Vec3)->bool{
  if self.attack_target.is_some_and(|i|self.attack_contact(world,i,point)){
   // Contact can precede the first pursuit sweep. Prepare the same source
   // records without moving Sonic or running collision a second time.
   if self.terrain_collision.as_ref().is_none_or(|r|r.entity[0]!=13){
    if let Some(records)=self.prepare_source_attack(world,13){self.terrain_collision=Some(records);}
   }
   let response=world.homing_contact_response(self.attack_target.unwrap());
   if let Some(i)=self.attack_target{if self.targeted_hit_eligible(world,i){self.defeated[i]=true;world.event(ActionEvent::TargetHit(i),self.position);}}
   self.attack_target=None;self.state=State::Jump;
   match response{
    HomingContactResponse::DemoUpward=>self.velocity=Vec3::Y*6.,
    HomingContactResponse::SourceCategory0Upright=>{
     if let Some(records)=self.terrain_collision.as_mut().filter(|r|r.entity[0]==13){
      records.character[0x35e]=self.rebound_cycle;
      sa2_runtime::original_homing_fallback::category0_contact(&mut records.entity,&mut records.motion,&mut records.character,fallback_math()).expect("verified category0 contact");
      let air=sa2_runtime::air_motion_host::AirMotion::from_homing_contact(&records.entity,&records.motion,&records.character).expect("category0 rebound handoff");
      let out=air.output();self.velocity=Vec3::from_array(out.velocity);self.yaw=out.yaw;self.air_motion=Some(air);
     }else{
      // Host-only/early-contact adapter has no source orientation records.
      if let Ok(v)=sa2_runtime::original_homing_fallback::category0_world_impulse((self.velocity/6.).to_array(),fallback_math()){self.velocity=Vec3::from_array(v)*6.;}
     }
    },
    HomingContactResponse::SourceCategory2|HomingContactResponse::SourceCategory3=>{
     if let Some(records)=self.terrain_collision.as_mut().filter(|r|r.entity[0]==13){
      let category=if response==HomingContactResponse::SourceCategory2{2}else{3};
      records.character[0x35e]=self.rebound_cycle;
      sa2_runtime::original_homing_fallback::contact_transition(&mut records.entity,&mut records.character,category).expect("verified homing contact transition");
      self.air_motion=Some(sa2_runtime::air_motion_host::AirMotion::from_homing_contact(&records.entity,&records.motion,&records.character).expect("homing rebound handoff"));
     }
    }
   }
   self.air_dash_used=false;self.jump_timer=30;self.jump_motion=JumpMotion::Rebound;
   self.rebound_variant=self.rebound_cycle;self.rebound_cycle=(self.rebound_cycle+1)%3;
   return true;
  }
  false
 }
 fn decide_supported_homing(&mut self,input:Input,world:&dyn World)->bool{
  if self.state!=State::Homing||world.source_terrain().is_none(){return false;}
  let Some(mut records)=self.terrain_collision.clone().filter(|s|s.entity[0]==13&&s.entity[5]&3!=0)else{return false;};
  if let Some(fallback)=self.homing_fallback.as_mut(){
   let expired=fallback.decide_no_contact().expect("fallback supported decision");
   let(e,m,c)=fallback.source_records();records.entity=e.to_vec();records.motion=m.to_vec();records.character=c.to_vec();
   if expired{self.terrain_collision=Some(records);self.end_homing();return false;}
  }else{
   if let Some(point)=self.attack_target.and_then(|i|world.target(i).filter(|p|p.is_finite()&&self.defeated.get(i)==Some(&false)&&self.valid_attack_target(world,i))){
    if self.finish_homing_contact(world,point){return true;}
   }
   // Let the existing timeout path run before any supported landing selection.
   if self.attack_timer>=360{return false;}
   self.attack_timer+=1;
   records.character[0x366..0x368].copy_from_slice(&self.attack_timer.to_be_bytes());
  }
  let direction=((std::f32::consts::FRAC_PI_2-input.direction.x.atan2(input.direction.y))*65536./std::f32::consts::TAU)as i32 as u32&0xffff;
  let control=sa2_runtime::original_control::ControlContext{directions:[direction,0],magnitudes:[input.direction.length().min(1.),0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0};
  self.grounded=true;
      let memory=sa2_runtime::original_contact::Memory{base:0x80210000,bytes:&records.workspace};
      sa2_runtime::original_homing_landing::advance(&mut records.entity,&records.motion,&mut records.character,&control,&memory,&mut records.control_cache,Some(self.heading_history.delayed_angles(3)[1]),self.ticks as u32,&mut self.idle_seed,true,fallback_math()).expect("source homing landing decision");
      let action=records.entity[0];let selected=u16::from_be_bytes(records.character[0x176..0x178].try_into().unwrap());
      let mut ground=sa2_runtime::ground_motion_host::GroundMotion::from_collision_state(&records).expect("homing landed records");
      self.velocity=Vec3::from_array(ground.reconciled_terrain_velocity(fallback_math()).expect("homing landed velocity"));
      self.ground_motion=Some(ground);self.air_motion=None;self.landings+=1;
      self.homing_fallback=None;self.attack_target=None;self.air_dash_used=false;
      self.landing_animation=if action==0{Some(selected)}else{None};
      self.state=match action{12=>State::Brake,0=>State::Land,_=>State::Run};
  self.terrain_collision=Some(records);
  false
 }
 fn tick_homing_fallback(&mut self,input:Input,world:&dyn World){
  let result=self.homing_fallback.as_mut().unwrap().tick(self.position.to_array(),input.direction.to_array(),fallback_math());
  let Ok(out)=result else{self.end_homing();return;};
  self.velocity=Vec3::from_array(out.velocity);self.yaw=out.yaw;
  if out.expired{
   // The source decision clears action13 and attack flags before returning.
   // Preserve that result for the next controller instead of leaving a stale
   // attacking collision record after dropping the fallback adapter.
   if let Some(records)=self.terrain_collision.as_mut(){
    let(e,m,c)=self.homing_fallback.as_ref().unwrap().source_records();
    records.entity=e.to_vec();records.motion=m.to_vec();records.character=c.to_vec();
   }
   self.end_homing();return;
  }
  if let Some(terrain)=world.source_terrain(){
   let ground=sa2_runtime::ground_motion_host::GroundMotion::new(&self.physics.encode()).expect("fallback physics");
   let mut records=self.terrain_collision.clone().unwrap_or_else(||terrain.state(&ground));
   let direction=((std::f32::consts::FRAC_PI_2-input.direction.x.atan2(input.direction.y))*65536./std::f32::consts::TAU)as i32 as u32&0xffff;
   let common=sa2_runtime::original_position_a::PositionContext{control:sa2_runtime::original_control::ControlContext{directions:[direction,0],magnitudes:[input.direction.length().min(1.),0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0},incoming_f26:0.,entity_identity:0x80210100,motion_identity:0x80210600,character_identity:0x80210200,workspace_identity:0x80210000,platform_offset_identity:0x80212000};
   let ctx=sa2_runtime::original_position_b::PositionBContext{common:&common,direction_mode:0,special_offset:0.,special_radius:0.};
   world.prepare_platform_memory(&mut records.memory).expect("fallback platform memory");
   match self.homing_fallback.as_mut().unwrap().resolve_source_terrain_with_platform(&mut records,&terrain.world(),&ctx,fallback_math(),|a,p,e,s,m|world.platform_callback(a,p,e,s,m)){
    Ok(contact)=>{
     self.position=Vec3::from_array(contact.position);self.velocity=Vec3::from_array(contact.velocity);self.grounded=false;
     self.terrain_collision=Some(records);
    },
    Err(_)=>{self.end_homing();}
   }
   return;
  }
  let before=self.position;let delta=self.velocity/60.;
  let steps=(delta.length()/0.15).ceil().max(1.)as usize;
  for _ in 0..steps{
   let expected=self.position+self.velocity*(1./60./steps as f32);
   world.move_player(self,1./60./steps as f32);
   if !self.position.is_finite()||!self.velocity.is_finite(){self.position=before;self.velocity=Vec3::ZERO;self.end_homing();return;}
   if self.grounded{self.homing_fallback=None;self.attack_target=None;self.air_dash_used=false;self.state=State::Land;return;}
   if self.position.distance_squared(expected)>0.000001{self.end_homing();return;}
  }
 }
 /// Source action13 expiry changes action/flags without assigning a new impulse.
 fn end_homing(&mut self){self.homing_fallback=None;self.state=State::Fall;self.attack_target=None;self.air_dash_used=true;self.grounded=false;}
 fn end_light_attack(&mut self){self.state=State::Fall;self.attack_target=None;self.velocity=-Vec3::Y*(self.p(5)*6.);self.air_dash_used=true;self.grounded=false;}
 /// Let the host sweep/resolve the body even when its target-visibility test passes.
 fn move_attack(&mut self,world:&dyn World,delta:Vec3,input:Input)->bool{
  if let Some(ok)=self.move_source_attack(world,13,input){return ok;}
  let count=(delta.length()/0.15).ceil().max(1.) as usize;
  let step=delta/count as f32;let dt=1./60./count as f32;
  let velocity=self.velocity;
  for _ in 0..count{
   let previous=self.position;
   let expected=previous+step;
   self.velocity=step/dt;self.grounded=false;
   world.move_player(self,dt);
   if !self.position.is_finite(){self.position=previous;return false;}
   if self.position.distance_squared(expected)>0.000001{return false;}
  }
  self.velocity=velocity;self.grounded=false;true
 }
 /// Action60 does not cancel merely because the collision routine resolved movement.
 fn move_light_attack(&mut self,world:&dyn World,input:Input)->bool{
  if let Some(ok)=self.move_source_attack(world,60,input){return ok;}
  // Original position parent clears the previous grounded bit before its sweep.
  self.grounded=false;
  let count=((self.velocity.length()/60.)/0.15).ceil().max(1.)as usize;
  let dt=1./60./count as f32;
  for _ in 0..count{
   let previous=self.position;
   world.move_player(self,dt);
   if !self.position.is_finite()||!self.velocity.is_finite(){self.position=previous;return false;}
  }
  true
 }
 fn exit_light_without_target(&mut self,input:Input,world:&dyn World){
  use sa2_runtime::{original_light_attack,original_ground_accel::GroundContext,original_ground_camera::CameraGroundContext,original_contact::Memory};
  let empty=sa2_runtime::static_terrain_host::StaticTerrain::new(&[]).expect("empty source terrain");
  let terrain=world.source_terrain().unwrap_or(&empty);
  let ground=sa2_runtime::ground_motion_host::GroundMotion::new(&self.physics.encode()).expect("light exit physics");
  let mut records=self.terrain_collision.clone().unwrap_or_else(||terrain.state(&ground));
  if records.entity[0]!=60{records.entity[4..6].copy_from_slice(&(0x500u16|if self.grounded{1}else{0}).to_be_bytes());}
  records.entity[0]=60;
  let yaw=((std::f32::consts::FRAC_PI_2-self.yaw)*65536./std::f32::consts::TAU)as i32 as u32&0xffff;
  records.entity[12..16].copy_from_slice(&yaw.to_be_bytes());
  for i in 0..3{
   records.entity[0x14+4*i..0x18+4*i].copy_from_slice(&(self.position[i]*10.).to_be_bytes());
   records.motion[4*i..4*i+4].copy_from_slice(&(self.velocity[i]/6.).to_be_bytes());
  }
  records.character[0xc0..0x144].copy_from_slice(&self.physics.encode());
  let local=sa2_runtime::original_ground::rotate_vector(&records.entity,(self.velocity/6.).to_array(),false,fallback_math()).expect("light exit local velocity");
  for i in 0..3{records.character[0x64+4*i..0x68+4*i].copy_from_slice(&local[i].to_be_bytes());}
  let direction=((std::f32::consts::FRAC_PI_2-input.direction.x.atan2(input.direction.y))*65536./std::f32::consts::TAU)as i32 as u32&0xffff;
  let control=sa2_runtime::original_control::ControlContext{directions:[direction,0],magnitudes:[input.direction.length().min(1.),0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0};
  let ctx=GroundContext{camera:CameraGroundContext{control:&control,stick_x:[0;2],cameras:[0;2],incoming_f28:0.,path_scratch:[0;0x44]},ordinary_path_scratch:[0;0x44]};
  original_light_attack::advance_without_target(&mut records.entity,&mut records.motion,&mut records.character,&ctx,&Memory{base:0x80210000,bytes:&records.workspace},&mut records.control_cache,fallback_math()).expect("light no-target forces");
  original_light_attack::exit_failed_pursuit(&mut records.entity,&mut records.character).expect("light failed pursuit exit");
  self.state=State::Fall;self.attack_target=None;self.air_dash_used=true;self.jump_motion=JumpMotion::Intro;
  if world.source_terrain().is_some(){
   records.entity[0x2c..0x30].copy_from_slice(&0x80213000_u32.to_be_bytes());
   records.memory.bytes[0x13000..0x130b0].fill(0);records.memory.bytes[0x13011]=255;
   let common=sa2_runtime::original_position_a::PositionContext{control,incoming_f26:0.,entity_identity:0x80210100,motion_identity:0x80210600,character_identity:0x80210200,workspace_identity:0x80210000,platform_offset_identity:0x80212000};
   let ctx=sa2_runtime::original_position_b::PositionBContext{common:&common,direction_mode:0,special_offset:0.,special_radius:0.};
   world.prepare_platform_memory(&mut records.memory).expect("light exit platform memory");
   sa2_runtime::original_collision_loop::update(&mut records,&mut 0,&terrain.world(),&ctx,fallback_math(),|a,p,e,s,m|world.platform_callback(a,p,e,s,m)).expect("light exit source collision");
   sa2_runtime::original_movement::reconcile_velocity(&mut records.entity,&mut records.motion,&mut records.character,[0.,-1.,0.],0,fallback_math()).expect("light exit reconciliation");
   self.position=Vec3::from_array(std::array::from_fn(|i|f32::from_be_bytes(records.entity[0x14+4*i..0x18+4*i].try_into().unwrap())*0.1));
   self.grounded=records.entity[5]&3!=0;
   self.air_motion=Some(sa2_runtime::air_motion_host::AirMotion::from_completed_fall_contact(&records.entity,&records.motion,&records.character).expect("light exit fall handoff"));
   self.light_landing_pending=self.grounded;
  }
  self.velocity=Vec3::from_array(std::array::from_fn(|i|f32::from_be_bytes(records.motion[4*i..4*i+4].try_into().unwrap())*6.));
  self.yaw=std::f32::consts::FRAC_PI_2-u32::from_be_bytes(records.entity[12..16].try_into().unwrap())as f32*std::f32::consts::TAU/65536.;
  self.terrain_collision=Some(records);
  if world.source_terrain().is_none(){
   let position=self.position;let velocity=self.velocity;let grounded=self.grounded;
   world.move_player(self,1./60.);
   if !self.position.is_finite()||!self.velocity.is_finite(){self.position=position;self.velocity=velocity;self.grounded=grounded;}
  }
 }
 fn tick_light_attack(&mut self,input:Input,world:&dyn World){
  if self.attack_target.is_some_and(|i|world.target(i).is_some_and(|p|!p.is_finite()||!self.valid_attack_target(world,i))){self.end_light_attack();return;}
  // Source selector8007ad3c resets366on every action60call; no elapsed expiry.
  self.attack_timer=self.attack_timer.saturating_add(1);
  self.attack_target=self.select_light_target(world);
  let Some(index)=self.attack_target else{self.exit_light_without_target(input,world);return;};
  let Some(point)=world.target(index).filter(|t|t.is_finite()&&self.defeated.get(index)==Some(&false)&&self.valid_attack_target(world,index)) else{self.end_light_attack();return;};
  let delta=point-self.position;
  if !world.attack_path_clear(self.position,point){self.end_light_attack();return;}
  self.light_steering_counter=self.light_steering_counter.wrapping_add(1);
  if self.light_steering_counter>=2{
   use sa2_runtime::original_light_attack::{normalized_inputs,weights,blend};
   // Explicit upright host adapter: SA positions x10, velocities /6.
   let result=normalized_inputs((delta*10.).to_array(),(self.velocity/6.).to_array(),[0.,-1.,0.],fallback_math())
    .and_then(|(target,current)|{
     let magnitude=input.direction.length().min(1.);
     weights(self.light_steering_counter,if magnitude>0.{Some(magnitude)}else{None})
      .and_then(|w|blend(target,current,w,fallback_math()))
    });
   let Ok(direction)=result else{self.end_light_attack();return;};
   self.velocity=(Vec3::from_array(direction)*10.)*6.;
  }
  if self.velocity.x*self.velocity.x+self.velocity.z*self.velocity.z>0.{self.yaw=self.velocity.x.atan2(self.velocity.z);}
  if !self.move_light_attack(world,input){self.end_light_attack();return;}
  if self.attack_contact(world,index,point)&&self.targeted_hit_eligible(world,index){
   self.defeated[index]=true;world.event(ActionEvent::TargetHit(index),self.position);self.chain_hits+=1;self.last_attack_hits+=1;
   self.attack_target=self.select_light_target(world);
  }
 }
 /// Apply the demo's support response after the host resolves a descending contact.
 /// Position is Sonic's feet; normal must be finite, upward and unit length.
 /// Call only for actual support contact, not upward passage through a plane.
 /// This shares current landing/bounce behavior; it is not the full SA2 collision solver.
 pub fn resolve_support(&mut self,position:Vec3,normal:Vec3)->Result<(), &'static str>{
  if !position.is_finite()||!normal.is_finite()||normal.y<=0.||(normal.length_squared()-1.).abs()>0.0001||!self.velocity.is_finite(){return Err("invalid support contact");}
  self.position=position;
  if !self.grounded{self.landings+=1;self.landing_animation=Some(16);}
  if self.state==State::BounceDown{
   self.velocity.y=if self.bounce_repeat{3.6*6.}else{2.9*6.};
   self.grounded=false;self.state=State::BounceUp;self.bounced+=1;self.jump_timer=30;self.landing_animation=None;
  }else{
   if self.ground_motion.is_some()&&matches!(self.state,State::Idle|State::Run|State::Brake|State::Land|State::Roll|State::Somersault){self.velocity-=normal*self.velocity.dot(normal);}
   else{self.velocity.y=0.;}
   self.grounded=true;
  }
  self.air_dash_used=false;Ok(())
 }
 /// Explicit host activation; shared Action priority is resolved by the caller.
 pub fn begin_ring_dash(&mut self,world:&dyn World)->Result<bool,String>{
  if !matches!(self.state,State::Idle|State::Run|State::Jump|State::Fall){return Ok(false);}
  let action=if self.grounded{1}else{10};
  let Some(mut records)=self.prepare_source_attack(world,action)else{return Ok(false)};
  // The demo enables the Light Shoes upgrade; other hosts may add inventory gating.
  records.character[0x24..0x28].copy_from_slice(&1u32.to_be_bytes());records.character[12]=0;records.character[13]=59;
  let center=(self.position+self.model_rotation()*Vec3::Y*0.54)*10.;
  let candidates=self.ring_candidates(world,center);
  let choice=sa2_runtime::ring_motion_host::offer_and_activate(&mut records,center.to_array(),&candidates,true,true,fallback_math()).map_err(|e|e.to_string())?;
  if choice!=sa2_runtime::original_action_choice::Choice::Offered(59){return Ok(false);}
  self.ring_motion=Some(sa2_runtime::ring_motion_host::RingMotion::from_active(records).map_err(|e|e.to_string())?);
  self.ground_motion=None;self.air_motion=None;self.bounce_motion=None;self.landing_animation=None;self.state=State::RingDash;Ok(true)
 }
 fn ring_candidates(&self,world:&dyn World,center:Vec3)->Vec<sa2_runtime::original_ring_dash::RingCandidate>{
  (0..world.ring_count()).filter(|&i|self.collected.get(i)!=Some(&true)).filter_map(|i|world.ring(i)).map(|p|{
   let position=p*10.;sa2_runtime::original_ring_dash::RingCandidate{position:position.to_array(),offset:[0.;3],cached_distance:(position-center).length_squared(),collider_available:true}
  }).collect()
 }
 fn tick_ring_dash(&mut self,input:Input,world:&dyn World){
  let terrain=world.source_terrain().expect("active ring source terrain");
  let direction=((std::f32::consts::FRAC_PI_2-input.direction.x.atan2(input.direction.y))*65536./std::f32::consts::TAU)as i32 as u32&0xffff;
  let common=sa2_runtime::original_position_a::PositionContext{control:sa2_runtime::original_control::ControlContext{directions:[direction,0],magnitudes:[input.direction.length().min(1.),0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0},incoming_f26:0.,entity_identity:0x80210100,motion_identity:0x80210600,character_identity:0x80210200,workspace_identity:0x80210000,platform_offset_identity:0x80212000};
  let ctx=sa2_runtime::original_position_b::PositionBContext{common:&common,direction_mode:0,special_offset:0.,special_radius:0.};
  let center=(self.position+self.model_rotation()*Vec3::Y*0.54)*10.;let candidates=self.ring_candidates(world,center);
  let ring=self.ring_motion.as_mut().expect("active ring records");
  let timed_out=ring.decide_timeout_with_control(&common.control,fallback_math()).expect("ring timeout");
  let mut ended=timed_out;
  if !ended{ended=ring.physics(center.to_array(),&candidates,6.,&terrain.world(),&ctx,fallback_math(),|m|world.prepare_platform_memory(m),|a,p,e,s,m|world.platform_callback(a,p,e,s,m)).expect("ring physics").stop_sound;}
  if !ended{
   let r=ring.records();
   let feet=Vec3::from_array(std::array::from_fn(|i|f32::from_be_bytes(r.entity[0x14+i*4..0x18+i*4].try_into().unwrap())*0.1));
   let up=Vec3::from_array(sa2_runtime::original_ground::rotate_vector(&r.entity,[0.,1.,0.],true,fallback_math()).expect("ring pickup orientation"));
   let pickup_center=feet+up*0.65;
   for i in 0..world.ring_count(){
    if self.collected.get(i)==Some(&true){continue;}
    if let Some(p)=world.ring(i){if pickup_center.distance(p)<0.8{
     ring.contact(0x11,(p*10.).to_array(),fallback_math()).expect("ring pickup bookkeeping");
     self.collected[i]=true;world.event(ActionEvent::RingCollected(i),feet);
    }}
   }
  }
  let r=ring.records();
  self.position=Vec3::from_array(std::array::from_fn(|i|f32::from_be_bytes(r.entity[0x14+i*4..0x18+i*4].try_into().unwrap())*0.1));
  self.velocity=Vec3::from_array(std::array::from_fn(|i|f32::from_be_bytes(r.motion[i*4..i*4+4].try_into().unwrap())*6.));
  self.yaw=std::f32::consts::FRAC_PI_2-u32::from_be_bytes(r.entity[12..16].try_into().unwrap())as f32*std::f32::consts::TAU/65536.;self.grounded=r.entity[5]&3!=0;
  self.terrain_collision=Some(r.clone());
  if ended{
   let mut r=self.ring_motion.take().unwrap().into_records();self.state=State::Fall;
   if timed_out{
    // Expiry modifies local velocity. Preserve it across the host's next-frame
    // world-to-local reconciliation, which otherwise restores stale dash speed.
    let local=std::array::from_fn(|i|f32::from_be_bytes(r.character[0x64+i*4..0x68+i*4].try_into().unwrap()));
    let velocity=sa2_runtime::original_ground::rotate_vector(&r.entity,local,true,fallback_math()).expect("ring exit world velocity");
    for i in 0..3{r.motion[i*4..i*4+4].copy_from_slice(&velocity[i].to_be_bytes());}
    self.velocity=Vec3::from_array(velocity)*6.;self.terrain_collision=Some(r.clone());
   }
   self.air_motion=Some(sa2_runtime::air_motion_host::AirMotion::from_fall_records(&r.entity,&r.motion,&r.character).expect("ring exit records"));
   self.resolve_air_landing(input,world);
  }
  self.max_height=self.max_height.max(self.position.y);self.max_speed=self.max_speed.max(self.speed());
  if world.should_respawn(self.position){self.reset_in(world);world.event(ActionEvent::Respawn,self.position);}
 }
 fn prepare_source_attack(&self,world:&dyn World,action:u8)->Option<sa2_runtime::original_position_a::PositionState>{
  let terrain=world.source_terrain()?;
  let ground=sa2_runtime::ground_motion_host::GroundMotion::new(&self.physics.encode()).expect("attack physics");
  let mut records=self.terrain_collision.clone().unwrap_or_else(||terrain.state(&ground));
  let entering=records.entity[0]!=action;
  records.entity[0]=action;
  if entering{
   let flags=0x500_u16|if self.grounded{1}else{0};records.entity[4..6].copy_from_slice(&flags.to_be_bytes());
   records.character[0x70..0x7c].fill(0);
  }
  let yaw=((std::f32::consts::FRAC_PI_2-self.yaw)*65536./std::f32::consts::TAU)as i32 as u32&0xffff;
  records.entity[12..16].copy_from_slice(&yaw.to_be_bytes());
  for i in 0..3{
   records.entity[0x14+4*i..0x18+4*i].copy_from_slice(&(self.position[i]*10.).to_be_bytes());
   records.motion[4*i..4*i+4].copy_from_slice(&(self.velocity[i]/6.).to_be_bytes());
  }
  records.character[0xc0..0x144].copy_from_slice(&self.physics.encode());
  // Pursuit writes both world motion and its local counterpart before contact.
  // Reconciliation here would incorrectly erase upward speed while supported.
  let local=sa2_runtime::original_ground::rotate_vector(&records.entity,(self.velocity/6.).to_array(),false,fallback_math()).expect("attack pursuit local velocity");
  for i in 0..3{records.character[0x64+4*i..0x68+4*i].copy_from_slice(&local[i].to_be_bytes());}
  records.entity[0x2c..0x30].copy_from_slice(&0x80213000_u32.to_be_bytes());
  records.memory.bytes[0x13000..0x130b0].fill(0);records.memory.bytes[0x13011]=255;
  Some(records)
 }
 fn move_source_attack(&mut self,world:&dyn World,action:u8,input:Input)->Option<bool>{
  let terrain=world.source_terrain()?;
  let mut records=self.prepare_source_attack(world,action)?;
  let direction=((std::f32::consts::FRAC_PI_2-input.direction.x.atan2(input.direction.y))*65536./std::f32::consts::TAU)as i32 as u32&0xffff;
  let common=sa2_runtime::original_position_a::PositionContext{control:sa2_runtime::original_control::ControlContext{directions:[direction,0],magnitudes:[input.direction.length().min(1.),0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0},incoming_f26:0.,entity_identity:0x80210100,motion_identity:0x80210600,character_identity:0x80210200,workspace_identity:0x80210000,platform_offset_identity:0x80212000};
  let ctx=sa2_runtime::original_position_b::PositionBContext{common:&common,direction_mode:0,special_offset:0.,special_radius:0.};
  world.prepare_platform_memory(&mut records.memory).expect("attack platform memory");
  if sa2_runtime::original_attack_contact::advance(&mut records,&mut 0,&terrain.world(),&ctx,fallback_math(),|a,p,e,s,m|world.platform_callback(a,p,e,s,m)).is_err(){return Some(false);}
  let position=Vec3::from_array(std::array::from_fn(|i|f32::from_be_bytes(records.entity[0x14+4*i..0x18+4*i].try_into().unwrap())*0.1));
  let velocity=Vec3::from_array(std::array::from_fn(|i|f32::from_be_bytes(records.motion[4*i..4*i+4].try_into().unwrap())*6.));
  if !position.is_finite()||!velocity.is_finite(){return Some(false);}
  self.position=position;self.velocity=velocity;self.grounded=records.entity[5]&3!=0;
  self.terrain_collision=Some(records);Some(true)
 }
 fn tick_source_bounce(&mut self,input:Input,world:&dyn World)->bool{
  use sa2_runtime::{bounce_motion_host::{BounceMotion,BounceStep},ground_motion_host::GroundMotion};
  let Some(terrain)=world.source_terrain() else{return false};
  let active=matches!(self.state,State::BounceDown|State::BounceUp);
  // Source69/70 consume prior contact before the ordinary airborne A branch.
  let pending_support=self.bounce_motion.as_ref().is_some_and(|b|b.records().entity[5]&3!=0);
  if self.grounded||(!active&&!input.bounce_pressed)||(input.jump_pressed&&!self.air_dash_used&&!pending_support){self.bounce_motion=None;return false;}
  if self.bounce_motion.is_none(){
   let ground=GroundMotion::new(&self.physics.encode()).expect("bounce physics");
   let mut records=self.terrain_collision.clone().unwrap_or_else(||terrain.state(&ground));
   records.entity[5]&=!3;
   for i in 0..3{
    records.entity[0x14+4*i..0x18+4*i].copy_from_slice(&(self.position[i]*10.).to_be_bytes());
    records.motion[4*i..4*i+4].copy_from_slice(&(self.velocity[i]/6.).to_be_bytes());
   }
   let yaw=((std::f32::consts::FRAC_PI_2-self.yaw)*65536./std::f32::consts::TAU)as i32 as u32&0xffff;
   records.entity[12..16].copy_from_slice(&yaw.to_be_bytes());
   records.character[0xc0..0x144].copy_from_slice(&self.physics.encode());
   records.entity[0x2c..0x30].copy_from_slice(&0x80213000_u32.to_be_bytes());
   records.memory.bytes[0x13000..0x130b0].fill(0);records.memory.bytes[0x13011]=255;
   sa2_runtime::original_movement::reconcile_velocity(&mut records.entity,&mut records.motion,&mut records.character,[0.,-1.,0.],0,fallback_math()).expect("bounce entry local motion");
   self.bounce_motion=Some(BounceMotion::enter(records,fallback_math()).expect("bounce source entry"));
   self.state=State::BounceDown;self.bounce_fall_loop=false;self.bounce_fall_started=false;self.air_dash_used=false;
   world.event(ActionEvent::BounceStart,self.position);
  }
  let angle=((std::f32::consts::FRAC_PI_2-input.direction.x.atan2(input.direction.y))*65536./std::f32::consts::TAU)as i32 as u32&0xffff;
  let common=sa2_runtime::original_position_a::PositionContext{control:sa2_runtime::original_control::ControlContext{directions:[angle,0],magnitudes:[input.direction.length().min(1.),0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0},incoming_f26:0.,entity_identity:0x80210100,motion_identity:0x80210600,character_identity:0x80210200,workspace_identity:0x80210000,platform_offset_identity:0x80212000};
  let ctx=sa2_runtime::original_position_b::PositionBContext{common:&common,direction_mode:0,special_offset:0.,special_radius:0.};
  let bounce=self.bounce_motion.as_mut().unwrap();
  if bounce.records().entity[0]==70&&f32::from_be_bytes(bounce.records().character[0x68..0x6c].try_into().unwrap())<=0.{self.bounce_fall_started=true;}
  let outcome=bounce.tick_with_platform(input.bounce_pressed,&terrain.world(),&ctx,fallback_math(),|m|world.prepare_platform_memory(m),|a,p,e,s,m|world.platform_callback(a,p,e,s,m)).expect("persistent bounce frame");
  let records=bounce.records();
  self.position=Vec3::from_array(std::array::from_fn(|i|f32::from_be_bytes(records.entity[0x14+4*i..0x18+4*i].try_into().unwrap())*0.1));
  self.velocity=Vec3::from_array(std::array::from_fn(|i|f32::from_be_bytes(records.motion[4*i..4*i+4].try_into().unwrap())*6.));
  self.yaw=std::f32::consts::FRAC_PI_2-u32::from_be_bytes(records.entity[12..16].try_into().unwrap())as f32*std::f32::consts::TAU/65536.;
  self.terrain_collision=Some(records.clone());self.grounded=false;self.landing_animation=None;
  self.state=if records.entity[0]==69{State::BounceDown}else{State::BounceUp};
  match outcome{
   BounceStep::Rebounded=>{self.bounced+=1;self.landings+=1;world.event(ActionEvent::BounceImpact,self.position);world.vibration(sa2_runtime::original_vibration::request(7,15,0),self.position);},
   BounceStep::Repeated=>{self.bounce_fall_loop=false;self.bounce_fall_started=false;world.event(ActionEvent::BounceStart,self.position);},
   BounceStep::Landed=>{
    let records=self.bounce_motion.take().unwrap().into_records();
    let mut ground=GroundMotion::from_landed_collision(&records).expect("bounce landing records");
    self.grounded=true;self.landings+=1;self.air_dash_used=false;
    if ground.landing_brake(input.direction.to_array(),fallback_math()).expect("bounce landing brake") {self.state=State::Brake;}
    else{self.landing_animation=ground.rebound_landing_with_context([input.direction.length().min(1.),0.],self.ticks as u32,&mut self.idle_seed,true,fallback_math()).expect("bounce landing animation");self.state=if self.landing_animation.is_some(){State::Land}else{State::Run};}
    if self.landing_animation==Some(17){world.vibration(sa2_runtime::original_vibration::request(6,15,0),self.position);}
    if self.state==State::Run{ground.moving_landing(Some(self.heading_history.delayed_angles(3)[1]),fallback_math()).expect("moving rebound landing records");}
    ground.finish_bounce_landing();
    self.velocity=Vec3::from_array(ground.reconciled_terrain_velocity(fallback_math()).expect("bounce resolved landing velocity"));
    self.ground_motion=Some(ground);
   },
   BounceStep::Flying=>{},
  }
  true
 }
 fn move_source_air_terrain(&mut self,world:&dyn World,input:Input)->bool{
  let Some(terrain)=world.source_terrain() else{return false};
  let Some(air)=self.air_motion.as_mut() else{return false};
  let collision=self.terrain_collision.get_or_insert_with(||{
   let ground=sa2_runtime::ground_motion_host::GroundMotion::new(&air.source_records().2[0xc0..0x144]).expect("air collision physics");
   terrain.state(&ground)
  });
  let angle=((std::f32::consts::FRAC_PI_2-input.direction.x.atan2(input.direction.y))*65536./std::f32::consts::TAU)as i32 as u32&0xffff;
  let common=sa2_runtime::original_position_a::PositionContext{control:sa2_runtime::original_control::ControlContext{directions:[angle,0],magnitudes:[input.direction.length().min(1.),0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0},incoming_f26:0.,entity_identity:0x80210100,motion_identity:0x80210600,character_identity:0x80210200,workspace_identity:0x80210000,platform_offset_identity:0x80212000};
  let ctx=sa2_runtime::original_position_b::PositionBContext{common:&common,direction_mode:0,special_offset:0.,special_radius:0.};
  world.prepare_platform_memory(&mut collision.memory).expect("platform memory preparation");
  let grounded=air.resolve_source_terrain_with_platform(collision,&terrain.world(),&ctx,fallback_math(),|a,p,e,s,m|world.platform_callback(a,p,e,s,m)).expect("source terrain contact");
  let out=air.output();
  self.position=Vec3::from_array(out.position);self.velocity=Vec3::from_array(out.velocity);self.yaw=out.yaw;
  if grounded&&!self.grounded{self.landings+=1;self.landing_animation=Some(16);self.air_dash_used=false;}
  self.grounded=grounded;
  // Landing selection in tick_in still consumes the unmodified impact records.
  true
 }
 fn move_source_terrain(&mut self,world:&dyn World,input:Input)->bool{
  let Some(terrain)=world.source_terrain() else{return false};
  let Some(ground)=self.ground_motion.as_mut() else{return false};
  let collision=self.terrain_collision.get_or_insert_with(||terrain.state(ground));
  let angle=((std::f32::consts::FRAC_PI_2-input.direction.x.atan2(input.direction.y))*65536./std::f32::consts::TAU)as i32 as u32&0xffff;
  let common=sa2_runtime::original_position_a::PositionContext{control:sa2_runtime::original_control::ControlContext{directions:[angle,0],magnitudes:[input.direction.length().min(1.),0.],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0},incoming_f26:0.,entity_identity:0x80210100,motion_identity:0x80210600,character_identity:0x80210200,workspace_identity:0x80210000,platform_offset_identity:0x80212000};
  let ctx=sa2_runtime::original_position_b::PositionBContext{common:&common,direction_mode:0,special_offset:0.,special_radius:0.};
  world.prepare_platform_memory(&mut collision.memory).expect("platform memory preparation");
  let out=ground.resolve_source_terrain_with_platform(collision,&terrain.world(),&ctx,fallback_math(),|a,p,e,s,m|world.platform_callback(a,p,e,s,m)).expect("source terrain contact");
  let records=ground.source_records().2;
  let wall=Vec3::from_array(std::array::from_fn(|i|f32::from_be_bytes(records[0x70+i*4..0x74+i*4].try_into().unwrap())));
  // Count only when contact opposes this frame's motion and the solver
  // actually prevents that displacement; retained normals alone are not hits.
  let attempted=self.velocity/60.;let moved=Vec3::from_array(out.position)-self.position;
  if wall.is_finite()&&wall.y.abs()<0.7&&attempted.dot(wall)< -0.00001&&(moved-attempted).dot(wall)>0.00001{self.wall_hits+=1;}
  self.position=Vec3::from_array(out.position);self.grounded=out.grounded;self.yaw=out.yaw;
  self.velocity=Vec3::from_array(ground.reconciled_terrain_velocity(fallback_math()).expect("source terrain reconciliation"));
  true
 }
 fn move_substep(&mut self,dt:f32){
  let old=self.position;let grounded=self.grounded;
  let delta=self.velocity*dt;let mut next=old;
  // Resolve each horizontal axis separately, allowing wall sliding.
  for axis in [0,2]{
   let mut candidate=next;candidate[axis]+=delta[axis];let mut blocked=false;
   for b in blocks(){if in_rect(vec2(candidate.x,candidate.z),b,RADIUS) && old.y+HEIGHT>b.min.y+0.01 && old.y<b.max.y-0.10 {
    blocked=true;break;
   }}
   if let Some(h)=ramp_height(vec2(candidate.x,candidate.z)){
    if h>old.y+0.12 && old.y+HEIGHT>0. {blocked=true;}
   }
   if blocked{self.velocity[axis]=0.;self.wall_hits+=1;}else{next=candidate;}
  }
  let mut target_y=old.y+delta.y;
  // Head collision against solid platforms when moving upward.
  if delta.y>0.{for b in blocks(){if in_rect(vec2(next.x,next.z),b,RADIUS) && old.y+HEIGHT<=b.min.y && target_y+HEIGHT>b.min.y{target_y=b.min.y-HEIGHT;self.velocity.y=0.;}}}
  let ceiling=if grounded{old.y+0.12}else{old.y+0.001};
  if let Some(h)=support(next,ceiling){
   if (self.velocity.y<=0. && target_y<=h) || (grounded && (h-old.y).abs()<0.15){
    target_y=h;
    self.resolve_support(vec3(next.x,h,next.z),DemoWorld.ground_normal(vec3(next.x,h,next.z))).expect("demo support contact");
   }else{self.grounded=false;}
  }else{self.grounded=false;}
  next.y=target_y;self.position=next;
 }
}
fn move_towards(a:Vec2,b:Vec2,amount:f32)->Vec2{let d=b-a;if d.length()<=amount{b}else{a+d.normalize()*amount}}

/// Accumulator shared by live input and scripted tests. Edge presses survive frames
/// with no simulation tick, and are consumed once, not once per substep.
#[derive(Default)]
pub struct Clock {accumulator:f64,pending_jump:bool,pending_bounce:bool}
impl Clock {
 pub fn advance(&mut self,p:&mut Player,input:Input,seconds:f64)->u32{self.advance_in(p,input,seconds,&DemoWorld)}
 pub fn advance_in(&mut self,p:&mut Player,mut input:Input,seconds:f64,world:&dyn World)->u32{
  self.accumulator+=if seconds.is_finite(){seconds.clamp(0.,0.1)}else{0.};self.pending_jump|=input.jump_pressed;self.pending_bounce|=input.bounce_pressed;let mut ticks=0;
  while self.accumulator+1e-9>=STEP{input.jump_pressed=self.pending_jump;input.bounce_pressed=self.pending_bounce;self.pending_jump=false;self.pending_bounce=false;p.tick_in(input,world);self.accumulator-=STEP;ticks+=1;}ticks
 }
 pub fn clear(&mut self){*self=Self::default();}
}

/// Original common collision-center block80086724; height is in source units.
fn source_attack_center(entity:&[u8],position:Vec3,height:f32)->Vec3{
 let height=if entity[0]==72{-height}else{height};
 let offset=sa2_runtime::original_ground::rotate_vector(entity,[0.,height,0.],true,fallback_math()).expect("source collision center orientation");
 position+Vec3::from_array(offset)*0.1
}

/// Source8000a6c0 strict radius-sum overlap; caller supplies world-space centers.
pub fn sphere_overlap(a:Vec3,ar:f32,b:Vec3,br:f32)->bool{
 if !a.is_finite()||!b.is_finite()||!ar.is_finite()||!br.is_finite()||ar<0.||br<0.{return false;}
 let radius=ar+br;let distance=a.distance_squared(b);
 radius.is_finite()&&distance.is_finite()&&distance<radius*radius
}

#[cfg(test)]
mod landing_continuity_tests {
 #[cfg(feature="reference-tests")]
 #[test]fn attack_center_matches_original_records(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let data:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/attack-center.json")).unwrap()).unwrap();
  let rows=data["rows"].as_array().unwrap();assert_eq!(rows.len(),56);
  for r in rows{
   let mut e=[0u8;64];e[0]=r["action"].as_u64().unwrap()as u8;
   for i in 0..3{e[8+i*4..12+i*4].copy_from_slice(&(r["angles"][i].as_u64().unwrap()as u32).to_be_bytes());}
   let position=Vec3::from_array(std::array::from_fn(|i|r["position"][i].as_f64().unwrap()as f32*0.1));
   let expected=Vec3::from_array(std::array::from_fn(|i|r["center"][i].as_f64().unwrap()as f32*0.1));
   let actual=source_attack_center(&e,position,r["height"].as_f64().unwrap()as f32);
   assert!(actual.distance(expected)<0.00001,"action {} actual {actual:?} expected {expected:?}",e[0]);
  }
 }
 #[test]fn pursuit_attack_center_uses_matching_records_only(){
  for (state,action) in [(State::Homing,13),(State::LightAttack,60)]{
   let mut p=Player::new();p.state=state;p.grounded=false;
   let mut records=p.prepare_source_attack(&DemoWorld,action).expect("source demo terrain");
   records.entity[8..12].copy_from_slice(&0x4000u32.to_be_bytes());
   let expected=source_attack_center(&records.entity,p.position,p.p(31));
   let upright=p.position+Vec3::Y*(p.p(31)*0.1);
   assert!(expected.distance(upright)>0.5);
   p.terrain_collision=Some(records);assert_eq!(p.attack_center(),expected);
   // Targets at opposite volume edges distinguish rotated and obsolete upright centers.
   let away=(expected-upright).normalize();
   let radius=p.p(27)*0.1+DemoWorld.target_radius(0);
   let rotated_only=expected+away*(radius-0.01);
   let upright_only=upright-away*(radius-0.01);
   assert!(p.attack_contact(&DemoWorld,0,rotated_only));
   assert!(!sphere_overlap(upright,p.p(27)*0.1,rotated_only,DemoWorld.target_radius(0)));
   assert!(!p.attack_contact(&DemoWorld,0,upright_only));
   assert!(sphere_overlap(upright,p.p(27)*0.1,upright_only,DemoWorld.target_radius(0)));
   p.terrain_collision.as_mut().unwrap().entity[0]=0;
   assert_eq!(p.attack_center(),upright);
   let before=p.terrain_collision.as_ref().unwrap().entity.clone();let position=p.position;
   assert!(p.attack_contact(&DemoWorld,0,rotated_only),"early overlap uses prepared source orientation");
   assert!(!p.attack_contact(&DemoWorld,0,upright_only));
   assert_eq!(p.terrain_collision.as_ref().unwrap().entity,before,"query does not publish prepared records");
   assert_eq!(p.position,position,"query does not advance motion");
  }
 }
 #[cfg(feature="reference-tests")]
 #[test]fn bounce_center_uses_original_orientation(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let data:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/attack-center.json")).unwrap()).unwrap();
  let mut count=0;
  for r in data["rows"].as_array().unwrap(){
   if r["action"].as_u64()!=Some(69){continue;}
   let mut p=Player::new();p.grounded=false;p.state=State::BounceDown;
   p.position=Vec3::from_array(std::array::from_fn(|i|r["position"][i].as_f64().unwrap()as f32*0.1));
   let mut records=p.prepare_source_attack(&DemoWorld,69).unwrap();
   for i in 0..3{records.entity[8+i*4..12+i*4].copy_from_slice(&(r["angles"][i].as_u64().unwrap()as u32).to_be_bytes());}
   p.bounce_motion=Some(sa2_runtime::bounce_motion_host::BounceMotion::enter(records,fallback_math()).unwrap());
   assert_eq!(p.bounce_motion.as_ref().unwrap().records().entity[0],69);
   let expected=Vec3::from_array(std::array::from_fn(|i|r["center"][i].as_f64().unwrap()as f32*0.1));
   assert!(p.attack_center().distance(expected)<0.00001);
   assert!(p.effect_center().distance(expected)<0.00001);
   count+=1;
  }
  assert_eq!(count,8);
 }
 #[test]fn host_fallback_cancel_preserves_source_records(){
  struct Host;impl World for Host{
   fn target_count(&self)->usize{0}fn target(&self,_:usize)->Option<Vec3>{None}
   fn ring_count(&self)->usize{0}fn ring(&self,_:usize)->Option<Vec3>{None}
   fn attack_path_clear(&self,_:Vec3,_:Vec3)->bool{true}
   fn move_player(&self,p:&mut Player,dt:f32){p.position+=p.velocity*dt;}
   fn spawn(&self)->Vec3{vec3(0.,10.,0.)}fn should_respawn(&self,_:Vec3)->bool{false}
  }
  let mut p=Player::in_world(&Host);p.grounded=false;p.state=State::Homing;p.velocity=vec3(3.,2.,10.);
  let mut records=p.prepare_source_attack(&DemoWorld,13).unwrap();records.character[0x200]=0x5a;
  p.homing_fallback=Some(AirFallback::from_collision(&records).unwrap());assert!(p.terrain_collision.is_none());
  let before=p.position;p.tick_in(Input{bounce_pressed:true,..Default::default()},&Host);
  assert_eq!(p.state,State::Fall);assert_eq!(p.source_air_action(),Some(10));assert_ne!(p.position,before);
  assert_eq!(p.air_motion.as_ref().unwrap().source_records().2[0x200],0x5a,"retained original character record");
  p.tick_in(Input::default(),&Host);p.tick_in(Input{bounce_pressed:true,..Default::default()},&Host);
  assert_eq!(p.state,State::BounceDown);
 }
 #[cfg(feature="reference-tests")]
 #[test]fn fallback_effect_center_overrides_stale_pursuit(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let data:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/attack-center.json")).unwrap()).unwrap();
  let mut count=0;
  for r in data["rows"].as_array().unwrap(){
   if r["action"].as_u64()!=Some(13){continue;}
   let mut p=Player::new();p.grounded=false;p.state=State::Homing;
   p.position=Vec3::from_array(std::array::from_fn(|i|r["position"][i].as_f64().unwrap()as f32*0.1));
   let mut records=p.prepare_source_attack(&DemoWorld,13).unwrap();
   p.terrain_collision=Some(records.clone());
   for i in 0..3{records.entity[8+i*4..12+i*4].copy_from_slice(&(r["angles"][i].as_u64().unwrap()as u32).to_be_bytes());}
   p.homing_fallback=Some(AirFallback::from_collision(&records).unwrap());
   let expected=Vec3::from_array(std::array::from_fn(|i|r["center"][i].as_f64().unwrap()as f32*0.1));
   assert!(p.effect_center().distance(expected)<0.00001);
   p.terrain_collision=None;assert!(p.effect_center().distance(expected)<0.00001);
   p.state=State::Fall;assert_eq!(p.effect_center(),p.position+Vec3::Y*(p.p(31)*0.1));
   count+=1;
  }
  assert_eq!(count,8);
 }
 #[cfg(feature="reference-tests")]
 #[test]fn pursuit_effect_center_matches_original_records(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let data:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/attack-center.json")).unwrap()).unwrap();
  let mut count=0;
  for r in data["rows"].as_array().unwrap(){
   let action=r["action"].as_u64().unwrap()as u8;if !matches!(action,13|60){continue;}
   let mut p=Player::new();p.grounded=false;p.state=if action==13{State::Homing}else{State::LightAttack};
   p.position=Vec3::from_array(std::array::from_fn(|i|r["position"][i].as_f64().unwrap()as f32*0.1));
   let mut records=p.prepare_source_attack(&DemoWorld,action).unwrap();
   for i in 0..3{records.entity[8+i*4..12+i*4].copy_from_slice(&(r["angles"][i].as_u64().unwrap()as u32).to_be_bytes());}
   p.terrain_collision=Some(records);
   let expected=Vec3::from_array(std::array::from_fn(|i|r["center"][i].as_f64().unwrap()as f32*0.1));
   assert!(p.effect_center().distance(expected)<0.00001);
   p.terrain_collision.as_mut().unwrap().entity[0]=0;
   assert_eq!(p.effect_center(),p.position+Vec3::Y*(p.p(31)*0.1));
   count+=1;
  }
  assert_eq!(count,16);
 }
 #[cfg(feature="reference-tests")]
 #[test]fn effect_center_matches_original_rotated_centers(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let data:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/effect-center.json")).unwrap()).unwrap();
  let rows=data["rows"].as_array().unwrap();assert_eq!(rows.len(),8);
  for r in rows{
   let mut e=[0u8;64];let m=[0u8;64];let c=[0u8;0x3a0];
   for i in 0..3{e[8+i*4..12+i*4].copy_from_slice(&(r["angles"][i].as_u64().unwrap()as u32).to_be_bytes());}
   let mut p=Player::new();p.grounded=false;p.state=State::Jump;
   p.position=Vec3::from_array(std::array::from_fn(|i|r["position"][i].as_f64().unwrap()as f32*0.1));
   p.air_motion=Some(sa2_runtime::air_motion_host::AirMotion::from_fall_records(&e,&m,&c).unwrap());
   let expected=Vec3::from_array(std::array::from_fn(|i|r["center"][i].as_f64().unwrap()as f32*0.1));
   assert!(p.effect_center().distance(expected)<0.00001,"actual {:?} expected {expected:?}",p.effect_center());
  }
  let p=Player::new();assert_eq!(p.effect_center(),p.attack_center());
 }

 #[test]
 fn homing_contact_retains_source_records_into_rebound(){
  struct ContactWorld{response:HomingContactResponse}
  impl World for ContactWorld{
   fn attack_path_clear(&self,_:Vec3,_:Vec3)->bool{true}fn spawn(&self)->Vec3{vec3(0.,10.,0.)}fn should_respawn(&self,_:Vec3)->bool{false}
   fn target_count(&self)->usize{1}fn target(&self,_:usize)->Option<Vec3>{Some(vec3(0.,10.,0.))}
   fn target_radius(&self,_:usize)->f32{5.}
   fn ring_count(&self)->usize{0}fn ring(&self,_:usize)->Option<Vec3>{None}
   fn source_terrain(&self)->Option<&sa2_runtime::static_terrain_host::StaticTerrain>{DemoWorld.source_terrain()}
   fn homing_contact_response(&self,_:usize)->HomingContactResponse{self.response}
   fn move_player(&self,_:&mut Player,_:f32){panic!("source rebound expected");}
  }
  for response in [HomingContactResponse::SourceCategory0Upright,HomingContactResponse::SourceCategory2,HomingContactResponse::SourceCategory3]{for cycle in 0..3{for moved in [false,true]{
   let world=ContactWorld{response};let mut p=Player::in_world(&world);p.position=vec3(0.,10.,0.);p.velocity=vec3(6.,3.,12.);p.grounded=false;p.state=State::Homing;p.attack_target=Some(0);p.rebound_cycle=cycle;
   if moved{assert_eq!(p.move_source_attack(&world,13,Input::default()),Some(true));}
   let prior=p.prepare_source_attack(&world,13).unwrap().motion;let before_position=p.position;
   assert!(p.finish_homing_contact(&world,vec3(0.,10.,0.)));assert_eq!(p.position,before_position);
   let(e,m,c)=p.air_motion.as_ref().unwrap().source_records();assert_eq!(e[0],6);if response==HomingContactResponse::SourceCategory0Upright{assert_ne!(m,prior);assert!((p.velocity.y-15.).abs()<0.001);}else{assert_eq!(m,prior);}assert_eq!(e[4]&1,0);
   assert_eq!(u16::from_be_bytes(c[0x176..0x178].try_into().unwrap()),101+cycle as u16);assert_eq!(c[0x35e],(cycle+1)%3);
   p.tick_in(Input{jump_held:true,..Default::default()},&world);
   assert!(p.air_motion.is_some());assert!(p.position.is_finite()&&p.velocity.is_finite()&&p.yaw.is_finite());assert_eq!(p.rebound_variant,cycle);
   let c=p.air_motion.as_ref().unwrap().source_records().2;assert_eq!(i16::from_be_bytes(c[0x12..0x14].try_into().unwrap()),0,"rebound must not consume held-jump boost");
   let mut aura=crate::aura::Auras::default();aura.tick(&p,p.attack_center());assert_eq!(aura.spawned,1,"retained action6 rebound emits aura");
   assert_ne!(p.state,State::Homing,"held jump must not retrigger attack");
   p.tick_in(Input{jump_pressed:true,jump_held:true,..Default::default()},&world);
   assert_eq!(p.state,State::Homing);assert!(p.air_motion.is_none());assert!(p.homing_fallback.is_some());
   p.tick_in(Input{bounce_pressed:true,..Default::default()},&world);assert_eq!(p.state,State::Fall);
   p.tick_in(Input::default(),&world);
   p.tick_in(Input{bounce_pressed:true,..Default::default()},&world);assert_eq!(p.state,State::BounceDown);
   assert!(p.position.is_finite()&&p.velocity.is_finite());

  }}}
 }

 #[test]
 fn targeted_attack_contact_uses_matching_live_records(){
  let mut target=[0u8;0x30];target[3]=4;target[4..8].copy_from_slice(&0x400u32.to_be_bytes());
  for (state,action,strength,hit) in [(State::Homing,13,1,false),(State::LightAttack,60,2,true)]{
   let mut p=Player::new();p.position.y=10.;p.grounded=false;p.state=state;
   assert!(p.source_body_contact(&target,2).unwrap().is_none());
   assert_eq!(p.move_source_attack(&DemoWorld,action,Input::default()),Some(true));
   let contact=p.source_body_contact(&target,2).unwrap().unwrap();
   assert_eq!(contact.sonic_classification&3,strength);assert_eq!(contact.marks_receiver,hit);
   p.terrain_collision.as_mut().unwrap().entity[0]=70;
   assert!(p.source_body_contact(&target,2).unwrap().is_none(),"stale action must not classify current attack");
  }
 }
 #[test]
 fn live_bounce_contact_strength_changes_on_rebound(){
  let mut target=[0u8;0x30];target[3]=4;target[4..8].copy_from_slice(&0x400u32.to_be_bytes());
  let mut p=Player::new();assert!(p.source_body_contact(&target,2).unwrap().is_none());
  p.position.y=3.;p.grounded=false;p.state=State::Fall;p.tick(Input{bounce_pressed:true,..Default::default()});
  let down=p.source_body_contact(&target,2).unwrap().unwrap();
  assert_eq!(down.sonic_classification&3,2);assert!(down.marks_receiver);
  let mut rebound=false;
  for _ in 0..120{p.tick(Input::default());if p.state==State::BounceUp{
   let up=p.source_body_contact(&target,2).unwrap().unwrap();assert_eq!(up.sonic_classification&3,1);assert!(!up.marks_receiver);rebound=true;break;
  }}
  assert!(rebound);
 }
 #[test]
 fn moving_contacts_transfer_orientation_and_select_locomotion(){
  for bounce in [false,true]{
   let mut p=Player::new();p.tick(Input{jump_pressed:true,..Default::default()});p.tick(Input::default());
   p.position=vec3(-10.,3.,-6.);p.velocity=vec3(0.,-6.,6.);p.state=State::Fall;
   let mut landed=false;
   for tick in 0..240{
    p.tick(Input{direction:vec2(0.,1.),bounce_pressed:bounce&&tick==0,..Default::default()});
    if p.grounded{
     assert_eq!(p.state,State::Run);assert_eq!(p.bounced,u32::from(bounce));
     let(e,m,c)=p.ground_motion.as_ref().unwrap().source_records();
     assert_eq!(e[0],1);assert_eq!(&e[8..12],&m[0x18..0x1c]);assert_eq!(&e[0x10..0x14],&m[0x20..0x24]);
     assert!((6..=10).contains(&u16::from_be_bytes(c[0x176..0x178].try_into().unwrap())));
     landed=true;break;
    }
   }
   assert!(landed);
  }
 }
 #[test]
 fn rebound_landing_retains_source_idle_counter(){
  let mut p=Player::new();p.position.y=3.;p.grounded=false;p.state=State::Fall;
  p.tick(Input{bounce_pressed:true,..Default::default()});
  let mut landed=false;
  for _ in 0..240{
   p.tick(Input::default());
   if p.grounded{
    assert_eq!(p.bounced,1);
    let (e,_,c)=p.ground_motion.as_ref().unwrap().source_records();
    assert_eq!(i16::from_be_bytes(c[0x18..0x1a].try_into().unwrap()),1);
    assert_eq!(u16::from_be_bytes(e[4..6].try_into().unwrap())&0x500,0);
    assert!(p.landing_animation.is_some());landed=true;break;
   }
  }
  assert!(landed);
 }
 #[test]
 fn no_input_slope_contact_exits_air_state(){
  for speed in [3.,9.,20.]{
   let mut p=Player::new();p.tick(Input{jump_pressed:true,..Default::default()});p.tick(Input::default());
   p.position=vec3(0.,2.001,25.);p.velocity=vec3(0.,-speed,0.);p.state=State::Fall;
   for _ in 0..90{p.tick(Input::default());if p.grounded{break;}}
   assert!(p.grounded);assert_ne!(DemoWorld.ground_normal(p.position),Vec3::Y);
   assert!(matches!(p.state,State::Land|State::Idle|State::Run),"supported state {:?}",p.state);
   assert!(p.position.is_finite()&&p.velocity.is_finite());
   if p.landing_animation.is_some(){
    let (e,m,_)=p.ground_motion.as_ref().unwrap().source_records();
    assert_eq!(&e[8..12],&m[0x18..0x1c]);assert_eq!(&e[0x10..0x14],&m[0x20..0x24]);
   }
  }
 }
 #[test]
 fn slope_contact_runs_available_landing_brake(){
  for direction in [-1.,1.]{
  let mut p=Player::new();p.yaw=if direction<0.{std::f32::consts::PI}else{0.};
  p.tick(Input{jump_pressed:true,..Default::default()});p.tick(Input::default());
  p.position=vec3(0.,2.001,25.);p.grounded=false;p.state=State::Fall;p.velocity=vec3(0.,-12.,12.*direction);
  let input=Input{direction:vec2(0.,-direction),..Default::default()};
  let mut contact=false;
  for _ in 0..90{
   p.tick(input);
   if p.grounded{
    assert_ne!(DemoWorld.ground_normal(p.position),Vec3::Y,"contact must be on ramp");
    let mut decision=p.ground_motion.as_ref().expect("retained landing records").clone();
    let expected=decision.landing_brake(input.direction.to_array(),fallback_math()).unwrap();
    assert!(expected,"setup must reach reverse-input landing branch");
    assert_eq!(p.state,State::Brake,"slope must consume applicable source landing decision");
    contact=true;break;
   }
  }
  assert!(contact);
  }
 }
 use super::*;
 #[test]
 fn reverse_input_on_touchdown_selects_brake_immediately(){
  for reverse in [false,true] {
   let mut p=Player::new();p.tick(Input{jump_pressed:true,..Input::default()});
   // Establish a full airborne frame before injecting the descending impact.
   // Source collision retains previous support for the takeoff transition.
   p.tick(Input::default());
   p.position=vec3(0.,0.001,-6.);p.velocity=vec3(0.,-12.,6.);p.state=State::Fall;
   p.tick(Input{direction:vec2(0.,if reverse{-1.}else{1.}),..Input::default()});
   assert!(p.grounded);assert!(p.velocity.is_finite());
   assert!(p.velocity.z>0.&&p.velocity.z<7.,"landing decision must retain forward momentum");
   if reverse {
    assert_eq!(p.state,State::Brake);
    assert_eq!(p.source_animation_clock().unwrap().output().selected,18);
   }else{assert_ne!(p.state,State::Brake);}
   for _ in 0..180 {p.tick(Input::default());}
   assert!(p.grounded&&p.velocity.is_finite());assert_eq!(p.state,State::Idle);
  }
 }
 #[test]
 fn flat_impact_selects_light_and_long_landing(){
  for (speed,expected) in [(3f32,1u16),(9.,16),(20.,17)] {
   let mut p=Player::new();p.tick(Input{jump_pressed:true,..Input::default()});
   p.tick(Input::default());
   p.position=vec3(0.,0.001,-6.);p.velocity=vec3(0.,-speed,0.);p.state=State::Fall;
   p.tick(Input::default());
   assert!(p.grounded,"impact {speed}: position={:?} velocity={:?} flags={:?}",p.position,p.velocity,p.air_motion.as_ref().map(|a|a.source_records().0[..8].to_vec()));
   assert_eq!(p.source_animation_clock().unwrap().output().selected,expected,"impact {speed}");
   for _ in 0..150 {p.tick(Input::default());}
   assert_eq!(p.state,State::Idle);assert!(p.landing_animation.is_none());
  }
 }
 fn stationary_landing()->Player {
  let mut p=Player::new();p.tick(Input{jump_pressed:true,..Input::default()});
  for _ in 0..300 {p.tick(Input::default());if p.grounded{return p;}}
  panic!("jump did not land");
 }
 #[test]
 fn landing_uses_clip_completion_and_movement_interrupts(){
  let mut p=stationary_landing();
  assert_eq!(p.state,State::Land);
  for _ in 0..8 {p.tick(Input::default());}
  assert_eq!(p.state,State::Land,"must not expire at old eight-tick limit");
  for _ in 0..80 {p.tick(Input::default());if p.state==State::Idle{break;}}
  assert_eq!(p.state,State::Idle);assert!(p.landing_animation.is_none());
  let mut p=stationary_landing();
  p.tick(Input{direction:vec2(0.,1.),..Input::default()});
  assert!(p.landing_animation.is_none());assert_ne!(p.state,State::Land);
 }
 #[test]
 fn landing_interrupts_for_jump_charge_and_reset(){
  let mut p=stationary_landing();let jumps=p.jumps;
  p.tick(Input{jump_pressed:true,jump_held:true,..Input::default()});
  assert_eq!(p.state,State::Jump);assert!(!p.grounded);assert!(p.velocity.y>0.);
  assert_eq!(p.jumps,jumps+1);assert!(p.landing_animation.is_none());
  assert_eq!(p.source_animation_clock().unwrap().output().selected,65);
  p.tick(Input{jump_held:true,..Input::default()});assert_eq!(p.jumps,jumps+1);
  let mut p=stationary_landing();
  p.tick(Input{charge:true,..Input::default()});
  assert_eq!(p.state,State::Charge);assert!(p.landing_animation.is_none());
  assert_eq!(p.source_animation_clock().unwrap().output().selected,11);
  p.tick(Input::default());assert_eq!(p.state,State::Roll);
  let mut p=stationary_landing();p.reset();
  assert!(p.landing_animation.is_none());assert_eq!(p.state,State::Idle);
  p.tick(Input::default());assert_eq!(p.source_animation_clock().unwrap().output().selected,0);
 }
 #[test]
 fn long_landing_finishes_into_idle_without_restarting(){
  let mut clock=locomotion_animation_clock();
  let mut selected=17;let mut saw_landing=false;let mut completed=false;
  for _ in 0..100 {
   let output=clock.tick(selected,1.,1.,fallback_math()).unwrap();
   if output.current==17{saw_landing=true;}
   if saw_landing&&output.current==0 {completed=true;}
   selected=output.selected;
  }
  assert!(saw_landing&&completed);
  assert_eq!(clock.output().current,0);
 }
 #[test]
 fn ordinary_jump_returns_to_persistent_ground_controller(){
  let mut p=Player::new();
  p.tick(Input{jump_pressed:true,jump_held:true,..Input::default()});
  assert!(p.air_motion.is_some());assert!(!p.grounded);
  for _ in 0..300 {
   p.tick(Input::default());
   if p.grounded {break;}
  }
  assert!(p.grounded,"jump must land in bounded time");
  assert!(p.air_motion.is_none());
  assert!(p.ground_motion.is_some(),"landing must retain the controller before next ground tick");
  p.reset();
  assert!(p.air_motion.is_none());assert!(p.ground_motion.is_none());
 }
}



#[cfg(test)]
mod idle_clock_readiness_tests {
 use super::*;
 #[test]
 fn original_idle_variants_loop_and_exit_clip_finishes(){
  for (id,frames) in [(88,18.),(89,39.)]{
   let mut clock=locomotion_animation_clock();
   for _ in 0..1000 {clock.tick(id,0.,0.,fallback_math()).unwrap();}
   let out=clock.output();assert_eq!(out.selected,id);assert_eq!(out.current,id);
   assert!(out.frame>=0.&&out.frame<frames);
  }
  let mut clock=locomotion_animation_clock();let mut selected=90;
  for _ in 0..200 {selected=clock.tick(selected,0.,0.,fallback_math()).unwrap().selected;}
  assert_eq!(clock.output().selected,0);assert_eq!(clock.output().current,0);
 }
}
#[cfg(test)]
mod live_idle_tests{
 use super::*;
 fn waiting()->Player{let mut p=Player::new();for _ in 0..1000{p.tick(Input::default());}assert!(matches!(p.source_animation_clock().unwrap().output().selected,88|89));p}
 #[test]fn variants_activate_and_actions_interrupt(){
  for input in [Input{direction:vec2(0.,1.),..Default::default()},Input{jump_pressed:true,..Default::default()},Input{charge:true,..Default::default()}]{
   let mut p=waiting();p.tick(input);assert!(!p.idle_active);assert!(!matches!(p.source_animation_clock().unwrap().output().selected,88|89));
  }
  let mut p=waiting();p.reset();assert!(!p.idle_active);assert_eq!(p.idle_seed,0);
  for _ in 0..900{p.tick(Input::default());}assert_eq!(p.source_animation_clock().unwrap().output().selected,0);
 }
}
#[cfg(test)]
mod idle_lifecycle_tests {
 use super::*;
 #[test]
 fn both_variants_occur_and_movement_restarts_wait(){
  let mut p=Player::new();let mut seen=[false;2];let mut first=None;
  for _ in 0..15000{
   p.tick(Input::default());let out=p.source_animation_clock().unwrap().output();
   if matches!(out.selected,88|89)&&out.current==out.selected{
    if first.is_none(){first=Some(out.selected);}
    seen[(out.selected-88)as usize]=true;
   }
   if seen==[true,true]{break;}
  }
  assert_eq!(seen,[true,true],"real idle selector must reach both variants");
  let reached=p.ticks;
  for _ in 0..30{p.tick(Input{direction:vec2(0.,1.),..Default::default()});}
  for _ in 0..300{p.tick(Input::default());if p.idle_active{break;}}
  assert!(p.idle_active);let entry=p.ticks;
  for _ in 0..899{p.tick(Input::default());assert!(!matches!(p.source_animation_clock().unwrap().output().selected,88|89));}
  for _ in 0..50{p.tick(Input::default());}
  assert!(matches!(p.source_animation_clock().unwrap().output().selected,88|89));
  p.reset();for _ in 0..1000{p.tick(Input::default());}
  assert_eq!(Some(p.source_animation_clock().unwrap().output().selected),first);
  println!("IDLE_LIFECYCLE_PASS both_by_tick={reached} movement_idle_entry={entry}");
 }
}

#[cfg(test)]
mod lost_target_source_continuity_tests {
 use super::*;
 #[test]
 fn late_jump_hold_keeps_remaining_budget_during_jump_descent(){
  let make=||{
   let mut p=Player::new();p.physics.hang_time=2;
   p.tick(Input{jump_pressed:true,..Default::default()});
   for _ in 0..100{
    p.tick(Input::default());
    if p.state==State::Fall&&!p.grounded&&matches!(p.jump_motion,JumpMotion::FallIntro|JumpMotion::FallLoop){return p;}
   }
   panic!("must reach natural jump descent");
  };
  let mut held=make();let mut released=make();
  assert_eq!(held.position,released.position);
  held.tick(Input{jump_held:true,..Default::default()});released.tick(Input::default());
  assert!(held.velocity.y>released.velocity.y,"remaining held budget should affect descent");
  for _ in 0..2{held.tick(Input{jump_held:true,..Default::default()});}
  let c=held.air_motion.as_ref().unwrap().source_records().2;
  assert_eq!(i16::from_be_bytes(c[0x12..0x14].try_into().unwrap()),3);
 }
 #[test]
 fn landing_then_walkoff_does_not_reuse_jump_descent_context(){
  let mut p=Player::new();p.position=vec3(0.,4.,47.5);
  p.tick(Input{jump_pressed:true,..Default::default()});
  let mut descended=false;let mut landed=false;
  for _ in 0..160{
   p.tick(Input::default());
   descended|=matches!(p.jump_motion,JumpMotion::FallIntro|JumpMotion::FallLoop);
   if p.grounded{landed=true;break;}
  }
  assert!(descended&&landed);assert!(p.jump_motion==JumpMotion::Intro);
  let mut walked_off=false;
  for _ in 0..100{
   p.tick(Input{direction:vec2(0.,1.),..Default::default()});
   if !p.grounded{walked_off=true;break;}
  }
  assert!(walked_off);assert_eq!(p.state,State::Fall);
  for _ in 0..3{
   assert!(p.jump_motion==JumpMotion::Intro,"walkoff inherited jump animation context");
   assert_eq!(p.air_motion.as_ref().unwrap().source_records().0[0],10);
   assert_eq!(p.clip(),"fall");
   p.tick(Input::default());
  }
 }
 #[test]
 fn descending_jump_keeps_jump_action_input_priority(){
  let mut p=Player::new();p.tick(Input{jump_pressed:true,..Default::default()});
  let mut reached=false;
  for _ in 0..100{
   p.tick(Input::default());
   if p.state==State::Fall&&!p.grounded&&matches!(p.jump_motion,JumpMotion::FallIntro|JumpMotion::FallLoop){reached=true;break;}
  }
  assert!(reached,"must reach actual jump descent before landing");
  p.tick(Input{jump_pressed:true,bounce_pressed:true,..Default::default()});
  assert_eq!(p.state,State::Homing);
 }
 #[test]
 fn ordinary_jump_homing_precedes_bounce_when_available(){
  for available in [false,true]{
   let mut p=Player::new();p.tick(Input{jump_pressed:true,..Default::default()});
   for _ in 0..10{p.tick(Input{jump_held:true,..Default::default()});}
   assert_eq!(p.state,State::Jump);assert!(!p.grounded);
   p.air_dash_used=!available;
   p.tick(Input{jump_pressed:true,bounce_pressed:true,..Default::default()});
   assert_eq!(p.state,if available{State::Homing}else{State::BounceDown});
  }
 }
 #[test]
 fn rebound_homing_precedes_repeat_when_both_buttons_are_pressed(){
  for available in [false,true]{
   let mut p=Player::new();p.position.y=3.;p.grounded=false;p.state=State::Fall;
   let mut reached=false;
   for tick in 0..100{
    p.tick(Input{bounce_pressed:tick==0,..Default::default()});
    if p.state==State::BounceUp&&p.bounce_motion.as_ref().is_some_and(|b|b.records().entity[5]&3==0){reached=true;break;}
   }
   assert!(reached);p.air_dash_used=!available;
   p.tick(Input{jump_pressed:true,bounce_pressed:true,..Default::default()});
   assert_eq!(p.state,if available{State::Homing}else{State::BounceDown});
  }
 }
 #[test]
 fn bounce_support_precedes_simultaneous_homing_press(){
  for action in [69,70]{
   let mut p=Player::new();p.position.y=3.;p.grounded=false;p.state=State::Fall;
   let mut reached=false;
   for tick in 0..200{
    p.tick(Input{bounce_pressed:tick==0,..Default::default()});
    if p.bounce_motion.as_ref().is_some_and(|b|b.records().entity[0]==action&&b.records().entity[5]&3!=0){reached=true;break;}
   }
   assert!(reached,"source action{action} must reach pending contact");
   let bounced=p.bounced;let landings=p.landings;
   p.tick(Input{jump_pressed:true,bounce_pressed:true,..Default::default()});
   assert_ne!(p.state,State::Homing,"contact must win for action{action}");
   if action==69{assert_eq!(p.state,State::BounceUp);assert_eq!(p.bounced,bounced+1);}
   else{assert!(p.grounded);assert_eq!(p.landings,landings+1);}
  }
 }
 struct SourceHost{terrain:sa2_runtime::static_terrain_host::StaticTerrain,alive:std::cell::Cell<bool>}
 impl World for SourceHost {
  fn target_count(&self)->usize{1} fn target(&self,_:usize)->Option<Vec3>{self.alive.get().then_some(vec3(0.,10.,8.))}
  fn ring_count(&self)->usize{0} fn ring(&self,_:usize)->Option<Vec3>{None}
  fn spawn(&self)->Vec3{vec3(0.,10.,0.)} fn should_respawn(&self,_:Vec3)->bool{false}
  fn attack_path_clear(&self,_:Vec3,_:Vec3)->bool{true}
  fn source_terrain(&self)->Option<&sa2_runtime::static_terrain_host::StaticTerrain>{Some(&self.terrain)}
  fn move_player(&self,_:&mut Player,_:f32){panic!("source homing must not invoke legacy movement")}
 }
 #[test]
 fn post_homing_fall_does_not_restore_held_jump_boost(){
  for timeout in [false,true]{
   let mut outcomes=Vec::new();
   for held in [false,true]{
    let w=SourceHost{terrain:sa2_runtime::static_terrain_host::StaticTerrain::new(&[]).unwrap(),alive:std::cell::Cell::new(true)};
    let mut p=Player::in_world(&w);p.grounded=false;p.state=State::Homing;p.attack_target=Some(0);p.air_dash_used=true;
    p.tick_in(Input::default(),&w);
    assert_eq!(p.state,State::Homing);
    if timeout{
     w.alive.set(false);
     for _ in 0..20{
      p.tick_in(Input::default(),&w);
      if p.state==State::Fall{break;}
     }
    }else{p.tick_in(Input{bounce_pressed:true,..Default::default()},&w);}
    assert_eq!(p.state,State::Fall);
    let mut frames=Vec::new();
    for _ in 0..6{
     p.tick_in(Input{jump_held:held,..Default::default()},&w);
     assert!(!p.grounded);
     let (e,m,c)=p.air_motion.as_ref().expect("post-homing air records").source_records();
     assert_eq!(e[0],10);
     frames.push((p.position,p.velocity,e.to_vec(),m.to_vec(),c.to_vec()));
    }
    outcomes.push(frames);
   }
   assert_eq!(outcomes[0],outcomes[1],"held jump must not boost action10 after homing; timeout={timeout}");
  }
 }
 #[test]
 fn source_cancel_clears_attack_and_runs_air_physics(){
  for lost_target in [false,true]{
  let w=SourceHost{terrain:sa2_runtime::static_terrain_host::StaticTerrain::new(&[]).unwrap(),alive:std::cell::Cell::new(true)};
  let mut p=Player::in_world(&w);p.grounded=false;p.state=State::Homing;p.attack_target=Some(0);p.air_dash_used=true;
  p.tick_in(Input::default(),&w);
  if lost_target{
   w.alive.set(false);
   for _ in 0..4{p.tick_in(Input::default(),&w);}
   assert!(p.homing_fallback.is_some());assert!(p.velocity.y<0.);
  }
  let before=p.position;let ticks=p.ticks;let incoming=p.velocity;
  p.tick_in(Input{bounce_pressed:true,..Default::default()},&w);
  assert_eq!(p.state,State::Fall);assert_eq!(p.ticks,ticks+1);assert_ne!(p.position,before);
  let records=p.terrain_collision.as_ref().unwrap();assert_eq!(records.entity[0],10);assert_eq!(u16::from_be_bytes(records.entity[4..6].try_into().unwrap())&0x500,0);
  assert_eq!(&records.character[0x364..0x366],&[0,0]);assert!(!p.defeated[0]);
  if lost_target{assert!(p.velocity.y<incoming.y,"fall continues the latest descending fallback velocity");}
  }
 }

 #[test]
 fn removed_source_target_preserves_attack_flags_until_timeout(){
  let mut final_states=Vec::new();
  for press in [false,true]{
  let w=SourceHost{terrain:sa2_runtime::static_terrain_host::StaticTerrain::new(&[]).unwrap(),alive:std::cell::Cell::new(true)};
  let mut p=Player::in_world(&w);p.grounded=false;p.state=State::Homing;p.attack_target=Some(0);p.air_dash_used=true;
  p.tick_in(Input::default(),&w);assert_eq!(p.state,State::Homing);
  assert_eq!(u16::from_be_bytes(p.terrain_collision.as_ref().unwrap().entity[4..6].try_into().unwrap())&0x500,0x500);
  w.alive.set(false);
  for tick in 0..15 {
   p.tick_in(Input::default(),&w);assert_eq!(p.state,State::Homing,"fallback frame {tick}");
   let records=p.homing_fallback.as_ref().unwrap().source_records();
   assert_eq!(u16::from_be_bytes(records.0[4..6].try_into().unwrap())&0x500,0x500);
   assert!(p.position.is_finite()&&p.velocity.is_finite());assert!(!p.defeated[0]);
   if tick==0{w.alive.set(true);}
   assert_eq!(p.attack_target,None);
  }
  let before=p.position;let ticks=p.ticks;p.tick_in(Input{bounce_pressed:press,..Default::default()},&w);assert_eq!(p.state,State::Fall);assert_ne!(p.position,before);assert_eq!(p.ticks,ticks+1);
  let records=p.terrain_collision.as_ref().unwrap();assert_eq!(records.entity[0],10);
  assert_eq!(u16::from_be_bytes(records.entity[4..6].try_into().unwrap())&0x500,0);
  final_states.push((p.position,p.velocity,records.entity.clone(),records.character.clone()));
  }
  assert_eq!(final_states[0],final_states[1],"expiry consumes the late B press before cancellation or bounce");
 }
}
