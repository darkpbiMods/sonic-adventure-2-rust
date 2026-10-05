//! Original directional nearest-contact query, GSNE8P revision zero.
use crate::{disc::Result,original_contact::Memory,original_gameplay::angle_distance,original_math::OriginalMath};
fn position(memory:&Memory<'_>,entity:u32)->Result<[f32;3]>{
    Ok([f32::from_bits(memory.word(entity.wrapping_add(0x14))?),f32::from_bits(memory.word(entity.wrapping_add(0x18))?),f32::from_bits(memory.word(entity.wrapping_add(0x1c))?)])
}
/// Complete 8011b65c..8011b6c8 with 8002f15c square-root dependency.
/// Source clamps squared distances below 0.025 to zero, before sqrt.
pub fn distance(a:[f32;3],b:[f32;3],math:&OriginalMath)->Result<f32>{
    if a.iter().chain(b.iter()).any(|x|!x.is_finite()){return Err("nonfinite contact position".into());}
    let z=a[2]-b[2];let x=a[0]-b[0];let y=a[1]-b[1];
    let squared=z*z+(x*x+y*y);
    if !squared.is_finite(){return Err("contact distance outside finite domain".into());}
    Ok(if squared<math.constant(0x803b6520-0x5598){math.constant(0x803b6520-0x5594)}else{math.sqrt_refined(squared)})
}
/// Complete 8004f2d8..8004f3c8. Stops at a null task, not the shape-index
/// sentinel. Reads shape zero's type and keeps the first equal-distance match.
pub fn nearest(memory:&Memory<'_>,task:u32,direction:u32,cone:i32,math:&OriginalMath)->Result<u32>{
    let entity=memory.word(task.wrapping_add(0x34))?;
    let collision=memory.word(entity.wrapping_add(0x2c))?;
    let origin=position(memory,entity)?;
    let mut nearest_distance=math.constant(0x803b6520-0x73fc);let mut selected=0;
    for index in 0..16 {
        let entry=collision.wrapping_add(0x10+index*8);
        let other_task=memory.word(entry.wrapping_add(4))?;
        if other_task==0{break;}
        let other_entity=memory.word(other_task.wrapping_add(0x34))?;
        let other_collision=memory.word(other_entity.wrapping_add(0x2c))?;
        let shapes=memory.word(other_collision.wrapping_add(0xc))?;
        if memory.byte(shapes)?==0x12{continue;}
        let point=position(memory,other_entity)?;
        if origin.iter().chain(point.iter()).any(|x|!x.is_finite()){return Err("nonfinite cone position".into());}
        let dx=point[0]-origin[0];let dz=point[2]-origin[2];
        if !dx.is_finite() || !dz.is_finite(){return Err("cone delta outside finite domain".into());}
        let angle=math.angle_from_atan2(dx,dz)as u32;
        if cone<(angle_distance(direction,angle)as i32){continue;}
        let d=distance(origin,point,math)?;
        if d<nearest_distance{nearest_distance=d;selected=other_task;}
    }
    Ok(selected)
}
