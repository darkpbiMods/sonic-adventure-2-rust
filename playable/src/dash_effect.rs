//! Source model20's two-update endpoint behavior and integer-angle orientation.
//! Host transcendental math and task scheduling remain approximations.
use glam::{Mat3,Vec3};
use crate::simulation::{Player,State};

pub struct Dash {
 pub age:u8,pub mode2:bool,pub tail:Vec3,pub head:Vec3,
 pub tail_basis:Mat3,pub head_basis:Mat3,
}
fn basis(back:Vec3)->Mat3{
 // 8008a338..378: source coordinates are (-host.z, host.y, -host.x).
 // Truncate radians * the DOL's double constant before the Y/Z wrappers
 // convert signed integer angles back to single-precision radians.
 // Zero displacement leaves source registers unspecified; use identity here.
 let Some(direction)=back.try_normalize() else{return Mat3::IDENTITY;};
 let radians=|angle:f32|{
  let units=(angle as f64*10430.38043493439_f64) as i32;
  (units as f64*9.587378008291125e-5_f64) as f32
 };
 let (sy,cy)=radians(back.x.atan2(-back.z)).sin_cos();
 let (sp,cp)=radians(direction.y.clamp(-1.,1.).asin()).sin_cos();
 // C * Ry(yaw) * Rz(pitch) * C^-1, C(x,y,z)=(-z,y,-x).
 let x=Vec3::new(cy,0.,sy);
 let y=Vec3::new(-sy*sp,cp,cy*sp);
 let z=Vec3::new(-sy*cp,-sp,cy*cp);
 Mat3::from_cols(x,y,z)
}
impl Dash {
 fn update(&mut self,center:Vec3,history:u8)->bool{
  self.age+=1;
  if self.age>20||(self.mode2&&self.age>=19){return false;}
  if self.age==1{
   let back=self.tail-center;
   if !back.is_finite()||back.length()>=4.{return false;}
   self.tail_basis=basis(back);self.head_basis=self.tail_basis;self.head=center;
   self.mode2=history&6!=6;
  }else if self.age==2{
   let back=self.head-center;
   if back.length_squared()>0.{self.head_basis=basis(back);self.head=center;}
  }
  true
 }
 pub fn vertex(&self,local:Vec3,source_index:usize)->Vec3{
  // Original even vertices have longitudinal x=8; odd vertices have x=0.
  let radial=Vec3::new(local.x,local.y,0.);
  if source_index%2==0{self.tail+self.tail_basis*radial}else{self.head+self.head_basis*radial}
 }
}
#[derive(Default)]pub struct DashEffects{
 pub trails:Vec<Dash>,pub spawned:u32,pub mode_spawns:[u32;2],history:u8,last_respawn:Option<u32>,
}
impl DashEffects{
 pub fn reset_if_needed(&mut self,p:&Player){
  if self.last_respawn!=Some(p.respawns){self.trails.clear();self.history=0;self.last_respawn=Some(p.respawns);}
 }
 pub fn tick(&mut self,p:&Player,center:Vec3){
  self.reset_if_needed(p);
  let history=self.history;
  self.trails.retain_mut(|trail|{
   let keep=trail.update(center,history);
   if keep&&trail.age==1{self.mode_spawns[trail.mode2 as usize]+=1;}
   keep
  });
  let forward=p.velocity.dot(Vec3::new(p.yaw.sin(),0.,p.yaw.cos()));
  let active=(p.state==State::Roll&&forward>=p.physics.parameter(9).unwrap()*6.)
   ||matches!(p.state,State::Homing|State::LightAttack)
   ||(p.state==State::Jump&&p.clip()=="jump_dash");
  if active&&center.is_finite(){
   self.trails.push(Dash{age:0,mode2:false,tail:center,head:center,tail_basis:Mat3::IDENTITY,head_basis:Mat3::IDENTITY});self.spawned+=1;
  }
  self.history=(self.history|u8::from(active)).wrapping_shl(1);
 }
}

