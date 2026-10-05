//! Original ring Light Dash candidate selector8007b150..8007b2b4.
use crate::{disc::Result,original_math::OriginalMath};
/// Ordered host view of an original candidate entry. Missing collision records
/// are skipped, matching the two null-pointer checks in the original selector.
pub struct RingCandidate{pub position:[f32;3],pub offset:[f32;3],pub cached_distance:f32,pub collider_available:bool}
pub fn select(entity:&[u8],player_center:[f32;3],radius:f32,candidates:&[RingCandidate],math:&OriginalMath)->Result<Option<usize>>{
 if entity.len()!=0x40||!radius.is_finite()||!player_center.iter().all(|x|x.is_finite()){return Err("invalid ring selector inputs".into());}
 if candidates.is_empty(){return Ok(None);}
 let forward=crate::original_ground::rotate_vector(entity,[1.,0.,0.],true,math)?;
 let forward=math.estimated_vector(forward)?.1.unwrap_or(forward);
 let mut best:Option<usize>=None;
 for (i,c) in candidates.iter().enumerate(){
  if !c.collider_available{continue;}
  if !c.position.iter().chain(c.offset.iter()).chain([&c.cached_distance]).all(|x|x.is_finite()){return Err("nonfinite ring candidate".into());}
  let delta:[f32;3]=std::array::from_fn(|j|(c.position[j]+c.offset[j])-player_center[j]);
  let (distance,unit)=math.estimated_vector(delta)?;
  if distance>radius as f64{continue;}
  // Exact coincidence normalizes to NaNs on Gekko. The original signed-angle
  // conversion then passes its signed cone comparison. Preserve that observed
  // selector result without propagating nonfinite vectors into the host.
  if let Some(unit)=unit{if crate::original_geometry::signed_angle(forward,unit,None,math)?>0x3000{continue;}}
  if best.is_none_or(|old|c.cached_distance<candidates[old].cached_distance){best=Some(i);}
 }
 Ok(best)
}

/// Selected-ring branch8007af78..8007b12c, before collision movement.
/// player_center is the current world collider center. Local64 stores a unit
/// direction here, while motion stores speed-scaled world velocity.
pub fn steer(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],player_center:[f32;3],target:&RingCandidate,gravity_y:f32,math:&OriginalMath)->Result<()>{
 if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0||!target.collider_available{return Err("invalid ring steering records".into());}
 if !player_center.iter().chain(target.position.iter()).chain(target.offset.iter()).chain([&target.cached_distance,&gravity_y]).all(|x|x.is_finite()){return Err("nonfinite ring steering input".into());}
 let mut e=entity.to_vec();let mut m=motion.to_vec();let mut c=character.to_vec();
 let word=|b:&[u8],i:usize|u32::from_be_bytes(b[i..i+4].try_into().unwrap());
 let delta:[f32;3]=std::array::from_fn(|j|(target.position[j]+target.offset[j])-player_center[j]);
 if !delta.iter().all(|v|v.is_finite()){return Err("ring steering delta overflow".into());}
 let mut angle=(math.double_constant(0x803b6520-0x6f48)*math.atan2(delta[2],delta[0])as f64)as i32 as u32;
 if gravity_y>0.{angle=0x8000u32.wrapping_sub(angle);}
 let yaw=crate::original_gameplay::angle_approach(word(&e,12),angle,0x1800);
 e[12..16].copy_from_slice(&yaw.to_be_bytes());m[0x1c..0x20].copy_from_slice(&yaw.to_be_bytes());
 let speed=math.sqrt_refined(target.cached_distance).clamp(math.constant(0x803b6520-0x6f9c),math.constant(0x803b6520-0x6ea4));
 let (length,_)=math.estimated_vector(delta)?;
 let direction=if length<=math.constant(0x803b6520-0x6fb4)as f64{std::array::from_fn(|i|f32::from_bits(word(&m,i*4)))}else{delta};
 let unit=math.estimated_vector(direction)?.1.ok_or("ring steering zero direction outside finite source domain")?;
 for i in 0..3{m[i*4..i*4+4].copy_from_slice(&(unit[i]*speed).to_be_bytes());}
 let local=crate::original_ground::rotate_vector(&e,unit,false,math)?;
 for i in 0..3{c[0x64+i*4..0x68+i*4].copy_from_slice(&local[i].to_be_bytes());}
 let timer=i16::from_be_bytes(c[0x362..0x364].try_into().unwrap()).max(5);c[0x362..0x364].copy_from_slice(&timer.to_be_bytes());
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);Ok(())
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn invalid_steering_preserves_all_records(){
  let math=OriginalMath::from_data_bundle(include_bytes!("../../playable/assets/original-math-data.bin").to_vec()).unwrap();
  let(mut e,mut m,mut c)=([0u8;0x40],[0u8;0x40],[0u8;0x3a0]);let before=(e,m,c);
  let mut target=RingCandidate{position:[0.;3],offset:[0.;3],cached_distance:100.,collider_available:true};
  // Zero target delta and zero incoming speed have no finite source direction.
  assert!(steer(&mut e,&mut m,&mut c,[0.;3],&target,-1.,&math).is_err());assert_eq!((e,m,c),before);
  target.position=[f32::NAN,0.,0.];assert!(steer(&mut e,&mut m,&mut c,[0.;3],&target,-1.,&math).is_err());assert_eq!((e,m,c),before);
  target.position=[f32::MAX;3];target.offset=[f32::MAX;3];assert!(steer(&mut e,&mut m,&mut c,[0.;3],&target,-1.,&math).is_err());assert_eq!((e,m,c),before);
 }
}

