//! Host-authored static triangles for the recovered collision solver.
use crate::{disc::Result,original_terrain_candidates::*,original_position_a::PositionState,original_platform::PlatformMemory,original_terrain_sweep::SweepState,ground_motion_host::GroundMotion};
pub struct StaticTerrain{objects:Vec<WorldObject>,entries:Vec<StaticEntry>,triangles:Vec<Triangle>,nodes:Vec<QueryNode>}
impl StaticTerrain{
 /// One continuous static collision object with a Z-sorted triangle chain.
 /// Useful for connected surfaces: their contact planes must be considered
 /// together rather than sequentially resolved as independent objects.
 pub fn new_mesh(vertices:&[[[f32;3];3]])->Result<Self>{
  Self::new_mesh_with_flags(vertices,0)
 }
 /// Raw original surface flags; caller is responsible for selecting their
 /// intended collision policy. This does not infer material flags from geometry.
 pub fn new_mesh_with_flags(vertices:&[[[f32;3];3]],flags:u32)->Result<Self>{
  if vertices.is_empty(){return Self::new(vertices);}
  Self::with_mesh_ranges(vertices,&[(0..vertices.len(),flags)])
 }
 /// Connect selected disjoint ranges while preserving all original identifiers.
 /// Ungrouped triangles remain separate objects. Range flags are raw source flags.
 pub fn with_mesh_ranges(vertices:&[[[f32;3];3]],ranges:&[(std::ops::Range<usize>,u32)])->Result<Self>{
  let mut occupied=vec![false;vertices.len()];
  for (range,_) in ranges{
   if range.start>=range.end||range.end>vertices.len(){return Err("invalid connected mesh range".into());}
   for i in range.clone(){if occupied[i]{return Err("overlapping connected mesh ranges".into());}occupied[i]=true;}
  }
  let mut result=Self::new(vertices)?;let mut removed=std::collections::HashSet::new();
  for (range,flags) in ranges{
   let (start,end,flags)=(range.start,range.end,*flags);
  let mut center=[0.;3];
  for j in 0..3{
   let lo=vertices[start..end].iter().flatten().map(|p|p[j]).fold(f32::INFINITY,f32::min);
   let hi=vertices[start..end].iter().flatten().map(|p|p[j]).fold(f32::NEG_INFINITY,f32::max);
   center[j]=lo*0.5+hi*0.5;
  }
  let radius=vertices[start..end].iter().flatten().map(|p|(0..3).map(|j|(p[j]-center[j]).powi(2)).sum::<f32>().sqrt()).fold(0.,f32::max)+0.001;
  if !radius.is_finite(){return Err("mesh bounds overflow".into());}
  result.objects[start].flags=flags;result.entries[start].flags=flags;
  result.objects[start].position=center;result.objects[start].model_radius=radius;
  let mut order:Vec<_>=(start..end).collect();
  order.sort_by(|&a,&b|{
   let z=|i:usize|f32::from_be_bytes(result.triangles[i].data[16..20].try_into().unwrap());
   z(a).total_cmp(&z(b))
  });
  let ids:Vec<_>=order.iter().map(|&i|result.triangles[i].identifier).collect();
  for (n,&i) in order.iter().enumerate(){
   result.triangles[i].data[0..4].copy_from_slice(&ids.get(n+1).copied().unwrap_or(0).to_be_bytes());
   result.triangles[i].data[4..8].copy_from_slice(&n.checked_sub(1).map_or(0,|j|ids[j]).to_be_bytes());
  }
  result.nodes[start].data[0x10..0x14].copy_from_slice(&ids[0].to_be_bytes());
  result.nodes[start].data[0x14..0x18].copy_from_slice(&ids[ids.len()-1].to_be_bytes());
  result.nodes[start].data[0x28..0x2c].copy_from_slice(&radius.to_be_bytes());
   for i in start+1..end{removed.insert(result.objects[i].identifier);}
  }
  result.objects.retain(|o|!removed.contains(&o.identifier));
  result.entries.retain(|e|!removed.contains(&e.identifier));
  result.nodes.retain(|n|!removed.contains(&u32::from_be_bytes(n.data[8..12].try_into().unwrap())));
  Ok(result)
 }
 /// Vertices are source units, Y up. Preserve winding: it determines contact normals.
 pub fn new(vertices:&[[[f32;3];3]])->Result<Self>{
  // Total world objects are separate from per-query192candidate/128contact caps.
  if vertices.len()>1024{return Err("static host terrain exceeds current 1024-object adapter capacity".into());}
  let mut result=Self{objects:Vec::new(),entries:Vec::new(),triangles:Vec::new(),nodes:Vec::new()};
  for (index,v) in vertices.iter().enumerate(){
   if !v.iter().flatten().all(|x|x.is_finite()){return Err("nonfinite terrain vertex".into());}
   let center:[f32;3]=std::array::from_fn(|j|(v[0][j]+v[1][j]+v[2][j])/3.);
   let radius2=v.iter().map(|p|(0..3).map(|j|(p[j]-center[j]).powi(2)).sum::<f32>()).fold(0.,f32::max);
   let radius=radius2.sqrt();if !radius.is_finite()||!center.iter().all(|x|x.is_finite()){return Err("terrain bounds overflow".into());}
   let id=0x81000000+index as u32*0x100;let tid=id+0x40;
   let mut data=[0;0x40];
   for (off,values) in [(8,center),(0x18,v[0]),(0x24,v[1]),(0x30,v[2])]{for j in 0..3{data[off+4*j..off+4*j+4].copy_from_slice(&values[j].to_be_bytes());}}
   // Outward rounding keeps broad-phase bounds conservative.
   let padded=radius+0.001;let bound=padded*padded;if !bound.is_finite(){return Err("terrain bounds overflow".into());}data[0x14..0x18].copy_from_slice(&bound.to_be_bytes());
   result.triangles.push(Triangle{identifier:tid,data});
   result.objects.push(WorldObject{identifier:id,flags:0,position:center,angles:[0;3],scale:[1.;3],model_center:[0.;3],model_radius:padded});
   result.entries.push(StaticEntry{flags:0,identifier:id,attached:0});
   let mut node=[0;0x2c];for (off,value) in [(8,id),(0x10,tid),(0x14,tid)]{node[off..off+4].copy_from_slice(&value.to_be_bytes());}
   node[0x28..0x2c].copy_from_slice(&padded.to_be_bytes());result.nodes.push(QueryNode{data:node});
  }
  Ok(result)
 }
 /// A triangle snapshot with explicit source task attachments. Triangle order
 /// determines stable identifiers across snapshots; the host owns task memory.
 /// This does not update geometry or advance platform motion automatically.
 pub fn with_attachments(vertices:&[[[f32;3];3]],attachments:&[(usize,u32)])->Result<Self>{
  let mut terrain=Self::new(vertices)?;
  for &(index,task) in attachments {
   let entry=terrain.entries.get_mut(index).ok_or("attachment triangle index out of range")?;
   if task==0{return Err("attachment requires nonzero task address".into());}
   if entry.attached!=0{return Err("duplicate triangle attachment".into());}
   entry.attached=task;
  }
  // One attached task is one object: separate triangle objects would apply
  // the same platform displacement twice at shared edges.
  let mut groups=std::collections::BTreeMap::<u32,Vec<usize>>::new();
  for &(index,task) in attachments{groups.entry(task).or_default().push(index);}
  let mut removed=std::collections::HashSet::new();
  for indexes in groups.values_mut(){
   if indexes.len()<2{continue;}
   indexes.sort_unstable();let first=indexes[0];
   let center=terrain.objects[first].position;
   let mut radius=0f32;
   for &index in indexes.iter(){for vertex in vertices[index]{let d=(0..3).map(|j|(vertex[j]-center[j]).powi(2)).sum::<f32>().sqrt();radius=radius.max(d);}}
   radius+=0.001;if !radius.is_finite(){return Err("attached terrain bounds overflow".into());}
   terrain.objects[first].model_radius=radius;
   terrain.nodes[first].data[0x28..0x2c].copy_from_slice(&radius.to_be_bytes());
   for &index in indexes.iter().skip(1){removed.insert(terrain.objects[index].identifier);}
   indexes.sort_by(|&a,&b|terrain.objects[a].position[2].total_cmp(&terrain.objects[b].position[2]));
   let ids:Vec<_>=indexes.iter().map(|&i|terrain.triangles[i].identifier).collect();
   for (n,&index) in indexes.iter().enumerate(){
    terrain.triangles[index].data[0..4].copy_from_slice(&ids.get(n+1).copied().unwrap_or(0).to_be_bytes());
    terrain.triangles[index].data[4..8].copy_from_slice(&n.checked_sub(1).map_or(0,|i|ids[i]).to_be_bytes());
   }
   terrain.nodes[first].data[0x10..0x14].copy_from_slice(&ids[0].to_be_bytes());
   terrain.nodes[first].data[0x14..0x18].copy_from_slice(&ids[ids.len()-1].to_be_bytes());
  }
  terrain.objects.retain(|o|!removed.contains(&o.identifier));
  terrain.entries.retain(|e|!removed.contains(&e.identifier));
  terrain.nodes.retain(|n|!removed.contains(&u32::from_be_bytes(n.data[8..12].try_into().unwrap())));
  Ok(terrain)
 }
 pub fn world(&self)->CandidateWorld<'_>{CandidateWorld{objects:&self.objects,static_entries:&self.entries,dynamic_entries:&[],dynamic_enabled:false,triangles:&self.triangles}}
 pub fn state(&self,controller:&GroundMotion)->PositionState{
  let(e,m,c)=controller.source_records();
  PositionState{entity:e.to_vec(),motion:m.to_vec(),character:c.to_vec(),workspace:controller.source_workspace().to_vec(),candidates:CandidateState::new(0),nodes:self.nodes.clone(),sweep:SweepState::new(0),collision_flag:0,control_cache:[0;4],memory:PlatformMemory{base:0x80200000,bytes:vec![0;0x20000]}}
 }
}
