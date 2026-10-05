use sa2_runtime::{ring_motion_host::RingMotion,ground_motion_host::GroundMotion,static_terrain_host::StaticTerrain,original_math::OriginalMath,original_ring_dash::RingCandidate,original_position_a::PositionContext,original_position_b::PositionBContext,original_control::ControlContext};
fn context()->PositionContext{PositionContext{control:ControlContext{directions:[0;2],magnitudes:[0.;2],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0},incoming_f26:0.,entity_identity:0x80210100,motion_identity:0x80210600,character_identity:0x80210200,workspace_identity:0x80210000,platform_offset_identity:0x80212000}}
fn read(b:&[u8],i:usize)->f32{f32::from_be_bytes(b[i..i+4].try_into().unwrap())}
#[test]fn persistent_ring_physics_moves_on_actual_source_floor(){
 let math=OriginalMath::from_data_bundle(include_bytes!("../assets/original-math-data.bin").to_vec()).unwrap();
 let terrain=StaticTerrain::new_mesh(&[[[-1000.,0.,-1000.],[-1000.,0.,1000.],[1000.,0.,1000.]],[[-1000.,0.,-1000.],[1000.,0.,1000.],[1000.,0.,-1000.]]]).unwrap();
 let g=GroundMotion::new(include_bytes!("../assets/physics.bin")).unwrap();let mut state=terrain.state(&g);state.entity[0x2c..0x30].copy_from_slice(&0x80213000u32.to_be_bytes());state.memory.bytes[0x13011]=255;state.character[0x27]=1;
 let offered=[RingCandidate{position:[20.,5.4,0.],offset:[0.;3],cached_distance:400.,collider_available:true}];
 assert_eq!(sa2_runtime::ring_motion_host::offer_and_activate(&mut state,[0.,5.4,0.],&offered,true,true,&math).unwrap(),sa2_runtime::original_action_choice::Choice::Offered(59));
 let mut ring=RingMotion::from_active(state).unwrap();let common=context();let ctx=PositionBContext{common:&common,direction_mode:0,special_offset:0.,special_radius:0.};
 for _ in 0..20{
  let before=read(&ring.records().entity,0x14);let center=[before,5.4,0.];let targets=[RingCandidate{position:[before+20.,5.4,0.],offset:[0.;3],cached_distance:400.,collider_available:true}];
  let out=ring.physics(center,&targets,6.,&terrain.world(),&ctx,&math,|_|Ok(()),|_,_,_,_,_|panic!("unexpected platform callback")).unwrap();
  assert_eq!(out.selected,Some(0));assert_eq!(out.collision_response,0);assert!(!out.stop_sound&&!out.reset_requested);
  assert!(read(&ring.records().entity,0x14)>before+10.);assert!(read(&ring.records().entity,0x18).abs()<0.001);assert_eq!(ring.records().entity[0],59);
 }
}
#[test]fn ring_collision_failure_preserves_motion_and_host_memory(){
 let math=OriginalMath::from_data_bundle(include_bytes!("../assets/original-math-data.bin").to_vec()).unwrap();let terrain=StaticTerrain::new(&[]).unwrap();let g=GroundMotion::new(include_bytes!("../assets/physics.bin")).unwrap();
 let mut state=terrain.state(&g);state.entity[0x2c..0x30].copy_from_slice(&0x80213000u32.to_be_bytes());state.memory.bytes[0x13011]=255;let mut ring=RingMotion::enter(state,&math).unwrap();let before=ring.records().clone();
 let mut common=context();common.entity_identity=0xffffffff;let ctx=PositionBContext{common:&common,direction_mode:0,special_offset:0.,special_radius:0.};
 let targets=[RingCandidate{position:[20.,5.4,0.],offset:[0.;3],cached_distance:400.,collider_available:true}];
 assert!(ring.physics([0.,5.4,0.],&targets,6.,&terrain.world(),&ctx,&math,|m|{m.bytes[0]=123;Ok(())},|_,_,_,_,_|Ok(())).is_err());
 let after=ring.records();assert_eq!(after.entity,before.entity);assert_eq!(after.motion,before.motion);assert_eq!(after.character,before.character);assert_eq!(after.workspace,before.workspace);assert_eq!(after.memory.bytes,before.memory.bytes);assert_eq!(after.candidates.pointers,before.candidates.pointers);
}
#[test]fn ring_wall_containment_until_timeout(){
 let math=OriginalMath::from_data_bundle(include_bytes!("../assets/original-math-data.bin").to_vec()).unwrap();
 let terrain=StaticTerrain::new(&[[[30.,-100.,-100.],[30.,-100.,100.],[30.,100.,100.]],[[30.,-100.,-100.],[30.,100.,100.],[30.,100.,-100.]],
 [[-1000.,0.,-1000.],[-1000.,0.,1000.],[1000.,0.,1000.]],[[-1000.,0.,-1000.],[1000.,0.,1000.],[1000.,0.,-1000.]]]).unwrap();
 let g=GroundMotion::new(include_bytes!("../assets/physics.bin")).unwrap();let mut state=terrain.state(&g);state.entity[5]=1;state.entity[0x2c..0x30].copy_from_slice(&0x80213000u32.to_be_bytes());state.memory.bytes[0x13011]=255;let mut ring=RingMotion::enter(state,&math).unwrap();let common=context();let ctx=PositionBContext{common:&common,direction_mode:0,special_offset:0.,special_radius:0.};
 let mut stopped=false;
 for tick in 0..370{
  if ring.decide_timeout(0,&math).unwrap(){assert_eq!(tick,361);assert_eq!(ring.records().entity[0],10);stopped=true;break;}
  let x=read(&ring.records().entity,0x14);let y=read(&ring.records().entity,0x18);let center=[x,y+5.4,0.];let targets=[RingCandidate{position:[x+20.,y+5.4,0.],offset:[0.;3],cached_distance:400.,collider_available:true}];
  let out=ring.physics(center,&targets,6.,&terrain.world(),&ctx,&math,|_|Ok(()),|_,_,_,_,_|panic!("unexpected platform callback")).unwrap();
  assert!(read(&ring.records().entity,0x14)<30.01,"passed through wall tick={tick} y={}",read(&ring.records().entity,0x18));
  assert!(!out.stop_sound);assert_eq!(out.collision_response,0);
 }
 assert!(stopped,"blocked dash did not time out");assert!(read(&ring.records().entity,0x14)>0.,"dash never approached wall");
}

