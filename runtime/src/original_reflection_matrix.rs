//! Reflection texture matrix construction at 8011d398..8011d3dc.
//! Input is global matrix 80357880, supplied by the host; no normalization.
use crate::{disc::Result,original_math::Matrix};
pub fn build(source:Matrix)->Result<Matrix>{
    if source.iter().flatten().any(|x|!x.is_finite()) {return Err("Nonfinite reflection source matrix".into());}
    let mut result=[[0.;4];3];
    for row in 0..2 {for col in 0..4 {result[row][col]=-0.5*source[row][col];}result[row][3]+=0.5;}
    result[2][3]=1.;
    Ok(result)
}
