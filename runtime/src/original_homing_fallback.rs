//! Homing fallback force branches; target selection and decision dispatch are external.
use crate::{disc::Result,original_ground_accel::{accelerate,GroundContext},original_contact::Memory,original_math::OriginalMath,original_movement::integrate_velocity,original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES}};
/// Positive-counter branch8007b350..38c.
pub fn advance(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],ctx:&GroundContext,memory:&Memory,cache:&mut[u32;4],math:&OriginalMath)->Result<()>{run(entity,motion,character,ctx,memory,cache,math,false)}
/// No selected candidate branch8007b574..59c; first update has no extra damping.
pub fn begin_selection_failure(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],ctx:&GroundContext,memory:&Memory,cache:&mut[u32;4],math:&OriginalMath)->Result<()>{run(entity,motion,character,ctx,memory,cache,math,true)}
fn run(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],ctx:&GroundContext,memory:&Memory,cache:&mut[u32;4],math:&OriginalMath,begin:bool)->Result<()>{
 if entity.len()!=ENTITY_BYTES||motion.len()!=0x40||character.len()!=CHARACTER_BYTES{return Err("invalid homing fallback extent".into());}
 let counter=i16::from_be_bytes(character[0x364..0x366].try_into().unwrap());
 if !begin&&counter<0{return Err("nonnegative homing fallback counter required".into());}
 let(mut e,mut m,mut c,mut next_cache)=(entity.to_vec(),motion.to_vec(),character.to_vec(),*cache);
 c[0x364..0x366].copy_from_slice(&(if begin{1}else{counter.wrapping_add(1)}).to_be_bytes());
 if !begin{
 let speed=f32::from_be_bytes(c[0x64..0x68].try_into().unwrap())*math.constant(0x803af698);
 c[0x64..0x68].copy_from_slice(&speed.to_be_bytes());
 }
 accelerate(&mut e,&mut m,&mut c,ctx,memory,&mut next_cache,math)?;
 integrate_velocity(&e,&mut m,&mut c,ctx.camera.control.magnitudes,math)?;
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);*cache=next_cache;Ok(())
}

/// No-contact timeout block80081220..27c. Returns true when action exits.
pub fn cancel_action(entity:&mut[u8],character:&mut[u8])->Result<()>{
 if entity.len()!=ENTITY_BYTES||character.len()!=CHARACTER_BYTES{return Err("invalid homing cancellation extent".into());}
 character[0x364..0x366].fill(0);entity[0]=10;
 character[0x176..0x178].copy_from_slice(&15u16.to_be_bytes());
 let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap())&!0x500;
 entity[4..6].copy_from_slice(&flags.to_be_bytes());character[0x35e]=0;Ok(())
}
/// No-contact timeout block80081220..27c. Returns true when action exits.
/// Contact queries must precede this; input/landing decisions follow on false.
pub fn expire_no_contact(entity:&mut[u8],character:&mut[u8])->Result<bool>{
 if entity.len()!=ENTITY_BYTES||character.len()!=CHARACTER_BYTES{return Err("invalid homing decision extent".into());}
 let fallback=i16::from_be_bytes(character[0x364..0x366].try_into().unwrap());
 let total=i16::from_be_bytes(character[0x366..0x368].try_into().unwrap());
 let expired=if fallback>=15{true}else{
  character[0x366..0x368].copy_from_slice(&total.wrapping_add(1).to_be_bytes());total>=360
 };
 if expired{
  character[0x364..0x366].copy_from_slice(&0i16.to_be_bytes());entity[0]=10;
  if i16::from_be_bytes(character[0x176..0x178].try_into().unwrap())==100{character[0x176..0x178].copy_from_slice(&15i16.to_be_bytes());}
  let status=u16::from_be_bytes(entity[4..6].try_into().unwrap())&!0x500;
  entity[4..6].copy_from_slice(&status.to_be_bytes());character[0x35e]=0;
 }
 Ok(expired)
}

