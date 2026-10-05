//! Complete air acceleration8004b330..8004b568 with real controller lookup.
use crate::{disc::Result,original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES,angle_distance},original_math::OriginalMath,
 original_contact::Memory,original_control::{ControlContext,InputOutputs},original_ground::rotate_vector};
fn word(b:&[u8],o:usize)->u32{u32::from_be_bytes(b[o..o+4].try_into().unwrap())}
fn float(b:&[u8],o:usize)->f32{f32::from_bits(word(b,o))}
fn put(b:&mut[u8],o:usize,v:u32){b[o..o+4].copy_from_slice(&v.to_be_bytes());}
pub fn accelerate(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],ctx:&ControlContext,memory:&Memory,cache:&mut[u32;4],math:&OriginalMath)->Result<()>{
 if entity.len()!=ENTITY_BYTES||motion.len()!=0x40||character.len()!=CHARACTER_BYTES{return Err("invalid complete air state extent".into());}
 for o in [0x64,0x68,0x6c,0xe8,0x100,0x10c,0x110,0x118,0x11c,0x138]{if !float(character,o).is_finite(){return Err("nonfinite air parameter".into());}}
 if ctx.gravity.iter().any(|v|!v.is_finite())||[4,12,16,20].iter().any(|&o|!float(motion,o).is_finite()){return Err("nonfinite air external force".into());}
 let(mut e,mut m,mut c,mut next_cache)=(entity.to_vec(),motion.to_vec(),character.to_vec(),*cache);
 let force=std::array::from_fn(|i|ctx.gravity[i].mul_add(float(&c,0x138),float(&m,12+i*4)));
 let mut acceleration=rotate_vector(&e,force,false,math)?;
 for(i,o)in [0x110,0x118,0x11c].into_iter().enumerate(){acceleration[i]=float(&c,o).mul_add(float(&c,0x64+i*4),acceleration[i]);}
 let zero=math.constant(0x803b6520-0x75f8);let mut input=InputOutputs{direction:Some(0),magnitude:Some(zero)};
 let active=crate::original_control::read(&e,&c,ctx,memory,&mut next_cache,&mut input,math)?;
 let addition=if active{
  let direction=input.direction.unwrap();let magnitude=input.magnitude.unwrap();let distance=angle_distance(word(&e,12),direction);
  let addition=if float(&c,0x64)>float(&c,0xe8)&&distance>0x6000{
   // Source braking keeps the PREVIOUS motion target; do not replace it.
   float(&c,0x10c)*magnitude
  }else{
   put(&mut m,0x1c,direction);
   if distance>0x1000{zero}else{
    let value=float(&c,0x100)*magnitude;
    if float(&m,4)<zero{math.constant(0x803b6520-0x75fc)*value}else{value}
   }
  };
  crate::original_movement::steer(&mut e,&mut c,word(&m,0x1c),ctx.magnitudes,math)?;addition
 }else{put(&mut m,0x1c,word(&e,12));zero};
 acceleration[0]+=addition;
 if acceleration.iter().any(|v|!v.is_finite()){return Err("nonfinite air acceleration result".into());}
 for i in 0..3{put(&mut c,0x58+i*4,acceleration[i].to_bits());}
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);*cache=next_cache;Ok(())
}
