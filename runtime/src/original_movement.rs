//! Original movement routines recovered from GSNE8P rev0.
//! This module is under reconstruction; it is not the complete action dispatcher.
use crate::{disc::Result,original_gameplay::{angle_distance,angle_approach,ENTITY_BYTES,CHARACTER_BYTES},original_math::{OriginalMath,IDENTITY,transform}};
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
fn short(b:&[u8],i:usize)->u16{u16::from_be_bytes(b[i..i+2].try_into().unwrap())}
fn write_word(b:&mut[u8],i:usize,value:u32){b[i..i+4].copy_from_slice(&value.to_be_bytes());}
/// Original 0x8004e560 with both optional output pointers null. This path does
/// not invoke its camera/surface direction dependencies.
pub(crate) fn input_active(entity:&[u8],character:&[u8],magnitudes:[f32;2])->Result<bool>{
    let status=short(entity,4);
    if status&0x4000!=0{return Ok(false);}
    let magnitude=*magnitudes.get(character[0]as usize).ok_or("unsupported original player slot")?;
    if !magnitude.is_finite(){return Err("nonfinite controller magnitude outside verified domain".into());}
    Ok(magnitude!=0. && (status&0x2000!=0 || short(character,0x14)==0))
}
/// Complete steering routine 0x80047714..0x80047a1c, including its ground and
/// airborne velocity blends. Original global controller magnitudes are explicit
/// inputs; matrix-stack scratch storage is represented by local matrices.
pub fn steer(entity:&mut[u8],character:&mut[u8],target:u32,magnitudes:[f32;2],math:&OriginalMath)->Result<()>{
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES{return Err("unsupported movement state size".into());}
    let active=input_active(entity,character,magnitudes)?;
    let velocity=[float(character,0x64),float(character,0x68),float(character,0x6c)];
    let grip=float(character,0x40);let surface=float(character,0x34);
    if velocity.iter().any(|v|!v.is_finite()) || !grip.is_finite() || !surface.is_finite(){return Err("nonfinite movement state outside verified domain".into());}
    let x=word(entity,8)as i32;let y=word(entity,12)as i32;let z=word(entity,16)as i32;
    let mut m=math.rotate(&IDENTITY,b'z',z)?;
    m=math.rotate(&m,b'x',x)?;m=math.rotate(&m,b'y',y.wrapping_neg())?;
    let world=transform(&m,velocity);
    let distance=angle_distance(y as u32,target);
    let step=if distance>0x2000 {0x800}else if distance>0x1000 {distance>>2}else{distance>>3};
    let yaw=angle_approach(y as u32,target,step as i32);
    let mut m=math.rotate(&IDENTITY,b'y',yaw as i32)?;
    m=math.rotate(&m,b'x',x.wrapping_neg())?;m=math.rotate(&m,b'z',z.wrapping_neg())?;
    let turned=transform(&m,world);
    let grounded=short(entity,4)&3!=0;
    let (old_weight,new_weight)=if !grounded {
        (math.constant(0x803aef78),math.constant(0x803aefd8))
    }else{
        let base=if !active {math.constant(0x803af034)}
            else if surface>math.constant(0x803af008){math.constant(0x803af088)}
            else{math.constant(0x803aef54)};
        let weight=if grip<math.constant(0x803aef58){base*grip}else{base};
        (weight,math.constant(0x803aef58)-weight)
    };
    // Store only after fallible validation/rotation, avoiding partial host state.
    write_word(entity,12,yaw);
    for i in 0..3 {let v=velocity[i].mul_add(old_weight,turned[i]*new_weight);write_word(character,0x64+i*4,v.to_bits());}
    Ok(())
}

