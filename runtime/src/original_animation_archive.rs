//! Original motion/model relocation and post-I/O registry population.
//! PC file reading and PRS decoding supply the owned archive allocation; this
//! module preserves the original pointer graph and registry operations.
use crate::{disc::Result,original_contact::Memory};
struct Archive{base:u32,mem:Vec<u8>}
impl Archive{
    fn w(&self,a:u32)->Result<u32>{Memory{base:self.base,bytes:&self.mem}.word(a)}
    fn h(&self,a:u32)->Result<u16>{Memory{base:self.base,bytes:&self.mem}.half(a)}
    fn put(&mut self,a:u32,b:&[u8])->Result<()>{let i=a.checked_sub(self.base).ok_or("archive write below memory")? as usize;
        self.mem.get_mut(i..i+b.len()).ok_or("archive write outside memory")?.copy_from_slice(b);Ok(())}
    fn ww(&mut self,a:u32,v:u32)->Result<()>{self.put(a,&v.to_be_bytes())}
    fn fix(&mut self,slot:u32,archive:u32)->Result<u32>{
        let mut p=self.w(slot)?;
        if p!=0 && p&0x80000000==0{p=p.wrapping_add(archive);self.ww(slot,p)?;}
        Ok(p)
    }
    // Complete8002f628..8002f88c. An already-fixed data pointer short-circuits
    // the entire motion, even if another pointer still looks relative.
    fn motion(&mut self,header:u32,archive:u32,joints:i32)->Result<()>{
        if header==0{return Ok(());}let data=self.w(header)?;
        if data==0 || data&0x80000000!=0{return Ok(());}
        let mut data=data.wrapping_add(archive);self.ww(header,data)?;
        let flags=self.h(header+8)? as u32;let channels=(self.h(header+0xa)?&15)as u32;
        let a=if flags&0x10!=0{Some((flags&15).count_ones())}else{None};
        let b=if flags&0x20!=0{Some((flags&31).count_ones())}else{None};
        // At f708 r9 retains flags&0x40, not flags&2. Preserve this source quirk.
        let c=if flags&0x40!=0{Some((flags&1).count_ones()+1+(flags&0x80).count_ones()+(flags&0x100).count_ones())}else{None};
        if joints>1_000_000{return Err("motion relocation exceeds bounded host joint domain".into());}
        for _ in 0..joints.max(0){
            for channel in 0..channels{self.fix(data.wrapping_add(channel*4),archive)?;}
            for channel in [a,b,c].into_iter().flatten(){
                let mut keys=self.w(data.wrapping_add(channel*4))?;
                if keys==0{continue;}
                let count=self.w(data.wrapping_add((channel+channels)*4))? as i32;
                if count>1_000_000{return Err("nested motion relocation exceeds bounded host key domain".into());}
                for _ in 0..count.max(0){self.fix(keys.wrapping_add(4),archive)?;keys=keys.wrapping_add(8);}
            }
            data=data.wrapping_add(channels*8);
        }
        Ok(())
    }
    // Complete8002f88c..8002f95c. The explicit continuation preserves the
    // sibling pointer captured before recursive child processing.
    fn model(&mut self,root:u32,archive:u32)->Result<()>{
        enum Work{Enter(u32),Sibling(u32,u32)}
        let mut work=vec![Work::Enter(root)];let mut visits=0usize;
        while let Some(item)=work.pop(){match item{
            Work::Enter(node)=>{
                if node==0{continue;}visits+=1;
                if visits>1_000_000{return Err("model relocation graph is cyclic or exceeds host visit limit".into());}
                let mesh=self.w(node.wrapping_add(4))?;
                let child=self.w(node.wrapping_add(0x2c))?;let sibling=self.w(node.wrapping_add(0x30))?;
                if mesh!=0 && mesh&0x80000000==0{
                    let mesh=mesh.wrapping_add(archive);self.ww(node.wrapping_add(4),mesh)?;
                    // These two payload additions have no high-bit check in source.
                    for offset in [0,4]{let p=self.w(mesh.wrapping_add(offset))?;
                        if p!=0{self.ww(mesh.wrapping_add(offset),p.wrapping_add(archive))?;}}
                }
                work.push(Work::Sibling(node,sibling));
                if child!=0{
                    let p=if child&0x80000000==0{let p=child.wrapping_add(archive);self.ww(node.wrapping_add(0x2c),p)?;p}else{child};
                    work.push(Work::Enter(p));
                }
            }
            Work::Sibling(node,sibling)=>{
                if sibling!=0{
                    let p=if sibling&0x80000000==0{let p=sibling.wrapping_add(archive);self.ww(node.wrapping_add(0x30),p)?;p}else{sibling};
                    work.push(Work::Enter(p));
                }
            }
        }}
        Ok(())
    }
    fn register(&mut self,archive:u32,registry:u32,models:bool)->Result<u32>{
        if archive==0{return Ok(0);}let mut record=archive;
        for _ in 0..1_000_000{
            let id=if models{self.w(record)? as i32}else{self.h(record)? as i16 as i32};
            if id == -1{return Ok(archive);}
            let limit=if models{0x214}else{0x12c};
            if id>=0 && id<limit{
                let slot=registry.wrapping_add(if models{0xa40}else{0xe0}).wrapping_add((id as u32)*8);
                if self.w(slot.wrapping_add(4))?==0{
                    let pointer=self.fix(record.wrapping_add(4),archive)?;
                    if !models{self.put(slot.wrapping_add(2),&self.h(record.wrapping_add(2))?.to_be_bytes())?;}
                    self.ww(slot.wrapping_add(4),pointer)?;
                }
            }
            // Even skipped/occupied/invalid registry slots still relocate the
            // supplied archive's pointer graph, as in the original loops.
            let pointer=self.fix(record.wrapping_add(4),archive)?;
            if pointer!=0{
                if models{self.model(pointer,archive)?;}
                else{let joints=self.h(record.wrapping_add(2))? as i16 as i32;self.motion(pointer,archive,joints)?;}
            }
            record=record.wrapping_add(8);
        }
        Err("archive directory has no bounded sentinel".into())
    }
}
pub fn relocate_motion(header:u32,archive:u32,joints:i32,base:u32,memory:&mut[u8])->Result<()>{
    let mut c=Archive{base,mem:memory.to_vec()};c.motion(header,archive,joints)?;memory.copy_from_slice(&c.mem);Ok(())
}
pub fn relocate_model(root:u32,archive:u32,base:u32,memory:&mut[u8])->Result<()>{
    let mut c=Archive{base,mem:memory.to_vec()};c.model(root,archive)?;memory.copy_from_slice(&c.mem);Ok(())
}
/// Complete post-read body800500c8..8005017c. Registry maps801e5c48. The
/// original8001a190 DVD/decode/allocation front end is an explicit host seam.
pub fn register_motions(archive:u32,registry:u32,base:u32,memory:&mut[u8])->Result<u32>{
    let mut c=Archive{base,mem:memory.to_vec()};let result=c.register(archive,registry,false)?;memory.copy_from_slice(&c.mem);Ok(result)
}
/// Complete post-read body8005023c..800502e0, with the same host I/O seam.
pub fn register_models(archive:u32,registry:u32,base:u32,memory:&mut[u8])->Result<u32>{
    let mut c=Archive{base,mem:memory.to_vec()};let result=c.register(archive,registry,true)?;memory.copy_from_slice(&c.mem);Ok(result)
}
