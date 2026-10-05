use macroquad::prelude::*;
use sonic_playground::{simulation::*,assets::Assets};
fn main(){
 let args:Vec<_>=std::env::args().collect();let dir=std::path::Path::new(args.get(1).map(String::as_str).unwrap_or("assets"));
 if args.iter().any(|a|a=="--controller"){let p=sonic_playground::controller::Controller::default().poll();println!("XInput connected: {} index: {}",p.connected,p.index);return;}
 let a=Assets::read(dir).expect("asset validation");
 let bounce=Assets::read_bounce(&dir.join("bounce")).expect("bounce model validation");
 assert!(bounce.manifest.vertex_count>0 && !bounce.manifest.groups.is_empty());
 assert_eq!(bounce.pose_frame("bounce",0.),bounce.pose_frame("bounce",7.));
 assert_ne!(bounce.pose_frame("bounce",0.),bounce.pose_frame("bounce",3.));
 for name in ["walk","run","spin"]{let c=&a.clips[name];let n=a.manifest.vertex_count;let movement:f32=c.data[..n].iter().zip(&c.data[n*3..n*4]).map(|(x,y)|(x[0]-y[0]).abs()+(x[1]-y[1]).abs()+(x[2]-y[2]).abs()).sum();assert!(movement>1.,"frozen clip {name}");}
 println!("PASS playable package assets");
 let input=Input{direction:vec2(0.,1.),..Default::default()};
 // Original force trajectory: full stick reaches 8.300407 host units/s at60ticks.
 let mut p=Player::new();for _ in 0..60{p.tick(input);}assert_eq!(p.velocity.z.to_bits(),8.300407409667969_f32.to_bits());assert!(p.position.z> -4. && p.grounded);
 for _ in 0..300{p.tick(input);if p.position.y>3.5{break;}}assert!(p.position.y>3.5 && p.grounded,"ramp {:?}",p.position);assert!(p.collected_count()>0);
 println!("PASS acceleration, rings, slope and ramp/platform seam");
 let mut p=Player::new();p.position=vec3(10.,0.,0.);for _ in 0..100{p.tick(Input{direction:vec2(1.,0.),..Default::default()});}assert!(p.position.x<=13.-RADIUS+0.001 && p.wall_hits>0);assert_eq!(p.position.y,0.);
 println!("PASS wall stops player");
 let mut p=Player::new();p.position=vec3(0.,4.,40.);for _ in 0..180{p.tick(input);if !p.grounded{break;}}assert!(!p.grounded && p.position.z>48.,"ledge {:?}",p.position);for _ in 0..100{p.tick(Input::default());}assert!(p.grounded && p.position.y==0. && p.landings>0);
 println!("PASS walk off ledge and land");
 let mut p=Player::new();p.tick(Input{jump_pressed:true,jump_held:true,..Default::default()});for _ in 0..90{p.tick(Input::default());}assert!((p.max_height-1.42611).abs()<0.001 && p.grounded && p.landings==1 && p.jumps==1);
 println!("PASS jump and landing");
 let mut p=Player::new();p.position=vec3(24.,7.,30.);p.grounded=false;for _ in 0..90{p.tick(Input::default());}assert!((p.position.y-2.).abs()<0.001 && p.grounded);
 println!("PASS elevated platform landing");
 let mut p=Player::new();for _ in 0..60{p.tick(Input{charge:true,..Default::default()});}assert_eq!(p.state,State::Charge);p.tick(Input::default());assert!(p.speed()>40. && p.state==State::Roll);p.reset();assert_eq!(p.position,vec3(0.,0.,-6.));assert_eq!(p.collected_count(),0);
 println!("PASS charge/release and reset");
 let mut states=Vec::new();for fps in [30,60,144]{let mut p=Player::new();let mut clock=Clock::default();for _ in 0..fps*2{clock.advance(&mut p,input,1./fps as f64);}states.push((p.position,p.velocity,p.ticks));}assert_eq!(states[0],states[1]);assert_eq!(states[1],states[2]);
 let mut p=Player::new();let mut clock=Clock::default();clock.advance(&mut p,Input{jump_pressed:true,..Default::default()},1./144.);clock.advance(&mut p,Input::default(),1./60.);assert_eq!(p.jumps,1);
 println!("PASS 30/60/144 Hz equivalence and input-edge retention");
 let mut p=Player::new();for _ in 0..90{p.tick(Input{direction:vec2(0.,0.5),..Default::default()});}assert_eq!(p.velocity.z.to_bits(),1.2347997426986694_f32.to_bits());
 println!("PASS analog half-stick speed");
 let mut p=Player::new();p.position=vec3(50.,-60.,0.);p.grounded=false;p.tick(Input::default());assert_eq!(p.respawns,1);
 println!("PASS out-of-world recovery");
 let mut p=Player::new();p.tick(Input{bounce_pressed:true,..Default::default()});assert_eq!(p.bounced,0);assert!(p.grounded);
 p.position.y=2.;p.grounded=false;p.tick(Input{bounce_pressed:true,..Default::default()});assert_eq!(p.state,State::BounceDown);assert!(p.velocity.y< -20.);
 for _ in 0..20 {p.tick(Input::default());if p.bounced==1{break;}}
 assert_eq!(p.bounced,1);assert!(p.velocity.y>0. && !p.grounded && p.position.y>=0.);
 for _ in 0..8{p.tick(Input::default());}p.tick(Input{bounce_pressed:true,..Default::default()});
 for _ in 0..25{p.tick(Input::default());if p.bounced==2{break;}}assert_eq!(p.bounced,2);
 for _ in 0..140{p.tick(Input::default());}assert_eq!(p.bounced,2);assert!(p.grounded);
 let mut p=Player::new();p.position=vec3(24.,5.,30.);p.grounded=false;p.tick(Input{bounce_pressed:true,..Default::default()});
 for _ in 0..25{p.tick(Input::default());if p.bounced==1{break;}}assert_eq!(p.bounced,1);assert!(p.position.y>=2. && p.velocity.y>0.);
 let mut p=Player::new();p.position.y=3.;p.grounded=false;let mut clock=Clock::default();clock.advance(&mut p,Input{bounce_pressed:true,..Default::default()},1./144.);clock.advance(&mut p,Input::default(),1./60.);assert_eq!(p.state,State::BounceDown);
 println!("PASS bounce scenarios");
 let mut p=Player::new();p.collected.fill(true);for _ in 0..125{p.tick(Input{charge:true,..Default::default()});}
 assert!(p.charge>=FULL_CHARGE-0.001);p.tick(Input::default());assert_eq!(p.state,State::LightAttack);
 // Source contact retains world impact velocity; pursuit reads that raw motion
 // on its next update. The old host solver's exact hit count is not an SA2 oracle.
 let mut retained_floor_impact=false;
 for _ in 0..160{
  p.tick(Input::default());
  retained_floor_impact|=p.grounded&&p.position.y.abs()<0.001&&p.velocity.y< -1.;
  assert!(p.position.is_finite()&&p.velocity.is_finite());
 }
 assert!(retained_floor_impact,"charged contact must preserve source world impact velocity");
 assert!(p.last_attack_hits>0);assert_eq!(p.state,State::LightAttack);
 let mut p=Player::new();for _ in 0..125{p.tick(Input{charge:true,..Default::default()});}
 for _ in 0..27{p.tick(Input{direction:vec2(1.,0.),..Default::default()});}
 assert_eq!(p.last_attack_hits,targets().len()as u32);assert!(p.defeated.iter().all(|x|*x));assert!(p.attack_target.is_none() && p.state==State::LightAttack && p.position.is_finite());
 // Failed selection on the next update performs source forces and exit.
 p.tick(Input::default());assert_eq!(p.state,State::Fall);assert!(p.velocity.is_finite()&&p.position.is_finite());
 assert!(clear_attack_path(vec3(0.,0.,-6.),vec3(-6.,0.15,0.)));assert!(!clear_attack_path(vec3(12.,1.,0.),vec3(15.,1.,0.)));
 let mut p=Player::new();p.position=vec3(-35.,0.,-10.);for _ in 0..125{p.tick(Input{charge:true,..Default::default()});}p.tick(Input::default());assert_eq!(p.state,State::Idle);assert_eq!(p.charge,0.);assert_eq!(p.speed(),0.);assert!(p.attack_target.is_none());assert_eq!(p.last_attack_hits,0);
 println!("PASS Light Speed Attack scenarios");
 assert!(a.manifest.groups.iter().all(|g|g.indices.len()<sonic_playground::assets::DRAW_INDICES&&g.vertices.len()<sonic_playground::assets::DRAW_VERTICES));
 println!("PASS complete mesh draw capacity: {} triangles",a.manifest.groups.iter().map(|g|g.indices.len()/3).sum::<usize>());
 println!("PASS playable simulation scenarios");
}