/// Action-window selected Light Dash entry80078eac..80078f00, before sound2012.
/// Caller must first establish upgrade/input eligibility and an offered action.
pub fn enter(entity:&mut[u8],character:&mut[u8],math:&OriginalMath)->Result<()>{
 if entity.len()!=0x40||character.len()!=0x3a0{return Err("invalid ring entry records".into());}
 entity[0]=59;character[0x176..0x178].copy_from_slice(&87u16.to_be_bytes());
 character[0x362..0x364].copy_from_slice(&10i16.to_be_bytes());
 character[0x366..0x368].fill(0);character[0x36e..0x370].fill(0);
 character[0x64..0x68].copy_from_slice(&math.constant(0x803b6520-0x6ea4).to_be_bytes());
 let flags=(u16::from_be_bytes(entity[4..6].try_into().unwrap())&!0x100)|0x400;entity[4..6].copy_from_slice(&flags.to_be_bytes());Ok(())
}
///8007f320..8007f348/434 after the interrupt helper declines handling.
/// Returns true for the expiration branch. Sound and velocity exit are separate.
pub fn advance_timer(character:&mut[u8])->Result<bool>{
 if character.len()!=0x3a0{return Err("invalid ring timer records".into());}
 let timer=i16::from_be_bytes(character[0x362..0x364].try_into().unwrap());
 character[0x362..0x364].copy_from_slice(&timer.wrapping_sub(1).to_be_bytes());
 if timer<=0{return Ok(true);}
 let elapsed=i16::from_be_bytes(character[0x366..0x368].try_into().unwrap());
 character[0x366..0x368].copy_from_slice(&elapsed.wrapping_add(1).to_be_bytes());
 Ok(elapsed>360)
}

/// Timer-expiration exit around the external8004db5c query, before stop sound.
/// `direction_class` is the input-direction classifier's signed result; it is not a ring count.
pub fn expire(entity:&mut[u8],character:&mut[u8],direction_class:i32,math:&OriginalMath)->Result<()>{
 if entity.len()!=0x40||character.len()!=0x3a0{return Err("invalid ring expiry records".into());}
 let(mut e,mut c)=(entity.to_vec(),character.to_vec());
 e[0]=10;c[0x176..0x178].copy_from_slice(&15u16.to_be_bytes());
 let extended=direction_class>=2&&i16::from_be_bytes(c[0x36e..0x370].try_into().unwrap())>1;
 let limit=if extended{f32::from_be_bytes(c[0xf4..0xf8].try_into().unwrap())}else{math.constant(0x803b6520-0x6f9c)};
 if !limit.is_finite(){return Err("nonfinite ring expiry limit".into());}
 if extended{c[0x64..0x68].copy_from_slice(&limit.to_be_bytes());}
 let local=std::array::from_fn(|i|f32::from_be_bytes(c[0x64+i*4..0x68+i*4].try_into().unwrap()));
 let (length,unit)=math.estimated_vector(local)?;
 if if extended{length>0.}else{length>limit as f64}{
  let unit=unit.ok_or("invalid ring expiry direction")?;
  for i in 0..3{c[0x64+i*4..0x68+i*4].copy_from_slice(&(unit[i]*limit).to_be_bytes());}
 }
 c[0x362..0x364].fill(0);let flags=u16::from_be_bytes(e[4..6].try_into().unwrap())&!0x400;e[4..6].copy_from_slice(&flags.to_be_bytes());
 entity.copy_from_slice(&e);character.copy_from_slice(&c);Ok(())
}

