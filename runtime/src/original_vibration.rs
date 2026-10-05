//! VibTask updater8002fdcc and destructor8002fe70 decisions.
//! Commands are original controller command numbers, not Xbox motor strengths.

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Request {pub mode:u8,pub strength:i8,pub rate:u8,pub limit:i32}
/// Enabled, mapped-controller branch of8002fb2c plus the factory's limit+1.
/// Global enable/controller routing is the host's responsibility.
pub fn request(strength:i32,rate:i32,duration:i32)->Request{
 let mut strength=strength.clamp(-7,7);
 if strength==1{strength=2;}else if strength== -1{strength= -2;}
 let duration=duration.clamp(0,255);
 Request{mode:if duration==0{1}else{2},strength:strength as i8,rate:rate.clamp(15,59)as u8,
  limit:if duration==0{strength.abs()}else{duration+1}}
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Delete {None,Deferred,Immediate}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Update {pub counter:i32,pub motor_command:Option<u8>,pub delete:Delete}

/// The original context byte equal to1 requests immediate task deletion.
/// Motor start precedes the signed expiry comparison, including limit0.
pub fn update(counter:i32,limit:i32,context_byte:u8)->Update{
 if context_byte==1{return Update{counter,motor_command:None,delete:Delete::Immediate};}
 let motor_command=(counter==1).then_some(1);
 if counter>limit{Update{counter,motor_command,delete:Delete::Deferred}}
 else{Update{counter:counter.wrapping_add(1),motor_command,delete:Delete::None}}
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Cleanup {pub slot:u32,pub motor_command:Option<u8>}
/// An old task must not clear or stop a replacement task's controller slot.
pub fn cleanup(task:u32,slot:u32,mode:u8)->Cleanup{
 if task!=slot{Cleanup{slot,motor_command:None}}
 else{Cleanup{slot:0,motor_command:Some(if mode==3{0}else{2})}}
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn original_request_fixture(){
  let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("movement-verification/vibration-request.json");
  let report:serde_json::Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
  assert_eq!(report["passed"],true);
  let mut count=0;
  for row in report["rows"].as_array().unwrap(){
   if !row["called"].as_bool().unwrap(){continue;}
   let a=&row["arguments"];let f=&row["factory_arguments"];
   let got=request(a[0].as_i64().unwrap()as i32,a[1].as_i64().unwrap()as i32,a[2].as_i64().unwrap()as i32);
   assert_eq!(got.mode as i64,f[1].as_i64().unwrap());
   assert_eq!(got.strength as i64,f[2].as_i64().unwrap());
   assert_eq!(got.rate as i64,f[3].as_i64().unwrap());
   assert_eq!(got.limit as i64,f[5].as_i64().unwrap()+1);count+=1;
  }
  assert_eq!(count,8);
  assert_eq!(request(6,15,0),Request{mode:1,strength:6,rate:15,limit:6});
 }
 #[test]fn original_lifecycle_fixture(){
  let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("movement-verification/vibration-lifecycle.json");
  let report:serde_json::Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
  assert_eq!(report["passed"],true);
  assert_eq!(report["dol_sha256"],"a53335308d1f820be236fd2b6342fee850eaf048eb26aac21ddb1903b060db2e");
  let rows=report["updates"].as_array().unwrap();assert_eq!(rows.len(),26);
  for r in rows{
   let got=update(r["age"].as_i64().unwrap()as i32,r["limit"].as_i64().unwrap()as i32,u8::from(r["paused"].as_bool().unwrap()));
   assert_eq!(got.counter as i64,r["after"].as_i64().unwrap());
   assert_eq!(got.motor_command.map(i64::from).unwrap_or(-1),r["command"].as_i64().unwrap());
   assert_eq!(match got.delete{Delete::None=>0,Delete::Deferred=>1,Delete::Immediate=>2},r["delete"].as_i64().unwrap());
  }
  let rows=report["destructors"].as_array().unwrap();assert_eq!(rows.len(),6);
  for r in rows{
   let task=0x80210000;let slot=if r["owns"].as_bool().unwrap(){task}else{task+0x1000};
   let got=cleanup(task,slot,r["mode"].as_u64().unwrap()as u8);
   assert_eq!(got.slot as u64,r["slot"].as_u64().unwrap());
   assert_eq!(got.motor_command.map(i64::from).unwrap_or(-1),r["command"].as_i64().unwrap());
  }
 }
 #[test]fn signed_counter_and_exact_context_byte(){
  assert_eq!(update(i32::MAX,i32::MAX,0).counter,i32::MIN);
  assert_eq!(update(-1,0,2),Update{counter:0,motor_command:None,delete:Delete::None});
  assert_eq!(update(1,0,0),Update{counter:1,motor_command:Some(1),delete:Delete::Deferred});
 }
}
