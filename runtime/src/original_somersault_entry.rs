//! Entry branch80078a10..80078aa0, before sound/feedback calls.
use crate::disc::Result;
/// Returns the selected animation. Caller has already accepted a somersault input.
/// Source clears its action timer and queues the shared-button continuation3.
pub fn enter(entity:&mut[u8],character:&mut[u8])->Result<u16>{
 if entity.len()!=0x40||character.len()!=0x3a0{return Err("invalid somersault entry records".into());}
 let speed=f32::from_be_bytes(character[0x64..0x68].try_into().unwrap());
 let threshold=f32::from_be_bytes(character[0xe8..0xec].try_into().unwrap());
 if !speed.is_finite()||!threshold.is_finite(){return Err("nonfinite somersault entry speed".into());}
 let fast=speed>threshold;let animation=if fast{96_u16}else{91_u16};
 entity[0]=if fast{66}else{61};
 let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap())|0x400;
 entity[4..6].copy_from_slice(&flags.to_be_bytes());
 character[0x176..0x178].copy_from_slice(&animation.to_be_bytes());
 character[0x366..0x368].fill(0);character[0x35d]=3;
 // Verified source803af680 constant, shared by both branches.
 character[0x64..0x68].copy_from_slice(&1.875_f32.to_be_bytes());
 Ok(animation)
}
