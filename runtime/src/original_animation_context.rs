//! Original motion context80113848/8011388c/801138c8/80113b58.
use crate::{disc::Result,original_contact::Memory,original_math::OriginalMath};
/// Raw words preserve the original64byte context and untouched trailing slot.
pub fn initialize(context:&mut[u32;16],memory:&Memory,motion:u32,time:f32,math:&OriginalMath)->Result<()>{
 let(data,frames,flags,layout)=if motion==0{(0,0,0,0)}else{(memory.word(motion)?,memory.word(motion+4)?,memory.half(motion+8)?as u32,memory.half(motion+10)?as u32)};
 let selector=(layout&0xc0)>>6;
 // Three source interpolation families. A fourth selector aliases beyond the
 // source dispatch arrays; reject until that domain is separately reconstructed.
 if selector>2{return Err("unverified interpolation selector3".into());}
 let mut c=*context;c[0]=data;c[6]=frames;c[8]=flags;c[5]=time.to_bits();c[7]=0;
 c[1]=(layout&!0xc0).wrapping_mul(4);c[2]=c[1].wrapping_mul(2);c[3]=data;c[4]=data.wrapping_add(c[1]);
 c[9]=math.constant(0x801d30a0+selector*4).to_bits();c[10]=math.constant(0x801d30ac+selector*4).to_bits();c[11]=math.constant(0x801d30b8).to_bits();
 c[12]=if flags&1!=0{0x80112d20}else{0x80113f88};
 if flags&2!=0{c[13]=0x80112e18;c[15]=0;}else if flags&0x2000!=0{c[13]=0;c[15]=0x80112e94;}else{c[13]=0x80113f90;c[15]=0;}
 c[14]=if flags&4!=0{0x80112d20}else{0x80113f88};*context=c;Ok(())
}
pub fn advance(context:&mut[u32;16]){context[0]=context[0].wrapping_add(context[2]);context[3]=context[0];context[4]=context[0].wrapping_add(context[1]);context[7]=0;}
