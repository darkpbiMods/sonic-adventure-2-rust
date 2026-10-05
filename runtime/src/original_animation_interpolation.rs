//! Remaining evaluator family80113018/801130ac/801132b4.
use crate::{disc::Result,original_contact::Memory,original_animation_keys::{interval,search},original_math::OriginalMath};
pub fn planar(m:&Memory,keys:u32,count:u32,time:f32,math:&OriginalMath)->Result<[u32;3]>{
 let(a,b,t)=interval(m,keys,16,count,time)?;let mut out=[0;3];
 for i in 0..2{let x=f32::from_bits(m.word(a+4+i as u32*4)?);let y=f32::from_bits(m.word(b+4+i as u32*4)?);let d=y-x;let p=t*d;out[i]=(x+p).to_bits();}
 out[2]=math.constant(0x803b6520-0x5708).to_bits();Ok(out)
}
pub fn scalar_angle(m:&Memory,keys:u32,count:u32,time:f32)->Result<u32>{
 let(a,b,t)=interval(m,keys,8,count,time)?;let x=m.word(a+4)?;let y=m.word(b+4)?;let d=y.wrapping_sub(x)as i16 as f32;let p=t*d;let v=x as i32 as f32+p;
 Ok(if v.is_nan()||v<=-2147483648.0{0x80000000}else if v>=2147483648.0{0x7fffffff}else{v.trunc()as i32 as u32})
}
pub fn cubic(m:&Memory,keys:u32,count:u32,time:f32,math:&OriginalMath)->Result<[u32;3]>{
 if !time.is_finite()||count==0{return Err("cubic requires finite time and nonempty keys".into());}
 let converted=if time<0.0{0}else if time>=4294967296.0{u32::MAX}else{time.trunc()as u32};let i=search(m,keys,16,count,converted)?;
 let indices=[i.wrapping_add(count.wrapping_sub(1))%count,i,i.wrapping_add(1)%count,i.wrapping_add(2)%count];
 let pointers=indices.map(|v|keys.wrapping_add(v.wrapping_mul(16)));let frame=m.word(pointers[1])?;let delta=m.word(pointers[2])?.wrapping_sub(frame);
 let c=|offset:u32|math.constant(0x803b6520-offset);
 let divisor=if delta as i32>0{delta as f32}else{c(0x56f8)};
 let t=(time-frame as f32)/divisor;let t2=t*t;let t3=t2*t;
 let twice=c(0x56ec)*t2;let a=c(0x56f0)*((-t3+twice)-t);
 let b=c(0x56e8)+(c(0x56e4)*t3-c(0x56e0)*t2);
 let cc=c(0x56f0)*t+(c(0x56dc)*t3+twice);let d=c(0x56f0)*(t3-t2);
 let mut out=[0;3];for axis in 0..3{let v=|n:usize|->Result<f32>{Ok(f32::from_bits(m.word(pointers[n]+4+axis as u32*4)?))};let p0=a*v(0)?;let p1=b*v(1)?;let p2=cc*v(2)?;let p3=d*v(3)?;out[axis]=(((p0+p1)+p2)+p3).to_bits();}Ok(out)
}
