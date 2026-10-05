//! Host-independent procedural attack trail. This is an approximation, not an
//! extracted SA2 effect. Simulation never depends on presentation history.
use glam::Vec3;
use crate::simulation::{Player,State};
#[derive(Clone,Copy)]
pub struct TrailPoint {pub position:Vec3,pub age:f32}
#[derive(Default)]
pub struct AttackEffects {pub trail:Vec<TrailPoint>,pub phase:f32,last_respawn:Option<u32>}
impl AttackEffects {
 pub fn active(state:State)->bool{matches!(state,State::Roll|State::BounceDown|State::BounceUp|State::Homing|State::LightAttack)}
 pub fn update(&mut self,p:&Player,dt:f32){
  if self.last_respawn!=Some(p.respawns){self.trail.clear();self.last_respawn=Some(p.respawns);}
  if !dt.is_finite()||dt<=0.{return;}
  let dt=dt.min(0.1);self.phase=(self.phase+dt*12.).rem_euclid(std::f32::consts::TAU);
  for v in &mut self.trail{v.age+=dt;}
  self.trail.retain(|v|v.age<0.18);
  let center=p.position+Vec3::Y*0.5;
  if self.trail.last().is_some_and(|v|v.position.distance(center)>4.){self.trail.clear();}
  if Self::active(p.state)&&center.is_finite()&&self.trail.last().is_none_or(|v|v.position.distance(center)>0.035){
   self.trail.push(TrailPoint{position:center,age:0.});
   if self.trail.len()>32{self.trail.remove(0);}
  }
 }
}
#[cfg(feature="renderer")]
impl AttackEffects {
 pub fn draw(&self,p:&Player){
  use macroquad::prelude::{Color,draw_line_3d};
  for pair in self.trail.windows(2){
   let alpha=(1.-pair[0].age/0.18).clamp(0.,1.);
   for offset in [-0.065,0.,0.065]{
    let shift=Vec3::Y*offset;
    draw_line_3d(pair[0].position+shift,pair[1].position+shift,Color::new(0.2,0.65,1.,alpha));
   }
  }
  if !Self::active(p.state)&&p.state!=State::Charge{return;}
  let ready=p.state==State::LightAttack || (p.state==State::Charge&&p.charge>=crate::simulation::FULL_CHARGE-0.001);
  let color=if ready{Color::new(1.,0.85,0.25,0.85)}else{Color::new(0.2,0.65,1.,0.75)};
  let center=p.position+Vec3::Y*0.5;
  for plane in 0..3{for segment in 0..24{
   let point=|i:usize|{
    let a=i as f32/24.*std::f32::consts::TAU+self.phase;
    let (x,y)=(a.cos()*0.48,a.sin()*0.48);
    center+match plane{0=>Vec3::new(x,y,0.),1=>Vec3::new(0.,x,y),_=>Vec3::new(x,0.,y)}
   };
   // Broken rotating arcs leave the original mesh visible.
   if segment%8<5{draw_line_3d(point(segment),point(segment+1),color);}
  }}
 }
}
#[cfg(test)]
mod tests {
 use super::*;
 #[test]fn trail_expires_pauses_and_clears_on_reset(){
  let mut p=Player::new();let mut e=AttackEffects::default();p.state=State::Homing;
  for _ in 0..60{p.position.z+=0.1;e.update(&p,1./60.);}
  assert!(e.trail.len()>1&&e.trail.len()<=32);
  let before=(e.trail.len(),e.phase,e.trail[0].age);e.update(&p,0.);
  assert_eq!(before,(e.trail.len(),e.phase,e.trail[0].age));
  p.state=State::Idle;for _ in 0..12{e.update(&p,1./60.);}assert!(e.trail.is_empty());
  p.state=State::Roll;e.update(&p,1./60.);assert!(!e.trail.is_empty());
  p.reset();e.update(&p,0.);assert!(e.trail.is_empty());
 }
}
