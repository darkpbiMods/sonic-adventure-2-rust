use std::{fs::File, io::{Read, Seek, SeekFrom}, path::Path};
use serde::Serialize;
use sha2::{Digest, Sha256};
pub type Result<T> = std::result::Result<T, String>;
const HEADER: usize = 0x8000;
const MAX_READ: usize = 32 * 1024 * 1024;
fn be(b: &[u8], i: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(b.get(i..i+4).ok_or("truncated integer")?.try_into().unwrap()))
}
pub fn digest(b: &[u8]) -> String { format!("{:x}", Sha256::digest(b)) }
pub struct Disc { file: File, block: u64, map: Vec<Option<u64>>, pub trailing_bytes: u64 }
impl Disc {
    pub fn open(path: &Path) -> Result<Self> {
        let mut file=File::open(path).map_err(|e|e.to_string())?;
        let size=file.metadata().map_err(|e|e.to_string())?.len();
        let mut h=vec![0;HEADER]; file.read_exact(&mut h).map_err(|e|e.to_string())?;
        if &h[..4]!=b"CISO" { return Err("expected GameCube CISO image".into()); }
        let block=u32::from_le_bytes(h[4..8].try_into().unwrap()) as u64;
        if block==0 || block>16*1024*1024 { return Err("invalid CISO block size".into()); }
        let mut physical=HEADER as u64; let mut map=Vec::new();
        for &present in &h[8..] {
            match present {
                0=>map.push(None),
                1=>{map.push(Some(physical));physical+=block;},
                _=>return Err("invalid CISO presence flag".into())
            }
        }
        if physical>size { return Err("truncated CISO payload".into()); }
        Ok(Self{file,block,map,trailing_bytes:size-physical})
    }
    pub fn read(&mut self, mut off:u64, n:usize)->Result<Vec<u8>> {
        if n>MAX_READ || off.checked_add(n as u64).ok_or("read overflow")?>self.block*self.map.len() as u64 {
            return Err("disc read outside bounds".into());
        }
        let mut data=vec![0;n];let mut written=0;
        while written<n {
            let block=(off/self.block) as usize;let within=off%self.block;
            let count=(n-written).min((self.block-within) as usize);
            if let Some(pos)=self.map[block] {
                self.file.seek(SeekFrom::Start(pos+within)).map_err(|e|e.to_string())?;
                self.file.read_exact(&mut data[written..written+count]).map_err(|e|e.to_string())?;
            }
            written+=count;off+=count as u64;
        }
        Ok(data)
    }
    pub fn inspect(&mut self)->Result<Inventory> {
        let boot=self.read(0,0x440)?;
        if be(&boot,0x1c)?!=0xc2339f3d { return Err("not a GameCube disc".into()); }
        let disc_id=String::from_utf8(boot[..6].to_vec()).map_err(|e|e.to_string())?;
        if disc_id!="GSNE8P" || boot[7]!=0 { return Err(format!("unverified disc build: {} rev {}",disc_id,boot[7])); }
        let dol_off=be(&boot,0x420)? as u64;
        let head=self.read(dol_off,0x100)?;
        let mut sections=Vec::new();let mut total=0x100;
        for index in 0..18 {
            let offset=be(&head,index*4)?;let address=be(&head,0x48+index*4)?;let size=be(&head,0x90+index*4)?;
            if size==0 {continue;}
            let end=offset.checked_add(size).ok_or("DOL file overflow")?;
            let mem_end=address.checked_add(size).ok_or("DOL memory overflow")?;
            if offset<0x100 || end as usize>MAX_READ || address<0x80000000 || mem_end>0x81800000 {
                return Err("DOL section outside bounds".into());
            }
            total=total.max(end);
            sections.push(Section{index,offset,address,size});
        }
        for (i,a) in sections.iter().enumerate() { for b in &sections[i+1..] {
            if a.address<b.address+b.size && b.address<a.address+a.size {return Err("overlapping DOL memory".into());}
            if a.offset<b.offset+b.size && b.offset<a.offset+a.size {return Err("overlapping DOL sections".into());}
        }}
        let entry=be(&head,0xe0)?;
        if !sections.iter().any(|s|s.index<7 && s.address<=entry && entry<s.address+s.size) {return Err("entry outside executable code".into());}
        let dol=self.read(dol_off,total as usize)?;
        let dol_sha256=digest(&dol);
        if dol_sha256!="a53335308d1f820be236fd2b6342fee850eaf048eb26aac21ddb1903b060db2e" {return Err("DOL differs from verified SA2 build".into());}
        let fst=self.read(be(&boot,0x424)? as u64,be(&boot,0x428)? as usize)?;
        let files=parse_fst(&fst,self.block*self.map.len() as u64)?;
        let at=|address:u32,length:usize|->Result<&[u8]>{
            let end=address.checked_add(length as u32).ok_or("table overflow")?;
            let s=sections.iter().find(|s|s.address<=address&&end<=s.address+s.size).ok_or("table outside DOL")?;
            let off=(s.offset+address-s.address)as usize;
            dol.get(off..off+length).ok_or("table file bounds".into())
        };
        let sonic=crate::sonic::Tables{physics:crate::sonic::Physics::decode(at(0x80196578,132)?)?,
            animation_table_address:0x801b762c,animations:crate::sonic::decode_animations(at(0x801b762c,203*16)?)?};
        Ok(Inventory{disc_id,revision:boot[7],dol_sha256,dol_size:total,entry,sections,files,trailing_bytes:self.trailing_bytes,sonic})
    }
}
#[derive(Serialize)] pub struct Section {pub index:usize,pub offset:u32,pub address:u32,pub size:u32}
#[derive(Serialize)] pub struct Entry {pub path:String,pub offset:u32,pub size:u32}
#[derive(Serialize)] pub struct Inventory {pub disc_id:String,pub revision:u8,pub dol_sha256:String,pub dol_size:u32,pub entry:u32,pub sections:Vec<Section>,pub files:Vec<Entry>,pub trailing_bytes:u64,pub sonic:crate::sonic::Tables}
fn parse_fst(fst:&[u8],disc_len:u64)->Result<Vec<Entry>> {
    if be(fst,0)?>>24!=1 {return Err("invalid FST root".into());}
    let count=be(fst,8)? as usize;
    let names=count.checked_mul(12).ok_or("FST overflow")?;
    if count==0 || names>fst.len() {return Err("invalid FST count".into());}
    let mut stack=vec![(String::new(),count,0usize)];let mut entries=Vec::new();
    for i in 1..count {
        while stack.last().is_some_and(|x|i>=x.1) {stack.pop();}
        let (prefix,parent_end,parent_index)=stack.last().ok_or("FST ancestry")?;
        let word=be(fst,i*12)?;let offset=be(fst,i*12+4)?;let size=be(fst,i*12+8)?;
        let start=names+(word&0xffffff) as usize;
        let rest=fst.get(start..).ok_or("FST name range")?;
        let end=rest.iter().position(|&x|x==0).ok_or("unterminated FST name")?;
        let name=std::str::from_utf8(&rest[..end]).map_err(|e|e.to_string())?;
        if name.is_empty() || name=="." || name==".." || name.contains(['/', '\\', ':']) {return Err("unsafe FST path".into());}
        let path=format!("{prefix}{name}");
        match word>>24 {
            0=>{if offset as u64+size as u64>disc_len {return Err("FST file out of range".into());}entries.push(Entry{path,offset,size});},
            1=>{if size as usize<=i || size as usize>*parent_end || offset as usize!=*parent_index {return Err("invalid FST directory".into());} stack.push((format!("{path}/"),size as usize,i));},
            _=>return Err("invalid FST type".into())
        }
    }
    Ok(entries)
}
#[cfg(test)] mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize,Ordering};
    static NEXT:AtomicUsize=AtomicUsize::new(0);
    fn with_image(bytes:&[u8],run:impl FnOnce(&Path)) {
        let path=std::env::temp_dir().join(format!("sa2-rust-{}-{}.ciso",std::process::id(),NEXT.fetch_add(1,Ordering::Relaxed)));
        std::fs::write(&path,bytes).unwrap();run(&path);std::fs::remove_file(path).unwrap();
    }
    #[test] fn truncated_fst() {assert!(parse_fst(&[],10).is_err());}
    #[test] fn root_only() {assert!(parse_fst(&[1,0,0,0,0,0,0,0,0,0,0,1],100).unwrap().is_empty());}
    #[test] fn bad_root_count() {assert!(parse_fst(&[1,0,0,0,0,0,0,0,255,255,255,255],100).is_err());}
    #[test] fn sparse_cross_block_and_bounds() {
        let mut h=vec![0;HEADER];h[..4].copy_from_slice(b"CISO");h[4..8].copy_from_slice(&4u32.to_le_bytes());h[8]=1;h[10]=1;
        h.extend_from_slice(b"ABCDEFGH");h.extend_from_slice(b"footer");
        with_image(&h,|p|{let mut d=Disc::open(p).unwrap();assert_eq!(d.trailing_bytes,6);
            assert_eq!(d.read(2,8).unwrap(),b"CD\0\0\0\0EF");
            assert!(d.read(u64::MAX,4).is_err());assert!(d.read(0,MAX_READ+1).is_err());});
    }
    #[test] fn reject_bad_ciso_payload_and_flags() {
        let mut h=vec![0;HEADER];h[..4].copy_from_slice(b"CISO");h[4..8].copy_from_slice(&4u32.to_le_bytes());h[8]=1;
        with_image(&h,|p|assert!(Disc::open(p).is_err()));
        h[8]=2;with_image(&h,|p|assert!(Disc::open(p).is_err()));
        h[8]=0;h[4..8].fill(0);with_image(&h,|p|assert!(Disc::open(p).is_err()));
    }
}
