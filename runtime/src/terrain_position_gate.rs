//! Original 800454bc..8004551c candidate-position decision.
//! Distance is supplied in source units by the original candidate calculation.
//! This is not a generic host velocity dead zone or a complete terrain response.
use crate::{disc::Result as RuntimeResult,original_math::OriginalMath};
#[derive(Clone,Copy,serde::Deserialize)]
pub struct StaticCandidate {
 pub base:[f32;3],pub delta:[f32;3],pub origin:[f32;3],
 pub normal:[f32;3],pub offset:[f32;3],
}
/// Original projected workspace center. Keep the add/subtract offset operations:
/// their single-precision rounding is observable even when algebra cancels them.
pub fn project(c:StaticCandidate,math:&OriginalMath)->RuntimeResult<[f32;3]>{
 if !c.offset.iter().all(|v|v.is_finite()){return Err("nonfinite terrain offset".into());}
 let point=std::array::from_fn(|i|c.base[i]+c.delta[i]);
 let plane=[c.origin[0],c.origin[1],c.origin[2],c.normal[0],c.normal[1],c.normal[2]];
 let projected=crate::original_geometry::point_plane(point,plane,true,math)?.1.ok_or("missing projected point")?;
 let center=std::array::from_fn(|i|c.offset[i]+projected[i]);
 if !center.iter().all(|v:&f32|v.is_finite()){return Err("terrain candidate overflow".into());}
 Ok(center)
}
/// Bounded static-contact composition; caller must supply actual source query
/// outputs. Does not integrate velocity, select contacts or run the terrain tail.
pub fn apply_static(position:&mut[f32;3],c:StaticCandidate,local:[f32;2],override_enabled:bool,math:&OriginalMath)->RuntimeResult<bool>{
 let center=project(c,math)?;
 let candidate=std::array::from_fn(|i|center[i]-c.offset[i]);
 let distance=crate::original_terrain_contacts::distance(*position,candidate,math)?;
 let commit=should_commit(distance,local[0],local[1],false,override_enabled)?;
 if commit{*position=candidate;}
 Ok(commit)
}
pub fn should_commit(distance:f32,local_forward:f32,local_lateral:f32,has_task:bool,override_enabled:bool)->Result<bool,&'static str>{
 if ![distance,local_forward,local_lateral].iter().all(|v|v.is_finite())||distance<0.{
  return Err("invalid terrain candidate distance or local velocity");
 }
 let stationary=!has_task&&local_forward==0.&&local_lateral==0.;
 Ok(override_enabled||(distance>0.0001&&(!stationary||distance>1.)))
}
#[cfg(all(test,feature="reference-tests"))]mod tests{
 use super::*;
 #[test]fn recorded_original_composition(){
  let math=OriginalMath::from_dol(std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../source/main.dol")).unwrap()).unwrap();
  for (fixture,count) in [(include_str!("../tests/fixtures/terrain-static-candidate.json"),120),(include_str!("../tests/fixtures/terrain-live-candidate.json"),6)]{
  let report:serde_json::Value=serde_json::from_str(fixture.trim_start_matches('\u{feff}')).unwrap();
  let rows=report["rows"].as_array().unwrap();assert_eq!(rows.len(),count);
  for row in rows{
   let c:StaticCandidate=serde_json::from_value(row["candidate"].clone()).unwrap();
   let mut position:[f32;3]=serde_json::from_value(row["position"].clone()).unwrap();
   let local:[f32;2]=serde_json::from_value(row["local"].clone()).unwrap();
   let expected:[f32;3]=serde_json::from_value(row["result"].clone()).unwrap();
   apply_static(&mut position,c,local,row["override"].as_bool().unwrap(),&math).unwrap();
   assert_eq!(position.map(f32::to_bits),expected.map(f32::to_bits),"{row}");
  }
  }
 }
 #[test]fn recorded_original_projection(){
  let math=OriginalMath::from_dol(std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../source/main.dol")).unwrap()).unwrap();
  let report:serde_json::Value=serde_json::from_str(include_str!("../tests/fixtures/terrain-projection.json")).unwrap();
  let rows=report["rows"].as_array().unwrap();assert_eq!(rows.len(),8);
  for row in rows{
   let c:StaticCandidate=serde_json::from_value(row.clone()).unwrap();
   let expected:[f32;3]=serde_json::from_value(row["result"].clone()).unwrap();
   assert_eq!(project(c,&math).unwrap().map(f32::to_bits),expected.map(f32::to_bits));
   let mut position=[10.,20.,30.];let before=position;let mut bad=c;bad.normal=[0.;3];
   assert!(apply_static(&mut position,bad,[0.;2],true,&math).is_err());assert_eq!(position,before);
  }
 }
 #[test]fn recorded_original_decisions(){
  let report:serde_json::Value=serde_json::from_str(include_str!("../tests/fixtures/terrain-position-gate.json")).unwrap();
  let rows=report["rows"].as_array().unwrap();assert_eq!(rows.len(),40);
  for r in rows{
   let speed=if r["moving"].as_bool().unwrap(){1.}else{0.};
   let result=should_commit(r["distance"].as_f64().unwrap() as f32,speed,0.,r["attached"].as_bool().unwrap(),r["override"].as_i64().unwrap()!=0).unwrap();
   assert_eq!(result,r["committed"].as_bool().unwrap(),"{r}");
  }
 }
 #[test]fn invalid_inputs_do_not_become_override_commits(){
  for bad in [f32::NAN,f32::INFINITY,-1.]{assert!(should_commit(bad,0.,0.,false,true).is_err());}
  assert!(should_commit(0.,f32::NAN,0.,false,true).is_err());
  assert!(should_commit(0.,0.,f32::INFINITY,false,true).is_err());
 }
}
