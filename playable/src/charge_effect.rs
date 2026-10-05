//! Charge ribbon lifecycle from8008919c/80089490/80089998.
//! Host coordinate arithmetic, RNG stream, upright gravity and tick parity remain adaptations.
use glam::Vec3;
use crate::simulation::{Player,State};

#[derive(Clone,Copy,Debug,PartialEq)]
pub struct RibbonVertex{pub position:Vec3,pub uv:[f32;2],pub color:u32}
pub struct Ribbon{pub ready:bool,pub vertices:Vec<RibbonVertex>,radius:f32,pitch:i32,yaw:i32,turn:i32}
fn point(center:Vec3,radius:f32,pitch:i32,yaw:i32)->Vec3{
 static MATH:std::sync::OnceLock<sa2_runtime::original_math::OriginalMath>=std::sync::OnceLock::new();
 let math=MATH.get_or_init(||sa2_runtime::original_math::OriginalMath::from_data_bundle(include_bytes!("../assets/original-math-data.bin").to_vec()).expect("verified original math data"));
 // Convert the host anchor to source axes/units before the original FMAs.
 // The host coordinate roundtrip is separate from source arithmetic parity.
 let source_center=[-center.z*10.,center.y*10.,-center.x*10.];
 let [x,y,z]=sa2_runtime::original_charge_point::point(source_center,radius,pitch,yaw,math).expect("finite charge point");
 Vec3::new(-z,y,-x)*0.1
}
impl Ribbon{
 pub fn head(&self)->Option<Vec3>{(self.vertices.len()<32).then(||self.vertices[self.vertices.len()-2].position)}
 fn new(center:Vec3,seed:&mut u32,ready:bool)->Self{
  let mut random=||sa2_runtime::original_effect_state::random(seed) as f32/32768.;
  let radius=if ready{1.5_f32.mul_add(random(),3.)}else{15.0_f32.mul_add(random(),25.)};
  let pitch=(12288.*random())as i32-2048;
  let yaw=(65536.*random())as i32;
  let turn=(1536.*random())as i32+1024;
  let vertices=vec![
   RibbonVertex{position:point(center,radius,pitch,yaw),uv:[0.,0.],color:0xffffffff},
   RibbonVertex{position:point(center,radius-1.5,pitch,yaw),uv:[0.,1.],color:0xffffffff}];
  Self{ready,vertices,radius,pitch,yaw,turn}
 }
 fn tick(&mut self,center:Vec3)->bool{
  if self.vertices.len()<32{
   let center=if self.ready{center+Vec3::Y*(self.vertices.len()as f32*0.02)}else{center};
   self.radius*=if self.ready{1.01}else{0.85};self.yaw=self.yaw.wrapping_add(self.turn);
   for _ in 0..2{
    let i=self.vertices.len();let radius=self.radius-if i%2==1{if self.ready{0.4}else{0.8}}else{0.};
    let u=if i>=30{1.}else if i%4<2{0.5625}else{0.4375};
    self.vertices.push(RibbonVertex{position:point(center,radius,self.pitch,self.yaw),uv:[u,(i%2)as f32],color:0xffffffff});
   }
   true
  }else{
   for vertex in &mut self.vertices{
    if vertex.color<=0x08000000{return false;}
    vertex.color-=0x08000000;
   }
   true
  }
 }
}
#[derive(Default)]
pub struct ChargeEffects{pub ribbons:Vec<Ribbon>,pub spawned:u32,pub ready_spawned:u32,seed:u32,tick:u32,last_respawn:Option<u32>}
impl ChargeEffects{
 pub fn reset_if_needed(&mut self,p:&Player){
  if self.last_respawn!=Some(p.respawns){self.ribbons.clear();self.tick=0;self.last_respawn=Some(p.respawns);}
 }
 /// Supply the midpoint of entity position and collision center, before ready rise.
 pub fn tick(&mut self,p:&Player,center:Vec3){
  self.reset_if_needed(p);
  if !center.is_finite(){self.ribbons.clear();return;}
  self.ribbons.retain_mut(|r|r.tick(center));
  // Source80076694..c4 also checks ability flags; demo enables Light Attack.
  if p.state==State::Charge&&p.charge<crate::simulation::FULL_CHARGE&&self.tick&1!=0{
   self.ribbons.push(Ribbon::new(center,&mut self.seed,false));self.spawned+=1;
  }
  if p.charge>=crate::simulation::FULL_CHARGE&&self.tick&3==0{
   self.ribbons.push(Ribbon::new(center,&mut self.seed,true));self.spawned+=1;self.ready_spawned+=1;
  }
  self.tick=self.tick.wrapping_add(1);
 }
}

