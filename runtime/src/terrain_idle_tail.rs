//! Bounded 800456c4..800457d4 policy. Caller owns contact search and bypass.
#[derive(Clone,Copy,serde::Deserialize)]
pub struct Context {
 pub previous_support:u16, pub local_y:f32, pub local_forward:f32,
 pub run_speed:f32, pub slope:f32, pub previous_collision_flags:u32,
 pub input_active:bool, pub input_magnitude:f32, pub input_lock:i16,
}
/// Apply the original tail to source-unit position and world motion. This must
/// not be called on an arbitrary host post-contact position: preceding source
/// displacement, workspace correction and bypass conditions are prerequisites.
pub fn apply(position:&mut [f32;3],motion:&mut [f32;3],correction:[f32;3],c:Context)->Result<bool,&'static str>{
 if !position.iter().chain(motion.iter()).chain(correction.iter()).all(|v|v.is_finite())
  || ![c.local_y,c.local_forward,c.run_speed,c.slope,c.input_magnitude].iter().all(|v|v.is_finite()) {
  return Err("nonfinite terrain tail input");
 }
 let clear=c.previous_support&1!=0 && c.previous_support&0x4000==0
  && (!c.input_active||(c.input_magnitude<=0.9&&c.input_lock==0))
  && c.local_y<=0. && c.local_forward<=c.run_speed
  && c.previous_collision_flags&0x1c==0 && c.slope>0.7;
 let next=std::array::from_fn(|i|if clear{(-0.5_f32).mul_add(motion[i],position[i])}else{position[i]+correction[i]});
 if !next.iter().all(|v|v.is_finite()){return Err("terrain tail position overflow");}
 *position=next;if clear{*motion=[0.;3];}Ok(clear)
}
#[cfg(all(test,feature="reference-tests"))]mod tests{
 use super::*;
 #[test]fn recorded_original_thresholds(){
  let report:serde_json::Value=serde_json::from_str(include_str!("../tests/fixtures/terrain-idle-tail.json")).unwrap();
  let rows=report["rows"].as_array().unwrap();assert_eq!(rows.len(),25);
  for row in rows {
   let c:Context=serde_json::from_value(row["context"].clone()).unwrap();
   let mut p=[10.,20.,30.];let mut v=[2.,4.,6.];
   assert_eq!(apply(&mut p,&mut v,[0.25,0.5,0.75],c).unwrap(),row["clears"].as_bool().unwrap(),"{}",row["case"]);
   let expected_p:[f32;3]=serde_json::from_value(row["position"].clone()).unwrap();
   let expected_v:[f32;3]=serde_json::from_value(row["motion"].clone()).unwrap();
   assert_eq!(p,expected_p);assert_eq!(v,expected_v);
  }
 }
 #[test]fn original_recorded_branch_controls(){
  for (status,y,x,slope,flags,stick,lock,clear) in [
   (1,0.,0.,0.8,0,0.,0,true),(0,0.,0.,0.8,0,0.,0,false),
   (0x4001,0.,0.,0.8,0,0.,0,false),(1,0.01,0.,0.8,0,0.,0,false),
   (1,0.,4.,0.8,0,0.,0,false),(1,0.,0.,0.7,0,0.,0,false),
   (1,0.,0.,0.8,4,0.,0,false),(1,0.,0.,0.8,0,0.5,0,true),
   (1,0.,0.,0.8,0,1.,0,false),(1,0.,0.,0.8,0,0.5,1,true)]{
   let mut p=[10.,20.,30.];let mut v=[2.,4.,6.];
   let c=Context{previous_support:status,local_y:y,local_forward:x,run_speed:3.,slope,previous_collision_flags:flags,input_active:stick!=0.&&lock==0,input_magnitude:stick,input_lock:lock};
   assert_eq!(apply(&mut p,&mut v,[0.25,0.5,0.75],c).unwrap(),clear);
   assert_eq!(p,if clear{[9.,18.,27.]}else{[10.25,20.5,30.75]});
   assert_eq!(v,if clear{[0.;3]}else{[2.,4.,6.]});
   let before=(p,v);let mut bad=c;bad.slope=f32::NAN;
   assert!(apply(&mut p,&mut v,[0.;3],bad).is_err());assert_eq!((p,v),before);
  }
 }
}
