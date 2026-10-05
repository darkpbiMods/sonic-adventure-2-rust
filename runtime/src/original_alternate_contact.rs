//! Alternate collision response 800407a0 and position branch 80042e0c.
use crate::{disc::Result,original_gameplay::{angle_distance,ENTITY_BYTES,CHARACTER_BYTES},original_geometry::{signed_angle,intersect_planes},original_ground::rotate_vector,original_math::OriginalMath,original_angle_table};
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn short(b:&[u8],i:usize)->u16{u16::from_be_bytes(b[i..i+2].try_into().unwrap())}
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn vector(b:&[u8],i:usize)->[f32;3]{[float(b,i),float(b,i+4),float(b,i+8)]}
fn put_vector(b:&mut[u8],i:usize,v:[f32;3]){for j in 0..3{put(b,i+j*4,v[j].to_bits());}}
fn finite(v:[f32;3])->Result<[f32;3]>{if v.into_iter().all(f32::is_finite){Ok(v)}else{Err("nonfinite alternate contact vector".into())}}
fn unit(v:[f32;3],math:&OriginalMath)->Result<[f32;3]>{math.estimated_vector(finite(v)?)?.1.ok_or_else(||"zero alternate contact normalization outside verified domain".into())}
fn length(v:[f32;3],math:&OriginalMath)->Result<f64>{Ok(math.estimated_vector(finite(v)?)?.0)}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],b[0]*a[0]))}
fn validate(e:&[u8],c:&[u8],w:&[u8],m:&[u8])->Result<()>{
 if e.len()!=ENTITY_BYTES || c.len()!=CHARACTER_BYTES || w.len()<0x78 || m.len()<0x24{return Err("unsupported alternate contact state size".into());}
 if c[0]>1{return Err("alternate contact player outside original slots".into());}Ok(())
}
fn cross(a:[f32;3],b:[f32;3])->Result<([f32;3],f64)>{
 let a=finite(a)?;let b=finite(b)?;
 let v=finite([b[2].mul_add(a[1],-(b[1]*a[2])),b[0].mul_add(a[2],-(b[2]*a[0])),b[1].mul_add(a[0],-(b[0]*a[1]))])?;
 let squared=(v[2]as f64).mul_add(v[2]as f64,(v[1]as f64).mul_add(v[1]as f64,(v[0]*v[0])as f64));
 if !squared.is_finite(){return Err("alternate contact cross overflow".into());}
 let length=if squared==0.{0.}else{crate::gekko_estimate::reciprocal_seed(crate::gekko_estimate::reciprocal_sqrt_seed(squared))};
 Ok((v,length))
}
fn scaled(v:[f32;3],scale:f64)->Result<[f32;3]>{finite(v.map(|x|((x as f64)*scale)as f32))}

