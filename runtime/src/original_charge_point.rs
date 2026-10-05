//! Charge/ready ribbon radial point, source80089554..800895b4.
use crate::{disc::Result,original_angle_table,original_math::OriginalMath};

/// All coordinates and radii are in original game units and axes.
pub fn point(center:[f32;3],radius:f32,pitch:i32,yaw:i32,math:&OriginalMath)->Result<[f32;3]>{
 if !radius.is_finite() || !center.iter().all(|v|v.is_finite()){
  return Err("nonfinite charge point input".into());
 }
 let [sp,cp]=original_angle_table::entry(pitch as u32,math)?;
 let [sy,cy]=original_angle_table::entry(yaw as u32,math)?;
 Ok([cp.mul_add(radius*sy,center[0]),radius.mul_add(sp,center[1]),cp.mul_add(radius*cy,center[2])])
}