/// Initial target-vector vertical branch8007b67c..6c4, before normalization.
/// Inputs/outputs use original SA units; caller retains horizontal components.
pub fn initial_vertical(delta_y:f32,gravity_y:f32,counter:i16,special_stage:bool)->Result<(f32,i16)>{
 if !delta_y.is_finite()||!gravity_y.is_finite(){return Err("nonfinite homing vertical input".into());}
 if delta_y*gravity_y<0. && (!special_stage||delta_y>=20.) && counter<0{Ok((0.,1))}else{Ok((delta_y,counter))}
}

/// Post-initializer action transition80080218..24c for ordinary airborne homing.
pub fn enter_action(entity:&mut[u8],character:&mut[u8])->Result<()>{
 if entity.len()!=ENTITY_BYTES||character.len()!=CHARACTER_BYTES{return Err("invalid homing startup extent".into());}
 let counter=i16::from_be_bytes(character[0x364..0x366].try_into().unwrap());
 if counter>0{character[0x176..0x178].copy_from_slice(&100i16.to_be_bytes());}
 else{let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap())|0x500;entity[4..6].copy_from_slice(&flags.to_be_bytes());}
 entity[0]=13;Ok(())
}

/// Category0 world impulse from80081148..11d0. Orientation setter is separate.
pub fn category0_world_impulse(velocity:[f32;3],math:&OriginalMath)->Result<[f32;3]>{
 if velocity.iter().any(|v|!v.is_finite()){return Err("nonfinite homing impact velocity".into());}
 let (_,normal)=math.estimated_vector([velocity[0],0.,velocity[2]])?;
 Ok(match normal{Some(n)=>[n[0]*-4.,2.5,n[2]*-4.],None=>velocity})
}

/// Original800810c0 contact continuation for categories2/3. World/local
/// velocities and orientation are unchanged; category0 requires its own setter.
pub fn contact_transition(entity:&mut[u8],character:&mut[u8],category:u16)->Result<()>{
 if entity.len()!=ENTITY_BYTES||character.len()!=CHARACTER_BYTES{return Err("invalid homing contact extent".into());}
 if !matches!(category,2|3){return Err("unsupported homing contact category".into());}
 let cycle=character[0x35e];
 if cycle>2{return Err("unsupported homing rebound cycle".into());}
 entity[0]=6;
 let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap())&!0x100;
 entity[4..6].copy_from_slice(&flags.to_be_bytes());
 character[0x364..0x366].copy_from_slice(&(-1i16).to_be_bytes());
 character[0x176..0x178].copy_from_slice(&(101u16+cycle as u16).to_be_bytes());
 character[0x35e]=(cycle+1)%3;Ok(())
}

/// Category0 contact including direction setter and world/local impulse.
pub fn category0_contact(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],math:&OriginalMath)->Result<()>{
 if motion.len()!=0x40{return Err("invalid homing contact motion extent".into());}
 let(mut e,mut m,mut c)=(entity.to_vec(),motion.to_vec(),character.to_vec());
 contact_transition(&mut e,&mut c,2)?;
 let velocity:[f32;3]=std::array::from_fn(|i|f32::from_be_bytes(m[i*4..i*4+4].try_into().unwrap()));
 if velocity.iter().any(|x|!x.is_finite()){return Err("nonfinite contact velocity".into());}
 if let Some(normal)=math.estimated_vector([velocity[0],0.,velocity[2]])?.1{
  let angles=math.direction_angles(normal)?;
  for i in 0..3{e[8+i*4..12+i*4].copy_from_slice(&angles[i].to_be_bytes());}
  m[0x1c..0x20].copy_from_slice(&angles[1].to_be_bytes());
  let world=[normal[0]*-4.,2.5,normal[2]*-4.];
  let mut local=crate::original_ground::rotate_vector(&e,world,false,math)?;
  for i in [0,2]{if local[i]<=math.constant(0x803b6520-0x758c)&&local[i]>=math.constant(0x803b6520-0x7588){local[i]=0.;}}
  for i in 0..3{m[i*4..i*4+4].copy_from_slice(&world[i].to_be_bytes());c[0x64+i*4..0x68+i*4].copy_from_slice(&local[i].to_be_bytes());}
 }
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);Ok(())
}

