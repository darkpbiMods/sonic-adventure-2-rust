//! Complete terrain orientation helper 80042694..800428f8.
use crate::{disc::Result, original_gameplay::{angle_approach, CHARACTER_BYTES, ENTITY_BYTES}, original_ground::rotate_vector, original_math::OriginalMath};

fn word(b: &[u8], i: usize) -> u32 { u32::from_be_bytes(b[i..i+4].try_into().unwrap()) }
fn float(b: &[u8], i: usize) -> f32 { f32::from_bits(word(b, i)) }
fn put(b: &mut [u8], i: usize, v: u32) { b[i..i+4].copy_from_slice(&v.to_be_bytes()); }
fn vector(b: &[u8], i: usize) -> [f32;3] { [float(b,i),float(b,i+4),float(b,i+8)] }
fn dot(a: [f32;3], b: [f32;3]) -> f32 { a[2].mul_add(b[2], a[1].mul_add(b[1], b[0]*a[0])) }

/// Workspace includes normal vectors +1c/+48 and target pitch/roll +34/+38.
/// The caller supplies source gravity and surface flags. All changes are staged.
pub fn align(entity: &mut [u8], character: &[u8], workspace: &[u8], flags: u32, gravity: [f32;3], math: &OriginalMath) -> Result<()> {
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES || workspace.len()<0x54 {
        return Err("unsupported terrain alignment state size".into());
    }
    let normal=vector(workspace,0x48);
    let other=vector(workspace,0x1c);
    if !normal.into_iter().chain(other).chain(gravity).all(f32::is_finite) {
        return Err("nonfinite terrain normal or gravity".into());
    }
    let current_dot=dot(gravity,normal);
    let other_dot=dot(gravity,other);
    if !current_dot.is_finite() || !other_dot.is_finite() { return Err("terrain dot overflow".into()); }
    let status=u16::from_be_bytes(character[0x1c..0x1e].try_into().unwrap());
    let target_pitch=word(workspace,0x34);let target_roll=word(workspace,0x38);
    let (pitch,roll)=if other_dot>current_dot || status&1==0 || status&0x2000!=0 {
        (target_pitch,target_roll)
    } else {
        let local=rotate_vector(entity,normal,false,math)?;
        // Source uses two multiplies plus an add, not a fused sum of squares.
        let square=local[2]*local[2]+local[0]*local[0];
        if !square.is_finite() { return Err("terrain normal square overflow".into()); }
        let horizontal=math.sqrt_refined(square);
        let scaled=math.double_constant(0x803b6520-0x75e8)*(math.atan2(horizontal,local[1]) as f64);
        if !scaled.is_finite() { return Err("nonfinite terrain angle".into()); }
        let angle=(scaled as i32 as u32)&0xffff;
        let mut step=if angle>0x8000 {0x10000-angle} else {angle} as i32;
        if flags&0x100000!=0 { step=0x400; }
        else {
            let speed=float(character,0x64);
            let [jog,run,rush,dash]=[float(character,0xe4),float(character,0xe8),float(character,0xec),float(character,0xf4)];
            if ![speed,jog,run,rush,dash].into_iter().all(f32::is_finite) { return Err("nonfinite terrain speed".into()); }
            if speed>=dash { /* source retains the calculated angle */ }
            else if speed>=rush { step=step.min(0x2000); }
            else if speed>=run { step=step.min(0x1500); }
            else if speed>=jog { step=step.min(0x1000); }
            else { step=0x400; }
        }
        (angle_approach(word(entity,8),target_pitch,step),angle_approach(word(entity,0x10),target_roll,step))
    };
    put(entity,8,pitch);put(entity,0x10,roll);Ok(())
}

#[cfg(test)]mod tests {
    use super::*;
    #[test]fn invalid_alignment_preserves_entity() {
        let math=OriginalMath::from_dol(std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../source/main.dol")).unwrap()).unwrap();
        let mut entity=[0;ENTITY_BYTES];let mut character=[0;CHARACTER_BYTES];let mut workspace=[0;0x60];
        character[0x1d]=1;put(&mut entity,8,i32::MAX as u32);put(&mut workspace,0x4c,f32::MAX.to_bits());put(&mut workspace,0x20,1_f32.to_bits());
        let before=entity;
        // Finite operands can overflow the dot; the angle itself is supported.
        assert!(align(&mut entity,&character,&workspace,0,[0.,-2.,0.],&math).is_err());assert_eq!(entity,before);
        put(&mut workspace,0x4c,1_f32.to_bits());put(&mut workspace,0x48,f32::INFINITY.to_bits());assert!(align(&mut entity,&character,&workspace,0,[0.,-1.,0.],&math).is_err());assert_eq!(entity,before);
    }
}
