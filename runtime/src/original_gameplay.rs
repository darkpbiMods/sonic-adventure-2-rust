//! Verified build GSNE8P rev0 only. Source DOL SHA a53335308d1f820be236fd2b6342fee850eaf048eb26aac21ddb1903b060db2e.
//! These are narrow translations, NOT the entire Sonic action dispatcher.
//! Ordinary jump block 0x8007a8e8..0x8007a97c (exclusive), with attached object
//! pointer character+0xa0 == 0. External motion inheritance and audio are excluded.
//! Caller 0x80083f38..0x80083f48 passes the same character pointer as r5/r6;
//! the dispatcher preserves it as r30/r31 in the jump function.
pub const ENTITY_BYTES:usize=0x40;
pub const CHARACTER_BYTES:usize=0x3a0;
/// Complete leaf at 0x8002b904..0x8002b918. Angles wrap at 65536.
pub fn angle_delta(from:u32,to:u32)->i32 {
    (to.wrapping_sub(from) as u16 as i16) as i32
}
/// Complete leaf at 0x8002b8e0..0x8002b904; includes the half-turn case.
pub fn angle_distance(from:u32,to:u32)->u32 { angle_delta(from,to).unsigned_abs() }
/// Complete leaf at 0x8002b918..0x8002b958. Keep signed comparison and
/// wrapping arithmetic, even for unusual caller-provided step values.
pub fn angle_approach(from:u32,to:u32,step:i32)->u32 {
    let current=from&0xffff;
    let target=to&0xffff;
    let delta=angle_delta(current,target);
    if delta<=step && delta>=step.wrapping_neg() { return target; }
    if delta<0 {current.wrapping_sub(step as u32)&0xffff}
    else {current.wrapping_add(step as u32)&0xffff}
}
pub fn jump_input_allowed(status:u16,pressed_byte:u8)->bool {
    // 0x8004e454..0x8004e490: mask bit 14 then test signed byte != 0.
    status&0x4000==0 && pressed_byte!=0
}
/// Original 0x8007b630..0x8007b67c. Aim has already been converted for gravity.
/// Lifetime is the source signed character counter, not elapsed seconds.
pub fn homing_heading(current:u32,target:u32,lifetime:i16,direct:bool,ability_flags:u32)->u32 {
    if lifetime>120 || direct { target }
    else { angle_approach(current,target,if ability_flags&0x8000!=0{0x1000}else{0x800}) }
}
/// Original 8007b5e4..8007b630; preserves signed-zero atan2 and conversion order.
pub fn homing_aim(x:f32,z:f32,gravity_y:f32,math:&crate::original_math::OriginalMath)->crate::disc::Result<u32>{
    if !x.is_finite() || !z.is_finite() || !gravity_y.is_finite(){return Err("nonfinite homing aim".into());}
    let angle=(math.double_constant(0x803b6520-0x6f48)*(math.atan2(z,x)as f64))as i32 as u32&0xffff;
    Ok(if gravity_y>0.{0x8000u32.wrapping_sub(angle)}else{angle})
}
/// Original 8007b6c4..8007b7dc after vertical filtering and heading selection.
/// Source heading zero points +X. Host coordinate conversion belongs to caller.
pub fn homing_direction(delta:[f32;3],heading:u32,gravity_y:f32,math:&crate::original_math::OriginalMath)->crate::disc::Result<[f32;3]>{
    if !gravity_y.is_finite(){return Err("nonfinite homing gravity".into());}
    let (_,normalized)=math.estimated_vector(delta)?;
    // The original normalizer produces NaNs for coincident points. Reject this
    // at the host boundary; callers must resolve contact before steering.
    let direction=normalized.ok_or("coincident homing target requires contact resolution")?;
    let heading=if gravity_y>0.{0x8000u32.wrapping_sub(heading)}else{heading};
    let planar=math.sqrt_refined(1.-direction[1]*direction[1]);
    let [sin,cos]=crate::original_angle_table::entry(heading,math)?;
    Ok([cos*planar,direction[1],sin*planar])
}
fn u16_at(b:&[u8],i:usize)->u16{u16::from_be_bytes([b[i],b[i+1]])}
/// Host-owned equivalent of one original (task, cached metric) list entry.
pub struct HomingCandidate {
    pub kind:u8, pub center:Option<[f32;3]>, pub position:[f32;3],
    pub world_center:bool, pub metric:f32,
}
/// Normal-list insertion eligibility from 8000f3e0. Caller supplies a recognized
/// category-zero player and a non-type-17 target (type17 uses a separate list).
/// Positions are entity origins in source units, not collider centers.
pub fn homing_list_metric(player:[f32;3],target:[f32;3],target_collision_flags:u16,count:usize)->crate::disc::Result<Option<f32>>{
    if player.iter().chain(target.iter()).any(|v|!v.is_finite()){return Err("nonfinite homing list position".into());}
    let x=player[0]-target[0];let y=player[1]-target[1];let z=player[2]-target[2];
    let metric=z.mul_add(z,x.mul_add(x,y*y));
    if !metric.is_finite(){return Err("homing list distance overflow".into());}
    Ok(if metric<10000. && target_collision_flags&0x40!=0 && count<0x290{Some(metric)}else{None})
}
/// Ordinary list scan 8007b470..8007b56c. List construction remains the host's job.
pub fn select_homing_candidate(candidates:&[HomingCandidate],position:[f32;3],heading:u32,gravity_y:f32,ability:u32,special:bool,math:&crate::original_math::OriginalMath)->crate::disc::Result<Option<usize>>{
    if position.iter().any(|v|!v.is_finite()){return Err("nonfinite homing position".into());}
    let mut best=math.constant(0x803b6520-0x6e84);let mut selected=None;
    for (i,c) in candidates.iter().enumerate(){
        if c.kind==10{continue;}
        let Some(mut center)=c.center else{continue;};
        if !c.metric.is_finite() || center.iter().chain(c.position.iter()).any(|v|!v.is_finite()){return Err("nonfinite homing candidate".into());}
        if !c.world_center{for axis in 0..3{center[axis]+=c.position[axis];}}
        let aim=homing_aim(center[0]-position[0],center[2]-position[2],gravity_y,math)?;
        if ability&0x8000==0 && !special && angle_distance(heading,aim)>0x5000{continue;}
        if c.metric<best{best=c.metric;selected=Some(i);}
    }
    Ok(selected)
}
fn write_u16(b:&mut[u8],i:usize,x:u16){b[i..i+2].copy_from_slice(&x.to_be_bytes());}
pub fn ordinary_jump_block(entity:&mut[u8],character:&mut[u8])->Result<(),String>{
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES{return Err("unsupported state buffer size".into());}
    if character[0xa0..0xa4]!=[0,0,0,0]{return Err("attached-object jump path not translated".into());}
    jump_common(entity,character)
}
/// Jump entry8007a8e8..a97c with the attachment memory branch and80039a28.
/// `motion` must be the record owned by character byte0's player. The host
/// resolves the original global motion-pointer table before calling this API.
/// Memory is read-only; all output records remain unchanged on failure.
pub fn jump_block_with_platform(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],memory:&crate::original_contact::Memory)->crate::disc::Result<()>{
    if entity.len()!=ENTITY_BYTES||character.len()!=CHARACTER_BYTES||motion.len()!=0x40{return Err("unsupported attached jump record size".into());}
    let attachment=u32::from_be_bytes(character[0xa0..0xa4].try_into().unwrap());
    let mut transfer=None;
    if attachment!=0 {
        let object=memory.word(attachment.wrapping_add(0x34))?;
        let kind=memory.byte(object.wrapping_add(2))?;
        if !(0x1b..=0x21).contains(&kind) {
            let slots=memory.word(attachment.wrapping_add(0x3c))?;
            if slots!=0 {
                if character[0]>1{return Err("attached jump player outside two source slots".into());}
                transfer=Some([memory.word(slots.wrapping_add(0x10))?,memory.word(slots.wrapping_add(0x14))?,memory.word(slots.wrapping_add(0x18))?]);
            }
        }
    }
    let(mut e,mut c)=(entity.to_vec(),character.to_vec());
    jump_common(&mut e,&mut c)?;
    if let Some(words)=transfer {for (i,word) in words.iter().enumerate(){motion[0xc+i*4..0x10+i*4].copy_from_slice(&word.to_be_bytes());}}
    entity.copy_from_slice(&e);character.copy_from_slice(&c);Ok(())
}
fn jump_common(entity:&mut[u8],character:&mut[u8])->Result<(),String>{
    // This translation is verified for finite source physics values, including signed zero.
    let bits=u32::from_be_bytes(character[0xd8..0xdc].try_into().unwrap());
    if !f32::from_bits(bits).is_finite(){return Err("nonfinite source jump velocity is outside verified domain".into());}
    entity[0]=6;
    character[0x68..0x6c].copy_from_slice(&bits.to_be_bytes());
    write_u16(character,0x176,0x41);
    let status=((u16_at(entity,4)&0xdaff)|0x400)&!2;
    write_u16(entity,4,status);
    character[0x35e]=0;
    write_u16(character,0x12,0);
    Ok(())
}
/// Host bridge only: game units/frame -> test world units/second (0.1 * 60).
/// The status/animation outputs are retained, not silently discarded.
pub struct JumpEntry {pub velocity:f32,pub status:u16,pub action:u8,pub animation:u16}
pub fn enter_normal_jump(status:u16,pressed_byte:u8,jump_speed_bits:u32)->Result<Option<JumpEntry>,String>{
    if !jump_input_allowed(status,pressed_byte){return Ok(None);}
    let mut e=[0;ENTITY_BYTES];let mut c=[0;CHARACTER_BYTES];
    e[4..6].copy_from_slice(&status.to_be_bytes());c[0xd8..0xdc].copy_from_slice(&jump_speed_bits.to_be_bytes());
    ordinary_jump_block(&mut e,&mut c)?;
    Ok(Some(JumpEntry{velocity:f32::from_be_bytes(c[0x68..0x6c].try_into().unwrap())*6.,status:u16_at(&e,4),action:e[0],animation:u16_at(&c,0x176)}))
}
#[cfg(test)]
mod tests{
 use super::*;
 #[test]fn unsupported_state_is_not_silently_approximated(){let mut e=[0;ENTITY_BYTES];let mut c=[0;CHARACTER_BYTES];c[0xa3]=1;assert!(ordinary_jump_block(&mut e,&mut c).is_err());assert_eq!(e,[0;ENTITY_BYTES]);}
 #[test]fn adapter_outputs(){let x=enter_normal_jump(3,1,1.66_f32.to_bits()).unwrap().unwrap();assert_eq!(x.action,6);assert_eq!(x.animation,65);assert_eq!(x.status,0x401);assert_eq!(x.velocity,1.66*6.);assert!(enter_normal_jump(0x4003,1,1.66_f32.to_bits()).unwrap().is_none());}
}
