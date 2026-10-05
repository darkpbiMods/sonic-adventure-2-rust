//! Animation choice in 8007fccc..8007fd40, after the caller's stop decision.
//! This does not classify contacts or replace the original caller conditions.
use crate::{disc::Result, original_math::OriginalMath};

/// Ordinary 800548e0 stop decision, excluding special posture/action branches.
/// On true the caller must apply idle presentation, then landing selection.
/// Records are unchanged on false or error.
pub fn ordinary_stop(entity: &mut [u8], character: &mut [u8], magnitudes: [f32;2]) -> Result<bool> {
    if entity.len()!=0x40 || character.len()!=0x3a0 { return Err("invalid stop records".into()); }
    let flags=u16::from_be_bytes(entity[4..6].try_into().unwrap());
    if flags&0x800!=0 || entity[0]==0x36 {return Err("special stop action requires separate adapter".into());}
    if crate::original_movement::input_active(entity,character,magnitudes)? {return Ok(false);}
    let speed=f32::from_be_bytes(character[0x64..0x68].try_into().unwrap());
    if !speed.is_finite() {return Err("nonfinite stop speed".into());}
    if speed!=0. {return Ok(false);}
    entity[0]=0;
    character[0x18..0x1a].fill(0);
    Ok(true)
}

pub fn select(velocity: [f32; 3], math: &OriginalMath) -> Result<u16> {
    let (length, _) = math.estimated_vector(velocity)?;
    Ok(if length < math.constant(0x803af56c) as f64 {
        1
    } else if length < math.constant(0x803af640) as f64 {
        16
    } else {
        17
    })
}

/// Stationary path of 8007fcac..8007fd40, including the preceding idle update.
/// Collision reconciliation and reverse-input braking precede this call.
/// False leaves records untouched; the caller then owns the running branch.
/// The hard-landing secondary effect is not dispatched here.
pub fn stationary_records(entity:&mut[u8],motion:&[u8],character:&mut[u8],magnitudes:[f32;2],
                          gravity:[f32;3],frame:u32,seed:&mut u32,context_draw:bool,math:&OriginalMath)->Result<Option<u16>>{
    stationary_impl(entity,motion,character,magnitudes,gravity,frame,seed,context_draw,true,math)
}
/// Rebound branch800806cc..80080780 retains the surface-idle counter update.
pub fn rebound_records(entity:&mut[u8],motion:&[u8],character:&mut[u8],magnitudes:[f32;2],
                       gravity:[f32;3],frame:u32,seed:&mut u32,context_draw:bool,math:&OriginalMath)->Result<Option<u16>>{
    stationary_impl(entity,motion,character,magnitudes,gravity,frame,seed,context_draw,false,math)
}
fn stationary_impl(entity:&mut[u8],motion:&[u8],character:&mut[u8],magnitudes:[f32;2],
                   gravity:[f32;3],frame:u32,seed:&mut u32,context_draw:bool,reset_counter:bool,math:&OriginalMath)->Result<Option<u16>>{
    if motion.len()!=0x40{return Err("invalid landing motion".into());}
    let(mut e,mut c,mut next_seed)=(entity.to_vec(),character.to_vec(),*seed);
    if !ordinary_stop(&mut e,&mut c,magnitudes)?{return Ok(None);}
    crate::original_idle_slope::advance(&e,motion,&mut c,gravity,frame,&mut next_seed,context_draw,math)?;
    e[8..12].copy_from_slice(&motion[0x18..0x1c]);
    e[0x10..0x14].copy_from_slice(&motion[0x20..0x24]);
    let velocity=std::array::from_fn(|i|f32::from_be_bytes(motion[i*4..i*4+4].try_into().unwrap()));
    let selected=select(velocity,math)?;
    c[0x176..0x178].copy_from_slice(&selected.to_be_bytes());
    if reset_counter{e[0]=0;c[0x18..0x1a].fill(0);}
    entity.copy_from_slice(&e);character.copy_from_slice(&c);*seed=next_seed;
    Ok(Some(selected))
}

