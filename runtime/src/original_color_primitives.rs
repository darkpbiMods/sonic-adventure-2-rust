//! Color strip emitters8011fef4/8011fff4 as a host command payload.
use crate::{disc::Result,original_contact::Memory};
use crate::original_primitives::Output;
/// Setup opcodes:1=GXSetVtxAttrFmt,2=GXClearVtxDesc,3=GXSetVtxDesc.
/// FIFO is primitive-only (begin + vertex payload), not GX dirty-state writes.
/// Finite vertex attributes are verified. Zero strip/vertex counts are rejected
/// because the original decrement loops have no verified terminating zero case.
pub fn emit(memory:&Memory,stream:u32,vertices:u32,count_flags:u32,textured:bool,high_precision:bool)->Result<Output>{
 let count=count_flags&0x3fff;if count==0{return Err("zero primitive strip count".into());}
 let mut out=Output{setup:Vec::new(),fifo:Vec::new()};
 if textured{out.setup.push(vec![1,0,13,1,3,if high_precision{12}else{8}]);}
 out.setup.extend([vec![2],vec![3,9,1],vec![3,11,1]]);if textured{out.setup.push(vec![3,13,1]);}
 let mut cursor=stream;let extra=((count_flags>>14)&3)*2;
 for _ in 0..count{let length=(memory.half(cursor)?as i16 as i32).unsigned_abs();cursor=cursor.wrapping_add(2);if length==0{return Err("zero primitive vertex count".into());}
  out.fifo.push(0x98);out.fifo.extend_from_slice(&(length as u16).to_be_bytes());
  for _ in 0..length{let index=memory.half(cursor)?as i16 as i32;cursor=cursor.wrapping_add(2);let vertex=vertices.wrapping_add(index.wrapping_mul(32)as u32);
   for j in 0..3{out.fifo.extend_from_slice(&memory.word(vertex+j*4)?.to_be_bytes());}
   out.fifo.extend_from_slice(&memory.word(vertex+24)?.to_be_bytes());
   if textured{out.fifo.extend_from_slice(&memory.half(cursor)?.to_be_bytes());out.fifo.extend_from_slice(&memory.half(cursor+2)?.to_be_bytes());cursor=cursor.wrapping_add(4);}cursor=cursor.wrapping_add(extra);
  }
 }Ok(out)
}
