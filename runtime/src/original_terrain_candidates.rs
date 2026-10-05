//! Object broadphase 80023808 and triangle candidate query 800d1020.
use crate::{disc::Result,original_math::{OriginalMath,IDENTITY},original_sweep_math::transform,
    original_terrain_sweep::TerrainObject};
pub const POINTER_BASE:u32=0x8028f998;
pub struct WorldObject {
    pub identifier:u32,pub flags:u32,pub position:[f32;3],pub angles:[u32;3],pub scale:[f32;3],
    pub model_center:[f32;3],pub model_radius:f32,
}
pub struct StaticEntry {pub flags:u32,pub identifier:u32,pub attached:u32}
pub struct DynamicEntry {pub position:[f32;3],pub radius:f32,pub identifier:u32,pub flags:u32}
pub struct Triangle {pub identifier:u32,pub data:[u8;0x40]}
#[derive(Clone)]pub struct QueryNode {pub data:[u8;0x2c]}
#[derive(Clone)]pub struct CandidateState {pub object_count:u16,pub objects:Vec<u8>,pub triangle_count:u32,pub pointers:Vec<u8>}
impl CandidateState{pub fn new(fill:u8)->Self{Self{object_count:0,objects:vec![fill;1024*12],triangle_count:0,pointers:vec![fill;192*4]}}}
pub struct CandidateWorld<'a>{pub objects:&'a[WorldObject],pub static_entries:&'a[StaticEntry],pub dynamic_entries:&'a[DynamicEntry],pub dynamic_enabled:bool,pub triangles:&'a[Triangle]}
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn vector(b:&[u8],i:usize)->[f32;3]{std::array::from_fn(|j|float(b,i+j*4))}
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn finite(v:[f32;3])->Result<[f32;3]>{if v.into_iter().all(f32::is_finite){Ok(v)}else{Err("nonfinite terrain candidate vector".into())}}
fn scalar(v:f32)->Result<f32>{if v.is_finite(){Ok(v)}else{Err("nonfinite terrain candidate arithmetic".into())}}
fn append_object(state:&mut CandidateState,flags:u32,id:u32,attached:u32){
    let i=state.object_count as usize*12;put(&mut state.objects,i,flags);put(&mut state.objects,i+4,id);put(&mut state.objects,i+8,attached);state.object_count+=1;
}
fn dynamic_sqrt(x:f32,math:&OriginalMath)->f32{
    if x<=0.{return x;}
    let mut inv=crate::gekko_estimate::reciprocal_sqrt_seed(x as f64);
    let half=math.double_constant(0x803b6520-0x7ac0);let three=math.double_constant(0x803b6520-0x7ab8);
    for _ in 0..3{let square=inv*inv;inv*=half;inv*=-( (x as f64).mul_add(square,-three));}
    ((x as f64)*inv)as f32
}
fn object_collection(position:[f32;3],radius:f32,world:&CandidateWorld,state:&mut CandidateState,math:&OriginalMath)->Result<()> {
    state.object_count=0;let position=finite(position)?;let radius=scalar(radius)?;
    let padding=math.constant(0x803b6520-0x7ab0);
    for entry in world.static_entries.iter().rev(){
        let object=world.objects.iter().find(|o|o.identifier==entry.identifier).ok_or("missing static terrain object")?;
        let origin=finite(object.position)?;let mut center=finite(object.model_center)?;
        let ignore_y=entry.flags&0x2002!=0;
        if !ignore_y && entry.flags&0x10000000!=0{
            let mut matrix=IDENTITY;
            let [x,y,z]=object.angles;
            let rotations=if object.flags&0x20!=0{[(b'y',y),(b'x',x),(b'z',z)]}else{[(b'z',z),(b'y',y),(b'x',x)]};
            for (axis,angle)in rotations{matrix=math.rotate(&matrix,axis,angle as i32)?;}
            // 800edab8 uses the same paired affine lane order as 80115df4.
            center=finite(transform(&matrix,center))?;
        }
        let delta=finite(std::array::from_fn(|i|if ignore_y && i==1{math.constant(0x803b6520-0x7ad0)}else{position[i]-(origin[i]+center[i])}))?;
        let square=scalar(delta[2].mul_add(delta[2],delta[0].mul_add(delta[0],delta[1]*delta[1])))?;
        let model_radius=scalar(object.model_radius)?;
        let extent=if object.flags&4!=0{padding+(model_radius+radius)}else{
            let scale=finite(object.scale)?;let mut chosen=scale[0];if chosen>scale[1]{chosen=scale[1];}if chosen>scale[2]{chosen=scale[2];}
            padding+model_radius.mul_add(chosen,radius)
        };
        let limit=scalar(extent*extent)?;
        if square<limit{
            append_object(state,entry.flags,entry.identifier,entry.attached);
            if state.object_count==1024{return Ok(());}
        }
    }
    if world.dynamic_enabled{
        if world.dynamic_entries.len()>i16::MAX as usize{return Err("dynamic terrain count outside source signed-short domain".into());}
        for entry in world.dynamic_entries{
            if entry.flags&0x2003==0{continue;}
            let p=finite(entry.position)?;let x=p[0]-position[0];let z=p[2]-position[2];
            let y=if entry.flags&0x2002!=0{math.constant(0x803b6520-0x7ad0)}else{let d=p[1]-position[1];d*d};
            let squared=scalar(z*z+(x*x+y))?;
            let d=dynamic_sqrt(squared,math)-radius;
            if d<scalar(entry.radius)?{
                append_object(state,entry.flags,entry.identifier,0);
                if state.object_count==1024{return Ok(());}
            }
        }
    }
    Ok(())
}
pub fn collect_objects(position:[f32;3],radius:f32,world:&CandidateWorld,state:&mut CandidateState,math:&OriginalMath)->Result<()> {
    if state.objects.len()!=1024*12{return Err("invalid object candidate storage".into());}
    let mut next=state.clone();object_collection(position,radius,world,&mut next,math)?;*state=next;Ok(())
}
fn triangle<'a>(world:&'a CandidateWorld,id:u32)->Result<&'a Triangle>{world.triangles.iter().find(|t|t.identifier==id).ok_or_else(||format!("unresolved terrain triangle {id:08x}"))}
fn centroid_z(world:&CandidateWorld,id:u32)->Result<f32>{scalar(float(&triangle(world,id)?.data,0x10))}
fn overlaps(t:&Triangle,p:[f32;3],r2:f32)->Result<bool>{
    let center=finite(vector(&t.data,8))?;let limit=scalar(r2+float(&t.data,0x14))?;
    let x=center[0]-p[0];let mut d=scalar(x*x)?;if d>limit{return Ok(false);}
    let y=center[1]-p[1];d=scalar(y.mul_add(y,d))?;if d>limit{return Ok(false);}
    let z=center[2]-p[2];d=scalar(z.mul_add(z,d))?;Ok(d<=limit)
}
fn append_triangle(node:&mut QueryNode,id:u32,state:&mut CandidateState){
    let i=state.triangle_count as usize;
    if word(&node.data,0x1c)==0{put(&mut node.data,0x1c,POINTER_BASE+i as u32*4);}
    let count=word(&node.data,0x24).wrapping_add(1);put(&mut node.data,0x24,count);
    put(&mut state.pointers,i*4,id);state.triangle_count+=1;
}
/// Nodes are in original linked-list order. Their raw output fields preserve
/// source identities, including nodes left untouched after capacity is reached.
pub fn collect_triangles(workspace:&[u8],world:&CandidateWorld,nodes:&mut[QueryNode],state:&mut CandidateState,
                         debug_draw:bool,math:&OriginalMath)->Result<u32>{
    if debug_draw{return Err("terrain candidate debug text is not integrated".into());}
    if workspace.len()<0x1c || state.objects.len()!=1024*12 || state.pointers.len()!=192*4{return Err("invalid terrain candidate state extent".into());}
    let mut next=state.clone();let mut next_nodes=nodes.to_vec();next.triangle_count=0;
    let radius=scalar(float(workspace,0))?;let p=finite(vector(workspace,4))?;
    let (speed,_)=math.estimated_vector(finite(vector(workspace,0x10))?)?;
    let expanded=scalar(((radius as f64)+speed)as f32)?;let square=scalar(expanded*expanded)?;
    object_collection(p,radius,world,&mut next,math)?;
    'objects:for node in &mut next_nodes{
        put(&mut node.data,0x1c,0);put(&mut node.data,0x24,0);
        let id=word(&node.data,8);
        if !(0..next.object_count as usize).any(|i|word(&next.objects,i*12+4)==id){continue;}
        let padding=scalar(float(&node.data,0x28))?;
        let lower=scalar((p[2]-padding)-expanded)?;let upper=scalar(expanded+(p[2]+padding))?;
        let mut current=word(&node.data,0x10);let tail=word(&node.data,0x14);
        let first_z=centroid_z(world,current)?;
        if first_z<lower && centroid_z(world,tail)?>upper{
            let mut visited=std::collections::HashSet::new();
            while current!=0 && centroid_z(world,current)?<=lower{
                if !visited.insert(current){return Err("cyclic forward terrain list".into());}
                current=word(&triangle(world,current)?.data,0);
            }
        }
        let forward=current!=0 && centroid_z(world,current)?>lower;
        if !forward{current=tail;}
        let mut visited=std::collections::HashSet::new();
        while current!=0{
            let z=centroid_z(world,current)?;
            if if forward{z>=upper}else{z<=lower}{break;}
            if !visited.insert(current){return Err("cyclic terrain query list".into());}
            let t=triangle(world,current)?;
            if overlaps(t,p,square)?{
                append_triangle(node,current,&mut next);
                if next.triangle_count==192{break 'objects;}
            }
            current=word(&t.data,if forward{0}else{4});
        }
    }
    nodes.clone_from_slice(&next_nodes);*state=next;Ok(state.triangle_count)
}
/// Bridge the query's real pointer-list results into the already translated
/// sweep, preserving order and every original triangle byte.
pub fn sweep_objects(nodes:&[QueryNode],world:&CandidateWorld,state:&CandidateState)->Result<Vec<TerrainObject>>{
    nodes.iter().map(|node|{
        let count=word(&node.data,0x24)as usize;let start=word(&node.data,0x1c);
        if count==0{return Ok(TerrainObject{identifier:word(&node.data,8),triangles:Vec::new()});}
        let offset=start.checked_sub(POINTER_BASE).ok_or("invalid query pointer base")?as usize;
        if offset%4!=0 || count>192 || offset+count*4>state.pointers.len(){return Err("invalid query pointer extent".into());}
        let triangles=(0..count).map(|i|triangle(world,word(&state.pointers,offset+i*4)).map(|t|t.data)).collect::<Result<Vec<_>>>()?;
        Ok(TerrainObject{identifier:word(&node.data,8),triangles})
    }).collect()
}
