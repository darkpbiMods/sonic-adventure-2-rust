//! Original 800424c8..80042694 platform-motion parent.
//! Indirect calls are explicit. Callers must provide recovered callback code;
//! there is deliberately no default successful/no-op callback.
use crate::{disc::Result,original_math::OriginalMath};

#[derive(Clone)]
pub struct PlatformMemory { pub base:u32, pub bytes:Vec<u8> }
impl PlatformMemory {
    fn index(&self,address:u32,n:usize)->Result<usize>{
        let i=address.checked_sub(self.base).ok_or("platform address below memory")? as usize;
        if i.checked_add(n).is_none_or(|end|end>self.bytes.len()){return Err("platform address outside memory".into());}
        Ok(i)
    }
    pub fn word(&self,address:u32)->Result<u32>{let i=self.index(address,4)?;Ok(u32::from_be_bytes(self.bytes[i..i+4].try_into().unwrap()))}
    pub fn put_word(&mut self,address:u32,value:u32)->Result<()>{let i=self.index(address,4)?;self.bytes[i..i+4].copy_from_slice(&value.to_be_bytes());Ok(())}
    pub fn float(&self,address:u32)->Result<f32>{Ok(f32::from_bits(self.word(address)?))}
    pub fn put_float(&mut self,address:u32,value:f32)->Result<()>{self.put_word(address,value.to_bits())}
}
#[derive(Clone,Copy)]
pub struct PlatformCall {pub entity:u32,pub character:u32,pub workspace:u32,pub offset:u32,pub player_tasks:[u32;2]}

/// Callback receives its actual source address and source r3/r4/r5 arguments.
/// All guest side effects must stay in `memory`; errors roll them back together
/// with the parent. The callback may alter slot deltas and entity angles.
pub fn apply<F>(memory:&mut PlatformMemory,call:PlatformCall,math:&OriginalMath,mut callback:F)->Result<bool>
where F:FnMut(u32,u32,u32,u32,&mut PlatformMemory)->Result<()> {
    let mut next=memory.clone();
    let task=next.word(call.character.wrapping_add(0xa0))?;
    let slots=next.word(task.wrapping_add(0x3c))?;
    if slots==0{return Ok(false);}
    let player=next.bytes[next.index(call.character,1)?] as i8;
    let slot=slots.wrapping_add(if player==0{0}else{0x1c});
    let address=next.word(slot)?;
    if address!=0 {
        if !(0..=1).contains(&player){return Err("platform callback player outside two source slots".into());}
        let mut saved=[0;3];
        for i in 0..3 {
            let n=i as u32*4;saved[i]=next.word(call.entity.wrapping_add(0x14+n))?;
            let value=next.float(call.workspace.wrapping_add(4+n))?-next.float(call.offset.wrapping_add(n))?;
            next.put_float(call.entity.wrapping_add(0x14+n),value)?;
        }
        callback(address,call.player_tasks[player as usize],call.entity,slot,&mut next)?;
        for i in 0..3 {
            let n=i as u32*4;
            let value=next.float(call.entity.wrapping_add(0x14+n))?+next.float(call.offset.wrapping_add(n))?;
            next.put_float(call.workspace.wrapping_add(4+n),value)?;
        }
        for i in 0..3{next.put_word(call.entity.wrapping_add(0x14+i as u32*4),saved[i])?;}
    }
    for n in [0,4,8] {
        let value=next.float(call.workspace.wrapping_add(4+n))?+next.float(slot.wrapping_add(0x10+n))?;
        next.put_float(call.workspace.wrapping_add(4+n),value)?;
    }
    for n in [0,4,8] {
        let value=next.word(call.entity.wrapping_add(8+n))?.wrapping_add(next.word(slot.wrapping_add(4+n))?)&0xffff;
        next.put_word(call.entity.wrapping_add(8+n),value)?;
    }
    let delta=[next.float(slot.wrapping_add(0x10))?,next.float(slot.wrapping_add(0x14))?,next.float(slot.wrapping_add(0x18))?];
    let moved=math.estimated_vector(delta)?.0!=0.;
    *memory=next;Ok(moved)
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn callback_failure_rolls_back_all_guest_effects(){
  let math=OriginalMath::from_dol(std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../source/main.dol")).unwrap()).unwrap();
  let b=0x80200000;let mut m=PlatformMemory{base:b,bytes:vec![0;0x1000]};
  m.put_word(b+0x2a0,b+0x600).unwrap();m.put_word(b+0x63c,b+0x800).unwrap();m.put_word(b+0x800,0xdeadbeef).unwrap();
  m.put_float(b+0x114,19.).unwrap();m.put_float(b+0x904,8.).unwrap();
  let before=m.bytes.clone();let call=PlatformCall{entity:b+0x100,character:b+0x200,workspace:b+0x900,offset:b+0xa00,player_tasks:[b+0xb00,b+0xb40]};
  let result=apply(&mut m,call,&math,|address,_,entity,_,m|{
   assert_eq!(address,0xdeadbeef);assert_eq!(m.float(entity+0x14)?,8.);
   m.put_word(entity+8,123)?;Err("unrecovered callback".into())
  });
  assert!(result.is_err());assert_eq!(m.bytes,before);
 }
}
