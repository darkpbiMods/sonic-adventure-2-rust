//! Original contact-motion response 80040540 and position branch 80042b4c.
use crate::{disc::Result,original_gameplay::{angle_distance,ENTITY_BYTES,CHARACTER_BYTES},original_geometry::signed_angle,original_math::OriginalMath};
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn short(b:&[u8],i:usize)->u16{u16::from_be_bytes(b[i..i+2].try_into().unwrap())}
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn vector(b:&[u8],i:usize)->[f32;3]{[float(b,i),float(b,i+4),float(b,i+8)]}
fn put_vector(b:&mut[u8],i:usize,v:[f32;3]){for j in 0..3{put(b,i+j*4,v[j].to_bits());}}
fn finite(v:[f32;3])->Result<[f32;3]>{if v.into_iter().all(f32::is_finite){Ok(v)}else{Err("nonfinite contact motion vector".into())}}
fn unit(v:[f32;3],math:&OriginalMath)->Result<[f32;3]>{math.estimated_vector(finite(v)?)?.1.ok_or_else(||"zero contact normalization outside verified domain".into())}
fn validate(e:&[u8],c:&[u8],w:&[u8],m:&[u8])->Result<()>{
 if e.len()!=ENTITY_BYTES || c.len()!=CHARACTER_BYTES || w.len()<0x78 || m.len()<0x24{return Err("unsupported contact motion state size".into());}
 if c[0]>1{return Err("contact player outside original two-player slots".into());}Ok(())
}

// 8011502c computes cross components in f32, then accumulates its length in
// double precision after the first square. Only the zero result is used here.
fn cross_length(a:[f32;3],b:[f32;3])->Result<f64>{
 let a=finite(a)?;let b=finite(b)?;
 let x=b[2].mul_add(a[1],-(b[1]*a[2]));
 let y=b[0].mul_add(a[2],-(b[2]*a[0]));
 let z=b[1].mul_add(a[0],-(b[0]*a[1]));
 let squared=(z as f64).mul_add(z as f64,(y as f64).mul_add(y as f64,(x*x) as f64));
 if !squared.is_finite(){return Err("nonfinite contact cross product".into());}
 // Original frsqrte(0) produces +infinity; fres(+infinity) produces +zero.
 if squared==0.{return Ok(0.);}
 let inv=crate::gekko_estimate::reciprocal_sqrt_seed(squared);
 Ok(crate::gekko_estimate::reciprocal_seed(inv))
}

fn respond_inner(e:&mut[u8],c:&[u8],w:&mut[u8],m:&mut[u8],direction:[f32;3],math:&OriginalMath)->Result<()>{
 let k=|offset:u32|math.constant(0x803b6520-offset);
 let grounded=short(c,0x1c)&3!=0;
 if !grounded && math.estimated_vector(finite(vector(c,0x64))?)?.0<=float(c,0xe4) as f64{return Ok(());}
 if cross_length(direction,vector(w,0x48))?==k(0x75f8) as f64{return Ok(());}
 let kind=c[1] as i8;
 if (kind==0 || kind==1) && float(c,0x134)<math.constant(0x80196578+(kind as u32)*0x84+0x74){
  put_vector(w,0x6c,[k(0x75f8);3]);put_vector(m,0,[k(0x75f8);3]);return Ok(());
 }
 let new_direction=finite(vector(w,0x6c))?;
 if math.estimated_vector(new_direction)?.0==k(0x75f8) as f64{return Ok(());}
 let old_unit=unit(vector(w,0x10),math)?;let new_unit=unit(new_direction,math)?;
 let angle=signed_angle(old_unit,new_unit,Some(finite(vector(w,0x1c))?),math)?;
 let mut yaw=word(e,0xc).wrapping_sub(angle);
 if angle_distance(word(m,0x1c),yaw)>0x6000{yaw=yaw.wrapping_add(0x8000);}
 put(e,0xc,yaw);put(m,0x1c,yaw);
 if math.estimated_vector(new_direction)?.0>=k(0x7518) as f64{
  let mut speed=math.estimated_vector(finite(vector(m,0))?)?.0;
  if speed>k(0x7514) as f64 && grounded{speed=((speed*k(0x7510) as f64) as f32) as f64;}
  let velocity=finite(new_unit.map(|x|((x as f64)*speed) as f32))?;
  put_vector(m,0,velocity);
 }
 Ok(())
}