#[test]fn ring_without_pickups_expires_despite_targets_remaining(){
 let math=OriginalMath::from_data_bundle(include_bytes!("../assets/original-math-data.bin").to_vec()).unwrap();
 let terrain=StaticTerrain::new_mesh(&[[[-10000.,0.,-1000.],[-10000.,0.,1000.],[10000.,0.,1000.]],[[-10000.,0.,-1000.],[10000.,0.,1000.],[10000.,0.,-1000.]]]).unwrap();
 let g=GroundMotion::new(include_bytes!("../assets/physics.bin")).unwrap();let mut state=terrain.state(&g);state.entity[0x2c..0x30].copy_from_slice(&0x80213000u32.to_be_bytes());state.memory.bytes[0x13011]=255;let mut ring=RingMotion::enter(state,&math).unwrap();let common=context();let ctx=PositionBContext{common:&common,direction_mode:0,special_offset:0.,special_radius:0.};
 for tick in 0..=361{
  let expired=ring.decide_timeout(0,&math).unwrap();
  assert_eq!(expired,tick==361,"wrong elapsed timeout boundary at tick {tick}");
  if expired{assert_eq!(ring.records().entity[0],10);assert_eq!(&ring.records().character[0x362..0x364],&[0,0]);return;}
  let x=read(&ring.records().entity,0x14);let targets=[RingCandidate{position:[x+20.,5.4,0.],offset:[0.;3],cached_distance:400.,collider_available:true}];
  let out=ring.physics([x,5.4,0.],&targets,6.,&terrain.world(),&ctx,&math,|_|Ok(()),|_,_,_,_,_|panic!("unexpected platform callback")).unwrap();
  assert_eq!(out.selected,Some(0));assert_eq!(out.collision_response,0);assert!(!out.stop_sound);assert_eq!(ring.records().entity[0],59);
 }
 panic!("dash failed to time out");
}

