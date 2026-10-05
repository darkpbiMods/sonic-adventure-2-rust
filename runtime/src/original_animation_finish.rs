//! Action-facing forced blend completion8003d7d4..8003d8d0.
use crate::{disc::Result,original_contact::Memory,original_heap::HeapContext,
    original_math::OriginalMath,original_animation_support::{cleanup,reset_time}};
/// Registry maps801e5c48, as in the main updater. Mode2 adopts the active
/// entry's mode, releases channels, then performs the source reset. Other
/// modes are exact no-ops. Host errors roll back the entire transaction.
pub fn finish(state:u32,base:u32,memory:&mut[u8],registry:u32,heap:&HeapContext,index:i32,math:&OriginalMath)->Result<()>{
    let view=Memory{base,bytes:memory};
    if view.half(state)? as i16!=2{return Ok(());}
    let active=view.half(state.wrapping_add(4))? as i16 as i32;
    let entry=view.word(state.wrapping_add(0x24))?.wrapping_add(active.wrapping_mul(16) as u32);
    let mode=view.half(entry.wrapping_add(4))?;
    let mut m=memory.to_vec();let at=state.checked_sub(base).ok_or("animation state below memory")? as usize;
    m.get_mut(at..at+2).ok_or("animation mode outside memory")?.copy_from_slice(&mode.to_be_bytes());
    cleanup(state,base,&mut m,registry.wrapping_add(0xe0),heap,index)?;
    reset_time(state,base,&mut m,registry.wrapping_add(0xe0),math)?;
    memory.copy_from_slice(&m);Ok(())
}
