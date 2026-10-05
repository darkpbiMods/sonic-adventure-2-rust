//! Animation cleanup8003c908..8003ca00 and reset8003d720..8003d7d4.
use crate::{disc::Result,original_contact::Memory,original_heap::{self,HeapContext},original_math::OriginalMath};
fn word(m:&[u8],base:u32,a:u32)->Result<u32>{Memory{base,bytes:m}.word(a)}
fn half(m:&[u8],base:u32,a:u32)->Result<u16>{Memory{base,bytes:m}.half(a)}
fn put(m:&mut[u8],base:u32,a:u32,b:&[u8])->Result<()>{
 let start=a.checked_sub(base).ok_or("animation address below mapped memory")?as usize;
 let end=start.checked_add(b.len()).ok_or("animation address overflow")?;
 m.get_mut(start..end).ok_or("animation write outside mapped memory")?.copy_from_slice(b);Ok(())
}
fn entry(m:&[u8],base:u32,state:u32,selected:u32,registry:u32)->Result<u32>{
 let id=half(m,base,state.wrapping_add(selected))?as i16 as i32;
 let table=word(m,base,state.wrapping_add(0x24))?;
 let motion=half(m,base,table.wrapping_add(id.wrapping_mul(16)as u32))?as i16 as i32;
 Ok(registry.wrapping_add(motion.wrapping_mul(8)as u32))
}
/// Registry is the mapped equivalent of global801e5d28. Buffer pairing and
/// null-pointer behavior follow the source; unallocated partner words persist.
pub fn cleanup(state:u32,base:u32,memory:&mut[u8],registry:u32,heap:&HeapContext,index:i32)->Result<()>{
 let mut mem=memory.to_vec();let motion=entry(&mem,base,state,2,registry)?;
 let header=word(&mem,base,state.wrapping_add(0x28))?;
 let channels=half(&mem,base,header.wrapping_add(0xa))?as u32;
 let mut buffers=word(&mem,base,header)?;let count=half(&mem,base,motion.wrapping_add(2))?as i16;
 let paired=channels.wrapping_mul(4);
 for _ in 0..count.max(0){
  for channel in 0..if channels==3{3}else{2}{
   let slot=buffers.wrapping_add(channel*4);let pointer=word(&mem,base,slot)?;
   if pointer!=0{
    original_heap::free(base,&mut mem,heap,index,pointer)?;
    put(&mut mem,base,slot,&0u32.to_be_bytes())?;put(&mut mem,base,slot.wrapping_add(paired),&0u32.to_be_bytes())?;
   }
  }
  buffers=buffers.wrapping_add(if channels==3{24}else{16});
 }
 memory.copy_from_slice(&mem);Ok(())
}
pub fn reset_time(state:u32,base:u32,memory:&mut[u8],registry:u32,math:&OriginalMath)->Result<()>{
 let mut mem=memory.to_vec();let mode=half(&mem,base,state)?as i16;
 let value=if mode==7||mode==8{
  let motion=entry(&mem,base,state,4,registry)?;
  let data=word(&mem,base,motion.wrapping_add(4))?;let frames=word(&mem,base,data.wrapping_add(4))?;
  ((frames as f32)-math.constant(0x803b6520-0x75c8)).to_bits()
 }else if [3,10,11,13].contains(&mode){
  let value=word(&mem,base,state.wrapping_add(0x18))?;
  put(&mut mem,base,state.wrapping_add(0x10),&value.to_be_bytes())?;
  put(&mut mem,base,state.wrapping_add(0x18),&math.constant(0x803b6520-0x75f8).to_bits().to_be_bytes())?;
  value
 }else{math.constant(0x803b6520-0x75f8).to_bits()};
 put(&mut mem,base,state.wrapping_add(0x10),&value.to_be_bytes())?;
 put(&mut mem,base,state.wrapping_add(0xa),&0u16.to_be_bytes())?;
 memory.copy_from_slice(&mem);Ok(())
}
