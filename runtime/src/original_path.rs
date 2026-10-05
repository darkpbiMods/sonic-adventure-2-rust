//! Original path geometry and record queries used by ground path steering.
use crate::{disc::Result,original_contact::Memory,original_math::OriginalMath,
 original_cone::distance,original_gameplay::angle_delta};
fn point(memory:&Memory,address:u32)->Result<[f32;3]>{
 Ok([f32::from_bits(memory.word(address)?),f32::from_bits(memory.word(address.wrapping_add(4))?),f32::from_bits(memory.word(address.wrapping_add(8))?)])
}
fn finite(v:&[f32])->Result<()>{if v.iter().any(|x|!x.is_finite()){Err("nonfinite path geometry".into())}else{Ok(())}}
fn sub(a:[f32;3],b:[f32;3])->[f32;3]{std::array::from_fn(|i|a[i]-b[i])}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],a[0]*b[0]))}
fn plain_dot(a:[f32;3],b:[f32;3])->f32{a[2]*b[2]+(a[0]*b[0]+a[1]*b[1])}
/// Complete 80027d50..80027dd8: endpoint-relative dot-product test.
pub fn endpoint_dot(a:[f32;3],b:[f32;3],p:[f32;3])->Result<f32>{
 finite(&a)?;finite(&b)?;finite(&p)?;let result=dot(sub(b,a),sub(p,a));finite(&[result])?;Ok(result)
}
/// Complete 8011b51c..8011b65c in the finite, nonzero-direction domain.
/// Projection multiplication/addition is deliberately not fused.
pub fn line_distance(p:[f32;3],origin:[f32;3],direction:[f32;3],math:&OriginalMath)->Result<(f32,[f32;3])>{
 finite(&p)?;finite(&origin)?;finite(&direction)?;
 let denominator=plain_dot(direction,direction);
 if !denominator.is_finite()||denominator<=0.{return Err("degenerate path line direction".into());}
 let ratio=plain_dot(direction,sub(p,origin))/denominator;
 let projected=std::array::from_fn(|i|origin[i]+direction[i]*ratio);finite(&projected)?;
 let delta=sub(p,projected);let squared=plain_dot(delta,delta);finite(&[squared])?;
 let d=if squared<math.constant(0x803b6520-0x55a0){math.constant(0x803b6520-0x559c)}else{math.sqrt_refined(squared)};
 Ok((d,projected))
}
/// Complete 800ce5e8..800ce718. Endpoint tests use the original fused dot;
/// the interior projection uses the separate scalar point-line routine.
pub fn segment_distance(p:[f32;3],a:[f32;3],b:[f32;3],math:&OriginalMath)->Result<(f32,[f32;3])>{
 let zero=math.constant(0x803b6520-0x60b0);
 if endpoint_dot(a,b,p)?<zero{return Ok((distance(p,a,math)?,a));}
 if endpoint_dot(b,a,p)?<zero{return Ok((distance(p,b,math)?,b));}
 line_distance(p,a,sub(b,a),math)
}
#[derive(Clone)]pub struct NearestOutputs{pub point:Option<[f32;3]>,pub along:Option<f32>}
/// Complete 8005c460..8005c6f8. For a valid-format path with no iterated
/// segment the source returns its incoming f28 register, left uninitialized by
/// this function. It is an explicit input rather than a fabricated zero.
pub fn nearest(memory:&Memory,path:u32,p:[f32;3],output:&mut NearestOutputs,incoming_f28:f64,math:&OriginalMath)->Result<f64>{
 let flags=memory.half(path)?;let kind=flags&0x7fff;
 let zero=math.constant(0x803b6520-0x7300);if kind!=1&&kind!=2{return Ok(zero as f64);}
 let mut remaining=memory.half(path.wrapping_add(2))?as i16 as i32;
 let records=memory.word(path.wrapping_add(8))?;let mut record=records;
 let stop=if flags&0x8000!=0{0}else{1};let(mut cumulative,mut best)=(zero,incoming_f28);let mut next=output.clone();
 let (stride,position,length)=if kind==1{(0x14,8,4)}else{(0x1c,0x10,0xc)};
 while remaining>stop{
  let a=point(memory,record.wrapping_add(position))?;
  let segment_length=f32::from_bits(memory.word(record.wrapping_add(length))?);finite(&[segment_length])?;
  record=record.wrapping_add(stride);
  // Original type-2 closure also uses first-record +8, despite its ordinary
  // position field being +10. Preserve that read, not a corrected layout.
  let b=point(memory,if remaining==1{records.wrapping_add(8)}else{record.wrapping_add(position)})?;
  let (d,projected)=segment_distance(p,a,b,math)?;
  if cumulative==zero||(d as f64)<best{
   best=d as f64;
   if next.point.is_some(){next.point=Some(projected);}
   if next.along.is_some(){next.along=Some(cumulative+distance(a,projected,math)?);}
  }
  cumulative+=segment_length;finite(&[cumulative])?;remaining-=1;
 }
 *output=next;Ok(best)
}
fn copy_words(memory:&Memory,address:u32,out:&mut[u8],offset:usize,count:usize)->Result<()>{
 for i in 0..count{out[offset+i*4..offset+i*4+4].copy_from_slice(&memory.word(address.wrapping_add(i as u32*4))?.to_be_bytes());}Ok(())
}
/// Complete 8005cdcc..8005d030, including its else-if endpoint adjustment.
/// Type 0/1/2 write 24/40/56 bytes respectively; unrelated output bytes remain.
pub fn adjacent(memory:&Memory,path:u32,index:i32,output:&mut[u8])->Result<()>{
 if output.len()!=0x38{return Err("invalid adjacent path output extent".into());}
 let flags=memory.half(path)?;let mut previous=index.wrapping_sub(1);let mut next=index.wrapping_add(1);
 if flags&0x8000==0{
  if previous<0{previous=0;}else{let count=memory.half(path.wrapping_add(2))?as i16 as i32;if next>=count{next=count.wrapping_sub(1);}}
 }else if previous<0{previous=(memory.half(path.wrapping_add(2))?as i16 as i32).wrapping_sub(1);}
 else if next>=memory.half(path.wrapping_add(2))?as i16 as i32{next=0;}
 let kind=flags&0x7fff;if kind>2{return Ok(());}
 let records=memory.word(path.wrapping_add(8))?;let stride=[12_i32,20,28][kind as usize];
 let a=records.wrapping_add(index.wrapping_mul(stride)as u32);let b=records.wrapping_add(next.wrapping_mul(stride)as u32);let mut out=output.to_vec();
 match kind{
  0=>{copy_words(memory,a,&mut out,0,3)?;copy_words(memory,b,&mut out,12,3)?;},
  1=>{
   let before=records.wrapping_add(previous.wrapping_mul(stride)as u32);
   for offset in [0_u32,2]{
    let (old,current,following)=(memory.half(before.wrapping_add(offset))?as i16 as i32,memory.half(a.wrapping_add(offset))?as i16 as i32,memory.half(b.wrapping_add(offset))?as i16 as i32);
    let low=old.wrapping_add(angle_delta(old as u32,current as u32)>>1)as u16;
    let high=current.wrapping_add(angle_delta(current as u32,following as u32)>>1)as u16;
    out[offset as usize..offset as usize+2].copy_from_slice(&low.to_be_bytes());
    out[20+offset as usize..22+offset as usize].copy_from_slice(&high.to_be_bytes());
   }
   copy_words(memory,a.wrapping_add(8),&mut out,8,3)?;copy_words(memory,b.wrapping_add(8),&mut out,28,3)?;
   copy_words(memory,a.wrapping_add(4),&mut out,4,1)?;copy_words(memory,b.wrapping_add(4),&mut out,24,1)?;
  },
  2=>{
   copy_words(memory,a.wrapping_add(16),&mut out,16,3)?;copy_words(memory,b.wrapping_add(16),&mut out,44,3)?;
   copy_words(memory,a,&mut out,0,3)?;copy_words(memory,b,&mut out,28,3)?;
   copy_words(memory,a.wrapping_add(12),&mut out,12,1)?;copy_words(memory,b.wrapping_add(12),&mut out,40,1)?;
  },_=>unreachable!()
 }
 output.copy_from_slice(&out);Ok(())
}
/// 80111240 finite-input floor behavior. Large finite f32 values are already
/// integral; signed zero is retained by the source and by this operation.
pub fn floor(value:f32)->Result<f32>{finite(&[value])?;Ok(value.floor())}
/// Complete 8002b770..8002b7c4 in its finite arithmetic domain.
pub fn wrap(value:f32,length:f32,math:&OriginalMath)->Result<f32>{
 let zero=math.constant(0x803b6520-0x7910);if length==zero{return Ok(zero);}
 finite(&[value,length])?;let quotient=floor(value/length)?;
 let result=-(length.mul_add(quotient,-value));finite(&[result])?;Ok(result)
}
