//! Native Xbox/XInput polling. No remapping software or Steam Input is required.
use glam::{Vec2, vec2};
#[cfg(test)]
use glam::{Mat4, Vec3, vec3};
pub const A:u16=0x1000;pub const B:u16=0x2000;pub const X:u16=0x4000;pub const Y:u16=0x8000;
pub const MENU:u16=0x10;pub const VIEW:u16=0x20;pub const RS:u16=0x80;
pub const LB:u16=0x100;pub const RB:u16=0x200;
/// Camera looks along +forward with +Y up; screen right is forward cross up.
pub fn camera_direction(stick:Vec2,yaw:f32)->Vec2{
 let forward=vec2(yaw.sin(),yaw.cos());let right=vec2(-yaw.cos(),yaw.sin());
 forward*stick.y+right*stick.x
}
/// Follow-camera back projected onto the contact plane, shared by view and input.
pub fn surface_camera_back(yaw:f32,rotation:glam::Mat3)->glam::Vec3{
 let back=glam::vec3(-yaw.sin(),0.,-yaw.cos());let up=rotation.y_axis;
 (back-up*back.dot(up)).try_normalize().unwrap_or(-rotation.z_axis)
}
/// Convert a camera-plane command into the heading coordinates of the source
/// controller. Its heading is measured before the contact pitch/roll rotation.
pub fn surface_camera_direction(stick:Vec2,camera_yaw:f32,rotation:glam::Mat3,player_yaw:f32)->Vec2{
 let forward=-surface_camera_back(camera_yaw,rotation);
 let right=forward.cross(rotation.y_axis).normalize_or_zero();
 let local=rotation.transpose()*(forward*stick.y+right*stick.x);
 let forward=vec2(player_yaw.sin(),player_yaw.cos());
 let right=vec2(player_yaw.cos(),-player_yaw.sin());
 forward*local.z+right*local.x
}
#[repr(C)]
#[derive(Default,Clone,Copy)]
pub struct RawGamepad {pub buttons:u16,pub left_trigger:u8,pub right_trigger:u8,pub lx:i16,pub ly:i16,pub rx:i16,pub ry:i16}
#[repr(C)]
#[derive(Default)]
struct RawState{packet:u32,gamepad:RawGamepad}
#[cfg(windows)]
#[link(name="xinput9_1_0")]
unsafe extern "system" {fn XInputGetState(index:u32,state:*mut RawState)->u32;}
#[repr(C)]struct RawVibration{left:u16,right:u16}
#[cfg(windows)]
#[link(name="xinput9_1_0")]
unsafe extern "system" {fn XInputSetState(index:u32,vibration:*const RawVibration)->u32;}
fn set_motor(index:u32,on:bool){
 #[cfg(windows)]{
  // GameCube motor commands are binary. This moderate Xbox motor strength is
  // a host adaptation, not a recovered dual-motor amplitude.
  let value=RawVibration{left:if on{32767}else{0},right:0};
  unsafe{XInputSetState(index,&value);}
 }
 #[cfg(not(windows))]let _=(index,on);
}
pub struct MotorOutput<F:FnMut(u32,bool)=fn(u32,bool)>{index:Option<u32>,sink:F}
impl Default for MotorOutput{fn default()->Self{Self::with_sink(set_motor as fn(u32,bool))}}
impl<F:FnMut(u32,bool)> MotorOutput<F>{
 pub fn with_sink(sink:F)->Self{Self{index:None,sink}}
 pub fn select(&mut self,index:Option<u32>){
  if self.index!=index{self.stop();self.index=index;}
 }
 pub fn command(&mut self,command:crate::feedback::MotorCommand){
  if let Some(index)=self.index{(self.sink)(index,command==crate::feedback::MotorCommand::Start);}
 }
 pub fn stop(&mut self){if let Some(index)=self.index{(self.sink)(index,false);}}
}
impl<F:FnMut(u32,bool)> Drop for MotorOutput<F>{fn drop(&mut self){self.stop();}}
#[derive(Default,Clone,Copy)]
pub struct Snapshot {pub raw:Option<RawGamepad>,pub connected:bool,pub index:u32,pub left:Vec2,pub right:Vec2,pub zoom:f32,pub buttons:u16,pub pressed:u16,pub disconnected:bool}
impl Snapshot {
 pub fn held(self,b:u16)->bool{self.buttons&b!=0}pub fn pressed(self,b:u16)->bool{self.pressed&b!=0}
 pub fn movement(self,yaw:f32)->crate::simulation::Input {
  crate::simulation::Input{direction:camera_direction(self.left,yaw),jump_pressed:self.pressed(A),jump_held:self.held(A),charge:false,action_held:self.held(B|X),bounce_pressed:self.pressed(B|X)&&self.buttons&!self.pressed&(B|X)==0}
 }
}
#[derive(Default)]
pub struct Controller {previous:u16,index:Option<u32>}
pub fn stick(x:i16,y:i16,deadzone:f32)->Vec2{
 let v=vec2(x as f32,y as f32);let length=v.length();
 if length<=deadzone{Vec2::ZERO}else{v/length*((length.min(32767.)-deadzone)/(32767.-deadzone))}
}
impl Controller {
 pub fn sample(&mut self,state:Option<(u32,RawGamepad)>)->Snapshot{
  if let Some((index,g))=state {
   let disconnected=self.index.is_some_and(|previous|previous!=index);
   let same=self.index==Some(index);let pressed=if same{g.buttons & !self.previous}else{0};
   self.previous=g.buttons;self.index=Some(index);
   let mut left=stick(g.lx,g.ly,7849.);
   let dpad=vec2((g.buttons&8!=0)as i32 as f32-(g.buttons&4!=0)as i32 as f32,(g.buttons&1!=0)as i32 as f32-(g.buttons&2!=0)as i32 as f32);
   if dpad.length_squared()>0.{left=dpad.normalize();}
   let trigger=|x:u8|if x>30{(x as f32-30.)/225.}else{0.};
   Snapshot{raw:Some(g),connected:true,index,left,right:stick(g.rx,g.ry,8689.),zoom:trigger(g.right_trigger)-trigger(g.left_trigger),buttons:g.buttons,pressed,disconnected}
  }else{let disconnected=self.index.is_some();self.previous=0;self.index=None;Snapshot{disconnected,..Default::default()}}
 }
 pub fn poll(&mut self)->Snapshot{
  #[cfg(windows)]{
   let mut order=vec![0,1,2,3];if let Some(i)=self.index{order.retain(|&n|n!=i);order.insert(0,i);}
   for i in order {let mut s=RawState::default();if unsafe{XInputGetState(i,&mut s)}==0{return self.sample(Some((i,s.gamepad)));}}
  }
  self.sample(None)
 }
}
#[cfg(test)]
mod tests {
 use super::*;
 #[test]fn abi_sizes(){assert_eq!(std::mem::size_of::<RawGamepad>(),12);assert_eq!(std::mem::size_of::<RawState>(),16);}
 #[test]fn deadzones(){assert_eq!(stick(1000,-3000,7849.),Vec2::ZERO);assert!((stick(32767,0,7849.).x-1.).abs()<1e-6);assert!((stick(-32768,32767,7849.).length()-1.).abs()<1e-6);let v=stick(20000,0,7849.);assert!(v.x>0.4&&v.x<0.6);}
 #[test]fn buttons_and_disconnect(){let mut p=Controller::default();p.sample(Some((0,RawGamepad::default())));let s=p.sample(Some((0,RawGamepad{buttons:A|X,..Default::default()})));assert!(s.pressed(A)&&s.held(X));assert!(!p.sample(Some((0,RawGamepad{buttons:A,..Default::default()}))).pressed(A));assert!(p.sample(None).disconnected);assert!(!p.sample(None).disconnected);assert!(!p.sample(Some((0,RawGamepad{buttons:A,..Default::default()}))).pressed(A));}
 #[test]fn controller_handoff_signals_pause_without_press_edges(){
  let mut p=Controller::default();
  let first=p.sample(Some((0,RawGamepad::default())));assert!(!first.disconnected);
  let active=p.sample(Some((0,RawGamepad{buttons:X,..Default::default()})));assert!(!active.disconnected&&active.pressed(X));
  let replacement=p.sample(Some((1,RawGamepad{buttons:A|MENU|X,lx:32767,..Default::default()})));
  assert!(replacement.connected&&replacement.disconnected);assert_eq!(replacement.index,1);assert_eq!(replacement.pressed,0);
  let held=p.sample(Some((1,RawGamepad{buttons:A|MENU|X,..Default::default()})));assert!(!held.disconnected);assert_eq!(held.pressed,0);
  p.sample(Some((1,RawGamepad::default())));
  assert!(p.sample(Some((1,RawGamepad{buttons:MENU,..Default::default()}))).pressed(MENU));
 }
 #[test]fn triggers_and_dpad(){let mut p=Controller::default();let s=p.sample(Some((1,RawGamepad{buttons:1|8,right_trigger:255,..Default::default()})));assert!((s.left.length()-1.).abs()<1e-6);assert_eq!(s.zoom,1.);}
 #[test]fn gameplay_mapping(){let s=Snapshot{left:vec2(0.,0.5),buttons:A|X,pressed:A,..Default::default()};let i=s.movement(std::f32::consts::FRAC_PI_2);assert!(i.direction.distance(vec2(0.5,0.))<1e-6);assert!(i.jump_pressed&&i.jump_held&&i.action_held&&!i.charge);}
 #[test]fn stick_right_projects_right(){for yaw in [0_f32,0.4,1.57,3.14,4.7]{let eye=vec3(-yaw.sin()*5.,2.,-yaw.cos()*5.);let view=Mat4::look_at_rh(eye,Vec3::ZERO,Vec3::Y);let delta=camera_direction(Vec2::X,yaw);let projected=view.transform_vector3(vec3(delta.x,0.,delta.y));assert!(projected.x>0.99,"reversed at yaw {yaw}");assert!(camera_direction(-Vec2::X,yaw).dot(delta)< -0.99);}}
 #[test]fn bounce_is_press_not_hold(){let mut c=Controller::default();c.sample(Some((0,RawGamepad::default())));assert!(c.sample(Some((0,RawGamepad{buttons:B,..Default::default()}))).movement(0.).bounce_pressed);assert!(!c.sample(Some((0,RawGamepad{buttons:B,..Default::default()}))).movement(0.).bounce_pressed);}
}
