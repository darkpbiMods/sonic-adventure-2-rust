//! Descriptor classification80083f74..800840a4, not downstream damage dispatch.
use crate::disc::Result;
pub fn update(entity:&[u8],character:&[u8],descriptor:&mut[u8])->Result<()>{
 if entity.len()!=0x40||character.len()!=0x3a0||descriptor.len()<8{return Err("invalid classification records".into());}
 let action=entity[0];let status=u16::from_be_bytes(entity[4..6].try_into().unwrap());
 let mut flags=u32::from_be_bytes(descriptor[4..8].try_into().unwrap())&!0x4000;
 let (a,b)=if u16::from_be_bytes(character[0x10..0x12].try_into().unwrap())&0x8000!=0{
  if action==69{flags|=0x4000;}(3,3)
 }else if u32::from_be_bytes(character[0x24..0x28].try_into().unwrap())&0x8000!=0{(3,3)}
 else{match action{
  60=>(2,3),59=>(1,3),69=>{flags|=0x4000;(2,1)},87=>(1,2),
  _ if status&0x400!=0=>{
   if matches!(action,61..=63|66..=68)||i16::from_be_bytes(character[0x368..0x36a].try_into().unwrap())>=2{flags|=0x4000;}(1,1)
  },_=>(0,0)
 }};
 descriptor[3]=(descriptor[3]&0xf0)|a|(b<<2);descriptor[4..8].copy_from_slice(&flags.to_be_bytes());Ok(())
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn whole_descriptors_match_original(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  let data:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/attack-classification.json")).unwrap()).unwrap();
  let bytes=|v:&serde_json::Value|v.as_array().unwrap().iter().map(|x|x.as_u64().unwrap()as u8).collect::<Vec<_>>();
  let rows=data["rows"].as_array().unwrap();assert_eq!(rows.len(),432);
  for r in rows{
   let mut e=[0u8;0x40];let mut c=[0u8;0x3a0];e[0]=r["action"].as_u64().unwrap()as u8;
   e[4..6].copy_from_slice(&(r["status"].as_u64().unwrap()as u16).to_be_bytes());
   c[0x368..0x36a].copy_from_slice(&(r["counter"].as_i64().unwrap()as i16).to_be_bytes());
   match r["override"].as_u64().unwrap(){1=>c[0x10]=0x80,2=>c[0x26]=0x80,_=>{}}
   let mut d=bytes(&r["before"]);update(&e,&c,&mut d).unwrap();assert_eq!(d,bytes(&r["after"]));
  }
 }
}
