//! Original collision geometry dependencies, GSNE8P revision zero.
use crate::{disc::Result,original_math::OriginalMath};
mod plane_block{include!("generated_plane.rs");}
fn finite(v:&[f32])->Result<()>{if v.iter().any(|x|!x.is_finite()){Err("nonfinite geometry outside verified domain".into())}else{Ok(())}}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],b[0]*a[0]))}
fn plain_dot(a:[f32;3],b:[f32;3])->f32{a[2]*b[2]+(a[0]*b[0]+a[1]*b[1])}
fn normal(p:[f32;6])->[f32;3]{[p[3],p[4],p[5]]}

/// Complete 8002b7c4..8002b890: clamped acos, optional cross/axis sign.
pub fn signed_angle(a:[f32;3],b:[f32;3],axis:Option<[f32;3]>,math:&OriginalMath)->Result<u32>{
    finite(&a)?;finite(&b)?;if let Some(v)=axis{finite(&v)?;}
    let d=dot(a,b);finite(&[d])?;
    if d<=math.constant(0x803b6520-0x7878){return Ok(0x8000);}
    if d>=math.constant(0x803b6520-0x78dc){return Ok(0);}
    let mut angle=(math.double_constant(0x803b6520-0x7850)*(math.acos(d)as f64))as i32;
    let cross=[b[2].mul_add(a[1],-(b[1]*a[2])),b[0].mul_add(a[2],-(b[2]*a[0])),b[1].mul_add(a[0],-(b[0]*a[1]))];
    if let Some(axis)=axis{if dot(axis,cross)<math.constant(0x803b6520-0x7910){angle=0x10000_i32.wrapping_sub(angle);}}
    Ok(angle as u32)
}

/// Complete 8011b6c8..8011b7b8 for nonzero finite plane normals.
pub fn point_plane(point:[f32;3],plane:[f32;6],project:bool,math:&OriginalMath)->Result<(f32,Option<[f32;3]>)>{
    finite(&point)?;finite(&plane)?;let n=normal(plane);
    let squared=plain_dot(n,n);if !squared.is_finite() || squared<=0.{return Err("degenerate plane normal outside verified domain".into());}
    let numerator=(-plain_dot(n,[plane[0],plane[1],plane[2]]))+plain_dot(n,point);
    let projection=if project{let factor=(-numerator)/squared;Some(std::array::from_fn(|i|point[i]+n[i]*factor))}else{None};
    let distance=numerator.abs()*(math.constant(0x803b6520-0x5590)/math.sqrt_refined(squared));
    finite(&[distance])?;if let Some(v)=projection{finite(&v)?;}
    Ok((distance,projection))
}

/// Complete 8011bdb4..8011be8c parallel-plane predicate.
pub fn parallel(a:[f32;6],b:[f32;6],math:&OriginalMath)->Result<bool>{
    finite(&a)?;finite(&b)?;let a=normal(a);let b=normal(b);
    let aa=plain_dot(a,a);let bb=plain_dot(b,b);
    if !aa.is_finite() || !bb.is_finite() || aa<=0. || bb<=0.{return Err("degenerate plane normal outside verified domain".into());}
    let length_b=math.sqrt_refined(bb);let length_a=math.sqrt_refined(aa);
    let ratio=plain_dot(a,b)/(length_a*length_b);finite(&[ratio])?;
    Ok(ratio.abs()>math.constant(0x803b6520-0x5578))
}

/// Complete 8011b7b8..8011ba84: parallel distance or intersection line.
pub fn intersect_planes(a:[f32;6],b:[f32;6],output:bool,math:&OriginalMath)->Result<(f32,Option<[f32;6]>)>{
    if parallel(a,b,math)?{
        let d=point_plane([a[0],a[1],a[2]],b,false,math)?.0;
        Ok((d,if output{Some([math.constant(0x803b6520-0x5588);6])}else{None}))
    }else{
        let result=if output{let r=plane_block::solve(a,b,math);finite(&r)?;Some(r)}else{None};
        Ok((math.constant(0x803b6520-0x5588),result))
    }
}
