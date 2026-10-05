//! Hit eligibility from8000f2f4; overlap, callbacks and damage amount are external.
use crate::disc::Result;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct ClassifiedContact {
 pub sonic_classification:u8,
 pub sonic_descriptor_flags:u32,
 pub marks_receiver:bool,
}
/// Evaluate an already-overlapping pair. Inputs are source records, not game
/// state labels. The host owns shape overlap, contact registration, target HP,
/// callbacks and repeated-contact policy. No input record is modified.
pub fn classify_sonic_contact(entity:&[u8],character:&[u8],sonic_descriptor:&[u8],
                              receiver_descriptor:&[u8],receiver_category:u16)->Result<ClassifiedContact>{
 if sonic_descriptor.len()!=0x30||receiver_descriptor.len()!=0x30{return Err("contact requires 48-byte source descriptors".into());}
 let mut descriptor=sonic_descriptor.to_vec();
 crate::original_attack_classification::update(entity,character,&mut descriptor)?;
 let receiver_flags=u32::from_be_bytes(receiver_descriptor[4..8].try_into().unwrap());
 Ok(ClassifiedContact{
  sonic_classification:descriptor[3],
  sonic_descriptor_flags:u32::from_be_bytes(descriptor[4..8].try_into().unwrap()),
  marks_receiver:marks_hit(descriptor[3],receiver_descriptor[3],receiver_flags,receiver_category)?,
 })
}
/// Classification bytes are descriptor+3; receiver flags are descriptor+4.
/// Only the first four source category masks have been verified for this API.
pub fn marks_hit(attacker:u8,receiver:u8,receiver_flags:u32,receiver_category:u16)->Result<bool>{
 let mask=match receiver_category{0=>0x20,1=>0,2|3=>0x40,_=>return Err("unverified collision category".into())};
 Ok(receiver_flags&0x400!=0&&(attacker&3)>((receiver>>2)&3)&&attacker&mask!=0)
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn composed_moves_respect_receiver_defense_and_category(){
  let c=[0u8;0x3a0];let mut descriptor=[0u8;0x30];descriptor[3]=0xe0;
  let mut receiver=[0u8;0x30];receiver[4..8].copy_from_slice(&0x400u32.to_be_bytes());
  for (action,status,attack) in [(0,0,0),(6,0x100,0),(59,0x500,1),(70,0x400,1),(69,0x500,2),(60,0x500,2)]{
   let mut e=[0u8;0x40];e[0]=action;e[4..6].copy_from_slice(&(status as u16).to_be_bytes());
   for defense in 0..=3{
    receiver[3]=defense<<2;
    for category in 0..=3{
     let result=classify_sonic_contact(&e,&c,&descriptor,&receiver,category).unwrap();
     assert_eq!(result.sonic_classification&3,attack);
     assert_eq!(result.marks_receiver,category!=1&&attack>defense);
    }
   }
  }
  let mut e=[0u8;0x40];e[0]=69;receiver[3]=0;
  let mut excluded=descriptor;excluded[3]=0x20;
  assert!(!classify_sonic_contact(&e,&c,&excluded,&receiver,2).unwrap().marks_receiver);
  receiver[4..8].fill(0);
  assert!(!classify_sonic_contact(&e,&c,&descriptor,&receiver,2).unwrap().marks_receiver);
  assert!(classify_sonic_contact(&e,&c,&descriptor,&receiver,4).is_err());
  assert!(classify_sonic_contact(&e,&c,&descriptor[..8],&receiver,2).is_err());
 }
 #[test]fn matches_original_pair_matrix(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let data:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/attack-pair.json")).unwrap()).unwrap();
  let rows=data["rows"].as_array().unwrap();assert_eq!(rows.len(),16384);
  for r in rows{
   let n=|s:&str|r[s].as_u64().unwrap();
   assert_eq!(marks_hit(n("a")as u8,n("b")as u8,n("fb")as u32,n("category")as u16).unwrap(),r["hit_b"].as_bool().unwrap());
   assert_eq!(marks_hit(n("b")as u8,n("a")as u8,n("fa")as u32,n("category")as u16).unwrap(),r["hit_a"].as_bool().unwrap());
  }
  assert!(marks_hit(0xff,0,0x400,4).is_err());
  assert!(marks_hit(0xff,0,0x400,u16::MAX).is_err());
 }
}