/// Original global inputs made explicit for the host. Contact type 0x11 uses a
/// separate surface-relative direction path, which is not translated yet.
pub struct AirContext {
    pub directions:[u32;2],pub magnitudes:[f32;2],pub gravity:[f32;3],
    pub level:i16,pub control_mode:u32,pub has_direction_surface:bool,
}
fn ordinary_input(entity:&[u8],character:&[u8],ctx:&AirContext,math:&OriginalMath)->Result<Option<(u32,f32)>>{
    if !input_active(entity,character,ctx.magnitudes)? {return Ok(None);}
    let player=character[0]as usize;let mut direction=ctx.directions[player];
    if short(character,0x1a)as i16>0 {
        direction=match ctx.control_mode&0x18 {
            0=>direction.wrapping_add(0x4000),
            8=>0x8000_u32.wrapping_sub(direction),
            16=>direction.wrapping_add(0x8000),
            _=>0x4000_u32.wrapping_sub(direction),
        };
    }
    if ctx.level!=0x2c && ctx.level!=0x32 {
        if ctx.has_direction_surface{return Err("surface-relative input direction dependency not translated".into());}
        if ctx.gravity[1]>math.constant(0x803aef78){direction=0x8000_u32.wrapping_sub(direction);}
    }
    Ok(Some((direction,ctx.magnitudes[player])))
}
/// Entire 0x8004b330 acceleration routine, with the no-direction-surface path
/// of its input dependency. Includes gravity transform, axis drag, braking,
/// conditional steering and doubled input acceleration while descending.
/// It computes acceleration; velocity integration/collision are separate calls.
pub fn air_acceleration(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],ctx:&AirContext,math:&OriginalMath)->Result<()>{
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES || motion.len()<0x24{return Err("unsupported air movement state size".into());}
    let input=ordinary_input(entity,character,ctx,math)?;
    for off in [0x64,0x68,0x6c,0xe8,0x100,0x10c,0x110,0x118,0x11c,0x138] {
        if !float(character,off).is_finite(){return Err("nonfinite air parameter outside verified domain".into());}
    }
    if ctx.gravity.iter().any(|v|!v.is_finite()) || [4,12,16,20].iter().any(|&i|!float(motion,i).is_finite()){return Err("nonfinite external force outside verified domain".into());}
    let gravity_weight=float(character,0x138);
    let force=std::array::from_fn(|i|ctx.gravity[i].mul_add(gravity_weight,float(motion,12+i*4)));
    let mut m=math.rotate(&IDENTITY,b'y',word(entity,12)as i32)?;
    m=math.rotate(&m,b'x',(word(entity,8)as i32).wrapping_neg())?;
    m=math.rotate(&m,b'z',(word(entity,16)as i32).wrapping_neg())?;
    let mut acceleration=transform(&m,force);
    for (i,offset)in [0x110,0x118,0x11c].into_iter().enumerate(){acceleration[i]=float(character,offset).mul_add(float(character,0x64+i*4),acceleration[i]);}
    let mut e=entity.to_vec();let mut c=character.to_vec();let mut target=word(motion,0x1c);
    let addition=if let Some((direction,magnitude))=input {
        let distance=angle_distance(word(entity,12),direction);
        let addition=if float(character,0x64)>float(character,0xe8) && distance>0x6000 {
            // The original deliberately keeps the PREVIOUS target on this path.
            float(character,0x10c)*magnitude
        }else{
            target=direction;
            if distance>0x1000 {0.}else{
                let v=float(character,0x100)*magnitude;
                if float(motion,4)<0. {math.constant(0x803aef24)*v}else{v}
            }
        };
        steer(&mut e,&mut c,target,ctx.magnitudes,math)?;
        addition
    }else{target=word(entity,12);0.};
    acceleration[0]+=addition;
    for i in 0..3{write_word(&mut c,0x58+i*4,acceleration[i].to_bits());}
    entity.copy_from_slice(&e);character.copy_from_slice(&c);write_word(motion,0x1c,target);
    Ok(())
}

