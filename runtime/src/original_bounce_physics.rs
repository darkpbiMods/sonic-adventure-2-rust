//! Action69/70 physics dispatch, source80084f64..80085014.
//! Action decisions run first. External effects after physics are host-owned.
use crate::{disc::Result,original_math::OriginalMath,
 original_position_a::PositionState,original_position_b::PositionBContext,
 original_terrain_candidates::CandidateWorld,original_platform::PlatformMemory,
 original_movement::{AirContext,align_airborne,air_acceleration,integrate_velocity,reconcile_velocity}};

pub fn advance<F>(state:&mut PositionState,special_mode:&mut u32,world:&CandidateWorld,ctx:&PositionBContext,math:&OriginalMath,callback:F)->Result<u32>
where F:FnMut(u32,u32,u32,u32,&mut PlatformMemory)->Result<()>{
 if state.entity.len()!=0x40||!matches!(state.entity[0],69|70){return Err("bounce physics requires action69 or70".into());}
 let mut next=state.clone();let mut mode=*special_mode;
 let control=&ctx.common.control;
 let air=AirContext{directions:control.directions,magnitudes:control.magnitudes,gravity:control.gravity,level:control.level,control_mode:control.control_mode,has_direction_surface:false};
 // Current host contract uses standard downward gravity. Arbitrary gravity
 // needs the source global target angles instead of silently assuming zero.
 if air.gravity!=[0.,-1.,0.]{return Err("bounce alignment requires host gravity target angles".into());}
 align_airborne(&mut next.entity,&mut next.character,0,0,math)?;
 air_acceleration(&mut next.entity,&mut next.motion,&mut next.character,&air,math)?;
 integrate_velocity(&next.entity,&mut next.motion,&mut next.character,air.magnitudes,math)?;
 let response=crate::original_collision_loop::update(&mut next,&mut mode,world,ctx,math,callback)?;
 reconcile_velocity(&mut next.entity,&mut next.motion,&mut next.character,air.gravity,air.level,math)?;
 *state=next;*special_mode=mode;Ok(response)
}
