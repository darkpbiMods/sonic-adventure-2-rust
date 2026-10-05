//! Source action13/60 terrain phase after pursuit:45880 then3e668.
//! Includes homing's intervening repeated-wall response85228..852d4.
//! Caller retains source flags and supplies already-computed pursuit motion.
use crate::{disc::Result,original_math::OriginalMath,original_position_a::PositionState,
 original_position_b::PositionBContext,original_terrain_candidates::CandidateWorld,
 original_platform::PlatformMemory};
pub fn advance<F>(state:&mut PositionState,special_mode:&mut u32,world:&CandidateWorld,ctx:&PositionBContext,math:&OriginalMath,callback:F)->Result<u32>
where F:FnMut(u32,u32,u32,u32,&mut PlatformMemory)->Result<()>{
 if state.entity.len()!=0x40||!matches!(state.entity[0],13|60){return Err("attack contact requires action13 or60".into());}
 let mut next=state.clone();let mut mode=*special_mode;
 let response=crate::original_collision_loop::update(&mut next,&mut mode,world,ctx,math,callback)?;
 if state.entity[0]==13 {
  let previous=std::array::from_fn(|i|f32::from_be_bytes(state.character[0x70+4*i..0x74+4*i].try_into().unwrap()));
  homing_wall_response(&next.entity,&mut next.motion,&mut next.character,previous,math)?;
 }
 crate::original_movement::reconcile_velocity(&mut next.entity,&mut next.motion,&mut next.character,ctx.common.control.gravity,ctx.common.control.level,math)?;
 *state=next;*special_mode=mode;Ok(response)
}

/// Source85228..852d4, before reconciliation. Returns whether motion changed.
pub fn homing_wall_response(entity:&[u8],motion:&mut[u8],character:&mut[u8],previous:[f32;3],math:&OriginalMath)->Result<bool>{
 if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0{return Err("invalid homing wall records".into());}
 let current=std::array::from_fn(|i|f32::from_be_bytes(character[0x70+4*i..0x74+4*i].try_into().unwrap()));
 let threshold=math.constant(0x803af6f4)as f64;
 if math.estimated_vector(current)?.0<=threshold||math.estimated_vector(previous)?.0<=threshold{return Ok(false);}
 let dot=current[2].mul_add(previous[2],previous[1].mul_add(current[1],previous[0]*current[0]));
 // Initialized801123b0 lookup for angle0x3c00, captured after801182cc.
 let cosine=f32::from_bits(0x3dc8bd65);
 if dot>=cosine{return Ok(false);}
 let world=std::array::from_fn(|i|f32::from_be_bytes(motion[4*i..4*i+4].try_into().unwrap()));
 let mut local=crate::original_ground::rotate_vector(entity,world,false,math)?;
 local[0]=0.;local[2]=0.;
 if u16::from_be_bytes(entity[4..6].try_into().unwrap())&3!=0{local[1]=0.;}
 let result=crate::original_ground::rotate_vector(entity,local,true,math)?;
 for i in 0..3{character[0x64+4*i..0x68+4*i].copy_from_slice(&local[i].to_be_bytes());motion[4*i..4*i+4].copy_from_slice(&result[i].to_be_bytes());}
 Ok(true)
}