#[test]fn ring_contacts_refresh_timeout_and_stop_contact_ends_action(){
 let math=OriginalMath::from_data_bundle(include_bytes!("../assets/original-math-data.bin").to_vec()).unwrap();
 let terrain=StaticTerrain::new_mesh(&[[[-20000.,0.,-1000.],[-20000.,0.,1000.],[20000.,0.,1000.]],[[-20000.,0.,-1000.],[20000.,0.,1000.],[20000.,0.,-1000.]]]).unwrap();
 let g=GroundMotion::new(include_bytes!("../assets/physics.bin")).unwrap();let mut state=terrain.state(&g);state.entity[0x2c..0x30].copy_from_slice(&0x80213000u32.to_be_bytes());state.memory.bytes[0x13011]=255;let mut ring=RingMotion::enter(state,&math).unwrap();let common=context();let ctx=PositionBContext{common:&common,direction_mode:0,special_offset:0.,special_radius:0.};
 for tick in 0..500{
  assert!(!ring.decide_timeout(0,&math).unwrap());
  let x=read(&ring.records().entity,0x14);
  if tick%100==0{
   assert!(!ring.contact(0x11,[x,5.4,0.],&math).unwrap());
   assert!(!ring.contact(0x11,[x,5.4,0.],&math).unwrap());
   assert_eq!(i16::from_be_bytes(ring.records().character[0x36e..0x370].try_into().unwrap()),tick/100+1);
   assert_eq!(&ring.records().character[0x366..0x368],&[0,0]);
  }
  let targets=[RingCandidate{position:[x+20.,5.4,0.],offset:[0.;3],cached_distance:400.,collider_available:true}];
  let out=ring.physics([x,5.4,0.],&targets,6.,&terrain.world(),&ctx,&math,|_|Ok(()),|_,_,_,_,_|panic!("unexpected platform callback")).unwrap();assert!(!out.stop_sound);
 }
 let before=ring.records().clone();assert!(ring.contact(0x11,[f32::NAN,0.,0.],&math).is_err());assert_eq!(ring.records().entity,before.entity);assert_eq!(ring.records().character,before.character);
 assert!(ring.contact(0x15,[0.;3],&math).unwrap());assert_eq!(ring.records().entity[0],10);
 assert!(ring.decide_timeout(0,&math).is_err());assert!(ring.contact(0x11,[0.;3],&math).is_err());
 assert!(ring.physics([0.;3],&[],6.,&terrain.world(),&ctx,&math,|_|Ok(()),|_,_,_,_,_|Ok(())).is_err());
}

#[test]fn timeout_reads_control_only_at_expiry_and_rolls_back_on_failure(){
 let math=OriginalMath::from_data_bundle(include_bytes!("../assets/original-math-data.bin").to_vec()).unwrap();
 let terrain=StaticTerrain::new(&[]).unwrap();let g=GroundMotion::new(include_bytes!("../assets/physics.bin")).unwrap();
 let mut ring=RingMotion::enter(terrain.state(&g),&math).unwrap();
 let mut common=context();common.control.magnitudes[0]=f32::NAN;
 for _ in 0..10{assert!(!ring.decide_timeout_with_control(&common.control,&math).unwrap());}
 let before=ring.records().clone();
 assert!(ring.decide_timeout_with_control(&common.control,&math).is_err());
 assert_eq!(ring.records().entity,before.entity);assert_eq!(ring.records().character,before.character);assert_eq!(ring.records().control_cache,before.control_cache);
 common.control.magnitudes[0]=0.;assert!(ring.decide_timeout_with_control(&common.control,&math).unwrap());assert_eq!(ring.records().entity[0],10);
}
#[test]fn timeout_uses_real_direction_class_for_multi_ring_exit_speed(){
 let math=OriginalMath::from_data_bundle(include_bytes!("../assets/original-math-data.bin").to_vec()).unwrap();
 let terrain=StaticTerrain::new(&[]).unwrap();let g=GroundMotion::new(include_bytes!("../assets/physics.bin")).unwrap();
 for (direction,expected) in [(0u32,8f32),(0x8000,2.)]{
  let mut state=terrain.state(&g);state.character[0xf4..0xf8].copy_from_slice(&8f32.to_be_bytes());
  let mut ring=RingMotion::enter(state,&math).unwrap();ring.contact(0x11,[0.,0.,0.],&math).unwrap();ring.contact(0x11,[2.,0.,0.],&math).unwrap();
  let mut common=context();common.control.magnitudes[0]=1.;common.control.directions[0]=direction;
  for _ in 0..10{assert!(!ring.decide_timeout_with_control(&common.control,&math).unwrap());}
  assert!(ring.decide_timeout_with_control(&common.control,&math).unwrap());
  assert!((read(&ring.records().character,0x64)-expected).abs()<0.002,"exit direction {direction}: actual {} expected {expected}",read(&ring.records().character,0x64));
 }
}

