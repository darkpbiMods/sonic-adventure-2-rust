//! Terrain sweep 800cee08..800cfe6c, default debug drawing disabled.
use crate::{disc::Result, original_math::{OriginalMath, IDENTITY}, original_sweep_math::{inverse,rotate_x,rotate_y,transform}};

pub struct TerrainObject { pub identifier:u32, pub triangles:Vec<[u8;0x40]> }
#[derive(Clone)]
pub struct SweepState {
    pub hit_count:u32,
    /// Original globals -77a8, -77ac, -77b0: triangle, endpoint, hit counters.
    pub counters:[u32;3],
    pub zero:[f32;3], pub edge1:[f32;3], pub edge2:[f32;3],
    pub cache:Vec<u8>, pub vertices:Vec<u8>, pub contacts:Vec<u8>,
}
impl SweepState {
    pub fn new(fill:u8)->Self {Self{hit_count:0,counters:[0;3],zero:[0.;3],edge1:[0.;3],edge2:[0.;3],
        cache:vec![fill;128*0x9c],vertices:vec![fill;128*36],contacts:vec![fill;128*0x58]}}
}
fn scalar(b:&[u8],i:usize)->f32{f32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn vector(b:&[u8],i:usize)->[f32;3]{std::array::from_fn(|j|scalar(b,i+j*4))}
fn put_scalar(b:&mut[u8],i:usize,v:f32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn put_vector(b:&mut[u8],i:usize,v:[f32;3]){for j in 0..3{put_scalar(b,i+j*4,v[j]);}}
fn finite(v:[f32;3])->Result<[f32;3]>{if v.into_iter().all(f32::is_finite){Ok(v)}else{Err("nonfinite terrain sweep arithmetic".into())}}
fn add(a:[f32;3],b:[f32;3])->[f32;3]{std::array::from_fn(|i|a[i]+b[i])}
fn sub(a:[f32;3],b:[f32;3])->[f32;3]{std::array::from_fn(|i|a[i]-b[i])}
fn squared(v:[f32;3])->f32{v[2].mul_add(v[2],v[1].mul_add(v[1],v[0]*v[0]))}

fn contains(point:[f32;3],a:[f32;3],b:[f32;3],mode:u32,third:f32)->bool{
    if mode==0 {
        // The original inclusive side-product test uses projected x/z only.
        let s0=(a[2]-0.).mul_add(point[0]-0.,-((a[0]-0.)*(point[2]-0.)));
        let s1=(b[2]-a[2]).mul_add(point[0]-a[0],-((b[0]-a[0])*(point[2]-a[2])));
        let s2=(0.-b[2]).mul_add(point[0]-b[0],-((0.-b[0])*(point[2]-b[2])));
        s0*s1>=0. && s1*s2>=0. && s2*s0>=0.
    }else{
        // Source's alternative test is a union of vertex-centered spheres,
        // not a conventional barycentric or swept-sphere triangle test.
        let center=std::array::from_fn(|i|third*(b[i]+(0.+a[i])));
        let mut radius_squared=squared(sub([0.;3],center));
        for v in [a,b]{let d=squared(sub(v,center));if d>radius_squared{radius_squared=d;}}
        for v in [[0.;3],a,b]{
            let d=sub(v,point);let mut distance=d[0]*d[0];
            if distance>=radius_squared{continue;}
            distance=d[2].mul_add(d[2],distance);if distance>=radius_squared{continue;}
            distance=d[1].mul_add(d[1],distance);if distance<radius_squared{return true;}
        }
        false
    }
}

/// First matching object wins, as in the source linked list. Debug mode is
/// rejected explicitly; it is not silently substituted with renderer stubs.
/// Unsupported arithmetic returns an error without committing partial state.
pub fn sweep(workspace:&mut[u8],objects:&[TerrainObject],identifier:u32,mode:u32,
             debug_draw:bool,state:&mut SweepState,math:&OriginalMath)->Result<bool>{
    if workspace.len()<0x84{return Err("short terrain sweep workspace".into());}
    if debug_draw{return Err("original terrain debug renderer is not integrated".into());}
    if state.cache.len()!=128*0x9c || state.vertices.len()!=128*36 || state.contacts.len()!=128*0x58{
        return Err("invalid terrain sweep storage extent".into());
    }
    let mut w=workspace.to_vec();let mut next=state.clone();
    let hit=sweep_inner(&mut w,objects,identifier,mode,&mut next,math)?;
    workspace.copy_from_slice(&w);*state=next;Ok(hit)
}
fn sweep_inner(w:&mut[u8],objects:&[TerrainObject],identifier:u32,mode:u32,
               state:&mut SweepState,math:&OriginalMath)->Result<bool>{
    state.zero=[0.;3];
    let radius=scalar(w,0);
    if !radius.is_finite(){return Err("nonfinite terrain radius".into());}
    if radius<=0.{return Ok(false);}
    state.hit_count=0;
    let position=finite(vector(w,4))?;let velocity=finite(vector(w,0x10))?;
    let (speed,_)=math.estimated_vector(velocity)?;
    let step=math.constant(0x803b6520-0x6088)*radius;
    if step==0.{return Err("terrain sweep step underflow".into());}
    let Some(object)=objects.iter().find(|o|o.identifier==identifier)else{return Ok(false)};
    let small=math.constant(0x803b6520-0x6080);
    let upper=math.constant(0x803b6520-0x6084);let lower=math.constant(0x803b6520-0x607c);
    let angle_factor=math.double_constant(0x803b6520-0x60a0);
    for triangle in &object.triangles{
        state.counters[0]=state.counters[0].wrapping_add(1);
        let origin=finite(vector(triangle,0x18))?;
        let a=finite(sub(finite(vector(triangle,0x24))?,origin))?;
        let b=finite(sub(finite(vector(triangle,0x30))?,origin))?;
        let relative=finite(sub(position,origin))?;
        let f7=a[1]-0.;let f6=a[2]-0.;let f3=0.-b[2];let f4=0.-b[1];let f5=0.-b[0];let f8=a[0]-0.;
        let normal=[f4.mul_add(f6,-(f3*f7)), f3.mul_add(f8,-(f5*f6)), f5.mul_add(f7,-(f4*f8))];
        let (_,unit)=math.estimated_vector(normal)?;let normal=unit.unwrap_or([0.,1.,0.]);
        let [x,y,z]=normal;
        let (pitch,yaw)=if y>upper || (z.abs()<small && x.abs()<small && y>0.){(0,0)}
        else if y<lower || (z.abs()<small && x.abs()<small && y<0.){(-0x8000,-0x8000)}
        else{(((angle_factor*(math.acos(y)as f64))as i32).wrapping_neg(),((angle_factor*(math.atan2(x,z)as f64))as i32).wrapping_neg())};
        let mut matrix=IDENTITY;
        rotate_x(&mut matrix,pitch as u32,math)?;rotate_y(&mut matrix,yaw as u32,math)?;
        state.edge1=finite(transform(&matrix,a))?;state.edge2=finite(transform(&matrix,b))?;
        let local_position=finite(transform(&matrix,relative))?;
        let local_velocity=finite(transform(&matrix,velocity))?;
        let flags=u32::from_be_bytes(triangle[0x3c..0x40].try_into().unwrap());
        let two_sided=flags&0x00800000!=0;
        let signed_radius=if two_sided && local_position[1]<0.{radius*math.constant(0x803b6520-0x6078)}else{radius};
        let (_,unit)=math.estimated_vector(local_velocity)?;
        let delta=unit.unwrap_or([0.;3]).map(|v|v*step);
        let mut point=local_position;let mut progress=0_f32;
        let hit=loop{
            let endpoint=(progress as f64)>=speed;
            if endpoint{point=finite(add(local_position,local_velocity))?;state.counters[1]=state.counters[1].wrapping_add(1);}
            if point[1].abs()<=radius && contains(point,state.edge1,state.edge2,mode,math.constant(0x803b6520-0x6074)){
                state.counters[2]=state.counters[2].wrapping_add(1);break true;
            }
            if endpoint{break false;}
            let next=progress+step;
            if !next.is_finite() || next==progress{return Err("nonprogressing terrain sweep arithmetic".into());}
            progress=next;point=finite(add(point,delta))?;
        };
        if !hit{continue;}
        let index=state.hit_count as usize;
        state.vertices[index*36..index*36+36].copy_from_slice(&triangle[0x18..0x3c]);
        let corrected=if two_sided && local_position[1]<0.{normal.map(|v|-v)}else{normal};
        let angles=math.surface_angles(corrected)?;
        let matrix=inverse(&matrix)?.unwrap_or(matrix);
        let contact=finite(add(transform(&matrix,[local_position[0],0.,local_position[2]]),origin))?;
        let center=finite(add(transform(&matrix,[local_position[0],signed_radius,local_position[2]]),origin))?;
        let parallel=finite(transform(&matrix,[local_velocity[0],0.,local_velocity[2]]))?;
        let perpendicular=finite(transform(&matrix,[0.,local_velocity[1],0.]))?;
        let mut record=[0_u8;0x58];
        record[..4].copy_from_slice(&angles[0].to_be_bytes());record[4..8].copy_from_slice(&angles[1].to_be_bytes());
        for (offset,v) in [(8,normal),(0x14,corrected),(0x20,contact),(0x2c,center),(0x38,parallel),(0x44,perpendicular)]{put_vector(&mut record,offset,v);}
        put_scalar(&mut record,0x50,local_position[1]);put_scalar(&mut record,0x54,local_position[1]+local_velocity[1]);
        let base=index*0x9c;
        state.cache[base+4..base+0x44].copy_from_slice(triangle);
        state.cache[base+0x44..base+0x9c].copy_from_slice(&record);
        state.hit_count+=1;if state.hit_count==128{break;}
    }
    let count=state.hit_count as usize;
    if count==0{return Ok(false);}
    let mut selected=0;let mut distance=scalar(&state.cache,0x98);
    for i in 0..count{
        let end=scalar(&state.cache,i*0x9c+0x98);
        if end<distance{distance=end;selected=i;}
        state.contacts[i*0x58..(i+1)*0x58].copy_from_slice(&state.cache[i*0x9c+0x44..(i+1)*0x9c]);
    }
    w[0x28..0x2a].copy_from_slice(&(count as u16).to_be_bytes());w[0x2a..0x2c].copy_from_slice(&(selected as u16).to_be_bytes());
    let chosen=&state.contacts[selected*0x58..(selected+1)*0x58];
    w[0x34..0x54].copy_from_slice(&chosen[..0x20]);w[0x6c..0x84].copy_from_slice(&chosen[0x38..0x50]);
    let mut contact=vector(&state.contacts,0x20);let mut center=vector(&state.contacts,0x2c);
    if count>1{
        for i in 1..count{contact=add(contact,vector(&state.contacts,i*0x58+0x20));center=add(center,vector(&state.contacts,i*0x58+0x2c));}
        let inv=1./count as f32;contact=contact.map(|v|v*inv);center=center.map(|v|v*inv);
    }
    put_vector(w,0x54,finite(contact)?);put_vector(w,0x60,finite(center)?);Ok(true)
}
