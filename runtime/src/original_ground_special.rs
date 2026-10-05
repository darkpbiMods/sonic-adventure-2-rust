//! Source ground-acceleration branches. The ordinary parent is separate.
use crate::{disc::Result,original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES,angle_distance,angle_approach},
 original_math::OriginalMath,original_contact::Memory,original_control::{ControlContext,InputOutputs},
 original_ground::{rotate_vector,steer_speed_dependent},original_movement::steer};
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn vector(b:&[u8],i:usize)->[f32;3]{std::array::from_fn(|j|float(b,i+j*4))}
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn putv(b:&mut[u8],i:usize,v:[f32;3]){for j in 0..3{put(b,i+j*4,v[j].to_bits());}}

/// Complete 8004b568..8004ba74, selected by character+88 flag 0x100.
/// Unlike the ordinary ground parent, this branch does not rotate its initial
/// gravity/external force into character-local coordinates. Preserve that order.
/// Full controller lookup runs against the caller's original contact graph.
pub fn accelerate_flag100(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],ctx:&ControlContext,memory:&Memory,cache:&mut[u32;4],math:&OriginalMath)->Result<()>{
 if entity.len()!=ENTITY_BYTES||character.len()!=CHARACTER_BYTES||motion.len()!=0x40{return Err("invalid special-ground state extent".into());}
 for offset in [0x34,0x40,0x64,0x68,0x6c,0xe4,0xe8,0xec,0xf4,0xfc,0x104,0x114,0x118,0x11c,0x128,0x138]{
  if !float(character,offset).is_finite(){return Err("nonfinite special-ground parameter".into());}
 }
 if ctx.gravity.iter().chain(vector(motion,12).iter()).any(|v|!v.is_finite()){return Err("nonfinite special-ground external force".into());}
 let mut e=entity.to_vec();let mut m=motion.to_vec();let mut c=character.to_vec();let mut next_cache=*cache;
 let k=|offset:u32|math.constant(0x803b6520-offset);let zero=k(0x75f8);
 let mut force:[f32;3]=std::array::from_fn(|i|ctx.gravity[i].mul_add(float(&c,0x138),float(&m,12+i*4)));
 let mut input=InputOutputs{direction:Some(0),magnitude:Some(zero)};
 let active=crate::original_control::read(&e,&c,ctx,memory,&mut next_cache,&mut input,math)?;
 let speed=float(&c,0x64);let drag=float(&c,0x114);
 if !active&&speed>float(&c,0xe8){force[0]=drag.mul_add(speed,force[0]);}
 else if speed>float(&c,0xec){force[0]=drag.mul_add(speed-float(&c,0xec),force[0]);}
 else if speed<zero{force[0]=drag.mul_add(speed,force[0]);}
 force[1]=float(&c,0x118).mul_add(float(&c,0x68),force[1]);
 force[2]=float(&c,0x11c).mul_add(float(&c,0x6c),force[2]);
 let addition=if !active{
  put(&mut c,0x3c,zero.to_bits());
  if speed>zero{float(&c,0x104)}else if speed<zero{-float(&c,0x104)}else{zero}
 }else{
  let target=input.direction.unwrap();let mut addition=input.magnitude.unwrap();
  addition=if float(&c,0xe8)>speed{addition*float(&c,0xfc)}else{addition*(k(0x7518)*float(&c,0xfc))};
  let distance=angle_distance(word(&e,12),target);put(&mut m,0x1c,target);
  if speed==zero&&distance>0x1000{
   addition=zero;let world=rotate_vector(&e,vector(&c,0x64),true,math)?;
   let yaw=angle_approach(word(&e,12),target,0x2000);put(&mut e,12,yaw);
   let local=rotate_vector(&e,world,false,math)?;putv(&mut c,0x64,local);
  }else if speed>=k(0x75cc)*(float(&c,0xe4)+float(&c,0xe8))&&distance>0x1000{
   addition=float(&c,0x104);steer(&mut e,&mut c,target,ctx.magnitudes,math)?;
  }else if speed>=float(&c,0xe4)&&distance<0x1000{
   steer_speed_dependent(&mut e,&mut c,target,math)?;
  }else{
   if speed>=float(&c,0xe4)&&speed<=float(&c,0xec)&&distance>0x2000{addition*=k(0x7560);}
   steer(&mut e,&mut c,target,ctx.magnitudes,math)?;
  }
  put(&mut c,0x3c,addition.to_bits());addition
 };
 let speed=float(&c,0x64);let friction=float(&c,0x40)*(force[1]*float(&c,0x128));
 if speed==zero{
  force[0]+=addition;
  if force[0]<friction&&force[0]>-friction{force[0]=zero;}
 }else if addition<zero{
  if speed>zero{
   let mut value=force[0]+addition;if value*addition<zero{value=zero;}force[0]=value;
  }else{
   let mut value=friction+(addition+force[0]);if value>=zero{value=zero;}force[0]=value;
  }
 }else if speed<zero{
  force[0]+=addition;if force[0]*addition<zero{force[0]=zero;}
 }else if friction>zero&&friction<addition{
  let mut value=addition+force[0];if value>friction{put(&mut c,0x38,(value-friction).to_bits());value=friction;}force[0]=value;
 }else{force[0]+=addition;}
 let sideways=float(&c,0x6c);
 if sideways==zero{if force[2]<friction&&force[2]>-friction{force[2]=zero;}}
 else{force[2]=k(0x75cc)*(-sideways);}
 force[1]+=zero;
 if force.iter().any(|v|!v.is_finite()){return Err("special-ground force outside finite domain".into());}
 putv(&mut c,0x58,force);
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);*cache=next_cache;Ok(())
}
