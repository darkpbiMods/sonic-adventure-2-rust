//! Arbitrary-axis rotation used by the special ground-surface movement path.
use crate::{disc::Result,original_math::{OriginalMath,Matrix,concat}};
/// 800eddf0..800ede34, including paired squared-length order and one seed
/// refinement. Zero/nonfinite axes are outside the current verified domain.
pub fn normalize(axis:[f32;3],math:&OriginalMath)->Result<[f32;3]>{
 if axis.iter().any(|v|!v.is_finite()){return Err("nonfinite rotation axis".into());}
 let [x,y,z]=axis;let squared=z.mul_add(z,x*x)+y*y;
 if !squared.is_finite()||squared<=0.{return Err("degenerate rotation axis".into());}
 let seed=crate::gekko_estimate::reciprocal_sqrt_seed(squared as f64);
 let square=(seed*crate::gekko_rounding::normal_25_bit(seed))as f32;
 let half=(seed*math.constant(0x803b6520-0x5d30)as f64)as f32;
 let correction=-(square.mul_add(squared,-math.constant(0x803b6520-0x5d2c)));
 let refined=correction*half;
 let out=axis.map(|v|v*refined);
 if out.iter().any(|v|!v.is_finite()){return Err("axis normalization refinement outside finite domain".into());}
 Ok(out)
}
/// Complete 800ed694..800ed798, preserving paired sums and fused off-diagonal
/// entries. The original routine normalizes its input even when already unit.
pub fn matrix(axis:[f32;3],radians:f32,math:&OriginalMath)->Result<Matrix>{
 let (s,c)=math.sin_cos(radians)?;let t=math.constant(0x803b6520-0x5d58)-c;
 let [x,y,z]=normalize(axis,math)?;let zero=math.constant(0x803b6520-0x5d54);
 let (xt,yt,zt)=(x*t,y*t,z*t);let (xs,ys)=(x*s,y*s);
 let xy=xt*y;let xz=xt*z;let yz=yt*z;
 Ok([[xt*x+c,-(z.mul_add(s,-xy)),xz+ys,zero],
     [z.mul_add(s,xy),yt*y+c,(-xs)+yz,zero],
     [xz+(-ys),xs+yz,zt*z+c,zero]])
}
/// Complete 80115218..80115290. The original nullable matrix pointer maps to
/// an explicit host matrix; callers select their current matrix before entry.
pub fn rotate(current:&Matrix,axis:[f32;3],angle:i32,math:&OriginalMath)->Result<Matrix>{
 if current.iter().flatten().any(|v|!v.is_finite()){return Err("nonfinite axis-rotation matrix".into());}
 let radians=(math.double_constant(0x803b6520-0x5688)*(angle as f64))as f32;
 let out=concat(current,&matrix(axis,radians,math)?);
 if out.iter().flatten().any(|v|!v.is_finite()){return Err("axis-rotation arithmetic overflow".into());}
 Ok(out)
}
