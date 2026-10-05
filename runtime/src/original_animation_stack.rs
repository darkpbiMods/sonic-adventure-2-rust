//! Original matrix stack80116098/801160e4 with explicit host storage bounds.
use crate::disc::Result;
pub fn push(base:u32,memory:&mut[u8],current:&mut u32)->Result<u32>{
 let source=current.checked_sub(base).ok_or("matrix pointer below host mapping")?as usize;
 let target=source.checked_add(48).ok_or("matrix stack offset overflow")?;
 let end=target.checked_add(48).ok_or("matrix stack extent overflow")?;
 if end>memory.len(){return Err("matrix push exceeds host capacity".into());}
 let next=current.checked_add(48).ok_or("matrix pointer overflow")?;
 memory.copy_within(source..source+48,target);*current=next;Ok(1)
}
pub fn pop(bottom:u32,current:&mut u32)->u32{let next=current.wrapping_sub(48);if next<bottom{0}else{*current=next;1}}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn overflow_is_transactional(){let mut mem=vec![0x55;96];let initial=mem.clone();let mut current=0x80200030;assert!(push(0x80200000,&mut mem,&mut current).is_err());assert_eq!(mem,initial);assert_eq!(current,0x80200030);}
}