/// Moving path8007fd44..fd6c / 80080758..80780 after a false stop decision.
pub fn moving_records(entity:&mut[u8],motion:&[u8],character:&mut[u8],historical_yaw:Option<u32>,math:&OriginalMath)->Result<()>{
    if entity.len()!=0x40||motion.len()!=0x40||character.len()!=0x3a0{return Err("invalid moving landing records".into());}
    let(mut e,mut c)=(entity.to_vec(),character.to_vec());
    e[8..12].copy_from_slice(&motion[0x18..0x1c]);e[0x10..0x14].copy_from_slice(&motion[0x20..0x24]);e[0]=1;
    crate::original_locomotion_selection::advance(&e,&mut c,historical_yaw,math)?;
    entity.copy_from_slice(&e);character.copy_from_slice(&c);Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moving_landing_whole_records_match_both_original_callers(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let math=OriginalMath::from_data_bundle(std::fs::read(root.join("playable/assets/original-math-data.bin")).unwrap()).unwrap();
        let bytes=|v:&serde_json::Value|v.as_array().unwrap().iter().map(|x|x.as_u64().unwrap()as u8).collect::<Vec<_>>();
        for name in ["stationary-moving-slope-landing.json","rebound-moving-slope-landing.json"]{
            let fixture:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification").join(name)).unwrap()).unwrap();
            let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),18);
            for row in rows{
                let mut e=bytes(&row["before"]["entity"]);let m=bytes(&row["before"]["motion"]);let mut c=bytes(&row["before"]["character"]);
                moving_records(&mut e,&m,&mut c,Some(row["yaw"].as_u64().unwrap()as u32),&math).unwrap();
                assert_eq!(e,bytes(&row["after"]["entity"]));assert_eq!(m,bytes(&row["after"]["motion"]));assert_eq!(c,bytes(&row["after"]["character"]));
            }
        }
    }
    #[test]
    fn rebound_slope_whole_records_match_original(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let math=OriginalMath::from_data_bundle(std::fs::read(root.join("playable/assets/original-math-data.bin")).unwrap()).unwrap();
        let fixture:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/rebound-slope-landing.json")).unwrap()).unwrap();
        let bytes=|v:&serde_json::Value|v.as_array().unwrap().iter().map(|x|x.as_u64().unwrap()as u8).collect::<Vec<_>>();
        let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),18);
        for row in rows{
            let mut e=bytes(&row["before"]["entity"]);let m=bytes(&row["before"]["motion"]);let mut c=bytes(&row["before"]["character"]);let mut seed=0;
            let selected=rebound_records(&mut e,&m,&mut c,[0.,0.],[0.,-1.,0.],0,&mut seed,false,&math).unwrap();
            assert_eq!(selected,Some(row["animation"].as_u64().unwrap()as u16));
            assert_eq!(e,bytes(&row["after"]["entity"]));assert_eq!(m,bytes(&row["after"]["motion"]));assert_eq!(c,bytes(&row["after"]["character"]));
            assert_eq!(i16::from_be_bytes(c[0x18..0x1a].try_into().unwrap()),1);
        }
    }
    #[test]
    fn stationary_slope_whole_records_match_original(){
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let math=OriginalMath::from_data_bundle(std::fs::read(root.join("playable/assets/original-math-data.bin")).unwrap()).unwrap();
        let fixture:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("movement-verification/stationary-slope-landing.json")).unwrap()).unwrap();
        let bytes=|v:&serde_json::Value|v.as_array().unwrap().iter().map(|x|x.as_u64().unwrap()as u8).collect::<Vec<_>>();
        let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),18);
        for row in rows{
            let mut e=bytes(&row["before"]["entity"]);let m=bytes(&row["before"]["motion"]);let mut c=bytes(&row["before"]["character"]);let mut seed=0;
            let selected=stationary_records(&mut e,&m,&mut c,[0.,0.],[0.,-1.,0.],0,&mut seed,false,&math).unwrap();
            assert_eq!(selected,Some(row["animation"].as_u64().unwrap()as u16));
            assert_eq!(e,bytes(&row["after"]["entity"]),"entity");
            assert_eq!(m,bytes(&row["after"]["motion"]),"motion");
            assert_eq!(c,bytes(&row["after"]["character"]),"character");
            let mut e=bytes(&row["before"]["entity"]);let mut c=bytes(&row["before"]["character"]);
            c[0x64..0x68].copy_from_slice(&1f32.to_be_bytes());
            let before=(e.clone(),c.clone(),seed);
            assert_eq!(stationary_records(&mut e,&m,&mut c,[0.,0.],[0.,-1.,0.],0,&mut seed,false,&math).unwrap(),None);
            assert_eq!((e.clone(),c.clone(),seed),before);
            c[0x64..0x68].fill(0);c[0x7c..0x80].copy_from_slice(&f32::NAN.to_be_bytes());
            let before=(e.clone(),c.clone(),seed);
            assert!(stationary_records(&mut e,&m,&mut c,[0.,0.],[0.,-1.,0.],0,&mut seed,false,&math).is_err());
            assert_eq!((e,c,seed),before);
        }
    }
    #[test]
    fn unsupported_stop_records_are_unchanged() {
        for (action, flags, speed) in [(0x36,1,0.),(10,0x801,0.),(10,1,f32::NAN)] {
            let mut e=[0u8;0x40];let mut c=[0u8;0x3a0];
            e[0]=action;e[4..6].copy_from_slice(&(flags as u16).to_be_bytes());
            c[0x64..0x68].copy_from_slice(&speed.to_be_bytes());c[0x18]=123;
            let before=(e,c);
            assert!(ordinary_stop(&mut e,&mut c,[0.,0.]).is_err());
            assert_eq!((e,c),before);
        }
    }
}
