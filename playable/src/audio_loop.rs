//! Host-independent sample cursor for original DSP intro/loop regions.
//! A host mixer supplies decoded PCM at its source sample rate; resampling is separate.
use crate::simulation::{Player, State, FULL_CHARGE};

pub struct LoopCursor { start: usize, end: usize, position: usize, active: bool }
impl LoopCursor {
 pub fn new(sample_count: usize, start: usize, end: usize) -> Result<Self, &'static str> {
  if start >= end || end > sample_count { return Err("invalid PCM loop region"); }
  Ok(Self { start, end, position: 0, active: false })
 }
 pub fn start(&mut self) { self.position=0; self.active=true; }
 pub fn stop(&mut self) { self.active=false; }
 pub fn active(&self)->bool { self.active }
 pub fn fill(&mut self, pcm: &[i16], output: &mut [i16]) -> Result<(), &'static str> {
  if pcm.len()<self.end { output.fill(0); self.stop(); return Err("PCM shorter than loop region"); }
  for sample in output {
   if !self.active { *sample=0; continue; }
   *sample=pcm[self.position]; self.position+=1;
   if self.position==self.end { self.position=self.start; }
  }
  Ok(())
 }
}

#[derive(Debug,PartialEq,Eq)]
pub enum ChargeSoundCommand { Start, Stop, Pause, Resume }
#[derive(Default)]
pub struct ChargeSoundState { active: bool, paused:bool, respawn: Option<u32> }
impl ChargeSoundState {
 /// Reset during a charge stops the old voice before starting the new one.
 /// Pause retains the existing voice and its sample position.
 pub fn update(&mut self,p:&Player,paused:bool)->Vec<ChargeSoundCommand> {
  self.update_active(p.state==State::Charge&&p.charge<FULL_CHARGE,p.respawns,paused)
 }
 /// Shared host voice lifecycle; the caller supplies action eligibility.
 pub fn update_active(&mut self,desired:bool,respawn:u32,paused:bool)->Vec<ChargeSoundCommand> {
  let mut commands=Vec::with_capacity(2);
  if self.respawn!=Some(respawn) && self.active {
   commands.push(ChargeSoundCommand::Stop);self.active=false;self.paused=false;
  }
  self.respawn=Some(respawn);
  if !desired&&self.active{commands.push(ChargeSoundCommand::Stop);self.active=false;self.paused=false;}
  if desired&&!self.active&&!paused{commands.push(ChargeSoundCommand::Start);self.active=true;self.paused=false;}
  if self.active&&self.paused!=paused{commands.push(if paused{ChargeSoundCommand::Pause}else{ChargeSoundCommand::Resume});self.paused=paused;}
  commands
 }
}
