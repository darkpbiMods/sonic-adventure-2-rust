//! Verified-build data for the future Sonic runtime. This module loads original
//! parameters; it does not yet implement movement or animation progression.
//! Physics table: 0x80196578, copied by 0x8004f8b8..0x8004f8e4.
//! Animation table: 0x801b762c, installed by 0x8008758c..0x80087594.
//! Field interpretation cross-reference: X-Hax SA2Structs.h / GC_SA2B boot.ini.
use serde::Serialize;
use crate::disc::Result;
#[derive(Serialize)]
pub struct Physics {
    pub source_address:u32,
    pub hang_time:i32,
    // IEEE-754 bits are retained exactly, including any signed zero.
    pub parameter_bits:[u32;32],
}
pub const PHYSICS_NAMES:[&str;32]=[
    "floor_grip","speed_cap_h","speed_cap_v","speed_max_h","push_speed_max",
    "jump_speed","no_control_speed","slide_speed","jog_speed","run_speed","rush_speed",
    "knockback_speed","dash_speed","jump_add","run_accel","air_accel","run_decel",
    "run_brake","air_brake","air_decel","air_resist","air_resist_v","air_resist_h",
    "ground_friction","ground_friction_h","friction_cap","rat_bound","radius",
    "height","weight","camera_y","center_height"];
impl Physics {
    pub fn decode(data:&[u8])->Result<Self> {
        if data.len()!=132 {return Err("physics record must be 132 bytes".into());}
        let hang_time=i32::from_be_bytes(data[..4].try_into().unwrap());
        let mut bits=[0;32];
        for (i,b)in bits.iter_mut().enumerate(){*b=u32::from_be_bytes(data[4+i*4..8+i*4].try_into().unwrap());}
        if bits.iter().any(|&b|!f32::from_bits(b).is_finite()){return Err("nonfinite physics parameter".into());}
        Ok(Self{source_address:0x80196578,hang_time,parameter_bits:bits})
    }
    /// Exact source record, including signed zero and caller-updated parameter bits.
    pub fn encode(&self)->[u8;132]{
        let mut data=[0u8;132];data[..4].copy_from_slice(&self.hang_time.to_be_bytes());
        for (i,bits)in self.parameter_bits.iter().enumerate(){data[4+i*4..8+i*4].copy_from_slice(&bits.to_be_bytes());}
        data
    }
    pub fn parameter(&self,index:usize)->Option<f32>{self.parameter_bits.get(index).map(|&b|f32::from_bits(b))}
}
#[derive(Serialize)]
pub struct Animation {
    pub animation:i16,pub model:i16,pub mode:i16,pub next:i16,
    pub transition_bits:u32,pub speed_bits:u32,
}
pub fn decode_animations(data:&[u8])->Result<Vec<Animation>>{
    if data.len()!=203*16{return Err("expected 203 Sonic animation records".into());}
    data.chunks_exact(16).map(|b|{
        let s=|i|i16::from_be_bytes(b[i..i+2].try_into().unwrap());
        let u=|i|u32::from_be_bytes(b[i..i+4].try_into().unwrap());
        let item=Animation{animation:s(0),model:s(2),mode:s(4),next:s(6),transition_bits:u(8),speed_bits:u(12)};
        if !f32::from_bits(item.transition_bits).is_finite() || !f32::from_bits(item.speed_bits).is_finite(){return Err("nonfinite animation parameter".into());}
        Ok(item)
    }).collect()
}
#[derive(Serialize)]
pub struct Tables {pub physics:Physics,pub animation_table_address:u32,pub animations:Vec<Animation>}
