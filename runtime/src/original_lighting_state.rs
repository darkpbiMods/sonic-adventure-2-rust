//! Lighting front end8011caac, preserving explicit external callback dispatch.
/// Commands are [callee address, arguments...]. An external callback command
/// must be resolved by the host; it is never silently discarded here.
pub fn calls(render_flags:u32,callback:u32,header:u32,vertex_flags:u32,light_mask:u32,diffuse:u32,attenuation:u32)->Vec<Vec<u32>>{
 if render_flags&0x80000!=0 && callback!=0{return vec![vec![callback,0]];}
 let enable=u32::from(vertex_flags&1==0 && header&1==0);let material=u32::from(vertex_flags&2!=0);
 let mut out=vec![vec![0x80102f44,0,enable,0,material,light_mask,diffuse,attenuation],vec![0x80102f44,2,0,0,material,light_mask,diffuse,attenuation]];
 if header&2==0{out.push(vec![0x80102ef8,2]);out.push(vec![0x80102f44,1,1,0,material,light_mask,0,0]);out.push(vec![0x80102f44,3,0,0,material,light_mask,0,0]);}
 else{out.push(vec![0x80102f44,1,0,0,0,light_mask,0,2]);}out
}