/// Resolved-contact branch8007f450..8007f524, excluding stop-sound emission.
/// Returns true when contact category0x15 requests stopping sound2012.
/// Ring category0x11 counts positions separated by more than0.5source units.
pub fn contact(entity:&mut[u8],character:&mut[u8],category:u8,position:[f32;3],math:&OriginalMath)->Result<bool>{
 if entity.len()!=0x40||character.len()!=0x3a0{return Err("invalid ring contact records".into());}
 if category==0x15{
  entity[0]=10;character[0x176..0x178].copy_from_slice(&15u16.to_be_bytes());
  character[0x64..0x68].copy_from_slice(&math.constant(0x803b6520-0x6fb8).to_be_bytes());character[0x362..0x364].fill(0);
  let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap())&!0x400;entity[4..6].copy_from_slice(&flags.to_be_bytes());return Ok(true);
 }
 if category!=0x11{return Ok(false);}
 if !position.iter().all(|v|v.is_finite()){return Err("nonfinite ring contact position".into());}
 let count=i16::from_be_bytes(character[0x36e..0x370].try_into().unwrap());
 let previous=std::array::from_fn(|i|f32::from_be_bytes(character[0x384+i*4..0x388+i*4].try_into().unwrap()));
 let changed=count<1||crate::original_cone::distance(previous,position,math)?>math.constant(0x803b6520-0x6f88);
 character[0x366..0x368].fill(0);
 if changed{
  character[0x36e..0x370].copy_from_slice(&count.wrapping_add(1).to_be_bytes());
  for i in 0..3{character[0x384+i*4..0x388+i*4].copy_from_slice(&position[i].to_be_bytes());}
 }
 Ok(false)
}

/// Complete8007ae98 motion composition before action59 terrain collision.
/// Host supplies original input magnitudes, gravity and globalR13-7e08speed.
pub fn advance_motion(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],workspace:&mut[u8],workspace_base:u32,center:[f32;3],candidates:&[RingCandidate],gravity:[f32;3],magnitudes:[f32;2],fallback_speed:f32,math:&OriginalMath)->Result<Option<usize>>{
 if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0||!fallback_speed.is_finite(){return Err("invalid ring motion records".into());}
 let(mut e,mut m,mut c,mut w)=(entity.to_vec(),motion.to_vec(),character.to_vec(),workspace.to_vec());
 let target=select(&e,center,math.constant(0x803b6520-0x6eb4),candidates,math)?;
 if let Some(i)=target{steer(&mut e,&mut m,&mut c,center,&candidates[i],gravity[1],math)?;}
 else{
  let count=i16::from_be_bytes(c[0x36e..0x370].try_into().unwrap());
  if count<=1&&select(&e,center,math.constant(0x803b6520-0x6e8c),candidates,math)?.is_none(){
   let local=std::array::from_fn(|i|f32::from_be_bytes(c[0x64+i*4..0x68+i*4].try_into().unwrap()));
   let (length,unit)=math.estimated_vector(local)?;
   if length>fallback_speed as f64{let unit=unit.ok_or("nonfinite ring fallback direction")?;for i in 0..3{c[0x64+i*4..0x68+i*4].copy_from_slice(&(unit[i]*fallback_speed).to_be_bytes());}}
   c[0x362..0x364].fill(0);
  }
  crate::original_target_support::accelerate(&e,&m,&mut c,gravity,workspace_base,&mut w,math)?;
  crate::original_movement::integrate_velocity(&e,&mut m,&mut c,magnitudes,math)?;
 }
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);workspace.copy_from_slice(&w);Ok(target)
}

