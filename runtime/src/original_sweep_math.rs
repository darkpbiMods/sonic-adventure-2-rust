//! Matrix operations used by terrain sweep 800cee08.
//! Preserve paired-single lane arithmetic rather than the scalar transform's order.
use crate::{disc::Result, original_angle_table, original_math::OriginalMath};

pub type Matrix = [[f32; 4]; 3];

/// 800ed484..800ed57c. None is the original singular return, for which the
/// destination remains untouched. Preserve cofactor and reciprocal refinement
/// ordering, including single-precision products preceding fused operations.
pub fn inverse(m: &Matrix) -> Result<Option<Matrix>> {
    if !m.iter().flatten().all(|v| v.is_finite()) {
        return Err("nonfinite sweep inverse input".into());
    }
    let f11 = [m[0][1].mul_add(m[1][2], -(m[1][1]*m[0][2])),
               m[0][2].mul_add(m[1][0], -(m[1][2]*m[0][0]))];
    let f13 = [m[1][1].mul_add(m[2][2], -(m[2][1]*m[1][2])),
               m[1][2].mul_add(m[2][0], -(m[2][2]*m[1][0]))];
    let f12 = [m[2][1].mul_add(m[0][2], -(m[0][1]*m[2][2])),
               m[2][2].mul_add(m[0][0], -(m[0][2]*m[2][0]))];
    let f10 = m[1][0].mul_add(m[2][1], -(m[1][1]*m[2][0]));
    let f9 = m[0][1].mul_add(m[2][0], -(m[0][0]*m[2][1]));
    let f8 = m[0][0].mul_add(m[1][1], -(m[0][1]*m[1][0]));
    let det = m[2][0].mul_add(f11[0], m[1][0].mul_add(f12[0], m[0][0]*f13[0]));
    if det == 0. { return Ok(None); }
    if !det.is_finite() { return Err("sweep inverse determinant overflow".into()); }
    let seed = (crate::gekko_estimate::reciprocal_seed(det.abs() as f64) as f32).copysign(det);
    let inv = -(det.mul_add(seed*seed, -(seed+seed)));
    let f13=f13.map(|v|v*inv); let f12=f12.map(|v|v*inv); let f11=f11.map(|v|v*inv);
    let (f10,f9,f8)=(f10*inv,f9*inv,f8*inv);
    let translation = |x:f32,y:f32,z:f32| -(z.mul_add(m[2][3], y.mul_add(m[1][3], x*m[0][3])));
    let out = [[f13[0],f12[0],f11[0],translation(f13[0],f12[0],f11[0])],
               [f13[1],f12[1],f11[1],translation(f13[1],f12[1],f11[1])],
               [f10,f9,f8,translation(f10,f9,f8)]];
    if !out.iter().flatten().all(|v|v.is_finite()) { return Err("sweep inverse arithmetic overflow".into()); }
    Ok(Some(out))
}

/// 80115df4..80115e54. Value input also preserves the original alias-safe loads.
pub fn transform(matrix: &Matrix, point: [f32; 3]) -> [f32; 3] {
    matrix.map(|row| {
        let left = row[2].mul_add(point[2], row[0] * point[0]);
        let right = row[3].mul_add(1.0, row[1] * point[1]);
        left + right
    })
}

/// 8011bf0c..8011bf64, using the original startup sine/cosine table.
pub fn rotate_x(matrix: &mut Matrix, angle: u32, math: &OriginalMath) -> Result<()> {
    let [s, c] = original_angle_table::entry(angle, math)?;
    for row in matrix {
        let (y, z) = (row[1], row[2]);
        row[1] = s.mul_add(z, c * y);
        row[2] = c.mul_add(z, (-s) * y);
    }
    Ok(())
}

/// 8011bf64..8011bfe0, preserving the source multiply/add order.
pub fn rotate_y(matrix: &mut Matrix, angle: u32, math: &OriginalMath) -> Result<()> {
    let [s, c] = original_angle_table::entry(angle, math)?;
    for row in matrix {
        let (x, z) = (row[0], row[2]);
        row[0] = (-s).mul_add(z, c * x);
        row[2] = c.mul_add(z, s * x);
    }
    Ok(())
}
