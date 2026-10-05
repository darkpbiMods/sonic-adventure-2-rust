//! Windows host adapter: intro buffer followed by the original PCM loop region.
//! WinMM owns queued buffers until reset/unprepare. Gain and mixing are host adaptations.
use crate::{audio_loop::{ChargeSoundState,ChargeSoundCommand},simulation::Player};
use serde::Deserialize;
use sha2::{Digest,Sha256};
use std::path::Path;
#[derive(Deserialize)]struct Manifest{entries:Vec<Entry>}
#[derive(Deserialize)]struct Entry{name:String,file:String,sha256:String,playback_rate:u32,r#loop:Option<Region>}
#[derive(Deserialize)]struct Region{start_frame:usize,end_frame_exclusive:usize}

pub struct ChargeAudio{device:native::Device,state:ChargeSoundState,pub starts:u32,pub stops:u32}
impl ChargeAudio{
 pub fn load(root:&Path)->Result<Self,String>{
  Self::load_cue(root,"charge-manifest.json","charge-upgraded","charge-upgraded.wav")
 }
 pub fn load_ring(root:&Path)->Result<Self,String>{
  Self::load_cue(root,"manifest.json","light-attack","light-attack.wav")
 }
 fn load_cue(root:&Path,manifest_file:&str,name:&str,file:&str)->Result<Self,String>{
  let root=root.join("audio");
  let manifest:Manifest=serde_json::from_slice(&std::fs::read(root.join(manifest_file)).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
  // Demo enables Sonic's upgrades; source mask00010001 selects cue2011.
  let e=manifest.entries.iter().find(|e|e.name==name).ok_or("missing looping cue")?;
  if e.file!=file||e.playback_rate!=22050{return Err("invalid charge asset mapping".into());}
  let b=std::fs::read(root.join(&e.file)).map_err(|e|e.to_string())?;
  if format!("{:x}",Sha256::digest(&b))!=e.sha256{return Err("charge WAV hash mismatch".into());}
  if b.len()<44||&b[..4]!=b"RIFF"||&b[8..16]!=b"WAVEfmt "||&b[16..24]!=[16,0,0,0,1,0,1,0]||&b[34..40]!=[16,0,b'd',b'a',b't',b'a']{return Err("invalid charge WAV format".into());}
  let rate=u32::from_le_bytes(b[24..28].try_into().unwrap());
  let length=u32::from_le_bytes(b[40..44].try_into().unwrap())as usize;
  if rate!=e.playback_rate||length!=b.len()-44||length%2!=0{return Err("invalid charge WAV length/rate".into());}
  let pcm:Vec<i16>=b[44..].chunks_exact(2).map(|s|(i16::from_le_bytes([s[0],s[1]])as f32*0.35).round()as i16).collect();
  let region=e.r#loop.as_ref().ok_or("missing cue loop")?;
  crate::audio_loop::LoopCursor::new(pcm.len(),region.start_frame,region.end_frame_exclusive).map_err(str::to_owned)?;
  let device=native::Device::new(rate,&pcm[..region.start_frame],&pcm[region.start_frame..region.end_frame_exclusive])?;
  Ok(Self{device,state:Default::default(),starts:0,stops:0})
 }
 pub fn update(&mut self,p:&Player,paused:bool)->Result<(),String>{
  let commands=self.state.update(p,paused);
  self.apply(commands)
 }
 pub fn update_ring(&mut self,p:&Player,paused:bool)->Result<(),String>{
  let commands=self.state.update_active(p.state==crate::simulation::State::RingDash,p.respawns,paused);
  self.apply(commands)
 }
 fn apply(&mut self,commands:Vec<ChargeSoundCommand>)->Result<(),String>{
  for command in commands{match command{
   ChargeSoundCommand::Start=>{self.device.start()?;self.starts+=1;},
   ChargeSoundCommand::Stop=>{self.device.stop()?;self.stops+=1;},
   ChargeSoundCommand::Pause=>self.device.pause()?,
   ChargeSoundCommand::Resume=>self.device.resume()?,
  }}Ok(())
 }
}
#[cfg(windows)]mod native{
 use std::{ffi::c_void,mem::size_of,ptr::null_mut};
 #[repr(C,packed(2))]struct Format{tag:u16,channels:u16,rate:u32,bytes_per_second:u32,align:u16,bits:u16,extra:u16}
 #[repr(C)]struct Header{data:*mut i16,length:u32,recorded:u32,user:usize,flags:u32,loops:u32,next:*mut Header,reserved:usize}
 #[cfg(test)]
 #[repr(C)]struct MultimediaTime{kind:u32,payload:[u32;2]}
 #[link(name="winmm")]unsafe extern "system"{
  fn waveOutOpen(out:*mut *mut c_void,id:u32,format:*const Format,callback:usize,instance:usize,flags:u32)->u32;
  fn waveOutPrepareHeader(h:*mut c_void,header:*mut Header,size:u32)->u32;
  fn waveOutUnprepareHeader(h:*mut c_void,header:*mut Header,size:u32)->u32;
  fn waveOutWrite(h:*mut c_void,header:*mut Header,size:u32)->u32;
  fn waveOutPause(h:*mut c_void)->u32;fn waveOutRestart(h:*mut c_void)->u32;
  fn waveOutReset(h:*mut c_void)->u32;fn waveOutClose(h:*mut c_void)->u32;
  #[cfg(test)]fn waveOutGetPosition(h:*mut c_void,time:*mut MultimediaTime,size:u32)->u32;
 }
 fn check(code:u32,operation:&str)->Result<(),String>{if code==0{Ok(())}else{Err(format!("{operation}: WinMM error {code}"))}}
 struct Buffer{header:Box<Header>,_pcm:Box<[i16]>,prepared:bool}
 impl Buffer{fn new(pcm:&[i16])->Self{
  let mut pcm=pcm.to_vec().into_boxed_slice();
  let header=Box::new(Header{data:pcm.as_mut_ptr(),length:(pcm.len()*2)as u32,recorded:0,user:0,flags:0,loops:0,next:null_mut(),reserved:0});
  Self{header,_pcm:pcm,prepared:false}
 }}
 pub struct Device{handle:*mut c_void,buffers:Vec<Buffer>}
 impl Device{
  pub fn new(rate:u32,intro:&[i16],loop_pcm:&[i16])->Result<Self,String>{
   if intro.is_empty()||loop_pcm.is_empty(){return Err("empty charge region".into());}
   let f=Format{tag:1,channels:1,rate,bytes_per_second:rate*2,align:2,bits:16,extra:0};
   let mut d=Self{handle:null_mut(),buffers:vec![Buffer::new(intro),Buffer::new(loop_pcm)]};
   unsafe{check(waveOutOpen(&mut d.handle,u32::MAX,&f,0,0,0),"open charge audio")?;
    for b in &mut d.buffers{check(waveOutPrepareHeader(d.handle,&mut *b.header,size_of::<Header>()as u32),"prepare charge buffer")?;b.prepared=true;}
   }Ok(d)
  }
  pub fn start(&mut self)->Result<(),String>{
   self.stop()?;
   let result=(||unsafe{
    check(waveOutPause(self.handle),"pause charge queue")?;
    for (i,b) in self.buffers.iter_mut().enumerate(){
     // Reset returned both buffers. Keep WHDR_PREPARED; source loop is only buffer1.
     b.header.flags=2|if i==1{4|8}else{0};b.header.loops=if i==1{u32::MAX}else{0};
     check(waveOutWrite(self.handle,&mut *b.header,size_of::<Header>()as u32),"queue charge buffer")?;
    }
    check(waveOutRestart(self.handle),"start charge audio")
   })();
   if result.is_err(){let _=self.stop();}result
  }
  pub fn stop(&mut self)->Result<(),String>{unsafe{check(waveOutReset(self.handle),"stop charge audio")}}
  pub fn pause(&mut self)->Result<(),String>{unsafe{check(waveOutPause(self.handle),"pause charge audio")}}
  pub fn resume(&mut self)->Result<(),String>{unsafe{check(waveOutRestart(self.handle),"resume charge audio")}}
  #[cfg(test)]pub fn sample_position(&self)->Result<u32,String>{
   let mut time=MultimediaTime{kind:2,payload:[0;2]};
   unsafe{check(waveOutGetPosition(self.handle,&mut time,size_of::<MultimediaTime>()as u32),"read charge position")?;}
   if time.kind!=2{return Err(format!("driver substituted position format {}",time.kind));}
   Ok(time.payload[0])
  }
 }
 impl Drop for Device{fn drop(&mut self){
  if self.handle.is_null(){return;}
  unsafe{
   let reset=waveOutReset(self.handle);
   for mut b in self.buffers.drain(..){
    if b.prepared&&(reset!=0||waveOutUnprepareHeader(self.handle,&mut *b.header,size_of::<Header>()as u32)!=0){
     // On driver failure, retain allocations rather than free driver-owned memory.
     std::mem::forget(b);
    }
   }
   let _=waveOutClose(self.handle);
  }
 }}
}
#[cfg(all(test,windows))]
mod device_tests{
 use super::*;
 use crate::simulation::State;
 use std::{thread::sleep,time::Duration};
 #[test]
 #[ignore = "plays charge audio on the Windows default output device"]
 fn native_charge_pause_preserves_position(){
  let mut audio=ChargeAudio::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")).unwrap();
  let mut player=Player::new();player.state=State::Charge;
  audio.update(&player,false).unwrap();sleep(Duration::from_millis(300));
  let playing=audio.device.sample_position().unwrap();assert!(playing>0);
  audio.update(&player,true).unwrap();
  let paused=audio.device.sample_position().unwrap();
  sleep(Duration::from_millis(200));
  let held=audio.device.sample_position().unwrap();assert_eq!(paused,held);
  audio.update(&player,false).unwrap();sleep(Duration::from_millis(150));
  let resumed=audio.device.sample_position().unwrap();assert!(resumed>held);
  assert_eq!((audio.starts,audio.stops),(1,0));
  player.reset();audio.update(&player,false).unwrap();
  let stopped=audio.device.sample_position().unwrap();assert_eq!(stopped,0);
  assert_eq!((audio.starts,audio.stops),(1,1));
  println!("NATIVE_CHARGE_PAUSE_PASS playing={playing} paused={paused} held={held} resumed={resumed} stopped={stopped}");
 }
 #[test]
 #[ignore = "plays charge audio on the Windows default output device"]
 fn native_ring_pause_preserves_position(){
  let mut audio=ChargeAudio::load_ring(&Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")).unwrap();
  let mut player=Player::new();player.state=State::RingDash;
  audio.update_ring(&player,false).unwrap();sleep(Duration::from_millis(300));
  let playing=audio.device.sample_position().unwrap();assert!(playing>0);
  audio.update_ring(&player,true).unwrap();
  let paused=audio.device.sample_position().unwrap();
  sleep(Duration::from_millis(200));
  let held=audio.device.sample_position().unwrap();assert_eq!(paused,held);
  audio.update_ring(&player,false).unwrap();sleep(Duration::from_millis(150));
  let resumed=audio.device.sample_position().unwrap();assert!(resumed>held);
  assert_eq!((audio.starts,audio.stops),(1,0));
  player.reset();audio.update_ring(&player,false).unwrap();
  let stopped=audio.device.sample_position().unwrap();assert_eq!(stopped,0);
  assert_eq!((audio.starts,audio.stops),(1,1));
  println!("NATIVE_RING_PAUSE_PASS playing={playing} paused={paused} held={held} resumed={resumed} stopped={stopped}");
 }
}
#[cfg(not(windows))]mod native{
 pub struct Device;
 impl Device{
  pub fn new(_:u32,_:&[i16],_:&[i16])->Result<Self,String>{Err("charge output adapter requires Windows".into())}
  pub fn start(&mut self)->Result<(),String>{Err("charge output unavailable".into())}
  pub fn stop(&mut self)->Result<(),String>{Ok(())}
  pub fn pause(&mut self)->Result<(),String>{Ok(())}
  pub fn resume(&mut self)->Result<(),String>{Ok(())}
 }
}
