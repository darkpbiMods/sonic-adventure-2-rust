use sonic_playground::source_pose::SourcePose;
fn main() -> Result<(), Box<dyn std::error::Error>> {
 let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
 let dir=root.join(".local/animation");
 let pose=SourcePose::from_math_bundle(&std::fs::read(dir.join("animation-bundle-audit.json"))?,std::fs::read(dir.join("animation-bundle-audit.json.bin"))?,std::fs::read(root.join("playable/assets/original-math-data.bin"))?)?;
 let sampled=pose.sample(96,4.,66,0)?;
 let vertices:usize=sampled.meshes.iter().map(|m|m.vertices.len()/32).sum();
 assert!(!sampled.meshes.is_empty()&&vertices>0);
 println!("SONIC_POSE_OK meshes={} vertices={vertices}",sampled.meshes.len());
 Ok(())
}