/// Selected player motion is resolved from global 801e7748 by the host.
pub fn respond(entity:&mut[u8],character:&[u8],workspace:&mut[u8],motion:&mut[u8],direction:[f32;3],math:&OriginalMath)->Result<()>{
 validate(entity,character,workspace,motion)?;
 let(mut e,mut w,mut m)=(entity.to_vec(),workspace.to_vec(),motion.to_vec());
 respond_inner(&mut e,character,&mut w,&mut m,direction,math)?;
 entity.copy_from_slice(&e);workspace.copy_from_slice(&w);motion.copy_from_slice(&m);Ok(())
}
fn respond_inner(e:&mut[u8],c:&[u8],w:&mut[u8],m:&mut[u8],direction:[f32;3],math:&OriginalMath)->Result<()>{
 let k=|o:u32|math.constant(0x803b6520-o);
 if matches!(c[1]as i8,6|7){return Ok(());}
 let grounded=short(c,0x1c)&3!=0;
 if !grounded && length(vector(c,0x64),math)?<=float(c,0xe4)as f64{return Ok(());}
 let (cross,cross_length)=cross(direction,vector(w,0x48))?;
 if cross_length==0.{return Ok(());}
 let plane_normal=if length(cross,math)?<=k(0x750c)as f64{vector(w,0x1c)}else{unit(cross,math)?};
 let p=vector(w,0x54);let n=vector(w,0x48);let q=vector(w,4);
 let line=intersect_planes([p[0],p[1],p[2],n[0],n[1],n[2]],[q[0],q[1],q[2],plane_normal[0],plane_normal[1],plane_normal[2]],true,math)?.1.unwrap();
 let mut line_direction=[line[3],line[4],line[5]];
 let old_direction=if length(direction,math)?==0.{direction}else{unit(direction,math)?};
 if dot(line_direction,old_direction)>k(0x75f8){line_direction=line_direction.map(|x|-x);}
 let source=if grounded{vector(w,0x10)}else{vector(w,0x6c)};
 put_vector(w,0x6c,scaled(line_direction,length(source,math)?)?);
 if word(c,0x8c)&0x80000!=0{put_vector(m,0,scaled(line_direction,length(vector(m,0),math)?)?);}
 let old=vector(w,0x10);let mut yaw=word(e,0xc);
 if length(old,math)?>k(0x7590)as f64{
  let angle=signed_angle(unit(old,math)?,unit(vector(w,0x6c),math)?,Some(vector(w,0x1c)),math)?;
  yaw=yaw.wrapping_sub(angle);
 }
 if angle_distance(word(m,0x1c),yaw)>0x6000{yaw=yaw.wrapping_add(0x8000);}
 put(e,0xc,yaw);put(m,0x1c,yaw);Ok(())
}

