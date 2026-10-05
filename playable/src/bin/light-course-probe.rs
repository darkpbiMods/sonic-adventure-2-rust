use sonic_playground::simulation::*;
fn main(){
 let args:Vec<_>=std::env::args().collect();
 let magnitude:f32=args.get(2).map(|v|v.parse().unwrap()).unwrap_or(0.);
 let frames:usize=args.get(3).map(|v|v.parse().unwrap()).unwrap_or(161);
 let mut p=Player::new();for _ in 0..125{p.tick(Input{charge:true,..Default::default()});}
 let mut rows=Vec::new();
 for tick in 0..frames{
  p.tick(Input{direction:glam::vec2(magnitude,0.),..Default::default()});
  let distances:Vec<_>=targets().iter().map(|t|p.attack_center().distance(*t)).collect();
  rows.push(serde_json::json!({"tick":tick,"position":p.position.to_array(),"velocity":p.velocity.to_array(),"state":p.state.label(),"target":p.attack_target,"counter":p.light_steering_counter,"hits":p.last_attack_hits,"distances":distances}));
 }
 std::fs::write(&args[1],serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
}