#[test]fn activation_preserves_context_priority_and_requires_fresh_press(){
 use sa2_runtime::{ring_motion_host::offer_and_activate,original_action_choice::Choice};
 let math=OriginalMath::from_data_bundle(include_bytes!("../assets/original-math-data.bin").to_vec()).unwrap();let terrain=StaticTerrain::new(&[]).unwrap();let g=GroundMotion::new(include_bytes!("../assets/physics.bin")).unwrap();
 let targets=[RingCandidate{position:[20.,5.4,0.],offset:[0.;3],cached_distance:400.,collider_available:true}];
 for (upgrade,pressed,held,competing,preferred,expected) in [
  (true,true,true,false,0,Choice::Offered(59)),(true,false,true,false,0,Choice::None),(true,false,false,false,0,Choice::None),
  (false,true,true,false,0,Choice::None),(true,true,true,true,60,Choice::Offered(60)),(true,true,true,true,59,Choice::Offered(59)),
  (true,false,false,true,59,Choice::ReleaseLightAttack)]{
  let mut s=terrain.state(&g);s.character[0x24..0x28].copy_from_slice(&(if upgrade{1u32}else{0}).to_be_bytes());s.character[13]=preferred;
  if competing{s.character[4]=60;s.character[12]=1;}
  let action=s.entity[0];assert_eq!(offer_and_activate(&mut s,[0.,5.4,0.],&targets,pressed,held,&math).unwrap(),expected);
  if expected==Choice::Offered(59){assert_eq!(s.entity[0],59);assert_eq!(&s.character[0x176..0x178],&87u16.to_be_bytes());assert_eq!(&s.character[0x362..0x364],&10u16.to_be_bytes());let ring=RingMotion::from_active(s.clone()).unwrap();assert_eq!(ring.records().character,s.character);}
  else{assert_eq!(s.entity[0],action);}
 }
}
#[test]fn failed_activation_does_not_change_offers_or_state(){
 use sa2_runtime::ring_motion_host::offer_and_activate;
 let math=OriginalMath::from_data_bundle(include_bytes!("../assets/original-math-data.bin").to_vec()).unwrap();let terrain=StaticTerrain::new(&[]).unwrap();let g=GroundMotion::new(include_bytes!("../assets/physics.bin")).unwrap();let mut s=terrain.state(&g);s.character[0x27]=1;
 let before=s.clone();let targets=[RingCandidate{position:[f32::NAN,0.,0.],offset:[0.;3],cached_distance:1.,collider_available:true}];
 assert!(offer_and_activate(&mut s,[0.;3],&targets,true,true,&math).is_err());assert_eq!(s.entity,before.entity);assert_eq!(s.character,before.character);assert_eq!(s.memory.bytes,before.memory.bytes);
}

#[test]fn public_player_enters_ring_motion_and_original_animation(){
 use sonic_playground::simulation::{Player,State,Input,DemoWorld};use glam::vec3;
 let mut p=Player::new();p.position=vec3(0.,0.,1.);
 assert!(p.begin_ring_dash(&DemoWorld).unwrap());assert_eq!(p.state,State::RingDash);
 let before=p.position;p.tick(Input::default());
 assert_eq!(p.state,State::RingDash);assert!(p.position.z>before.z+0.5);assert!(p.position.is_finite()&&p.velocity.is_finite());assert_eq!(p.respawns,0);
 assert_eq!(p.source_animation_clock().unwrap().output().selected,87);assert!(p.model_rotation().is_finite());
 let mut far=Player::new();assert!(!far.begin_ring_dash(&DemoWorld).unwrap());assert_eq!(far.state,State::Idle);
}

#[test]fn public_player_collects_flat_ring_line_and_exits(){
 use sonic_playground::simulation::{Player,State,Input};use glam::vec3;
 let mut p=Player::new();p.position=vec3(-22.,0.,-12.);p.tick(Input{action_held:true,bounce_pressed:true,..Default::default()});assert_eq!(p.state,State::RingDash);
 let mut ended=false;
 for _ in 0..400{p.tick(Input::default());assert!(p.position.is_finite()&&p.velocity.is_finite());assert_eq!(p.respawns,0);if p.state!=State::RingDash{ended=true;break;}}
 assert!(ended,"dash never ended");assert!(p.collected[15..24].iter().all(|x|*x),"collected {:?}, position {:?}",&p.collected[15..24],p.position);assert_eq!(p.collected_count(),9);assert!(p.velocity.length()<=12.01,"timeout restored stale dash velocity {:?}",p.velocity);
 for _ in 0..10{p.tick(Input::default());assert!(p.position.is_finite());assert_eq!(p.collected_count(),9);assert!(p.speed()<=12.01,"exit speed reinflated {:?}",p.velocity);}
}

#[test]fn held_action_without_fresh_edge_does_not_start_ring_dash(){
 use sonic_playground::simulation::{Player,State,Input};use glam::vec3;
 let mut p=Player::new();p.position=vec3(-22.,0.,-12.);
 for _ in 0..5{p.tick(Input{action_held:true,..Default::default()});assert_ne!(p.state,State::RingDash);}
 p.tick(Input::default());p.tick(Input{action_held:true,bounce_pressed:true,..Default::default()});assert_eq!(p.state,State::RingDash);
}
