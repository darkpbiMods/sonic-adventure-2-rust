//! Host-owned flat world independent of demo terrain; ordinary movement, jump and bounce.
use glam::{Vec3,vec2,vec3};
use sonic_playground::simulation::{ActionEvent,Input,Player,State,World};
use std::cell::{Cell,RefCell};
#[derive(Default)]
struct HostWorld { events:RefCell<Vec<ActionEvent>>, snapshots:Cell<u64>, targets:RefCell<Vec<Option<Vec3>>>, terrain:Option<sa2_runtime::static_terrain_host::StaticTerrain> }
impl World for HostWorld {
 fn source_terrain(&self)->Option<&sa2_runtime::static_terrain_host::StaticTerrain>{self.terrain.as_ref()}
 fn target_count(&self)->usize{self.targets.borrow().len()}
 fn target(&self,i:usize)->Option<Vec3>{self.targets.borrow().get(i).copied().flatten()}
 fn target_attackable(&self,index:usize)->bool{index!=2}
 fn target_radius(&self,_:usize)->f32{0.42}
 fn ring_count(&self)->usize{0}
 fn ring(&self,_:usize)->Option<Vec3>{None}
 fn attack_path_clear(&self,_:Vec3,_:Vec3)->bool{true}
 fn spawn(&self)->Vec3{vec3(1000.,10.,1000.)}
 fn should_respawn(&self,p:Vec3)->bool{!p.is_finite()||p.y< -100.}
 fn move_player(&self,p:&mut Player,dt:f32){
  p.position+=p.velocity*dt;
  if p.position.y<=10.&&p.velocity.y<=0.{p.resolve_support(vec3(p.position.x,10.,p.position.z),Vec3::Y).unwrap();}
  else{p.grounded=false;}
 }
 fn event(&self,event:ActionEvent,_:Vec3){self.events.borrow_mut().push(event);}
 fn after_tick(&self,_:&Player){self.snapshots.set(self.snapshots.get()+1);}
}
fn main(){
 let mut world=HostWorld::default();
 if std::env::args().any(|a|a=="--source-terrain"){
  // Host metres -> source units (x10), translated to the independent spawn.
  world.terrain=Some(sa2_runtime::static_terrain_host::StaticTerrain::new(&[
   [[5000.,100.,5000.],[5000.,100.,15000.],[15000.,100.,5000.]],
   [[15000.,100.,5000.],[5000.,100.,15000.],[15000.,100.,15000.]],
  ]).unwrap());
 }
 let mut player=Player::in_world(&world);
 for _ in 0..120{player.tick_in(Input{direction:vec2(0.,1.),..Default::default()},&world);}
 assert!(player.position.z>world.spawn().z&&player.position.is_finite());
 assert!(player.collected.is_empty()&&player.defeated.is_empty());
 player.tick_in(Input{jump_pressed:true,jump_held:true,..Default::default()},&world);
 assert!(!player.grounded&&player.velocity.y>0.);
 let mut peak=player.position.y;
 for _ in 0..180{player.tick_in(Input::default(),&world);peak=peak.max(player.position.y);}
 assert!(peak>10.5&&player.grounded&&player.position.y==10.);
 assert_eq!(world.events.borrow().iter().filter(|&&e|e==ActionEvent::Jump).count(),1);
 assert_eq!(world.snapshots.get(),player.ticks);
 player.tick_in(Input{jump_pressed:true,..Default::default()},&world);
 player.tick_in(Input{bounce_pressed:true,..Default::default()},&world);
 for _ in 0..120{player.tick_in(Input::default(),&world);}
 assert_eq!(player.bounced,1);
 assert_eq!(world.events.borrow().iter().filter(|&&e|e==ActionEvent::BounceImpact).count(),1);
 assert!(player.grounded);
 let snapshots=world.snapshots.get();player.reset_in(&world);
 assert_eq!(player.position,world.spawn());assert_eq!(player.respawns,1);
 assert_eq!(world.snapshots.get(),snapshots,"reset does not emit a tick");
 // Create slots before reset, then retain their indices while marking removals.
 *world.targets.borrow_mut()=vec![Some(world.spawn()+vec3(0.,0.54,4.)),Some(world.spawn()+vec3(0.,0.54,8.)),Some(world.spawn()+vec3(0.,0.54,2.))];
 player.reset_in(&world);world.events.borrow_mut().clear();
 for _ in 0..125{player.tick_in(Input{charge:true,..Default::default()},&world);}
 let mut damage=[0u32;3];
 for _ in 0..120{
  player.tick_in(Input{direction:vec2(0.,1.),..Default::default()},&world);
  // Drain after tick: callbacks enqueue; they do not mutate/reenter the player.
  for event in world.events.borrow_mut().drain(..){
   if let ActionEvent::TargetHit(index)=event{
    damage[index]+=1;world.targets.borrow_mut()[index]=None;
   }
  }
 }
 assert_eq!(damage,[1,1,0]);assert_eq!(world.target_count(),3);
 assert!(world.target(2).is_some(),"immune target must survive");
 assert_eq!(player.defeated,vec![true,true,false]);
 assert!(player.position.is_finite()&&player.velocity.is_finite());
 // Exercise public fallback handoffs with stable but absent target slots.
 for target in world.targets.borrow_mut().iter_mut(){*target=None;}
 for cancel in [false,true]{
  player.reset_in(&world);world.events.borrow_mut().clear();
  player.position=world.spawn()+Vec3::Y*20.;player.grounded=false;player.state=State::Fall;
  player.tick_in(Input{jump_pressed:true,..Default::default()},&world);
  assert_eq!(player.state,State::Homing);assert_eq!(player.attack_target,None);
  if cancel{
   for _ in 0..3{player.tick_in(Input::default(),&world);}
   let before=player.position;
   player.tick_in(Input{bounce_pressed:true,..Default::default()},&world);
   assert_eq!(player.state,State::Fall);assert_eq!(player.source_air_action(),Some(10));assert_ne!(player.position,before);
   player.tick_in(Input::default(),&world);
   player.tick_in(Input{bounce_pressed:true,..Default::default()},&world);
   assert_eq!(player.state,State::BounceDown);
  }else{
   let mut expired=false;
   for _ in 0..20{
    let before=player.position;player.tick_in(Input::default(),&world);
    if player.state==State::Fall{assert_eq!(player.source_air_action(),Some(10));assert_ne!(player.position,before);expired=true;break;}
   }
   assert!(expired,"targetless dash must expire while airborne");
  }
  assert!(!world.events.borrow().iter().any(|e|matches!(e,ActionEvent::TargetHit(_))));
  assert!(player.position.is_finite()&&player.velocity.is_finite());
 }
 println!("HEADLESS_HOST_OK independent spawn/collision/events/reset/charged damage/fallback expiry/cancel-bounce; peak={peak} ticks={snapshots}");
}
