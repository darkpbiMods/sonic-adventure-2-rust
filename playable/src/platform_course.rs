//! Optional fixed-tick platform preview; same snapshot drives collision and draw.
use std::cell::Cell;
use glam::{Vec3,vec3};
use sa2_runtime::{disc::Result,static_terrain_host::StaticTerrain,original_platform::PlatformMemory};
const PERIOD:usize=240;
const TASK:u32=0x80218000;const OBJECT:u32=0x80218100;const SLOT:u32=0x80218200;
pub struct PlatformCourse{frames:Vec<StaticTerrain>,centers:Vec<Vec3>,next:Cell<usize>,visible:Cell<usize>}
impl PlatformCourse{
 pub fn new()->Result<Self>{
  let source=crate::simulation::demo_source_terrain();let world=source.world();
  let base:Vec<[[f32;3];3]>=world.triangles.iter().map(|t|std::array::from_fn(|i|std::array::from_fn(|j|f32::from_be_bytes(t.data[0x18+i*12+j*4..0x1c+i*12+j*4].try_into().unwrap())))).collect();
  let centers:Vec<_>=(0..PERIOD).map(|i|vec3(4.*(i as f32*std::f32::consts::TAU/PERIOD as f32).sin(),1.5,-6.)).collect();
  let mut frames=Vec::with_capacity(PERIOD);
  for &center in &centers{
   let mut vertices=base.clone();let start=vertices.len();
   let corners=[center+vec3(-4.,0.,-3.),center+vec3(4.,0.,-3.),center+vec3(4.,0.,3.),center+vec3(-4.,0.,3.)].map(|v|(v*10.).to_array());
   vertices.push([corners[0],corners[2],corners[1]]);vertices.push([corners[0],corners[3],corners[2]]);
   frames.push(StaticTerrain::with_attachments(&vertices,&[(start,TASK),(start+1,TASK)])?);
  }
  Ok(Self{frames,centers,next:Cell::new(0),visible:Cell::new(0)})
 }
 pub fn terrain(&self)->&StaticTerrain{&self.frames[self.next.get()]}
 pub fn center(&self)->Vec3{self.centers[self.visible.get()]}
 pub fn spawn(&self)->Vec3{self.centers[self.next.get()]}
 pub fn after_tick(&self){let i=self.next.get();self.visible.set(i);self.next.set((i+1)%PERIOD);}
 pub fn prepare(&self,m:&mut PlatformMemory)->Result<()>{
  let i=self.next.get();let previous=(i+PERIOD-1)%PERIOD;
  let delta=(self.centers[i]-self.centers[previous])*10.;
  m.put_word(TASK+0x34,OBJECT)?;m.put_word(OBJECT+4,0x01000000)?;m.put_word(TASK+0x3c,SLOT)?;m.put_word(SLOT,0x80001234)?;
  for j in 0..3{m.put_float(SLOT+0x10+j as u32*4,delta[j])?;}Ok(())
 }
 pub fn callback(&self,address:u32,slot:u32)->Result<()>{if address!=0x80001234||slot!=SLOT{return Err("unknown platform preview callback".into());}Ok(())}
}
