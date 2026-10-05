//! Original heap800e44a8..800e46cc and creation800e473c..800e47a8.
use crate::{disc::Result,original_contact::Memory};
pub struct HeapContext{pub table:u32,pub count:i32}
fn read(m:&[u8],base:u32,a:u32)->Result<u32>{Memory{base,bytes:m}.word(a)}
fn put(m:&mut[u8],base:u32,a:u32,v:u32)->Result<()>{
 let at=a.checked_sub(base).ok_or("heap write below mapped memory")?as usize;
 let end=at.checked_add(4).ok_or("heap address overflow")?;
 m.get_mut(at..end).ok_or("heap write outside mapped memory")?.copy_from_slice(&v.to_be_bytes());Ok(())
}
fn descriptor(ctx:&HeapContext,index:i32)->Result<u32>{
 if index<0||index>=ctx.count{return Err("heap index outside descriptor table".into());}
 Ok(ctx.table.wrapping_add((index as u32).wrapping_mul(12)))
}
pub fn create(base:u32,memory:&mut[u8],ctx:&HeapContext,start:u32,end:u32)->Result<i32>{
 let start=start.wrapping_add(31)&!31;let end=end&!31;
 let mut mem=memory.to_vec();
 for i in 0..ctx.count{
  let d=descriptor(ctx,i)?;
  if (read(&mem,base,d)?as i32)<0{
   let size=end.wrapping_sub(start);put(&mut mem,base,d,size)?;
   put(&mut mem,base,start,0)?;put(&mut mem,base,start.wrapping_add(4),0)?;
   let size=read(&mem,base,d)?;put(&mut mem,base,start.wrapping_add(8),size)?;
   put(&mut mem,base,d.wrapping_add(4),start)?;put(&mut mem,base,d.wrapping_add(8),0)?;
   memory.copy_from_slice(&mem);return Ok(i);
  }
 }
 Ok(-1)
}
pub fn allocate(base:u32,memory:&mut[u8],ctx:&HeapContext,index:i32,size:u32)->Result<u32>{
 let mut staged=memory.to_vec();let result=allocate_in_place(base,&mut staged,ctx,index,size)?;memory.copy_from_slice(&staged);Ok(result)
}
/// Internal caller must own rollback if a mapped-memory error occurs.
pub(crate) fn allocate_in_place(base:u32,memory:&mut[u8],ctx:&HeapContext,index:i32,size:u32)->Result<u32>{
 let d=descriptor(ctx,index)?;let mut mem=memory;let requested=size.wrapping_add(63)&!31;
 let mut block=read(&mem,base,d.wrapping_add(4))?;let mut budget=mem.len()/32+1;
 while block!=0{
  if budget==0{return Err("cyclic heap free list".into());}budget-=1;
  if (requested as i32)<=(read(&mem,base,block.wrapping_add(8))?as i32){break;}
  block=read(&mem,base,block.wrapping_add(4))?;
 }
 if block==0{return Ok(0);}
 let remainder=read(&mem,base,block.wrapping_add(8))?.wrapping_sub(requested);
 if remainder<64{
  let next=read(&mem,base,block.wrapping_add(4))?;let mut head=read(&mem,base,d.wrapping_add(4))?;
  if next!=0{let prev=read(&mem,base,block)?;put(&mut mem,base,next,prev)?;}
  let prev=read(&mem,base,block)?;
  if prev==0{head=read(&mem,base,block.wrapping_add(4))?;}
  else{let next=read(&mem,base,block.wrapping_add(4))?;put(&mut mem,base,prev.wrapping_add(4),next)?;}
  put(&mut mem,base,d.wrapping_add(4),head)?;
 }else{
  put(&mut mem,base,block.wrapping_add(8),requested)?;let rest=block.wrapping_add(requested);
  put(&mut mem,base,rest.wrapping_add(8),remainder)?;
  let prev=read(&mem,base,block)?;put(&mut mem,base,rest,prev)?;
  let next=read(&mem,base,block.wrapping_add(4))?;put(&mut mem,base,rest.wrapping_add(4),next)?;
  let next=read(&mem,base,rest.wrapping_add(4))?;if next!=0{put(&mut mem,base,next,rest)?;}
  let prev=read(&mem,base,rest)?;
  if prev!=0{put(&mut mem,base,prev.wrapping_add(4),rest)?;}else{put(&mut mem,base,d.wrapping_add(4),rest)?;}
 }
 let used=read(&mem,base,d.wrapping_add(8))?;put(&mut mem,base,block.wrapping_add(4),used)?;put(&mut mem,base,block,0)?;
 if used!=0{put(&mut mem,base,used,block)?;}put(&mut mem,base,d.wrapping_add(8),block)?;
 Ok(block.wrapping_add(32))
}
fn insert(base:u32,mem:&mut[u8],head:u32,block:u32)->Result<u32>{
 let(mut next,mut prev)=(head,0);let mut budget=mem.len()/32+1;
 while next!=0&&block>next{
  if budget==0{return Err("cyclic heap insertion list".into());}budget-=1;prev=next;next=read(mem,base,next.wrapping_add(4))?;
 }
 put(mem,base,block.wrapping_add(4),next)?;put(mem,base,block,prev)?;
 if next!=0{
  put(mem,base,next,block)?;let size=read(mem,base,block.wrapping_add(8))?;
  if block.wrapping_add(size)==next{
   let total=size.wrapping_add(read(mem,base,next.wrapping_add(8))?);put(mem,base,block.wrapping_add(8),total)?;
   next=read(mem,base,next.wrapping_add(4))?;put(mem,base,block.wrapping_add(4),next)?;if next!=0{put(mem,base,next,block)?;}
  }
 }
 if prev==0{return Ok(block);}
 put(mem,base,prev.wrapping_add(4),block)?;let size=read(mem,base,prev.wrapping_add(8))?;
 if prev.wrapping_add(size)==block{
  let total=size.wrapping_add(read(mem,base,block.wrapping_add(8))?);put(mem,base,prev.wrapping_add(8),total)?;
  put(mem,base,prev.wrapping_add(4),next)?;if next!=0{put(mem,base,next,prev)?;}
 }
 Ok(head)
}
pub fn free(base:u32,memory:&mut[u8],ctx:&HeapContext,index:i32,pointer:u32)->Result<()>{
 let d=descriptor(ctx,index)?;let block=pointer.wrapping_sub(32);let mut mem=memory.to_vec();
 let next=read(&mem,base,block.wrapping_add(4))?;let mut used=read(&mem,base,d.wrapping_add(8))?;
 if next!=0{let prev=read(&mem,base,block)?;put(&mut mem,base,next,prev)?;}
 let prev=read(&mem,base,block)?;
 if prev==0{used=read(&mem,base,block.wrapping_add(4))?;}else{let next=read(&mem,base,block.wrapping_add(4))?;put(&mut mem,base,prev.wrapping_add(4),next)?;}
 put(&mut mem,base,d.wrapping_add(8),used)?;let head=read(&mem,base,d.wrapping_add(4))?;
 let head=insert(base,&mut mem,head,block)?;put(&mut mem,base,d.wrapping_add(4),head)?;
 memory.copy_from_slice(&mem);Ok(())
}
