//! Portable rest rig and linear position skinning. Matrices use column vectors.
use glam::{Mat4, Vec3};
use serde::Deserialize;
type Rows = [[f32; 4]; 4];
fn matrix(rows: Rows) -> Mat4 { Mat4::from_cols_array_2d(&rows).transpose() }
#[derive(Deserialize)]
struct Bone { parent: Option<usize>, rest: Rows, inverse_bind: Rows }
#[derive(Deserialize)]
struct Corner { vertex: usize, normal: [f32;3], uv: [f32;2] }
#[derive(Deserialize)]
struct Triangle { corners: [usize;3], material: usize }
#[derive(Deserialize)]
pub struct Rig {
    format: String, version: u32, lean_joint: usize,
    mesh_to_armature: Rows, armature_to_world: Rows,
    bones: Vec<Bone>, positions: Vec<[f32;3]>, weights: Vec<Vec<(usize,f32)>>,
    corners: Vec<Corner>, triangles: Vec<Triangle>, materials: Vec<String>,
}
impl Rig {
    pub fn read(bytes: &[u8]) -> Result<Self,String> {
        let rig: Self = serde_json::from_slice(bytes).map_err(|e|e.to_string())?;
        rig.validate()?;
        Ok(rig)
    }
    fn validate(&self) -> Result<(),String> {
        let valid_matrix = |rows| { let m=matrix(rows); m.is_finite() && m.determinant().abs()>1e-8 && m.row(3).abs_diff_eq(glam::Vec4::W,1e-6) };
        if self.format!="SA2_PORTABLE_RIG" || self.version!=1 || self.bones.is_empty() || self.bones.len()>4096 || self.lean_joint>=self.bones.len() || self.positions.is_empty() || self.positions.len()!=self.weights.len() || !valid_matrix(self.mesh_to_armature) || !valid_matrix(self.armature_to_world) { return Err("Invalid rig header".into()); }
        for (i,b) in self.bones.iter().enumerate() {
            if b.parent.is_some_and(|p|p>=i) || !valid_matrix(b.rest) || !valid_matrix(b.inverse_bind) || !(matrix(b.rest)*matrix(b.inverse_bind)).abs_diff_eq(Mat4::IDENTITY,1e-4) { return Err("Invalid bone or inverse bind".into()); }
        }
        for (p,weights) in self.positions.iter().zip(&self.weights) {
            if !Vec3::from_array(*p).is_finite() || weights.is_empty() || weights.iter().any(|&(i,w)|i>=self.bones.len() || !w.is_finite() || w<=0. || w>1.) || (weights.iter().map(|x|x.1).sum::<f32>()-1.).abs()>1e-5 { return Err("Invalid vertex weights".into()); }
        }
        if self.corners.iter().any(|c|c.vertex>=self.positions.len() || !Vec3::from_array(c.normal).is_finite() || (Vec3::from_array(c.normal).length_squared()-1.).abs()>1e-4 || c.uv.iter().any(|x|!x.is_finite())) || self.triangles.iter().any(|t|t.material>=self.materials.len() || t.corners.iter().any(|&i|i>=self.corners.len())) { return Err("Invalid corner topology".into()); }
        Ok(())
    }
    pub fn rest_pose(&self) -> Vec<Mat4> { self.bones.iter().map(|b|matrix(b.rest)).collect() }
    /// Absolute posed bone matrices in armature coordinates; result in mesh coordinates.
    pub fn skin_positions(&self, pose: &[Mat4]) -> Result<Vec<Vec3>,String> {
        if pose.len()!=self.bones.len() || pose.iter().any(|m|!m.is_finite() || !m.row(3).abs_diff_eq(glam::Vec4::W,1e-6)) { return Err("Invalid pose".into()); }
        let to_arm=matrix(self.mesh_to_armature); let to_mesh=to_arm.inverse();
        let skin: Vec<_> = pose.iter().zip(&self.bones).map(|(p,b)|to_mesh * *p * matrix(b.inverse_bind) * to_arm).collect();
        let result:Vec<_>=self.positions.iter().zip(&self.weights).map(|(p,weights)|weights.iter().fold(Vec3::ZERO,|sum,&(i,w)|sum+skin[i].transform_point3(Vec3::from_array(*p))*w)).collect();
        if result.iter().any(|v|!v.is_finite()) { return Err("Nonfinite skinned position".into()); }
        Ok(result)
    }
}
