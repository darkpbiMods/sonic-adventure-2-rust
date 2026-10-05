//! Complete ordinary ground-acceleration parent8004c734..8004d6d0.
use crate::{disc::Result,original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES,angle_distance,angle_approach},
 original_math::OriginalMath,original_contact::Memory,original_control::InputOutputs,
 original_ground::{rotate_vector,steer_speed_dependent},original_ground_camera::CameraGroundContext,
 original_geometry::{intersect_planes,signed_angle},original_ground_path::PathContext};
fn word(b:&[u8],o:usize)->u32{u32::from_be_bytes(b[o..o+4].try_into().unwrap())}
fn float(b:&[u8],o:usize)->f32{f32::from_bits(word(b,o))}
fn vector(b:&[u8],o:usize)->[f32;3]{std::array::from_fn(|i|float(b,o+i*4))}
fn put(b:&mut[u8],o:usize,v:u32){b[o..o+4].copy_from_slice(&v.to_be_bytes());}
fn putv(b:&mut[u8],o:usize,v:[f32;3]){for i in 0..3{put(b,o+i*4,v[i].to_bits());}}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],a[0]*b[0]))}
fn cross(a:[f32;3],b:[f32;3])->[f32;3]{[b[2].mul_add(a[1],-(b[1]*a[2])),b[0].mul_add(a[2],-(b[2]*a[0])),b[1].mul_add(a[0],-(b[0]*a[1]))]}
fn status(e:&[u8])->u16{u16::from_be_bytes(e[4..6].try_into().unwrap())}
fn turn(e:&mut[u8],c:&mut[u8],target:u32,math:&OriginalMath)->Result<()>{
 let world=rotate_vector(e,vector(c,0x64),true,math)?;
 put(e,12,angle_approach(word(e,12),target,0x2000));putv(c,0x64,rotate_vector(e,world,false,math)?);Ok(())
}
pub struct GroundContext<'a>{pub camera:CameraGroundContext<'a>,pub ordinary_path_scratch:[u8;0x44]}
/// Shared original parent, including character6/7 steering selection. Its two
/// special flags dispatch before ordinary parameters or contact planes are read.
pub fn accelerate(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],ctx:&GroundContext,memory:&Memory,cache:&mut[u32;4],math:&OriginalMath)->Result<()>{
 if entity.len()!=ENTITY_BYTES||motion.len()!=0x40||character.len()!=CHARACTER_BYTES{return Err("invalid ground acceleration extent".into());}
 let flags=word(character,0x88);
 if flags&0x100000!=0{return crate::original_ground_camera::accelerate(entity,motion,character,&ctx.camera,memory,cache,math);}
 if flags&0x100!=0{return crate::original_ground_special::accelerate_flag100(entity,motion,character,ctx.camera.control,memory,cache,math);}
 for o in [0x34,0x40,0x64,0x68,0x6c,0x70,0x74,0x78,0x7c,0x80,0x84,0xd0,0xe4,0xe8,0xec,0xf4,0xfc,0x104,0x114,0x118,0x11c,0x120,0x124,0x128,0x138]{
  if !float(character,o).is_finite(){return Err("nonfinite ground acceleration parameter".into());}
 }
 let(mut e,mut m,mut c,mut next_cache)=(entity.to_vec(),motion.to_vec(),character.to_vec(),*cache);
 let control=ctx.camera.control;let k=|o:u32|math.constant(0x803b6520-o);let zero=k(0x75f8);
 let force=std::array::from_fn(|i|control.gravity[i].mul_add(float(&c,0x138),float(&m,12+i*4)));
 let mut force=rotate_vector(&e,force,false,math)?;let mut addition=zero;let mut vertical=zero;
 let mut input=InputOutputs{direction:Some(0),magnitude:Some(zero)};
 let active=crate::original_control::read(&e,&c,control,memory,&mut next_cache,&mut input,math)?;
 let workspace=word(&c,0x90);let mut normal=[0.;3];for i in 0..3{normal[i]=f32::from_bits(memory.word(workspace.wrapping_add(0x1c+i as u32*4))?);}
 let lateral=cross(normal,vector(&m,0));let slope=float(&c,0x34);let speed=float(&c,0x64);let jog=float(&c,0xe8);
 if slope<k(0x75a8)&&lateral[1].abs()>k(0x7514)&&speed>k(0x74bc){force=[zero,zero-float(&c,0x138),zero];}
 else if slope<k(0x74d0)&&speed>k(0x74bc){force[1]=-(k(0x75f0).mul_add(float(&c,0x138),-force[1]));}
 else if slope<k(0x7418)&&speed>k(0x74bc){force[1]=-(k(0x7560).mul_add(float(&c,0x138),-force[1]));}
 else if slope<k(0x75a0)&&speed>k(0x74bc){force[1]=-(k(0x7518).mul_add(float(&c,0x138),-force[1]));}
 else if slope<k(0x75cc)&&speed<jog&&speed> -jog{force[0]*=k(0x7414);force[2]*=k(0x7414);}
 else if slope<k(0x7534)&&speed<jog&&speed> -jog{force[2]*=k(0x75fc);}
 else if slope<k(0x749c)&&speed<float(&c,0xe4)&&speed> -jog{force[2]*=k(0x74e4);}
 let limit=float(&c,0xd0);let drag=float(&c,0x114);
 if !active&&speed>jog{force[0]=drag.mul_add(speed,force[0]);}
 else if active&&speed>limit&&slope>k(0x7510){force[0]=(k(0x742c)*drag).mul_add(speed-limit,force[0]);}
 else if speed>limit{force[0]=drag.mul_add(speed-limit,force[0]);}
 else if speed<zero{force[0]=drag.mul_add(speed,force[0]);}
 force[1]=float(&c,0x118).mul_add(float(&c,0x68),force[1]);force[2]=float(&c,0x11c).mul_add(float(&c,0x6c),force[2]);
 if !active{
  if float(&c,0x34)<k(0x7428){crate::original_movement::steer_slope(&mut e,&mut m,&mut c,control.gravity,control.level,control.magnitudes,math)?;}
  else if float(&c,0x64)>zero{addition=float(&c,0x104);}else if float(&c,0x64)<zero{addition=-float(&c,0x104);}
  put(&mut c,0x3c,zero.to_bits());
 }else{
  let mut target=input.direction.unwrap();let mut amount=input.magnitude.unwrap();
  if status(&e)&0x2000!=0{
   let path=PathContext{magnitudes:control.magnitudes,incoming_f28:ctx.camera.incoming_f28,incoming_f30:zero as f64,sample_scratch:ctx.ordinary_path_scratch};
   crate::original_ground_path::steer(&mut e,&mut m,&mut c,memory,&path,math)?;
   target=word(&m,0x1c);vertical=if status(&e)&0x2000!=0{vertical-k(0x7560)}else{zero};
  }else if float(&c,0x34)<k(0x7410)&&word(&c,0x88)&0x80000!=0{
   crate::original_surface::align_ground(&mut e,&mut m,&mut c,control.gravity,math)?;
  }
  let speed=float(&c,0x64);let slow=float(&c,0xe4);let jog=float(&c,0xe8);let run=float(&c,0xec);let accel=float(&c,0xfc);
  if speed<float(&c,0xd0){
   if speed<slow{
    if amount>k(0x7514){amount*=accel;}else if speed<k(0x7518)*slow{if amount<k(0x75cc){amount=k(0x75cc);}amount*=accel;}else{amount=zero;}
   }else if speed<jog{
    if amount>k(0x7534)||speed<k(0x75cc)*(jog+jog){amount*=accel;}else{amount=zero;}
   }else if speed<run{amount*=if amount>k(0x7548){accel}else{k(0x7580)*accel};}
   else{amount*=accel;}
  }else if float(&c,0x34)<zero{amount*=accel;}
  else{amount*=k(0x7518)*accel;}
  let contact=vector(&c,0x70);
  if math.estimated_vector(contact)?.0>zero as f64{
   let mut facing=e.clone();put(&mut facing,12,target);let forward=rotate_vector(&facing,[k(0x75c8),zero,zero],true,math)?;
   if dot(forward,contact)<zero{
    let surface=vector(&c,0x7c);
    let plane_a=[zero,zero,zero,contact[0],contact[1],contact[2]];let plane_b=[zero,zero,zero,surface[0],surface[1],surface[2]];
    let line=intersect_planes(plane_a,plane_b,true,math)?.1.unwrap();let mut tangent=[line[3],line[4],line[5]];
    if dot(tangent,forward)<zero{tangent=tangent.map(|v|-v);}
    target=target.wrapping_sub(signed_angle(forward,tangent,Some(surface),math)?);
   }
  }
  let kind=c[1]as i8;let alternate=kind==6||kind==7;
  let auxiliary=if alternate{memory.word(control.player_tasks[c[0]as usize].wrapping_add(0x40))?}else{0};
  let distance=angle_distance(word(&e,12),target);let speed=float(&c,0x64);
  put(&mut m,0x1c,target);
  if speed==zero&&distance>0x1000{
   if alternate{steer_speed_dependent(&mut e,&mut c,target,math)?;}else{amount=zero;turn(&mut e,&mut c,target,math)?;}
  }else if speed>=k(0x75cc)*(float(&c,0xe4)+float(&c,0xe8))&&distance>0x1000{
   amount=float(&c,0x104);
   if alternate{steer_speed_dependent(&mut e,&mut c,target,math)?;}else{crate::original_movement::steer(&mut e,&mut c,target,control.magnitudes,math)?;}
  }else if (speed>=float(&c,0xe4)&&distance<0x1000)||(speed>=float(&c,0xf4)&&word(&c,0x88)&0x80000!=0){
   steer_speed_dependent(&mut e,&mut c,target,math)?;
  }else{
   if speed>=float(&c,0xe4)&&speed<=float(&c,0xec)&&distance>0x2000{amount*=k(0x7560);}
   if alternate&&memory.byte(auxiliary.wrapping_add(0x35c))?==1{steer_speed_dependent(&mut e,&mut c,target,math)?;}
   else{crate::original_movement::steer(&mut e,&mut c,target,control.magnitudes,math)?;}
  }
  put(&mut c,0x3c,amount.to_bits());addition=amount;
 }
 let speed=float(&c,0x64);let friction=float(&c,0x40)*(force[1]*float(&c,0x128));
 let near_rest=|x:f32|x<k(0x7420)&&x>k(0x741c);
 // The source ordinary parent has this additional slow, inactive slope
 // damping branch before the otherwise matching forward-friction tree.
 if float(&c,0x34)<k(0x7428)&&speed<float(&c,0xe4)&&speed> -float(&c,0xe4)&&!active{
  force[0]*=k(0x7600);force[2]*=k(0x7600);
 }else{
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
 }
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
