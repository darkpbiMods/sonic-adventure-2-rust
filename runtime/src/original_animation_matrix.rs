//! Original in-place affine operations8011610c(scale),8011616c(translate).
pub type Matrix=[[f32;4];3];
// Source uses scalar double fmul for Z, followed by psq_st. Its normal-value
// conversion discards low significand bits instead of rounding as fmuls does.
fn stored_double(value:f64)->f32{let rounded=value as f32;if rounded.is_finite()&&(rounded as f64).abs()>value.abs(){f32::from_bits(rounded.to_bits()-1)}else{rounded}}
pub fn scale(matrix:&mut Matrix,value:[f32;3]){for row in matrix{row[0]*=value[0];row[1]*=value[1];row[2]=stored_double(row[2]as f64*value[2]as f64);}}
pub fn translate(matrix:&mut Matrix,value:[f32;3]){
 for row in matrix{let x=row[0]*value[0];let y=row[1]*value[1];let left=row[2].mul_add(value[2],x);let right=row[3].mul_add(1.,y);row[3]=left+right;}
}
