//! Original direction80048490 and targeting acceleration80048834.
use crate::{disc::Result,original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES},
 original_math::{OriginalMath,IDENTITY,transform},original_contact::Memory,
 original_control::InputOutputs,original_ground::rotate_vector,
 original_ground_camera::CameraGroundContext,original_ground_path::PathContext};
fn word(b:&[u8],o:usize)->u32{u32::from_be_bytes(b[o..o+4].try_into().unwrap())}
fn float(b:&[u8],o:usize)->f32{f32::from_bits(word(b,o))}
fn vector(b:&[u8],o:usize)->[f32;3]{std::array::from_fn(|i|float(b,o+i*4))}
fn put(b:&mut[u8],o:usize,v:u32){b[o..o+4].copy_from_slice(&v.to_be_bytes());}
fn status(e:&[u8])->u16{u16::from_be_bytes(e[4..6].try_into().unwrap())}
fn integer(v:f64)->i32{if v.is_nan(){i32::MIN}else{v as i32}}
pub struct DirectionContext<'a,'b>{pub camera:&'a CameraGroundContext<'b>,pub incoming_f30:f64}
/// Original80048490..80048724. Caller scratch/FPR state is explicit because
/// the early path branch inherits it before this routine writes any FPRs.
pub fn direction(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],ctx:&DirectionContext,memory:&Memory,cache:&mut[u32;4],math:&OriginalMath)->Result<()>{
 if entity.len()!=ENTITY_BYTES||motion.len()!=0x40||character.len()!=CHARACTER_BYTES{return Err("invalid target direction extent".into());}
 if status(entity)&0x4000!=0{return Ok(());}
 let camera=ctx.camera;let control=camera.control;
 let(mut e,mut m,mut c,mut next_cache)=(entity.to_vec(),motion.to_vec(),character.to_vec(),*cache);
 if status(&e)&0x2000!=0{
  let path=PathContext{magnitudes:control.magnitudes,incoming_f28:camera.incoming_f28,incoming_f30:ctx.incoming_f30,sample_scratch:camera.path_scratch};
  crate::original_ground_path::steer(&mut e,&mut m,&mut c,memory,&path,math)?;
 }else{
  let mut input=InputOutputs{direction:Some(0),magnitude:None};
  let active=crate::original_control::read(&e,&c,control,memory,&mut next_cache,&mut input,math)?;
  if !active&&status(&e)&3!=0{
   // Both source speed branches call this same helper and then return.
   crate::original_movement::steer_slope(&mut e,&mut m,&mut c,control.gravity,control.level,control.magnitudes,math)?;
  }else{
   let target=if !active{word(&e,12)}else if word(&c,0x88)&0x100000!=0{
    let player=c[0]as i8;if !(0..2).contains(&player){return Err("invalid direction player".into());}let player=player as usize;
    let stick=integer(math.double_constant(0x803b6520-0x75e8)*math.atan2(camera.stick_x[player]as f32,control.stick_axis[player]as f32)as f64);
    let pointer=camera.cameras[player];
    let matrix=math.rotate(&IDENTITY,b'y',memory.word(pointer.wrapping_add(0x10))?as i32)?;
    let matrix=math.rotate(&matrix,b'x',memory.word(pointer.wrapping_add(0xc))?as i32)?;
    let zero=math.constant(0x803b6520-0x75f8);
    let view=transform(&matrix,[zero,zero,math.constant(0x803b6520-0x75b0)]);
    let normal=vector(&c,0x7c);
    let axis=crate::original_axis_rotation::rotate(&IDENTITY,normal,stick,math)?;
    let desired=transform(&axis,view);
    let forward=rotate_vector(&e,[math.constant(0x803b6520-0x75c8),zero,zero],true,math)?;
    let angle=crate::original_geometry::signed_angle(forward,desired,Some(normal),math)?;
    let target=word(&e,12).wrapping_add(angle)&0xffff;put(&mut m,0x1c,target);
    crate::original_ground::steer_speed_dependent(&mut e,&mut c,target,math)?;
    target
   }else{input.direction.unwrap()};
   put(&mut m,0x1c,target);
   crate::original_movement::steer(&mut e,&mut c,target,control.magnitudes,math)?;
  }
 }
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);*cache=next_cache;Ok(())
}
/// Complete80048834..80048acc. The source consumes workspace+78..80 on the
/// grounded branch, adds only transformed Y/Z, then applies axis drag.
pub fn accelerate(entity:&[u8],motion:&[u8],character:&mut[u8],gravity:[f32;3],base:u32,memory:&mut[u8],math:&OriginalMath)->Result<()>{
 if entity.len()!=ENTITY_BYTES||motion.len()!=0x40||character.len()!=CHARACTER_BYTES{return Err("invalid target acceleration extent".into());}
 if status(entity)&0x4000!=0{return Ok(());}
 for o in [0x34,0x64,0x68,0x6c,0xe8,0x114,0x118,0x11c,0x138]{if !float(character,o).is_finite(){return Err("nonfinite target acceleration parameter".into());}}
 if gravity.iter().chain(vector(motion,12).iter()).any(|v|!v.is_finite()){return Err("nonfinite target external force".into());}
 let(mut c,mut mem)=(character.to_vec(),memory.to_vec());let k=|o:u32|math.constant(0x803b6520-o);
 let force=std::array::from_fn(|i|gravity[i].mul_add(float(&c,0x138),float(motion,12+i*4)));
 let mut force=rotate_vector(entity,force,false,math)?;
 if float(&c,0x64)>float(&c,0xe8)&&float(&c,0x34)<k(0x75a4){force[1]*=k(0x7468);}
 if status(entity)&3!=0{
  let divisor=float(&c,0xd0)/(float(&c,0x138)/k(0x74e4));
  if !divisor.is_finite()||divisor==0.{return Err("undefined grounded target divisor".into());}
  let offset=word(&c,0x90).checked_sub(base).ok_or("target workspace below mapped memory")?as usize;
  let start=offset.checked_add(0x78).ok_or("target workspace overflow")?;
  let slice=mem.get_mut(start..start+12).ok_or("target workspace outside mapped memory")?;
  let extra=vector(slice,0).map(|v|v/divisor);
  for i in 0..3{put(slice,i*4,k(0x75f8).to_bits());}
  let local=rotate_vector(entity,extra,false,math)?;
  force[1]+=local[1];force[2]+=local[2];
 }
 let drag=if status(entity)&0x100!=0&&(float(&c,0x34)as f64)<=math.double_constant(0x803b6520-0x7460){k(0x7458)}else{float(&c,0x114)};
 force[0]=drag.mul_add(float(&c,0x64),force[0]);
 force[1]=float(&c,0x118).mul_add(float(&c,0x68),force[1]);
 force[2]=float(&c,0x11c).mul_add(float(&c,0x6c),force[2]);
 if force.iter().any(|v|!v.is_finite()){return Err("nonfinite target acceleration result".into());}
 for i in 0..3{put(&mut c,0x58+i*4,force[i].to_bits());}
 character.copy_from_slice(&c);memory.copy_from_slice(&mem);Ok(())
}
