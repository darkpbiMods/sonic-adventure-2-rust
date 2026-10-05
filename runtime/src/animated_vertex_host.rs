//! Host composition of recovered animation hierarchy and vertex processing.
//! Each snapshot is taken when that mesh is drawn: later meshes can overwrite
//! the shared weighted-vertex buffer, so the final buffer alone is insufficient.
use crate::{disc::Result, original_contact::Memory, original_math::OriginalMath,
    original_animation_context::initialize, original_animation_euler::table,
    original_animation_hierarchy::{Callbacks,walk}, original_sonic_node,
    original_mesh_vertices};

pub struct MeshSnapshot { pub mesh: u32, pub vertices: Vec<u8> }
pub struct Output { pub context: [u32;16], pub current: u32, pub flags: u32, pub meshes: Vec<MeshSnapshot> }
pub struct Input {
    pub base: u32, pub root: u32, pub motion: u32, pub frame: f32,
    pub models: u32, pub character: u32, pub scratch: u32, pub players: u32,
    pub stack: u32, pub vertices: u32, pub vertex_bytes: usize,
}
struct Observer<'a> { math: &'a OriginalMath, input: &'a Input, meshes: Vec<MeshSnapshot>, flags: u32 }
impl Callbacks for Observer<'_> {
    fn node(&mut self,base:u32,m:&mut[u8],node:u32,current:u32)->Result<()> {
        let i=self.input;
        original_sonic_node::apply_in_place(base,m,node,&original_sonic_node::Context{
            models:i.models,character:i.character,scratch:i.scratch,players:i.players,matrix:current
        },self.math)
    }
    fn mesh(&mut self,base:u32,m:&mut[u8],mesh:u32,current:u32)->Result<()> {
        original_mesh_vertices::apply_in_place(base,m,mesh,current,self.input.vertices,0.,&mut self.flags)?;
        let start=self.input.vertices.checked_sub(base).ok_or("vertex buffer below mapping")? as usize;
        let end=start.checked_add(self.input.vertex_bytes).ok_or("vertex buffer extent overflow")?;
        let vertices=m.get(start..end).ok_or("vertex buffer outside mapping")?.to_vec();
        self.meshes.push(MeshSnapshot{mesh,vertices}); Ok(())
    }
}
/// Mapped source assets and initialized character/matrix workspace are supplied
/// by the host. Changes commit only after the complete bounded traversal succeeds.
/// Normal-w and initial stream flags are zero, matching the verified Sonic path.
pub fn sample(memory:&mut[u8], input:&Input, math:&OriginalMath)->Result<Output> {
    if !input.frame.is_finite() || input.vertex_bytes==0 || input.vertex_bytes%32!=0 {
        return Err("Invalid animated vertex sample request".into());
    }
    let start=input.vertices.checked_sub(input.base).ok_or("vertex buffer below mapping")? as usize;
    let end=start.checked_add(input.vertex_bytes).ok_or("vertex buffer extent overflow")?;
    memory.get(start..end).ok_or("vertex buffer outside mapping")?;
    let mut staged=memory.to_vec();
    let mut context=[0;16];
    initialize(&mut context,&Memory{base:input.base,bytes:&staged},input.motion,input.frame,math)?;
    let mut observer=Observer{math,input,meshes:vec![],flags:0};
    let mut current=input.stack;
    walk(input.base,&mut staged,input.root,&mut context,input.stack,&mut current,&table(math)?,math,&mut observer,4096)?;
    memory.copy_from_slice(&staged);
    Ok(Output{context,current,flags:observer.flags,meshes:observer.meshes})
}
