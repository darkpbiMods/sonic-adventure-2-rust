//! Ordinary stopped reverse-turn branch 80080fc8..80081018.
use crate::{disc::Result,original_contact::Memory,original_math::OriginalMath,original_target_support::DirectionContext};
pub fn advance(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],ctx:&DirectionContext,memory:&Memory,cache:&mut[u32;4],math:&OriginalMath)->Result<bool>{
 if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0{return Err("invalid brake turn extent".into());}
 let speed=f32::from_be_bytes(character[0x64..0x68].try_into().unwrap());
 if !speed.is_finite(){return Err("nonfinite brake turn speed".into());}
 if speed>0.{return Ok(false);}
 if !crate::original_reverse_input::check(entity,character,ctx.camera.control,memory,cache,math)?{return Ok(false);}
 let(mut e,mut m,mut c)=(entity.to_vec(),motion.to_vec(),character.to_vec());
 e[0]=1;c[0x176..0x178].copy_from_slice(&27_u16.to_be_bytes());
 crate::original_target_support::direction(&mut e,&mut m,&mut c,ctx,memory,cache,math)?;
 e[12..16].copy_from_slice(&m[0x1c..0x20]);
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);Ok(true)
}
