//! Complete80048724..80048830: external force plus gravity in local space.
use crate::{disc::Result,original_math::OriginalMath};
pub fn accelerate(entity:&[u8],motion:&[u8],character:&mut[u8],gravity:[f32;3],math:&OriginalMath)->Result<()>{
 if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0{return Err("invalid somersault force records".into());}
 if u16::from_be_bytes(entity[4..6].try_into().unwrap())&0x4000!=0{return Ok(());}
 let strength=f32::from_be_bytes(character[0x138..0x13c].try_into().unwrap());
 let external:[f32;3]=std::array::from_fn(|i|f32::from_be_bytes(motion[12+4*i..16+4*i].try_into().unwrap()));
 if !strength.is_finite()||!gravity.iter().chain(external.iter()).all(|v|v.is_finite()){return Err("nonfinite somersault force input".into());}
 let world=std::array::from_fn(|i|gravity[i].mul_add(strength,external[i]));
 let local=crate::original_ground::rotate_vector(entity,world,false,math)?;
 for i in 0..3{character[0x58+4*i..0x5c+4*i].copy_from_slice(&local[i].to_be_bytes());}
 Ok(())
}