#[cfg(feature="renderer")]
pub mod renderer{
 use super::*;
 use macroquad::prelude::*;
 use macroquad::miniquad::{BlendState,BlendFactor,BlendValue,Equation,Comparison,CullFace};
 use serde::Deserialize;
 use sha2::{Digest,Sha256};
 use std::{path::Path,collections::BTreeMap};
 #[derive(Deserialize)]struct Models{models:Vec<Model>}
 #[derive(Deserialize)]struct Model{source_id:u32,groups:Vec<Group>}
 #[derive(Deserialize)]struct Group{vertices:Vec<[f32;5]>,indices:Vec<u16>,flags:Flags}
 #[derive(Deserialize)]struct Flags{source_alpha:u32,destination_alpha:u32}
 #[derive(Deserialize)]struct Manifest{exported_source_indices:Vec<Vec<usize>>,original_colors_argb:Vec<u32>,textures:BTreeMap<String,Texture>,sequences:BTreeMap<String,Vec<String>>}
 #[derive(Deserialize)]struct Texture{file:String,sha256:String}
 pub struct DashRenderer{mesh:Mesh,local:Vec<Vec3>,source_indices:Vec<usize>,textures:BTreeMap<String,Texture2D>,sequences:BTreeMap<String,Vec<String>>,material:Material,pub draws:u32}
 impl DashRenderer{
  pub fn new(dir:&Path)->Result<Self,String>{
   let dir=dir.join("effects");
   let read=|name:&str|std::fs::read(dir.join(name)).map_err(|e|e.to_string());
   let models:Models=serde_json::from_slice(&read("effects.json")?).map_err(|e|e.to_string())?;
   let manifest:Manifest=serde_json::from_slice(&read("dash.json")?).map_err(|e|e.to_string())?;
   let model=models.models.into_iter().find(|m|m.source_id==20).ok_or("missing dash model")?;
   if model.groups.len()!=1||manifest.exported_source_indices.len()!=1{return Err("unsupported dash groups".into());}
   let group=model.groups.into_iter().next().unwrap();let source_indices=manifest.exported_source_indices[0].clone();
   if manifest.original_colors_argb.len()!=12{return Err("missing original dash vertex colors".into());}
   if group.flags.source_alpha!=4||group.flags.destination_alpha!=1||group.vertices.len()!=source_indices.len()||group.vertices.len()!=12||source_indices.iter().any(|i|*i>=12)||group.indices.len()!=36||group.indices.iter().any(|i|*i as usize>=group.vertices.len())||group.vertices.iter().flatten().any(|v|!v.is_finite()){return Err("invalid dash geometry/material".into());}
   let mut textures=BTreeMap::new();
   for (name,entry) in manifest.textures{
    if entry.file!=format!("{name}.png"){return Err("unexpected dash texture path".into());}
    let bytes=read(&entry.file)?;
    if format!("{:x}",Sha256::digest(&bytes))!=entry.sha256{return Err("dash texture hash mismatch".into());}
    let texture=Texture2D::from_image(&Image::from_file_with_format(&bytes,None).map_err(|e|e.to_string())?);texture.set_filter(FilterMode::Linear);textures.insert(name,texture);
   }
   for (name,count) in [("normal",20),("mode2",19)]{
    let sequence=manifest.sequences.get(name).ok_or("missing dash sequence")?;
    if sequence.len()!=count||sequence.iter().any(|s|!textures.contains_key(s)){return Err("invalid dash sequence".into());}
   }
   let local=group.vertices.iter().map(|v|vec3(v[0],v[1],v[2])).collect();
   let mesh=Mesh{vertices:group.vertices.iter().zip(&source_indices).map(|(v,index)|{
    let c=manifest.original_colors_argb[*index];let color=Color::from_rgba((c>>16)as u8,(c>>8)as u8,c as u8,(c>>24)as u8);
    Vertex::new(v[0],v[1],v[2],v[3],v[4],color)
   }).collect(),indices:group.indices,texture:None};
   let material=load_material(ShaderSource::Glsl{vertex:crate::aura::renderer::VERTEX,fragment:crate::aura::renderer::FRAGMENT},MaterialParams{pipeline_params:PipelineParams{depth_test:Comparison::LessOrEqual,depth_write:false,cull_face:CullFace::Nothing,color_blend:Some(BlendState::new(Equation::Add,BlendFactor::Value(BlendValue::SourceAlpha),BlendFactor::One)),..Default::default()},..Default::default()}).map_err(|e|e.to_string())?;
   Ok(Self{mesh,local,source_indices,textures,sequences:manifest.sequences,material,draws:0})
  }
  pub fn draw(&mut self,effects:&DashEffects){
   gl_use_material(&self.material);
   for trail in effects.trails.iter().filter(|t|t.age>0){
    let name=&self.sequences[if trail.mode2{"mode2"}else{"normal"}][trail.age as usize-1];
    self.mesh.texture=Some(self.textures[name].clone());
    for ((vertex,local),index) in self.mesh.vertices.iter_mut().zip(&self.local).zip(&self.source_indices){vertex.position=trail.vertex(*local,*index);}
    draw_mesh(&self.mesh);self.draws+=1;
   }
   gl_use_default_material();
  }
 }
}