#[cfg(test)]mod contact_tests{
 use super::*;
 #[test]fn category0_complete_records_match_original(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let math=OriginalMath::from_dol(std::fs::read(root.join("source/main.dol")).unwrap()).unwrap();
  let data:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/homing-category0-result.json")).unwrap()).unwrap();
  let bytes=|v:&serde_json::Value|v.as_array().unwrap().iter().map(|x|x.as_u64().unwrap()as u8).collect::<Vec<_>>();
  let rows=data["rows"].as_array().unwrap();assert_eq!(rows.len(),45);
  for r in rows{
   let(mut e,mut m,mut c)=(bytes(&r["before"]["entity"]),bytes(&r["before"]["motion"]),bytes(&r["before"]["character"]));
   category0_contact(&mut e,&mut m,&mut c,&math).unwrap();
   assert_eq!(e,bytes(&r["after"]["entity"]));assert_eq!(m,bytes(&r["after"]["motion"]));assert_eq!(c,bytes(&r["after"]["character"]));
  }
 }
 #[test]fn category0_invalid_input_is_transactional(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let math=OriginalMath::from_dol(std::fs::read(root.join("source/main.dol")).unwrap()).unwrap();
  for invalid_cycle in [false,true]{
   let mut e=[0u8;0x40];let mut m=[0u8;0x40];let mut c=[0u8;0x3a0];e[0]=13;
   if invalid_cycle{c[0x35e]=3;}else{m[..4].copy_from_slice(&f32::NAN.to_be_bytes());}
   let before=(e,m,c);assert!(category0_contact(&mut e,&mut m,&mut c,&math).is_err());assert_eq!((e,m,c),before);
  }
 }
 #[test]fn direction_setter_matches_original_contact_angles(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let math=OriginalMath::from_dol(std::fs::read(root.join("source/main.dol")).unwrap()).unwrap();
  let data:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/homing-category0-result.json")).unwrap()).unwrap();
  let rows=data["rows"].as_array().unwrap();assert_eq!(rows.len(),45);
  for r in rows{
   let v=[r["velocity"][0].as_f64().unwrap()as f32,0.,r["velocity"][2].as_f64().unwrap()as f32];
   let angles=if let Some(normal)=math.estimated_vector(v).unwrap().1{math.direction_angles(normal).unwrap()}else{std::array::from_fn(|i|r["initial_angles"][i].as_u64().unwrap()as u32)};
   let expected:[u32;3]=std::array::from_fn(|i|r["angles"][i].as_u64().unwrap()as u32);
   assert_eq!(angles,expected,"velocity {v:?}");
  }
 }
 #[test]fn contact_records_match_original_cases(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let data:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/homing-contact-result.json")).unwrap()).unwrap();
  assert_eq!(data["passed"],true);let rows=data["rows"].as_array().unwrap();assert_eq!(rows.len(),12);
  for r in rows{
   let mut e=[0u8;0x40];let mut c=[0u8;0x3a0];e[0]=13;e[4..6].copy_from_slice(&0x5ffu16.to_be_bytes());
   c[0x35e]=r["cycle"].as_u64().unwrap()as u8;c[0x364..0x366].copy_from_slice(&7i16.to_be_bytes());
   for i in 0..3{c[0x64+i*4..0x68+i*4].copy_from_slice(&(r["velocity"][i].as_f64().unwrap()as f32).to_be_bytes());}
   let(mut expected_e,mut expected_c)=(e,c);expected_e[0]=6;expected_e[4..6].copy_from_slice(&0x4ffu16.to_be_bytes());
   expected_c[0x364..0x366].copy_from_slice(&(-1i16).to_be_bytes());expected_c[0x176..0x178].copy_from_slice(&(r["animation"].as_u64().unwrap()as u16).to_be_bytes());expected_c[0x35e]=(expected_c[0x35e]+1)%3;
   contact_transition(&mut e,&mut c,r["category"].as_u64().unwrap()as u16).unwrap();assert_eq!(e,expected_e);assert_eq!(c,expected_c);
  }
  for (category,cycle) in [(0,0),(4,0),(2,3)]{let mut e=[0x55;0x40];let mut c=[0x66;0x3a0];c[0x35e]=cycle;let before=(e,c);assert!(contact_transition(&mut e,&mut c,category).is_err());assert_eq!((e,c),before);}
 }
}
