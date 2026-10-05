//! Reconstructed ground movement dependencies, GSNE8P revision zero.
//! Ground acceleration and collision are still being reconstructed.
use crate::{disc::Result,original_gameplay::{angle_approach,ENTITY_BYTES,CHARACTER_BYTES},original_math::{OriginalMath,IDENTITY,transform}};
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}

/// Complete 8003e9e0 local-to-world and 8003ea6c world-to-local wrappers.
/// Original global matrix scratch is represented by a local matrix.
pub fn rotate_vector(entity:&[u8],v:[f32;3],to_world:bool,math:&OriginalMath)->Result<[f32;3]>{
    if entity.len()!=ENTITY_BYTES || v.iter().any(|x|!x.is_finite()){return Err("unsupported vector wrapper input".into());}
    let [pitch,yaw,roll]=[word(entity,8)as i32,word(entity,12)as i32,word(entity,16)as i32];
    let m=if to_world {
        let m=math.rotate(&IDENTITY,b'z',roll)?;
        let m=math.rotate(&m,b'x',pitch)?;
        math.rotate(&m,b'y',yaw.wrapping_neg())?
    }else{
        let m=math.rotate(&IDENTITY,b'y',yaw)?;
        let m=math.rotate(&m,b'x',pitch.wrapping_neg())?;
        math.rotate(&m,b'z',roll.wrapping_neg())?
    };
    let out=transform(&m,v);
    if out.iter().any(|x|!x.is_finite()){return Err("nonfinite vector wrapper result".into());}
    Ok(out)
}

/// Complete 800473b4..800475d8: speed-dependent turn rate and slope-dependent
/// velocity blend. Motion argument is unused by the original routine.
pub fn steer_speed_dependent(entity:&mut[u8],character:&mut[u8],target:u32,math:&OriginalMath)->Result<()>{
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES{return Err("unsupported ground steering state size".into());}
    let velocity=[float(character,0x64),float(character,0x68),float(character,0x6c)];
    let dash=float(character,0xf4);let slope=float(character,0x34);
    if !dash.is_finite() || !slope.is_finite(){return Err("nonfinite ground steering state".into());}
    let world=rotate_vector(entity,velocity,true,math)?;
    let c=|offset:u32|math.constant(0x803b6520-offset);
    let mut rate=0x100;
    if velocity[0]>dash {
        let excess=(velocity[0]-dash)*c(0x7490);
        if !excess.is_finite(){return Err("ground turn speed outside finite domain".into());}
        let root=math.sqrt_refined(excess);
        rate=(-(c(0x748c).mul_add(root,-256.)))as i32;
    }
    let yaw=angle_approach(word(entity,12),target,rate);
    let mut next=[0;ENTITY_BYTES];next.copy_from_slice(entity);put(&mut next,12,yaw);
    let turned=rotate_vector(&next,world,false,math)?;
    let (old_weight,new_weight)=if slope>c(0x7518){(c(0x7498),c(0x7494))}else{(c(0x75cc),c(0x75cc))};
    let blended=std::array::from_fn::<_,3,_>(|i|velocity[i].mul_add(old_weight,turned[i]*new_weight));
    if blended.iter().any(|x|!x.is_finite()){return Err("nonfinite ground steering result".into());}
    put(entity,12,yaw);
    for i in 0..3{put(character,0x64+i*4,blended[i].to_bits());}
    Ok(())
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn unsupported_turn_input_preserves_state(){
        let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../source/main.dol");
        let math=OriginalMath::from_dol(std::fs::read(path).unwrap()).unwrap();
        let mut e=[0;ENTITY_BYTES];let mut c=[0;CHARACTER_BYTES];
        put(&mut c,0x64,f32::INFINITY.to_bits());let before=c;
        assert!(steer_speed_dependent(&mut e,&mut c,0x4000,&math).is_err());
        assert_eq!(e,[0;ENTITY_BYTES]);assert_eq!(c,before);
        // Large signed angles are supported. Exercise a real downstream
        // overflow instead: rotating two finite MAX components overflows.
        c=[0;CHARACTER_BYTES];put(&mut e,8,0x2000);
        put(&mut c,0x68,f32::MAX.to_bits());put(&mut c,0x6c,f32::MAX.to_bits());let old_c=c;let before=e;
        assert!(steer_speed_dependent(&mut e,&mut c,0x4000,&math).is_err());
        assert_eq!(e,before);assert_eq!(c,old_c);
    }
}
