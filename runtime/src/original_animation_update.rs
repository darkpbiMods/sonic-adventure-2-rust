//! Complete animation state machine8003d8d0..8003e668.
use crate::{disc::Result,original_contact::Memory,original_math::OriginalMath,
    original_heap::{self,HeapContext},original_animation_support as support,
    original_animation_transition as transition};

/// Presentation metadata captured before the original transition constructs channels.
#[derive(Debug,Clone,Copy,PartialEq)]
pub struct BlendEndpoints {pub from_animation:u16,pub from_frame:f32,pub to_animation:u16,pub to_frame:f32}
struct Update<'a>{blend:Option<BlendEndpoints>,base:u32,state:u32,registry:u32,heap:&'a HeapContext,index:i32,math:&'a OriginalMath,mem:Vec<u8>}
impl Update<'_>{
    fn w(&self,a:u32)->Result<u32>{Memory{base:self.base,bytes:&self.mem}.word(a)}
    fn h(&self,a:u32)->Result<u16>{Memory{base:self.base,bytes:&self.mem}.half(a)}
    fn s(&self,a:u32)->Result<i32>{Ok(self.h(a)? as i16 as i32)}
    fn f(&self,a:u32)->Result<f32>{let v=f32::from_bits(self.w(a)?);if !v.is_finite(){return Err("nonfinite animation input outside verified domain".into());}Ok(v)}
    fn put(&mut self,a:u32,b:&[u8])->Result<()>{
        let at=a.checked_sub(self.base).ok_or("animation state write below memory")? as usize;
        self.mem.get_mut(at..at+b.len()).ok_or("animation state write outside memory")?.copy_from_slice(b);Ok(())
    }
    fn wh(&mut self,a:u32,v:u16)->Result<()>{self.put(a,&v.to_be_bytes())}
    fn ww(&mut self,a:u32,v:u32)->Result<()>{self.put(a,&v.to_be_bytes())}
    fn wf(&mut self,a:u32,v:f32)->Result<()>{if !v.is_finite(){return Err("nonfinite animation result outside verified domain".into());}self.ww(a,v.to_bits())}
    fn entry(&self,id:i32)->Result<u32>{Ok(self.w(self.state+0x24)?.wrapping_add(id.wrapping_mul(16) as u32))}
    fn frame_count(&self,id:i32)->Result<f32>{
        let motion=self.s(self.entry(id)?)?;
        let header=self.w(self.registry.wrapping_add(0xe4).wrapping_add(motion.wrapping_mul(8) as u32))?;
        Ok(self.w(header+4)? as f32)
    }
    fn model(&self,id:i32)->Result<u32>{
        let model=self.s(self.entry(id)?+2)?;
        self.w(self.registry.wrapping_add(0xa44).wrapping_add(model.wrapping_mul(8) as u32))
    }
    fn constant(&self,offset:u32)->f32{self.math.constant(0x803b6520-offset)}
    fn zero(&self)->f32{self.constant(0x75f8)}
    fn one(&self)->f32{self.constant(0x75c8)}
    fn reset(&mut self)->Result<()>{support::reset_time(self.state,self.base,&mut self.mem,self.registry+0xe0,self.math)}
    fn cleanup(&mut self)->Result<()>{support::cleanup(self.state,self.base,&mut self.mem,self.registry+0xe0,self.heap,self.index)}
    fn transition(&mut self)->Result<()>{
        let endpoints=BlendEndpoints{from_animation:self.h(self.state+4)?,from_frame:self.f(self.state+0x10)?,to_animation:self.h(self.state+2)?,to_frame:self.f(self.state+0x18)?};
        transition::transition(self.state,self.base,&mut self.mem,self.registry,self.heap,self.index)?;
        self.blend=Some(endpoints);Ok(())
    }
    fn flag(&mut self,bits:u16)->Result<()>{self.wh(self.state+0xc,self.h(self.state+0xc)?|bits)}
    fn allocate(&mut self,size:u32)->Result<u32>{original_heap::allocate(self.base,&mut self.mem,self.heap,self.index,size)}
    fn free(&mut self,p:u32)->Result<()>{original_heap::free(self.base,&mut self.mem,self.heap,self.index,p)}
    // Identical inline reset bodies at dbb0, e18c, e3fc are reset_time's logic.
    fn finish_blend(&mut self,entry:u32)->Result<()>{
        self.wh(self.state,self.h(entry+4)?)?;self.cleanup()?;self.reset()
    }
    fn advance_blend(&mut self,entry:u32)->Result<()>{
        let t=self.f(self.state+0x10)?+self.f(entry+8)?;self.wf(self.state+0x10,t)?;
        if t>=self.one(){self.finish_blend(entry)?;}Ok(())
    }
    fn switch_direct(&mut self,entry:u32)->Result<()>{
        let next=self.h(entry+6)?&0xff;
        self.wh(self.state+2,next)?;self.wh(self.state+4,next)?;
        let e=self.entry(next as i32)?;self.wh(self.state,self.h(e+4)?)?;
        self.wh(self.state+8,self.h(e+6)?&0xff)?;self.flag(1)?;self.reset()
    }
    fn wrap(&mut self,frames:f32,upper:f32)->Result<()>{
        let mut t=self.f(self.state+0x10)?;let mut steps=0u32;
        // Repeated single-rounded arithmetic, including mode3's asymmetric periods.
        while t>=upper{
            let next=t-upper;
            if next==t||steps>=1_000_000{return Err("original animation wrap stalled or exceeds bounded host domain".into());}
            t=next;self.wf(self.state+0x10,t)?;steps+=1;
        }
        while self.zero()>t{
            let next=t+frames;
            if next==t||steps>=1_000_000{return Err("original animation wrap stalled or exceeds bounded host domain".into());}
            t=next;self.wf(self.state+0x10,t)?;steps+=1;
        }
        Ok(())
    }
    fn run(&mut self)->Result<()>{
        let state=self.state;let suppress=self.h(state+0xc)?&4!=0;
        let mut selected=self.h(state+2)? as i32;let active=self.s(state+4)?;
        self.wh(state+6,active as u16)?;self.ww(state+0x14,self.w(state+0x10)?)?;self.wh(state+0xc,0)?;
        let mode=self.s(state)?;
        if active!=selected && ![0,2,15].contains(&mode){
            self.wh(state,1)?;self.wh(state+0xa,0)?;
            let new=self.s(state+2)?;
            if self.s(self.entry(new)?+4)?==11 && self.s(self.entry(active)?+4)?==11{
                let old_frames=self.frame_count(active)?;let new_frames=self.frame_count(new)?;
                let remaining=old_frames-self.f(state+0x10)?;
                let mut time=(new_frames*remaining)/old_frames;
                time=self.constant(0x75a8).mul_add(new_frames,time);
                self.wf(state+0x18,time)?;
                if time>=new_frames{time-=new_frames;self.wf(state+0x18,time)?;}
            }
        }else{self.wh(state+0xa,self.h(state+0xa)?.wrapping_add(1))?;}
        match self.s(state)?{
            0=>{
                self.wf(state+0x10,self.zero())?;self.wh(state+4,selected as u16)?;self.wh(state,1)?;
                self.wf(state+0x18,self.zero())?;self.wf(state+0x10,self.zero())?;
                let header=self.allocate(12)?;self.ww(state+0x28,header)?;
                if header==0{return Err("original initialization dereferences failed header allocation".into());}
                self.ww(header+4,2)?;self.wh(header+8,3)?;self.wh(header+0xa,2)?;
                let data=self.allocate(111*24)?;
                if data==0{return Err("original calloc writes through failed allocation".into());}
                self.put(data,&[0u8;111*24])?;self.ww(header,data)?;
            }
            1=>{
                let entry=self.entry(selected)?;
                if !suppress && self.f(entry+8)?!=self.one() && self.model(selected)?==self.model(self.s(state+4)?)?{
                    self.transition()?;self.wf(state+0x10,self.zero())?;
                }else{self.wf(state+0x10,self.one())?;}
                self.wh(state+4,selected as u16)?;self.wh(state,2)?;
                self.advance_blend(self.entry(self.s(state+4)?)?)?;
            }
            2=>self.advance_blend(self.entry(self.s(state+4)?)?)?,
            mode @ (3|10|11|13)=>{
                let entry=self.entry(selected)?;
                if mode!=13{
                    let t=self.f(state+0x10)?;let rate=self.f(entry+0xc)?;
                    let t=if mode==3{t+rate}else{
                        let speed=self.f(self.w(state+if mode==10{0x1c}else{0x20})?)?;
                        speed.mul_add(rate,t)
                    };self.wf(state+0x10,t)?;
                }
                let frames=self.frame_count(selected)?;self.wrap(frames,if mode==3{frames-self.one()}else{frames})?;
            }
            mode @ (4|6|12)=>{
                let entry=self.entry(selected)?;
                let rate=if mode==12{self.f(self.w(state+0x20)?)?}else{self.f(entry+0xc)?};
                let t=self.f(state+0x10)?+rate;self.wf(state+0x10,t)?;
                let end=self.frame_count(selected)?-self.one();
                if end<=t{
                    if mode==6{self.wf(state+0x10,end)?;self.flag(1)?;}else{self.switch_direct(entry)?;}
                }else if mode==4 && end<=t+self.f(entry+0xc)?{self.flag(2)?;}
            }
            mode @ (7|8)=>{
                let entry=self.entry(selected)?;let t=self.f(state+0x10)?-self.f(entry+0xc)?;self.wf(state+0x10,t)?;
                if self.zero()>t{
                    if mode==8{self.wf(state+0x10,self.zero())?;self.flag(1)?;}else{self.switch_direct(entry)?;}
                }
            }
            mode @ (5|9)=>{
                let entry=self.entry(selected)?;let t=self.f(state+0x10)?+self.f(entry+0xc)?;self.wf(state+0x10,t)?;
                if self.frame_count(selected)?-self.one()<=t{
                    if mode==5{self.wh(state+8,self.h(entry+6)?)?;}
                    self.wh(state+0xa,0)?;
                    let next=self.s(state+8)?;let next_entry=self.entry(next)?;
                    if self.f(next_entry+8)?!=self.one() && self.model(next)?==self.model(selected)?{
                        self.wh(state+2,(next as u16)&0xff)?;selected=next;
                        self.transition()?;self.wf(state+0x10,self.zero())?;
                    }else{self.wf(state+0x10,self.one())?;}
                    self.wh(state+4,selected as u16)?;
                    // rlwinm at e128/e398 retains the low16 bits before scaling.
                    let blend_entry=self.entry((selected as u16) as i32)?;
                    self.wh(state+8,self.h(blend_entry+6)?&0xff)?;self.wh(state,2)?;
                    self.advance_blend(blend_entry)?;
                    if mode==9{self.flag(1)?;}
                }
            }
            _=>{
                self.cleanup()?;
                let header=self.w(state+0x28)?;
                if header!=0{
                    let data=self.w(header)?;
                    if data!=0{self.free(data)?;self.ww(header,0)?;}
                    self.free(header)?;self.ww(state+0x28,0)?;
                }
            }
        }
        Ok(())
    }
}
/// Mapped registry corresponds to801e5c48; actual heap callbacks selected by
/// the caller are represented by HeapContext/index. Source/model/heap regions
/// must be distinct. Errors roll back the host transaction, not valid source
/// state changes. Invalid pointers, nonfinite inputs and nonterminating source
/// loops are outside this bounded adapter domain.
pub fn update(state:u32,base:u32,memory:&mut[u8],registry:u32,heap:&HeapContext,index:i32,math:&OriginalMath)->Result<()>{
    update_with_transition(state,base,memory,registry,heap,index,math).map(|_|())
}
/// Reports a transition built during this update, not every tick of an active blend.
/// Metadata and mapped memory commit together only after a successful update.
pub fn update_with_transition(state:u32,base:u32,memory:&mut[u8],registry:u32,heap:&HeapContext,index:i32,math:&OriginalMath)->Result<Option<BlendEndpoints>>{
    let mut context=Update{blend:None,state,base,registry,heap,index,math,mem:memory.to_vec()};
    context.run()?;memory.copy_from_slice(&context.mem);Ok(context.blend)
}