/// The motion buffer represents the pointer selected from global 801e7748
/// by character player index. The host resolves that pointer before this call.
pub fn respond(entity:&mut[u8],character:&[u8],workspace:&mut[u8],motion:&mut[u8],direction:[f32;3],math:&OriginalMath)->Result<()>{
 validate(entity,character,workspace,motion)?;
 let(mut e,mut w,mut m)=(entity.to_vec(),workspace.to_vec(),motion.to_vec());
 respond_inner(&mut e,character,&mut w,&mut m,direction,math)?;
 entity.copy_from_slice(&e);workspace.copy_from_slice(&w);motion.copy_from_slice(&m);Ok(())
}

pub struct PositionContact {
 pub previous_position:[f32;3],
 pub offset:[f32;3],
 pub previous_result:u32,
 pub flags:u32,
 /// Value of source global 803b52c0-78f4.
 pub stop_mode:u32,
}

/// Complete 80042b4c..80042e0c, using the reconstructed 80040540 response.
pub fn position(entity:&mut[u8],character:&mut[u8],workspace:&mut[u8],motion:&mut[u8],contact:&PositionContact,math:&OriginalMath)->Result<u32>{
 validate(entity,character,workspace,motion)?;
 let(mut e,mut c,mut w,mut m)=(entity.to_vec(),character.to_vec(),workspace.to_vec(),motion.to_vec());
 let result=position_inner(&mut e,&mut c,&mut w,&mut m,contact,math)?;
 entity.copy_from_slice(&e);character.copy_from_slice(&c);workspace.copy_from_slice(&w);motion.copy_from_slice(&m);Ok(result)
}
fn position_inner(e:&mut[u8],c:&mut[u8],w:&mut[u8],m:&mut[u8],contact:&PositionContact,math:&OriginalMath)->Result<u32>{
 let normal=finite(vector(w,0x48))?;put_vector(c,0x70,normal);
 let mut result=contact.previous_result;
 let grounded=short(e,4)&1!=0;
 let velocity=finite(vector(m,0))?;
 let direction=if grounded{
  let old=finite(contact.previous_position)?;let pos=finite(vector(e,0x14))?;
  let delta=finite(std::array::from_fn(|i|old[i]-pos[i]))?;
  if math.estimated_vector(delta)?.0!=0.{delta}else{velocity.map(|x|-x)}
 }else{velocity.map(|x|-x)};
 if math.estimated_vector(direction)?.0==0.{
  if grounded{return Ok(result);}
 }else{
  let direction=unit(direction,math)?;
  let angle=signed_angle(direction,normal,None,math)?;
  if angle>=0x4000{return Ok(result);}
  if grounded || contact.flags&0x1000!=0{result=1;}
  else if contact.stop_mode!=0 && short(e,4)&2!=0{
   put_vector(w,0x6c,[0.;3]);put_vector(m,0,[0.;3]);
  }
  respond_inner(e,c,w,m,direction,math)?;
 }
 let position=finite(vector(w,0x60))?;let offset=finite(contact.offset)?;
 let adjusted=finite(std::array::from_fn(|i|position[i]-offset[i]))?;
 put_vector(w,4,position);put_vector(e,0x14,adjusted);
 let remaining=vector(w,0x6c);put_vector(w,0x10,remaining);
 Ok(result)
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn degenerate_response_preserves_all_state(){
  let math=OriginalMath::from_dol(std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../source/main.dol")).unwrap()).unwrap();
  let mut e=[0;ENTITY_BYTES];let mut c=[0;CHARACTER_BYTES];let mut w=[0;0x80];let mut m=[0;0x40];
  c[0x1d]=1;c[1]=2;put_vector(&mut w,0x48,[0.,1.,0.]);put_vector(&mut w,0x6c,[1.,0.,0.]);
  let old=(e,w,m);assert!(respond(&mut e,&c,&mut w,&mut m,[1.,0.,0.],&math).is_err());assert_eq!((e,w,m),old);
 }
}
