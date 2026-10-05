//! Experimental original geometry path. Original joint transitions; GX shading remains approximate.
use macroquad::prelude::*;
use serde::Deserialize;
use std::{collections::BTreeMap,path::Path};
use crate::{source_pose::SourcePose,draw_packets::{Primitive,assemble},simulation::{Player,State}};
#[derive(Deserialize)]struct Inventory{rigs:Vec<Plan>}
#[derive(Deserialize)]struct Plan{pose_index:usize,primitives:Vec<Primitive>}
type PoseKey=(u16,u32,u8,u32,Option<sa2_runtime::original_animation_update::BlendEndpoints>);
pub struct SourceRenderer{packets:std::cell::RefCell<Option<(PoseKey,Vec<crate::draw_packets::Packet>)>>,blend:std::cell::RefCell<Option<(sa2_runtime::original_animation_update::BlendEndpoints,crate::source_pose::PreparedTransition)>>,pose:SourcePose,plans:Vec<Plan>,textures:BTreeMap<(u32,u32,u32),Texture2D>,animations:BTreeMap<String,u16>,floor:f32}
fn wrap(value:u32)->Result<macroquad::miniquad::TextureWrap,String>{
 use macroquad::miniquad::TextureWrap;
 match value {0=>Ok(TextureWrap::Clamp),1=>Ok(TextureWrap::Repeat),2=>Ok(TextureWrap::Mirror),_=>Err("Invalid source texture wrapping".into())}
}
impl SourceRenderer{
 pub fn new(root:&Path,assets:&Path)->Result<Self,String>{
  let read=|p:&Path|std::fs::read(p).map_err(|e|e.to_string());
  let dir=root.join("movement-verification");
  let pose=SourcePose::from_math_bundle(&read(&dir.join("animation-bundle-audit.json"))?,read(&dir.join("animation-bundle-audit.json.bin"))?,read(&assets.join("original-math-data.bin"))?)?;
  let inventory:Inventory=serde_json::from_slice(&read(&dir.join("polygon-inventory.json"))?).map_err(|e|e.to_string())?;
  let manifest:serde_json::Value=serde_json::from_slice(&read(&assets.join("sonic.json"))?).map_err(|e|e.to_string())?;
  let mut animations=BTreeMap::new();for (name,value) in manifest["clips"].as_object().ok_or("Missing clips")?{animations.insert(name.clone(),value["source_animation"].as_u64().ok_or("Missing animation ID")? as u16);}animations.insert("bounce".into(),30);
  let floor=manifest["source_floor"].as_f64().ok_or("Missing floor")?as f32;
  let bindings:serde_json::Value=serde_json::from_slice(&read(&dir.join("draw-textures-result.json"))?).map_err(|e|e.to_string())?;
  let mut textures=BTreeMap::new();
  for binding in bindings["bindings"].as_array().ok_or("Missing texture bindings")?{
   let folder=if binding["pose_index"]==12 {assets.join("bounce")}else{assets.to_path_buf()};
   let file=binding["playable_files"][0]["file"].as_str().ok_or("Missing texture file")?;
   let image=Image::from_file_with_format(&read(&folder.join(file))?,None).map_err(|e|e.to_string())?;
   let id=binding["texture_id"].as_u64().ok_or("Missing texture ID")?as u32;
   let modes:std::collections::BTreeSet<_>=inventory.rigs.iter().flat_map(|r|r.primitives.iter()).filter_map(|p|p.material_state.texture.as_ref()).filter(|t|t.id==id).map(|t|(t.wrap_u,t.wrap_v)).collect();
   for (u,v) in modes {
    let texture=Texture2D::from_image(&image);texture.set_filter(FilterMode::Linear);
    let handle=texture.raw_miniquad_id();let wu=wrap(u)?;let wv=wrap(v)?;
    // Startup-only backend access; no references survive this call. Distinct
    // variants prevent later sampler changes from altering queued mesh draws.
    unsafe {macroquad::window::get_internal_gl().quad_context.texture_set_wrap(handle,wu,wv);}
    textures.insert((id,u,v),texture);
   }
  }
  Ok(Self{packets:std::cell::RefCell::new(None),blend:std::cell::RefCell::new(None),pose,plans:inventory.rigs,textures,animations,floor})
 }
 /// Prepare local geometry; true means the pose was rebuilt, false a cache hit.
 pub fn prepare_pose(&self,p:&Player)->Result<bool,String>{
  let clock=p.source_animation_clock();
  let id=if let Some(clock)=clock{
   match p.source_draw_selection(){sa2_runtime::original_sonic_draw::Selection::SpinModel=>30,_=>clock.output().current}
  }else{*self.animations.get(p.clip()).ok_or("Unmapped animation clip")?};
  let(model,last)=self.pose.model_and_last_frame(id)?;
  let action=if p.state==State::Run{1}else{0};
  let frame=clock.map(|c|c.output().frame).unwrap_or((p.animation_frame as f32).clamp(0.,last));
  let key=(id,frame.to_bits(),action,p.source_lean(),clock.and_then(|c|c.blend()));
  let mut packet_cache=self.packets.borrow_mut();
  if packet_cache.as_ref().map(|(key,_)|*key)!=Some(key){
  let mut cached=self.blend.borrow_mut();
  let output=if let Some(endpoints)=clock.and_then(|c|c.blend()){
   if cached.as_ref().map(|(key,_)|*key)!=Some(endpoints){
    let prepared=self.pose.prepare_transition(endpoints.from_animation,endpoints.from_frame,endpoints.to_animation,endpoints.to_frame)?;
    *cached=Some((endpoints,prepared));
   }
   cached.as_ref().unwrap().1.sample(clock.unwrap().output().frame,action,p.source_lean())?
  }else{
   *cached=None;
   let frame=clock.map(|c|c.output().frame).unwrap_or((p.animation_frame as f32).clamp(0.,last));
   if p.source_draw_selection()==sa2_runtime::original_sonic_draw::Selection::SpinModel{
    self.pose.sample_spin_override(clock.ok_or("Missing override clock")?.output().current,frame,action,p.source_lean())?
   }else{self.pose.sample(id,frame,action,p.source_lean())?}
  };
  let plan=self.plans.iter().find(|r|r.pose_index==if model==6{12}else{0}).ok_or("Missing draw plan")?;
  let packets=assemble(&plan.primitives,&output.meshes)?;
  *packet_cache=Some((key,packets));
  return Ok(true);
  }
  Ok(false)
 }
 pub fn draw(&self,p:&Player)->Result<(),String>{
  self.prepare_pose(p)?;
  let packet_cache=self.packets.borrow();
  let packets=&packet_cache.as_ref().unwrap().1;
  let rotation=p.model_rotation();let light=vec3(-0.35,0.85,-0.4).normalize();
  for packet in packets{
   if packet.vertices.len()>=crate::assets::DRAW_VERTICES || packet.indices.len()>=crate::assets::DRAW_INDICES{return Err("Source draw exceeds capacity".into());}
   let texture=packet.texture.as_ref().map(|t|self.textures.get(&(t.id,t.wrap_u,t.wrap_v)).cloned().ok_or("Unbound texture sampler")).transpose()?;
   let mut vertices=Vec::with_capacity(packet.vertices.len());
   for v in &packet.vertices{
    let pos=rotation*vec3(v.position[2]*0.1,(v.position[1]-self.floor)*0.1,-v.position[0]*0.1)+p.position;
    let normal=rotation*vec3(v.normal[2],v.normal[1],-v.normal[0]).normalize_or_zero();
    let l=0.67+0.33*normal.dot(light).max(0.);
    // Source identity reflection matrix; draw-time normal space is still under investigation.
    let uv=if packet.generated_uv{[0.5-0.5*v.normal[0],0.5-0.5*v.normal[1]]}else{v.uv};
    let c=packet.color;vertices.push(Vertex::new(pos.x,pos.y,pos.z,uv[0],uv[1],Color::new(c[0]as f32/255.*l,c[1]as f32/255.*l,c[2]as f32/255.*l,c[3]as f32/255.)));
   }
   draw_mesh(&Mesh{vertices,indices:packet.indices.iter().map(|i|*i as u16).collect(),texture});
  }Ok(())
 }
 /// Diagnostic: compare reuse to a fresh pose and fresh transition workspace.
 pub fn verify_cached_pose(&self,p:&Player)->Result<(),String>{
  self.prepare_pose(p)?;
  if self.prepare_pose(p)?{return Err("unchanged pose missed cache".into());}
  let old=self.packets.borrow_mut().take().ok_or("missing prepared packets")?;
  *self.blend.borrow_mut()=None;
  self.prepare_pose(p)?;
  let fresh=self.packets.borrow();
  if old!=*fresh.as_ref().ok_or("missing rebuilt packets")?{return Err("cached pose differs from fresh reconstruction".into());}
  Ok(())
 }
}
