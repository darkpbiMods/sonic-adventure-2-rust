//! Original rolling speed exit, 80054570..800545cc.
use crate::disc::Result;
pub fn advance(entity:&mut[u8],character:&mut[u8])->Result<bool>{
 if entity.len()!=0x40||character.len()!=0x3a0{return Err("invalid roll exit record extent".into());}
 let read=|i|f32::from_be_bytes(character[i..i+4].try_into().unwrap());
 let(speed,threshold)=(read(0x64),read(0xe4));
 if !speed.is_finite()||!threshold.is_finite(){return Err("nonfinite roll exit speed".into());}
 if speed>=threshold{return Ok(false);}
 entity[0]=if speed<=0.{character[0x18..0x1a].fill(0);0}else{1};
 character[0x176..0x178].copy_from_slice(&26_u16.to_be_bytes());
 let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap())&!0x500;
 entity[4..6].copy_from_slice(&flags.to_be_bytes());
 Ok(true)
}