pub struct PositionContact {
 pub previous_position:[f32;3],pub offset:[f32;3],pub alternate_offset:[f32;3],
 pub previous_result:u32,pub flags:u32,pub stop_mode:u32,pub direction_mode:u32,pub level:i16,
}
/// Motion argument aliases the globally selected player's motion buffer.
pub fn position(entity:&mut[u8],character:&mut[u8],workspace:&mut[u8],motion:&mut[u8],contact:&PositionContact,math:&OriginalMath)->Result<u32>{
 validate(entity,character,workspace,motion)?;
 let(mut e,mut c,mut w,mut m)=(entity.to_vec(),character.to_vec(),workspace.to_vec(),motion.to_vec());
 let result=position_inner(&mut e,&mut c,&mut w,&mut m,contact,math)?;
 entity.copy_from_slice(&e);character.copy_from_slice(&c);workspace.copy_from_slice(&w);motion.copy_from_slice(&m);Ok(result)
}
fn set_position(e:&mut[u8],w:&mut[u8],offset:[f32;3])->Result<()>{
 let p=finite(vector(w,0x60))?;let offset=finite(offset)?;
 let adjusted=finite(std::array::from_fn(|i|p[i]-offset[i]))?;
 put_vector(w,4,p);put_vector(e,0x14,adjusted);Ok(())
}
fn stop_local(c:&mut[u8],w:&mut[u8]){put_vector(w,0x6c,[0.;3]);put_vector(c,0x64,[0.;3]);}
fn project_horizontal(e:&[u8],v:[f32;3],math:&OriginalMath)->Result<[f32;3]>{
 let mut local=rotate_vector(e,v,false,math)?;local[1]=0.;rotate_vector(e,local,true,math)
}
fn position_inner(e:&mut[u8],c:&mut[u8],w:&mut[u8],m:&mut[u8],ctx:&PositionContact,math:&OriginalMath)->Result<u32>{
 let k=|o:u32|math.constant(0x803b6520-o);
 let normal=finite(vector(w,0x48))?;put_vector(c,0x70,normal);
 let mut result=ctx.previous_result;
 'branch:{
  if short(e,4)&1!=0{
   let previous=finite(ctx.previous_position)?;let current=finite(vector(e,0x14))?;
   let delta=finite(std::array::from_fn(|i|previous[i]-current[i]))?;
   if length(delta,math)?==0.{break 'branch;}
   let direction=unit(delta,math)?;let angle=signed_angle(direction,normal,None,math)?;
   if ctx.flags&0x1000!=0{
    if angle>0x4000{return Ok(result);}
    result=1;respond_inner(e,c,w,m,direction,math)?;break 'branch;
   }
   let speed=float(c,0x64);
   if speed>=k(0x74e0){
    if ctx.flags&0x400000!=0{result=2;}
    stop_local(c,w);break 'branch;
   }
   if angle<=0x1000 && speed>=float(c,0xf0) && ctx.flags&0x400000!=0{result=2;break 'branch;}
   if angle<=0x4000{result=1;respond_inner(e,c,w,m,direction,math)?;}
   break 'branch;
  }
  if e[0]==0x38{result=1;stop_local(c,w);break 'branch;}
  let direction=if ctx.direction_mode!=0{rotate_vector(e,[k(0x75b0),k(0x75f8),k(0x75f8)],true,math)?}else{finite(vector(m,0))?.map(|x|-x)};
  if length(direction,math)?==0.{break 'branch;}
  let direction=unit(direction,math)?;let angle=signed_angle(direction,normal,None,math)?;
  let alignment=dot(direction,normal);
  if ctx.flags&0x1000!=0{
   if angle>0x4000{return Ok(result);}
   result=1;
   if ctx.level==0x2b{
    put(w,0x6c,k(0x75f8).to_bits());put(w,0x74,k(0x75f8).to_bits());
    let remaining=vector(w,0x6c);put_vector(w,0x10,remaining);
    set_position(e,w,ctx.alternate_offset)?;return Ok(1);
   }
   respond_inner(e,c,w,m,direction,math)?;break 'branch;
  }
  if float(c,0x64)>=k(0x74e0) && alignment>=k(0x74dc){result=2;stop_local(c,w);break 'branch;}
  if float(c,0x68)>k(0x75f8) && dot(vector(w,0x1c),normal)<original_angle_table::entry(0x7000,math)?[1]{
   result=1;
   put_vector(w,0x6c,project_horizontal(e,vector(w,0x6c),math)?);
   put_vector(m,0,project_horizontal(e,vector(m,0),math)?);
   let timer=word(c,0xc0)as u16;c[0x12..0x14].copy_from_slice(&timer.to_be_bytes());
   break 'branch;
  }
  if alignment>=k(0x74dc){result=if float(c,0x64)>=float(c,0xf0){2}else{1};break 'branch;}
  if matches!(c[1]as i8,4|5) && angle<=0x3400 && ctx.flags&0x1080==0{result=1;break 'branch;}
  if angle>=0x4000{
   if float(c,0x68)>=-float(c,0x138){return Ok(result);}
   break 'branch;
  }
  if short(c,0x1c)&3!=0{
   if ctx.stop_mode!=0{put_vector(w,0x6c,[k(0x75f8);3]);}
   respond_inner(e,c,w,m,direction,math)?;
  }
 }
 set_position(e,w,if e[0]==0x38{ctx.alternate_offset}else{ctx.offset})?;
 let remaining=vector(w,0x6c);
 if matches!(e[0]as i8,0xd|0xa) && length(remaining,math)?>k(0x75f4)as f64{
  put_vector(w,0x6c,unit(remaining,math)?.map(|x|x*k(0x75f4)));
 }
 let remaining=vector(w,0x6c);put_vector(w,0x10,remaining);Ok(result)
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn degenerate_alternate_contact_preserves_state(){
  let math=OriginalMath::from_dol(std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../source/main.dol")).unwrap()).unwrap();
  let mut e=[0;ENTITY_BYTES];let mut c=[0;CHARACTER_BYTES];let mut w=[0;0x80];let mut m=[0;0x40];c[0x1d]=1;
  put_vector(&mut w,0x48,[0.,1.,0.]);put_vector(&mut w,0x1c,[0.,1.,0.]);put_vector(&mut w,0x10,[2.,0.,0.]);
  let old=(e,w,m);assert!(respond(&mut e,&c,&mut w,&mut m,[0.01,1.,0.],&math).is_err());assert_eq!((e,w,m),old);
 }
}
