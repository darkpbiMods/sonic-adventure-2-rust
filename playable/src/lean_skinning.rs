//! Renderer-independent position correction for the original local lean joint.
//! The affected contribution is a weighted homogeneous point, not the final
//! vertex multiplied by a scalar. Its w component retains translation weight.
use glam::{Mat4, Vec3, Vec4};

pub fn deform_position(baseline: Vec3, affected: Vec4, subtree_delta: Mat4) -> Vec3 {
    baseline + (subtree_delta * affected - affected).truncate()
}
