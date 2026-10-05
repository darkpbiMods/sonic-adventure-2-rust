//! Vertex kernels8011daf0 (29) and8011dc98 (2c).
//! Matrix input is transposed to [m00,m10,m20,m01,...,tx,ty,tz].
use crate::{disc::Result,original_contact::Memory};
fn f(m:&Memory,a:u32)->Result<f32>{Ok(f32::from_bits(m.word(a)?))}
fn put(base:u32,m:&mut[u8],a:u32,value:f32)->Result<()>{let i=a.checked_sub(base).ok_or("vertex output below mapping")?as usize;m.get_mut(i..i+4).ok_or("vertex output outside mapping")?.copy_from_slice(&value.to_bits().to_be_bytes());Ok(())}
/// Descriptor: output pointer, packed source pointer, vertex count. Finite data
/// in distinct mapped input/output regions is the verified contract. Count zero
/// would underflow the original loop; reject it explicitly instead of guessing.
/// Unused source prefetch reads are not reproduced. Errors roll back all writes.
pub fn apply(base:u32,memory:&mut[u8],descriptor:u32,matrix:u32,kind:u8,flags:u32)->Result<()>{
 let input=Memory{base,bytes:memory};let output=input.word(descriptor)?;let source=input.word(descriptor+4)?;let count=input.word(descriptor+8)?;
 let mut staged=memory.to_vec();apply_in_place(base,&mut staged,source,output,count,matrix,kind,flags)?;memory.copy_from_slice(&staged);Ok(())
}
/// Nontransactional internal path; caller owns rollback.
pub(crate) fn apply_in_place(base:u32,memory:&mut[u8],source:u32,output:u32,count:u32,matrix:u32,kind:u8,flags:u32)->Result<()>{
 if kind!=0x29 && kind!=0x2c{return Err("unrecovered vertex chunk format".into());}
 let mut bytes=memory;let input=Memory{base,bytes:&bytes};
 if count==0{return Err("zero vertex count has no verified termination".into());}
 let stride=if kind==0x29{24}else{28};
 let length=(count as usize).checked_mul(stride).ok_or("vertex input extent overflow")?;
 let start=source.checked_sub(base).ok_or("vertex input below mapping")?as usize;
 input.bytes.get(start..start.checked_add(length).ok_or("vertex extent overflow")?).ok_or("vertex input outside mapping")?;
 let mut mat=[0.;12];for i in 0..12{mat[i]=f(&input,matrix+i as u32*4)?;}
 for i in 0..count{
  let m=Memory{base,bytes:&bytes};let a=source+i*stride as u32;let mut v=[0.;6];for j in 0..6{v[j]=f(&m,a+j as u32*4)?;}
  let(index,weight)=if kind==0x2c{(m.half(a+26)?as u32,(m.half(a+24)?as i16 as f32)*(1./256.))}else{(i,1.)};
  let dest=output.checked_add(index.checked_mul(32).ok_or("vertex index overflow")?).ok_or("vertex destination overflow")?;
  let mut result=[0.;6];
  for axis in 0..3{
   let p=mat[6+axis].mul_add(v[2],mat[3+axis].mul_add(v[1],mat[axis].mul_add(v[0],mat[9+axis])));
   let n=mat[6+axis].mul_add(v[5],mat[3+axis].mul_add(v[4],mat[axis]*v[3]));
   result[axis]=p;result[axis+3]=n;
  }
  if kind==0x2c{for j in 0..6{result[j]=if flags==0{result[j]*weight}else{result[j].mul_add(weight,f(&m,dest+j as u32*4)?)};}}
  for j in 0..6{put(base,&mut bytes,dest+j as u32*4,result[j])?;}
 }
 Ok(())
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn malformed_output_and_zero_count_preserve_memory(){
  let mut memory=vec![0;1024];memory[0..4].copy_from_slice(&1020u32.to_be_bytes());memory[4..8].copy_from_slice(&128u32.to_be_bytes());memory[1020..].fill(0x55);
  let initial=memory.clone();assert!(apply(0,&mut memory,0,256,0x29,0).is_err());assert_eq!(memory,initial);
  memory[8..12].copy_from_slice(&1u32.to_be_bytes());let initial=memory.clone();assert!(apply(0,&mut memory,0,256,0x29,0).is_err());assert_eq!(memory,initial);
 }
}
