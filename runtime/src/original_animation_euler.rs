//! Table initialization801182cc..8011834c and Euler80115f68..80116098.
use crate::{disc::Result,original_math::OriginalMath,original_animation_matrix::Matrix};
pub fn table(math:&OriginalMath)->Result<Vec<[f32;2]>>{
 let factor=math.constant(0x803b6520-0x55f4);let divisor=math.constant(0x803b6520-0x55f0);
 (0..65536).map(|i|{let product=factor*((i*2)as f32);let angle=product/divisor;math.sin_cos(angle).map(|(s,c)|[s,c])}).collect()
}
pub fn rotate(matrix:&mut Matrix,angles:[u32;3],order:u32,table:&[[f32;2]])->Result<()>{
 if table.len()!=65536{return Err("incomplete source rotation table".into());}
 let axes=if order==0{[2,1,0]}else{[1,0,2]};
 for axis in axes{let index=(angles[axis]&0xffff)as usize;if index==0{continue;}let[s,c]=table[index];
  for row in matrix.iter_mut(){let[x,y,z,_]=*row;match axis{
   0=>{let a=-s*y;let b=c*y;row[2]=c.mul_add(z,a);row[1]=s.mul_add(z,b);},
   1=>{let a=s*x;let b=c*x;row[2]=c.mul_add(z,a);row[0]=(-s).mul_add(z,b);},
   2=>{let a=c*x;let b=-s*x;row[0]=s.mul_add(y,a);row[1]=c.mul_add(y,b);},_=>unreachable!()
  }}
 }Ok(())
}
