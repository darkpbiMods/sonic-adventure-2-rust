//! Special strip paths8011d6a4/8011d91c; texture coordinates generated upstream.
use crate::{disc::Result,original_contact::Memory,original_primitives::Output};
/// Both paths emit position and normal only. The textured path additionally
/// skips four source UV bytes, and both honor extra per-vertex halfwords.
/// This recovers primitive payload, not upstream texture-generation state.
pub fn emit(memory:&Memory,stream:u32,vertices:u32,count_flags:u32,textured:bool)->Result<Output>{
 let count=count_flags&0x3fff;if count==0{return Err("zero special strip count".into());}
 let mut out=Output{setup:vec![vec![2],vec![3,9,1],vec![3,10,1]],fifo:Vec::new()};
 let mut cursor=stream;let skip=((count_flags>>14)&3)*2+if textured{4}else{0};
 for _ in 0..count{let length=(memory.half(cursor)?as i16 as i32).unsigned_abs();cursor=cursor.wrapping_add(2);if length==0{return Err("zero special vertex count".into());}
  out.fifo.push(0x98);out.fifo.extend_from_slice(&(length as u16).to_be_bytes());
  for _ in 0..length{let index=memory.half(cursor)?as i16 as i32;cursor=cursor.wrapping_add(2);let vertex=vertices.wrapping_add(index.wrapping_mul(32)as u32);
   for j in 0..6{out.fifo.extend_from_slice(&memory.word(vertex+j*4)?.to_be_bytes());}cursor=cursor.wrapping_add(skip);
  }
 }Ok(out)
}
