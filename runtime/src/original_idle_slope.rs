//! Eligible slope-idle pose branch80055048..5511c; caller owns slope eligibility.
use crate::{disc::Result,original_math::OriginalMath};
pub fn select(entity:&[u8],motion:&[u8],character:&mut[u8],gravity:[f32;3],math:&OriginalMath)->Result<()>{
 if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0{return Err("invalid slope idle records".into());}
 let word=|b:&[u8],i|u32::from_be_bytes(b[i..i+4].try_into().unwrap());
 let normal=std::array::from_fn(|i|f32::from_bits(word(character,0x7c+i*4)));
 if !normal.iter().chain(gravity.iter()).all(|v|v.is_finite()){return Err("nonfinite idle surface".into());}
 let selected=if crate::original_gameplay::angle_distance(word(motion,0x1c),word(entity,12))>0x200{6}else{
  let forward=crate::original_ground::rotate_vector(entity,[math.constant(0x803b6520-0x73d8),0.,0.],true,math)?;
  let plane=[0.,0.,0.,normal[0],normal[1],normal[2]];
  let project=|v|->Result<[f32;3]>{
   let p=crate::original_geometry::point_plane(v,plane,true,math)?.1.ok_or("missing idle projection")?;
   math.estimated_vector(p)?.1.ok_or("degenerate idle projection".into())
  };
  let angle=crate::original_geometry::signed_angle(project(forward)?,project(gravity)?,Some(normal),math)?;
  if !(0x4000..=0xc000).contains(&angle){5}else{4}
 };
 character[0x176..0x178].copy_from_slice(&(selected as u16).to_be_bytes());
 if selected==6{character[0x44..0x48].copy_from_slice(&math.constant(0x803b6520-0x73b0).to_be_bytes());}
 Ok(())
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn matches_original_eligible_slope_records(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let math=OriginalMath::from_data_bundle(std::fs::read(root.join("playable/assets/original-math-data.bin")).unwrap()).unwrap();
  let data:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/idle-slopes-result.json")).unwrap()).unwrap();
  let bytes=|v:&serde_json::Value|v.as_array().unwrap().iter().map(|x|x.as_u64().unwrap()as u8).collect::<Vec<_>>();
  let mut count=0;
  for row in data["observations"].as_array().unwrap(){
   if row["normal"][1].as_f64().unwrap()!=0.8{continue;}
   let mut c=bytes(&row["before"]);select(&bytes(&row["entity"]),&bytes(&row["motion"]),&mut c,[0.,-1.,0.],&math).unwrap();
   // Full routine also executes the counter tail, outside this pose branch.
   c[0x18..0x1a].copy_from_slice(&1u16.to_be_bytes());
   assert_eq!(c,bytes(&row["after"]));count+=1;
  }
  assert_eq!(count,12);
 }
}
/// Outer normal/slope/retain branch80054d88..80055048.
#[derive(Debug,PartialEq,Eq)]
pub enum SurfaceIdle{Normal,Slope,Retain}
pub fn classify(normal:[f32;3],gravity:[f32;3],flags:u32,math:&OriginalMath)->Result<SurfaceIdle>{
 if !normal.iter().chain(gravity.iter()).all(|v|v.is_finite()){return Err("nonfinite idle surface".into());}
 let dot=normal[2].mul_add(gravity[2],normal[1].mul_add(gravity[1],normal[0]*gravity[0]));
 if !dot.is_finite(){return Err("idle surface dot overflow".into());}
 let cosine=|index:u32|->Result<f32>{
  let product=math.constant(0x803b6520-0x55f4)*((index*2)as f32);
  Ok(math.sin_cos(product/math.constant(0x803b6520-0x55f0))?.1)
 };
 Ok(if -dot>cosine(0x11c7)?||flags&0x100!=0{SurfaceIdle::Normal}else if -dot>cosine(0x2000)?{SurfaceIdle::Slope}else{SurfaceIdle::Retain})
}
#[cfg(test)]mod classification_tests{
 use super::*;
 #[test]fn matches_original_surface_boundaries(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let math=OriginalMath::from_data_bundle(std::fs::read(root.join("playable/assets/original-math-data.bin")).unwrap()).unwrap();
  let data:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/idle-surface-result.json")).unwrap()).unwrap();
  for row in data["observations"].as_array().unwrap(){
   let normal=std::array::from_fn(|i|row["normal"][i].as_f64().unwrap()as f32);
   let actual=classify(normal,[0.,-1.,0.],row["flags"].as_u64().unwrap()as u32,&math).unwrap();
   assert_eq!(format!("{actual:?}"),row["expected"].as_str().unwrap());
  }
  assert!(classify([f32::NAN,1.,0.],[0.,-1.,0.],0,&math).is_err());
  assert!(classify([f32::MAX;3],[f32::MAX;3],0,&math).is_err());
 }
}
/// Complete Sonic idle record update for verified surface/context inputs.
/// External contextual output is signalled by the return value, not executed.
pub fn advance(entity:&[u8],motion:&[u8],character:&mut[u8],gravity:[f32;3],frame:u32,seed:&mut u32,context_draw:bool,math:&OriginalMath)->Result<bool>{
 if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0{return Err("invalid idle records".into());}
 if character[1]!=0{return Err("idle adapter requires Sonic kind0".into());}
 let selected=u16::from_be_bytes(character[0x176..0x178].try_into().unwrap());
 if matches!(selected,16|17|26|27|63|64){return Ok(false);}
 let mut next=character.to_vec();let mut next_seed=*seed;
 let normal=std::array::from_fn(|i|f32::from_be_bytes(next[0x7c+i*4..0x80+i*4].try_into().unwrap()));
 let flags=u32::from_be_bytes(next[0x88..0x8c].try_into().unwrap());
 let branch=classify(normal,gravity,flags,math)?;
 let mut idle=crate::original_idle_selection::IdleState{selected,counter:i16::from_be_bytes(next[0x18..0x1a].try_into().unwrap())};
 let entered=if branch==SurfaceIdle::Normal{
  let entered=crate::original_idle_selection::advance(&mut idle,&mut next_seed,frame,context_draw);
  next[0x176..0x178].copy_from_slice(&idle.selected.to_be_bytes());
  if entered{next[0x17c..0x17e].fill(0);}entered
 }else{
  if branch==SurfaceIdle::Slope{select(entity,motion,&mut next,gravity,math)?;}
  let selected=u16::from_be_bytes(next[0x176..0x178].try_into().unwrap());
  if !matches!(selected,88..=90)||frame&1!=0{idle.counter=idle.counter.wrapping_add(1);}
  false
 };
 next[0x18..0x1a].copy_from_slice(&idle.counter.to_be_bytes());
 character.copy_from_slice(&next);*seed=next_seed;Ok(entered)
}
#[cfg(test)]mod combined_tests{
 use super::*;
 #[test]fn complete_surface_records_and_error_rollback(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let math=OriginalMath::from_data_bundle(std::fs::read(root.join("playable/assets/original-math-data.bin")).unwrap()).unwrap();
  let data:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/idle-slopes-result.json")).unwrap()).unwrap();
  let bytes=|v:&serde_json::Value|v.as_array().unwrap().iter().map(|x|x.as_u64().unwrap()as u8).collect::<Vec<_>>();
  for row in data["observations"].as_array().unwrap(){
   let mut c=bytes(&row["before"]);let mut seed=123;
   assert!(!advance(&bytes(&row["entity"]),&bytes(&row["motion"]),&mut c,[0.,-1.,0.],0,&mut seed,true,&math).unwrap());
   assert_eq!(c,bytes(&row["after"]));assert_eq!(seed,123);
  }
  let mut c=vec![0;0x3a0];c[0x80..0x84].copy_from_slice(&f32::NAN.to_be_bytes());let before=c.clone();let mut seed=42;
  assert!(advance(&[0;0x40],&[0;0x40],&mut c,[0.,-1.,0.],0,&mut seed,true,&math).is_err());
  assert_eq!(c,before);assert_eq!(seed,42);
 }
}
