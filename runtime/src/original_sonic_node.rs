//! Normal Sonic callback8007dc8c..8007e304, explicit mapped global addresses.
use crate::{disc::Result,original_contact::Memory,original_math::{OriginalMath,Matrix,transform},original_animation_matrix::translate};
pub struct Context{pub models:u32,pub character:u32,pub scratch:u32,pub players:u32,pub matrix:u32}
fn word(m:&[u8],base:u32,a:u32)->Result<u32>{Memory{base,bytes:m}.word(a)}
fn read_matrix(m:&[u8],base:u32,a:u32)->Result<Matrix>{let mut v=[[0.;4];3];for r in 0..3{for c in 0..4{v[r][c]=f32::from_bits(word(m,base,a+(r*4+c)as u32*4)?);}}Ok(v)}
fn put(m:&mut[u8],base:u32,a:u32,v:u32)->Result<()>{let i=a.checked_sub(base).ok_or("Sonic node write below mapping")?as usize;m.get_mut(i..i+4).ok_or("Sonic node write outside mapping")?.copy_from_slice(&v.to_be_bytes());Ok(())}
fn write_vector(m:&mut[u8],base:u32,a:u32,v:[f32;3])->Result<()>{for(i,x)in v.iter().enumerate(){put(m,base,a+i as u32*4,x.to_bits())?;}Ok(())}
fn write_matrix(m:&mut[u8],base:u32,a:u32,v:&Matrix)->Result<()>{for r in 0..3{for c in 0..4{put(m,base,a+(r*4+c)as u32*4,v[r][c].to_bits())?;}}Ok(())}
fn hit(m:&[u8],base:u32,table:u32,node:u32,offsets:[u32;3])->Result<bool>{for o in offsets{if word(m,base,table+o)?==node{return Ok(true);}}Ok(false)}
fn point(mut matrix:Matrix,v:[f32;3])->[f32;3]{translate(&mut matrix,v);matrix.map(|r|r[3])}
fn record(m:&mut[u8],base:u32,c:&Context,matrix:&Matrix,position:u32,directions:&[(u32,[f32;3])])->Result<()>{
 let secondary=read_matrix(m,base,c.scratch+0x3c)?;let p=point(*matrix,[0.;3]);write_vector(m,base,c.character+position,point(secondary,p))?;
 for(offset,v)in directions{write_vector(m,base,c.character+offset,transform(&secondary,transform(matrix,*v)))?;}Ok(())
}
pub fn apply(base:u32,memory:&mut[u8],node:u32,c:&Context,math:&OriginalMath)->Result<()>{
 let mut staged=memory.to_vec();apply_in_place(base,&mut staged,node,c,math)?;memory.copy_from_slice(&staged);Ok(())
}
/// Caller must stage memory if rollback is required.
pub(crate) fn apply_in_place(base:u32,memory:&mut[u8],node:u32,c:&Context,math:&OriginalMath)->Result<()>{
 let mut m=memory;let mut matrix=read_matrix(&m,base,c.matrix)?;
 let head=hit(&m,base,c.models,node,[0xc,0x104,0x18c])?;
 if head||word(&m,base,word(&m,base,c.models+0x34)?+0x2c)?==node{record(&mut m,base,c,&matrix,0x230,&[(0x2cc,[1.,0.,0.])])?;}
 else{
  let groups=[[0x14,0x10c,0x194],[0x8c,0x164,0x1ec],[0x94,0x16c,0x1f4],[0x1c,0x114,0x19c],[0x44,0x12c,0x1b4],[0x7c,0x154,0x1dc],[0x84,0x15c,0x1e4],[0x4c,0x134,0x1bc],[0x54,0x13c,0x1c4]];
  for(index,group)in groups.into_iter().enumerate(){if !hit(&m,base,c.models,node,group)?{continue;}
   if index<3{
    let player=Memory{base,bytes:&m}.byte(c.character)?as i8 as i32;let entity=word(&m,base,c.players.wrapping_add(player.wrapping_mul(4)as u32))?;
    let angle=if index==0{if (Memory{base,bytes:&m}).byte(entity)?==1{Some(word(&m,base,c.character+0x2c)?as i32)}else{None}}
     else if (Memory{base,bytes:&m}).half(entity+4)?&0x800!=0{let object=word(&m,base,c.character+0x94)?;let body=word(&m,base,object+0x38)?;let angle=word(&m,base,body+0x30)?as i32;Some(if index==2{angle.wrapping_neg()}else{angle})}else{None};
    if let Some(angle)=angle{matrix=math.rotate(&matrix,b'x',angle)?;}
   }else{match index{
    3=>record(&mut m,base,c,&matrix,0x218,&[(0x2b4,[1.,0.,0.])])?,
    4=>record(&mut m,base,c,&matrix,0x224,&[(0x2c0,[1.,0.,0.])])?,
    5=>record(&mut m,base,c,&matrix,0x1e8,&[(0x26c,[0.,0.,1.]),(0x284,[0.,-1.,0.])])?,
    6=>record(&mut m,base,c,&matrix,0x1f4,&[(0x278,[0.,0.,-1.]),(0x290,[0.,-1.,0.])])?,
    7=>record(&mut m,base,c,&matrix,0x200,&[(0x29c,[-1.,0.,0.])])?,
    8=>record(&mut m,base,c,&matrix,0x20c,&[(0x2a8,[-1.,0.,0.])])?,_=>unreachable!()}
   }break;
  }
 }
 for(a,b,oa,ob)in [([0xbc,0x174,0x1fc],[0xc4,0x17c,0x204],0x9c,0xcc),([0x5c,0x144,0x1cc],[0x64,0x14c,0x1d4],0xfc,0x1bc)]{if hit(&m,base,c.models,node,a)?{write_matrix(&mut m,base,c.scratch+oa,&matrix)?;}else if hit(&m,base,c.models,node,b)?{write_matrix(&mut m,base,c.scratch+ob,&matrix)?;}}
 write_matrix(&mut m,base,c.matrix,&matrix)?;Ok(())
}
