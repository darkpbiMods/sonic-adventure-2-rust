//! Original ground path steering80038f40..80039484.
use crate::{disc::Result,original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES,angle_distance,angle_approach},
 original_math::OriginalMath,original_contact::Memory,original_path::{nearest,NearestOutputs},
 original_path_sample::sample,original_ground::rotate_vector,original_geometry::{point_plane,signed_angle}};
fn word(b:&[u8],o:usize)->u32{u32::from_be_bytes(b[o..o+4].try_into().unwrap())}
fn float(b:&[u8],o:usize)->f32{f32::from_bits(word(b,o))}
fn vector(b:&[u8],o:usize)->[f32;3]{std::array::from_fn(|i|float(b,o+i*4))}
fn put(b:&mut[u8],o:usize,v:u32){b[o..o+4].copy_from_slice(&v.to_be_bytes());}
fn putv(b:&mut[u8],o:usize,v:[f32;3]){for i in 0..3{put(b,o+i*4,v[i].to_bits());}}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],a[0]*b[0]))}
fn cross(a:[f32;3],b:[f32;3])->[f32;3]{[b[2].mul_add(a[1],-(b[1]*a[2])),b[0].mul_add(a[2],-(b[2]*a[0])),b[1].mul_add(a[0],-(b[0]*a[1]))]}
// Extend the finite helper for NaNs generated inside this routine. The source
// normalizes zero without a guard (frsqrte(0)=infinity; 0*infinity=NaN). These
// intermediate payloads never escape the parent; unordered comparisons and
// fctiwz behavior do. Initial nonfinite path inputs remain outside this domain.
fn normalize(v:[f32;3],math:&OriginalMath)->Result<[f32;3]>{
 if v.iter().any(|x|x.is_nan()){return Ok([f32::NAN;3]);}
 Ok(math.estimated_vector(v)?.1.unwrap_or([f32::NAN;3]))
}
fn length(v:[f32;3],math:&OriginalMath)->Result<f64>{
 if v.iter().any(|x|x.is_nan()){Ok(f64::NAN)}else{Ok(math.estimated_vector(v)?.0)}
}
fn project(v:[f32;3],plane:[f32;6],math:&OriginalMath)->Result<[f32;3]>{
 if v.iter().any(|x|x.is_nan()){Ok([f32::NAN;3])}else{Ok(point_plane(v,plane,true,math)?.1.unwrap())}
}
fn path_angle(a:[f32;3],b:[f32;3],normal:[f32;3],math:&OriginalMath)->Result<u32>{
 // Both source ordered clamp tests fail for an unordered dot. acos remains
 // NaN, fctiwz yields INT_MIN, and the unordered axis test does not negate it.
 if dot(a,b).is_nan(){Ok(i32::MIN as u32)}else{signed_angle(a,b,Some(normal),math)}
}
fn clear(e:&mut[u8]){let v=u16::from_be_bytes(e[4..6].try_into().unwrap())&!0x2000;e[4..6].copy_from_slice(&v.to_be_bytes());}
/// Scratch is the caller's preexisting stack sample record, not fabricated game
/// state. Nearest/sample can leave fields untouched on empty/invalid paths.
pub struct PathContext{pub magnitudes:[f32;2],pub incoming_f28:f64,pub incoming_f30:f64,pub sample_scratch:[u8;0x44]}
pub fn steer(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],memory:&Memory,ctx:&PathContext,math:&OriginalMath)->Result<()>{
 if entity.len()!=ENTITY_BYTES||motion.len()!=0x40||character.len()!=CHARACTER_BYTES{return Err("invalid ground-path state extent".into());}
 let player=character[0]as i8;if !(0..2).contains(&player){return Err("invalid ground-path player".into());}
 let mut e=entity.to_vec();let mut m=motion.to_vec();let mut c=character.to_vec();
 let k=|o:u32|math.constant(0x803b6520-o);let zero=k(0x75f8);
 let magnitude=ctx.magnitudes[player as usize];let threshold=k(0x759c)*float(&c,0xf4);
 if !magnitude.is_finite()||!threshold.is_finite()||!float(&c,0x64).is_finite(){return Err("nonfinite ground-path input".into());}
 if magnitude==zero{clear(&mut e);entity.copy_from_slice(&e);return Ok(());}
 if float(&c,0x64)<threshold{put(&mut c,0x64,threshold.to_bits());}
 let path=word(&c,0xb8);let mut record=ctx.sample_scratch;
 let mut near=NearestOutputs{point:Some(vector(&record,0x14)),along:Some(float(&c,0x30))};
 nearest(memory,path,vector(&e,0x14),&mut near,ctx.incoming_f28,math)?;
 putv(&mut record,0x14,near.point.unwrap());put(&mut c,0x30,near.along.unwrap().to_bits());
 let forward=rotate_vector(&e,[k(0x75c8),zero,zero],true,math)?;
 put(&mut record,0x10,float(&c,0x30).to_bits());
 if sample(memory,path,&mut record,ctx.incoming_f30,math)?{
  // Original adds travel for nonnegative forward/tangent dot, subtracts for negative.
  let travel=if dot(forward,vector(&record,0x38))>=zero{float(&c,0x30)+float(&c,0x64)}else{
   let tangent=vector(&record,0x38).map(|v|-v);putv(&mut record,0x38,tangent);float(&c,0x30)-float(&c,0x64)
  };put(&mut c,0x30,travel.to_bits());
 }
 let old=vector(&record,0x14);put(&mut record,0x10,float(&c,0x30).to_bits());
 let usable=sample(memory,path,&mut record,ctx.incoming_f30,math)?;
 if !usable{clear(&mut e);}else{
  let workspace=word(&c,0x90);let mut normal=[0.;3];for i in 0..3{normal[i]=f32::from_bits(memory.word(workspace.wrapping_add(0x1c+i as u32*4))?);}
  let point=vector(&record,0x14);let position=vector(&e,0x14);
  let correction=normalize(std::array::from_fn(|i|point[i]-position[i]),math)?;
  let new_forward=rotate_vector(&e,[k(0x75c8),zero,zero],true,math)?;
  let mut movement=normalize(std::array::from_fn(|i|point[i]-old[i]),math)?;
  if dot(new_forward,movement)<zero{movement=movement.map(|v|-v);}
  let correction=std::array::from_fn(|i|k(0x7598).mul_add(movement[i],correction[i]));
  let plane=[zero,zero,zero,normal[0],normal[1],normal[2]];
  let desired=project(correction,plane,math)?;
  let previous=project(forward,plane,math)?;
  if length(previous,math)?==zero as f64||length(desired,math)?==zero as f64{clear(&mut e);}else{
   let desired=normalize(desired,math)?;let previous=normalize(previous,math)?;
   let angle=path_angle(desired,previous,normal,math)?;let yaw=word(&e,12);
   let mut target=yaw.wrapping_add(angle)&0xffff;
   if angle_distance(target,yaw)>0x4000{target=target.wrapping_add(0x8000);}
   put(&mut m,0x1c,target);let distance=angle_distance(target,yaw)as i32;let tangent=vector(&record,0x38);
   let mut first=cross(forward,tangent);let mut second=cross(desired,tangent);
   first=if length(first,math)?<=k(0x75a8)as f64{normal}else{normalize(first,math)?};
   second=if length(second,math)?<=k(0x75a8)as f64{normal}else{normalize(second,math)?};
   let alignment=dot(first,second);
   let step=if alignment<zero{distance>>1}else if alignment<=k(0x7594){distance>>4}else if distance>0x400{
    let v=math.double_constant(0x803b6520-0x75e8)*math.acos(dot(tangent,forward))as f64;
    (if v.is_nan(){i32::MIN}else{v as i32})>>4
   }else{distance>>7};
   put(&mut e,12,angle_approach(target,yaw,step));
  }
 }
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);Ok(())
}
