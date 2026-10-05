//! Complete braking force helper 80048ec4..80049278.
use crate::{disc::Result,original_math::OriginalMath};
fn word(b:&[u8],o:usize)->u32{u32::from_be_bytes(b[o..o+4].try_into().unwrap())}
fn f(b:&[u8],o:usize)->f32{f32::from_bits(word(b,o))}
pub fn accelerate(entity:&mut[u8],motion:&[u8],character:&mut[u8],gravity:[f32;3],magnitudes:[f32;2],math:&OriginalMath)->Result<()>{
 if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0{return Err("invalid brake records".into());}
 let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap());if flags&0x4000!=0{return Ok(());}
 let(mut e,mut c)=(entity.to_vec(),character.to_vec());
 let zero=math.constant(0x803aef28);
 for o in [0x40,0x64,0x68,0x6c,0x108,0x114,0x118,0x11c,0x120,0x124,0x128,0x138]{if !f(&c,o).is_finite(){return Err("nonfinite brake parameter".into());}}
 let world=std::array::from_fn(|i|gravity[i].mul_add(f(&c,0x138),f(motion,12+4*i)));
 let mut force=crate::original_ground::rotate_vector(&e,world,false,math)?;
 for i in 0..3{force[i]=f(&c,0x114+4*i).mul_add(f(&c,0x64+4*i),force[i]);}
 let speed=[f(&c,0x64),f(&c,0x6c)];
 let braking=speed.map(|v|if v>zero{f(&c,0x108).mul_add(f(&c,0x40),zero)}else if v<zero{-f(&c,0x108).mul_add(f(&c,0x40),-zero)}else{zero});
 if flags&3!=0{
  for (j,axis) in [0_usize,2].into_iter().enumerate(){
   if speed[j]==zero{
    let friction=f(&c,0x40)*(force[1]*f(&c,0x128));
    if force[axis]<friction&&force[axis]>-friction{force[axis]=zero;}
   }else{
    let friction=if force[1]<zero{f(&c,0x40)*(force[1]*f(&c,if axis==0{0x120}else{0x124}))}else{zero};
    if axis==0{
     let mut v=force[0]+braking[0];
     if v>zero{v-=friction;if v<zero{v=zero;}}
     else if v<zero{v+=friction;if v>zero{v=zero;}}
     force[0]=v;
    }else{
     let mut addition=braking[1];if force[2]>zero{addition-=friction;}else if force[2]<zero{addition+=friction;}
     force[2]+=addition;
    }
    if (speed[j]>zero&&speed[j]< -force[axis])||(speed[j]<zero&&speed[j]>-force[axis]){force[axis]=-speed[j];}
   }
  }
 }
 crate::original_movement::steer(&mut e,&mut c,word(motion,0x1c),magnitudes,math)?;
 for i in 0..3{c[0x58+4*i..0x5c+4*i].copy_from_slice(&force[i].to_be_bytes());}
 entity.copy_from_slice(&e);character.copy_from_slice(&c);Ok(())
}
