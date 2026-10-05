#![cfg_attr(target_os="windows", windows_subsystem="windows")]
use macroquad::prelude::*;
use sonic_playground::{assets::Renderer,simulation::*};
use sonic_playground::controller::{self as xbox,Controller,Snapshot};
use std::path::PathBuf;
use serde_json::json;
use std::io::Write;


struct PresentationWorld {fire:std::cell::RefCell<sonic_playground::fire_effect::FireEffects>,platform:Option<sonic_playground::platform_course::PlatformCourse>,feedback_requests:std::cell::Cell<[u32;2]>,feedback:std::cell::RefCell<sonic_playground::feedback::Feedback>,motor_commands:std::cell::RefCell<Vec<sonic_playground::feedback::MotorCommand>>,feedback_respawns:std::cell::Cell<u32>,events:std::cell::RefCell<Vec<ActionEvent>>,auras:std::cell::RefCell<sonic_playground::aura::Auras>,dashes:std::cell::RefCell<sonic_playground::dash_effect::DashEffects>,charges:std::cell::RefCell<sonic_playground::charge_effect::ChargeEffects>}
impl World for PresentationWorld {
 fn source_terrain(&self)->Option<&sa2_runtime::static_terrain_host::StaticTerrain>{Some(if let Some(p)=&self.platform{p.terrain()}else{demo_source_terrain()})}
 fn prepare_platform_memory(&self,m:&mut sa2_runtime::original_platform::PlatformMemory)->sa2_runtime::disc::Result<()>{if let Some(p)=&self.platform{p.prepare(m)}else{Ok(())}}
 fn platform_callback(&self,a:u32,_:u32,_:u32,s:u32,_:&mut sa2_runtime::original_platform::PlatformMemory)->sa2_runtime::disc::Result<()>{self.platform.as_ref().ok_or("platform preview disabled")?.callback(a,s)}
 fn homing_contact_response(&self,_:usize)->HomingContactResponse{if self.platform.is_some(){HomingContactResponse::SourceCategory2}else{HomingContactResponse::DemoUpward}}
 fn vibration(&self,r:sa2_runtime::original_vibration::Request,_:Vec3){let mut counts=self.feedback_requests.get();if r==sa2_runtime::original_vibration::request(7,15,0){counts[0]+=1;}else if r==sa2_runtime::original_vibration::request(6,15,0){counts[1]+=1;}self.feedback_requests.set(counts);self.feedback.borrow_mut().request(r);}
 fn after_tick(&self,p:&Player){
  if self.feedback_respawns.replace(p.respawns)!=p.respawns{if let Some(c)=self.feedback.borrow_mut().clear(){self.motor_commands.borrow_mut().push(c);}}
  self.motor_commands.borrow_mut().extend(self.feedback.borrow_mut().tick());
  let center=p.effect_center();
  self.auras.borrow_mut().tick(p,center);
  self.dashes.borrow_mut().tick(p,center);
  // Both charge variants orbit the entity/collider midpoint. The ready
  // variant adds its age-dependent rise inside ChargeEffects.
  self.charges.borrow_mut().tick(p,(p.position+center)*0.5);
  self.fire.borrow_mut().tick_player(p).expect("fire particle tick");
  if let Some(platform)=&self.platform{platform.after_tick();}
 }
 fn event(&self,e:ActionEvent,_:Vec3){self.events.borrow_mut().push(e);}
 fn target_count(&self)->usize{DemoWorld.target_count()}
 fn target(&self,i:usize)->Option<Vec3>{DemoWorld.target(i)}
 fn target_radius(&self,i:usize)->f32{DemoWorld.target_radius(i)}
 fn ring_count(&self)->usize{DemoWorld.ring_count()}
 fn ring(&self,i:usize)->Option<Vec3>{DemoWorld.ring(i)}
 fn attack_path_clear(&self,a:Vec3,b:Vec3)->bool{DemoWorld.attack_path_clear(a,b)}
 fn move_player(&self,p:&mut Player,dt:f32){DemoWorld.move_player(p,dt)}
 fn ground_normal(&self,p:Vec3)->Vec3{DemoWorld.ground_normal(p)}
 fn spawn(&self)->Vec3{self.platform.as_ref().map_or_else(||DemoWorld.spawn(),|p|p.spawn())}
 fn should_respawn(&self,p:Vec3)->bool{DemoWorld.should_respawn(p)}
}

