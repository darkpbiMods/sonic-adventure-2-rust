//! Matrix transpose8011c18c and vertex stream8011d1d4, including23/2a.
use crate::{disc::Result,original_contact::Memory,original_vertex_skinning};
fn word(base:u32,m:&[u8],a:u32)->Result<u32>{Memory{base,bytes:m}.word(a)}
fn put(base:u32,m:&mut[u8],a:u32,v:u32)->Result<()>{let i=a.checked_sub(base).ok_or("vertex write below mapping")?as usize;m.get_mut(i..i+4).ok_or("vertex write outside mapping")?.copy_from_slice(&v.to_be_bytes());Ok(())}
fn float(base:u32,m:&[u8],a:u32)->Result<f32>{Ok(f32::from_bits(word(base,m,a)?))}
pub fn transpose_push(base:u32,memory:&mut[u8],current:&mut u32)->Result<()>{let next=current.checked_add(48).ok_or("matrix pointer overflow")?;let mut values=[0;12];for i in 0..12{values[i]=word(base,memory,*current+i as u32*4)?;}let start=next.checked_sub(base).ok_or("matrix destination below mapping")? as usize;let end=start.checked_add(48).ok_or("matrix destination overflow")?;let target=memory.get_mut(start..end).ok_or("matrix destination outside mapping")?;for(i,index)in [0,4,8,1,5,9,2,6,10,3,7,11].into_iter().enumerate(){target[i*4..i*4+4].copy_from_slice(&values[index].to_be_bytes());}*current=next;Ok(())}
fn extra(base:u32,m:&mut[u8],source:u32,output:u32,count:u32,matrix:u32,kind:u8,normal_w:f32)->Result<()>{
 if kind==0x23 && count==0{return Err("zero type23 count has no verified termination".into());}
 let count=if kind==0x2a{(count as i32).max(0)as u32}else{count};let stride=if kind==0x23{16}else{28};let mut mat=[0.;12];for i in 0..12{mat[i]=float(base,m,matrix+i as u32*4)?;}
 for i in 0..count{let a=source+i*stride;let dest=output+i*32;let mut pos=[0.;3];for j in 0..3{pos[j]=float(base,m,a+j as u32*4)?;}
  if kind==0x23{let color=word(base,m,a+12)?.rotate_left(8);for axis in 0..3{let value=mat[6+axis].mul_add(pos[2],mat[3+axis].mul_add(pos[1],mat[axis].mul_add(pos[0],mat[9+axis])));put(base,m,dest+axis as u32*4,value.to_bits())?;}put(base,m,dest+24,color)?;}
  else{
   for (start,w)in [(0,1.),(12,normal_w)]{let mut v=[0.;3];for j in 0..3{v[j]=float(base,m,a+start+j as u32*4)?;}for r in 0..3{let row=&mat[r*4..r*4+4];let value=row[2].mul_add(v[2],row[0]*v[0])+row[3].mul_add(w,row[1]*v[1]);put(base,m,dest+start+r as u32*4,value.to_bits())?;}}
   let color=float(base,m,a+24)?;put(base,m,dest+24,color as u32)?;
  }
 }Ok(())
}
/// Preserves source stream order and global render flags. Unsupported header
/// kinds are skipped as in the source. Host budget bounds malformed streams.
/// Finite inputs and distinct source/output/matrix regions are verified.
pub fn dispatch(base:u32,memory:&mut[u8],stream:u32,output:u32,matrix:u32,normal_w:f32,flags:&mut u32,budget:usize)->Result<()>{
 let mut staged=memory.to_vec();let mut state=*flags;dispatch_in_place(base,&mut staged,stream,output,matrix,normal_w,&mut state,budget)?;memory.copy_from_slice(&staged);*flags=state;Ok(())
}
pub(crate) fn dispatch_in_place(base:u32,memory:&mut[u8],stream:u32,output:u32,matrix:u32,normal_w:f32,flags:&mut u32,budget:usize)->Result<()>{
 let mut m=memory;let mut state=*flags;let mut cursor=stream;
 for _ in 0..budget{let header=word(base,&m,cursor)?;let kind=header as u8;if kind==0xff{*flags=state;return Ok(());}
  let spec=word(base,&m,cursor+4)?;let dest=output.wrapping_add((spec&65535)*32);let count=spec>>16;
  match kind{
   0x29|0x2c=>{original_vertex_skinning::apply_in_place(base,&mut m,cursor+8,dest,count,matrix,kind,(header>>8)&0x7f)?;},
   0x23|0x2a=>{state|=if kind==0x23{3}else{2};extra(base,&mut m,cursor+8,dest,count,matrix,kind,normal_w)?;},_=>{}
  }
  cursor=cursor.wrapping_add(4).wrapping_add((header>>16)*4);
 }Err("vertex stream traversal budget exhausted".into())
}
