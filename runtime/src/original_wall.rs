//! Original wall response 8004614c..800463c4; collision generation is separate.
use crate::{disc::Result,original_gameplay::{angle_distance,ENTITY_BYTES,CHARACTER_BYTES},original_geometry::{intersect_planes,signed_angle},original_math::OriginalMath};
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn short(b:&[u8],i:usize)->u16{u16::from_be_bytes(b[i..i+2].try_into().unwrap())}
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
/// Values read from entity's collision record: flags at +4, vector at +90.
/// Original 80006880 copies that vector only if collision flag4 is set.
pub struct WallContact{pub flags:u16,pub vector:[f32;3]}
pub fn respond(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],contact:&WallContact,level:i16,math:&OriginalMath)->Result<()>{
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES || motion.len()<0x24{return Err("unsupported wall state size".into());}
    let status=short(entity,4);if status&3==0{return Ok(());}
    let velocity=[float(motion,0),float(motion,4),float(motion,8)];
    let (speed,old_unit)=math.estimated_vector(velocity)?;
    if speed<(math.constant(0x803b6520-0x75c8)as f64) || float(character,0x68)>0. || contact.flags&0x10==0 || status&4!=0 || contact.flags&4==0{return Ok(());}
    let mut normal=contact.vector;if level==0x5a{normal[1]=0.;}
    let (normal_length,normal_unit)=math.estimated_vector(normal)?;
    if normal_length==0.{return Ok(());}
    let normal=normal_unit.ok_or("undefined contact normal")?;
    let ground=[float(character,0x7c),float(character,0x80),float(character,0x84)];
    let plane=[0.,0.,0.,normal[0],normal[1],normal[2]];
    let ground_plane=[0.,0.,0.,ground[0],ground[1],ground[2]];
    let line=intersect_planes(plane,ground_plane,true,math)?.1.unwrap();
    let mut direction=[line[3],line[4],line[5]];
    let old_unit=old_unit.ok_or("undefined wall velocity normalization")?;
    let dot=direction[2].mul_add(old_unit[2],direction[1].mul_add(old_unit[1],old_unit[0]*direction[0]));
    if dot<0.{direction=direction.map(|x|-x);}
    // The source scales the unnormalized intersection direction by speed.
    let new_velocity=direction.map(|x|((speed)*(x as f64))as f32);
    let (_,new_unit)=math.estimated_vector(new_velocity)?;
    let new_unit=new_unit.ok_or("degenerate wall intersection outside verified domain")?;
    let angle=signed_angle(old_unit,new_unit,Some(ground),math)?;
    let mut yaw=word(entity,0xc).wrapping_sub(angle);
    if angle_distance(word(motion,0x1c),yaw)>0x6000{yaw=yaw.wrapping_add(0x8000);}
    for i in 0..3{put(character,0x70+i*4,normal[i].to_bits());put(motion,i*4,new_velocity[i].to_bits());}
    put(entity,0xc,yaw);put(motion,0x1c,yaw);Ok(())
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn degenerate_wall_response_rejects_without_partial_state(){
        let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../source/main.dol");
        let math=OriginalMath::from_dol(std::fs::read(path).unwrap()).unwrap();
        let mut e=[0;ENTITY_BYTES];let mut c=[0;CHARACTER_BYTES];let mut d=[0;0x40];
        e[5]=1;put(&mut c,0x80,1_f32.to_bits());put(&mut d,0,2_f32.to_bits());
        let old=(e,c,d);
        assert!(respond(&mut e,&mut d,&mut c,&WallContact{flags:0x14,vector:[0.,1.,0.]},0,&math).is_err());
        assert_eq!((e,c,d),old);
    }
}
