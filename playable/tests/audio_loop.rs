use sonic_playground::{audio_loop::{LoopCursor,ChargeSoundState,ChargeSoundCommand::{Start,Stop,Pause,Resume}},simulation::{Player,State,FULL_CHARGE}};
#[test]fn intro_is_once_loop_has_no_tail_and_block_sizes_do_not_change_samples(){
 let pcm=[10,11,20,21,22,99];
 let mut a=LoopCursor::new(6,2,5).unwrap();a.start();
 let mut whole=[0;17];a.fill(&pcm,&mut whole).unwrap();
 assert_eq!(whole,[10,11,20,21,22,20,21,22,20,21,22,20,21,22,20,21,22]);
 let mut b=LoopCursor::new(6,2,5).unwrap();b.start();let mut chunked=[0;17];
 for part in chunked.chunks_mut(4){b.fill(&pcm,part).unwrap();}
 assert_eq!(whole,chunked);b.stop();b.fill(&pcm,&mut chunked).unwrap();assert_eq!(chunked,[0;17]);
 b.start();b.fill(&pcm,&mut chunked[..2]).unwrap();assert_eq!(&chunked[..2],&[10,11]);
}
#[test]fn malformed_regions_and_changed_pcm_fail_silently_and_stop(){
 for (n,s,e) in [(0,0,0),(5,3,3),(5,4,2),(5,0,6)]{assert!(LoopCursor::new(n,s,e).is_err());}
 let mut c=LoopCursor::new(5,1,5).unwrap();c.start();let mut out=[42;4];
 assert!(c.fill(&[1,2],&mut out).is_err());assert_eq!(out,[0;4]);assert!(!c.active());
}
#[test]fn charge_voice_starts_once_and_stops_for_all_state_exits(){
 let mut p=Player::new();let mut s=ChargeSoundState::default();assert!(s.update(&p,false).is_empty());
 for exit in [State::Idle,State::Roll,State::Jump,State::Fall,State::LightAttack]{
  p.state=State::Charge;p.charge=0.;assert_eq!(s.update(&p,false),vec![Start]);
  for _ in 0..119{assert!(s.update(&p,false).is_empty());}
  p.state=exit;assert_eq!(s.update(&p,false),vec![Stop]);assert!(s.update(&p,false).is_empty());
 }
 p.state=State::Charge;assert_eq!(s.update(&p,false),vec![Start]);
 p.charge=FULL_CHARGE;assert_eq!(s.update(&p,false),vec![Stop]);
 p.charge=0.;assert_eq!(s.update(&p,false),vec![Start]);
 assert_eq!(s.update(&p,true),vec![Pause]);assert!(s.update(&p,true).is_empty());assert_eq!(s.update(&p,false),vec![Resume]);
 p.reset();assert_eq!(s.update(&p,false),vec![Stop]);
 p.state=State::Charge;assert_eq!(s.update(&p,false),vec![Start]);p.respawns+=1;
 assert_eq!(s.update(&p,false),vec![Stop,Start]);
}
#[test]fn paused_reset_and_charge_exit_discard_voice_without_resuming(){
 let mut p=Player::new();let mut s=ChargeSoundState::default();p.state=State::Charge;
 assert!(s.update(&p,true).is_empty());assert_eq!(s.update(&p,false),vec![Start]);
 assert_eq!(s.update(&p,true),vec![Pause]);p.respawns+=1;
 assert_eq!(s.update(&p,true),vec![Stop]);assert!(s.update(&p,true).is_empty());
 assert_eq!(s.update(&p,false),vec![Start]);assert_eq!(s.update(&p,true),vec![Pause]);
 p.charge=FULL_CHARGE;assert_eq!(s.update(&p,true),vec![Stop]);assert!(s.update(&p,false).is_empty());
}
#[test]
fn ring_voice_lifecycle_exits_pause_and_respawn(){
 let mut p=Player::new();let mut s=ChargeSoundState::default();
 let update=|s:&mut ChargeSoundState,p:&Player,pause|s.update_active(p.state==State::RingDash,p.respawns,pause);
 for exit in [State::Idle,State::Run,State::Fall,State::Jump,State::LightAttack]{
  p.state=State::RingDash;
  assert!(update(&mut s,&p,true).is_empty());
  assert_eq!(update(&mut s,&p,false),vec![Start]);
  for _ in 0..400 {assert!(update(&mut s,&p,false).is_empty());}
  assert_eq!(update(&mut s,&p,true),vec![Pause]);
  assert!(update(&mut s,&p,true).is_empty());
  assert_eq!(update(&mut s,&p,false),vec![Resume]);
  p.state=exit;assert_eq!(update(&mut s,&p,false),vec![Stop]);
 }
 p.state=State::RingDash;assert_eq!(update(&mut s,&p,false),vec![Start]);
 p.respawns+=1;assert_eq!(update(&mut s,&p,false),vec![Stop,Start]);
 assert_eq!(update(&mut s,&p,true),vec![Pause]);
 p.reset();assert_eq!(update(&mut s,&p,true),vec![Stop]);
 assert!(update(&mut s,&p,false).is_empty());
}
