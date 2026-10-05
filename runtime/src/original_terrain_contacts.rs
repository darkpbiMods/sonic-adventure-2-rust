//! Contact-list predicate 80041814..80041984 and distance 80027c2c..80027cc0.
use crate::{disc::Result,original_geometry::signed_angle,original_math::OriginalMath};
fn vector(b:&[u8],i:usize)->[f32;3]{std::array::from_fn(|j|f32::from_be_bytes(b[i+j*4..i+j*4+4].try_into().unwrap()))}
fn finite(v:[f32;3])->Result<[f32;3]>{if v.into_iter().all(f32::is_finite){Ok(v)}else{Err("nonfinite terrain contact vector".into())}}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],b[0]*a[0]))}
pub fn distance(a:[f32;3],b:[f32;3],math:&OriginalMath)->Result<f32>{
 let a=finite(a)?;let b=finite(b)?;let d=finite(std::array::from_fn(|i|a[i]-b[i]))?;
 let squared=d[2].mul_add(d[2],d[0].mul_add(d[0],d[1]*d[1]));
 if !squared.is_finite(){return Err("terrain contact distance overflow".into());}
 Ok(math.sqrt_refined(squared))
}
/// Records mirror original global 80295d10, stride 0x58. The count is the
/// workspace halfword at +28. The original stack arrays hold at most 128.
pub fn qualifying_contact(workspace:&[u8],records:&[u8],gravity:[f32;3],math:&OriginalMath)->Result<bool>{
 if workspace.len()<0x2a{return Err("short terrain contact workspace".into());}
 let count=u16::from_be_bytes(workspace[0x28..0x2a].try_into().unwrap())as usize;
 if count>128 || records.len()<count*0x58{return Err("unsupported terrain contact count or record extent".into());}
 let gravity=finite(gravity)?;let ground=finite(vector(workspace,0x1c))?;
 let position=finite(vector(workspace,4))?;let velocity=finite(vector(workspace,0x10))?;
 let mut count_qualifying=0;let mut closest=math.constant(0x803b6520-0x7554);
 for record in records[..count*0x58].chunks_exact(0x58){
  let distance=distance(vector(record,0x2c),position,math)?;
  let normal=finite(vector(record,0x14))?;
  let angle=signed_angle(ground,normal,None,math)?;
  if angle<=0x2c00 || dot(gravity,ground)>dot(gravity,normal){continue;}
  let (_,unit)=math.estimated_vector(velocity)?;let direction=unit.unwrap_or(velocity);
  if angle<0x7000 && dot(direction,normal)<math.constant(0x803b6520-0x75f8){
   count_qualifying+=1;
   if closest>distance{closest=distance;}
  }
 }
 // Original closest-distance and per-record flags are local scratch only.
 let _=closest;Ok(count_qualifying>0)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn malformed_contact_extent_is_rejected(){
  let math=OriginalMath::from_dol(std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../source/main.dol")).unwrap()).unwrap();
  let mut w=[0;0x84];w[0x29]=1;assert!(qualifying_contact(&w,&[],[0.,-1.,0.],&math).is_err());
  w[0x29]=129;assert!(qualifying_contact(&w,&vec![0;129*0x58],[0.,-1.,0.],&math).is_err());
 }
}
