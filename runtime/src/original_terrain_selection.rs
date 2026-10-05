//! Complete contact limiter 800414f0 and selectors 80041984/80041c78.
use crate::{disc::Result,original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES},original_math::OriginalMath,
    original_geometry::signed_angle,original_terrain_contacts::distance,original_angle_table};

#[derive(Clone,Copy)]
pub enum Selection { Limit{incoming_f26:f64}, Primary, Alternate }
pub struct SelectionContext {pub gravity:[f32;3],pub surface_flags:u32,pub descriptor_attached:bool}
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn vector(b:&[u8],i:usize)->[f32;3]{std::array::from_fn(|j|float(b,i+j*4))}
fn put_vector(b:&mut[u8],i:usize,v:[f32;3]){for j in 0..3{b[i+j*4..i+j*4+4].copy_from_slice(&v[j].to_be_bytes());}}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],b[0]*a[0]))}
fn checked(v:[f32;3])->Result<[f32;3]>{if v.into_iter().all(f32::is_finite){Ok(v)}else{Err("nonfinite terrain selection vector".into())}}
fn cap(b:&mut[u8],off:usize,amount:f32,math:&OriginalMath)->Result<()> {
    let v=checked(vector(b,off))?;let (_,unit)=math.estimated_vector(v)?;
    let out=unit.unwrap_or([0.;3]).map(|x|x*amount);put_vector(b,off,checked(out)?);Ok(())
}
fn short_character(c:&[u8],math:&OriginalMath)->bool{
    (c[1]==0 && float(c,0x134)<math.constant(0x80196578+0x74)) ||
    (c[1]==1 && float(c,0x134)<math.constant(0x80196578+0xf8))
}
fn corner_exempt(e:&[u8],c:&[u8])->bool{(c[1]==0 || c[1]==1) && (0x4c..=0x52).contains(&(e[0]as i8))}

/// Source records have stride 0x58 and the source stack arrays hold 128.
/// A Limit call must receive the caller's actual f26, including its full f64
/// precision. The original callee restores f26 on return; it is not an output.
pub fn select(kind:Selection,e:&[u8],motion:&mut[u8],c:&[u8],workspace:&mut[u8],records:&[u8],
              collision_flag:&mut u32,ctx:&SelectionContext,math:&OriginalMath)->Result<u32>{
    if e.len()!=ENTITY_BYTES || c.len()!=CHARACTER_BYTES || motion.len()<12 || workspace.len()<0x2a{
        return Err("unsupported terrain selection state extent".into());
    }
    let count=u16::from_be_bytes(workspace[0x28..0x2a].try_into().unwrap())as usize;
    if count>128 || records.len()<count*0x58{return Err("unsupported terrain selection record count".into());}
    checked(ctx.gravity)?;checked(vector(workspace,0x1c))?;checked(vector(workspace,4))?;checked(vector(workspace,0x10))?;
    if ![float(c,0x134),float(c,0x64),float(c,0xe4),float(c,0xe8),float(c,0xf0)].into_iter().all(f32::is_finite){return Err("nonfinite terrain selection character".into());}
    let mut m=motion.to_vec();let mut w=workspace.to_vec();let mut flag=*collision_flag;
    let result=select_inner(kind,e,&mut m,c,&mut w,&records[..count*0x58],&mut flag,ctx,math)?;
    motion.copy_from_slice(&m);workspace.copy_from_slice(&w);*collision_flag=flag;Ok(result)
}
fn select_inner(kind:Selection,e:&[u8],m:&mut[u8],c:&[u8],w:&mut[u8],records:&[u8],flag:&mut u32,ctx:&SelectionContext,math:&OriginalMath)->Result<u32>{
    let limit=matches!(kind,Selection::Limit{..});let alternate=matches!(kind,Selection::Alternate);
    let k=|off:u32|math.constant(0x803b6520-off);
    let mut nearest=k(0x7554);let mut selected:Option<usize>=None;
    let mut opposite_nearest=match kind{Selection::Limit{incoming_f26}=>incoming_f26,_=>0.};
    let mut opposite_selected:Option<usize>=None;let mut opposing_count=0;let mut ground_count=0;
    let mut distances=Vec::with_capacity(records.len()/0x58);let mut result=0;
    for (i,record)in records.chunks_exact(0x58).enumerate(){
        let d=distance(vector(record,0x2c),vector(w,4),math)?;distances.push(d);
        let normal=checked(vector(record,0x14))?;
        let angle=signed_angle(vector(w,0x1c),normal,None,math)?;
        let ground=angle<=0x2c00 || dot(ctx.gravity,vector(w,0x1c))>dot(ctx.gravity,normal);
        if ground{
            if limit{ground_count+=1;if nearest>d{nearest=d;selected=Some(i);}}
            continue;
        }
        let velocity=vector(w,0x10);let (_,unit)=math.estimated_vector(velocity)?;
        let direction=unit.unwrap_or(velocity);let incidence=dot(direction,normal);
        if (!limit && !alternate && angle>=0x7000) || incidence>=if limit{k(0x7568)}else{k(0x75f8)}{continue;}
        let old=if limit{opposite_selected}else{selected};
        let threshold=if alternate{k(0x7504)}else{k(0x75c8)};
        if old.is_none() || dot(normal,vector(records,old.unwrap()*0x58+0x14))<threshold{opposing_count+=1;}
        if limit{
            // The branch tests the greater-than CR bit; unordered skips the
            // assignment too, leaving the incoming NaN and absent index intact.
            if opposite_nearest>d as f64{opposite_nearest=d as f64;opposite_selected=Some(i);}
        }else if nearest>d{nearest=d;selected=Some(i);}
        if alternate && ctx.surface_flags&0x80000==0{
            if incidence < -original_angle_table::entry(0x600,math)?[1]{
                put_vector(m,0,[k(0x75f8);3]);
                if ctx.surface_flags&0x400000!=0 && float(c,0x64)>=float(c,0xf0){result=2;}
            }
            if dot(vector(w,0x10),normal)<k(0x7508){
                let (speed,_)=math.estimated_vector(checked(vector(m,0))?)?;
                if speed>float(c,0xe8)as f64{cap(m,0,float(c,0xe8),math)?;result=1;}
            }
        }
        // Alternate's sum of opposing normals and local flag arrays are
        // stack-only temporaries, not read after the loop or returned.
    }
    if opposing_count>0{
        if opposing_count>=2 && !corner_exempt(e,c){*flag=1;}
        if !ctx.descriptor_attached && short_character(c,math){
            put_vector(w,0x10,[k(0x75f8);3]);put_vector(m,0,[k(0x75f8);3]);return Ok(1);
        }
        if alternate && e[0]==1 && opposing_count>=2 && ctx.surface_flags&0x1000==0{
            let target=k(0x7514)*float(c,0xe4);put_vector(w,0x10,[k(0x75f8);3]);
            let (speed,_)=math.estimated_vector(checked(vector(m,0))?)?;
            if speed>target as f64{cap(m,0,target,math)?;}
            return Ok(1);
        }
    }
    if (limit && ground_count>0) || (!limit && opposing_count>0){
        // The original sentinel assumes a nearby contact; without a selected
        // index it would read before the local distance array. Do not fabricate
        // that unspecified stack value for out-of-domain fixtures.
        let index=selected.ok_or("terrain contact outside original distance sentinel domain")?;
        let (speed,_)=math.estimated_vector(vector(w,0x10))?;
        if (distances[index]as f64)<speed{
            cap(w,0x10,distances[index],math)?;
            return Ok(if alternate{result}else{1});
        }
    }
    Ok(result)
}
