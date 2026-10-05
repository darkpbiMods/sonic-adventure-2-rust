//! Complete node transform80113c4c: sample, translate, rotate, scale.
use crate::{disc::Result,original_contact::Memory,original_math::OriginalMath,original_animation_matrix::{Matrix,translate,scale},original_animation_node,original_animation_euler,original_animation_quaternion_matrix};
pub fn apply(memory:&Memory,node:u32,context:&mut[u32;16],matrix:&mut Matrix,table:&[[f32;2]],math:&OriginalMath)->Result<()>{
 let mut c=*context;let mut m=*matrix;let mut output=[0;13];let mode=original_animation_node::sample(memory,node,&mut c,&mut output,math)?;
 translate(&mut m,std::array::from_fn(|i|f32::from_bits(output[i])));
 if mode<2{original_animation_euler::rotate(&mut m,[output[3],output[4],output[5]],mode,table)?;}else{original_animation_quaternion_matrix::apply(&mut m,std::array::from_fn(|i|f32::from_bits(output[9+i])),math);}
 scale(&mut m,std::array::from_fn(|i|f32::from_bits(output[6+i])));*context=c;*matrix=m;Ok(())
}
