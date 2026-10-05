//! Environmental physics and surface flags, original 800420c0..800424c8.
use crate::{disc::Result,original_gameplay::{CHARACTER_BYTES,ENTITY_BYTES},original_math::OriginalMath};
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}

/// Character indexes 0..8 are supported, including Sonic's original index 0.
/// Level is the signed halfword returned by original 8001dd78.
pub fn update(entity:&[u8],character:&mut[u8],surface_flags:u32,level:i16,math:&OriginalMath)->Result<()>{
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES{return Err("unsupported environment state size".into());}
    let kind=character[1] as i8;let physics_kind=character[3] as i8;
    if !(0..=8).contains(&kind) || !(0..=8).contains(&physics_kind){return Err("environment character index outside verified table domain".into());}
    let k=|offset:u32|math.constant(0x803b6520-offset);
    let table=|kind:i8,offset:u32|math.constant(0x80196578+(kind as u32)*0x84+offset);
    let environment=word(character,0x1a8);
    let mut next=character.to_vec();
    put(&mut next,0x88,word(character,0x88)|surface_flags);
    let below=if environment&(2|0x2000)!=0 {
        let y=float(entity,0x18);let height=float(character,0x134);let boundary=float(character,0x1b0);
        let top=y+height;
        if ![y,height,boundary,top].into_iter().all(f32::is_finite){return Err("nonfinite environmental height".into());}
        boundary>top
    }else{false};
    // All three original 24-entry jump tables have the same special-level set.
    // Their case bodies are at 80042140 / 80042288 / 80042398.
    let special=matches!(level,30|32|40|43|44|50|53);
    let base_weight=if special && matches!(kind,0|1|4|5|6|7){k(0x7578)}else{table(kind,0x78)};
    let values=if environment&2!=0 && below {
        Some([base_weight*if kind==8{k(0x75cc)}else{k(0x7500)},k(0x74fc)*table(kind,0x58),k(0x74f8)*table(physics_kind,0x3c)])
    }else if environment&0x2000!=0 && below {
        Some([base_weight*k(0x7514),k(0x75fc)*table(kind,0x58),k(0x7500)*table(physics_kind,0x3c)])
    }else if environment&2==0 {
        Some([base_weight,table(kind,0x58),table(physics_kind,0x3c)])
    }else{None};
    if let Some(values)=values{
        if !values.into_iter().all(f32::is_finite){return Err("nonfinite environmental physics".into());}
        for (offset,value)in [0x138,0x118,0xfc].into_iter().zip(values){put(&mut next,offset,value.to_bits());}
    }
    let grip=if surface_flags&8!=0{k(0x750c)}else if surface_flags&4!=0{k(0x75a8)}else if surface_flags&0x10!=0{k(0x75cc)}else if surface_flags&0x200000!=0{k(0x74fc)}else{k(0x75c8)};
    put(&mut next,0x40,grip.to_bits());character.copy_from_slice(&next);Ok(())
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn invalid_environment_preserves_character(){
  let math=OriginalMath::from_dol(std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../source/main.dol")).unwrap()).unwrap();
  let mut c=[0;CHARACTER_BYTES];let mut e=[0;ENTITY_BYTES];c[1]=255;let old=c;
  assert!(update(&e,&mut c,8,30,&math).is_err());assert_eq!(c,old);
  c[1]=0;put(&mut c,0x1a8,2);put(&mut e,0x18,f32::NAN.to_bits());let old=c;
  assert!(update(&e,&mut c,8,30,&math).is_err());assert_eq!(c,old);
 }
}