#[cfg(feature="renderer")]
pub mod renderer{
 use super::*;
 use macroquad::prelude::*;
 use macroquad::miniquad::{BlendState,BlendFactor,BlendValue,Equation,Comparison,CullFace};
 use serde::Deserialize;
 use sha2::{Digest,Sha256};
 use std::path::Path;
 #[derive(Deserialize)]struct Manifest{texture:String,sha256:String,source_texture_index:u32,head_texture:String,head_sha256:String,head_half_extent:[f32;2],ready_head_half_extent:[f32;2]}
 pub struct ChargeRenderer{mesh:Mesh,head_mesh:Mesh,head_extent:[f32;2],ready_head_extent:[f32;2],material:Material,pub draws:u32,pub head_draws:u32,pub ready_draws:u32}
 impl ChargeRenderer{
  pub fn new(dir:&Path)->Result<Self,String>{
   let dir=dir.join("effects");
   let manifest:Manifest=serde_json::from_slice(&std::fs::read(dir.join("charge.json")).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
   if manifest.texture!="lcharge01.png"||manifest.source_texture_index!=25{return Err("invalid charge texture mapping".into());}
   let bytes=std::fs::read(dir.join(&manifest.texture)).map_err(|e|e.to_string())?;
   if format!("{:x}",Sha256::digest(&bytes))!=manifest.sha256{return Err("charge texture hash mismatch".into());}
   let texture=Texture2D::from_image(&Image::from_file_with_format(&bytes,None).map_err(|e|e.to_string())?);
   texture.set_filter(FilterMode::Linear);
   if manifest.head_texture!="lcharge02.png"||manifest.head_half_extent.iter().any(|v|!v.is_finite()||(*v-0.32).abs()>0.00001){return Err("invalid charge head descriptor".into());}
   let bytes=std::fs::read(dir.join(&manifest.head_texture)).map_err(|e|e.to_string())?;
   if format!("{:x}",Sha256::digest(&bytes))!=manifest.head_sha256{return Err("charge head hash mismatch".into());}
   let head_texture=Texture2D::from_image(&Image::from_file_with_format(&bytes,None).map_err(|e|e.to_string())?);head_texture.set_filter(FilterMode::Linear);
   if manifest.ready_head_half_extent.iter().any(|v|!v.is_finite()||(*v-0.08).abs()>0.00001){return Err("invalid ready head descriptor".into());}
   let head_mesh=Mesh{vertices:Vec::with_capacity(4),indices:vec![0,1,2,0,2,3],texture:Some(head_texture)};
   // Modern additive adapter; exact source global override state remains open.
   let material=load_material(ShaderSource::Glsl{vertex:crate::aura::renderer::VERTEX,fragment:crate::aura::renderer::FRAGMENT},MaterialParams{pipeline_params:PipelineParams{depth_test:Comparison::LessOrEqual,depth_write:false,cull_face:CullFace::Nothing,color_blend:Some(BlendState::new(Equation::Add,BlendFactor::Value(BlendValue::SourceAlpha),BlendFactor::One)),..Default::default()},..Default::default()}).map_err(|e|e.to_string())?;
   Ok(Self{mesh:Mesh{vertices:Vec::with_capacity(32),indices:Vec::with_capacity(90),texture:Some(texture)},head_mesh,head_extent:manifest.head_half_extent,ready_head_extent:manifest.ready_head_half_extent,material,draws:0,head_draws:0,ready_draws:0})
  }
  pub fn draw(&mut self,effects:&ChargeEffects,eye:Vec3,target:Vec3){
   gl_use_material(&self.material);
   for ribbon in effects.ribbons.iter().filter(|r|r.vertices.len()>3){
    self.mesh.vertices.clear();self.mesh.indices.clear();
    for v in &ribbon.vertices{
     let c=v.color;let p=v.position;
     self.mesh.vertices.push(Vertex::new(p.x,p.y,p.z,v.uv[0],v.uv[1],Color::from_rgba((c>>16)as u8,(c>>8)as u8,c as u8,(c>>24)as u8)));
    }
    for i in 2..ribbon.vertices.len()as u16{
     if i%2==0{self.mesh.indices.extend_from_slice(&[i-2,i-1,i]);}
     else{self.mesh.indices.extend_from_slice(&[i-1,i-2,i]);}
    }
    draw_mesh(&self.mesh);self.draws+=1;if ribbon.ready{self.ready_draws+=1;}
   }
   let forward=(target-eye).try_normalize().unwrap_or(Vec3::Z);
   let right=forward.cross(Vec3::Y).try_normalize().unwrap_or(Vec3::X);
   let up=right.cross(forward).normalize();
   for ribbon in &effects.ribbons{
    let Some(center)=ribbon.head()else{continue};
    let extent=if ribbon.ready{self.ready_head_extent}else{self.head_extent};
    self.head_mesh.vertices.clear();
    for (x,y,u,v) in [(-1.,-1.,0.,1.),(1.,-1.,1.,1.),(1.,1.,1.,0.),(-1.,1.,0.,0.)]{
     let p=center+right*(x*extent[0])+up*(y*extent[1]);
     self.head_mesh.vertices.push(Vertex::new(p.x,p.y,p.z,u,v,WHITE));
    }
    draw_mesh(&self.head_mesh);self.head_draws+=1;
   }
   gl_use_default_material();
  }
 }
}
