//! Complete controller input direction routine 8004e560..8004e924.
use crate::{disc::Result,original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES},original_contact::Memory,
    original_contact_search::find_task,original_math::{OriginalMath,IDENTITY,transform},original_geometry::signed_angle};
pub struct ControlContext {
    pub directions:[u32;2],pub magnitudes:[f32;2],pub stick_axis:[i16;2],pub player_tasks:[u32;2],
    pub gravity:[f32;3],pub level:i16,pub control_mode:u32,
}
#[derive(Clone)]pub struct InputOutputs {pub direction:Option<u32>,pub magnitude:Option<f32>}
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn short(b:&[u8],i:usize)->u16{u16::from_be_bytes(b[i..i+2].try_into().unwrap())}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],b[0]*a[0]))}
pub fn read(entity:&[u8],character:&[u8],ctx:&ControlContext,memory:&Memory,cache:&mut[u32;4],outputs:&mut InputOutputs,math:&OriginalMath)->Result<bool>{
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES{return Err("invalid control state extent".into());}
    let status=short(entity,4);if status&0x4000!=0{return Ok(false);}
    let player=character[0]as usize;if player>1{return Err("control player outside two source slots".into());}
    let magnitude=ctx.magnitudes[player];if !magnitude.is_finite(){return Err("nonfinite controller magnitude".into());}
    if magnitude==0. || (status&0x2000==0 && short(character,0x14)as i16!=0){return Ok(false);}
    let mut next_cache=*cache;let mut next=outputs.clone();
    if next.direction.is_some(){
        let mut direction=ctx.directions[player];
        if short(character,0x1a)as i16>0{
            direction=match ctx.control_mode&0x18{0=>direction.wrapping_add(0x4000),8=>0x8000_u32.wrapping_sub(direction),16=>direction.wrapping_add(0x8000),_=>0x4000_u32.wrapping_sub(direction)};
        }
        if ctx.level!=0x2c && ctx.level!=0x32{
            let task=find_task(memory,ctx.player_tasks[player],0x11,&mut next_cache)?;
            if task==0{
                if !ctx.gravity[1].is_finite(){return Err("nonfinite control gravity".into());}
                if ctx.gravity[1]>math.constant(0x803b6520-0x75a8){direction=0x8000_u32.wrapping_sub(direction);}
            }else{
                let surface=memory.word(task.wrapping_add(0x34))?;
                let m=math.rotate(&IDENTITY,b'y',memory.word(surface.wrapping_add(0xc))?as i32)?;
                let m=math.rotate(&m,b'z',memory.word(surface.wrapping_add(0x10))?as i32)?;
                let zero=math.constant(0x803b6520-0x75f8);let one=math.constant(0x803b6520-0x75c8);
                let mut normal=transform(&m,[zero,one,zero]);if normal[1]<zero{normal=normal.map(|v|-v);}
                let m=math.rotate(&IDENTITY,b'z',word(entity,0x10)as i32)?;
                let m=math.rotate(&m,b'x',word(entity,8)as i32)?;
                let right=transform(&m,[one,zero,zero]);let up=transform(&m,[zero,one,zero]);
                let angle=signed_angle(right,normal,Some(up),math)?;
                let opposite=angle.wrapping_add(0x8000);
                if normal[1]>math.constant(0x803b6520-0x7498){
                    direction=if ctx.stick_axis[player]<0{angle}else{opposite};
                }else{
                    let m=math.rotate(&IDENTITY,b'y',direction as i32)?;
                    let input=transform(&m,[one,zero,zero]);
                    direction=if dot(input,normal)>=zero{angle}else{opposite};
                }
            }
        }
        next.direction=Some(direction);
    }
    if next.magnitude.is_some(){next.magnitude=Some(magnitude);}
    *cache=next_cache;*outputs=next;Ok(true)
}
