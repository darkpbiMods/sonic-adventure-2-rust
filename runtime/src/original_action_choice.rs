//! Contextual selector80078d94 prefix, before selected action dispatch.
use crate::disc::Result;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub enum Choice{None,ReleaseLightAttack,Offered(u8)}
pub fn choose(character:&mut[u8],pressed:bool,action_button_held:bool)->Result<Choice>{
 if character.len()!=0x3a0{return Err("invalid context selection record".into());}
 let count=character[0xc]as i8;
 if !pressed{return Ok(if !action_button_held&&count>0&&character[4]==60{Choice::ReleaseLightAttack}else{Choice::None});}
 if count==0{return Ok(Choice::None);}
 let preferred=character[0xd];
 // Source scans signed count bytes directly, rather than masking each index.
 // Negative counts skip the scan and retain the preferred action.
 let chosen=if count<0{preferred}else if character[4..4+count as usize].contains(&preferred){preferred}else{character[4]};
 character[0xe]=chosen;Ok(Choice::Offered(chosen))
}
