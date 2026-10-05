//! Original channel construction8003ca00..8003d678. Registry base801e5c48.
use crate::{disc::Result, original_contact::Memory, original_animation_graph,
    original_gameplay::angle_delta, original_heap::{self, HeapContext}};
fn word(m:&[u8],b:u32,a:u32)->Result<u32>{Memory{base:b,bytes:m}.word(a)}
fn half(m:&[u8],b:u32,a:u32)->Result<u16>{Memory{base:b,bytes:m}.half(a)}
fn put(m:&mut[u8],b:u32,a:u32,v:u32)->Result<()>{
    let i=a.checked_sub(b).ok_or("animation write below memory")? as usize;
    m.get_mut(i..i+4).ok_or("animation write outside memory")?.copy_from_slice(&v.to_be_bytes());Ok(())
}
fn triple(m:&[u8],b:u32,a:u32)->Result<[u32;3]>{Ok([word(m,b,a)?,word(m,b,a+4)?,word(m,b,a+8)?])}
fn evaluate(m:&[u8],b:u32,keys:u32,count:u32,frames:u32,time:f32,new:bool,rotation:bool)->Result<[u32;3]>{
    if !time.is_finite(){return Err("nonfinite animation time outside verified domain".into());}
    if new && time==0.0{return triple(m,b,keys+4);}
    let (mut previous,mut current)=(0u32,0u32);let last=count.wrapping_sub(1);
    // Do not reinterpret count0, or the source's overlapping three-channel fields.
    // A mapped-memory bound prevents malformed input from running indefinitely.
    loop{
        let at=keys.wrapping_add(current.wrapping_mul(16));
        if time<=word(m,b,at)? as f32{break;}
        if current>=last{previous=last;current=0;break;}
        previous=current;current+=1;
        if current as usize>m.len()/16{return Err("animation key scan exceeds mapped memory".into());}
    }
    let start=keys.wrapping_add(previous.wrapping_mul(16));let mut values=triple(m,b,start+4)?;
    if current==previous{return Ok(values);}
    let end=keys.wrapping_add(current.wrapping_mul(16));let next=triple(m,b,end+4)?;
    let frame=word(m,b,start)?;let mut span=word(m,b,end)?.wrapping_sub(frame);
    if current<previous{span=span.wrapping_add(frames);}
    let factor=(time-frame as f32)/(span as f32);
    if !factor.is_finite(){return Err("degenerate animation interval outside verified domain".into());}
    for axis in 0..3{
        values[axis]=if rotation{
            let delta=factor*(angle_delta(values[axis],next[axis]) as f32);
            // Rust's saturating conversion matches finite fctiwz; nonfinite rejected above.
            values[axis].wrapping_add((delta as i32) as u32)
        }else{
            let from=f32::from_bits(values[axis]);let to=f32::from_bits(next[axis]);
            if !from.is_finite()||!to.is_finite(){return Err("nonfinite animation key outside verified domain".into());}
            factor.mul_add(to-from,from).to_bits()
        };
    }
    Ok(values)
}
/// Transactional host errors; ordinary source allocation failure is committed.
/// Valid domain: separate source/model/output/heap storage, finite keys/times,
/// terminating key scans and model traversal fitting the original stack array.
pub fn build(state:u32,base:u32,memory:&mut[u8],registry:u32,heap:&HeapContext,index:i32)->Result<()>{
    let mut m=memory.to_vec();let table=word(&m,base,state+0x24)?;
    let selected=table.wrapping_add((half(&m,base,state+2)? as u32)*16);
    let active=table.wrapping_add((half(&m,base,state+4)? as u32)*16);
    let model=half(&m,base,selected+2)? as i16 as i32;
    let root=word(&m,base,registry.wrapping_add(0xa44).wrapping_add(model.wrapping_mul(8) as u32))?;
    let mut nodes=[0u32;111];let available=original_animation_graph::enumerate(root,&Memory{base,bytes:&m},&mut nodes)?;
    let new_id=half(&m,base,selected)? as i16 as i32;let old_id=half(&m,base,active)? as i16 as i32;
    let motion=|id:i32| registry.wrapping_add(id.wrapping_mul(8) as u32);
    let headers=[word(&m,base,motion(old_id)+0xe4)?,word(&m,base,motion(new_id)+0xe4)?];
    let counts=[if half(&m,base,headers[0]+8)?&4!=0{3}else{2},if half(&m,base,headers[1]+8)?&4!=0{3}else{2}];
    let mut inputs=[word(&m,base,headers[0])?,word(&m,base,headers[1])?];
    let output_header=word(&m,base,state+0x28)?;let mut output=word(&m,base,output_header)?;
    let channels=counts[0].max(counts[1]);let count=half(&m,base,motion(new_id)+0xe2)? as i16;
    if count<0 || count as usize>available{return Err("animation joint count exceeds initialized traversal".into());}
    for node in &nodes[..count as usize]{
        for channel in 0..channels{
            if channel==2{
                let rotation_count=word(&m,base,output+0xc)?;let position_count=word(&m,base,output+8)?;
                put(&mut m,base,output+0x10,rotation_count)?;put(&mut m,base,output+0xc,position_count)?;
            }
            let pointers=[if channel==2&&counts[0]!=3{0}else{word(&m,base,inputs[0]+channel*4)?},
                          if channel==2&&counts[1]!=3{0}else{word(&m,base,inputs[1]+channel*4)?}];
            let pointer=if pointers.iter().any(|p|*p!=0){original_heap::allocate_in_place(base,&mut m,heap,index,32)?}else{0};
            put(&mut m,base,output+channel*4,pointer)?;
            let count_slot=output+if channel==2{0x14}else{8+channel*4};
            if pointer==0{put(&mut m,base,count_slot,0)?;continue;}
            for endpoint in 0..2{
                let target=pointer+(endpoint as u32)*16;put(&mut m,base,target,endpoint as u32)?;
                let values=if pointers[endpoint]==0{triple(&m,base,*node+[8,0x14,0x20][channel as usize])?}else{
                    // Source always reads position count+8 and rotation count+c,
                    // even when the scale pointer also occupies+8. Preserve it.
                    let keys=word(&m,base,inputs[endpoint]+[8,0xc,0x14][channel as usize])?;
                    let frames=word(&m,base,headers[endpoint]+4)?;
                    let time=f32::from_bits(word(&m,base,state+if endpoint==0{0x10}else{0x18})?);
                    evaluate(&m,base,pointers[endpoint],keys,frames,time,endpoint==1,channel==1)?
                };
                for axis in 0..3{put(&mut m,base,target+4+axis*4,values[axis as usize])?;}
            }
            if channel==1{
                for offset in [4,8,12,20,24,28]{let value=word(&m,base,pointer+offset)? as i16 as i32 as u32;put(&mut m,base,pointer+offset,value)?;}
                for axis in 0..3{let from=word(&m,base,pointer+4+axis*4)?;let to=word(&m,base,pointer+20+axis*4)?;
                    put(&mut m,base,pointer+20+axis*4,original_animation_graph::continuous_angle(from,to)?)?;}
            }
            put(&mut m,base,count_slot,2)?;
        }
        output+=channels*8;for endpoint in 0..2{inputs[endpoint]+=counts[endpoint]*8;}
    }
    put(&mut m,base,output_header+8,if channels==3{0x00070003}else{0x00030002})?;
    memory.copy_from_slice(&m);Ok(())
}
/// Wrapper8003d678..8003d720 reads the two motion flags, then both branches
/// call8003ca00 identically. Those reads are repeated by build; no state writes
/// or other calls occur in the wrapper. Valid metadata has identical behavior.
pub fn transition(state:u32,base:u32,memory:&mut[u8],registry:u32,heap:&HeapContext,index:i32)->Result<()>{
    build(state,base,memory,registry,heap,index)
}
