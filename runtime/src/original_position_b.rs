//! Original second position parent 80043ed4..80045880.
use crate::{disc::Result,original_gameplay::{ENTITY_BYTES,CHARACTER_BYTES,angle_distance,angle_delta},
 original_math::{OriginalMath,IDENTITY,transform},original_geometry::{signed_angle,point_plane},
 original_terrain_candidates::{CandidateWorld,collect_triangles,sweep_objects},
 original_terrain_sweep::{TerrainObject,sweep},original_terrain_selection::{Selection,SelectionContext,select},
 original_terrain_response::{ResponseContext,respond},original_alternate_contact::{PositionContact,position},
 original_control::InputOutputs,original_contact::Memory,original_platform::PlatformMemory,
 original_position_a::{PositionState,PositionContext,platform}};
pub struct PositionBContext<'a>{
 pub common:&'a PositionContext,pub direction_mode:u32,pub special_offset:f32,pub special_radius:f32,
}
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn short(b:&[u8],i:usize)->u16{u16::from_be_bytes(b[i..i+2].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn vector(b:&[u8],i:usize)->[f32;3]{std::array::from_fn(|j|float(b,i+j*4))}
fn put(b:&mut[u8],i:usize,v:u32){b[i..i+4].copy_from_slice(&v.to_be_bytes());}
fn half(b:&mut[u8],i:usize,v:u16){b[i..i+2].copy_from_slice(&v.to_be_bytes());}
fn putv(b:&mut[u8],i:usize,v:[f32;3]){for j in 0..3{put(b,i+j*4,v[j].to_bits());}}
fn add(a:[f32;3],b:[f32;3])->[f32;3]{std::array::from_fn(|i|a[i]+b[i])}
fn sub(a:[f32;3],b:[f32;3])->[f32;3]{std::array::from_fn(|i|a[i]-b[i])}
fn add_at(b:&mut[u8],i:usize,v:[f32;3]){let old=vector(b,i);putv(b,i,add(old,v));}
fn dot(a:[f32;3],b:[f32;3])->f32{a[2].mul_add(b[2],a[1].mul_add(b[1],a[0]*b[0]))}
fn unit(v:[f32;3],math:&OriginalMath)->Result<[f32;3]>{math.estimated_vector(v)?.1.ok_or("position B zero normalization".into())}
fn orient(e:&[u8],v:[f32;3],normal:Option<[f32;3]>,math:&OriginalMath)->Result<[f32;3]>{
 let [pitch,roll]=if let Some(n)=normal{math.surface_angles(n)?}else{[word(e,8),word(e,0x10)]};
 let mut m=math.rotate(&IDENTITY,b'z',roll as i32)?;m=math.rotate(&m,b'x',pitch as i32)?;
 m=math.rotate(&m,b'y',(word(e,12)as i32).wrapping_neg())?;Ok(transform(&m,v))
}
fn offsets(s:&mut PositionState,normal:Option<[f32;3]>,ctx:&PositionBContext,math:&OriginalMath)->Result<([f32;3],[f32;3])>{
 let k=|o:u32|math.constant(0x803b6520-o);let zero=k(0x75f8);
 put(&mut s.workspace,0x34,word(&s.entity,8));put(&mut s.workspace,0x38,word(&s.entity,0x10));
 putv(&mut s.workspace,0x1c,orient(&s.entity,[zero,k(0x75c8),zero],normal,math)?);
 let mut local=[zero,k(0x75cc)*float(&s.character,0x134),zero];
 let halfheight=orient(&s.entity,local,normal,math)?;
 crate::original_offset::update(&s.motion,&s.character,vector(&s.workspace,0x1c),ctx.common.control.gravity,&mut local,math)?;
 Ok((orient(&s.entity,local,normal,math)?,halfheight))
}
fn environment(s:&mut PositionState,flags:u32,ctx:&PositionBContext,math:&OriginalMath)->Result<()>{crate::original_environment::update(&s.entity,&mut s.character,flags,ctx.common.control.level,math)}
fn input(s:&mut PositionState,magnitude:bool,ctx:&PositionBContext,math:&OriginalMath)->Result<(bool,Option<f32>)>{
 let mut out=InputOutputs{direction:None,magnitude:if magnitude{Some(math.constant(0x803b6520-0x75f8))}else{None}};
 let active=crate::original_control::read(&s.entity,&s.character,&ctx.common.control,&Memory{base:s.memory.base,bytes:&s.memory.bytes},&mut s.control_cache,&mut out,math)?;
 Ok((active,out.magnitude))
}
fn step(s:&mut PositionState,objects:&[TerrainObject],id:u32,flags:u32,special_mode:u32,ctx:&PositionBContext,math:&OriginalMath)->Result<bool>{
 let context=crate::original_terrain_step::StepContext{gravity:ctx.common.control.gravity,special_mode:special_mode!=0,special_offset:ctx.special_offset,special_radius:ctx.special_radius,surface_flags:flags,object_identifier:id};
 crate::original_terrain_step::step(&s.entity,&s.motion,&s.character,&mut s.workspace,objects,&mut s.sweep,&context,math)
}
fn alternate(s:&mut PositionState,previous:[f32;3],offset:[f32;3],halfheight:[f32;3],result:u32,flags:u32,ctx:&PositionBContext,math:&OriginalMath)->Result<u32>{
 let contact=PositionContact{previous_position:previous,offset:halfheight,alternate_offset:offset,previous_result:result,flags,stop_mode:s.collision_flag,direction_mode:ctx.direction_mode,level:ctx.common.control.level};
 position(&mut s.entity,&mut s.character,&mut s.workspace,&mut s.motion,&contact,math)
}
/// Global -78f8 is an output here: the original resets it and -78f4 on entry.
pub fn update<F>(state:&mut PositionState,special_mode:&mut u32,world:&CandidateWorld,ctx:&PositionBContext,math:&OriginalMath,mut callback:F)->Result<u32>
where F:FnMut(u32,u32,u32,u32,&mut PlatformMemory)->Result<()>{
 if state.entity.len()!=ENTITY_BYTES||state.character.len()!=CHARACTER_BYTES||state.motion.len()!=0x40||state.workspace.len()!=0x84{return Err("invalid position B state extent".into());}
 let mut next=state.clone();let mut mode=0;next.collision_flag=0;
 let result=inner(&mut next,&mut mode,world,ctx,math,&mut callback)?;*state=next;*special_mode=mode;Ok(result)
}
fn inner<F>(s:&mut PositionState,special_mode:&mut u32,world:&CandidateWorld,ctx:&PositionBContext,math:&OriginalMath,callback:&mut F)->Result<u32>
where F:FnMut(u32,u32,u32,u32,&mut PlatformMemory)->Result<()>{
 let k=|o:u32|math.constant(0x803b6520-o);let gravity=ctx.common.control.gravity;let zero=k(0x75f8);
 let previous_status=short(&s.character,0x1c)&3;half(&mut s.character,0x1c,short(&s.entity,4));let status=short(&s.entity,4);half(&mut s.entity,4,status&!1);
 let normal=if word(&s.character,0x8c)&0x100!=0{Some(vector(&s.character,0x7c))}else{None};
 let (mut offset,mut halfheight)=offsets(s,normal,ctx,math)?;
 putv(&mut s.workspace,4,add(vector(&s.entity,0x14),offset));put(&mut s.workspace,0,(k(0x75cc)*float(&s.character,0x134)).to_bits());
 putv(&mut s.workspace,0x10,add(vector(&s.motion,0),vector(&s.character,0x4c)));
 let previous_position=vector(&s.entity,0x14);
 collect_triangles(&s.workspace,world,&mut s.nodes,&mut s.candidates,false,math)?;
 let objects=sweep_objects(&s.nodes,world,&s.candidates)?;
 let mut list=Vec::<[u32;3]>::new();let mut chosen=None;let mut result=0;
 for record in s.candidates.objects[..s.candidates.object_count as usize*12].chunks_exact(12){
  let record=[word(record,0),word(record,4),word(record,8)];let [flags,id,attached]=record;
  if flags&0x2002!=0 || (short(&s.character,0x1c)&3==0 && flags&0x1000000!=0){continue;}
  if attached!=0{let entity=s.memory.word(attached.wrapping_add(0x34))?;if (s.memory.word(entity.wrapping_add(4))?>>16)&0x100==0{continue;}}
  if sweep(&mut s.workspace,&objects,id,0,false,&mut s.sweep,math)?{
   let ground=short(&s.character,0x1c)&3!=0;
   let kind=if ground{Selection::Alternate}else{Selection::Limit{incoming_f26:ctx.common.incoming_f26}};
   let context=SelectionContext{gravity,surface_flags:flags,descriptor_attached:attached!=0};
   let value=select(kind,&s.entity,&mut s.motion,&s.character,&mut s.workspace,&s.sweep.contacts,&mut s.collision_flag,&context,math)?;
   if if ground{value!=0}else{value==1}{chosen=Some(list.len());if ground && (result as i32)<value as i32{result=value;}}
  }
  list.push(record);if flags&0x100!=0{*special_mode=1;}if list.len()==31{break;}
 }
 if list.is_empty(){
  add_at(&mut s.entity,0x14,vector(&s.character,0x4c));add_at(&mut s.entity,0x14,vector(&s.motion,0));
  environment(s,0,ctx,math)?;put(&mut s.character,0xa0,0);put(&mut s.character,0xb0,0);return Ok(result);
 }
 let mut ordered=vec![None;list.len()];let mut cursor=0;
 if let Some(index)=chosen{ordered[0]=Some(list[index]);cursor=1;}
 let indexes:Vec<_>=if word(&s.character,0xa0)!=0{(0..list.len()).collect()}else{(0..list.len()).rev().collect()};
 for i in indexes{
  if Some(i)==chosen{continue;}
  if list[i][1]==word(&s.character,0xb0){let last=ordered.len()-1;ordered[last]=Some(list[i]);}
  else{if cursor>=ordered.len(){return Err("invalid position B candidate ordering".into());}ordered[cursor]=Some(list[i]);cursor+=1;}
 }
 let ordered:Vec<[u32;3]>=ordered.into_iter().collect::<Option<Vec<_>>>().ok_or("position B uninitialized source descriptor")?;
 putv(&mut s.workspace,4,add(vector(&s.entity,0x14),offset));putv(&mut s.workspace,0x10,add(vector(&s.motion,0),vector(&s.character,0x4c)));
 put(&mut s.character,0xa0,0);let mut landed=false;let mut last_id=0;
 for [flags,id,mut attached] in ordered{
  if !sweep(&mut s.workspace,&objects,id,0,false,&mut s.sweep,math)?{continue;}
  let mut checks=1_u32;
  if short(&s.entity,4)&1!=0 && last_id!=id{
   let displacement=if *special_mode==1{sub(add(vector(&s.workspace,0x54),vector(&s.workspace,0x6c)),vector(&s.entity,0x14))}
    else{sub(sub(add(vector(&s.workspace,0x60),vector(&s.workspace,0x6c)),halfheight),vector(&s.entity,0x14))};
   let (length,normalized)=math.estimated_vector(displacement)?;
   if length!=zero as f64{
    let direction=normalized.ok_or("position B displacement normalization")?;
    let incidence=if flags&0x100!=0 && direction[1]>zero{
     let angle=math.double_constant(0x803b6520-0x75e8)*(math.acos(direction[1])as f64);
     (if angle.is_nan(){i32::MIN}else{angle as i32})as f32
    }else{dot(direction,vector(&s.workspace,0x48))};
    if incidence<k(0x7568)||length>float(&s.character,0x134)as f64{checks=0;}
   }
  }
  checks|=4;
  let status=short(&s.entity,4);
  if status&3!=0 && status&0x2000==0{
   let mut displacement=sub(add(vector(&s.workspace,0x54),vector(&s.workspace,0x6c)),vector(&s.workspace,4));
   if *special_mode!=0{displacement[1]=zero;}
   if math.estimated_vector(displacement)?.0!=zero as f64{
    let direction=unit(displacement,math)?;
    let forward=orient(&s.entity,[if float(&s.character,0x64)>=zero{k(0x75c8)}else{k(0x75b0)},zero,zero],None,math)?;
    if !(*special_mode!=0 && forward[1].abs()==k(0x75c8)) && dot(direction,unit(forward,math)?)<k(0x74d0){checks&=!4;}
   }
  }
  checks|=8;
  if short(&s.character,0x1c)&3!=0 && short(&s.entity,4)&0x2000==0 && word(&s.character,0x8c)&0x100==0{
   let velocity=vector(&s.motion,0);
   if math.estimated_vector(velocity)?.0!=zero as f64{
    let velocity=unit(velocity,math)?;
    let displacement=add(offset,sub(add(vector(&s.workspace,0x54),vector(&s.workspace,0x6c)),vector(&s.workspace,4)));
    let length=math.estimated_vector(displacement)?.0;
    if length!=zero as f64 && length>k(0x75c8)as f64 && dot(unit(displacement,math)?,velocity)<zero{checks&=!8;}
   }
  }
  let mut angle=signed_angle(vector(&s.workspace,0x1c),vector(&s.workspace,0x48),None,math)? as i32;
  if angle>0x2c00{
   if dot(gravity,vector(&s.workspace,0x48))<zero{
    angle=signed_angle(gravity.map(|x|x*k(0x75b0)),vector(&s.workspace,0x48),None,math)?as i32;checks|=0x12;
   }else{
    step(s,&objects,id,flags,*special_mode,ctx,math)?;
    angle=signed_angle(vector(&s.workspace,0x1c),vector(&s.workspace,0x48),None,math)?as i32;checks|=2;
   }
  }else{
   let displacement=vector(&s.workspace,0x6c);let relative=sub(vector(&s.workspace,0x54),vector(&s.entity,0x14));
   if math.estimated_vector(displacement)?.0<=zero as f64 || (short(&s.character,0x1c)&3!=0 && dot(relative,displacement)>zero){checks|=2;}
  }
  // Ordered source rejection chain. Step attempts can update the contact data.
  let mut reject=short(&s.entity,4)&2!=0 || flags&0x401000!=0;
  if !reject{
   let direct=if angle<=0x2c00{
    let active=input(s,false,ctx,math)?.0;
    (active||dot(gravity,vector(&s.workspace,0x48))<zero||short(&s.character,0x1c)&3!=0)&&checks&5==5
   }else{false};
   if !direct && !(dot(gravity,vector(&s.workspace,0x1c))>dot(gravity,vector(&s.workspace,0x48)) || (flags&0x100!=0 && checks&5==5)){reject=true;}
  }
  if !reject{
   reject=(previous_status!=0&&short(&s.character,0x1c)&3==0)||checks&8==0||
    (dot(gravity,vector(&s.workspace,0x48))>zero&&short(&s.character,0x1c)&3==0);
  }
  if !reject && checks&2==0 && !step(s,&objects,id,flags,*special_mode,ctx,math)?{reject=true;}
  if !reject && math.estimated_vector(vector(&s.motion,0))?.0>=float(&s.character,0xe8)as f64 &&
   ((s.entity[0]==6)==(flags&0x100!=0)){
   if dot(unit(vector(&s.motion,0),math)?,vector(&s.workspace,0x48))>k(0x74cc)&&checks&0x10==0{reject=true;}
  }
  if !reject && float(&s.character,0x68)>zero&&short(&s.character,0x1c)&3!=0&&checks&0x10==0{reject=true;}
  if reject{result=alternate(s,previous_position,offset,halfheight,result,flags,ctx,math)?;continue;}
  landed=true;last_id=id;
  let value=respond(true,&mut s.entity,&mut s.motion,&mut s.character,&mut s.workspace,&s.sweep.contacts,&ResponseContext{gravity,surface_flags:flags,motion_identity:ctx.common.motion_identity},math)?;
  if value!=0&&(result==0||value==2){result=value;}
  let stationary=float(&s.motion,0)==zero&&float(&s.motion,8)==zero&&short(&s.character,0x1c)&1!=0&&
   dot(gravity,vector(&s.workspace,0x48))>=dot(gravity,vector(&s.workspace,0x1c));
  if flags&0x100!=0{put(&mut s.entity,8,0);put(&mut s.entity,0x10,0);}
  else if flags&0x1c!=0{
   if !stationary{crate::original_terrain_align::align(&mut s.entity,&s.character,&s.workspace,flags,gravity,math)?;}
  }else if checks&0x10!=0{
   if short(&s.character,0x1c)&1!=0 && angle_distance(word(&s.entity,0x10),word(&s.workspace,0x38))>0x4000{
    let yaw=angle_delta(0x8000,word(&s.entity,12))as u32;put(&mut s.entity,12,yaw);put(&mut s.motion,0x1c,yaw);
   }
   put(&mut s.entity,8,word(&s.workspace,0x34));put(&mut s.entity,0x10,word(&s.workspace,0x38));
  }else if !stationary{
   let pitch=word(&s.workspace,0x34);let roll=word(&s.workspace,0x38);
   if angle_distance(word(&s.entity,0x10),roll)>0x4000{put(&mut s.workspace,0x38,roll.wrapping_sub(0x8000));}
   if float(&s.workspace,0x4c)<=zero&&angle_distance(word(&s.workspace,0x38),0)<0x4000{put(&mut s.workspace,0x34,0x8000_u32.wrapping_sub(pitch));}
   if angle_distance(0xc800,word(&s.workspace,0x34))<0x1000&&angle_distance(0,word(&s.workspace,0x38))>0x7000{
    let p=0xffff8000_u32.wrapping_sub(word(&s.workspace,0x34));let r=0x8000_u32.wrapping_sub(word(&s.workspace,0x38));
    put(&mut s.workspace,0x34,p);put(&mut s.workspace,0x38,r);
    let yaw=angle_delta(0x8000,word(&s.entity,12))as u32;put(&mut s.entity,12,yaw);put(&mut s.motion,0x1c,yaw);
   }
   crate::original_terrain_align::align(&mut s.entity,&s.character,&s.workspace,flags,gravity,math)?;
  }
  if result==0{
   let normal=if flags&0x100!=0{Some(vector(&s.workspace,0x48))}else{None};(offset,halfheight)=offsets(s,normal,ctx,math)?;
  }
  put(&mut s.motion,0x18,word(&s.workspace,0x34));put(&mut s.motion,0x20,word(&s.workspace,0x38));putv(&mut s.character,0x7c,vector(&s.workspace,0x48));
  if flags&0x100!=0{
   let displacement=vector(&s.workspace,0x6c);
   if math.estimated_vector(displacement)?.0!=zero as f64{
    let speed=math.estimated_vector(vector(&s.workspace,0x10))?.0;let unit=unit(displacement,math)?;
    putv(&mut s.workspace,0x6c,unit.map(|x|((x as f64)*speed)as f32));
   }
  }
  putv(&mut s.workspace,0x10,[zero;3]);if s.collision_flag!=0{putv(&mut s.workspace,0x6c,[zero;3]);}
  let point=add(vector(&s.workspace,0x60),vector(&s.workspace,0x6c));let record=short(&s.workspace,0x2a)as usize;
  if record>=128{return Err("position B contact index outside storage".into());}
  let p=vector(&s.sweep.contacts,record*0x58+0x20);let n=vector(&s.workspace,0x1c);
  let projection=point_plane(point,[p[0],p[1],p[2],n[0],n[1],n[2]],true,math)?.1.ok_or("missing position B plane projection")?;
  putv(&mut s.workspace,4,add(halfheight,projection));
  if attached!=0{put(&mut s.character,0xa0,attached);if !platform(s,halfheight,ctx.common,math,callback)?{attached=0;}}
  else{put(&mut s.character,0xa0,0);}
  let new_position=sub(vector(&s.workspace,4),halfheight);let distance=crate::original_terrain_contacts::distance(vector(&s.entity,0x14),new_position,math)?;
  if ctx.direction_mode!=0||((attached!=0||float(&s.character,0x64)!=zero||float(&s.character,0x6c)!=zero||distance>k(0x75c8))&&distance>k(0x7538)){putv(&mut s.entity,0x14,new_position);}
  let speed=float(&s.character,0x64);let incline=dot(gravity,vector(&s.workspace,0x1c));
  let ground=speed>=k(0x74c8)||speed<=k(0x74c4)||incline<k(0x74c0)||flags&0x100!=0||
   ((speed>=k(0x74bc)||speed<=k(0x74b8))&&incline<k(0x74c0));
  if ground{let status=short(&s.entity,4);half(&mut s.entity,4,status|1);environment(s,flags,ctx,math)?;}
  else{
   environment(s,0,ctx,math)?;
   if float(&s.character,0x64)>zero{
    let direction=vector(&s.workspace,0x3c);let old=vector(&s.entity,0x14);
    putv(&mut s.entity,0x14,std::array::from_fn(|i|k(0x75f4).mul_add(direction[i],old[i])));
    for i in [0,2]{let value=k(0x75fc).mul_add(direction[i],float(&s.motion,i*4));put(&mut s.motion,i*4,value.to_bits());}
   }
  }
 }
 if !landed{
  let status=short(&s.character,0x1c);let mut retreat=false;
  if status&1!=0&&status&0x4000==0{
   let (active,magnitude)=input(s,true,ctx,math)?;
   retreat=(!active||(magnitude.unwrap()<=k(0x7548)&&short(&s.character,0x14)==0))&&float(&s.character,0x68)<=zero&&
    float(&s.character,0x64)<=float(&s.character,0xe8)&&word(&s.character,0x8c)&0x1c==0&&float(&s.character,0x34)>k(0x7534);
  }
  if retreat{
   let old=vector(&s.entity,0x14);let velocity=vector(&s.motion,0);putv(&mut s.entity,0x14,std::array::from_fn(|i|-k(0x75cc).mul_add(velocity[i],-old[i])));putv(&mut s.motion,0,[zero;3]);
  }else{add_at(&mut s.entity,0x14,vector(&s.workspace,0x10));}
  environment(s,0,ctx,math)?;
 }else{put(&mut s.character,0xb0,last_id);add_at(&mut s.entity,0x14,vector(&s.workspace,0x10));}
 Ok(result)
}
