//! Effect factories8008ac80..8008ada8 with their real flags-zero task path.
use crate::{disc::Result,original_contact::Memory,original_math::OriginalMath,
 original_heap::{self,HeapContext},original_effect_state::{self,EffectContext}};
fn read(m:&[u8],base:u32,a:u32)->Result<u32>{Memory{base,bytes:m}.word(a)}
fn bytes(m:&mut[u8],base:u32,a:u32,value:&[u8])->Result<()>{
 let at=a.checked_sub(base).ok_or("effect factory address below memory")?as usize;
 let end=at.checked_add(value.len()).ok_or("effect factory extent overflow")?;
 m.get_mut(at..end).ok_or("effect factory write outside memory")?.copy_from_slice(value);Ok(())
}
fn put(m:&mut[u8],base:u32,a:u32,value:u32)->Result<()>{bytes(m,base,a,&value.to_be_bytes())}
/// Returned pointer is host metadata: these original factory routines are void.
/// The only destructor path here is the freshly-created task whose data malloc
/// failed; it has no children, destructor callback or attached allocations.
pub fn spawn(player:u8,fading:bool,base:u32,memory:&mut[u8],heap:&HeapContext,index:i32,layers:&mut[u32;8],active_destructor:&mut u32,effects:&EffectContext,seed:&mut u32,math:&OriginalMath)->Result<u32>{
 let(mut mem,mut next_layers,mut next_seed,mut destructor)=(memory.to_vec(),*layers,*seed,*active_destructor);
 let task=original_heap::allocate(base,&mut mem,heap,index,0x50)?;
 if task==0{return Ok(0);}
 // Full80071d04 constructor path selected by flags0/layer3. Bytes44/48
 // intentionally preserve and move the previous allocation's word44.
 put(&mut mem,base,task.wrapping_add(0x10),if fading{0x8008b06c}else{0x8008b000})?;
 for offset in [0x30,0x34,0x38,0x3c,0x40,0x14,0x18,0x1c,0x20,0x24,0x28,0x2c,0xc,8]{put(&mut mem,base,task.wrapping_add(offset),0)?;}
 let old=read(&mem,base,task.wrapping_add(0x44))?;put(&mut mem,base,task.wrapping_add(0x48),old)?;
 put(&mut mem,base,task.wrapping_add(0x44),0x803b6520-0x6da0)?;put(&mut mem,base,task.wrapping_add(0x4c),0)?;
 let head=next_layers[3];
 if head==0{put(&mut mem,base,task,task)?;put(&mut mem,base,task.wrapping_add(4),task)?;next_layers[3]=task;}
 else{
  let tail=read(&mem,base,head.wrapping_add(4))?;
  put(&mut mem,base,task.wrapping_add(4),tail)?;put(&mut mem,base,task,head)?;
  put(&mut mem,base,head.wrapping_add(4),task)?;put(&mut mem,base,tail,task)?;
 }
 let data=original_heap::allocate(base,&mut mem,heap,index,0x20)?;
 let result=if data==0{
  //80071af0's actual fresh-task branch; no callback or attached data exists.
  destructor=0;
  for offset in [0x10,0x14,0x18,0x1c,0x20,0x24,0x28,0x2c]{put(&mut mem,base,task.wrapping_add(offset),0)?;}
  let next=read(&mem,base,task)?;let previous=read(&mem,base,task.wrapping_add(4))?;
  put(&mut mem,base,next.wrapping_add(4),previous)?;put(&mut mem,base,previous,next)?;
  let replacement=if task==next{0}else{next};
  if let Some(layer)=next_layers.iter_mut().find(|v|**v==task){*layer=replacement;}
  for offset in [0xc,8,4,0]{put(&mut mem,base,task.wrapping_add(offset),0)?;}
  original_heap::free(base,&mut mem,heap,index,task)?;0
 }else{
  put(&mut mem,base,task.wrapping_add(0x40),data)?;bytes(&mut mem,base,data.wrapping_add(1),&[player])?;
  original_effect_state::initialize(task,base,&mut mem,effects,&mut next_seed,math)?;
  if !fading{bytes(&mut mem,base,data.wrapping_add(2),&0u16.to_be_bytes())?;}task
 };
 memory.copy_from_slice(&mem);*layers=next_layers;*seed=next_seed;*active_destructor=destructor;Ok(result)
}
