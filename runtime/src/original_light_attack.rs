//! Light Speed Attack steering arithmetic; target selection/control reads external.
use crate::{disc::Result,original_math::OriginalMath};
/// Empty action60 selector and no-target pursuit branch8007aa64..94.
/// Collision movement and failed-pursuit action transition are separate phases.
pub fn advance_without_target(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],ctx:&crate::original_ground_accel::GroundContext,memory:&crate::original_contact::Memory,cache:&mut[u32;4],math:&OriginalMath)->Result<()>{
 if entity.len()!=crate::original_gameplay::ENTITY_BYTES||motion.len()!=0x40||character.len()!=crate::original_gameplay::CHARACTER_BYTES||entity[0]!=60{return Err("invalid charged no-target records".into());}
 let(mut e,mut m,mut c,mut next_cache)=(entity.to_vec(),motion.to_vec(),character.to_vec(),*cache);
 c[0x366..0x368].fill(0);
 let forward=f32::from_be_bytes(c[0x64..0x68].try_into().unwrap())*math.constant(0x803af634);
 c[0x64..0x68].copy_from_slice(&forward.to_be_bytes());
 crate::original_ground_accel::accelerate(&mut e,&mut m,&mut c,ctx,memory,&mut next_cache,math)?;
 crate::original_movement::integrate_velocity(&e,&mut m,&mut c,ctx.camera.control.magnitudes,math)?;
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);*cache=next_cache;Ok(())
}
/// Failed-pursuit dispatcher branch 80084bfc..80084c1c. The pursuit's
/// resulting velocity must be retained for subsequent collision movement.
pub fn exit_failed_pursuit(entity:&mut[u8],character:&mut[u8])->Result<()>{
 if entity.len()!=crate::original_gameplay::ENTITY_BYTES||character.len()!=crate::original_gameplay::CHARACTER_BYTES{return Err("invalid charged exit extent".into());}
 entity[0]=10;
 character[0x176..0x178].copy_from_slice(&15u16.to_be_bytes());
 let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap())&!0x500;
 entity[4..6].copy_from_slice(&flags.to_be_bytes());
 Ok(())
}
/// Bounded launch transition 800777fc..830; audio and collider updates external.
pub fn enter_action(entity:&mut [u8],character:&mut [u8])->Result<()>{
 if entity.len()<6||character.len()<0x178{return Err("short charged launch record".into());}
 let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap());
 entity[0]=60;
 entity[4..6].copy_from_slice(&((flags&!0x200)|0x500).to_be_bytes());
 character[0x176..0x178].copy_from_slice(&12u16.to_be_bytes());
 character[0x68..0x6c].copy_from_slice(&16.0f32.to_be_bytes());
 Ok(())
}
/// 8007ab30..abc8. Delta is target collider center minus entity origin.
/// Gravity is the source world gravity direction, not acceleration magnitude.
pub fn normalized_inputs(delta:[f32;3],motion:[f32;3],gravity:[f32;3],math:&OriginalMath)->Result<([f32;3],[f32;3])>{
 if delta.iter().chain(motion.iter()).chain(gravity.iter()).any(|v|!v.is_finite()){return Err("nonfinite charged pursuit input".into());}
 let (_,target)=math.estimated_vector(delta)?;
 let target=target.unwrap_or(gravity.map(|v|-v));
 let (_,current)=math.estimated_vector(motion)?;
 Ok((target,current.unwrap_or(target)))
}
/// 8007abc8..ac34. counter is source signed16bit360, then masked to low8bits.
pub fn weights(counter:i16,input:Option<f32>)->Result<(f32,f32)>{
 if input.is_some_and(|m|!m.is_finite()){return Err("nonfinite charged steering input".into());}
 let mut target=2.+(counter as u8 as f32)/255.;let mut current=10.-target;
 if let Some(m)=input{let add=2.*m;target+=add;current-=add;}
 Ok((target,current))
}
/// 8007ac38..acac; target/current vectors have already passed source normalization.
pub fn blend(target:[f32;3],current:[f32;3],weight:(f32,f32),math:&OriginalMath)->Result<[f32;3]>{
 if target.iter().chain(current.iter()).chain([weight.0,weight.1].iter()).any(|v|!v.is_finite()){return Err("nonfinite charged steering vector".into());}
 let result=std::array::from_fn(|i|current[i].mul_add(weight.1,target[i]*weight.0));
 let (_,normalized)=math.estimated_vector(result)?;
 Ok(normalized.unwrap_or(current))
}
