//! Material color8011c628 and quantized color conversions80113f98/80113fc0.
use crate::{disc::Result,original_contact::Memory};
fn pack(v:[f32;4])->[u8;4]{[v[1],v[2],v[3],v[0]].map(|x|(x*256.)as u8)}
/// Returns the RGBA bytes passed to GXSetChanMatColor(channel4), or no call.
/// Global adjustment layout is alpha/red/green/blue. Finite values verified;
/// quantization saturates to unsigned8 after scaling by256 and truncating.
pub fn color(memory:&Memory,header:u32,data:u32,render_flags:u32,adjustment:[f32;4])->Result<Option<[u8;4]>>{
 if header&1==0{return Ok(None);}
 let a=memory.half(data)?;let b=memory.half(data+2)?;let source=[b as u8,(a>>8)as u8,a as u8,(b>>8)as u8];
 if render_flags&0x10!=0{return Ok(Some(pack(adjustment)));}
 if render_flags&0x20!=0{let mut v=[source[3],source[0],source[1],source[2]].map(|x|x as f32*(1./256.));for i in 0..4{v[i]+=adjustment[i];}return Ok(Some(pack(v)));}
 Ok(Some(source))
}