/// Complete velocity integrator 0x80046de4..0x80047220, including the special
/// direction-vector orientation path and original 0x80046950 speed limits.
/// World-space velocity is written to motion[0..12]; this does not move position.
pub fn integrate_velocity(entity:&[u8],motion:&mut[u8],character:&mut[u8],magnitudes:[f32;2],math:&OriginalMath)->Result<()>{
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES || motion.len()<12{return Err("unsupported integration state size".into());}
    let active=input_active(entity,character,magnitudes)?;
    for offset in [0x34,0x58,0x5c,0x60,0x64,0x68,0x6c,0xc8,0xcc,0xe4,0xe8,0x108,0x1b0] {
        if !float(character,offset).is_finite(){return Err("nonfinite velocity parameter outside verified domain".into());}
    }
    if !float(entity,0x18).is_finite() || !float(motion,4).is_finite(){return Err("nonfinite integration state outside verified domain".into());}
    let old=[float(character,0x64),float(character,0x68),float(character,0x6c)];
    let mut speed=std::array::from_fn::<_,3,_>(|i|old[i]+float(character,0x58+i*4));
    let threshold=math.constant(0x803af084)*float(character,0xe4);
    let surface=float(character,0x34);
    if old[0]>0. {
        if speed[0]<0. || (!active && speed[0]<threshold && surface>math.constant(0x803aefec)){speed[0]=0.;}
    }else if old[0]<0. {
        if speed[0]>math.constant(0x803aef7c) || (!active && speed[0]>-threshold && surface>math.constant(0x803aefec)){speed[0]=0.;}
    }
    if old[2]>0. {
        if (old[2]<threshold && old[2]==speed[2]) || (old[2]>=threshold && speed[2]<threshold){speed[2]=0.;}
    }else if old[2]<0. {
        if (old[2]>-threshold && old[2]==speed[2]) || (old[2]<=-threshold && speed[2]>-threshold){speed[2]=0.;}
    }
    let flags=word(character,0x1a8);
    let above=float(character,0x1b0)>math.constant(0x803af080)+float(entity,0x18);
    if flags&0x2000!=0 && above {
        if speed[0]>=math.constant(0x803aefc0)*float(character,0xe4){speed[0]+=float(character,0x108);}
    }else if flags&0x2002!=0 && above {
        if speed[0]>=math.constant(0x803aef54)*(float(character,0xe8)+float(character,0xe4)){speed[0]+=float(character,0x108);}
    }
    let special=word(character,0x88)&0x100!=0 && (entity[0]!=6 || float(motion,4)>0.);
    let [pitch,roll]=if special {
        math.surface_angles([float(character,0x7c),float(character,0x80),float(character,0x84)])?
    }else{[word(entity,8),word(entity,16)]};
    let mut m=math.rotate(&IDENTITY,b'z',roll as i32)?;
    m=math.rotate(&m,b'x',pitch as i32)?;
    m=math.rotate(&m,b'y',(word(entity,12)as i32).wrapping_neg())?;
    let mut world=transform(&m,speed);
    // Both status-bit branches in 0x80046950 perform these same ordered clamps.
    for i in [0,2,1] {
        let cap=float(character,if i==1 {0xcc}else{0xc8});
        if cap<world[i]{world[i]=cap;}
        if -cap>world[i]{world[i]=-cap;}
    }
    if !active {
        for value in &mut world {
            if math.constant(0x803af078)>*value && math.constant(0x803af07c)<*value {*value=0.;}
        }
    }
    if speed.iter().chain(world.iter()).any(|x|!x.is_finite()){return Err("nonfinite velocity result outside verified domain".into());}
    for i in 0..3 {write_word(character,0x64+i*4,speed[i].to_bits());write_word(motion,i*4,world[i].to_bits());}
    Ok(())
}

/// Complete 0x8003e668..0x8003e9e0 reconciliation after collision handling.
/// Converts world speed to local speed, applies original small-speed rules,
/// wraps orientation words and updates the slope/gravity relationship.
pub fn reconcile_velocity(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],gravity:[f32;3],level:i16,math:&OriginalMath)->Result<()>{
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES || motion.len()<0x24{return Err("unsupported reconciliation state size".into());}
    let world=[float(motion,0),float(motion,4),float(motion,8)];
    let normal=[float(character,0x7c),float(character,0x80),float(character,0x84)];
    if world.iter().chain(normal.iter()).chain(gravity.iter()).any(|x|!x.is_finite()) || !float(character,0xfc).is_finite() || !float(character,0x138).is_finite(){return Err("nonfinite reconciliation input outside verified domain".into());}
    let grounded=short(entity,4)&3!=0;let flags=word(character,0x88);
    let [pitch,roll]=if grounded && flags&0x100!=0 {math.surface_angles(normal)?}else{[word(entity,8),word(entity,16)]};
    let mut m=math.rotate(&IDENTITY,b'y',word(entity,12)as i32)?;
    m=math.rotate(&m,b'x',(pitch as i32).wrapping_neg())?;
    m=math.rotate(&m,b'z',(roll as i32).wrapping_neg())?;
    let mut local=transform(&m,world);
    if grounded {
        local[1]=0.;
        if word(character,0x8c)&0xc==0 {
            if local[0].abs()<math.constant(0x803b6520-0x75b4){local[0]=0.;}
            if local[2].abs()<=math.constant(0x803aefd8)*float(character,0xfc){local[2]=0.;}
        }
    }else{
        for i in [0,2] {if local[i].abs()<=math.constant(0x803b6520-0x758c){local[i]=0.;}}
    }
    let mut slope=if grounded {
        let dot=normal[0]*gravity[0];
        let dot=gravity[1].mul_add(normal[1],dot);
        -(gravity[2].mul_add(normal[2],dot))
    }else{math.constant(0x803aef58)};
    if !slope.is_finite() || local.iter().any(|x|!x.is_finite()){return Err("nonfinite reconciliation result outside verified domain".into());}
    if level==0x5a {
        let tangent=math.sqrt_refined(math.constant(0x803aef58)-slope*slope);
        let weight=float(character,0x138);
        let mut minimum=math.constant(0x803aef78)*weight;
        if local[0].abs()<=weight*tangent && slope>math.constant(0x803b6520-0x7544){minimum=math.constant(0x803b6520-0x759c)*(weight*slope);}
        if local[0].abs()<minimum {local[0]=0.;}
    }
    if flags&0x100!=0 {slope=math.constant(0x803aef58);}
    for i in 0..3 {write_word(character,0x64+i*4,local[i].to_bits());}
    write_word(character,0x34,slope.to_bits());
    for i in [8,12,16] {write_word(entity,i,word(entity,i)&0xffff);}
    for i in [0x18,0x1c,0x20] {write_word(motion,i,word(motion,i)&0xffff);}
    Ok(())
}

