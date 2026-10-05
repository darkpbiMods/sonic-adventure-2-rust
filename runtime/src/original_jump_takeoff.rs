//! Ordinary jump entry plus first physics pass, without collision or audio.
use crate::{disc::Result,original_math::OriginalMath,original_movement::{AirContext,align_airborne,air_acceleration,integrate_velocity},original_gameplay::ordinary_jump_block};

/// Source8007a8e8..a97c then80084eec..f30. Caller supplies already reconciled
/// contact records. Moving attachments and environment override are excluded.
/// The entry retains grounded bit1 until collision; do not clear it early.
pub fn advance(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],context:&AirContext,math:&OriginalMath)->Result<()>{
 let(mut e,mut m,mut c)=(entity.to_vec(),motion.to_vec(),character.to_vec());
 ordinary_jump_block(&mut e,&mut c)?;
 align_airborne(&mut e,&mut c,0,0,math)?;
 air_acceleration(&mut e,&mut m,&mut c,context,math)?;
 integrate_velocity(&e,&mut m,&mut c,context.magnitudes,math)?;
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);Ok(())
}

/// Verified attached entry and first physics pass. Caller supplies this player's
/// motion record and attachment memory. End-frame force clearing is separate.
pub fn advance_with_platform(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],memory:&crate::original_contact::Memory,context:&AirContext,math:&OriginalMath)->Result<()>{
 let(mut e,mut m,mut c)=(entity.to_vec(),motion.to_vec(),character.to_vec());
 crate::original_gameplay::jump_block_with_platform(&mut e,&mut m,&mut c,memory)?;
 align_airborne(&mut e,&mut c,0,0,math)?;
 air_acceleration(&mut e,&mut m,&mut c,context,math)?;
 integrate_velocity(&e,&mut m,&mut c,context.magnitudes,math)?;
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);Ok(())
}

/// Force/displacement clear8004f504..530, after frame movement and consumers.
/// This is not the rest of8004f4d0 (status, timers and history bookkeeping).
pub fn clear_frame_forces(motion:&mut[u8],character:&mut[u8])->Result<()>{
 if motion.len()!=0x40||character.len()!=0x3a0{return Err("invalid frame force records".into());}
 motion[0xc..0x18].fill(0);
 for offset in [0x60,0x5c,0x58,0x54,0x50,0x4c,0x38]{character[offset..offset+4].fill(0);}
 Ok(())
}
