//! Source 80080464..800804b0: action69 support-to-rebound transition.
//! Caller has already passed action interruption checks. The subsequent event
//! call is host-owned; this helper does not run forces or collision.
use crate::{disc::Result,original_math::OriginalMath};

/// Initial800802f8..80334 or repeated800808e0..80914 entry, before sound.
/// Caller has already accepted the action press and resolved higher priorities.
pub fn enter(entity:&mut[u8],character:&mut[u8],extension:&mut[u8],repeat:bool,math:&OriginalMath)->Result<()>{
 if entity.len()!=0x40||character.len()!=0x3a0||extension.len()<0x368{return Err("invalid bounce entry records".into());}
 let speed=math.constant(if repeat{0x803af650}else{0x803af654});
 if !speed.is_finite(){return Err("nonfinite bounce entry constant".into());}
 character[0x176..0x178].copy_from_slice(&30_u16.to_be_bytes());
 entity[0]=69;
 let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap())|0x500;
 entity[4..6].copy_from_slice(&flags.to_be_bytes());
 character[0x68..0x6c].copy_from_slice(&speed.to_be_bytes());
 if !repeat{extension[0x366..0x368].fill(0);}
 Ok(())
}

pub fn advance(entity:&mut[u8],motion:&[u8],character:&mut[u8],extension:&mut[u8],math:&OriginalMath)->Result<bool>{
 if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0||extension.len()<0x368{return Err("invalid bounce contact records".into());}
 if u16::from_be_bytes(entity[4..6].try_into().unwrap())&3==0{return Ok(false);}
 let repeat=i16::from_be_bytes(extension[0x366..0x368].try_into().unwrap())!=0;
 let speed=math.constant(if repeat{0x803af6d8}else{0x803af6d4});
 if !speed.is_finite(){return Err("nonfinite bounce contact constant".into());}
 entity[0]=70;
 entity[8..12].copy_from_slice(&motion[0x18..0x1c]);
 entity[0x10..0x14].copy_from_slice(&motion[0x20..0x24]);
 character[0x68..0x6c].copy_from_slice(&speed.to_be_bytes());
 if !repeat{extension[0x366..0x368].copy_from_slice(&1_i16.to_be_bytes());}
 Ok(true)
}
