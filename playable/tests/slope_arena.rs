use sonic_playground::slope_arena::SlopeArena;
#[test]
fn connected_mesh_has_conservative_bounds_and_complete_sorted_chain(){
 use sa2_runtime::static_terrain_host::StaticTerrain;use glam::Vec3;
 let vertices=SlopeArena::new().source_vertices();let mesh=StaticTerrain::new_mesh(&vertices).unwrap();let w=mesh.world();
 assert_eq!(w.objects.len(),1);assert_eq!(w.static_entries.len(),1);assert_eq!(w.triangles.len(),vertices.len());
 let o=&w.objects[0];let c=Vec3::from_array(o.position);
 for p in vertices.iter().flatten(){assert!(c.distance(Vec3::from_array(*p))<=o.model_radius);}
 let word=|b:&[u8],i:usize|u32::from_be_bytes(b[i..i+4].try_into().unwrap());
 let mut head=w.triangles.iter().find(|t|word(&t.data,4)==0).unwrap();let mut previous=0;let mut z=f32::NEG_INFINITY;let mut seen=std::collections::HashSet::new();
 loop{
  assert!(seen.insert(head.identifier));assert_eq!(word(&head.data,4),previous);
  let next_z=f32::from_bits(word(&head.data,16));assert!(next_z>=z);z=next_z;
  let next=word(&head.data,0);if next==0{break;}previous=head.identifier;head=w.triangles.iter().find(|t|t.identifier==next).unwrap();
 }
 assert_eq!(seen.len(),vertices.len());
 assert!(StaticTerrain::new_mesh(&[]).unwrap().world().objects.is_empty());
 assert!(StaticTerrain::new_mesh(&[[[f32::NAN;3];3]]).is_err());
 let flagged=StaticTerrain::new_mesh_with_flags(&vertices,4).unwrap();
 assert_eq!(flagged.world().objects[0].flags,4);assert_eq!(flagged.world().static_entries[0].flags,4);
}
#[test]
fn arena_faces_are_finite_nondegenerate_and_inward(){
 let a=SlopeArena::new();assert_eq!(a.bowl.len(),352);assert_eq!(a.tunnel.len(),64);
 for t in a.bowl.iter().chain(&a.tunnel){assert!(t.iter().all(|v|v.is_finite()));assert!((t[1]-t[0]).cross(t[2]-t[0]).length()>0.01);}
 for t in &a.bowl{let n=(t[1]-t[0]).cross(t[2]-t[0]).normalize();assert!(n.y>=-1e-5);let center=(t[0]+t[1]+t[2])/3.;let delta=SlopeArena::BOWL_CENTER-center;assert!(n.x*delta.x+n.z*delta.z>=-1e-4);}
 for t in &a.tunnel{let n=(t[1]-t[0]).cross(t[2]-t[0]).normalize();let center=(t[0]+t[1]+t[2])/3.;let d=SlopeArena::TUNNEL_CENTER-center;assert!(n.x*d.x+n.y*d.y>11.);assert!(n.z.abs()<1e-5);}
}
#[test]
fn tube_seam_is_welded_and_only_ends_are_open(){
 let a=SlopeArena::new();let mut edges=std::collections::BTreeMap::new();
 for t in a.tunnel{for i in 0..3{let key=|p:glam::Vec3|p.to_array().map(|v|if v==0.{0}else{v.to_bits()});let mut edge=[key(t[i]),key(t[(i+1)%3])];edge.sort();*edges.entry(edge).or_insert(0)+=1;}}
 let open:Vec<_>=edges.iter().filter(|(_,n)|**n==1).collect();assert_eq!(open.len(),64);assert!(edges.values().all(|n|*n==1||*n==2));
 for (edge,_) in open{let z0=f32::from_bits(edge[0][2]);let z1=f32::from_bits(edge[1][2]);assert_eq!(z0,z1);assert!(z0==100.||z0==190.);}
}
#[test]
fn source_collision_accepts_large_arena_and_supports_bowl_floor(){
 use sonic_playground::simulation::{World,Player,Input};use glam::Vec3;
 use sa2_runtime::static_terrain_host::StaticTerrain;
 struct Host{terrain:StaticTerrain}
 impl World for Host{
 fn source_terrain(&self)->Option<&StaticTerrain>{Some(&self.terrain)}
 fn spawn(&self)->Vec3{SlopeArena::BOWL_CENTER}
 fn target_count(&self)->usize{0}fn target(&self,_:usize)->Option<Vec3>{None}
 fn ring_count(&self)->usize{0}fn ring(&self,_:usize)->Option<Vec3>{None}
 fn attack_path_clear(&self,_:Vec3,_:Vec3)->bool{true}
 fn move_player(&self,_:&mut Player,_:f32){panic!("arena used fallback collision")}
 fn should_respawn(&self,p:Vec3)->bool{p.y < -50.}
 }
 let vertices=SlopeArena::new().source_vertices();assert_eq!(vertices.len(),416);
 let world=Host{terrain:StaticTerrain::new(&vertices).unwrap()};let mut p=Player::in_world(&world);
 for _ in 0..120{p.tick_in(Input::default(),&world);assert!(p.grounded);assert!(p.position.distance(SlopeArena::BOWL_CENTER)<0.001);}
 assert_eq!(p.respawns,0);
 let triangle=vertices[0];assert!(StaticTerrain::new(&vec![triangle;1024]).is_ok());assert!(StaticTerrain::new(&vec![triangle;1025]).is_err());
}
#[test]
fn demo_bowl_is_reachable_without_early_respawn(){
 use sonic_playground::simulation::{World,DemoWorld,Player,Input};use glam::{Vec3,vec2,vec3};
 struct Entrance;
 impl World for Entrance{
 fn source_terrain(&self)->Option<&sa2_runtime::static_terrain_host::StaticTerrain>{DemoWorld.source_terrain()}
 fn spawn(&self)->Vec3{vec3(-40.,0.,30.)}
 fn target_count(&self)->usize{0}fn target(&self,_:usize)->Option<Vec3>{None}
 fn ring_count(&self)->usize{0}fn ring(&self,_:usize)->Option<Vec3>{None}
 fn ground_normal(&self,p:Vec3)->Vec3{DemoWorld.ground_normal(p)}
 fn attack_path_clear(&self,a:Vec3,b:Vec3)->bool{DemoWorld.attack_path_clear(a,b)}
 fn move_player(&self,_:&mut Player,_:f32){panic!("bowl entry used fallback collision")}
 fn should_respawn(&self,p:Vec3)->bool{DemoWorld.should_respawn(p)}
 }
 let w=Entrance;let mut p=Player::in_world(&w);let mut depth=0f32;
 for _ in 0..200{p.tick_in(Input{direction:vec2(-1.,0.),..Default::default()},&w);depth=depth.min(p.position.y);assert!(p.position.is_finite());assert_eq!(p.respawns,0);}
 println!("bowl entry depth={depth} final={:?}",p.position);assert!(depth < -12.);
 let (height,normal)=SlopeArena::shared().support(SlopeArena::BOWL_CENTER,0.).unwrap();assert!((height+16.).abs()<0.001);assert!(normal.y>0.99);
 assert!(!DemoWorld.should_respawn(SlopeArena::BOWL_CENTER));assert!(DemoWorld.should_respawn(vec3(0.,-51.,0.)));
}
#[test]
fn camera_hits_tube_sides_ceiling_and_seam_but_not_open_ends(){
 use glam::{Vec3,vec3};let arena=SlopeArena::new();let center=vec3(0.,12.,145.);
 for i in 0..64{
  let angle=i as f32*std::f32::consts::TAU/64.;let direction=vec3(angle.sin(),angle.cos(),0.);
  let far=center+direction*20.;let fraction=arena.segment_hit(center,far).unwrap();assert!((0.59..=0.601).contains(&fraction));
  let clipped=arena.clip_camera(center,far);assert!((11.7..=11.801).contains(&clipped.distance(center)));assert!(arena.segment_hit(center,clipped).is_none());
 }
 for d in [Vec3::Z,-Vec3::Z]{assert!(arena.segment_hit(center,center+d*100.).is_none());}
 assert!(arena.segment_hit(center,center+Vec3::X*4.).is_none());
 assert!(arena.segment_hit(center,center).is_none());assert!(arena.segment_hit(center,Vec3::splat(f32::NAN)).is_none());
}
#[test]
fn camera_takes_nearest_surface_and_bowl_bottom(){
 use glam::vec3;let a=SlopeArena::new();
 let left=vec3(-20.,12.,145.);let right=vec3(20.,12.,145.);
 assert!((a.segment_hit(left,right).unwrap()-0.2).abs()<0.0001);
 let top=SlopeArena::BOWL_CENTER+vec3(0.,25.,0.);let bottom=SlopeArena::BOWL_CENTER-vec3(0.,5.,0.);
 assert!((a.segment_hit(top,bottom).unwrap()-25./30.).abs()<0.0001);
 let clipped=a.clip_camera(top,bottom);assert!((clipped.y+15.8).abs()<0.001);
}
#[test]
fn source_roll_completes_tube_loops_and_slow_roll_detaches(){
 use glam::{Vec3,vec3};
 use sa2_runtime::{ground_motion_host::GroundMotion,original_math::OriginalMath,static_terrain_host::StaticTerrain,original_control::ControlContext,original_position_a::PositionContext,original_position_b::PositionBContext};
 let math=OriginalMath::from_data_bundle(include_bytes!("../assets/original-math-data.bin").to_vec()).unwrap();
 let vertices:Vec<_>=SlopeArena::new().tunnel.iter().map(|t|t.map(|p|(p*10.).to_array())).collect();
 let terrain=StaticTerrain::new_mesh(&vertices).unwrap();
 let common=PositionContext{control:ControlContext{directions:[0;2],magnitudes:[0.;2],stick_axis:[0;2],player_tasks:[0;2],gravity:[0.,-1.,0.],level:0,control_mode:0},incoming_f26:0.,entity_identity:0x80210100,motion_identity:0x80210600,character_identity:0x80210200,workspace_identity:0x80210000,platform_offset_identity:0x80212000};
 let ctx=PositionBContext{common:&common,direction_mode:0,special_offset:0.,special_radius:0.};
 for terrain in [&terrain,sonic_playground::simulation::demo_source_terrain()]{
 for z in [102.,115.,130.,143.,145.,160.,175.,188.]{for speed in [30.,60.,90.]{
  let mut g=GroundMotion::new(include_bytes!("../assets/physics.bin")).unwrap();let mut collision=terrain.state(&g);
  let(mut p,mut v,mut yaw)=(vec3(0.,0.,z),vec3(speed,0.,0.),std::f32::consts::FRAC_PI_2);
  let(mut previous,mut total,mut inverted,mut detached)=(0f32,0f32,false,false);
  for _ in 0..240{
   g.tick_roll(p.to_array(),v.to_array(),yaw,[0.;2],true,&math).unwrap();
   let out=g.resolve_source_terrain(&mut collision,&terrain.world(),&ctx,&math).unwrap();
   p=Vec3::from_array(out.position);v=Vec3::from_array(g.reconciled_terrain_velocity(&math).unwrap());yaw=out.yaw;
   assert!(p.is_finite()&&v.is_finite());assert!((p.z-z).abs()<0.002);
   if !out.grounded{detached=true;break;}
   let a=p.x.atan2(12.-p.y);total+=(a-previous+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;previous=a;
   inverted|=out.source_normal[1]< -0.8;
   if total>=std::f32::consts::TAU{break;}
  }
  if speed==30.{assert!(detached&&!inverted&&total<std::f32::consts::FRAC_PI_2,"slow roll should detach z={z}");}
  else{assert!(!detached&&inverted&&total>=std::f32::consts::TAU,"incomplete loop z={z} speed={speed} angle={total}");}
 }}}
}

#[test]
fn connected_ranges_preserve_other_objects_and_reject_invalid_groups(){
 use sa2_runtime::static_terrain_host::StaticTerrain;
 let vertices=SlopeArena::new().source_vertices();let base=StaticTerrain::new(&vertices).unwrap();
 let grouped=StaticTerrain::with_mesh_ranges(&vertices,&[(352..416,0)]).unwrap();
 assert_eq!(grouped.world().objects.len(),353);assert_eq!(grouped.world().triangles.len(),416);
 for i in 0..352{assert_eq!(base.world().objects[i].identifier,grouped.world().objects[i].identifier);assert_eq!(base.world().triangles[i].data,grouped.world().triangles[i].data);}
 for ranges in [vec![(1..1,0)],vec![(0..417,0)],vec![(1..4,0),(3..6,0)]]{assert!(StaticTerrain::with_mesh_ranges(&vertices,&ranges).is_err());}
 let split=StaticTerrain::with_mesh_ranges(&vertices,&[(352..384,0),(384..416,4)]).unwrap();assert_eq!(split.world().objects.len(),354);
}
#[test]
fn public_player_rolls_full_loop_and_jumps_from_ceiling(){
 use sonic_playground::simulation::{Player,Input,State};use glam::{vec3,Vec3};
 for sign in [-1.,1.]{for jump in [false,true]{
  let mut p=Player::new();p.position=vec3(0.,0.,145.);p.velocity=vec3(60.*sign,0.,0.);p.yaw=sign*std::f32::consts::FRAC_PI_2;p.state=State::Roll;
  let(mut previous,mut total,mut tested)=(0f32,0f32,false);
  for _ in 0..180{
   p.tick(Input::default());assert!(p.grounded,"lost support at {:?}, angle {total}",p.position);
   let a=p.position.x.atan2(12.-p.position.y);total+=(a-previous+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;previous=a;
   let up=p.model_rotation()*Vec3::Y;
   if jump&&up.y< -0.8{
    let before=p.velocity;p.tick(Input{jump_pressed:true,jump_held:true,..Default::default()});
    assert!(!p.grounded);assert!((p.velocity-before).dot(up)>5.,"jump must push away from ceiling");assert_eq!(p.jumps,1);tested=true;break;
   }
   if !jump&&total*sign>=std::f32::consts::TAU{tested=true;break;}
  }
  assert!(tested,"public Player did not reach requested loop/jump");assert_eq!(p.respawns,0);
 }}
}
#[test]
fn shared_action_spin_launch_loops_and_slow_detachment_recovers(){
 use sonic_playground::simulation::{Player,Input,State};use glam::{vec2,vec3};
 for sign in [-1.,1.]{
 let mut p=Player::new();p.position=vec3(0.,0.,145.);p.yaw=sign*std::f32::consts::FRAC_PI_2;
 for tick in 0..50{p.tick(Input{action_held:true,bounce_pressed:tick==0,direction:vec2(sign,0.),..Default::default()});}
 assert_eq!(p.state,State::Charge);p.tick(Input::default());assert_eq!(p.state,State::Roll);
 let(mut previous,mut total)=(p.position.x.atan2(12.-p.position.y),0f32);
 for _ in 0..180{
  p.tick(Input::default());assert!(p.grounded,"shared-input launch lost support {total} {:?}",p.position);
  let a=p.position.x.atan2(12.-p.position.y);total+=(a-previous+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;previous=a;
  if total*sign>=std::f32::consts::TAU{break;}
 }
 assert!(total*sign>=std::f32::consts::TAU);
 }
 let mut slow=Player::new();slow.position=vec3(0.,0.,145.);slow.velocity=vec3(30.,0.,0.);slow.yaw=std::f32::consts::FRAC_PI_2;slow.state=State::Roll;
 let mut detached=false;let mut landed=false;
 for _ in 0..240{
  slow.tick(Input::default());assert!(slow.position.is_finite()&&slow.velocity.is_finite());
  if !slow.grounded{detached=true;assert!(matches!(slow.state,State::Fall|State::Jump));}
  else if detached{landed=true;break;}
 }
 assert!(detached&&landed);assert_eq!(slow.respawns,0);
}

#[test]
fn public_player_crosses_tube_entrance_in_both_directions(){
 use sonic_playground::simulation::{Player,Input};use glam::{vec2,vec3};
 for sign in [-1.,1.]{
  let mut p=Player::new();p.position=vec3(0.,0.,if sign>0.{95.}else{110.});p.yaw=if sign>0.{0.}else{std::f32::consts::PI};
  let mut crossed=false;
  for _ in 0..240{
   p.tick(Input{direction:vec2(0.,sign),..Default::default()});
   assert!(p.grounded&&p.position.y.abs()<0.05,"entrance failed sign={sign} {:?}",p.position);assert_eq!(p.respawns,0);
   if (sign>0.&&p.position.z>110.)||(sign<0.&&p.position.z<95.){crossed=true;break;}
  }
  assert!(crossed);
 }
}
#[test]
fn surface_camera_stick_matches_screen_right_and_preserves_flat_mapping(){
 use sonic_playground::controller::{camera_direction,surface_camera_back,surface_camera_direction};
 use glam::{Mat3,Mat4,Vec2,Vec3,vec2,vec3};
 for heading in [0f32,0.7,1.57,3.14,-1.57]{for camera in [0f32,0.6,1.4,3.]{
  let flat=Mat3::from_rotation_y(heading);
  for stick in [Vec2::X,Vec2::Y,vec2(-0.3,0.5)]{
   assert!(surface_camera_direction(stick,camera,flat,heading).distance(camera_direction(stick,camera))<0.00001);
  }
  for i in 0..32{
   let rot=Mat3::from_rotation_z(i as f32*std::f32::consts::TAU/32.)*flat;
   let back=surface_camera_back(camera,rot);let view=Mat4::look_at_rh(back*5.+rot.y_axis*2.,Vec3::ZERO,rot.y_axis);
   let input=surface_camera_direction(Vec2::X,camera,rot,heading);
   let f=vec2(heading.sin(),heading.cos());let r=vec2(heading.cos(),-heading.sin());
   let reconstructed=rot*vec3(input.dot(r),0.,input.dot(f));
   assert!(view.transform_vector3(reconstructed).x>0.99,"screen right reversed at {i}/{heading}/{camera}");
   assert!(input.is_finite()&&(input.length()-1.).abs()<0.00001);
  }
 }}
}
#[test]
fn contact_relative_stick_can_steer_a_complete_tunnel_loop(){
 use sonic_playground::{controller::surface_camera_direction,simulation::{Player,Input,State}};
 use glam::{vec2,vec3};
 for sign in [-1.,1.]{
  let mut p=Player::new();p.position=vec3(0.,0.,145.);p.velocity=vec3(90.*sign,0.,0.);p.yaw=sign*std::f32::consts::FRAC_PI_2;p.state=State::Roll;
  let(mut previous,mut total)=(0f32,0f32);
  for _ in 0..180{
   let direction=surface_camera_direction(vec2(-sign,0.),0.,p.model_rotation(),p.yaw);
   p.tick(Input{direction,..Default::default()});
   assert!(p.position.is_finite()&&p.velocity.is_finite());assert!(p.grounded,"lost support at {:?} angle{total}",p.position);
   let a=p.position.x.atan2(12.-p.position.y);total+=(a-previous+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;previous=a;
   if total*sign>=std::f32::consts::TAU{break;}
  }
  assert!(total*sign>=std::f32::consts::TAU,"incomplete controlled loop {total}");assert_eq!(p.respawns,0);
 }
}
