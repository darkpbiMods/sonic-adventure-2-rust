use serde::Deserialize;
use sonic_playground::draw_packets::{Primitive,assemble};
use sa2_runtime::{animated_vertex_host::{Input,sample},original_math::OriginalMath,original_contact::Memory,original_primitives};
#[derive(Deserialize)]struct Case {base:u32,root:u32,motion:u32,time:u32,models:u32,memory:Vec<u8>}
#[derive(Deserialize)]struct Inventory {rigs:Vec<Plan>}
#[derive(Deserialize)]struct Plan {pose_index:usize,primitives:Vec<Command>}
#[derive(Deserialize)]struct Command {#[serde(flatten)] primitive:Primitive,data:u32,count_flags:u32}
fn main(){
 let args:Vec<_>=std::env::args().collect();
 let math=OriginalMath::from_dol(std::fs::read(&args[1]).unwrap()).unwrap();
 let cases:Vec<Case>=serde_json::from_slice(&std::fs::read(&args[2]).unwrap()).unwrap();
 let inventory:Inventory=serde_json::from_slice(&std::fs::read(&args[3]).unwrap()).unwrap();
 assert_eq!(cases.len(),15);let mut vertices=0;let mut packets=0;
 for (index,mut case) in cases.into_iter().enumerate(){
  let work=case.base+0x200000;
  let plan=inventory.rigs.iter().find(|r|r.pose_index==if index<12{0}else{12}).unwrap();
  let output=sample(&mut case.memory,&Input{base:case.base,root:case.root,motion:case.motion,frame:f32::from_bits(case.time),models:case.models,character:work+0x100,scratch:work+0x1000,players:work+0x1800,stack:work+0x4000,vertices:work+0x10000,vertex_bytes:0x4000},&math).unwrap();
  for command in &plan.primitives {
   let p=&command.primitive;
   let host=assemble(std::slice::from_ref(p),&output.meshes).unwrap();let packet=&host[0];
   case.memory[0x210000..0x214000].copy_from_slice(&output.meshes[p.mesh_visit].vertices);
   let memory=Memory{base:case.base,bytes:&case.memory};
   let emitted=if packet.generated_uv {sa2_runtime::original_special_primitives::emit(&memory,command.data,work+0x10000,command.count_flags,p.kind==0x41).unwrap()}else{original_primitives::emit(&memory,command.data,work+0x10000,command.count_flags,p.kind==0x41,false).unwrap()};
   let bytes=&emitted.fifo;let mut cursor=0;let mut vertex_index=0;
   while cursor<bytes.len(){
    assert_eq!(bytes[cursor],0x98);let count=u16::from_be_bytes(bytes[cursor+1..cursor+3].try_into().unwrap())as usize;cursor+=3;
    for _ in 0..count {
     let v=&packet.vertices[vertex_index];vertex_index+=1;
     for value in v.position.iter().chain(v.normal.iter()) {
      assert_eq!(value.to_bits(),u32::from_be_bytes(bytes[cursor..cursor+4].try_into().unwrap()));cursor+=4;
     }
     if p.kind==0x41 && !packet.generated_uv {for value in v.uv {let raw=i16::from_be_bytes(bytes[cursor..cursor+2].try_into().unwrap());cursor+=2;assert_eq!(value.to_bits(),(raw as f32/256.).to_bits());}}
    }
   }
   assert_eq!(cursor,bytes.len());assert_eq!(vertex_index,packet.vertices.len());
   vertices+=vertex_index;packets+=1;
  }
 }
 assert_eq!(packets,12*19+3);
 println!("PASS real draw packets: {packets} packets, {vertices} vertex attributes compared");
}
