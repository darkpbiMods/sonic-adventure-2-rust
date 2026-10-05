//! Source key selection801139ac/801139f8 and linear80112f68/Euler80113154.
use crate::{disc::Result,original_contact::Memory};
/// Complete unsigned search. Caller supplies mapped records; empty counts retain
/// source index0 behavior, but sampling requires at least one mapped record.
pub fn search(memory:&Memory,keys:u32,stride:u32,count:u32,time:u32)->Result<u32>{
 let(mut low,mut high)=(0u32,count);let mut steps=0;
 while high.wrapping_sub(low)>1 {let middle=low.wrapping_add(high)>>1;let frame=memory.word(keys.wrapping_add(middle.wrapping_mul(stride)))?;
  if time<frame{high=middle;}else{low=middle;}steps+=1;if steps>64{return Err("key search outside terminating domain".into());}
 }Ok(low)
}
/// Source8010af3c conversion, finite animation-time domain.
fn unsigned_time(time:f32)->Result<u32>{if !time.is_finite(){return Err("nonfinite animation time".into());}Ok(if time<0.0{0}else if time>=4294967296.0{u32::MAX}else{time.trunc()as u32})}
pub fn interval(memory:&Memory,keys:u32,stride:u32,count:u32,time:f32)->Result<(u32,u32,f32)>{
 let index=search(memory,keys,stride,count,unsigned_time(time)?)?;
 let first=keys.wrapping_add(index.wrapping_mul(stride));let second=if index<count.wrapping_sub(1){keys.wrapping_add(index.wrapping_add(1).wrapping_mul(stride))}else{keys};
 let frame=memory.word(first)?;let delta=memory.word(second)?.wrapping_sub(frame)as i32;
 let distance=time-frame as f32;let fraction=if delta>0{distance/delta as f32}else{distance};
 Ok((first,second,fraction))
}
pub fn linear(memory:&Memory,keys:u32,count:u32,time:f32)->Result<[u32;3]>{
 let(a,b,t)=interval(memory,keys,16,count,time)?;let mut out=[0;3];
 for i in 0..3 {let x=f32::from_bits(memory.word(a+4+i as u32*4)?);let y=f32::from_bits(memory.word(b+4+i as u32*4)?);let difference=y-x;let product=t*difference;out[i]=(x+product).to_bits();}Ok(out)
}
pub fn euler(memory:&Memory,keys:u32,count:u32,time:f32)->Result<[u32;3]>{
 let(a,b,t)=interval(memory,keys,16,count,time)?;let mut out=[0;3];
 for i in 0..3 {let x=memory.word(a+4+i as u32*4)?;let y=memory.word(b+4+i as u32*4)?;let delta=y.wrapping_sub(x)as i16 as f32;let product=t*delta;let value=x as i32 as f32+product;
  // Gekko fctiwz saturates finite positive overflow; invalid/negative overflow
  // returns INT_MIN. Keep this distinct from Rust's NaN-to-zero cast.
  out[i]=if value.is_nan()||value<=-2147483648.0{0x80000000}else if value>=2147483648.0{0x7fffffff}else{value.trunc()as i32 as u32};
 }Ok(out)
}
