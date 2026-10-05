//! Original stateful collision-type lookup dependencies.
use crate::{disc::Result,original_contact::Memory};

/// Original 80006ce8..80006d7c global cursor update. Four words correspond to
/// 801df680: task, caller routine, index and type tag. Retains the source's
/// assignment of -1 to the tag on mismatch, rather than substituting the type.
fn advance(cache:&mut[u32;4],task:u32,caller:u32,kind:u32)->u32{
    if cache[0]!=task {cache[0]=task;cache[1]=0;cache[2]=u32::MAX;cache[3]=u32::MAX;}
    if cache[1]!=caller {cache[1]=caller;cache[2]=u32::MAX;cache[3]=u32::MAX;}
    if cache[3]!=kind {cache[3]=u32::MAX;cache[2]=u32::MAX;}
    cache[2]=cache[2].wrapping_add(1);
    if cache[2]as i32>16{cache[2]=16;}
    cache[2]
}

/// Complete 80005fbc..800060ec. Host failures leave the cursor unchanged.
pub fn find_record(memory:&Memory<'_>,task:u32,kind:u32,cache:&mut[u32;4])->Result<u32>{
    let mut next=*cache;
    let result=find_inner(memory,task,kind as u8,&mut next)?;
    *cache=next;Ok(result)
}
fn find_inner(memory:&Memory<'_>,task:u32,kind:u8,cache:&mut[u32;4])->Result<u32>{
    let mut index=advance(cache,task,0x80005fbc,kind as u32);
    if task==0{return Ok(0);}
    let entity=memory.word(task.wrapping_add(0x34))?;
    if entity==0{return Ok(0);}
    let collision=memory.word(entity.wrapping_add(0x2c))?;
    if collision==0 || memory.half(collision.wrapping_add(4))?&1==0{return Ok(0);}
    if (index as i32)<0{return Err("negative contact cursor outside valid source state".into());}
    while index<16 {
        let entry=collision.wrapping_add(0x10+index*8);
        let shape_index=memory.byte(entry.wrapping_add(1))?as i8;
        let other_task=memory.word(entry.wrapping_add(4))?;
        if shape_index == -1{return Ok(0);}
        if other_task!=0 {
            let other_entity=memory.word(other_task.wrapping_add(0x34))?;
            if other_entity!=0 {
                let other_collision=memory.word(other_entity.wrapping_add(0x2c))?;
                if other_collision!=0 {
                    let shapes=memory.word(other_collision.wrapping_add(0xc))?;
                    let address=shapes.wrapping_add(((shape_index as i32)*0x30)as u32);
                    if memory.byte(address)?==kind {cache[2]=index;return Ok(entry);}
                }
            }
        }
        index+=1;
    }
    Ok(0)
}

/// Complete 800060ec..80006120 wrapper, returning the matched task pointer.
pub fn find_task(memory:&Memory<'_>,task:u32,kind:u32,cache:&mut[u32;4])->Result<u32>{
    let mut next=*cache;
    let entry=find_record(memory,task,kind,&mut next)?;
    let result=if entry==0{0}else{memory.word(entry.wrapping_add(4))?};
    *cache=next;Ok(result)
}
