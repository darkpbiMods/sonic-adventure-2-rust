//! Fire Somersault particle update80089d2c. Task deletion is returned to host.
use crate::{disc::Result,original_math::OriginalMath};
pub fn update(data:&mut[u8],math:&OriginalMath)->Result<bool>{
 if data.len()!=32{return Err("invalid fire particle record".into());}
 let mut next=data.to_vec();let age=i16::from_be_bytes(next[2..4].try_into().unwrap()).wrapping_add(1);next[2..4].copy_from_slice(&age.to_be_bytes());
 if age>=14{data.copy_from_slice(&next);return Ok(true);}
 let f=|i:usize|f32::from_be_bytes(data[i..i+4].try_into().unwrap());
 next[28..32].copy_from_slice(&(f(28)*math.constant(0x803af80c)).to_be_bytes());
 for i in 0..3{
  next[16+i*4..20+i*4].copy_from_slice(&(f(4+i*4)+f(16+i*4)).to_be_bytes());
  next[4+i*4..8+i*4].copy_from_slice(&(f(4+i*4)*math.constant(0x803af810)).to_be_bytes());
 }
 data.copy_from_slice(&next);Ok(false)
}

/// Post-allocation initialization80089a80..80089d0c, excluding task callback writes.
pub fn initialize(player:u8,center:[f32;3],velocity:[f32;3],seed:&mut u32,math:&OriginalMath)->Result<[u8;32]>{
 if player>1||center.iter().chain(velocity.iter()).any(|v|!v.is_finite()){return Err("invalid fire spawn input".into());}
 let mut next_seed=*seed;let mut d=[0;32];d[1]=player;
 let c=|o:u32|math.constant(0x803b6520-o);
 let mut random=||c(0x6de0)*crate::original_effect_state::random(&mut next_seed)as f32;
 let age=(c(0x6d24)*random())as i16;d[2..4].copy_from_slice(&age.to_be_bytes());
 for i in 0..3{let offset=(-c(0x6d80)).mul_add(random(),c(0x6d48));d[16+i*4..20+i*4].copy_from_slice(&(center[i]+offset).to_be_bytes());}
 let size=c(0x6d1c).mul_add(random(),c(0x6d20));d[28..32].copy_from_slice(&size.to_be_bytes());
 let (_,unit)=math.estimated_vector(velocity)?;let unit=unit.unwrap_or([f32::from_bits(0x7fc00000);3]);
 for i in 0..3{let noise=(-c(0x6da8)).mul_add(random(),c(0x6dac));let value=unit[i]*c(0x6d18)+noise;d[4+i*4..8+i*4].copy_from_slice(&value.to_be_bytes());}
 *seed=next_seed;Ok(d)
}

/// Rendering must not submit nonfinite vertices; source zero-velocity spawn
/// intentionally produces NaN velocity, then NaN position on its first update.
pub fn drawable(data:&[u8])->bool{
 data.len()==32&&(16..32).step_by(4).all(|i|f32::from_be_bytes(data[i..i+4].try_into().unwrap()).is_finite())
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn nonfinite_position_is_not_drawable(){let mut data=[0u8;32];data[28..32].copy_from_slice(&0.15f32.to_be_bytes());assert!(drawable(&data));data[16..20].copy_from_slice(&f32::NAN.to_be_bytes());assert!(!drawable(&data));assert!(!drawable(&data[..31]));}
}
