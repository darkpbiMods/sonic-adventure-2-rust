//! Fixed-tick fire particle storage. Source units; presentation and emission policy external.
use sa2_runtime::{original_fire_particle as source,original_math::OriginalMath};
#[derive(Clone,Copy)]pub struct Spawn{pub player:u8,pub center:[f32;3],pub velocity:[f32;3]}
#[derive(Clone,Default)]pub struct FireEffects{particles:Vec<[u8;32]>,seed:u32,last_respawn:Option<u32>,pub spawned:u64}
impl FireEffects{
 pub fn tick_player(&mut self,p:&crate::simulation::Player)->Result<(),String>{
  static MATH:std::sync::OnceLock<OriginalMath>=std::sync::OnceLock::new();
  let math=MATH.get_or_init(||OriginalMath::from_data_bundle(include_bytes!("../assets/original-math-data.bin").to_vec()).expect("fire math"));
  if self.last_respawn!=Some(p.respawns){self.clear();self.last_respawn=Some(p.respawns);}
  self.tick(p.fire_spawn(),math)
 }
 pub fn records(&self)->&[[u8;32]]{&self.particles}
 pub fn seed(&self)->u32{self.seed}
 pub fn clear(&mut self){self.particles.clear();}
 /// Advance existing particles, then enqueue an optional newly-created particle.
 /// The host must call this once per simulation tick, never per render frame.
 pub fn tick(&mut self,spawn:Option<Spawn>,math:&OriginalMath)->Result<(),String>{
  let mut next=self.clone();let mut retained=Vec::with_capacity(next.particles.len()+1);
  for mut data in next.particles{if !source::update(&mut data,math).map_err(|e|e.to_string())?{retained.push(data);}}
  if let Some(s)=spawn{retained.push(source::initialize(s.player,s.center,s.velocity,&mut next.seed,math).map_err(|e|e.to_string())?);next.spawned+=1;}
  next.particles=retained;*self=next;Ok(())
 }
 pub fn visible(&self)->impl Iterator<Item=&[u8;32]>{self.particles.iter().filter(|d|source::drawable(d.as_slice()))}
}

#[derive(Clone,Copy,Debug)]pub struct Billboard{pub center:glam::Vec3,pub half_extent:f32,pub texture:usize}
pub fn billboard(data:&[u8])->Option<Billboard>{
 if !source::drawable(data){return None;}
 let age=i16::from_be_bytes(data[2..4].try_into().ok()?);if !(0..14).contains(&age){return None;}
 let value=|i|f32::from_be_bytes(data[i..i+4].try_into().unwrap());
 Some(Billboard{center:glam::Vec3::new(value(16),value(20),value(24))*0.1,half_extent:value(28)*1.6,texture:(age/2)as usize})
}
#[cfg(feature="renderer")]
pub mod renderer{
 use super::*;use macroquad::prelude::*;use macroquad::miniquad::{BlendState,BlendFactor,BlendValue,Equation,Comparison,CullFace};use sha2::{Sha256,Digest};
 pub struct FireRenderer{textures:Vec<Texture2D>,mesh:Mesh,material:Material,pub draws:u32}
 impl FireRenderer{
  pub fn new(dir:&std::path::Path)->Result<Self,String>{
   let dir=dir.join("effects");let manifest:serde_json::Value=serde_json::from_slice(&std::fs::read(dir.join("fire-somersault.json")).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
   let entries=manifest["textures"].as_array().ok_or("missing fire textures")?;if entries.len()!=7{return Err("invalid fire texture count".into());}let mut textures=Vec::new();
   for (i,e)in entries.iter().enumerate(){let name=format!("fire_ss{:02}.png",i+2);if e["file"]!=name||e["index"]!=i+28{return Err("invalid fire sequence".into());}let bytes=std::fs::read(dir.join(name)).map_err(|e|e.to_string())?;if e["sha256"]!=format!("{:x}",Sha256::digest(&bytes)){return Err("fire texture hash mismatch".into());}let texture=Texture2D::from_image(&Image::from_file_with_format(&bytes,None).map_err(|e|e.to_string())?);texture.set_filter(FilterMode::Linear);textures.push(texture);}
   let material=load_material(ShaderSource::Glsl{vertex:crate::aura::renderer::VERTEX,fragment:crate::aura::renderer::FRAGMENT},MaterialParams{pipeline_params:PipelineParams{depth_test:Comparison::LessOrEqual,depth_write:false,cull_face:CullFace::Nothing,color_blend:Some(BlendState::new(Equation::Add,BlendFactor::Value(BlendValue::SourceAlpha),BlendFactor::OneMinusValue(BlendValue::SourceAlpha))),..Default::default()},..Default::default()}).map_err(|e|e.to_string())?;
   Ok(Self{textures,material,mesh:Mesh{vertices:Vec::with_capacity(4),indices:vec![0,1,2,0,2,3],texture:None},draws:0})
  }
  pub fn draw(&mut self,fx:&FireEffects,eye:Vec3,target:Vec3){self.draw_with_up(fx,eye,target,Vec3::Y);}
  pub fn draw_with_up(&mut self,fx:&FireEffects,eye:Vec3,target:Vec3,camera_up:Vec3){
   self.draws=0;let forward=(target-eye).try_normalize().unwrap_or(Vec3::Z);let right=forward.cross(camera_up).try_normalize().unwrap_or(Vec3::X);let up=right.cross(forward).normalize();
   gl_use_material(&self.material);
   for data in fx.records(){let Some(b)=billboard(data)else{continue};self.mesh.texture=Some(self.textures[b.texture].clone());self.mesh.vertices.clear();
    for(x,y,u,v)in[(-1.,-1.,0.,1.),(1.,-1.,1.,1.),(1.,1.,1.,0.),(-1.,1.,0.,0.)]{let p=b.center+(right*x+up*y)*b.half_extent;self.mesh.vertices.push(Vertex::new(p.x,p.y,p.z,u,v,WHITE));}
    draw_mesh(&self.mesh);self.draws+=1;
   }gl_use_default_material();
  }
 }
}
