//! Ordinary 44fe0..45398 path with r21=0, surface flags=0 and global
//! tangent-clear disabled. Caller must establish those branch conditions.
use crate::{disc::Result,original_math::OriginalMath,original_ground::rotate_vector};
fn write(b:&mut[u8],o:usize,v:[f32;3]){for i in 0..3{b[o+4*i..o+4*i+4].copy_from_slice(&v[i].to_be_bytes());}}
fn vector(b:&[u8],o:usize)->[f32;3]{std::array::from_fn(|i|f32::from_be_bytes(b[o+4*i..o+4*i+4].try_into().unwrap()))}
pub struct Offsets{pub center:[f32;3],pub correction:[f32;3]}
/// Continue the ordinary static branch through candidate commit. The selected
/// plane origin must come from the actual source contact record, not host feet.
pub fn prepare_and_commit(e:&mut[u8],m:&mut[u8],c:&mut[u8],w:&mut[u8],origin:[f32;3],override_enabled:bool,math:&OriginalMath)->Result<Offsets>{
 let(mut ne,mut nm,mut nc,mut nw)=(e.to_vec(),m.to_vec(),c.to_vec(),w.to_vec());
 let offsets=prepare(&mut ne,&mut nm,&mut nc,&mut nw,math)?;
 let candidate=crate::terrain_position_gate::StaticCandidate{base:vector(&nw,0x60),delta:vector(&nw,0x6c),origin,normal:vector(&nw,0x1c),offset:offsets.center};
 let center=crate::terrain_position_gate::project(candidate,math)?;
 let mut position=vector(&ne,0x14);
 let local=vector(&nc,0x64);
 crate::terrain_position_gate::apply_static(&mut position,candidate,[local[0],local[2]],override_enabled,math)?;
 write(&mut nw,4,center);write(&mut ne,0x14,position);nc[0xa0..0xa4].fill(0);
 e.copy_from_slice(&ne);m.copy_from_slice(&nm);c.copy_from_slice(&nc);w.copy_from_slice(&nw);Ok(offsets)
}
pub fn prepare(e:&mut[u8],m:&mut[u8],c:&mut[u8],w:&mut[u8],math:&OriginalMath)->Result<Offsets>{
 if e.len()!=0x40||m.len()!=0x40||c.len()!=0x3a0||w.len()!=0x84{return Err("invalid static orientation records".into());}
 let (mut ne,mut nm,mut nc,mut nw)=(e.to_vec(),m.to_vec(),c.to_vec(),w.to_vec());
 crate::original_terrain_align::align(&mut ne,&nc,&nw,0,[0.,-1.,0.],math)?;
 let up=rotate_vector(&ne,[0.,1.,0.],true,math)?;write(&mut nw,0x1c,up);
 nw[0x34..0x38].copy_from_slice(&ne[8..12]);nw[0x38..0x3c].copy_from_slice(&ne[16..20]);
 let height=f32::from_be_bytes(nc[0x134..0x138].try_into().unwrap());
 let mut correction=[0.,0.5*height,0.];let center=rotate_vector(&ne,correction,true,math)?;
 crate::original_offset::update(&nm,&nc,up,[0.,-1.,0.],&mut correction,math)?;
 let correction=rotate_vector(&ne,correction,true,math)?;
 nm[0x18..0x1c].copy_from_slice(&nw[0x34..0x38]);nm[0x20..0x24].copy_from_slice(&nw[0x38..0x3c]);
 nc[0x7c..0x88].copy_from_slice(&nw[0x48..0x54]);write(&mut nw,0x10,[0.;3]);
 e.copy_from_slice(&ne);m.copy_from_slice(&nm);c.copy_from_slice(&nc);w.copy_from_slice(&nw);
 Ok(Offsets{center,correction})
}
#[cfg(all(test,feature="reference-tests"))]mod tests{
 use super::*;
 #[test]fn original_caller_records(){
  let math=OriginalMath::from_dol(std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../source/main.dol")).unwrap()).unwrap();
  let rows:Vec<serde_json::Value>=serde_json::from_str(include_str!("../tests/fixtures/terrain-orientation-live.json").trim_start_matches('\u{feff}')).unwrap();assert_eq!(rows.len(),6);
  for r in rows{
   let bytes=|side:&str,key:&str|->Vec<u8>{serde_json::from_value(r[side][key].clone()).unwrap()};
   let(mut e,mut m,mut c,mut w)=(bytes("input","entity"),bytes("input","motion"),bytes("input","character"),bytes("input","workspace"));
   let offsets=prepare(&mut e,&mut m,&mut c,&mut w,&math).unwrap();
   for (key,actual) in [("entity",e),("motion",m),("character",c),("workspace",w)]{assert_eq!(actual,bytes("output",key),"{key}");}
   let center:[f32;3]=serde_json::from_value(r["center"].clone()).unwrap();assert_eq!(offsets.center.map(f32::to_bits),center.map(f32::to_bits));
   let(mut e,mut m,mut c,mut w)=(bytes("input","entity"),bytes("input","motion"),bytes("input","character"),bytes("input","workspace"));
   let origin:[f32;3]=serde_json::from_value(r["origin"].clone()).unwrap();
   prepare_and_commit(&mut e,&mut m,&mut c,&mut w,origin,r["override"].as_bool().unwrap(),&math).unwrap();
   for(key,actual)in[("entity",e),("motion",m),("character",c),("workspace",w)]{assert_eq!(actual,bytes("final",key),"combined {key}");}
  }
 }
}