fn configuration()->Conf{Conf{window_title:"Sonic / Rust Playground v0.44.1 - Bowl / Tunnel / Flame Ring / Light Dash".into(),window_width:1280,window_height:800,high_dpi:true,sample_count:4,window_resizable:true,..Default::default()}}
fn color(hex:u32)->Color{Color::from_hex(hex)}
fn solid(min:Vec3,max:Vec3,c:Color){draw_cube((min+max)*0.5,max-min,None,c);}
fn ground_mesh()->Mesh{
 let mut vertices=Vec::new();let mut indices=Vec::new();
 for x in -9..9{for z in -5..20{
  let c=if (x+z)%2==0{color(0x273947)}else{color(0x2c404e)};
  let i=vertices.len()as u16;let(x,z)=(x as f32*5.,z as f32*5.);
  for p in [vec3(x,0.,z),vec3(x+5.,0.,z),vec3(x+5.,0.,z+5.),vec3(x,0.,z+5.)]{vertices.push(Vertex::new(p.x,p.y,p.z,0.,0.,c));}
  indices.extend_from_slice(&[i,i+2,i+1,i,i+3,i+2]);
 }}Mesh{vertices,indices,texture:None}
}
fn ramp_mesh()->Mesh{
 let positions=[vec3(-8.,0.,15.),vec3(8.,0.,15.),vec3(8.,4.,35.),vec3(-8.,4.,35.),vec3(-8.,0.,35.),vec3(8.,0.,35.)];
 Mesh{vertices:positions.into_iter().map(|p|Vertex::new(p.x,p.y,p.z,0.,0.,color(0x487b85))).collect(),indices:vec![0,2,1,0,3,2,0,4,3,1,2,5,3,4,5,3,5,2],texture:None}
}
fn slope_mesh()->Mesh{
 let arena=sonic_playground::slope_arena::SlopeArena::shared();let mut vertices=Vec::new();let mut indices=Vec::new();
 for (i,t) in arena.bowl.iter().chain(&arena.tunnel).enumerate(){
  let n=(t[1]-t[0]).cross(t[2]-t[0]).normalize();let light=0.55+0.45*n.dot(vec3(-0.3,0.8,-0.2).normalize()).max(0.);
  let base=if i<arena.bowl.len(){color(0x557e91)}else if (i/2)%2==0{color(0x648b83)}else{color(0x46716b)};
  for p in t{indices.push(vertices.len()as u16);vertices.push(Vertex::new(p.x,p.y,p.z,0.,0.,Color::new(base.r*light,base.g*light,base.b*light,1.)));}
 }
 Mesh{vertices,indices,texture:None}
}
fn torus(center:Vec3,time:f32)->Mesh{
 let mut vertices=Vec::new();let mut indices=Vec::new();let q=Quat::from_rotation_y(time);
 for a in 0..20{let angle=a as f32/20.*std::f32::consts::TAU;for b in 0..6{
  let phi=b as f32/6.*std::f32::consts::TAU;let radial=vec3(angle.cos(),angle.sin(),0.);
  let n=q*(radial*phi.cos()+Vec3::Z*phi.sin());let p=center+q*(radial*(0.25+0.065*phi.cos())+Vec3::Z*0.065*phi.sin());
  let brightness=0.62+0.38*n.dot(vec3(-0.4,0.8,-0.2).normalize()).max(0.);
  vertices.push(Vertex::new(p.x,p.y,p.z,0.,0.,Color::new(brightness,brightness*0.76,0.11,1.)));
  let i=(a*6+b)as u16;let j=(a*6+(b+1)%6)as u16;let k=(((a+1)%20)*6+b)as u16;let l=(((a+1)%20)*6+(b+1)%6)as u16;indices.extend_from_slice(&[i,j,k,j,l,k]);
 }}Mesh{vertices,indices,texture:None}
}
fn camera_position(target:Vec3,desired:Vec3)->Vec3{
 // Keep the camera on the near side of solid obstacles and terrain.
 let mut result=desired;
 for i in 1..80{let t=i as f32/80.;let p=target.lerp(desired,t);let blocked=blocks().iter().any(|b|p.x>b.min.x-0.15&&p.x<b.max.x+0.15&&p.z>b.min.z-0.15&&p.z<b.max.z+0.15&&p.y>b.min.y-0.15&&p.y<b.max.y+0.15);
  let below=support(p,100.).is_some_and(|h|p.y<h+0.2);
  if blocked||below{result=target.lerp(desired,((i-1)as f32/80.).max(0.08));break;}
 }sonic_playground::slope_arena::SlopeArena::shared().clip_camera(target,result)
}
fn draw_world(p:&Player,ground:&Mesh,ramp:&Mesh,slopes:&Mesh,time:f32){
 draw_mesh(ground);draw_mesh(ramp);draw_mesh(slopes);
 for (i,b)in blocks().into_iter().enumerate(){solid(b.min,b.max,if i==0{color(0x427b87)}else if i<3{color(0x61738a)}else{color(0x76638b)});draw_cube_wires((b.min+b.max)*0.5,b.max-b.min,color(0x91b5c2));}
 // Two cyan lines lead from spawn up the ramp to its crest.
 for x in [-3.,3.] {for z in -10..48{let h=support(vec3(x,0.,z as f32),100.).unwrap_or(0.);draw_line_3d(vec3(x,h+0.025,z as f32),vec3(x,h+0.225_f32.min(if (15..35).contains(&z){0.225}else{0.025}),z as f32+1.),color(0x45d5da));}}
 draw_cube_wires(vec3(0.,0.03,-6.),vec3(4.,0.04,4.),color(0x5ee5bf));
 for (i,r) in rings().iter().enumerate(){if !p.collected[i]{draw_mesh(&torus(*r,time*1.6+i as f32*0.2));}}
 for(i,t)in targets().iter().enumerate(){
  if p.defeated[i]{draw_cube_wires(*t,Vec3::splat(0.5),color(0x51666e));continue;}
  draw_sphere(*t,0.42,None,if p.attack_target==Some(i){color(0xffe281)}else{color(0xe17d62)});
  draw_cube_wires(*t,Vec3::splat(0.9),if p.attack_target==Some(i){YELLOW}else{color(0xffc99a)});
  draw_line_3d(vec3(t.x,0.02,t.z),*t,color(0x9fbaad));
 }
 // Unobstructive test-course markers.
 for (x,z)in [(-9.,15.),(9.,35.),(-19.,76.),(19.,76.)]{solid(vec3(x,0.,z),vec3(x+0.15,2.,z+0.15),color(0x42d7d0));}
 if let Some(h)=support(p.position,100.){if p.position.y>=h-0.1{draw_cylinder(vec3(p.position.x,h+0.012,p.position.z),0.31,0.31,0.006,None,Color::new(0.03,0.06,0.08,0.45));}}
}
fn text(s:&str,x:f32,y:f32,size:f32,c:Color){draw_text(s,x,y,size,c);}
fn hud(p:&Player,paused:bool,help:bool,fps:i32,pad:Snapshot,selection:usize){
 let w=1280.;let h=w*screen_height()/screen_width();
 draw_rectangle(22.,20.,330.,88.,Color::new(0.035,0.075,0.105,0.93));
 draw_rectangle(22.,20.,4.,88.,color(0x55e3d1));
 text("SONIC / RUST PLAYGROUND v0.44",40.,50.,22.,WHITE);
 text("SA2 assets  /  experimental movement",40.,76.,17.,color(0xa1bac8));
 text(&if pad.connected{format!("XBOX CONNECTED / PLAYER {}",pad.index+1)}else{"CONNECT XBOX CONTROLLER / keyboard ready".into()},40.,96.,14.,if pad.connected{color(0x5ee5bf)}else{color(0xffd773)});
 draw_rectangle(w-320.,20.,298.,108.,Color::new(0.035,0.075,0.105,0.93));
 text(&format!("TARGETS {} / {}  |  RINGS {}",p.defeated.iter().filter(|&&x|x).count(),p.defeated.len(),p.collected_count()),w-302.,47.,20.,color(0xffd773));
 text(&format!("{:05.1} m/s   {}",p.speed(),p.state.label()),w-302.,73.,20.,WHITE);
 text(&format!("Chain: {}  |  Bounces: {}",p.last_attack_hits,p.bounced),w-302.,96.,17.,WHITE);
 text(&format!("{} FPS   |   60 Hz simulation",fps),w-302.,118.,15.,color(0xa1bac8));
 if p.state==State::Charge||p.charge>=FULL_CHARGE{
  let ready=p.charge>=FULL_CHARGE-0.001;let c=if ready{color(0xffd773)}else{color(0x55e3d1)};
  draw_rectangle(w*0.5-100.,h-172.,200.,8.,color(0x244353));draw_rectangle(w*0.5-100.,h-172.,200.*p.charge/FULL_CHARGE,8.,c);
  text(if ready{"LIGHT SPEED ATTACK READY - release B / X"}else{"Hold B / X to charge Light Speed Attack"},w*0.5-184.,h-182.,20.,c);
 }
 if help{
  draw_rectangle(22.,h-147.,615.,125.,Color::new(0.035,0.075,0.105,0.92));
  text("LS Move   A Jump / Homing   B / X Somersault / Ring Dash",38.,h-120.,18.,WHITE);
  text("Hold B / X Charge   Air: Bounce   Rolling: Uncurl",38.,h-94.,18.,WHITE);
  text("RIGHT STICK   Camera    LT / RT   Zoom    RS   Center",38.,h-68.,18.,color(0xb2c9d4));
  text("MENU   Pause / restart / quit    VIEW   Toggle help",38.,h-42.,18.,color(0xb2c9d4));
 }else{text("VIEW / H   Controls     MENU / ESC   Pause",25.,h-25.,18.,WHITE);}
 text("MENU: bowl / tunnel / ring dash practice starts",w-490.,h-25.,18.,color(0xc0d7e2));
 if p.collected_count()==p.collected.len(){text("COURSE CLEARED  /  R to go again",w*0.5-205.,145.,28.,color(0xffd773));}
 if paused{
  draw_rectangle(0.,0.,w,h,Color::new(0.015,0.025,0.045,0.72));
  text("PAUSED",w*0.5-90.,h*0.5-120.,48.,WHITE);
  for(i,label)in ["Resume","Restart course","Bowl practice","Tunnel practice","Ring dash practice","Quit demo"].iter().enumerate(){let y=h*0.5-78.+i as f32*35.;if i==selection{draw_rectangle(w*0.5-155.,y-24.,310.,32.,color(0x246d70));}text(label,w*0.5-128.,y,24.,WHITE);}
  text("D-PAD / STICK   Select     A   Confirm     B   Back",w*0.5-237.,h*0.5+145.,19.,color(0xc0d7e2));
 }
}
fn scripted(frame:u32,p:&mut Player)->xbox::RawGamepad{
 let i=match frame{
  0..=29=>Input::default(),
  30..=239=>Input{direction:vec2(0.,1.),..Default::default()},
  240=>Input{direction:vec2(0.,1.),jump_pressed:true,jump_held:true,..Default::default()},
  241..=249=>Input{direction:vec2(0.,1.),jump_held:true,..Default::default()},
  250..=339=>Input{direction:vec2(0.,1.),..Default::default()},
  340=>{p.reset();Input::default()},
  341..=410=>Input{charge:true,..Default::default()},
  411..=479=>Input::default(),
  480=>{p.reset();p.position=vec3(10.,0.,0.);Input::default()},
  481..=549=>Input{direction:vec2(-1.,0.),..Default::default()},
  550=>{p.reset();p.position=vec3(24.,7.,30.);p.grounded=false;Input::default()},
  621=>{p.reset();Input::default()},
  622..=746=>Input{charge:true,..Default::default()},
  747..=825|1272..=1350=>Input{direction:if p.state==State::LightAttack||frame==747||frame==1272{vec2(1.,0.)}else{Vec2::ZERO},..Default::default()},
  826=>{p.reset();Input::default()},
  827=>Input{jump_pressed:true,jump_held:true,..Default::default()},
  828..=839=>Input{jump_held:true,..Default::default()},
  840|871=>Input{bounce_pressed:true,..Default::default()},
  951=>{p.reset();p.position=vec3(-6.,2.,-4.);p.grounded=false;Input::default()},
  1031=>{p.reset();Input::default()},
  1032..=1271=>Input{charge:true,..Default::default()},
  952|980=>Input{jump_pressed:true,..Default::default()},
  1351=>{p.reset();p.position=vec3(30.,10.,-10.);p.velocity=vec3(3.,12.,2.);p.grounded=false;Input::default()},
  1352=>Input{jump_pressed:true,..Default::default()},
  1370=>{p.reset();p.position=vec3(30.,10.,-10.);p.velocity=vec3(3.,12.,2.);p.grounded=false;Input::default()},
  1371=>Input{jump_pressed:true,..Default::default()},
  1374|1376=>Input{bounce_pressed:true,..Default::default()},
  1391=>{p.reset();p.position=vec3(30.,0.,-10.);Input::default()},
  1392..=1511=>Input{charge:true,..Default::default()},
  1512..=1571=>Input{charge:true,direction:vec2(0.,1.),..Default::default()},
  1591=>{p.reset();p.position=vec3(0.,4.,47.5);p.velocity.z=6.;Input::default()},
  1592..=1620=>Input{direction:vec2(0.,1.),..Default::default()},
  1691=>{p.reset();p.position=vec3(0.,2.,25.);p.velocity.z=6.;Input::default()},
  1692=>Input{jump_pressed:true,jump_held:true,..Default::default()},
  1693..=1700=>Input{jump_held:true,..Default::default()},
  1791=>{p.reset();p.position=vec3(30.,0.,-10.);p.velocity.z=18.;p.state=State::Run;Input::default()},
  1792..=1850=>Input{direction:vec2(0.,-1.),..Default::default()},
  1891=>{p.reset();p.position=vec3(30.,0.,-10.);Input::default()},
  1892=>Input{jump_pressed:true,..Default::default()},
  1894=>{p.position.y=0.001;p.velocity=vec3(0.,-20.,0.);p.state=State::Fall;Input::default()},
  1991=>{p.reset();p.position=vec3(30.,0.,-10.);Input::default()},
  1992=>Input{jump_pressed:true,..Default::default()},
  1994=>{p.position.y=0.001;p.velocity=vec3(0.,-12.,6.);p.state=State::Fall;Input{direction:vec2(0.,-1.),..Default::default()}},
  2191=>{p.reset();p.position=vec3(30.,0.,-10.);p.velocity.z=18.;p.state=State::Run;Input::default()},
  2391=>{p.reset();p.position=vec3(0.,0.,-6.);for _ in 0..1000{p.tick(Input::default());}Input::default()},
  2591=>{p.reset();p.position=vec3(0.,0.,-6.);for _ in 0..13550{p.tick(Input::default());}Input::default()},
  2641..=2660=>Input{direction:vec2(0.,1.),..Default::default()},
  2431..=2450=>Input{direction:vec2(0.,1.),..Default::default()},
  2192..=2230=>Input{direction:vec2(1.,0.),..Default::default()},
  _=>Input::default(),
 };
 xbox::RawGamepad{buttons:(if i.jump_pressed||i.jump_held{xbox::A}else{0})|(if i.charge{xbox::X}else{0})|(if i.bounce_pressed{xbox::B}else{0}),lx:(i.direction.x*32767.)as i16,ly:(i.direction.y*32767.)as i16,..Default::default()}
}
#[macroquad::main(configuration)]
async fn main(){
 macroquad::window::gl_set_drawcall_buffer_capacity(sonic_playground::assets::DRAW_VERTICES,sonic_playground::assets::DRAW_INDICES);
 let exe=std::env::current_exe().unwrap_or_default();let dir=exe.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
 let args:Vec<_>=std::env::args().collect();let mut input_trace=args.windows(2).find(|a|a[0]=="--input-trace").map(|a|std::fs::OpenOptions::new().write(true).create_new(true).open(&a[1]).expect("input trace requires a new writable file"));
 let mut input_trace_rows=0_u32;
 let asset_dir=args.windows(2).find(|a|a[0]=="--assets").map(|a|PathBuf::from(&a[1])).unwrap_or(dir.join("assets"));
 let smoke=args.windows(2).find(|a|a[0]=="--smoke").map(|a|PathBuf::from(&a[1]));
 let new_features=args.iter().any(|a|a=="--new-features");assert!(!new_features||smoke.is_some());let mut feature_captures=Vec::new();
 let screenshots=!args.iter().any(|a|a=="--no-screenshots");
 let effect_reference=args.iter().any(|a|a=="--effect-reference");
 assert!(!effect_reference||smoke.is_some(),"--effect-reference requires --smoke");
 let mut effect_frames=Vec::new();
 if let Some(d)=&smoke{std::fs::create_dir_all(d).unwrap();let _=std::fs::remove_file(d.join("smoke.json"));}
 let mut renderer=match Renderer::new(&asset_dir){Ok(r)=>r,Err(e)=>{
  let _=std::fs::write(dir.join("launch-error.txt"),&e);
  if smoke.is_some(){panic!("{e}");}
  loop{clear_background(color(0x15212e));text("Could not load Sonic assets",45.,90.,32.,WHITE);text(&e,45.,140.,18.,WHITE);text("Keep the assets folder next to the executable. ESC to close.",45.,180.,20.,WHITE);if is_key_pressed(KeyCode::Escape){return;}next_frame().await;}
 }};
 let preview=args.iter().any(|a|a=="--audio-preview");
 let mut action_audio=if preview{
  Some(sonic_playground::action_audio::ActionAudio::load(&asset_dir).await.expect("action preview audio"))
 }else{None};
 let mut charge_audio=if preview{Some(sonic_playground::charge_audio::ChargeAudio::load(&asset_dir).expect("charge audio"))}else{None};
 let mut ring_audio=if preview{Some(sonic_playground::charge_audio::ChargeAudio::load_ring(&asset_dir).expect("ring audio"))}else{None};
 let mut presentation=PresentationWorld{fire:Default::default(),platform:if args.iter().any(|a|a=="--platform-preview"){Some(sonic_playground::platform_course::PlatformCourse::new().expect("platform preview geometry"))}else{None},feedback_requests:Default::default(),feedback:Default::default(),motor_commands:Default::default(),feedback_respawns:Default::default(),events:Default::default(),auras:Default::default(),dashes:Default::default(),charges:Default::default()};
 let source_root=args.windows(2).find(|a|a[0]=="--source-render").map(|a|PathBuf::from(&a[1])).or_else(||{let p=dir.join("source-render");p.is_dir().then_some(p)});
 let source_renderer=source_root.map(|root|sonic_playground::source_renderer::SourceRenderer::new(&root,&asset_dir).expect("source renderer initialization"));
 if let (Some(source),Some(out))=(&source_renderer,&smoke){
  let mut somersault=Player::new();
  somersault.tick(Input{bounce_pressed:true,..Default::default()});
  let mut somersault_frames=0;
  for _ in 0..100{
   if somersault.state==State::Somersault{source.prepare_pose(&somersault).expect("somersault original geometry");somersault_frames+=1;}
   somersault.tick(Input::default());
  }
  assert!(somersault_frames>20);assert_ne!(somersault.state,State::Somersault);
  std::fs::write(out.join("somersault-pose.json"),serde_json::to_vec_pretty(&json!({"passed":true,"frames":somersault_frames,"scope":"Public B input and original pose assembly; controller feel remains a playtest"})).unwrap()).unwrap();
  let mut sample=Player::new();
  let start=std::time::Instant::now();assert!(source.prepare_pose(&sample).unwrap());let cold_us=start.elapsed().as_micros();
  let start=std::time::Instant::now();for _ in 0..100{assert!(!source.prepare_pose(&sample).unwrap());}let hits_us=start.elapsed().as_micros();
  sample.position.x+=5.;sample.yaw+=0.5;assert!(!source.prepare_pose(&sample).unwrap(),"world transform must not invalidate local pose");
  sample.animation_frame=0.25;assert!(source.prepare_pose(&sample).unwrap(),"frame change must invalidate");
  sample.state=State::Run;assert!(source.prepare_pose(&sample).unwrap(),"action change must invalidate");
  std::fs::write(out.join("pose-cache.json"),serde_json::to_vec_pretty(&json!({"passed":true,"cold_prepare_us":cold_us,"hundred_hits_us":hits_us,"scope":"CPU local-pose preparation only; excludes world transform, draw and GPU"})).unwrap()).unwrap();
 }
 let mut platform_relative=None;let mut platform_drift=0f32;let mut platform_air=false;let mut platform_return=false;let mut platform_frames=Vec::new();
 let mut cache_checks=[0u32;3];
 let mut aura_renderer=sonic_playground::aura::renderer::AuraRenderer::new(&asset_dir).expect("original aura assets");
 let mut dash_renderer=sonic_playground::dash_effect::renderer::DashRenderer::new(&asset_dir).expect("original dash assets");
 let mut fire_renderer=sonic_playground::fire_effect::renderer::FireRenderer::new(&asset_dir).expect("original fire assets");
 let mut charge_renderer=sonic_playground::charge_effect::renderer::ChargeRenderer::new(&asset_dir).expect("original charge assets");
 let (ground,ramp,slopes)=(ground_mesh(),ramp_mesh(),slope_mesh());let mut player=Player::in_world(&presentation);let mut clock=Clock::default();
 let(mut paused,mut help,mut fullscreen)=(false,true,false);let(mut yaw,mut distance,mut pitch)=(0_f32,5.2_f32,0.36_f32);
 let mut target=player.position+Vec3::Y*0.65;let mut previous_mouse=mouse_position();let mut frame=0_u32;let mut telemetry=Vec::new();
 let mut controller=Controller::default();let mut script_controller=Controller::default();let mut menu_selection=0_usize;let mut previous_menu_axis=0_f32;
 let mut frame_intervals_ms=Vec::<f64>::new();
 let mut interval_rows=Vec::new();let mut steady_intervals=Vec::<f64>::new();
 let mut slow_frames=Vec::new();let mut previous_cpu=[0_f64;3];let mut previous_source_draw_ms=0_f64;let mut previous_await_ms=0_f64;
 let mut motor_output=xbox::MotorOutput::default();
 let mut feedback_commands=[0_u32;3];
 let mut feedback_respawns=player.respawns;
 let mut audio_respawns=player.respawns;let mut audio_was_paused=false;
 let mut incline_verified=false;let mut max_ramp=0_f32;let mut saw_jump=false;let mut saw_roll=false;let mut saw_wall=false;let mut saw_platform=false;let mut best_chain=0;let mut best_bounces=0;let mut saw_ready=false;let mut saw_homing=false;let mut homing_hits=0;let mut single_homing_hit=false;let mut observed_clips=std::collections::BTreeSet::new();
 loop{
  let cpu_start=std::time::Instant::now();
  let raw_dt=get_frame_time();let dt=raw_dt.min(0.1);let smoke_mode=smoke.is_some();
  if smoke_mode&&frame>=60&&raw_dt.is_finite()&&raw_dt>0.{
   let ms=raw_dt as f64*1000.;let fixture=matches!(frame-1,2391|2591);
   frame_intervals_ms.push(ms);if !fixture{steady_intervals.push(ms);}
   interval_rows.push(json!({"frame":frame-1,"interval_ms":ms,"fixture_setup":fixture,"state":player.state.label(),"cpu_ms":previous_cpu,"source_draw_ms":previous_source_draw_ms,"frame_await_ms":previous_await_ms}));
  }
  if smoke_mode&&frame>=60&&raw_dt>1./30.{slow_frames.push(json!({"frame":frame-1,"state":player.state.label(),"clip":player.clip(),"interval_ms":raw_dt as f64*1000.,"input_sim_audio_ms":previous_cpu[0],"draw_submission_ms":previous_cpu[1],"diagnostics_ms":previous_cpu[2],"source_draw_ms":previous_source_draw_ms,"frame_await_ms":previous_await_ms}));}
  let mut pad=controller.poll();let mut consumed=false;
  if pad.disconnected&&!smoke_mode{paused=true;clock.clear();menu_selection=0;}
  if is_key_pressed(KeyCode::F11){fullscreen=!fullscreen;set_fullscreen(fullscreen);}
  if is_key_pressed(KeyCode::H)||pad.pressed(xbox::VIEW){help=!help;}
  if !smoke_mode&&(is_key_pressed(KeyCode::Escape)||pad.pressed(xbox::MENU)){paused=!paused;clock.clear();menu_selection=0;consumed=true;}
  if paused&&!consumed{
   if pad.pressed(1)||(pad.left.y>0.6&&previous_menu_axis<=0.6)||is_key_pressed(KeyCode::Up){menu_selection=(menu_selection+5)%6;}
   if pad.pressed(2)||(pad.left.y< -0.6&&previous_menu_axis>=-0.6)||is_key_pressed(KeyCode::Down){menu_selection=(menu_selection+1)%6;}
   if pad.pressed(xbox::B){paused=false;consumed=true;clock.clear();}
   if pad.pressed(xbox::A)||is_key_pressed(KeyCode::Enter){
    match menu_selection{0=>{paused=false;},1=>{player.reset_in(&presentation);clock.clear();target=player.position+Vec3::Y*0.65;paused=false;},2..=4=>{presentation.platform=None;player.reset_in(&presentation);player.position=match menu_selection{2=>vec3(-70.,-16.,30.),3=>vec3(0.,0.,145.),_=>vec3(-22.,0.,-12.)};player.yaw=if menu_selection==3{std::f32::consts::FRAC_PI_2}else{0.};yaw=0.;pitch=0.36;target=player.position+Vec3::Y*0.65;paused=false;},_=>break,}
    clock.clear();consumed=true;
   }
  }
  previous_menu_axis=pad.left.y;
  if paused&&is_key_pressed(KeyCode::X){break;}
  if is_key_pressed(KeyCode::R){player.reset_in(&presentation);clock.clear();target=player.position+Vec3::Y*0.65;paused=false;}
  motor_output.select(if !smoke_mode&&pad.connected{Some(pad.index)}else{None});
  if paused||pad.disconnected||player.respawns!=feedback_respawns{
   presentation.feedback.borrow_mut().clear();presentation.motor_commands.borrow_mut().clear();motor_output.stop();
  }
  feedback_respawns=player.respawns;presentation.feedback_respawns.set(player.respawns);
  if (paused&&!audio_was_paused)||player.respawns!=audio_respawns{
   if let Some(audio)=&action_audio{audio.stop();}
  }
  audio_was_paused=paused;audio_respawns=player.respawns;
  let mouse=mouse_position();
  if !paused&&!smoke_mode{
   yaw+=(is_key_down(KeyCode::Q)as i32-is_key_down(KeyCode::E)as i32)as f32*dt*1.7;
   yaw-=pad.right.x*dt*2.6;
   yaw+=(pad.held(xbox::LB)as i32-pad.held(xbox::RB)as i32)as f32*dt*1.7;
   pitch=(pitch-pad.right.y*dt*1.1).clamp(0.12,1.1);distance=(distance-pad.zoom*dt*4.).clamp(2.5,10.);
   if pad.pressed(xbox::RS){yaw=player.yaw;pitch=0.36;}
   if is_mouse_button_down(MouseButton::Right){yaw-=(mouse.0-previous_mouse.0)*0.008;pitch=(pitch+(mouse.1-previous_mouse.1)*0.004).clamp(0.12,1.1);}
   distance=(distance-mouse_wheel().1*0.5).clamp(2.5,10.);
  }
  previous_mouse=mouse;
  if !paused{
   let input=if smoke_mode{script_controller.sample(Some((0,if new_features{new_features_input(frame,&mut player)}else if presentation.platform.is_some(){xbox::RawGamepad{buttons:if (120..=132).contains(&frame){xbox::A}else if frame==150{xbox::B}else{0},..Default::default()}}else{scripted(frame,&mut player)}))).movement(0.)}else{
    let x=(is_key_down(KeyCode::D)||is_key_down(KeyCode::Right))as i32-(is_key_down(KeyCode::A)||is_key_down(KeyCode::Left))as i32;
    let z=(is_key_down(KeyCode::W)||is_key_down(KeyCode::Up))as i32-(is_key_down(KeyCode::S)||is_key_down(KeyCode::Down))as i32;
    if consumed{pad.pressed=0;}
    let controller_input=pad.movement(yaw);
    let stick=if x!=0||z!=0{vec2(x as f32,z as f32).normalize()}else{pad.left};
    let on_tube=(100. ..=190.).contains(&player.position.z)&&player.position.x.abs()<12.1&&player.grounded;
    let direction=if on_tube{xbox::surface_camera_direction(stick,yaw,player.model_rotation(),player.yaw)}else{xbox::camera_direction(stick,yaw)};
    Input{direction,jump_pressed:!consumed&&(is_key_pressed(KeyCode::Space)||controller_input.jump_pressed),jump_held:is_key_down(KeyCode::Space)||controller_input.jump_held,charge:false,action_held:is_key_down(KeyCode::B)||controller_input.action_held,bounce_pressed:!consumed&&(is_key_pressed(KeyCode::B)||controller_input.bounce_pressed)}
   };
   let trace_before=player.position;let trace_ticks_before=player.ticks;
   clock.advance_in(&mut player,input,if smoke_mode{STEP}else{dt as f64},&presentation);
   if input_trace_rows<36000 {if let Some(trace)=input_trace.as_mut(){
    let raw=pad.raw.map(|r|json!({"lx":r.lx,"ly":r.ly,"rx":r.rx,"ry":r.ry,"buttons":r.buttons}));
    let row=json!({"frame":frame,"raw_dt":raw_dt,"simulation_dt":if smoke_mode{STEP}else{dt as f64},"controller":pad.index,"connected":pad.connected,"raw":raw,"left":pad.left.to_array(),"right":pad.right.to_array(),"camera_yaw":yaw,"direction":input.direction.to_array(),"jump":input.jump_pressed,"charge":input.charge,"bounce":input.bounce_pressed,"ticks_before":trace_ticks_before,"ticks_after":player.ticks,"position_before":trace_before.to_array(),"position_after":player.position.to_array(),"velocity":player.velocity.to_array(),"state":player.state.label(),"smoke":smoke_mode});
    if writeln!(trace,"{row}").is_err(){input_trace=None;}input_trace_rows+=1;
   }}
   if player.respawns!=audio_respawns{if let Some(audio)=&action_audio{audio.stop();}audio_respawns=player.respawns;}
   for event in presentation.events.borrow_mut().drain(..){
    if let Some(audio)=&mut action_audio{audio.event(event);}
   }
  }
  for command in presentation.motor_commands.borrow_mut().drain(..){use sonic_playground::feedback::MotorCommand;feedback_commands[match command{MotorCommand::Start=>0,MotorCommand::Stop=>1,MotorCommand::Brake=>2}]+=1;motor_output.command(command);}
  if let Some(audio)=&mut charge_audio{audio.update(&player,paused).expect("charge audio update");}
  if let Some(audio)=&mut ring_audio{audio.update_ring(&player,paused).expect("ring audio update");}
  if smoke_mode{yaw=if frame<60{2.7}else{0.};if matches!(frame,340|480|550|621|826|951|1351|1370|1391|1591|1691|1791){target=player.position+Vec3::Y*0.65;}}
  let cpu_sim=cpu_start.elapsed().as_secs_f64()*1000.;
  if new_features&&matches!(frame,60|120|160|220){target=player.position+Vec3::Y*0.65;}
  target=target.lerp(player.position+Vec3::Y*0.65,1.-(-12.*if smoke_mode{1./60.}else{dt}).exp());
  let effect_interval=effect_reference&&(340..480).contains(&frame);
  if effect_interval{distance=3.;pitch=0.95;target=player.position+Vec3::Y*0.5;}
  if effect_reference&&frame==480{distance=5.2;pitch=0.36;}
  let inside_tube=(100.0..=190.0).contains(&player.position.z)&&player.position.x.abs()<12.1&&player.grounded;
  let camera_up=if inside_tube{player.model_rotation().y_axis}else{Vec3::Y};
  if inside_tube{target=player.position+camera_up*0.65;}
  let back=vec3(-yaw.sin(),0.,-yaw.cos());
  let back=if inside_tube{xbox::surface_camera_back(yaw,player.model_rotation())}else{back};
  let desired=target+back*distance+camera_up*(pitch.sin()*distance+0.5);
  let eye=camera_position(target,desired);
  clear_background(color(0x93b9cb));
  set_camera(&Camera3D{position:eye,target,up:camera_up,fovy:50_f32.to_radians(),z_near:0.05,z_far:230.,..Default::default()});
  draw_world(&player,&ground,&ramp,&slopes,player.ticks as f32/60.);
  if let Some(platform)=&presentation.platform{
   let c=platform.center();
   let vertices=[vec3(-4.,0.,-3.),vec3(4.,0.,-3.),vec3(4.,0.,3.),vec3(-4.,0.,3.)].into_iter().map(|v|{let p=c+v;Vertex::new(p.x,p.y,p.z,0.,0.,color(0x38bda9))}).collect();
   draw_mesh(&Mesh{vertices,indices:vec![0,2,1,0,3,2],texture:None});
   draw_cube_wires(c,vec3(8.,0.03,6.),color(0xb8fff0));
   let relative=player.position-c;
   if relative.x.abs()<=4.&&relative.z.abs()<=3.&&relative.y>=-0.1{
    let shade=Color::new(0.03,0.06,0.08,0.45);
    let mut vertices=vec![Vertex::new(player.position.x,c.y+0.012,player.position.z,0.,0.,shade)];
    for i in 0..32{let a=i as f32*std::f32::consts::TAU/32.;let x=(player.position.x+0.31*a.cos()).clamp(c.x-4.,c.x+4.);let z=(player.position.z+0.31*a.sin()).clamp(c.z-3.,c.z+3.);vertices.push(Vertex::new(x,c.y+0.012,z,0.,0.,shade));}
    let indices=(0..32u16).flat_map(|i|[0,(i+1)%32+1,i+1]).collect();
    draw_mesh(&Mesh{vertices,indices,texture:None});
   }

  }
  // Diagnostic-only varied backdrop; no terrain or material changes.
  if effect_interval{
   for x in -12i32..12{for z in -40i32..60{
    let shade=if (x+z).rem_euclid(2)==0{color(0x567c31)}else{color(0x263b16)};
    draw_cube(vec3(x as f32*0.25,0.006,z as f32*0.25),vec3(0.25,0.002,0.25),None,shade);
   }}
  }
  previous_source_draw_ms=0.;
  if let Some(source)=&source_renderer{let source_start=std::time::Instant::now();source.draw(&player).expect("source pose draw");previous_source_draw_ms=source_start.elapsed().as_secs_f64()*1000.;
   if smoke_mode && (frame%15==0||player.source_lean()!=0){
    source.verify_cached_pose(&player).expect("cache reconstruction mismatch");cache_checks[0]+=1;
    if player.source_animation_clock().and_then(|c|c.blend()).is_some(){cache_checks[1]+=1;}
    if player.source_lean()!=0{cache_checks[2]+=1;}
   }
  }else{renderer.draw(&player,if paused{0.}else if smoke_mode{1./60.}else{dt});}

  // Original effect assets provide the attack and charge presentation.
  presentation.charges.borrow_mut().reset_if_needed(&player);
  charge_renderer.draw(&presentation.charges.borrow(),eye,target);
  fire_renderer.draw_with_up(&presentation.fire.borrow(),eye,target,camera_up);
  presentation.auras.borrow_mut().reset_if_needed(&player);
  aura_renderer.draw(&presentation.auras.borrow());
  presentation.dashes.borrow_mut().reset_if_needed(&player);
  dash_renderer.draw(&presentation.dashes.borrow());
  // Fixed virtual UI width avoids Windows per-monitor DPI changes overflowing panels.
  let mut ui_camera=Camera2D::from_display_rect(Rect::new(0.,0.,1280.,1280.*screen_height()/screen_width()));
  ui_camera.zoom.y=-ui_camera.zoom.y;
  set_camera(&ui_camera);
  hud(&player,paused,help,get_fps(),pad,menu_selection);
  let cpu_draw=cpu_start.elapsed().as_secs_f64()*1000.;
  if new_features{
   let out=smoke.as_ref().unwrap();let label=if frame==10{Some("fire")}else if frame==64{Some("rings")}else if frame==130{Some("bowl")}else if frame==232{Some("fast-somersault")}else if frame>=160&&frame<220&&feature_captures.len()==3&&player.model_rotation().y_axis.y< -0.8{Some("tunnel")}else{None};
   if let Some(label)=label{
    if label=="fire"{assert!(fire_renderer.draws>0&&player.state==State::Somersault);}if label=="rings"{assert_eq!(player.state,State::RingDash);}
    if screenshots{get_screen_data().export_png(out.join(format!("{label}.png")).to_str().unwrap());}
    feature_captures.push(json!({"label":label,"frame":frame,"state":player.state.label(),"position":player.position.to_array(),"up":player.model_rotation().y_axis.to_array(),"fire_draws":fire_renderer.draws}));
    if label=="fast-somersault"{assert!(fire_renderer.draws>0&&player.state==State::Somersault);assert!((96..=99).contains(&player.source_animation_clock().unwrap().output().current));assert_eq!(feature_captures.len(),5);std::fs::write(out.join("new-features.json"),serde_json::to_vec_pretty(&feature_captures).unwrap()).unwrap();break;}
   }
   assert!(frame<340,"missing inverted tunnel capture");frame+=1;next_frame().await;continue;
  }
  if let (Some(platform),Some(out))=(&presentation.platform,&smoke){
   let center=platform.center();let relative=player.position-center;
   if frame<120{let initial=*platform_relative.get_or_insert(relative);platform_drift=platform_drift.max((relative-initial).length());}
   platform_air|=!player.grounded;platform_return|=frame>150&&player.bounced>0&&player.grounded&&(player.position.y-center.y).abs()<0.02;
   if [60,130,155,200,300,479].contains(&frame){
    let name=format!("platform-{frame}.png");get_screen_data().export_png(out.join(&name).to_str().unwrap());
    platform_frames.push(json!({"frame":frame,"image":name,"player":player.position.to_array(),"platform":center.to_array(),"grounded":player.grounded,"state":player.state.label()}));
   }
   if frame==479{
    let passed=platform_drift<0.002&&platform_air&&platform_return&&player.jumps==1&&player.bounced==1&&player.respawns==0;
    std::fs::write(out.join("platform-smoke.json"),serde_json::to_vec_pretty(&json!({"passed":passed,"idle_relative_drift":platform_drift,"airborne":platform_air,"returned_to_platform":platform_return,"jumps":player.jumps,"bounces":player.bounced,"respawns":player.respawns,"frames":platform_frames,"physical_controller_verified":false})).unwrap()).unwrap();
    assert!(passed,"platform rendered course failed");break;
   }
   frame+=1;let await_start=std::time::Instant::now();next_frame().await;previous_await_ms=await_start.elapsed().as_secs_f64()*1000.;continue;
  }

  if let Some(out)=&smoke{
   if effect_interval{
    let trails=presentation.dashes.borrow();
    effect_frames.push(json!({"frame":frame,"state":player.state.label(),"position":player.position.to_array(),"velocity":player.velocity.to_array(),"eye":eye.to_array(),"target":target.to_array(),"animation":player.source_animation_clock().map(|c|{let o=c.output();json!({"id":o.current,"frame":o.frame})}),"trails":trails.trails.iter().map(|t|json!({"age":t.age,"mode2":t.mode2,"tail":t.tail.to_array(),"head":t.head.to_array()})).collect::<Vec<_>>()}));
    if screenshots&&(390..=450).contains(&frame)&&frame%5==0{get_screen_data().export_png(out.join(format!("effect-{frame}.png")).to_str().unwrap());}
   }
   if effect_reference&&frame==480{std::fs::write(out.join("effect-reference.json"),serde_json::to_vec_pretty(&effect_frames).unwrap()).unwrap();}
   let first_clip=observed_clips.insert(player.clip());
   if frame==180{incline_verified=player.grounded && player.position.z>15. && player.position.z<35. && (player.ground_normal-vec3(0.,1.,-0.2).normalize()).length()<1e-5;}
   if frame<240{max_ramp=max_ramp.max(player.position.y);}saw_jump|=player.state==State::Jump;saw_roll|=player.state==State::Roll;
   if frame>=480&&frame<550{saw_wall|=player.wall_hits>0;}if frame>550{saw_platform|=player.grounded&&(player.position.y-2.).abs()<0.01;}
   best_chain=best_chain.max(player.last_attack_hits);best_bounces=best_bounces.max(player.bounced);saw_ready|=player.charge>=FULL_CHARGE-0.001;
   if (951..=1030).contains(&frame){saw_homing|=player.state==State::Homing;homing_hits=player.defeated.iter().filter(|&&v|v).count();if frame==979{single_homing_hit=homing_hits==1;}}
   if [30,130,180,230,242,250,390,425,530,610,745,754,795,841,860,887,955,962,975,983,1010,1260,1280,1352,1360,1366,1374,1376,1380,1511,1571,1572,1590,1605,1690,1692,1700,1790,1792,1850,1890,1893,1900,1930,1990,1993,2000,2190,2430,2450,2630,2660].contains(&frame) || (first_clip && matches!(player.clip(),"bounce_fall_start"|"bounce_fall"|"turn")){
    let name=format!("frame-{frame:03}.png");if screenshots{get_screen_data().export_png(out.join(&name).to_str().unwrap());}
    telemetry.push(json!({"frame":frame,"image":if screenshots{Some(name)}else{None},"position":[player.position.x,player.position.y,player.position.z],"speed":player.speed(),"state":player.state.label(),"clip":player.clip(),"animation_clock":player.source_animation_clock().map(|c|{let o=c.output();json!({"selected":o.selected,"active":o.current,"mode":o.mode,"frame":o.frame})}),"grounded":player.grounded,"ground_normal":[player.ground_normal.x,player.ground_normal.y,player.ground_normal.z],"rings":player.collected_count()}));
   }
   if frame==1260{
    let charges=presentation.charges.borrow();
    assert!(player.charge>=FULL_CHARGE&&!charges.ribbons.is_empty()&&charges.ribbons.iter().all(|r|r.ready),"isolated ready glow was not sustained");
    assert!(charge_renderer.ready_draws>0,"ready ribbons were never rendered");
   }
   if frame==1350{assert_eq!(player.last_attack_hits,4,"release after sustained readiness must chain four targets");}
   if frame==1352{
    assert_eq!(player.state,State::Homing);assert_eq!(player.attack_target,None);
    assert!(player.velocity.y>10.&&player.velocity.z>25.);assert_eq!(player.clip(),"jump_dash");
   }
   if frame==1365{assert_eq!(player.state,State::Homing,"air dash ended before14force updates");}
   if frame==1366{
    assert_eq!(player.state,State::Fall);assert!(player.defeated.iter().all(|v|!*v));
    std::fs::write(out.join("air-dash.json"),serde_json::to_vec_pretty(&json!({"passed":true,"launch_frame":1352,"expiry_frame":1366,"velocity":player.velocity.to_array(),"position":player.position.to_array(),"no_false_hits":true})).unwrap()).unwrap();
   }
   if frame==1371{assert_eq!(player.state,State::Homing);assert_eq!(player.attack_target,None);}
   if frame==1374||frame==1375{assert_eq!(player.state,State::Fall);assert_eq!(player.clip(),"fall");assert_eq!(player.source_animation_clock().unwrap().output().selected,15);}
   if frame==1376{
    assert_eq!(player.state,State::BounceDown);assert_eq!(player.clip(),"bounce");
    assert!(player.velocity.y< -25.&&player.defeated.iter().all(|v|!*v));
    std::fs::write(out.join("cancel-bounce.json"),serde_json::to_vec_pretty(&json!({"passed":true,"launch_frame":1371,"cancel_frame":1374,"bounce_frame":1376,"velocity":player.velocity.to_array(),"no_false_hits":true})).unwrap()).unwrap();
   }
   if frame==1571{
    assert_eq!(player.charge,FULL_CHARGE);assert!(player.grounded&&player.state==State::Run);
    assert_eq!(player.velocity.z.to_bits(),6.657930374145508_f32.to_bits());
    assert_eq!(player.clip(),"speedwalk");
    assert_eq!(player.source_animation_clock().unwrap().output().selected,7);
    assert!(presentation.charges.borrow().ribbons.iter().any(|r|r.ready));
   }
   if frame==1572{
    assert_eq!(player.charge,0.);assert!(player.attack_target.is_none());
    assert_eq!(player.state,State::Run);let expected=6.657930374145508_f32-player.physics.parameter(16).unwrap().abs()*6.;assert!((player.velocity.z-expected).abs()<1e-5);
    std::fs::write(out.join("ready-movement.json"),serde_json::to_vec_pretty(&json!({"passed":true,"ready_walk_frame":1571,"release_frame":1572,"velocity":player.velocity.to_array(),"no_false_hits":player.defeated.iter().all(|v|!*v)})).unwrap()).unwrap();
   }
   if frame==1605{
    assert!(!player.grounded);assert_eq!(player.state,State::Fall);assert_eq!(player.jumps,0);
    assert!(player.velocity.y<0.);
    std::fs::write(out.join("walkoff-flight.json"),serde_json::to_vec_pretty(&json!({"passed":true,"frame":frame,"position":player.position.to_array(),"velocity":player.velocity.to_array(),"jumps":player.jumps})).unwrap()).unwrap();
   }
   if frame==1690{
    assert!(player.grounded);assert_eq!(player.jumps,0);
    std::fs::write(out.join("walkoff-landing.json"),serde_json::to_vec_pretty(&json!({"passed":true,"frame":frame,"position":player.position.to_array(),"jumps":player.jumps})).unwrap()).unwrap();
   }
   if frame==1692{
    assert!(!player.grounded);assert_eq!(player.state,State::Jump);assert_eq!(player.jumps,1);
    assert!(player.velocity.y>8.);
    let tilt=(player.model_rotation()*Vec3::Y).distance(Vec3::Y);assert!(tilt>0.01);
    std::fs::write(out.join("slope-takeoff.json"),serde_json::to_vec_pretty(&json!({"passed":true,"frame":frame,"position":player.position.to_array(),"velocity":player.velocity.to_array(),"tilt":tilt})).unwrap()).unwrap();
   }
   if frame==1792{assert_eq!(player.state,State::Brake);assert_eq!(player.clip(),"brake");assert!(player.velocity.z>0.&&player.velocity.z<18.);}
   if frame==1850{
    assert!(observed_clips.contains("turn"));assert_eq!(player.state,State::Run);assert!(player.velocity.z<0.);assert_ne!(player.clip(),"turn");
    std::fs::write(out.join("brake-turn.json"),serde_json::to_vec_pretty(&json!({"passed":true,"brake_frame":1792,"finished_frame":1850,"velocity":player.velocity.to_array(),"turn_observed":true})).unwrap()).unwrap();
   }
   if frame==1894{assert!(player.grounded);assert_eq!(player.source_animation_clock().unwrap().output().selected,17);}
   if frame==1930{assert_eq!(player.state,State::Land);assert_eq!(player.source_animation_clock().unwrap().output().current,17);}
   if frame==1990{assert_eq!(player.state,State::Idle,"hard landing must complete");}
   if frame==1994{assert_eq!(player.state,State::Brake);assert_eq!(player.source_animation_clock().unwrap().output().selected,18);assert!(player.velocity.z>0.);}
   if frame==2430{
    let idle=player.source_animation_clock().unwrap().output();
    assert!(matches!(idle.selected,88|89)&&idle.current==idle.selected);
    std::fs::write(out.join("idle-variant.json"),serde_json::to_vec_pretty(&json!({"selected":idle.selected,"current":idle.current,"frame":idle.frame,"warmup_ticks":1000})).unwrap()).unwrap();
   }
   if frame==2450{assert!(!matches!(player.source_animation_clock().unwrap().output().selected,88|89));}
   if frame==2630{
    let idle=player.source_animation_clock().unwrap().output();
    assert_eq!((idle.selected,idle.current),(89,89));
    std::fs::write(out.join("idle-variant-second.json"),serde_json::to_vec_pretty(&json!({"selected":idle.selected,"current":idle.current,"frame":idle.frame,"warmup_ticks":13550})).unwrap()).unwrap();
   }
   if frame==2660{assert!(!matches!(player.source_animation_clock().unwrap().output().selected,88|89));}
   if frame>=2790{
    if source_renderer.is_some(){
     assert!(cache_checks.iter().all(|n|*n>0),"cache comparison must cover blends and lean");
     std::fs::write(out.join("pose-cache-fidelity.json"),serde_json::to_vec_pretty(&json!({"passed":true,"checks":cache_checks[0],"blend_checks":cache_checks[1],"lean_checks":cache_checks[2]})).unwrap()).unwrap();
    }
    assert_eq!(player.state,State::Idle,"hard landing must complete");
    assert!(charge_renderer.head_draws>0&&charge_renderer.draws>0&&presentation.charges.borrow().spawned>0,"charge strip path not exercised");
    std::fs::write(out.join("charge.json"),serde_json::to_vec_pretty(&json!({"draws":charge_renderer.draws,"head_draws":charge_renderer.head_draws,"spawned":presentation.charges.borrow().spawned,"ready_spawned":presentation.charges.borrow().ready_spawned,"ready_draws":charge_renderer.ready_draws})).unwrap()).unwrap();
    assert!(dash_renderer.draws>0&&presentation.dashes.borrow().mode_spawns.iter().all(|n|*n>0),"original dash path not exercised");
    std::fs::write(out.join("dash.json"),serde_json::to_vec_pretty(&json!({"draws":dash_renderer.draws,"spawned":presentation.dashes.borrow().spawned,"modes":presentation.dashes.borrow().mode_spawns})).unwrap()).unwrap();
    assert!(aura_renderer.draws>0&&presentation.auras.borrow().spawned>0,"original aura path not exercised");
    assert!(presentation.auras.borrow().variant_spawns.iter().all(|n|*n>0),"both original aura textures must be exercised");
    std::fs::write(out.join("aura.json"),serde_json::to_vec_pretty(&json!({"draws":aura_renderer.draws,"spawned":presentation.auras.borrow().spawned,"variant_spawns":presentation.auras.borrow().variant_spawns})).unwrap()).unwrap();
    if let Some(audio)=&charge_audio{assert_eq!((audio.starts,audio.stops),(4,4),"charge lifecycle playback");std::fs::write(out.join("charge-audio.json"),serde_json::to_vec_pretty(&json!({"starts":audio.starts,"stops":audio.stops,"device_calls_succeeded":true,"audibility_verified":false})).unwrap()).unwrap();}
    let audio_requests=action_audio.as_ref().map(|a|a.requests).unwrap_or([0;5]);
    if let Some(audio)=&action_audio{
     assert!(audio.voice_requests>0);
     assert_eq!(audio.voice_requests,audio.requests[4],"one voice per charged attack start");
     std::fs::write(out.join("light-voice-audio.json"),serde_json::to_vec_pretty(&json!({"requests":audio.voice_requests,"language":"English","audibility_verified":false})).unwrap()).unwrap();
    }
    let passed=incline_verified&&max_ramp>=3.9&&saw_jump&&saw_roll&&saw_wall&&saw_platform&&best_chain==4&&best_bounces>=2&&saw_ready&&saw_homing&&single_homing_hit&&homing_hits==2&&(!preview||audio_requests.iter().all(|n|*n>0))&&["jump","spin_jump","jump_dash","rebound","bounce_fall_start","bounce_fall"].iter().all(|c|observed_clips.contains(c));
    frame_intervals_ms.sort_by(f64::total_cmp);
    assert!(!frame_intervals_ms.is_empty());
    let n=frame_intervals_ms.len();
    let timing=json!({"screenshot_capture":screenshots,"samples":n,"median_ms":frame_intervals_ms[n/2],"p95_ms":frame_intervals_ms[(n*95/100).saturating_sub(1)],"max_ms":frame_intervals_ms[n-1],"over_33_33_ms":frame_intervals_ms.iter().filter(|v|**v>1000./30.).count(),"scope":"Raw platform frame intervals after60warmupframes; includes vsync, OS scheduling, and screenshot readback/writes when enabled. Scripted simulation uses fixedstep; not a physical playtest or GPU-only timer."});
    steady_intervals.sort_by(f64::total_cmp);let count=steady_intervals.len();
    let steady=json!({"samples":count,"excluded_fixture_frames":[2391,2591],"median_ms":steady_intervals[count/2],"p95_ms":steady_intervals[(count*95/100).saturating_sub(1)],"max_ms":steady_intervals[count-1],"over_33_33_ms":steady_intervals.iter().filter(|v|**v>1000./30.).count(),"scope":"Same course excluding only two multi-tick idle fixture setup frames; includes normal transitions, OS scheduling, presentation, other diagnostic work and optional captures. Not guaranteed live-input FPS."});
    std::fs::write(out.join("frame-intervals.json"),serde_json::to_vec(&interval_rows).unwrap()).unwrap();
    std::fs::write(out.join("steady-frame-timing.json"),serde_json::to_vec_pretty(&steady).unwrap()).unwrap();
    std::fs::write(out.join("slow-frames.json"),serde_json::to_vec_pretty(&slow_frames).unwrap()).unwrap();
    std::fs::write(out.join("frame-timing.json"),serde_json::to_vec_pretty(&timing).unwrap()).unwrap();
    let feedback_counts=presentation.feedback_requests.get();
    let feedback_passed=feedback_counts[0]>=2&&feedback_counts[1]>=1&&feedback_commands[0]>0&&feedback_commands[2]>0;
    let feedback_report=json!({"passed":feedback_passed,"bounce_requests":feedback_counts[0],"hard_landing_requests":feedback_counts[1],"start_commands":feedback_commands[0],"stop_commands":feedback_commands[1],"brake_commands":feedback_commands[2],"physical_output_enabled":false,"scope":"Desktop fixed-tick queue and command dispatch; smoke mode selects no hardware slot. Physical vibration unverified."});
    std::fs::write(out.join("feedback.json"),serde_json::to_vec_pretty(&feedback_report).unwrap()).unwrap();
    assert!(feedback_passed,"feedback course failed: {feedback_report}");
    let report=json!({"passed":passed,"incline_contact_verified":incline_verified,"observed_clips":observed_clips,"audio_preview":preview,"audio_playback_requests":audio_requests,"audible_output_verified":false,"frames":frame+1,"max_ramp_height":max_ramp,"jump":saw_jump,"roll":saw_roll,"wall":saw_wall,"platform":saw_platform,"light_attack_hits":best_chain,"bounces":best_bounces,"full_charge":saw_ready,"homing_observed":saw_homing,"single_homing_hit":single_homing_hit,"homing_hits":homing_hits,"screenshots":telemetry,"controller_detected":pad.connected,"note":"Scripted Xbox raw states exercise the shared controller mapping, simulation and renderer. Physical controller playtest remains a user check."});
    std::fs::write(out.join("smoke.json"),serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    assert!(passed,"graphics course failed: {report}");break;
   }
  }
  let cpu_end=cpu_start.elapsed().as_secs_f64()*1000.;previous_cpu=[cpu_sim,cpu_draw-cpu_sim,cpu_end-cpu_draw];
  frame+=1;let await_start=std::time::Instant::now();next_frame().await;previous_await_ms=await_start.elapsed().as_secs_f64()*1000.;
 }
}






fn new_features_input(frame:u32,p:&mut Player)->xbox::RawGamepad{
 if matches!(frame,60|120|160){p.reset();p.position=match frame{60=>vec3(-22.,0.,-12.),120=>vec3(-70.,-16.,30.),_=>vec3(0.,0.,145.)};if frame==160{p.velocity=vec3(60.,0.,0.);p.yaw=std::f32::consts::FRAC_PI_2;p.state=State::Roll;}}
 if frame==220{p.reset();p.velocity=vec3(0.,0.,36.);p.state=State::Run;}
 xbox::RawGamepad{buttons:if (2..7).contains(&frame)||(220..224).contains(&frame)||frame==60{xbox::B}else{0},..Default::default()}
}
