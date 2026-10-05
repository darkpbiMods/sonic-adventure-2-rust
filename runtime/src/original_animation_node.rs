//! Complete node pose80113cd8 and channel adapters, before matrix application.
use crate::{disc::Result,original_contact::Memory,original_math::OriginalMath,original_animation_context::advance,original_animation_keys as keys,original_animation_interpolation as interpolation,original_animation_quaternion as quaternion};
fn channel(m:&Memory,c:&mut[u32;16],callback:u32,out:&mut[u32],math:&OriginalMath)->Result<bool>{
 if callback==0x80113f88||callback==0x80113f90{return Ok(false);}
 if ![0x80112d20,0x80112e18,0x80112e94].contains(&callback){return Err("unsupported original channel callback".into());}
 let offset=c[7].wrapping_mul(4);let pointer=m.word(c[3].wrapping_add(offset))?;
 if pointer!=0{
  let count=m.word(c[4].wrapping_add(offset))?;let time=f32::from_bits(c[5]);
  if callback==0x80112e94{out.copy_from_slice(&quaternion::sample(m,pointer,count,time,math)?);}
  else if callback==0x80112e18{if c[10]!=0x80113154{return Err("unsupported Euler evaluator".into());}out.copy_from_slice(&keys::euler(m,pointer,count,time)?);}
  else{let values=match c[9]{0x80112f68=>keys::linear(m,pointer,count,time)?,0x801132b4=>interpolation::cubic(m,pointer,count,time,math)?,0x80113018=>interpolation::planar(m,pointer,count,time,math)?,_=>return Err("unsupported vector evaluator".into())};out.copy_from_slice(&values);}
 }
 c[7]=c[7].wrapping_add(1);Ok(pointer!=0)
}
fn copy(m:&Memory,address:u32,out:&mut[u32])->Result<()>{for(i,v)in out.iter_mut().enumerate(){*v=m.word(address+i as u32*4)?;}Ok(())}
fn default_rotation(m:&Memory,node:u32,flags:u32,out:&mut[u32;13])->Result<u32>{
 if flags&0x400!=0{out[9]=m.word(node+0x34)?;copy(m,node+0x14,&mut out[10..13])?;Ok(2)}else{copy(m,node+0x14,&mut out[3..6])?;Ok(u32::from(flags&0x20!=0))}
}
/// Output layout: position[0..3], Euler[3..6], scale[6..9], quaternion[9..13].
/// Preserves whichever rotation output is unused. Invalid host inputs roll back.
pub fn sample(m:&Memory,node:u32,context:&mut[u32;16],output:&mut[u32;13],math:&OriginalMath)->Result<u32>{
 let mut c=*context;let mut out=*output;let flags=m.word(node)?;let rotation;
 if flags&0x40!=0{copy(m,node+8,&mut out[..3])?;copy(m,node+0x20,&mut out[6..9])?;rotation=default_rotation(m,node,flags,&mut out)?;}
 else{
  let callback=c[12];if !channel(m,&mut c,callback,&mut out[..3],math)?{copy(m,node+8,&mut out[..3])?;}
  let callback=c[13];
  if callback!=0{rotation=if channel(m,&mut c,callback,&mut out[3..6],math)?{u32::from(flags&0x20!=0)}else{default_rotation(m,node,flags,&mut out)?};}
  else{let callback=c[15];rotation=if channel(m,&mut c,callback,&mut out[9..13],math)?{2}else{default_rotation(m,node,flags,&mut out)?};}
  let callback=c[14];if !channel(m,&mut c,callback,&mut out[6..9],math)?{copy(m,node+0x20,&mut out[6..9])?;}advance(&mut c);
 }
 *context=c;*output=out;Ok(rotation)
}
