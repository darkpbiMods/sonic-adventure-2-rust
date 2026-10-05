//! Original collision-response classification, GSNE8P revision zero.
use crate::{disc::Result,original_contact::Memory,original_contact_search::find_task};

/// Complete 8004f3c8..8004f4d0. The two player-task pointers are explicit
/// inputs for the original global array at 801e7788. Cursor commits atomically
/// on successful host execution; valid source inputs retain source ordering.
pub fn classify(memory:&Memory<'_>,other_entity:u32,entity:u32,character:u32,player_tasks:[u32;2],cache:&mut[u32;4])->Result<u32>{
    let character_type=memory.byte(character.wrapping_add(1))?as i8;
    let response=memory.byte(other_entity.wrapping_add(2))?;
    if !(0..=5).contains(&character_type) && character_type!=8{return Ok(0);}
    match response {
        10|12=>Ok(0x200),13=>Ok(0x800),14=>Ok(0x1000),
        11=>{
            let player=memory.byte(character)?as i8;
            let task=*player_tasks.get(player as usize).ok_or("unsupported original player slot")?;
            let mut next=*cache;
            let flags=if find_task(memory,task,15,&mut next)?!=0 || find_task(memory,task,14,&mut next)?==0 || memory.half(entity.wrapping_add(4))?&0x800!=0 {0x400}else{0};
            *cache=next;Ok(flags)
        },
        _=>Ok(0),
    }
}
