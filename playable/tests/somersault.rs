use sonic_playground::simulation::{Player,State,Input};
#[test]
fn ground_b_attacks_and_recovers(){
 let mut p=Player::new();let start=p.position;
 p.tick(Input{bounce_pressed:true,..Default::default()});
 assert_eq!(p.state,State::Somersault);assert!(p.speed()>0.);assert!(p.grounded);
 let mut receiver=[0u8;0x30];receiver[4..8].copy_from_slice(&0x400u32.to_be_bytes());
 assert!(p.source_body_contact(&receiver,2).unwrap().unwrap().marks_receiver);
 let mut saw_recovery=false;
 for _ in 0..100{
  p.tick(Input::default());assert!(p.position.is_finite());assert_eq!(p.respawns,0);
  if let Some(c)=p.source_animation_clock(){saw_recovery|=c.output().current==94;}
 }
 assert!(saw_recovery);assert_ne!(p.state,State::Somersault);assert!(p.position.z>start.z);assert_eq!(p.bounced,0);
 p.tick(Input{bounce_pressed:true,..Default::default()});assert_eq!(p.state,State::Somersault);
}
#[test]
fn jump_has_priority_and_air_b_still_bounces(){
 let mut p=Player::new();p.tick(Input{jump_pressed:true,bounce_pressed:true,..Default::default()});
 assert!(!p.grounded);assert_ne!(p.state,State::Somersault);
 p.tick(Input{bounce_pressed:true,..Default::default()});assert_eq!(p.state,State::BounceDown);
}
#[test]
fn somersault_can_jump_or_charge(){
 for charge in [false,true]{
 let mut p=Player::new();p.tick(Input{bounce_pressed:true,..Default::default()});
 p.tick(Input{jump_pressed:!charge,charge,..Default::default()});
 assert_eq!(p.state,if charge{State::Charge}else{State::Jump});
 }
}
#[test]
fn somersault_hits_target_and_respects_source_wall(){
 use sonic_playground::simulation::{World,DemoWorld};use glam::{Vec3,vec3};
 struct Host{spawn:Vec3,target:Vec3}
 impl World for Host{
 fn source_terrain(&self)->Option<&sa2_runtime::static_terrain_host::StaticTerrain>{DemoWorld.source_terrain()}
 fn spawn(&self)->Vec3{self.spawn}fn target_count(&self)->usize{1}fn target(&self,_:usize)->Option<Vec3>{Some(self.target)}
 fn ring_count(&self)->usize{0}fn ring(&self,_:usize)->Option<Vec3>{None}
 fn attack_path_clear(&self,a:Vec3,b:Vec3)->bool{DemoWorld.attack_path_clear(a,b)}
 fn move_player(&self,_:&mut Player,_:f32){panic!("unexpected legacy collision")}
 fn should_respawn(&self,p:Vec3)->bool{p.y < -10.}
 }
 let w=Host{spawn:vec3(0.,0.,-6.),target:vec3(0.,0.65,-5.)};let mut p=Player::in_world(&w);
 p.tick_in(Input{bounce_pressed:true,..Default::default()},&w);
 for _ in 0..30{p.tick_in(Input::default(),&w);}
 assert!(p.defeated[0]);
 let w=Host{spawn:vec3(12.6,0.,3.),target:vec3(14.5,0.65,3.)};let mut p=Player::in_world(&w);p.yaw=std::f32::consts::FRAC_PI_2;
 p.tick_in(Input{bounce_pressed:true,..Default::default()},&w);
 for _ in 0..100{p.tick_in(Input::default(),&w);assert!(p.position.x<13.);assert!(p.position.is_finite());}
 assert!(!p.defeated[0]);assert_eq!(p.respawns,0);
}
#[test]
fn recovery_animation_runs_once_without_restarting(){
 let mut p=Player::new();p.tick(Input{bounce_pressed:true,..Default::default()});
 let mut entered=0;let mut was_recovery=false;let mut left_recovery=false;
 for _ in 0..110{
  let c=p.source_animation_clock().unwrap().output();let recovery=c.current==94;
  if recovery!=was_recovery{println!("transition ticks={} state={:?} clock={c:?}",p.ticks,p.state);}
  if recovery&&!was_recovery{entered+=1;assert!(!left_recovery,"recovery restarted: {c:?}");}
  if was_recovery&&!recovery{left_recovery=true;}
  was_recovery=recovery;p.tick(Input::default());
 }
 assert_eq!(entered,1);assert_ne!(p.state,State::Somersault);
}
#[test]
fn high_speed_entry_replays_then_recovers_once(){
 use glam::vec3;
 for speed in [0.,36.]{
  let mut p=Player::new();p.velocity=vec3(0.,0.,speed);p.state=State::Run;
  p.tick(Input{bounce_pressed:true,..Default::default()});
  let first=if speed==0.{91}else{96};assert_eq!(p.source_animation_clock().unwrap().output().selected,first);
  let(mut restarts,mut previous,mut recovery,mut was_recovery)=(0,first,0,false);
  for _ in 0..160{
   let o=p.source_animation_clock().unwrap().output();
   if o.selected==96&&previous==97{restarts+=1;}
   if o.current==94&&!was_recovery{recovery+=1;}
   was_recovery=o.current==94;previous=o.selected;
   p.tick(Input::default());assert!(p.position.is_finite());assert_eq!(p.respawns,0);
   if p.state!=State::Somersault{break;}
  }
  assert_eq!(restarts,if speed==0.{0}else{1});assert_eq!(recovery,1);assert_ne!(p.state,State::Somersault);
 }
}
#[test]
fn high_speed_late_presses_chain_and_cancellation_remains_available(){
 use glam::vec3;
 for cancel in [0,1,2]{
  let mut p=Player::new();p.velocity=vec3(0.,0.,36.);p.state=State::Run;
  p.tick(Input{bounce_pressed:true,..Default::default()});
  let(mut previous,mut replayed,mut second,mut third)=(96,false,false,false);
  let mut seen=std::collections::BTreeSet::new();
  for _ in 0..180{
   let o=p.source_animation_clock().unwrap().output();seen.insert(o.current);
   if o.selected==96&&previous==97{replayed=true;}
   previous=o.selected;
   let press=if replayed&&!second{second=true;true}else if second&&!third&&o.current==97&&o.mode!=2&&o.frame>=4.{third=true;true}else{false};
   if press&&cancel!=0{p.tick(Input{jump_pressed:cancel==1,charge:cancel==2,..Default::default()});assert_eq!(p.state,if cancel==1{State::Jump}else{State::Charge});break;}
   p.tick(Input{bounce_pressed:press,..Default::default()});
   if p.state!=State::Somersault{break;}
  }
  assert!(replayed&&second);assert_ne!(p.state,State::Somersault);assert_eq!(p.respawns,0);
  if cancel==0{assert!(third);assert!(seen.contains(&96)&&seen.contains(&97)&&seen.contains(&98)&&seen.contains(&99),"{seen:?}");assert!(!seen.contains(&94),"extra recovery after third stage");}
 }
}
