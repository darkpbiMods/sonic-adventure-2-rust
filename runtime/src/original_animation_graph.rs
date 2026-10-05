//! Model order8002b6c0/8002b54c and angle continuity8002b890..8002b8e0.
use crate::{disc::Result,original_contact::Memory,original_gameplay::angle_delta};
/// Separate output storage matches the original animation caller's stack array.
/// The recursive original is preorder (node, child, sibling). Shared nodes are
/// emitted again, as in the source; capacity guards cycles/oversized graphs.
pub fn enumerate(root:u32,memory:&Memory,output:&mut[u32])->Result<usize>{
 let mut pending=vec![root];let mut result=output.to_vec();let mut count=0;
 while let Some(node)=pending.pop(){
  if count>=result.len(){return Err("model traversal exceeds caller output capacity".into());}
  result[count]=node;count+=1;
  let child=memory.word(node.wrapping_add(0x2c))?;let sibling=memory.word(node.wrapping_add(0x30))?;
  if sibling!=0{pending.push(sibling);}if child!=0{pending.push(child);}
 }
 output.copy_from_slice(&result);Ok(count)
}
/// Preserve the original signed32-bit representation, not just its low16 bits.
/// Compute the terminating loop result directly. If no congruent signed value
/// can meet the source comparison, the original loop never terminates; reject.
pub fn continuous_angle(from:u32,to:u32)->Result<u32>{
 let delta=angle_delta(from,to);let from=from as i32 as i64;let to=to as i32 as i64;
 let result=if delta>0&&to<from{to+((from-to+65535)/65536)*65536}
 else if delta<0&&to>from{to-((to-from+65535)/65536)*65536}
 else if delta==0{from}else{to};
 if !(i32::MIN as i64..=i32::MAX as i64).contains(&result){return Err("original angle continuity would not terminate".into());}
 Ok(result as i32 as u32)
}
