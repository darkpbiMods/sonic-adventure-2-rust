//! Source800ed7f4 and wrapper80115e54. Input quaternion is scalar first.
use crate::{original_math::{OriginalMath,concat},original_animation_matrix::Matrix};
pub fn construct(q:[f32;4],math:&OriginalMath)->Matrix{
 let[w,x,y,z]=q;let one=math.constant(0x803b6520-0x5d58);let two=one+one;
 let xx=x*x;let yy=y*y;let zz=z*z;let zw=z*w;
 let norm=z.mul_add(z,xx)+w.mul_add(w,yy);
 let seed=crate::gekko_estimate::reciprocal_seed(norm as f64)as f32;
 let correction=-(norm.mul_add(seed,-two));let refined=seed*correction;let s=refined*two;
 let diagonal_z=-((xx+yy).mul_add(s,-one));
 let diagonal_y=-(z.mul_add(z,xx).mul_add(s,-one));let diagonal_x=-((zz+yy).mul_add(s,-one));
 let xy_plus=x.mul_add(y,zw);let xy_minus=x.mul_add(y,-zw);
 let yw=y*w;let xw=x*w;let xz_plus=x.mul_add(z,yw);let yz_plus=y.mul_add(z,xw);
 let xz_minus=-(yw.mul_add(two,-xz_plus));let yz_minus=-(xw.mul_add(two,-yz_plus));
 [[diagonal_x,xy_minus*s,xz_plus*s,0.],[xy_plus*s,diagonal_y,yz_minus*s,0.],[xz_minus*s,yz_plus*s,diagonal_z,0.]]
}
pub fn apply(matrix:&mut Matrix,q:[f32;4],math:&OriginalMath){*matrix=concat(matrix,&construct(q,math));}
