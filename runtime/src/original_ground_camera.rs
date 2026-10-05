//! Complete special ground acceleration8004ba74..8004c734.
use crate::{disc::Result,original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES,angle_distance,angle_delta,angle_approach},
 original_math::{OriginalMath,IDENTITY,transform},original_contact::Memory,original_control::{ControlContext,InputOutputs},
 original_ground::rotate_vector,original_geometry::{point_plane,signed_angle},original_ground_path::PathContext};
fn word(b:&[u8],o:usize)->u32{u32::from_be_bytes(b[o..o+4].try_into().unwrap())}
fn float(b:&[u8],o:usize)->f32{f32::from_bits(word(b,o))}
fn vector(b:&[u8],o:usize)->[f32;3]{std::array::from_fn(|i|float(b,o+i*4))}
fn put(b:&mut[u8],o:usize,v:u32){b[o..o+4].copy_from_slice(&v.to_be_bytes());}
fn putv(b:&mut[u8],o:usize,v:[f32;3]){for i in 0..3{put(b,o+i*4,v[i].to_bits());}}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],a[0]*b[0]))}
fn cross(a:[f32;3],b:[f32;3])->[f32;3]{[b[2].mul_add(a[1],-(b[1]*a[2])),b[0].mul_add(a[2],-(b[2]*a[0])),b[1].mul_add(a[0],-(b[0]*a[1]))]}
fn integer(v:f64)->i32{if v.is_nan(){i32::MIN}else{v as i32}}
fn normalize(v:[f32;3],math:&OriginalMath)->Result<[f32;3]>{math.estimated_vector(v)?.1.ok_or_else(||"degenerate camera ground direction".into())}
fn turn(e:&mut[u8],c:&mut[u8],target:u32,math:&OriginalMath)->Result<()>{
 let world=rotate_vector(e,vector(c,0x64),true,math)?;
 put(e,12,angle_approach(word(e,12),target,0x2000));
 putv(c,0x64,rotate_vector(e,world,false,math)?);Ok(())
}
pub struct CameraGroundContext<'a>{
 pub control:&'a ControlContext,pub stick_x:[i16;2],pub cameras:[u32;2],
 pub incoming_f28:f64,pub path_scratch:[u8;0x44],
}
/// Surface normal is character+7c, distinct from global gravity. Camera records
/// and path/workspace records are actual mapped guest inputs, not query stubs.
pub fn accelerate(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],ctx:&CameraGroundContext,memory:&Memory,cache:&mut[u32;4],math:&OriginalMath)->Result<()>{
 if entity.len()!=ENTITY_BYTES||character.len()!=CHARACTER_BYTES||motion.len()!=0x40{return Err("invalid camera-ground extent".into());}
 let player=character[0]as i8;if !(0..2).contains(&player){return Err("invalid camera-ground player".into());}let player=player as usize;
 for o in [0x34,0x40,0x64,0x68,0x6c,0x7c,0x80,0x84,0xd0,0xe4,0xe8,0xec,0xf4,0xfc,0x104,0x114,0x118,0x11c,0x120,0x124,0x128,0x138]{
  if !float(character,o).is_finite(){return Err("nonfinite camera-ground parameter".into());}
 }
 let(mut e,mut m,mut c,mut next_cache)=(entity.to_vec(),motion.to_vec(),character.to_vec(),*cache);
 let control=ctx.control;let k=|o:u32|math.constant(0x803b6520-o);let zero=k(0x75f8);
 let normal=vector(&c,0x7c);let weight=float(&c,0x138);
 let force=std::array::from_fn(|i|-(normal[i].mul_add(weight,-float(&m,12+i*4))));
 let mut force=rotate_vector(&e,force,false,math)?;
 let mut addition=zero;let mut vertical=zero;
 let mut input=InputOutputs{direction:Some(0),magnitude:Some(zero)};
 let active=crate::original_control::read(&e,&c,control,memory,&mut next_cache,&mut input,math)?;
 let speed=float(&c,0x64);let limit=float(&c,0xd0);let drag=float(&c,0x114);
 if !active&&speed>float(&c,0xe8){force[0]=drag.mul_add(speed,force[0]);}
 else if active&&speed>limit&&float(&c,0x34)>k(0x7510){force[0]=(k(0x742c)*drag).mul_add(speed-limit,force[0]);}
 else if speed>limit{force[0]=drag.mul_add(speed-limit,force[0]);}
 else if speed<zero{force[0]=drag.mul_add(speed,force[0]);}
 force[1]=float(&c,0x118).mul_add(float(&c,0x68),force[1]);
 force[2]=float(&c,0x11c).mul_add(float(&c,0x6c),force[2]);
 if !active{
  if float(&c,0x34)<k(0x7428){crate::original_movement::steer_slope(&mut e,&mut m,&mut c,control.gravity,control.level,control.magnitudes,math)?;}
  else if float(&c,0x64)>zero{addition=float(&c,0x104);}else if float(&c,0x64)<zero{addition=-float(&c,0x104);}
  put(&mut c,0x3c,zero.to_bits());
 }else{
  let mut target=input.direction.unwrap();let mut amount=input.magnitude.unwrap();
  if u16::from_be_bytes(e[4..6].try_into().unwrap())&0x2000!=0{
   let path=PathContext{magnitudes:control.magnitudes,incoming_f28:ctx.incoming_f28,incoming_f30:zero as f64,sample_scratch:ctx.path_scratch};
   crate::original_ground_path::steer(&mut e,&mut m,&mut c,memory,&path,math)?;
   vertical-=k(0x7560);target=word(&m,0x1c);
  }
  let speed=float(&c,0x64);let slow=float(&c,0xe4);let jog=float(&c,0xe8);let run=float(&c,0xec);let accel=float(&c,0xfc);
  if speed<float(&c,0xd0){
   if speed<slow{
    if amount>k(0x7514){amount*=accel;}
    else if speed<k(0x7518)*slow{if amount<k(0x75cc){amount=k(0x75cc);}amount*=accel;}
    else{amount=zero;}
   }else if speed<jog{
    if amount>k(0x7534)||speed<k(0x75cc)*(jog+jog){amount*=accel;}else{amount=zero;}
   }else if speed<run{
    amount*=if amount>k(0x7548){accel}else{k(0x7580)*accel};
   }else{amount*=accel;}
  }else{amount*=accel;}
  if speed==zero&&angle_distance(word(&e,12),target)>0x1000{
   put(&mut m,0x1c,target);amount=zero;turn(&mut e,&mut c,target,math)?;
  }else{
   let stick=integer(math.double_constant(0x803b6520-0x75e8)*math.atan2(-(ctx.stick_x[player]as f32),-(control.stick_axis[player]as f32))as f64);
   let camera=ctx.cameras[player];let mut direction=[0.;3];for i in 0..3{direction[i]=f32::from_bits(memory.word(camera.wrapping_add(0x18+i as u32*4))?);}
   let plane=[zero,zero,zero,normal[0],normal[1],normal[2]];
   let projected=normalize(point_plane(direction,plane,true,math)?.1.unwrap(),math)?;
   let matrix=crate::original_axis_rotation::rotate(&IDENTITY,normal,stick,math)?;
   let desired=transform(&matrix,projected);
   let forward=rotate_vector(&e,[k(0x75c8),zero,zero],true,math)?;
   let mut angle=signed_angle(desired,forward,Some(normal),math)?;
   // The original computes this camera-relative difference but discards it.
   let camera_yaw=memory.word(camera.wrapping_add(0x10))?;
   let _discarded=angle_distance(0,angle_delta(word(&e,12),0xc000_u32.wrapping_sub(camera_yaw))as u32);
   if dot(cross(forward,projected),cross(desired,projected))>=zero{
    let speed=float(&c,0x64);
    if speed>float(&c,0xf4){force[2]+=crate::original_angle_table::entry(angle,math)?[0];}
    else{let step=if speed>float(&c,0xec){0x100}else if speed>float(&c,0xe8){0x200}else if speed>float(&c,0xe4){0x400}else{0x800};angle=angle_approach(0,angle,step);}
   }
   put(&mut m,0x1c,word(&e,12).wrapping_add(angle)&0xffff);
   let velocity=vector(&m,0);let(length,unit)=math.estimated_vector(velocity)?;
   if length>float(&c,0xe4)as f64&&(dot(control.gravity,unit.unwrap())as f64)>=math.double_constant(0x803b6520-0x7488){
    let mut gravity=rotate_vector(&e,control.gravity,false,math)?;
    if gravity[0]<zero{gravity[0]=-gravity[0];}
    let angle=integer(math.double_constant(0x803b6520-0x75e8)*math.atan2(gravity[2],gravity[0])as f64);
    let target=word(&e,12).wrapping_add(angle as u32);
    if gravity[2]<zero{gravity[2]=-gravity[2];}
    let step=integer((k(0x7424)*gravity[2])as f64);let current=word(&m,0x1c);
    put(&mut m,0x1c,angle_approach(current,target,step));
   }
   turn(&mut e,&mut c,word(&m,0x1c),math)?;
  }
  put(&mut c,0x3c,amount.to_bits());addition=amount;
 }
 // Full source forward-friction tree. In the reverse-speed/no-flag branch,
 // source8004c4e0 intentionally does not add the prior forward force.
 let speed=float(&c,0x64);let friction=float(&c,0x40)*(force[1]*float(&c,0x128));
 let near_rest=|x:f32|x<k(0x7420)&&x>k(0x741c);
 if speed==zero{
  force[0]+=addition;
  if !active&&force[0]<friction&&force[0]>-friction{force[0]=zero;}
  if !active&&near_rest(force[0]){force[0]=zero;}
 }else if addition<zero{
  if word(&c,0x88)&4!=0{
   let cap=if force[1]<zero{float(&c,0x40)*(force[1]*float(&c,0x120))}else{zero};
   if cap>zero&&cap< -addition{
    let mut value=addition+force[0];
    if value<zero&&value< -cap{put(&mut c,0x38,(value-cap).to_bits());value=-cap;}
    else if value>zero&&value>cap{put(&mut c,0x38,(value-cap).to_bits());value=cap;}
    force[0]=value;
   }else if !active&&speed<=float(&c,0xe4)&&near_rest(force[0]){force[0]=zero;}
   else{force[0]+=addition;}
  }else if speed>zero{
   let mut value=force[0]+addition;if value*addition<zero{value=zero;}force[0]=value;
  }else if !active&&speed<=float(&c,0xe4)&&near_rest(force[0]){force[0]=zero;}
  else{let mut value=friction+addition;if value>=zero{value=zero;}force[0]=value;}
 }else if speed<zero&&near_rest(force[0]){
  force[0]+=addition;if force[0]*addition<zero{force[0]=zero;}
 }else if friction>zero&&friction<addition{
  let mut value=addition+force[0];if value>friction{put(&mut c,0x38,(value-friction).to_bits());value=friction;}force[0]=value;
 }else if !active&&speed<=float(&c,0xe4)&&near_rest(force[0]){force[0]=zero;}
 else{force[0]+=addition;}
 if float(&c,0x6c)==zero{
  if force[2]<friction&&force[2]>-friction{force[2]=zero;}
 }else{
  let cap=if force[1]<zero{float(&c,0x40)*(force[1]*float(&c,0x124))}else{zero};
  if word(&c,0x88)&4!=0{
   force[2]=zero+force[2];
   if force[2]>zero{force[2]-=cap;if force[2]<zero{force[2]=zero;}}
   else if force[2]<zero{force[2]+=cap;if force[2]>zero{force[2]=zero;}}
  }else{
   let adjustment=if force[2]>zero{zero-cap}else if force[2]<zero{zero+cap}else{zero};
   let mut value=force[2]+adjustment;
   if force[2]!=zero&&adjustment!=zero&&value*force[2]<zero{value=zero;}force[2]=value;
  }
 }
 force[1]+=vertical;
 if force.iter().any(|v|!v.is_finite()){return Err("nonfinite camera-ground acceleration".into());}
 putv(&mut c,0x58,force);entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);*cache=next_cache;Ok(())
}
