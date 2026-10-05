//! Texture-stage wrappers8011c960/c9ac/c9f8 and coordinate wrapper8011d1ac.
use crate::disc::Result;
/// Each command is [original GX entry address, integer arguments...].
pub fn plain()->Vec<Vec<u32>>{vec![vec![0x80104d14,1],vec![0x801014a8,0],vec![0x801044a8,0,4],vec![0x80104b74,0,255,255,4]]}
pub fn textured()->Vec<Vec<u32>>{vec![vec![0x80104d14,1],vec![0x801014a8,1],vec![0x801044a8,0,0],vec![0x80104b74,0,0,0,4]]}
pub fn layered(additional:u32)->Result<Vec<Vec<u32>>>{if additional>7{return Err("unverified texture stage count above eight".into());}let mut calls=vec![vec![0x80104d14,additional+1],vec![0x801014a8,additional+1]];for i in 0..=additional{calls.push(vec![0x801044a8,i,u32::from(i!=0)]);calls.push(vec![0x80104b74,i,i,i,4]);}Ok(calls)}
pub fn coordinate(args:[u32;4])->Vec<Vec<u32>>{vec![vec![0x801011d8,args[0],args[1],args[2],args[3],0,125]]}
