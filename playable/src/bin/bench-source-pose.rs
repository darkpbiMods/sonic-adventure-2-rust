use sonic_playground::source_pose::SourcePose;
use std::{path::Path,time::Instant,hint::black_box};
fn main(){
 let args:Vec<_>=std::env::args().collect();let root=Path::new(&args[1]);let dir=root.join("movement-verification");
 let pose=SourcePose::new(&std::fs::read(dir.join("animation-bundle-audit.json")).unwrap(),std::fs::read(dir.join("animation-bundle-audit.json.bin")).unwrap(),std::fs::read(root.join("source/main.dol")).unwrap()).unwrap();
 let mut results=vec![];let mut checksum=0usize;
 for id in [0,6,9,10,65,30] {
  for _ in 0..3 {black_box(pose.sample(id,0.,1,8192).unwrap());}
  let mut samples=vec![];
  let (_,last)=pose.model_and_last_frame(id).unwrap();
  for i in 0..120 {
   let frame=last*(i as f32/119.);
   let start=Instant::now();let result=black_box(pose.sample(id,frame,1,8192).unwrap());
   samples.push(start.elapsed().as_secs_f64()*1000.);
   checksum+=result.meshes.iter().map(|m|m.vertices.len()).sum::<usize>();
  }
  samples.sort_by(f64::total_cmp);
  results.push(serde_json::json!({"animation":id,"samples":samples.len(),"median_ms":samples[samples.len()/2],"p95_ms":samples[(samples.len()*95/100).saturating_sub(1)],"max_ms":samples[samples.len()-1]}));
 }
 let mut transitions=vec![];
 for (old,new) in [(0,6),(6,9),(9,10)] {
  let start=Instant::now();let prepared=pose.prepare_transition(old,0.,new,0.).unwrap();let build_ms=start.elapsed().as_secs_f64()*1000.;
  let mut samples=vec![];
  for i in 0..33 {let start=Instant::now();black_box(prepared.sample((i%5)as f32/4.,1,8192).unwrap());if i>=3{samples.push(start.elapsed().as_secs_f64()*1000.);}}
  samples.sort_by(f64::total_cmp);transitions.push(serde_json::json!({"old":old,"new":new,"build_ms":build_ms,"median_ms":samples[samples.len()/2],"p95_ms":samples[(samples.len()*95/100).saturating_sub(1)],"max_ms":samples[samples.len()-1]}));
 }
 let report=serde_json::json!({"profile":if cfg!(debug_assertions){"debug"}else{"release"},"results":results,"transitions":transitions,"consumed_bytes":checksum,"limits":["Single poses span120phases fromzero throughlastframe; transitions use frame-zero endpoints and five fractions; excludes asset loading, packet assembly and rendering"]});
 std::fs::write(&args[2],serde_json::to_vec_pretty(&report).unwrap()).unwrap();println!("{report}");
}
