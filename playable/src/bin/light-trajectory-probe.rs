use sonic_playground::simulation::*;
use glam::{Vec3,vec3};
use std::cell::Cell;
struct Host{target:Cell<Vec3>}
impl World for Host{
 // Synthetic candidate producer keeps the distant arithmetic fixture offered.
 // Real hosts should supply their actual cached distance or use the default adapter.
 fn light_target_distance(&self,_:usize,_:Vec3)->Option<f32>{Some(100.)}

 fn target_count(&self)->usize{1}fn target(&self,_:usize)->Option<Vec3>{Some(self.target.get())}
 fn target_radius(&self,_:usize)->f32{0.42}
 fn ring_count(&self)->usize{0}fn ring(&self,_:usize)->Option<Vec3>{None}
 fn spawn(&self)->Vec3{vec3(0.,2.,0.)}fn should_respawn(&self,_:Vec3)->bool{false}
 fn attack_path_clear(&self,_:Vec3,_:Vec3)->bool{true}
 fn move_player(&self,p:&mut Player,dt:f32){p.position+=p.velocity*dt;}
}
fn main(){
 let mut rows=Vec::new();
 for distant in [false,true]{
  let w=Host{target:Cell::new(if distant{vec3(0.,2.54,1000.)}else{vec3(0.,2.5,4.)})};
  let mut p=Player::in_world(&w);p.state=State::LightAttack;p.grounded=false;p.attack_target=Some(0);p.light_steering_counter=1;
  for tick in 0..if distant{1}else{152}{
   if tick==1&&!distant{w.target.set(vec3(4.,2.5,0.));}
   let delta=(w.target.get()-p.position)*10.;let motion=p.velocity/6.;let counter=p.light_steering_counter.wrapping_add(1);
   p.tick_in(Input::default(),&w);
   rows.push(serde_json::json!({"distant":distant,"tick":tick,"delta":delta.to_array(),"motion":motion.to_array(),"counter":counter,"velocity":p.velocity.to_array(),"active":p.state==State::LightAttack,"hits":p.last_attack_hits}));
   if p.state!=State::LightAttack{break;}
  }
 }
 std::fs::write(std::env::args().nth(1).unwrap(),serde_json::to_vec(&rows).unwrap()).unwrap();
}