/// Complete 0x80047a1c..0x80047cbc: approach the global gravity orientation,
/// retaining world velocity and the character's half-height center position.
pub fn align_airborne(entity:&mut[u8],character:&mut[u8],target_pitch:u32,target_roll:u32,math:&OriginalMath)->Result<()>{
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES{return Err("unsupported airborne alignment size".into());}
    if short(entity,4)&3!=0{return Ok(());}
    let speed=[float(character,0x64),float(character,0x68),float(character,0x6c)];
    let position=[float(entity,0x14),float(entity,0x18),float(entity,0x1c)];
    let dash=float(character,0xf4);let height=float(character,0x134);
    if speed.iter().chain(position.iter()).any(|v|!v.is_finite()) || !dash.is_finite() || !height.is_finite(){return Err("nonfinite airborne alignment input".into());}
    if word(character,0x24)&0x8000==0 {
        let sum=(speed[0]*speed[0])+(speed[1]*speed[1]);
        if (speed[2]*speed[2])+sum>dash*dash{return Ok(());}
    }
    let pitch=word(entity,8);let yaw=word(entity,12);let roll=word(entity,16);
    let forward=|p:u32,r:u32|->Result<_>{
        let m=math.rotate(&IDENTITY,b'z',r as i32)?;
        let m=math.rotate(&m,b'x',p as i32)?;
        math.rotate(&m,b'y',(yaw as i32).wrapping_neg())
    };
    let old_matrix=forward(pitch,roll)?;
    let world=transform(&old_matrix,speed);
    let offset=[0.,math.constant(0x803aef54)*height,0.];
    let center_offset=transform(&old_matrix,offset);
    let center=std::array::from_fn::<_,3,_>(|i|center_offset[i]+position[i]);
    let new_pitch=angle_approach(pitch,target_pitch,0x800);
    let new_roll=angle_approach(roll,target_roll,0x800);
    let m=math.rotate(&IDENTITY,b'y',yaw as i32)?;
    let m=math.rotate(&m,b'x',(new_pitch as i32).wrapping_neg())?;
    let m=math.rotate(&m,b'z',(new_roll as i32).wrapping_neg())?;
    let local=transform(&m,world);
    let new_offset=transform(&forward(new_pitch,new_roll)?,offset);
    let new_position=std::array::from_fn::<_,3,_>(|i|center[i]-new_offset[i]);
    if local.iter().chain(new_position.iter()).any(|v|!v.is_finite()){return Err("nonfinite airborne alignment result".into());}
    write_word(entity,8,new_pitch);write_word(entity,16,new_roll);
    for i in 0..3{write_word(character,0x64+i*4,local[i].to_bits());write_word(entity,0x14+i*4,new_position[i].to_bits());}
    Ok(())
}

