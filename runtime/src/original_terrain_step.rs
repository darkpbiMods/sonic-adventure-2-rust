//! Complete 80043600..80043ed4 terrain step, including real terrain sweep.
use crate::{disc::Result, original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES},
    original_math::{OriginalMath,IDENTITY,transform}, original_offset,
    original_geometry::{signed_angle,point_plane}, original_terrain_contacts::qualifying_contact,
    original_terrain_sweep::{sweep,SweepState,TerrainObject}};

pub struct StepContext {
    pub gravity:[f32;3],
    /// Global r13-78f8. Only zero/nonzero is significant here.
    pub special_mode:bool,
    pub special_offset:f32, pub special_radius:f32,
    pub surface_flags:u32, pub object_identifier:u32,
}
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn vector(b:&[u8],i:usize)->[f32;3]{std::array::from_fn(|j|float(b,i+j*4))}
fn put(b:&mut[u8],i:usize,v:f32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn put_vector(b:&mut[u8],i:usize,v:[f32;3]){for j in 0..3{put(b,i+j*4,v[j]);}}
fn add(a:[f32;3],b:[f32;3])->[f32;3]{std::array::from_fn(|i|a[i]+b[i])}
fn sub(a:[f32;3],b:[f32;3])->[f32;3]{std::array::from_fn(|i|a[i]-b[i])}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],b[0]*a[0]))}
fn orient(v:[f32;3],pitch:u32,roll:u32,yaw:u32,math:&OriginalMath)->Result<[f32;3]>{
    let m=math.rotate(&IDENTITY,b'z',roll as i32)?;
    let m=math.rotate(&m,b'x',pitch as i32)?;
    let m=math.rotate(&m,b'y',(yaw as i32).wrapping_neg())?;
    Ok(transform(&m,v))
}
fn pole(w:&mut[u8],e:&[u8],math:&OriginalMath){
    if float(w,0x50).abs()>=math.constant(0x803b6520-0x74d8){
        for off in [0x40,0x3c,0x4c,0x48]{put(w,off,math.constant(0x803b6520-0x75f8));}
        w[0x38..0x3c].copy_from_slice(&e[0x10..0x14]);
    }
}
/// Entity/motion/character are the selected player's source records. Errors
/// preserve caller state; a normal false return preserves the original query
/// side effects while restoring the source workspace snapshot.
pub fn step(entity:&[u8],motion:&[u8],character:&[u8],workspace:&mut[u8],
            objects:&[TerrainObject],state:&mut SweepState,context:&StepContext,math:&OriginalMath)->Result<bool>{
    if entity.len()!=ENTITY_BYTES || motion.len()<12 || character.len()!=CHARACTER_BYTES || workspace.len()!=0x84{
        return Err("invalid terrain step state extent".into());
    }
    let mut w=workspace.to_vec();let mut next=state.clone();
    let result=step_inner(entity,motion,character,&mut w,objects,&mut next,context,math)?;
    for off in [0,4,8,12,0x10,0x14,0x18,0x3c,0x40,0x44,0x48,0x4c,0x50,0x54,0x58,0x5c,0x60,0x64,0x68,0x6c,0x70,0x74,0x78,0x7c,0x80]{
        if !float(&w,off).is_finite(){return Err("nonfinite terrain step result".into());}
    }
    workspace.copy_from_slice(&w);*state=next;Ok(result)
}
fn step_inner(e:&[u8],m:&[u8],c:&[u8],w:&mut[u8],objects:&[TerrainObject],state:&mut SweepState,ctx:&StepContext,math:&OriginalMath)->Result<bool>{
    let k=|off:u32|math.constant(0x803b6520-off);
    pole(w,e,math);
    let prior_qualifies=qualifying_contact(w,&state.contacts,ctx.gravity,math)?;
    let snapshot=w.to_vec();
    let player_eight=ctx.special_mode && c[0]==8;
    let status=u16::from_be_bytes(e[4..6].try_into().unwrap());
    let normal_coordinates=ctx.special_mode && status&0x2000==0;
    let mut offset=[k(0x75f8),if player_eight{ctx.special_offset}else{k(0x74dc)},k(0x75f8)];
    original_offset::update(m,c,vector(w,0x1c),ctx.gravity,&mut offset,math)?;
    let [pitch,roll]=if normal_coordinates{math.surface_angles(vector(w,0x48))?}else{[word(e,8),word(e,0x10)]};
    let yaw=word(e,0xc);
    offset=orient(offset,pitch,roll,yaw,math)?;
    let radius_vector=orient([k(0x75f8),if player_eight{ctx.special_radius}else{float(w,0)},k(0x75f8)],pitch,roll,yaw,math)?;
    let surface_flag=ctx.surface_flags&0x100!=0;
    if !normal_coordinates{
        put_vector(w,4,add(offset,add(vector(e,0x14),sub(vector(w,0x60),vector(w,4)))));
    }else if surface_flag{
        put_vector(w,4,add(offset,sub(add(vector(w,0x6c),vector(w,0x60)),radius_vector)));
        put_vector(w,0x10,[k(0x75f8);3]);
    }else{
        put_vector(w,4,add(offset,sub(vector(w,0x60),radius_vector)));
        if signed_angle(vector(w,0x1c),vector(w,0x48),None,math)? >=0x2c00{
            let (length,unit)=math.estimated_vector(vector(w,0x6c))?;
            if length>=k(0x75c8)as f64{
                let direction=unit.ok_or("undefined terrain step normalization")?;
                let (speed,_)=math.estimated_vector(vector(w,0x10))?;
                put_vector(w,0x10,direction.map(|v|((v as f64)*speed)as f32));
            }
        }
    }
    let speed=float(c,0x64);let dash=float(c,0xf4);let jog=float(c,0xe4);
    if ![speed,dash,jog,float(c,0x68)].into_iter().all(f32::is_finite){return Err("nonfinite terrain step speed".into());}
    let radius=if speed>=dash || word(c,0x8c)&0x100!=0 || (float(c,0x68)<=k(0x75f8) && surface_flag){k(0x74d4)}
        else if speed>=jog && speed<=dash{k(0x753c)+(speed-jog)/(dash-jog)}else{k(0x753c)};
    put(w,0,radius);
    if !sweep(w,objects,ctx.object_identifier,0,false,state,math)?{w.copy_from_slice(&snapshot);return Ok(false);}
    let current=vector(w,0x48);let previous=vector(w,0x1c);
    if signed_angle(previous,current,None,math)?>0x2c00 && dot(ctx.gravity,previous)<=dot(ctx.gravity,current) && !surface_flag{
        w.copy_from_slice(&snapshot);return Ok(false);
    }
    let previous_status=u16::from_be_bytes(c[0x1c..0x1e].try_into().unwrap());
    if dot(ctx.gravity,current)>k(0x75f8) && previous_status&3==0{w.copy_from_slice(&snapshot);return Ok(false);}
    w[..4].copy_from_slice(&snapshot[..4]);w[0x10..0x1c].copy_from_slice(&snapshot[0x10..0x1c]);
    if prior_qualifies{
        w[0x6c..0x78].copy_from_slice(&snapshot[0x6c..0x78]);w[0x60..0x6c].copy_from_slice(&snapshot[0x60..0x6c]);
    }else{
        let [pitch,roll]=if normal_coordinates{math.surface_angles(current)?}else{[word(w,0x34),word(w,0x38)]};
        let radius_vector=orient([k(0x75f8),if player_eight{ctx.special_radius}else{float(w,0)},k(0x75f8)],pitch,roll,yaw,math)?;
        let point=vector(w,0x54);
        let plane=[point[0],point[1],point[2],current[0],current[1],current[2]];
        let projection=point_plane(vector(&snapshot,0x60),plane,true,math)?.1.ok_or("missing terrain projection")?;
        put_vector(w,0x60,add(radius_vector,projection));
        put_vector(w,0x6c,add(vector(w,0x6c),sub(point,vector(&snapshot,0x54))));
    }
    w[4..0x10].copy_from_slice(&snapshot[4..0x10]);pole(w,e,math);Ok(true)
}
