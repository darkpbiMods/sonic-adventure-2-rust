use macroquad::prelude::*;
use serde::Deserialize;
use sha2::{Digest,Sha256};
use std::{collections::BTreeMap,path::Path};
pub const DRAW_VERTICES:usize=16384;
pub const DRAW_INDICES:usize=32768;
#[derive(Deserialize)]
pub struct Group {pub texture:String,pub vertices:Vec<(usize,f32,f32)>,pub indices:Vec<u16>}
#[derive(Deserialize)]
pub struct ClipInfo {pub file:String,pub frames:usize,pub sha256:String,pub source_animation:u16,pub blend_per_tick:f32}
#[derive(Deserialize)]
pub struct Manifest {pub vertex_count:usize,pub groups:Vec<Group>,pub clips:BTreeMap<String,ClipInfo>}
pub struct Clip {pub frames:usize,pub data:Vec<[f32;6]>}
pub struct Assets {pub manifest:Manifest,pub clips:BTreeMap<String,Clip>}
impl Assets {
 pub fn read(dir:&Path)->Result<Self,String>{
  Self::read_required(dir,&["idle","walk","jog","run","sprint","spin","roll","fall","land","brake","turn","jump","spin_jump","jump_dash","rebound","rebound2","rebound3","bounce_fall_start","bounce_fall"])
 }
 pub fn read_bounce(dir:&Path)->Result<Self,String>{Self::read_required(dir,&["bounce"])}
 fn read_required(dir:&Path,required:&[&str])->Result<Self,String>{
  let manifest:Manifest=serde_json::from_slice(&std::fs::read(dir.join("sonic.json")).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
  if manifest.vertex_count==0 || manifest.vertex_count>100_000{return Err("Invalid vertex count".into());}
  for g in &manifest.groups{
   if g.vertices.len()>=DRAW_VERTICES || g.indices.len()>=DRAW_INDICES{return Err("Mesh exceeds renderer draw capacity; refusing to truncate geometry".into());}
   if g.indices.len()%3!=0 || g.indices.iter().any(|&x|x as usize>=g.vertices.len()) || g.vertices.iter().any(|&(i,u,v)|i>=manifest.vertex_count || !u.is_finite() || !v.is_finite()){return Err("Invalid mesh topology".into());}
   if !dir.join(&g.texture).is_file(){return Err(format!("Missing texture {}",g.texture));}
  }
  let mut clips=BTreeMap::new();
  for (name,info) in &manifest.clips{
   if info.source_animation>=203 || !info.blend_per_tick.is_finite() || info.blend_per_tick<=0. || info.blend_per_tick>1.{return Err(format!("Invalid animation transition {name}"));}
   let bytes=std::fs::read(dir.join(&info.file)).map_err(|e|e.to_string())?;
   if bytes.len()<12 || &bytes[..4]!=b"SA2V" || format!("{:x}",Sha256::digest(&bytes))!=info.sha256{return Err(format!("Corrupt animation {name}"));}
   let n=u32::from_le_bytes(bytes[4..8].try_into().unwrap())as usize;let frames=u32::from_le_bytes(bytes[8..12].try_into().unwrap())as usize;
   if n!=manifest.vertex_count || frames!=info.frames || frames==0 || frames>1000 || bytes.len()!=12+n*frames*24{return Err(format!("Invalid animation length {name}"));}
   let mut data=Vec::new();
   for v in bytes[12..].chunks_exact(24){let mut a=[0.;6];for i in 0..6{a[i]=f32::from_le_bytes(v[i*4..i*4+4].try_into().unwrap());}if a.iter().any(|x|!x.is_finite()){return Err("Nonfinite mesh data".into());}data.push(a);}
   clips.insert(name.clone(),Clip{frames,data});
  }
  for &name in required {if !clips.contains_key(name){return Err(format!("Missing required clip {name}"));}}
  Ok(Self{manifest,clips})
 }
 pub fn pose(&self,clip:&str,time:f32,rate:f32)->Vec<[f32;6]>{
  self.pose_frame(clip,time as f64*rate as f64)
 }
 pub fn pose_frame(&self,clip:&str,frame:f64)->Vec<[f32;6]>{
  let c=&self.clips[clip];let n=self.manifest.vertex_count;
  let clamped=matches!(clip,"land"|"jump"|"rebound"|"rebound2"|"rebound3"|"bounce_fall_start");
  // Original mode3 excludes the final duplicate endpoint from its loop period.
  let period=if matches!(clip,"spin_jump"|"jump_dash"|"bounce"|"bounce_fall"){c.frames.saturating_sub(1).max(1)}else{c.frames};
  let f=if clamped{frame.clamp(0.,c.frames as f64-1.)}else{frame.rem_euclid(period as f64)};
  let a=f as usize;let b=if clamped{(a+1).min(c.frames-1)}else{(a+1)%c.frames};let t=f.fract() as f32;
  (0..n).map(|i|{let mut v=[0.;6];for k in 0..6{v[k]=c.data[a*n+i][k]*(1.-t)+c.data[b*n+i][k]*t;}v}).collect()
 }
}
pub struct Renderer {normal:ModelRenderer,bounce:ModelRenderer,was_bounce:bool}
impl Renderer {
 pub fn new(dir:&Path)->Result<Self,String>{
  Ok(Self{normal:ModelRenderer::new(dir,Assets::read(dir)?,"idle")?,bounce:ModelRenderer::new(&dir.join("bounce"),Assets::read_bounce(&dir.join("bounce"))?,"bounce")?,was_bounce:false})
 }
 pub fn draw(&mut self,p:&crate::simulation::Player,dt:f32){
  let bounce=p.clip()=="bounce";let changed=bounce!=self.was_bounce;self.was_bounce=bounce;
  let model=if bounce{&mut self.bounce}else{&mut self.normal};
  // Different rigs/topologies cannot be interpolated vertex by vertex.
  if changed{model.pose=model.assets.pose_frame(p.clip(),p.animation_frame);model.from=model.pose.clone();model.last_clip=p.clip().into();model.transition=1.;}
  model.draw(p,dt);
 }
}
struct ModelRenderer {assets:Assets,meshes:Vec<Mesh>,pose:Vec<[f32;6]>,last_clip:String,transition:f32,from:Vec<[f32;6]>}
impl ModelRenderer {
 fn new(dir:&Path,assets:Assets,initial:&str)->Result<Self,String>{
  let mut meshes=Vec::new();
  for g in &assets.manifest.groups {
   let bytes=std::fs::read(dir.join(&g.texture)).map_err(|e|e.to_string())?;
   let im=Image::from_file_with_format(&bytes,None).map_err(|e|e.to_string())?;
   let texture=Texture2D::from_image(&im);texture.set_filter(FilterMode::Linear);
   meshes.push(Mesh{vertices:g.vertices.iter().map(|&(_,u,v)|Vertex::new(0.,0.,0.,u,v,WHITE)).collect(),indices:g.indices.clone(),texture:Some(texture)});
  }
  let pose=assets.pose(initial,0.,30.);let from=pose.clone();
  Ok(Self{assets,meshes,pose,last_clip:initial.into(),transition:1.,from})
 }
 pub fn draw(&mut self,p:&crate::simulation::Player,dt:f32){
  let clip=p.clip();if clip!=self.last_clip{
   let direct=matches!((self.last_clip.as_str(),clip),("jump","spin_jump")|("bounce_fall_start","bounce_fall"));
   self.from=self.pose.clone();self.transition=if direct{1.}else{0.};self.last_clip=clip.into();
  }
  self.transition=p.animation_blend().unwrap_or_else(||(self.transition+dt*60.*self.assets.manifest.clips[clip].blend_per_tick).min(1.));
  self.pose=self.assets.pose_frame(clip,p.animation_frame);
  let rot=p.model_rotation();
  let light=vec3(-0.35,0.85,-0.4).normalize();
  for (i,v) in self.pose.iter_mut().enumerate(){for(k,x)in v.iter_mut().enumerate(){*x=self.from[i][k]*(1.-self.transition)+*x*self.transition;}}
  for(g,m)in self.assets.manifest.groups.iter().zip(&mut self.meshes){
   for ((&(i,_,_),v),_)in g.vertices.iter().zip(&mut m.vertices).zip(0..){
    let a=self.pose[i];let pos=rot*vec3(a[0],a[1],a[2])+p.position;
    let normal=rot*vec3(a[3],a[4],a[5]).normalize_or_zero();let l=0.67+0.33*normal.dot(light).max(0.);
    v.position=pos;v.color=Color::new(l,l,l,1.).into();
   }
   draw_mesh(m);
  }
 }
}
