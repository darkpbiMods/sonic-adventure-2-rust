//! Original collision contact graph queries, GSNE8P revision zero.
use crate::disc::Result;

/// Read-only guest-address view. The eventual host adapter must preserve the
/// source graph's references and record layout; invalid references are errors.
pub struct Memory<'a>{pub base:u32,pub bytes:&'a[u8]}
impl Memory<'_>{
    fn read<const N:usize>(&self,address:u32)->Result<[u8;N]>{
        let offset=address.checked_sub(self.base).ok_or("contact address precedes memory view")? as usize;
        let end=offset.checked_add(N).ok_or("contact address overflow")?;
        self.bytes.get(offset..end).ok_or_else(||"contact address outside memory view".into()).map(|b|b.try_into().unwrap())
    }
    pub(crate) fn byte(&self,a:u32)->Result<u8>{Ok(self.read::<1>(a)?[0])}
    pub(crate) fn half(&self,a:u32)->Result<u16>{Ok(u16::from_be_bytes(self.read(a)?))}
    pub(crate) fn word(&self,a:u32)->Result<u32>{Ok(u32::from_be_bytes(self.read(a)?))}
}

/// Complete 8004e924..8004e9c4. Returns the last qualifying task pointer among
/// at most sixteen contact records, stopping at the original signed -1 index.
pub fn select_contact(memory:&Memory<'_>,entity:u32)->Result<u32>{
    let collision=memory.word(entity.wrapping_add(0x2c))?;
    if memory.half(collision.wrapping_add(4))?&0x14!=0x14 || memory.half(entity.wrapping_add(4))?&4!=0{return Ok(0);}
    let mut selected=0;
    for i in 0..16 {
        let entry=collision.wrapping_add(0x10+i*8);
        let index=memory.byte(entry.wrapping_add(1))?as i8;
        if index == -1 {break;}
        let task=memory.word(entry.wrapping_add(4))?;
        let other_entity=memory.word(task.wrapping_add(0x34))?;
        let other_collision=memory.word(other_entity.wrapping_add(0x2c))?;
        let shapes=memory.word(other_collision.wrapping_add(0xc))?;
        let shape_flag=shapes.wrapping_add(((index as i32)*0x30+2)as u32);
        if memory.byte(shape_flag)?&1!=0 && memory.half(other_collision.wrapping_add(4))?&4!=0 {selected=task;}
    }
    Ok(selected)
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn malformed_contact_reference_returns_error(){
        let mut bytes=[0;0x80];bytes[0x2c..0x30].copy_from_slice(&0x80000040_u32.to_be_bytes());bytes[0x44..0x46].copy_from_slice(&0x14_u16.to_be_bytes());
        let m=Memory{base:0x80000000,bytes:&bytes};
        assert!(select_contact(&m,0x80000000).is_err());
        assert!(m.word(0xffffffff).is_err());
    }
}
