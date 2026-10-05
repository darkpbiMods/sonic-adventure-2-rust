use sonic_playground::{controller::{Controller,RawGamepad,A,B,X},simulation::{Player,State,Clock,FULL_CHARGE}};
fn tick(c:&mut Controller,p:&mut Player,buttons:u16){p.tick(c.sample(Some((0,RawGamepad{buttons,..Default::default()}))).movement(0.));}
#[test]
fn both_aliases_require_hold_and_release_tap_somersault(){
 for button in [B,X]{
  let mut c=Controller::default();let mut p=Player::new();tick(&mut c,&mut p,0);
  for _ in 0..24{tick(&mut c,&mut p,button);assert_ne!(p.state,State::Charge);assert_ne!(p.state,State::Somersault);}
  tick(&mut c,&mut p,button);assert_eq!(p.state,State::Charge);
  tick(&mut c,&mut p,0);assert_eq!(p.state,State::Roll);
  let mut p=Player::new();for _ in 0..4{tick(&mut c,&mut p,button);}
  tick(&mut c,&mut p,0);assert_eq!(p.state,State::Somersault);
 }
}
#[test]
fn aliases_overlap_without_new_press_or_timer_reset(){
 let mut c=Controller::default();let mut p=Player::new();tick(&mut c,&mut p,0);
 for _ in 0..10{tick(&mut c,&mut p,B);}
 for _ in 0..10{tick(&mut c,&mut p,B|X);}
 for _ in 0..5{tick(&mut c,&mut p,X);}
 assert_eq!(p.state,State::Charge);
}
#[test]
fn connection_with_held_action_does_not_invent_charge(){
 let mut c=Controller::default();let mut p=Player::new();
 for _ in 0..60{tick(&mut c,&mut p,B);assert_eq!(p.charge,0.);}
 tick(&mut c,&mut p,0);assert_ne!(p.state,State::Somersault);
 for _ in 0..25{tick(&mut c,&mut p,B);}assert_eq!(p.state,State::Charge);
}
#[test]
fn shared_charge_survives_jump_and_releases_in_air(){
 let mut c=Controller::default();let mut p=Player::new();tick(&mut c,&mut p,0);
 for _ in 0..150{tick(&mut c,&mut p,B);}assert_eq!(p.charge,FULL_CHARGE);
 tick(&mut c,&mut p,A|B);assert!(!p.grounded);assert_eq!(p.charge,FULL_CHARGE);
 for _ in 0..8{tick(&mut c,&mut p,B);assert_eq!(p.charge,FULL_CHARGE);}
 tick(&mut c,&mut p,0);assert_eq!(p.state,State::LightAttack);
}
#[test]
fn shared_hold_timing_is_independent_of_render_rate(){
 let mut outcomes=Vec::new();
 for hz in [30,60,144]{
  let mut c=Controller::default();let mut p=Player::new();let mut clock=Clock::default();c.sample(Some((0,RawGamepad::default())));
  for _ in 0..hz{let input=c.sample(Some((0,RawGamepad{buttons:B,..Default::default()}))).movement(0.);clock.advance(&mut p,input,1./hz as f64);}
  outcomes.push((p.ticks,p.state,p.charge));
 }
 assert_eq!(outcomes[0],outcomes[1]);assert_eq!(outcomes[1],outcomes[2]);assert_eq!(outcomes[0].1,State::Charge);
}
#[test]
fn three_deliberate_presses_follow_original_chain_without_extra_kick(){
 let mut c=Controller::default();let mut p=Player::new();tick(&mut c,&mut p,0);
 for _ in 0..4{tick(&mut c,&mut p,B);}tick(&mut c,&mut p,0);
 assert_eq!(p.state,State::Somersault);
 tick(&mut c,&mut p,B);tick(&mut c,&mut p,0);
 let mut third=false;let mut observed=std::collections::BTreeSet::new();let mut completed=false;
 for _ in 0..120{
  let clock=p.source_animation_clock().unwrap().output();observed.insert(clock.current);
  let press=!third&&clock.current==92&&clock.mode!=2&&clock.frame>=4.;
  if press{third=true;}
  tick(&mut c,&mut p,if press{B}else{0});
  if p.state!=State::Somersault{completed=true;break;}
 }
 assert!(third&&completed);assert!(observed.contains(&91)&&observed.contains(&92)&&observed.contains(&93),"{observed:?}");
 assert!(!observed.contains(&94),"third-stage exit must not add cancel kick: {observed:?}");assert_eq!(p.respawns,0);
}
#[test]
fn held_second_press_does_not_invent_third_stage(){
 let mut c=Controller::default();let mut p=Player::new();tick(&mut c,&mut p,0);
 for _ in 0..4{tick(&mut c,&mut p,B);}tick(&mut c,&mut p,0);
 let mut saw_recovery=false;
 for _ in 0..100{
  tick(&mut c,&mut p,B);
  saw_recovery|=p.source_animation_clock().is_some_and(|a|a.output().current==94);
 }
 assert!(saw_recovery);assert_ne!(p.state,State::Somersault);assert_eq!(p.charge,0.);
}
#[test]
fn queued_combo_is_cleared_by_jump_and_fresh_entry(){
 let mut c=Controller::default();let mut p=Player::new();tick(&mut c,&mut p,0);
 for _ in 0..4{tick(&mut c,&mut p,B);}tick(&mut c,&mut p,0);tick(&mut c,&mut p,B);tick(&mut c,&mut p,0);
 tick(&mut c,&mut p,A);assert!(!p.grounded);
 for _ in 0..150{tick(&mut c,&mut p,0);}
 assert!(p.grounded);
 for _ in 0..4{tick(&mut c,&mut p,B);}tick(&mut c,&mut p,0);
 let mut recovery=false;
 for _ in 0..100{tick(&mut c,&mut p,0);recovery|=p.source_animation_clock().is_some_and(|a|a.output().current==94);}
 assert!(recovery);assert_ne!(p.state,State::Somersault);
}
#[cfg(feature="reference-tests")]
#[test]
fn combo_original_pose_and_blend_frames_are_valid(){
 use sonic_playground::source_pose::SourcePose;
 let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();let dir=root.join("movement-verification");
 let pose=SourcePose::from_math_bundle(&std::fs::read(dir.join("animation-bundle-audit.json")).unwrap(),std::fs::read(dir.join("animation-bundle-audit.json.bin")).unwrap(),std::fs::read(root.join("playable/assets/original-math-data.bin")).unwrap()).unwrap();
 let mut c=Controller::default();let mut p=Player::new();tick(&mut c,&mut p,0);
 for _ in 0..4{tick(&mut c,&mut p,B);}tick(&mut c,&mut p,0);tick(&mut c,&mut p,B);tick(&mut c,&mut p,0);
 let mut third=false;let mut count=0;
 for _ in 0..100{
  let clock=p.source_animation_clock().unwrap();let o=clock.output();
  if let Some(b)=clock.blend(){pose.sample_transition(b.from_animation,b.from_frame,b.to_animation,b.to_frame,o.frame,0,0).unwrap();}
  else{pose.sample(o.current,o.frame,0,0).unwrap();}
  count+=1;
  let press=!third&&o.current==92&&o.mode!=2&&o.frame>=4.;if press{third=true;}
  tick(&mut c,&mut p,if press{B}else{0});if p.state!=State::Somersault{break;}
 }
 assert!(third&&count>30);assert_ne!(p.state,State::Somersault);
}
