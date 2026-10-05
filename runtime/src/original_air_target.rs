//! Complete target correction80036ce4..800372f8, finite mapped-state adapter.
use crate::{disc::Result,original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES,angle_delta,angle_distance,angle_approach},
 original_math::OriginalMath,original_contact::Memory,original_ground_camera::CameraGroundContext,
 original_target_support::{self,DirectionContext}};
fn word(b:&[u8],o:usize)->u32{u32::from_be_bytes(b[o..o+4].try_into().unwrap())}
fn float(b:&[u8],o:usize)->f32{f32::from_bits(word(b,o))}
fn vector(b:&[u8],o:usize)->[f32;3]{std::array::from_fn(|i|float(b,o+i*4))}
fn put(b:&mut[u8],o:usize,v:u32){b[o..o+4].copy_from_slice(&v.to_be_bytes());}
fn integer(v:f64)->i32{if v.is_nan(){i32::MIN}else{v as i32}}
fn range(address:u32,length:usize,base:u32,size:usize)->Result<std::ops::Range<usize>>{
 let begin=address.checked_sub(base).ok_or("target address below mapped memory")?as usize;
 let end=begin.checked_add(length).ok_or("target extent overflow")?;
 if end>size{return Err("target address outside mapped memory".into());}Ok(begin..end)
}
fn sync(mem:&mut[u8],base:u32,addresses:[u32;3],e:&[u8],m:&[u8],c:&[u8])->Result<()>{
 for(address,bytes)in addresses.into_iter().zip([e,m,c]){let at=range(address,bytes.len(),base,mem.len())?;mem[at].copy_from_slice(bytes);}Ok(())
}
/// Addresses identify entity, motion and character in the same mapped memory.
/// Error paths are transactional; a valid false return commits source writes.
pub fn correct(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],camera:&CameraGroundContext,base:u32,memory:&mut[u8],addresses:[u32;3],cache:&mut[u32;4],math:&OriginalMath)->Result<bool>{
 if entity.len()!=ENTITY_BYTES||motion.len()!=0x40||character.len()!=CHARACTER_BYTES{return Err("invalid air target extent".into());}
 let count=i16::from_be_bytes(character[0x1e..0x20].try_into().unwrap()).max(0)as usize;
 let records=if count>0{range(word(character,0xb4),count*0x20,base,memory.len())?}else{0..0};
 for(address,length)in addresses.into_iter().zip([ENTITY_BYTES,0x40,CHARACTER_BYTES]){
  let state=range(address,length,base,memory.len())?;
  if count>0&&state.start<records.end&&records.start<state.end{return Err("target records alias state adapter".into());}
 }
 for v in vector(entity,0x14).into_iter().chain(vector(motion,0)){if !v.is_finite(){return Err("nonfinite target position/velocity".into());}}
 for o in [0xcc,0xe8,0xfc,0x110,0x118,0x11c,0x138]{if !float(character,o).is_finite(){return Err("nonfinite target simulation parameter".into());}}
 for i in 0..count{for o in [0,4,8,12,16]{if !float(memory,records.start+i*32+o).is_finite(){return Err("nonfinite target record".into());}}}
 let(mut e,mut m,mut c,mut mem,mut next_cache)=(entity.to_vec(),motion.to_vec(),character.to_vec(),memory.to_vec(),*cache);
 sync(&mut mem,base,addresses,&e,&m,&c)?;
 let result=(||->Result<bool>{
  let position=vector(&e,0x14);let zero=math.constant(0x803b6520-0x75f8);let mut selected:Option<usize>=None;
  for i in 0..count{
   let at=records.start+i*32;let target=vector(&mem,at);
   if position[1]<target[1]{continue;}
   let dx=target[0]-position[0];let dz=target[2]-position[2];
   let angle=integer(math.double_constant(0x803b6520-0x75e8)*math.atan2(dz,dx)as f64)as u32;
   let delta=angle_delta(word(&e,12),angle);
   if !(-0x1800..=0x1800).contains(&delta){
    // Original multiplies Z first then X, followed by a separate add.
    let squared=dz*dz+dx*dx;
    if !squared.is_finite(){return Err("nonfinite target horizontal distance".into());}
    if math.sqrt_refined(squared)>float(&mem,at+12){continue;}
   }
   if let Some(old)=selected{if float(&mem,old+16)<float(&mem,at+16){continue;}}
   put(&mut mem,at+24,delta as u32);put(&mut mem,at+28,angle);selected=Some(at);
  }
  let Some(at)=selected else{return Ok(false);};
  let delta=word(&mem,at+24)as i32;
  if !(-0x1800..=0x1800).contains(&delta)||float(&mem,at+16)<zero{return Ok(false);}
  let saved_yaw=word(&e,12);let saved_target=word(&m,0x1c);
  // The parent sets f28 to starting Y and f30 to zero before the call.
  let nested_camera=CameraGroundContext{control:camera.control,stick_x:camera.stick_x,cameras:camera.cameras,incoming_f28:position[1]as f64,path_scratch:camera.path_scratch};
  original_target_support::direction(&mut e,&mut m,&mut c,&DirectionContext{camera:&nested_camera,incoming_f30:zero as f64},&Memory{base,bytes:&mem},&mut next_cache,math)?;
  put(&mut e,12,saved_yaw);let desired=word(&m,0x1c);put(&mut m,0x1c,saved_target);
  if angle_distance(desired,word(&mem,at+28))>0x1800{return Ok(false);}
  let target=vector(&mem,at);let mut relative=std::array::from_fn::<_,3,_>(|i|position[i]-target[i]);
  let distance=math.sqrt_refined(relative[0]*relative[0]+relative[2]*relative[2]);
  if !distance.is_finite(){return Err("nonfinite target distance".into());}put(&mut mem,at+20,distance.to_bits());
  let mut velocity=vector(&m,0);
  for step in 0..120{
   velocity[0]+=float(&c,0x110).mul_add(velocity[0],zero);
   velocity[1]+=float(&c,0x118).mul_add(velocity[1],-float(&c,0x138));
   velocity[2]+=float(&c,0x11c).mul_add(velocity[2],zero);
   let minimum=-float(&c,0xcc);
   if minimum>velocity[1]{velocity[1]=minimum;}
   for i in 0..3{relative[i]+=velocity[i];}
   if relative.iter().chain(velocity.iter()).any(|v|!v.is_finite()){return Err("nonfinite target simulation".into());}
   // This deliberately compares relative Y to the absolute target Y.
   if relative[1]>target[1]{continue;}
   relative[0]+=target[0]-position[0];relative[2]+=target[2]-position[2];
   let travelled=math.sqrt_refined(relative[0]*relative[0]+relative[2]*relative[2]);
   if !travelled.is_finite(){return Err("nonfinite target travel".into());}
   let distance=float(&mem,at+20);let radius=float(&mem,at+12);
   let inner=-(math.constant(0x803b6520-0x75d0).mul_add(radius,-distance));
   let correction=if travelled>distance{Some(inner-travelled)}else if travelled>inner{
    // f0 is reloaded from velocity Z at80037030 and retained through sqrt.
    if velocity[2].mul_add(velocity[2],velocity[0]*velocity[0])>float(&c,0xe8){Some((distance-radius)-travelled)}else{None}
   }else{Some((-(math.constant(0x803b6520-0x75cc).mul_add(radius,-distance)))-travelled)};
   let mut force=if let Some(amount)=correction{
    let [sin,cos]=crate::original_angle_table::entry(word(&mem,at+28),math)?;[amount*cos,amount*sin]
   }else{[zero,zero]};
   let divisor=math.constant(0x803b6520-0x75c8)+step as f32;let limit=float(&c,0xfc);
   for v in &mut force{*v/=divisor;if *v>limit{*v=limit;}if *v< -limit{*v= -limit;}}
   let fx=float(&m,12)+force[0];let fz=float(&m,20)+force[1];put(&mut m,12,fx.to_bits());put(&mut m,20,fz.to_bits());
   let delta=word(&mem,at+24)as i32;
   if delta<0x1000&&delta> -0x1000{let yaw=angle_approach(word(&e,12),word(&mem,at+28),0x100);put(&mut e,12,yaw);}
   sync(&mut mem,base,addresses,&e,&m,&c)?;
   original_target_support::accelerate(&e,&m,&mut c,camera.control.gravity,base,&mut mem,math)?;
   return Ok(true);
  }
  Ok(false)
 })()?;
 sync(&mut mem,base,addresses,&e,&m,&c)?;
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);memory.copy_from_slice(&mem);*cache=next_cache;Ok(result)
}
