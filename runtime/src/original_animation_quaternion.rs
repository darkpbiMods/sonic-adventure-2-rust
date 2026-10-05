//! Normalized linear quaternion blend8011551c and key wrapper80112f08.
use crate::{disc::Result,original_contact::Memory,original_math::OriginalMath,original_animation_keys::interval};
pub fn blend(a:[f32;4],b:[f32;4],time:f32,math:&OriginalMath)->[u32;4]{
 let one=math.constant(0x803b6520-0x5670);let remaining=one-time;
 let p0=a[0]*b[0];let p1=a[1]*b[1];let p2=a[2]*b[2];let p3=a[3]*b[3];let dot=p3+(p2+(p0+p1));
 let add=dot>math.constant(0x803b6520-0x566c);
 let v=std::array::from_fn::<f32,4,_>(|i|{let x=a[i]*remaining;let y=b[i]*time;if add{x+y}else{x-y}});
 let s0=v[0]*v[0];let s1=v[1]*v[1];let s2=v[2]*v[2];let s3=v[3]*v[3];let squared=s3+(s2+(s0+s1));let inverse=one/math.sqrt_refined(squared);
 v.map(|x|{let result=x*inverse;if result.is_nan(){0x7fc00000}else{result.to_bits()}})
}
pub fn sample(memory:&Memory,keys:u32,count:u32,time:f32,math:&OriginalMath)->Result<[u32;4]>{
 let(a,b,t)=interval(memory,keys,20,count,time)?;let mut x=[0.;4];let mut y=[0.;4];for i in 0..4{x[i]=f32::from_bits(memory.word(a+4+i as u32*4)?);y[i]=f32::from_bits(memory.word(b+4+i as u32*4)?);}Ok(blend(x,y,t,math))
}
