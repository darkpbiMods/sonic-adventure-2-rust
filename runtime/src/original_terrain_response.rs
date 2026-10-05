//! Selected-contact responses 80040acc..80040f48 / 80040f48..800414f0.
use crate::{disc::Result,original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES,angle_distance},
    original_math::OriginalMath,original_geometry::{signed_angle,intersect_planes},
    original_terrain_contacts::distance,original_angle_table};
pub struct ResponseContext {
    pub gravity:[f32;3],pub surface_flags:u32,
    /// Original routine 80040acc leaves the motion pointer in r3 after its
    /// final redirection. Callers inspect that return, so retain source identity.
    pub motion_identity:u32,
}
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn vector(b:&[u8],i:usize)->[f32;3]{std::array::from_fn(|j|float(b,i+j*4))}
fn put_word(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn put_vector(b:&mut[u8],i:usize,v:[f32;3]){for j in 0..3{put_word(b,i+j*4,v[j].to_bits());}}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],b[0]*a[0]))}
fn checked(v:[f32;3])->Result<[f32;3]>{if v.into_iter().all(f32::is_finite){Ok(v)}else{Err("nonfinite terrain response vector".into())}}
fn cross(a:[f32;3],b:[f32;3])->[f32;3]{[b[2].mul_add(a[1],-(b[1]*a[2])),b[0].mul_add(a[2],-(b[2]*a[0])),b[1].mul_add(a[0],-(b[0]*a[1]))]}
fn plane(p:[f32;3],n:[f32;3])->[f32;6]{[p[0],p[1],p[2],n[0],n[1],n[2]]}

pub fn respond(alternate:bool,entity:&mut[u8],motion:&mut[u8],character:&mut[u8],workspace:&mut[u8],
               records:&[u8],ctx:&ResponseContext,math:&OriginalMath)->Result<u32>{
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES || motion.len()<0x20 || workspace.len()<0x84{return Err("invalid terrain response state extent".into());}
    let count=u16::from_be_bytes(workspace[0x28..0x2a].try_into().unwrap())as usize;
    if count>128 || records.len()<count*0x58{return Err("invalid terrain response contact extent".into());}
    checked(ctx.gravity)?;checked(vector(workspace,0x1c))?;checked(vector(workspace,0x10))?;
    for off in [0x134,0x64,0xe4,0xf0]{if !float(character,off).is_finite(){return Err("nonfinite response character input".into());}}
    let mut e=entity.to_vec();let mut m=motion.to_vec();let mut c=character.to_vec();let mut w=workspace.to_vec();
    let result=respond_inner(alternate,&mut e,&mut m,&mut c,&mut w,&records[..count*0x58],ctx,math)?;
    entity.copy_from_slice(&e);motion.copy_from_slice(&m);character.copy_from_slice(&c);workspace.copy_from_slice(&w);Ok(result)
}
fn respond_inner(alt:bool,e:&mut[u8],m:&mut[u8],c:&mut[u8],w:&mut[u8],records:&[u8],ctx:&ResponseContext,math:&OriginalMath)->Result<u32>{
    let k=|off:u32|math.constant(0x803b6520-off);
    let mut nearest=k(0x7554);let mut selected=None;let mut count=0;
    for (i,record)in records.chunks_exact(0x58).enumerate(){
        let d=distance(vector(record,0x2c),vector(w,4),math)?;
        let n=checked(vector(record,0x14))?;let ground=vector(w,0x1c);
        let angle=signed_angle(ground,n,None,math)?;
        if angle<=0x2c00 || if alt{dot(ctx.gravity,ground)>dot(ctx.gravity,n) || ctx.surface_flags&0x100!=0}else{ground[1]<n[1]}{continue;}
        let velocity=vector(w,0x10);let (_,unit)=math.estimated_vector(velocity)?;
        let incidence=dot(unit.unwrap_or(velocity),n);
        if (!alt && angle>=0x7000) || incidence>=k(0x75f8){continue;}
        let raw=dot(velocity,n);let cosine=original_angle_table::entry(0x600,math)?[1];
        if incidence < -cosine || raw<k(0x7508){
            count+=1;if nearest>d{nearest=d;selected=Some(i);}
        }
    }
    if count==0{return Ok(0);}
    let (speed,_)=math.estimated_vector(vector(w,0x10))?;
    let short=(c[1]==0 && float(c,0x134)<math.constant(0x80196578+0x74)) || (c[1]==1 && float(c,0x134)<math.constant(0x80196578+0xf8));
    if short && ctx.surface_flags&0x08000000==0{put_vector(w,0x6c,[k(0x75f8);3]);put_vector(m,0,[k(0x75f8);3]);return Ok(0);}
    let index=selected.ok_or("terrain response outside original distance sentinel domain")?;
    if nearest as f64>=speed || word(c,0x8c)&0x100!=0{return Ok(0);}
    let parallel=checked(vector(w,0x6c))?;let (length,unit)=math.estimated_vector(parallel)?;
    if length==0.{put_vector(w,0x6c,[k(0x75f8);3]);return Ok(0);}
    let adjusted=if alt{unit.ok_or("undefined parallel normalization")?.map(|x|x*nearest)}
        else{let perpendicular=checked(vector(w,0x78))?;std::array::from_fn(|i|parallel[i]+perpendicular[i])};
    put_vector(w,0x6c,checked(adjusted)?);
    let record=&records[index*0x58..(index+1)*0x58];let normal=vector(record,0x14);put_vector(c,0x70,normal);
    let opposite=checked(vector(m,0))?.map(|x|-x);let (length,unit)=math.estimated_vector(opposite)?;
    if length==0.{return Ok(0);}
    if alt && signed_angle(unit.ok_or("undefined motion normalization")?,normal,None,math)?<=0x1000{
        let local_speed=float(c,0x64);
        if local_speed>=float(c,0xf0){put_vector(w,0x6c,[k(0x75f8);3]);return Ok(2);}
        if local_speed>float(c,0xe4){
            let target=k(0x7590)*float(c,0xe4);
            let (_,unit)=math.estimated_vector(vector(m,0))?;
            put_vector(m,0,checked(unit.ok_or("undefined head-on motion")?.map(|x|x*target))?);
        }
        return Ok(1);
    }
    let cross=cross(vector(w,0x6c),normal);let (_,unit)=math.estimated_vector(cross)?;
    let perpendicular=unit.unwrap_or(vector(w,0x1c));
    let line=intersect_planes(plane(vector(record,0x20),normal),plane(checked(vector(w,0x60))?,perpendicular),true,math)?.1.ok_or("missing response intersection")?;
    let mut direction=[line[3],line[4],line[5]];
    let parallel=vector(w,0x6c);let (_,unit)=math.estimated_vector(parallel)?;
    if dot(direction,unit.unwrap_or(parallel))<k(0x75f8){direction=direction.map(|x|-x);}
    let (speed,_)=math.estimated_vector(vector(m,0))?;
    let redirected=checked(direction.map(|x|((x as f64)*speed)as f32))?;
    put_vector(m,0,redirected);put_vector(w,0x6c,redirected);
    if !alt{return Ok(ctx.motion_identity);}
    let yaw=((math.double_constant(0x803b6520-0x75e8)*(math.atan2(direction[2],direction[0])as f64))as i32 as u32)&0xffff;
    let difference=angle_distance(word(m,0x1c),yaw);
    let yaw=if difference>0x6000{yaw.wrapping_add(0x8000)}else{yaw};
    put_word(e,0xc,yaw);put_word(m,0x1c,yaw);Ok(difference)
}