/// Complete 0x80048150..0x80048370. Used by ground acceleration for
/// downhill steering with no stick input; retains the original stage exception.
pub fn steer_slope(entity:&mut[u8],motion:&mut[u8],character:&mut[u8],gravity:[f32;3],level:i16,magnitudes:[f32;2],math:&OriginalMath)->Result<()>{
    if entity.len()!=ENTITY_BYTES || character.len()!=CHARACTER_BYTES || motion.len()<0x24{return Err("unsupported slope state size".into());}
    if short(character,0x14)!=0 || word(character,0x88)&0x100!=0{return Ok(());}
    let velocity=[float(motion,0),float(motion,4),float(motion,8)];
    if velocity.iter().chain(gravity.iter()).any(|x|!x.is_finite()) || !float(character,0xe4).is_finite(){return Err("nonfinite slope state".into());}
    let (length,unit)=math.estimated_vector(velocity)?;
    if length>float(character,0xe4)as f64 {
        let unit=unit.ok_or("undefined normalization outside movement domain")?;
        let dot=gravity[2].mul_add(unit[2],gravity[1].mul_add(unit[1],unit[0]*gravity[0]));
        if (dot as f64)<math.double_constant(0x803b6520-0x7488){return Ok(());}
    }
    let yaw=word(entity,12);
    let m=math.rotate(&IDENTITY,b'y',yaw as i32)?;
    let m=math.rotate(&m,b'x',(word(entity,8)as i32).wrapping_neg())?;
    let m=math.rotate(&m,b'z',(word(entity,16)as i32).wrapping_neg())?;
    let [x,y,z]=transform(&m,gravity);
    let c=|offset:u32|math.constant(0x803b6520-offset);
    let target=if y<0. && (y>c(0x7480) || (y>c(0x746c) && level==0x22)) {
        // Branchwise negation retains the original signed-zero behavior.
        let positive_x=if x<0.{-x}else{x};
        let direction=yaw.wrapping_add(math.angle_from_atan2(z,positive_x)as u32);
        let positive_z=if z<0.{-z}else{z};
        let rate=if short(entity,4)&0x100!=0 {c(0x7474)}else{c(0x7470)};
        angle_approach(word(motion,0x1c),direction,(rate*positive_z)as i32)
    }else{yaw};
    // Stage mutations until the downstream steering routine accepts the domain.
    let mut next_entity=entity.to_vec();let mut next_character=character.to_vec();
    steer(&mut next_entity,&mut next_character,target,magnitudes,math)?;
    entity.copy_from_slice(&next_entity);character.copy_from_slice(&next_character);
    write_word(motion,0x1c,target);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source_math()->OriginalMath{
        let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../source/main.dol");
        OriginalMath::from_dol(std::fs::read(path).expect("verified local SA2 DOL required for movement tests")).unwrap()
    }
    #[test]
    fn integration_rejection_preserves_state(){
        let math=source_math();let mut e=[0;ENTITY_BYTES];let mut c=[0;CHARACTER_BYTES];let mut d=[0;0x40];
        write_word(&mut c,0x64,f32::NAN.to_bits());let old=c;
        assert!(integrate_velocity(&e,&mut d,&mut c,[0.,0.],&math).is_err());assert_eq!(c,old);assert_eq!(d,[0;0x40]);
        c=[0;CHARACTER_BYTES];write_word(&mut e,16,i32::MAX as u32);
        // Signed32-bit angles are supported; finite acceleration+speed overflow
        // must still reject transactionally after the arithmetic path executes.
        write_word(&mut c,0x64,f32::MAX.to_bits());write_word(&mut c,0x58,f32::MAX.to_bits());let old=c;
        assert!(integrate_velocity(&e,&mut d,&mut c,[0.,0.],&math).is_err());assert_eq!(c,old);assert_eq!(d,[0;0x40]);
    }
    #[test]
    fn reconciliation_rejection_preserves_state(){
        let math=source_math();let mut e=[0;ENTITY_BYTES];let mut c=[0;CHARACTER_BYTES];let mut d=[0;0x40];
        write_word(&mut c,0x80,f32::INFINITY.to_bits());let old=c;
        assert!(reconcile_velocity(&mut e,&mut d,&mut c,[0.,-1.,0.],0,&math).is_err());assert_eq!(c,old);assert_eq!(e,[0;ENTITY_BYTES]);assert_eq!(d,[0;0x40]);
    }
    #[test]
    fn unsupported_surface_input_is_not_approximated(){
        let math=source_math();let mut e=[0;ENTITY_BYTES];let mut c=[0;CHARACTER_BYTES];let mut d=[0;0x40];
        let ctx=AirContext{directions:[0,0],magnitudes:[1.,0.],gravity:[0.,-1.,0.],level:0,control_mode:0,has_direction_surface:true};
        assert!(air_acceleration(&mut e,&mut d,&mut c,&ctx,&math).is_err());assert_eq!(c,[0;CHARACTER_BYTES]);assert_eq!(e,[0;ENTITY_BYTES]);assert_eq!(d,[0;0x40]);
    }
}
