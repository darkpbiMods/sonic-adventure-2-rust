//! Original GSNE8P ground gravity alignment dependency.
use crate::{disc::Result,original_gameplay::{angle_approach,ENTITY_BYTES,CHARACTER_BYTES},original_math::OriginalMath,original_ground::rotate_vector};
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn short(b:&[u8],i:usize)->u16{u16::from_be_bytes(b[i..i+2].try_into().unwrap())}
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}

/// Complete 80047cbc..80047f14: steer towards local gravity when moving
/// sufficiently fast in the gravity direction, retaining world velocity.
pub fn align_ground(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],gravity:[f32;3],math:&OriginalMath)->Result<()>{
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES || motion.len()<0x24{return Err("unsupported ground alignment state size".into());}
    if short(character,0x14)!=0{return Ok(());}
    let speed=[float(motion,0),float(motion,4),float(motion,8)];
    let jog=float(character,0xe4);
    if speed.iter().chain(gravity.iter()).any(|x|!x.is_finite()) || !jog.is_finite(){return Err("nonfinite ground alignment state".into());}
    let (length,unit)=math.estimated_vector(speed)?;
    if length<=jog as f64{return Ok(());}
    let unit=unit.ok_or("undefined ground alignment normalization")?;
    let dot=gravity[2].mul_add(unit[2],gravity[1].mul_add(unit[1],unit[0]*gravity[0]));
    if (dot as f64)<math.double_constant(0x803b6520-0x7488){return Ok(());}
    let [x,y,z]=rotate_vector(entity,gravity,false,math)?;
    let c=|offset:u32|math.constant(0x803b6520-offset);
    let target=if y>c(0x7480){
        let positive_x=if x<0.{-x}else{x};
        let direction=word(entity,12).wrapping_add(math.angle_from_atan2(z,positive_x)as u32);
        let positive_z=if z<0.{-z}else{z};
        let rate=if short(entity,4)&0x100!=0 {
            ((c(0x747c)*positive_z)*float(character,0x64))/jog
        }else{c(0x7478)*positive_z};
        if !rate.is_finite(){return Err("ground alignment turn rate outside finite domain".into());}
        angle_approach(word(motion,0x1c),direction,rate as i32)
    }else{word(entity,12)};
    let velocity=[float(character,0x64),float(character,0x68),float(character,0x6c)];
    let world=rotate_vector(entity,velocity,true,math)?;
    let yaw=angle_approach(word(entity,12),target,0x2000);
    let mut next=[0;ENTITY_BYTES];next.copy_from_slice(entity);put(&mut next,12,yaw);
    let local=rotate_vector(&next,world,false,math)?;
    put(entity,12,yaw);put(motion,0x1c,target);
    for i in 0..3{put(character,0x64+i*4,local[i].to_bits());}
    Ok(())
}
