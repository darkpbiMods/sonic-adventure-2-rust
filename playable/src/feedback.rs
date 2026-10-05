//! Fixed-tick feedback task. The host owns controller routing and motor output.
use sa2_runtime::original_vibration::{self as source,Delete,Request};

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum MotorCommand {Start,Stop,Brake}
impl MotorCommand{
 fn from_source(value:u8)->Self{match value{0=>Self::Stop,1=>Self::Start,2=>Self::Brake,_=>unreachable!("verified original command")}}
}
#[derive(Default)]
pub struct Feedback{task:Option<(Request,i32)>,motor_active:bool}
impl Feedback{
 /// Reuses the controller task and resets its counter, as8002fce0 does.
 pub fn request(&mut self,request:Request){self.task=Some((request,0));}
 /// One call per fixed simulation tick, after requests for that tick.
 /// Deferred deletion is completed at this host tick boundary.
 pub fn tick(&mut self)->Vec<MotorCommand>{
  let Some((request,counter))=self.task else{return Vec::new();};
  let result=source::update(counter,request.limit,0);
  let mut commands=Vec::with_capacity(2);
  if let Some(command)=result.motor_command{
   commands.push(MotorCommand::from_source(command));self.motor_active=command==1;
  }
  if result.delete!=Delete::None{
   let cleanup=source::cleanup(1,1,request.mode);
   if let Some(command)=cleanup.motor_command{commands.push(MotorCommand::from_source(command));}
   self.task=None;self.motor_active=false;
  }else{self.task=Some((request,result.counter));}
  commands
 }
 /// Pause/reset/disconnect discard pending feedback and brake active output.
 pub fn clear(&mut self)->Option<MotorCommand>{
  self.task=None;
  if std::mem::take(&mut self.motor_active){Some(MotorCommand::Brake)}else{None}
 }
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn hard_landing_counter_trace(){
  let mut f=Feedback::default();f.request(source::request(6,15,0));
  let trace=(0..12).map(|_|f.tick()).collect::<Vec<_>>();
  assert_eq!(trace[0],vec![]);assert_eq!(trace[1],vec![MotorCommand::Start]);
  for row in &trace[2..7]{assert!(row.is_empty());}
  assert_eq!(trace[7],vec![MotorCommand::Brake]);
  assert!(trace[8..].iter().all(Vec::is_empty));assert_eq!(f.clear(),None);
 }
 #[test]fn replacement_retains_motor_but_restarts_counter(){
  let mut f=Feedback::default();f.request(source::request(6,15,0));f.tick();f.tick();
  for _ in 0..4{f.tick();}
  f.request(source::request(6,15,0));assert!(f.tick().is_empty());
  assert_eq!(f.tick(),vec![MotorCommand::Start]);
  for _ in 0..5{assert!(f.tick().is_empty());}
  assert_eq!(f.tick(),vec![MotorCommand::Brake]);
 }
 #[test]fn clear_before_and_during_feedback(){
  for ticks in [0,1,2,5]{
   let mut f=Feedback::default();f.request(source::request(6,15,0));
   for _ in 0..ticks{f.tick();}
   assert_eq!(f.clear(),if ticks>=2{Some(MotorCommand::Brake)}else{None});
   assert_eq!(f.clear(),None);for _ in 0..10{assert!(f.tick().is_empty());}
  }
 }
 #[test]fn zero_limit_preserves_start_then_cleanup_order(){
  let mut f=Feedback::default();f.request(source::request(0,15,0));
  assert!(f.tick().is_empty());assert_eq!(f.tick(),vec![MotorCommand::Start,MotorCommand::Brake]);
 }
}
