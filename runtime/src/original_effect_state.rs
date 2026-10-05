//! Original effect initialization/lifetime. Allocation and drawing are separate.
use crate::{disc::Result,original_contact::Memory,original_math::OriginalMath};
pub struct EffectContext{
 pub players:[u32;2],pub characters:[u32;2],
 /// Globals801e6724,801e742c,801e692c in source branch order.
 pub models:[u32;3],
 /// Globals r13-7df8/-7df0/-7de8/-7de0: Sonic, Shadow, types8/12.
 pub materials:[u32;4],
}
fn put(memory:&mut[u8],base:u32,address:u32,bytes:&[u8])->Result<()>{
 let at=address.checked_sub(base).ok_or("effect write below mapped memory")?as usize;
 let end=at.checked_add(bytes.len()).ok_or("effect write overflow")?;
 memory.get_mut(at..end).ok_or("effect write outside mapped memory")?.copy_from_slice(bytes);Ok(())
}
fn word(memory:&[u8],base:u32,address:u32)->Result<u32>{Memory{base,bytes:memory}.word(address)}
/// Entire8010dc88..8010dcac, preserving the32-bit global seed update.
pub fn random(seed:&mut u32)->u32{*seed=seed.wrapping_mul(0x41c64e6d).wrapping_add(0x3039);(*seed>>16)&0x7fff}
/// Entire8008ada8..8008b000. Valid guest writes and RNG commit atomically.
pub fn initialize(task:u32,base:u32,memory:&mut[u8],ctx:&EffectContext,seed:&mut u32,math:&OriginalMath)->Result<()>{
 let mut mem=memory.to_vec();let mut next_seed=*seed;let data=word(&mem,base,task.wrapping_add(0x40))?;
 let player=Memory{base,bytes:&mem}.byte(data.wrapping_add(1))?as i8;
 if !(0..2).contains(&player){return Err("effect player outside mapped player slots".into());}let player=player as usize;
 let object=ctx.players[player];
 if object!=0{
  let collision=word(&mem,base,object.wrapping_add(0x2c))?;
  if collision!=0{
   let shape=word(&mem,base,collision.wrapping_add(0xc))?;
   // Source loads two words before storing them; preserve the same ordering.
   let x=word(&mem,base,shape.wrapping_add(8))?;let y=word(&mem,base,shape.wrapping_add(12))?;
   put(&mut mem,base,data.wrapping_add(4),&x.to_be_bytes())?;put(&mut mem,base,data.wrapping_add(8),&y.to_be_bytes())?;
   let z=word(&mem,base,shape.wrapping_add(16))?;put(&mut mem,base,data.wrapping_add(12),&z.to_be_bytes())?;
  }
 }
 put(&mut mem,base,data.wrapping_add(0x1c),&math.constant(0x803b6520-0x6cf8).to_bits().to_be_bytes())?;
 put(&mut mem,base,data.wrapping_add(0x18),&1u32.to_be_bytes())?;
 let character=ctx.characters[player];let kind=Memory{base,bytes:&mem}.byte(character.wrapping_add(3))?as i8;
 let(model,material)=if kind==8{(ctx.models[0],ctx.materials[2])}else if kind==12{(ctx.models[1],ctx.materials[3])}
 else if (Memory{base,bytes:&mem}).byte(character.wrapping_add(1))?==0{(ctx.models[0],ctx.materials[0])}else{(ctx.models[2],ctx.materials[1])};
 put(&mut mem,base,data.wrapping_add(0x10),&model.to_be_bytes())?;
 let value=math.constant(0x803b6520-0x6de0)*(random(&mut next_seed)as f32);
 let material=if value>math.constant(0x803b6520-0x6d50){material}else{material.wrapping_add(12)};
 put(&mut mem,base,data.wrapping_add(0x14),&material.to_be_bytes())?;
 put(&mut mem,base,task.wrapping_add(0x1c),&0x8008b0ccu32.to_be_bytes())?;
 memory.copy_from_slice(&mem);*seed=next_seed;Ok(())
}
/// Entire8008b000..8008b06c or8008b06c..8008b0cc, including80071ae0.
/// Deferred deletion installs the original destructor; it does not free now.
pub fn update(task:u32,base:u32,memory:&mut[u8],fading:bool,math:&OriginalMath)->Result<()>{
 let mut mem=memory.to_vec();let data=word(&mem,base,task.wrapping_add(0x40))?;
 let delete=if fading{
  let value=f32::from_bits(word(&mem,base,data.wrapping_add(0x1c))?)-math.constant(0x803b6520-0x6dac);
  put(&mut mem,base,data.wrapping_add(0x1c),&value.to_bits().to_be_bytes())?;
  value<=math.constant(0x803b6520-0x6dec)
 }else{
  let timer=Memory{base,bytes:&mem}.half(data.wrapping_add(2))?as i16;
  if timer>0{
   let value=f32::from_bits(word(&mem,base,data.wrapping_add(0x1c))?)-math.constant(0x803b6520-0x6d6c);
   put(&mut mem,base,data.wrapping_add(0x1c),&value.to_bits().to_be_bytes())?;
  }
  let timer=Memory{base,bytes:&mem}.half(data.wrapping_add(2))?as i16;
  put(&mut mem,base,data.wrapping_add(2),&timer.wrapping_add(1).to_be_bytes())?;timer>2
 };
 if delete{
  put(&mut mem,base,task.wrapping_add(0x10),&0x80071af0u32.to_be_bytes())?;
  put(&mut mem,base,task.wrapping_add(0x1c),&0u32.to_be_bytes())?;
 }
 memory.copy_from_slice(&mem);Ok(())
}