#[cfg(test)]mod motion_tests{
 use super::*;
 #[test]fn failed_no_target_frame_rolls_back_force_and_workspace_updates(){
  let math=OriginalMath::from_data_bundle(include_bytes!("../../playable/assets/original-math-data.bin").to_vec()).unwrap();
  let(mut e,mut m,mut c,mut w)=([0u8;0x40],[0u8;0x40],[0u8;0x3a0],[0u8;0x84]);
  e[0]=59;e[5]=1;c[0xc0..0x144].copy_from_slice(include_bytes!("../../playable/assets/physics.bin"));
  c[0x90..0x94].copy_from_slice(&0x80210000u32.to_be_bytes());c[0x34..0x38].copy_from_slice(&1f32.to_be_bytes());c[0x64..0x68].copy_from_slice(&10f32.to_be_bytes());
  let before=(e,m,c,w);
  assert!(advance_motion(&mut e,&mut m,&mut c,&mut w,0x80210000,[0.;3],&[],[0.,-1.,0.],[f32::NAN,0.],6.,&math).is_err());
  assert_eq!((e,m,c,w),before);
 }
}

/// Action59 post-collision record changes800849cc..80084b30.
/// Returns whether to stop looping sound2012. Audio/vibration/external reset
/// calls are outputs for the host; the position query has already run.
pub fn finish_collision(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],response:u32,gravity:[f32;3],level:i16,speed_limit:f32,math:&OriginalMath)->Result<bool>{
 if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0||!speed_limit.is_finite()||speed_limit<0.{return Err("invalid ring collision records".into());}
 let(mut e,mut m,mut c)=(entity.to_vec(),motion.to_vec(),character.to_vec());
 if response!=0{
  e[0]=if response==2{16}else{10};c[0x176..0x178].copy_from_slice(&(if response==2{24u16}else{15u16}).to_be_bytes());
  c[0x362..0x364].fill(0);let flags=u16::from_be_bytes(e[4..6].try_into().unwrap())&!0x400;e[4..6].copy_from_slice(&flags.to_be_bytes());
 }
 if response!=2{
  let clamp_or_zero=|b:&mut[u8],offset:usize|->Result<()>{
   let v=std::array::from_fn(|i|f32::from_be_bytes(b[offset+i*4..offset+i*4+4].try_into().unwrap()));let(length,unit)=math.estimated_vector(v)?;
   let next=if length>speed_limit as f64{unit.ok_or("invalid ring collision direction")?.map(|x|x*speed_limit)}else{[0.;3]};
   for i in 0..3{b[offset+i*4..offset+i*4+4].copy_from_slice(&next[i].to_be_bytes());}Ok(())
  };
  if response!=0{clamp_or_zero(&mut m,0)?;}
  crate::original_movement::reconcile_velocity(&mut e,&mut m,&mut c,gravity,level,math)?;
  if response!=0{clamp_or_zero(&mut c,0x64)?;}
 }
 entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);Ok(response!=0)
}

/// Ring offer construction8007ea54..8007eab8. Other offered actions are host-owned.
pub fn offer(entity:&[u8],character:&mut[u8],center:[f32;3],candidates:&[RingCandidate],math:&OriginalMath)->Result<bool>{
 if entity.len()!=0x40||character.len()!=0x3a0{return Err("invalid ring offer records".into());}
 let upgrades=u32::from_be_bytes(character[0x24..0x28].try_into().unwrap());
 let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap());
 if upgrades&0x10001==0||flags&0x800!=0||entity[0]as i8>=59{return Ok(false);}
 if select(entity,center,math.constant(0x803b6520-0x6eb4),candidates,math)?.is_none(){return Ok(false);}
 let count=character[0xc];character[0xc]=count.wrapping_add(1);character[4+(count&7)as usize]=59;Ok(true)
}
