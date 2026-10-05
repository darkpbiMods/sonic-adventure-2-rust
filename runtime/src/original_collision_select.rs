//! Original collision response selection parents. Geometry response is separate.
use crate::{disc::Result,original_contact::Memory,original_cone::nearest,original_math::OriginalMath,original_response::classify};
#[derive(Debug)]pub struct Selection{pub flags:u32,pub cached_task:u32}
fn player_task(memory:&Memory<'_>,character:u32,players:[u32;2])->Result<u32>{
    let player=memory.byte(character)?as i8;
    players.get(player as usize).copied().ok_or_else(||"unsupported original player slot".into())
}
/// Complete 8004f1f8..8004f2d8. Returned cached_task is the source write to
/// character+9c; the host commits it after successful execution.
pub fn select(memory:&Memory<'_>,entity:u32,character:u32,players:[u32;2],cache:&mut[u32;4],math:&OriginalMath)->Result<Selection>{
    let mut next=*cache;let old=memory.word(character.wrapping_add(0x9c))?;
    let mut selected=old;
    let flags=if old!=0 {
        let other=memory.word(old.wrapping_add(0x34))?;
        classify(memory,other,entity,character,players,&mut next)?
    }else{
        let task=player_task(memory,character,players)?;
        let direction=0x4000_u32.wrapping_sub(memory.word(entity.wrapping_add(0xc))?);
        let target=nearest(memory,task,direction,0x3000,math)?;
        if target==0 {0}else{
            let other=memory.word(target.wrapping_add(0x34))?;
            let status=memory.half(other.wrapping_add(4))?;
            if status&0x40==0 || status&0x8000!=0 {0}else{
                let flags=classify(memory,other,entity,character,players,&mut next)?;
                if flags!=0{selected=target;}
                flags
            }
        }
    };
    *cache=next;Ok(Selection{flags,cached_task:selected})
}
/// Complete 8004f11c..8004f1f8: attached-object and ordinary contact paths.
pub fn dispatch(memory:&Memory<'_>,entity:u32,character:u32,players:[u32;2],cache:&mut[u32;4],math:&OriginalMath)->Result<Selection>{
    let status=memory.half(entity.wrapping_add(4))?;
    let old=memory.word(character.wrapping_add(0x9c))?;
    if status&0x800!=0 {
        let task=memory.word(character.wrapping_add(0x94))?;
        let other=memory.word(task.wrapping_add(0x34))?;
        let motion=memory.word(task.wrapping_add(0x38))?;
        let selected=if old==0{task}else{old};
        let flags=if memory.word(task.wrapping_add(0x10))?==0x80071af0 || motion==0 || other==0 {0}
            else if memory.word(other.wrapping_add(0x2c))?==0 {0}
            else{classify(memory,other,entity,character,players,cache)?};
        Ok(Selection{flags,cached_task:selected})
    }else{
        let collision=memory.word(entity.wrapping_add(0x2c))?;
        let flags=memory.half(collision.wrapping_add(4))?;
        if flags&1!=0 && status&4==0 && flags&4!=0 {select(memory,entity,character,players,cache,math)}
        else{Ok(Selection{flags:0,cached_task:old})}
    }
}
