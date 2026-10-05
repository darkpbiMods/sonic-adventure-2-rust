//! Blend8011cc08 and cached depth/alpha helpers801229ec/80122a44/80122ab4.
use crate::{disc::Result,original_contact::Memory};
pub struct State{pub override_byte:u8,pub selector:u32,pub depth_lock:u8,pub depth_mode:u32,pub texture:u32}
pub fn set_selector(header:u32,selector:&mut u32,dirty:&mut u32){*selector=header&0x3f00;*dirty|=8;}
pub fn apply(memory:&Memory,flags:u32,state:&mut State)->Result<Vec<Vec<u32>>>{
 let mode=if state.override_byte!=0{3}else{u32::from(flags&0x88!=0)};
 let src=(state.selector>>11)&7;let dst=(state.selector>>8)&7;
 let mut calls=vec![vec![0x80104ff8,mode,src,dst,0]];
 let target=if mode==0{0}else if mode==1 && flags&0x80==0 && src!=1 && dst!=1 && state.texture!=0 && memory.word(state.texture)?==14{1}else{2};
 if state.depth_lock==1 || state.depth_mode==target{return Ok(calls);}
 match target{
  0=>{calls.push(vec![0x8010517c,1,1,1]);calls.push(vec![0x801051f4,1]);},
  1=>{calls.push(vec![0x80104aa0,4,64,0,7,255]);calls.push(vec![0x8010517c,1,1,1]);calls.push(vec![0x801051f4,0]);},
  _=>{calls.push(vec![0x80104aa0,4,0,0,7,255]);calls.push(vec![0x8010517c,1,3,0]);calls.push(vec![0x801051f4,0]);}
 }
 state.depth_mode=target;Ok(calls)
}
