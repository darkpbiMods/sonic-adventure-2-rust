//! Original homing landing decision80081330..80081430, after sound dispatch.
use crate::{disc::Result,original_math::OriginalMath,original_control::ControlContext,original_contact::Memory};
pub fn advance(entity:&mut[u8],motion:&[u8],character:&mut[u8],control:&ControlContext,memory:&Memory,cache:&mut[u32;4],historical_yaw:Option<u32>,frame:u32,seed:&mut u32,context_draw:bool,math:&OriginalMath)->Result<bool> {
 if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0{return Err("invalid homing landing records".into());}
 let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap());
 if entity[0]!=13||flags&3==0{return Err("homing landing requires supported action13".into());}
 let mut next_seed=*seed;let mut idle_event=false;
 let(mut e,mut c,mut next_cache)=(entity.to_vec(),character.to_vec(),*cache);
 e[4..6].copy_from_slice(&(flags&!0x500).to_be_bytes());c[0x35e]=0;
 let reverse=crate::original_reverse_input::check(&e,&c,control,memory,&mut next_cache,math)?;
 let speed=f32::from_be_bytes(c[0x64..0x68].try_into().unwrap());
 if !speed.is_finite(){return Err("nonfinite homing landing speed".into());}
 if reverse&&speed>math.constant(0x803af568){
  e[0]=12;c[0x176..0x178].copy_from_slice(&18u16.to_be_bytes());c[0x17c..0x17e].fill(0);
  e[8..12].copy_from_slice(&motion[0x18..0x1c]);e[0x10..0x14].copy_from_slice(&motion[0x20..0x24]);
 }else{
  let stop=crate::original_landing_selection::ordinary_stop(&mut e,&mut c,control.magnitudes)?;
  if stop{idle_event=crate::original_idle_slope::advance(&e,motion,&mut c,control.gravity,frame,&mut next_seed,context_draw,math)?;}
  e[8..12].copy_from_slice(&motion[0x18..0x1c]);e[0x10..0x14].copy_from_slice(&motion[0x20..0x24]);
  if stop{
   let velocity=std::array::from_fn(|i|f32::from_be_bytes(motion[i*4..i*4+4].try_into().unwrap()));
   c[0x176..0x178].copy_from_slice(&crate::original_landing_selection::select(velocity,math)?.to_be_bytes());
  }else{e[0]=1;crate::original_locomotion_selection::advance(&e,&mut c,historical_yaw,math)?;}
 }
 entity.copy_from_slice(&e);character.copy_from_slice(&c);*cache=next_cache;*seed=next_seed;Ok(idle_event)
}
