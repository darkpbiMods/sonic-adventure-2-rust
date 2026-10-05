//! PC-side owned mapping for original Sonic animation assets. No fallback clips.
use serde::Serialize;
use crate::{disc::{Result,digest},prs,original_contact::Memory,original_animation_archive::{register_models,register_motions},original_animation_graph::enumerate};
#[derive(Clone,Copy)]pub enum ArchiveKind{Motions,Models}
pub struct ArchiveInput<'a>{pub name:&'a str,pub kind:ArchiveKind,pub compressed:&'a[u8]}
#[derive(Serialize)]pub struct LoadedArchive{pub name:String,pub address:u32,pub size:usize,pub compressed_sha256:String,pub decoded_sha256:String,pub models:bool}
#[derive(Serialize)]pub struct AnimationReference{pub index:u16,pub motion:i16,pub model:i16,pub mode:i16,pub next:u16,pub blend_bits:u32,pub rate_bits:u32,pub motion_pointer:u32,pub model_pointer:u32,pub frames:Option<u32>,pub motion_joints:Option<i16>,pub model_nodes:Option<usize>}
#[derive(Serialize)]pub struct BundleReport{pub archives:Vec<LoadedArchive>,pub entries:Vec<AnimationReference>,pub missing_motions:Vec<i16>,pub missing_models:Vec<i16>}
pub struct AnimationBundle{pub base:u32,pub registry:u32,pub entry_table:u32,pub heap_table:u32,pub heap_start:u32,pub memory:Vec<u8>,pub report:BundleReport}
fn put(m:&mut[u8],at:usize,bytes:&[u8]){m[at..at+bytes.len()].copy_from_slice(bytes);}
fn dol_bytes(dol:&[u8],address:u32,size:usize)->Result<&[u8]>{
    for i in 0..18{
        let w=|a|u32::from_be_bytes(dol[a..a+4].try_into().unwrap());let off=w(i*4)as usize;let base=w(0x48+i*4);let length=w(0x90+i*4)as usize;
        if address>=base && (address-base)as usize+size<=length{return dol.get(off+(address-base)as usize..off+(address-base)as usize+size).ok_or("DOL asset table outside file".into());}
    }Err("DOL asset table is not mapped".into())
}
/// Uses the verified Sonic table801b762c..801b82dc,203 entries. Its start is
/// assigned by80087590; the next character table begins at801b82dc, and Sonic
/// action code explicitly selects final index202 at800796b8/8007a1b8.
/// Archive storage remains owned and pointer-stable in the returned mapping.
pub fn load_sonic(dol:&[u8],inputs:&[ArchiveInput<'_>])->Result<AnimationBundle>{
    if digest(dol)!="a53335308d1f820be236fd2b6342fee850eaf048eb26aac21ddb1903b060db2e"{return Err("unverified Sonic executable".into());}
    let base=0x81000000u32;let registry=base+0x800;let entry_table=base+0x3000;
    let mut memory=vec![0u8;0x200000];
    // Exact ID/pointer layout from the pure initialization prefix800503c8.
    // Zeroed task arrays at registry+1ae0..1b60 are included in owned storage.
    for i in 0..300u16{put(&mut memory,0x800+0xe0+i as usize*8,&i.to_be_bytes());}
    for i in 0..532u32{put(&mut memory,0x800+0xa40+i as usize*8,&i.to_be_bytes());}
    put(&mut memory,0x3000,dol_bytes(dol,0x801b762c,203*16)?);
    let mut offset=0x8000usize;let mut archives=Vec::new();
    for input in inputs{
        let decoded=prs::decompress(input.compressed,16*1024*1024)?;
        let end=offset.checked_add(decoded.len()).ok_or("archive mapping overflow")?;
        if end>memory.len()-0x10000{return Err("animation bundle exceeds owned mapping; no assets omitted".into());}
        memory[offset..end].copy_from_slice(&decoded);let address=base+offset as u32;
        let models=matches!(input.kind,ArchiveKind::Models);
        if models{register_models(address,registry,base,&mut memory)?;}else{register_motions(address,registry,base,&mut memory)?;}
        archives.push(LoadedArchive{name:input.name.into(),address,size:decoded.len(),compressed_sha256:digest(input.compressed),decoded_sha256:digest(&decoded),models});
        offset=(end+31)&!31;
    }
    put(&mut memory,0x6000,&(-1i32).to_be_bytes());
    let view=Memory{base,bytes:&memory};let mut entries=Vec::new();let mut missing_motions=std::collections::BTreeSet::new();let mut missing_models=std::collections::BTreeSet::new();
    for index in 0..203u16{
        let a=entry_table+index as u32*16;let motion=view.half(a)?as i16;let model=view.half(a+2)?as i16;
        if !(0..300).contains(&motion)||!(0..532).contains(&model){return Err("Sonic table reference outside original registry limits".into());}
        let m=registry+0xe0+motion as u32*8;let motion_pointer=view.word(m+4)?;
        let model_pointer=view.word(registry+0xa44+model as u32*8)?;
        if motion_pointer==0{missing_motions.insert(motion);}if model_pointer==0{missing_models.insert(model);}
        let model_nodes=if model_pointer!=0{let mut nodes=[0u32;111];Some(enumerate(model_pointer,&view,&mut nodes)?)}else{None};
        entries.push(AnimationReference{index,motion,model,mode:view.half(a+4)?as i16,next:view.half(a+6)?,blend_bits:view.word(a+8)?,rate_bits:view.word(a+12)?,motion_pointer,model_pointer,
            frames:if motion_pointer==0{None}else{Some(view.word(motion_pointer+4)?)},motion_joints:if motion_pointer==0{None}else{Some(view.half(m+2)?as i16)},model_nodes});
    }
    Ok(AnimationBundle{base,registry,entry_table,heap_table:base+0x6000,heap_start:base+offset as u32,memory,
        report:BundleReport{archives,entries,missing_motions:missing_motions.into_iter().collect(),missing_models:missing_models.into_iter().collect()}})
}
