//! Startup table loop 801182cc..8011834c and lookups 80112398/801123b0.
use crate::{disc::Result,original_math::OriginalMath};
/// Compute the original table entry on demand. The startup loop uses twice
/// the table index, rounds the multiply and divide separately, then invokes
/// the same original sine/cosine routines reconstructed by OriginalMath.
pub fn entry(angle:u32,math:&OriginalMath)->Result<[f32;2]>{
 let index=angle&0xffff;
 let radians=(math.constant(0x803b6520-0x55f4)*((index*2)as f32))/math.constant(0x803b6520-0x55f0);
 let (sin,cos)=math.sin_cos(radians)?;Ok([sin,cos])
}
