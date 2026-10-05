//! Texture front ends8011c76c/8011c85c, independent of host GPU backend.
use crate::{disc::Result,original_contact::Memory};
pub struct Selection{pub descriptor:u32,pub init:[u32;8],pub load:[u32;2]}
pub fn select(memory:&Memory,root:u32,header:u32,texture:u32,slot:u32)->Result<Selection>{
 if slot>7{return Err("unverified texture slot above seven".into());}
 let list=memory.word(root+0x20)?;let table=memory.word(list)?;let entry=table.wrapping_add((texture&0x1fff)*12);let wrapper=memory.word(entry+8)?;let descriptor=memory.word(wrapper+12)?.wrapping_add(4);
 let object=0x80357fb0+slot*32;
 let x=if header&0x2000!=0{0}else if header&0x8000!=0{2}else{1};let y=if header&0x1000!=0{0}else if header&0x4000!=0{2}else{1};
 Ok(Selection{descriptor,init:[object,memory.word(descriptor+0x1c)?,memory.word(descriptor+0xc)?&65535,memory.word(descriptor+0x10)?&65535,memory.word(descriptor)?,x,y,memory.word(descriptor+0x18)?>>31],load:[object,slot]})
}
