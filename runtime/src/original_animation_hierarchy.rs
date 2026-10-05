//! Original hierarchy walker 8012042c. Mesh callback is supplied by the host.
use crate::{disc::Result,original_contact::Memory,original_math::{Matrix,OriginalMath},original_animation_transform,original_animation_stack};
pub trait Callbacks {
 fn node(&mut self,base:u32,memory:&mut[u8],node:u32,current:u32)->Result<()>;
 fn mesh(&mut self,base:u32,memory:&mut[u8],mesh:u32,current:u32)->Result<()>;
}
fn read_matrix(base:u32,m:&[u8],a:u32)->Result<Matrix>{let mut v=[[0.;4];3];for r in 0..3{for c in 0..4{v[r][c]=f32::from_bits(Memory{base,bytes:m}.word(a+(r*4+c)as u32*4)?);}}Ok(v)}
fn write_matrix(base:u32,m:&mut[u8],a:u32,v:Matrix)->Result<()>{let i=a.checked_sub(base).ok_or("matrix below mapping")?as usize;let dst=m.get_mut(i..i+48).ok_or("matrix outside mapping")?;for (part,x)in dst.chunks_exact_mut(4).zip(v.into_iter().flatten()){part.copy_from_slice(&x.to_bits().to_be_bytes());}Ok(())}
/// Host traversal budget bounds malformed cycles. Errors can leave partial state,
/// including callback side effects; callers needing rollback must stage the walk.
pub fn walk(base:u32,memory:&mut[u8],root:u32,context:&mut[u32;16],bottom:u32,current:&mut u32,table:&[[f32;2]],math:&OriginalMath,callbacks:&mut impl Callbacks,budget:usize)->Result<()>{
 // Explicit continuations preserve the source's reads after child and pop,
 // without depending on the host call stack for deeply nested model trees.
 enum Step{Enter(u32),Leave(u32)}
 let mut pending=vec![Step::Enter(root)];let mut remaining=budget;
 while let Some(step)=pending.pop(){match step{
  Step::Enter(node)=>{
   if remaining==0{return Err("model traversal budget exhausted".into());}remaining-=1;
   original_animation_stack::push(base,memory,current)?;
   let mut matrix=read_matrix(base,memory,*current)?;
   original_animation_transform::apply(&Memory{base,bytes:memory},node,context,&mut matrix,table,math)?;
   write_matrix(base,memory,*current,matrix)?;
   callbacks.node(base,memory,node,*current)?;
   let mesh=Memory{base,bytes:memory}.word(node+4)?;
   if mesh!=0 && (Memory{base,bytes:memory}).word(node)?&8==0{callbacks.mesh(base,memory,mesh,*current)?;}
   pending.push(Step::Leave(node));
   let child=Memory{base,bytes:memory}.word(node+0x2c)?;
   if child!=0{pending.push(Step::Enter(child));}
  },
  Step::Leave(node)=>{original_animation_stack::pop(bottom,current);let sibling=Memory{base,bytes:memory}.word(node+0x30)?;if sibling!=0{pending.push(Step::Enter(sibling));}}
 }}Ok(())
}
