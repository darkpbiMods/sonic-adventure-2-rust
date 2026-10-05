//! Complete input-direction classification8004db5c..8004dbf4.
use crate::{disc::Result,original_control::{ControlContext,InputOutputs},original_contact::Memory,original_math::OriginalMath};
pub fn classify(entity:&[u8],character:&[u8],ctx:&ControlContext,memory:&Memory,cache:&mut[u32;4],math:&OriginalMath)->Result<i32>{
 if entity.len()!=0x40||character.len()!=0x3a0{return Err("invalid direction classification records".into());}
 if u16::from_be_bytes(entity[4..6].try_into().unwrap())&0x4000!=0{return Ok(0);}
 let mut next_cache=*cache;let mut input=InputOutputs{direction:Some(0),magnitude:None};
 if !crate::original_control::read(entity,character,ctx,memory,&mut next_cache,&mut input,math)?{return Ok(0);}
 let yaw=u32::from_be_bytes(entity[12..16].try_into().unwrap());
 let delta=crate::original_gameplay::angle_distance(input.direction.unwrap(),yaw);
 *cache=next_cache;Ok(if delta>0x6000{1}else if delta>0x3400{2}else if delta>0x1400{3}else{4})
}
