//! Selection block80054a24..80054aec; true means helper continues to tilt/rate.
use crate::{disc::Result,original_math::OriginalMath};
pub fn select(entity:&[u8],character:&mut[u8],math:&OriginalMath)->Result<bool>{
 if entity.len()<6||character.len()<0x178{return Err("short locomotion records".into());}
 let selected=u16::from_be_bytes(character[0x176..0x178].try_into().unwrap());
 if [26,27,34].contains(&selected){return Ok(false);}
 let read=|offset:usize|->Result<f32>{let v=f32::from_be_bytes(character[offset..offset+4].try_into().unwrap());if !v.is_finite(){return Err("nonfinite locomotion selector".into());}Ok(v)};
 let speed=math.constant(0x803b6520-0x73dc).mul_add(read(0x38)?,read(0x64)?);
 if !speed.is_finite(){return Err("nonfinite adjusted speed".into());}
 let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap());
 let animation:u16=if flags&0x800!=0{if speed>read(0xe8)?{32}else{38}}
 else if speed>read(0xf4)?{10}else if speed>read(0xec)?{9}else if speed>read(0xe8)?{8}else if speed>read(0xe4)?{7}else{6};
 character[0x176..0x178].copy_from_slice(&animation.to_be_bytes());Ok(true)
}
/// Complete80054a08..80054b88 with the history getter supplied by the host.
/// historical_yaw is the angle triplet's Y from80038008(player,3,null,out).
/// Missing task/history (source leaves output uninitialized) is outside domain.
pub fn advance(entity:&[u8],character:&mut[u8],historical_yaw:Option<u32>,math:&OriginalMath)->Result<()> {
 use crate::original_gameplay::{angle_delta,angle_distance,angle_approach};
 if entity.len()<0x10{return Err("short locomotion entity".into());}
 let mut next=character.to_vec();if !select(entity,&mut next,math)?{return Ok(());}
 let f=|at:usize| f32::from_be_bytes(next[at..at+4].try_into().unwrap());
 let speed=f(0x64);let slope=f(0x38);let threshold=f(0xe8);
 if !threshold.is_finite(){return Err("nonfinite lean threshold".into());}
 let target=if speed>=threshold{
  let old=historical_yaw.ok_or("locomotion needs three-update heading history")?;
  let yaw=u32::from_be_bytes(entity[12..16].try_into().unwrap());
  angle_approach(0,angle_delta(yaw,old).wrapping_shl(2) as u32,0x1d00)
 }else{0};
 let lean=u32::from_be_bytes(next[0x2c..0x30].try_into().unwrap());
 let lean=angle_approach(lean,target,(angle_distance(lean,target)as i32)>>2);
 let rate=math.constant(0x803b6520-0x7348).mul_add(slope,speed);
 if !rate.is_finite(){return Err("nonfinite locomotion rate".into());}
 next[0x2c..0x30].copy_from_slice(&lean.to_be_bytes());next[0x44..0x48].copy_from_slice(&rate.to_be_bytes());
 character.copy_from_slice(&next);Ok(())
}
