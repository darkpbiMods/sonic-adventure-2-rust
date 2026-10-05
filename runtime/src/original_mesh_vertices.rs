//! Vertex portion of8011d414, before polygon/material drawing.
use crate::{disc::Result,original_contact::Memory,original_vertex_stream};
/// Executes the source transpose/push, optional vertex stream, and pop contract.
/// The input current pointer is restored; its spare stack slot remains written.
/// Shared output must persist between meshes for weighted indexed vertices.
pub fn apply(base:u32,memory:&mut[u8],mesh:u32,current:u32,output:u32,normal_w:f32,flags:&mut u32)->Result<()>{
 let mut staged=memory.to_vec();let mut state=*flags;apply_in_place(base,&mut staged,mesh,current,output,normal_w,&mut state)?;memory.copy_from_slice(&staged);*flags=state;Ok(())
}
pub(crate) fn apply_in_place(base:u32,memory:&mut[u8],mesh:u32,current:u32,output:u32,normal_w:f32,flags:&mut u32)->Result<()>{
 let mut bytes=memory;let mut pointer=current;let mut state=*flags;
 original_vertex_stream::transpose_push(base,&mut bytes,&mut pointer)?;
 let stream=Memory{base,bytes:&bytes}.word(mesh)?;
 if stream!=0{original_vertex_stream::dispatch_in_place(base,&mut bytes,stream,output,pointer,normal_w,&mut state,4096)?;}
 // The original pops to current. Pointer is local; retain its matrix writes.
 *flags=state;Ok(())
}
