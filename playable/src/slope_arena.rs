//! Authored slope arena in host units. The same triangles feed display/collision.
use glam::{Vec3,vec3};
pub struct SlopeArena{pub bowl:Vec<[Vec3;3]>,pub tunnel:Vec<[Vec3;3]>}
impl SlopeArena{
 pub const BOWL_CENTER:Vec3=vec3(-70.,-16.,30.);
 pub const TUNNEL_CENTER:Vec3=vec3(0.,12.,100.);
 pub fn new()->Self{
  let mut bowl=Vec::new();let mut tunnel=Vec::new();let n=32;
  let radial=|i:usize|{let a=(i%n)as f32*std::f32::consts::TAU/n as f32;vec3(a.cos(),0.,a.sin())};
  let ring=|i:usize,band:usize|{let a=band as f32*std::f32::consts::FRAC_PI_2/4.;Self::BOWL_CENTER+radial(i)*(8.+16.*a.sin())+Vec3::Y*(16.*(1.-a.cos()))};
  for i in 0..n{
   Self::triangle(&mut bowl,[Self::BOWL_CENTER,ring(i,0),ring(i+1,0)],Vec3::Y);
   for band in 0..4{
    let normal=-radial(i)*((band as f32+0.5)*std::f32::consts::FRAC_PI_2/4.).sin()+Vec3::Y*((band as f32+0.5)*std::f32::consts::FRAC_PI_2/4.).cos();
    Self::quad(&mut bowl,[ring(i,band),ring(i+1,band),ring(i+1,band+1),ring(i,band+1)],normal);
   }
   let a=ring(i,4);let b=ring(i+1,4);Self::quad(&mut bowl,[a,b,b+radial(i+1)*3.,a+radial(i)*3.],Vec3::Y);
  }
  //90-unit tube, radius12; exact shared endpoint coordinates seal the seam.
  let circle=|i:usize,z:f32|{let a=(i%n)as f32*std::f32::consts::TAU/n as f32;Self::TUNNEL_CENTER+vec3(12.*a.sin(),-12.*a.cos(),z)};
  for i in 0..n{
   let z=0.;let a=(i as f32+0.5)*std::f32::consts::TAU/n as f32;
   Self::quad(&mut tunnel,[circle(i,z),circle(i+1,z),circle(i+1,z+90.),circle(i,z+90.)],vec3(-a.sin(),a.cos(),0.));
  }
  Self{bowl,tunnel}
 }
 fn triangle(out:&mut Vec<[Vec3;3]>,mut t:[Vec3;3],normal:Vec3){if (t[1]-t[0]).cross(t[2]-t[0]).dot(normal)<0.{t.swap(1,2);}out.push(t);}
 fn quad(out:&mut Vec<[Vec3;3]>,p:[Vec3;4],normal:Vec3){Self::triangle(out,[p[0],p[1],p[2]],normal);Self::triangle(out,[p[0],p[2],p[3]],normal);}
 pub fn shared()->&'static Self{static ARENA:std::sync::OnceLock<SlopeArena>=std::sync::OnceLock::new();ARENA.get_or_init(Self::new)}
 /// First two-sided surface crossing along a finite segment. Fraction is [0,1].
 pub fn segment_hit(&self,from:Vec3,to:Vec3)->Option<f32>{
  if !from.is_finite()||!to.is_finite(){return None;}
  let d=to-from;if d.length_squared()<1e-12{return None;}
  let lo=from.min(to);let hi=from.max(to);let mut best=1f32;let mut found=false;
  for (faces,min,max) in [(&self.bowl,vec3(-97.,-16.001,3.),vec3(-43.,0.001,57.)),(&self.tunnel,vec3(-12.001,-0.001,100.),vec3(12.001,24.001,190.))]{
   if hi.cmplt(min).any()||lo.cmpgt(max).any(){continue;}
   for t in faces{
    let e1=t[1]-t[0];let e2=t[2]-t[0];let h=d.cross(e2);let det=e1.dot(h);
    if det.abs()<1e-7{continue;}
    let offset=from-t[0];let u=offset.dot(h)/det;if !(-1e-5..=1.00001).contains(&u){continue;}
    let q=offset.cross(e1);let v=d.dot(q)/det;if v< -1e-5||u+v>1.00001{continue;}
    let fraction=e2.dot(q)/det;
    if fraction>1e-6&&fraction<=best{best=fraction;found=true;}
   }
  }
  found.then_some(best)
 }
 /// Pull back along the view segment; does not move the tracked character.
 pub fn clip_camera(&self,target:Vec3,desired:Vec3)->Vec3{
  self.segment_hit(target,desired).map_or(desired,|hit|target.lerp(desired,(hit-0.2/target.distance(desired)).max(0.)))
 }
 /// Vertical ray against upward faces only. Wall/ceiling support is separate.
 pub fn support(&self,p:Vec3,ceiling:f32)->Option<(f32,Vec3)>{
  let faces=if p.x>=-97.&&p.x<=-43.&&p.z>=3.&&p.z<=57.{&self.bowl}else if p.x.abs()<=12.&&p.z>=100.&&p.z<=190.{&self.tunnel}else{return None};
  let mut hit:Option<(f32,Vec3)>=None;
  for t in faces{
   let n=(t[1]-t[0]).cross(t[2]-t[0]).normalize();if n.y<=0.00001{continue;}
   let y=t[0].y-(n.x*(p.x-t[0].x)+n.z*(p.z-t[0].z))/n.y;
   if y>ceiling+0.001{continue;}
   let q=vec3(p.x,y,p.z);
   if (0..3).all(|i|(t[(i+1)%3]-t[i]).cross(q-t[i]).dot(n)>=-0.0001)&&hit.is_none_or(|(old,_)|y>old){hit=Some((y,n));}
  }
  hit
 }
 pub fn source_vertices(&self)->Vec<[[f32;3];3]>{self.bowl.iter().chain(&self.tunnel).map(|t|t.map(|p|(p*10.).to_array())).collect()}
}
impl Default for SlopeArena{fn default()->Self{Self::new()}}
