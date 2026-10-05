//! Sonic normal-surface portion of80054d04; host supplies context and global RNG.
//! Slope selection and external contextual output are outside this adapter.
use crate::original_effect_state::random;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct IdleState{pub selected:u16,pub counter:i16}
/// Returns true when the contextual idle-entry helper must be notified.
/// `context_draw` indicates that helper's verified ordinary extra RNG draw.
pub fn advance(state:&mut IdleState,seed:&mut u32,frame:u32,context_draw:bool)->bool{
 if matches!(state.selected,16|17|26|27|63|64){return false;}
 let mut entered=false;
 if !matches!(state.selected,0..=3|88..=90){state.selected=0;}
 else if state.selected==1&&state.counter>180{state.counter=0;state.selected=0;}
 else if state.counter>900{
  let value=random(seed);
  state.selected=88+((value as f32*(1./32768.))*1.99_f32)as u16;
  state.counter=0;entered=true;
  if context_draw{let _=random(seed);}
 }
 if !matches!(state.selected,88..=90)||frame&1!=0{state.counter=state.counter.wrapping_add(1);}
 entered
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn preservation_and_timer_controls(){
  for selected in [16,17,26,27,63,64]{let mut state=IdleState{selected,counter:901};let before=state;let mut seed=7;assert!(!advance(&mut state,&mut seed,1,true));assert_eq!(state,before);assert_eq!(seed,7);}
  for (selected,counter,next,count) in [(1,180,1,181),(1,181,0,1),(0,900,0,901),(4,901,0,902),(88,0,88,0),(0,32767,88,0)]{
   let mut state=IdleState{selected,counter};let mut seed=0;
   advance(&mut state,&mut seed,0,false);assert_eq!(state,IdleState{selected:next,counter:count});
  }
 }
 #[test]fn original_random_and_counter_fixtures(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../movement-verification");
  let read=|name|serde_json::from_slice::<serde_json::Value>(&std::fs::read(root.join(name)).unwrap()).unwrap();
  for row in read("idle-random-result.json")["observations"].as_array().unwrap(){
   let initial=row["seed"].as_u64().unwrap()as u32;
   for extra in [false,true]{
    let mut seed=initial;let mut expected_seed=initial;random(&mut expected_seed);if extra{random(&mut expected_seed);}
    let mut state=IdleState{selected:0,counter:901};assert!(advance(&mut state,&mut seed,0,extra));
    assert_eq!(state.selected,row["selected"].as_u64().unwrap()as u16);assert_eq!(state.counter,0);assert_eq!(seed,expected_seed);
   }
  }
  // Counter-tail controls within the selector's already-eligible idle domain.
  for row in read("idle-counter-result.json")["observations"].as_array().unwrap(){
   let selected=row["selected"].as_u64().unwrap()as u16;let counter=row["timer"].as_i64().unwrap()as i16;
   if counter>180||!matches!(selected,0..=3|88..=90){continue;}
   let mut state=IdleState{selected,counter};let mut seed=3;
   advance(&mut state,&mut seed,row["frame"].as_u64().unwrap()as u32,false);
   assert_eq!(state.counter,counter.wrapping_add(i16::from(row["increment"].as_bool().unwrap())));assert_eq!(seed,3);
  }
 }
}
