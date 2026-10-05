//! Complete reverse-input predicate 8004e490..8004e55c.
use crate::{disc::Result,original_control::{ControlContext,InputOutputs},original_contact::Memory,original_math::OriginalMath};
pub fn check(entity:&[u8],character:&[u8],ctx:&ControlContext,memory:&Memory,cache:&mut[u32;4],math:&OriginalMath)->Result<bool>{
 if entity.len()!=0x40||character.len()!=0x3a0{return Err("invalid reverse input records".into());}
 let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap());
 if flags&0x6000!=0{return Ok(false);}
 let mut input=InputOutputs{direction:Some(0),magnitude:None};
 if !crate::original_control::read(entity,character,ctx,memory,cache,&mut input,math)?{return Ok(false);}
 let player=character[0]as usize;
 if crate::original_contact_search::find_task(memory,ctx.player_tasks[player],0x11,cache)?!=0{return Ok(false);}
 let yaw=u32::from_be_bytes(entity[12..16].try_into().unwrap());
 Ok(crate::original_gameplay::angle_distance(yaw,input.direction.unwrap())>0x6000)
}
