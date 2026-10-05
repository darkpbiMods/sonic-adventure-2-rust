//! Complete path interpolation 8005c910..8005cdcc, used by ground steering.
use crate::{disc::Result,original_contact::Memory,original_math::OriginalMath,
 original_path::{adjacent,wrap},original_gameplay::angle_delta,original_angle_table::entry};
fn word(b:&[u8],o:usize)->u32{u32::from_be_bytes(b[o..o+4].try_into().unwrap())}
fn float(b:&[u8],o:usize)->f32{f32::from_bits(word(b,o))}
fn vector(b:&[u8],o:usize)->[f32;3]{std::array::from_fn(|i|float(b,o+i*4))}
fn put(b:&mut[u8],o:usize,v:u32){b[o..o+4].copy_from_slice(&v.to_be_bytes());}
fn putv(b:&mut[u8],o:usize,v:[f32;3]){for i in 0..3{put(b,o+i*4,v[i].to_bits());}}
fn integer(v:f64)->i32{if v.is_nan(){i32::MIN}else{v as i32}}
fn normalize(v:[f32;3],math:&OriginalMath)->Result<[f32;3]>{math.estimated_vector(v)?.1.ok_or_else(||"degenerate sampled path direction".into())}
fn angles(v:[f32;3],math:&OriginalMath)->[u32;2]{
 let scale=math.double_constant(0x803b6520-0x72f8);
 [integer(scale*math.asin(v[2])as f64)as u32,integer(scale*math.atan2(v[0],v[1])as f64).wrapping_neg()as u32]
}
fn normal(pitch:u32,roll:u32,math:&OriginalMath)->Result<[f32;3]>{
 let [sp,cp]=entry(pitch,math)?;let [sr,cr]=entry(roll,math)?;
 Ok([(-sr)*cp,cr*cp,sp])
}
/// Input/output is the source 68-byte sample record, whose +10 holds distance.
/// A closed unsupported-format path can still wrap +10 before returning false.
/// incoming_f30 preserves the source register if no segment iteration occurs.
pub fn sample(memory:&Memory,path:u32,output:&mut[u8],incoming_f30:f64,math:&OriginalMath)->Result<bool>{
 if output.len()!=0x44{return Err("invalid path sample record extent".into());}
 let flags=memory.half(path)?;let mut travel=float(output,0x10);
 let length=f32::from_bits(memory.word(path.wrapping_add(4))?);
 if !travel.is_finite()||!length.is_finite(){return Err("nonfinite path sampling distance".into());}
 let zero=math.constant(0x803b6520-0x7300);let one=math.constant(0x803b6520-0x72fc);
 let mut out=output.to_vec();
 if flags&0x8000!=0{travel=wrap(travel,length,math)?;put(&mut out,0x10,travel.to_bits());}
 else if travel<zero||travel>length{return Ok(false);}
 let kind=flags&0x7fff;
 if kind!=1&&kind!=2{output.copy_from_slice(&out);return Ok(false);}
 let count=memory.half(path.wrapping_add(2))?as i16 as i32;let records=memory.word(path.wrapping_add(8))?;
 let (stride,length_offset)=if kind==1{(20_u32,4)}else{(28,12)};
 let(mut cumulative,mut segment_length,mut index)=(zero,incoming_f30,0_i32);
 while index<count{
  let mut length=f32::from_bits(memory.word(records.wrapping_add(index as u32*stride).wrapping_add(length_offset))?);
  if !length.is_finite(){return Err("nonfinite path segment length".into());}
  if length==zero{length=one;}segment_length=length as f64;
  if travel<=cumulative+length{break;}cumulative+=length;index+=1;
 }
 let mut pair=[0_u8;0x38];adjacent(memory,path,index,&mut pair)?;
 let ratio=((travel-cumulative)as f64/segment_length)as f32;
 if !ratio.is_finite(){return Err("path interpolation ratio outside finite domain".into());}
 let (a,b)=if kind==1{(vector(&pair,8),vector(&pair,28))}else{(vector(&pair,16),vector(&pair,44))};
 let direction:[f32;3]=std::array::from_fn(|i|b[i]-a[i]);let position=std::array::from_fn(|i|direction[i].mul_add(ratio,a[i]));
 if kind==1{
  let p=i16::from_be_bytes(pair[0..2].try_into().unwrap())as i32;
  let r=i16::from_be_bytes(pair[2..4].try_into().unwrap())as i32;
  let next_p=i16::from_be_bytes(pair[20..22].try_into().unwrap())as i32;
  let next_r=i16::from_be_bytes(pair[22..24].try_into().unwrap())as i32;
  let blend_p=p.wrapping_add(integer((ratio*(angle_delta(p as u32,next_p as u32)as f32))as f64));
  let blend_r=r.wrapping_add(integer((ratio*(angle_delta(r as u32,next_r as u32)as f32))as f64));
  put(&mut out,0,p as u32);put(&mut out,4,r as u32);put(&mut out,8,blend_p as u32);put(&mut out,12,blend_r as u32);
  putv(&mut out,0x14,position);putv(&mut out,0x20,normal(p as u32,r as u32,math)?);
  putv(&mut out,0x2c,normal(blend_p as u32,blend_r as u32,math)?);
  putv(&mut out,0x38,normalize(direction,math)?);
 }else{
  let before=vector(&pair,0);let after=vector(&pair,28);let inverse=one-ratio;
  // The original orientation blend weights the first vector by ratio and the
  // next vector by (1-ratio), opposite the position interpolation above.
  let blended=normalize(std::array::from_fn(|i|before[i].mul_add(ratio,after[i]*inverse)),math)?;
  putv(&mut out,0x14,position);putv(&mut out,0x20,before);putv(&mut out,0x2c,blended);
  putv(&mut out,0x38,normalize(direction,math)?);
  let first=angles(before,math);let second=angles(blended,math);
  put(&mut out,0,first[0]);put(&mut out,4,first[1]);put(&mut out,8,second[0]);put(&mut out,12,second[1]);
 }
 output.copy_from_slice(&out);Ok(true)
}
