//! Model19 snapshot lifetime from8008b000/8008b06c. The host supplies center.
use glam::Vec3;
use crate::simulation::{Player,State};

pub struct Snapshot {pub position:Vec3,pub scale:f32,pub variant:usize,shrink:Option<f32>,timer:i16}
#[derive(Default)]
pub struct Auras {pub snapshots:Vec<Snapshot>,last_respawn:Option<u32>,pub spawned:u32,seed:u32,pub variant_spawns:[u32;2],was_dash:bool}
impl Auras {
 pub fn with_seed(seed:u32)->Self{Self{seed,..Default::default()}}
 pub fn reset_if_needed(&mut self,p:&Player) {
  if self.last_respawn!=Some(p.respawns){self.snapshots.clear();self.was_dash=false;self.last_respawn=Some(p.respawns);}
 }
 pub fn tick(&mut self,p:&Player,center:Vec3) {
  let center=p.aura_snapshot_center(center);
  self.reset_if_needed(p);
  self.snapshots.retain_mut(|s|{
   if let Some(shrink)=s.shrink{s.scale-=shrink; s.scale>0.}
   else{let old=s.timer;if old>0{s.scale-=0.3;}s.timer=s.timer.wrapping_add(1);old<=2}
  });
  let ordinary_jump=match p.source_air_action(){Some(action)=>action==6,None=>!p.grounded&&p.state==State::Jump&&matches!(p.clip(),"jump"|"spin_jump")};
  let dash=p.state==State::Homing||(p.state==State::Jump&&p.clip()=="jump_dash");
  let launch=p.homing_launch_center().is_some()||(dash&&!self.was_dash);
  let center=if launch{p.homing_launch_center().unwrap_or(center)}else{center};
  self.was_dash=dash;
  if center.is_finite()&&(ordinary_jump||p.state==State::BounceDown||launch){
   let random=sa2_runtime::original_effect_state::random(&mut self.seed);
   let variant=if random as f32*0.000030517578125>0.5{0}else{1};
   // 8008ab88/8008abb4: homing entry's larger snapshot shrinks by0.3.
   let shrink=if launch{Some(0.3)}else if p.state==State::BounceDown{Some(0.1)}else{None};
   self.snapshots.push(Snapshot{position:center,scale:if launch{3.}else{1.2},variant,shrink,timer:0});
   self.variant_spawns[variant]+=1;
   self.spawned+=1;
  }
 }
}

#[cfg(feature="renderer")]
pub mod renderer {
 use super::*;
 use macroquad::prelude::*;
 use macroquad::miniquad::{BlendState,BlendFactor,Equation,Comparison,CullFace};
 use serde::Deserialize;
 use sha2::{Sha256,Digest};
 use std::path::Path;
 #[derive(Deserialize)]struct Manifest{models:Vec<Model>,aura_variants:Vec<Variant>}
 #[derive(Deserialize)]struct Variant{name:String,texture:String,sha256:String}
 #[derive(Deserialize)]struct Model{source_id:u32,groups:Vec<Group>}
 #[derive(Deserialize)]struct Group{vertices:Vec<[f32;5]>,indices:Vec<u16>,flags:Flags}
 #[derive(Deserialize)]struct Flags{source_alpha:u32,destination_alpha:u32,diffuse:[f32;4]}
 pub struct AuraRenderer{mesh:Mesh,local:Vec<Vec3>,material:Material,textures:Vec<Texture2D>,pub draws:u32}
 impl AuraRenderer{
  pub fn new(dir:&Path)->Result<Self,String>{
   let dir=dir.join("effects");
   let manifest:Manifest=serde_json::from_slice(&std::fs::read(dir.join("effects.json")).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
   let model=manifest.models.into_iter().find(|m|m.source_id==19).ok_or("missing original aura model")?;
   if model.groups.len()!=1{return Err("unsupported aura groups".into());}
   let group=model.groups.into_iter().next().unwrap();
   if group.flags.source_alpha!=1||group.flags.destination_alpha!=1||group.vertices.is_empty()||group.vertices.len()>16384||group.indices.len()%3!=0||group.indices.iter().any(|i|*i as usize>=group.vertices.len())||group.vertices.iter().flatten().any(|v|!v.is_finite()){return Err("invalid aura material/geometry".into());}
   if manifest.aura_variants.len()!=2{return Err("missing aura texture variants".into());}
   let mut textures=Vec::new();
   for (i,variant) in manifest.aura_variants.iter().enumerate(){
    if variant.name!=["s_ball_a","s_ball_b"][i]||variant.texture!=format!("aura-{i}.png"){return Err("invalid aura variant order".into());}
    let bytes=std::fs::read(dir.join(&variant.texture)).map_err(|e|e.to_string())?;
    if format!("{:x}",Sha256::digest(&bytes))!=variant.sha256{return Err("aura texture hash mismatch".into());}
    let image=Image::from_file_with_format(&bytes,None).map_err(|e|e.to_string())?;
    let texture=Texture2D::from_image(&image);texture.set_filter(FilterMode::Nearest);textures.push(texture);
   }
   let color=Color::new(group.flags.diffuse[0],group.flags.diffuse[1],group.flags.diffuse[2],group.flags.diffuse[3]);
   let local=group.vertices.iter().map(|v|vec3(v[0],v[1],v[2])).collect();
   let mesh=Mesh{vertices:group.vertices.iter().map(|v|Vertex::new(v[0],v[1],v[2],v[3],v[4],color)).collect(),indices:group.indices,texture:Some(textures[0].clone())};
   let material=load_material(ShaderSource::Glsl{vertex:VERTEX,fragment:FRAGMENT},MaterialParams{
    pipeline_params:PipelineParams{depth_test:Comparison::LessOrEqual,depth_write:false,cull_face:CullFace::Back,
     color_blend:Some(BlendState::new(Equation::Add,BlendFactor::One,BlendFactor::One)),..Default::default()},..Default::default()
   }).map_err(|e|e.to_string())?;
   Ok(Self{mesh,local,material,textures,draws:0})
  }
  pub fn draw(&mut self,auras:&Auras){
   gl_use_material(&self.material);
   for aura in &auras.snapshots{
    self.mesh.texture=Some(self.textures[aura.variant].clone());
    for (vertex,local) in self.mesh.vertices.iter_mut().zip(&self.local){vertex.position=*local*aura.scale+aura.position;}
    draw_mesh(&self.mesh);self.draws+=1;
   }
   gl_use_default_material();
  }
 }
 pub(crate) const VERTEX:&str=r#"#version 100
 attribute vec3 position;
 attribute vec2 texcoord;
 attribute vec4 color0;
 uniform mat4 Model;
 uniform mat4 Projection;
 varying lowp vec2 uv;
 varying lowp vec4 color;
 void main(){gl_Position=Projection*Model*vec4(position,1.0);uv=texcoord;color=color0/255.0;}
 "#;
 pub(crate) const FRAGMENT:&str=r#"#version 100
 varying lowp vec2 uv;
 varying lowp vec4 color;
 uniform sampler2D Texture;
 void main(){gl_FragColor=texture2D(Texture,uv)*color;}
 "#;
}
