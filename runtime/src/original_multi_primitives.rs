//! Multi-texture/color strip emitter80120158..801203e0.
use crate::{disc::Result,original_contact::Memory,original_primitives::Output};
fn word(base:u32,m:&[u8],a:u32)->Result<u32>{Memory{base,bytes:m}.word(a)}
fn advance(base:u32,m:&mut[u8],slot:u32,n:u32)->Result<u32>{let old=word(base,m,slot)?;let i=slot.checked_sub(base).ok_or("stream slot below mapping")?as usize;m.get_mut(i..i+4).ok_or("stream slot outside mapping")?.copy_from_slice(&old.wrapping_add(n).to_be_bytes());Ok(old)}
/// One to eight texture coordinate streams; input-pointer updates are part of
/// the result. Unsupported counts and malformed input leave memory unchanged.
/// Only stream0 supplies strip lengths and position/color indices. Secondary
/// indices and strip lengths are consumed without affecting emitted geometry.
pub fn emit(base:u32,memory:&mut[u8],streams:u32,vertices:u32,count_flags:u32,formats:u32,additional:u32)->Result<Output>{
 if additional>7{return Err("unverified texture count above eight".into());}
 let count=count_flags&0x3fff;if count==0{return Err("zero multi strip count".into());}
 let mut m=memory.to_vec();let mut out=Output{setup:Vec::new(),fifo:Vec::new()};
 for i in 0..=additional{out.setup.push(vec![1,0,13+i,1,3,if word(base,&m,formats+i*4)?!=0{12}else{8}]);}
 out.setup.extend([vec![2],vec![3,9,1],vec![3,11,1]]);for i in 0..=additional{out.setup.push(vec![3,13+i,1]);}
 for _ in 0..count{
  let header=advance(base,&mut m,streams,2)?;let length=(Memory{base,bytes:&m}.half(header)?as i16 as i32).unsigned_abs();
  for i in 1..=additional{advance(base,&mut m,streams+i*4,2)?;}
  if length==0{return Err("zero multi vertex count".into());}out.fifo.push(0x98);out.fifo.extend_from_slice(&(length as u16).to_be_bytes());
  for _ in 0..length{
   let a=advance(base,&mut m,streams,2)?;let index=Memory{base,bytes:&m}.half(a)?as i16 as i32;let v=vertices.wrapping_add(index.wrapping_mul(32)as u32);
   for j in 0..3{out.fifo.extend_from_slice(&word(base,&m,v+j*4)?.to_be_bytes());}out.fifo.extend_from_slice(&word(base,&m,v+24)?.to_be_bytes());
   for i in 0..=additional{let slot=streams+i*4;if i!=0{advance(base,&mut m,slot,2)?;}for _ in 0..2{let a=advance(base,&mut m,slot,2)?;out.fifo.extend_from_slice(&Memory{base,bytes:&m}.half(a)?.to_be_bytes());}}
  }
 }
 memory.copy_from_slice(&m);Ok(out)
}
