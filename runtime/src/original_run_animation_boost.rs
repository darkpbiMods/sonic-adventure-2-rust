//! Running animation completion adjustment, 800844bc..800844f8.
use crate::disc::Result;
pub fn advance(character:&mut[u8])->Result<bool>{
 if character.len()!=0x3a0{return Err("invalid running animation record".into());}
 let current=i16::from_be_bytes(character[0x178..0x17a].try_into().unwrap());
 let previous=i16::from_be_bytes(character[0x17a..0x17c].try_into().unwrap());
 if !((previous==27&&current!=27)||(previous==175&&current!=175)){return Ok(false);}
 let speed=f32::from_be_bytes(character[0x64..0x68].try_into().unwrap());
 let jog=f32::from_be_bytes(character[0xe4..0xe8].try_into().unwrap());
 if !speed.is_finite()||!jog.is_finite(){return Err("nonfinite animation speed adjustment".into());}
 character[0x64..0x68].copy_from_slice(&(speed+jog).to_be_bytes());Ok(true)
}
