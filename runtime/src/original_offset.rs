//! Vertical collision offset, original 800428f8..80042b4c.
use crate::{disc::Result,original_gameplay::CHARACTER_BYTES,original_math::OriginalMath};
fn word(b:&[u8],i:usize)->u32{u32::from_be_bytes(b[i..i+4].try_into().unwrap())}
fn float(b:&[u8],i:usize)->f32{f32::from_bits(word(b,i))}
/// `normal` is the collision workspace vector at +1c; gravity is global 801e5624.
/// Inputs are read-only. Output is committed only after supported finite arithmetic.
pub fn update(motion:&[u8],character:&[u8],normal:[f32;3],gravity:[f32;3],output:&mut[f32;3],math:&OriginalMath)->Result<()>{
    if motion.len()<12 || character.len()!=CHARACTER_BYTES{return Err("unsupported offset state size".into());}
    let k=|offset:u32|math.constant(0x803b6520_u32-offset);
    let velocity=[float(motion,0),float(motion,4),float(motion,8)];
    let mut amount=float(character,0xc4);
    let flags=word(character,0x8c);
    let status=u16::from_be_bytes(character[0x1c..0x1e].try_into().unwrap());
    let local_x=float(character,0x64);let local_y=float(character,0x68);
    let dash=float(character,0xf4);let run=float(character,0xe8);
    if !velocity.into_iter().chain(normal).chain(gravity).chain([amount,local_x,local_y,dash,run,output[1]]).all(f32::is_finite){return Err("nonfinite offset input".into());}
    let mut y=output[1];
    if flags&0x100!=0 {
        let scale=if velocity[1]<=k(0x75f8){k(0x7514)}else{k(0x7590)};
        y=-(scale.mul_add(amount,-y));
    }else if local_y<=k(0x75f8){
        if flags&8!=0{amount*=k(0x7580);}else{
            let slope=-gravity[2].mul_add(normal[2],gravity[1].mul_add(normal[1],normal[0]*gravity[0]));
            if !slope.is_finite(){return Err("nonfinite offset slope".into());}
            if status&0x2000!=0{
                amount*=if slope<=k(0x74f4){k(0x74f0)}else if slope<=k(0x74ec){k(0x74e8)}else{k(0x74e4)};
            }else if !(local_x>=dash && status&3!=0) && slope>k(0x75a0){
                if slope<k(0x7590){
                    let (length,unit)=math.estimated_vector(velocity)?;
                    let unit_y=if length==0.{k(0x75f8)}else{unit.ok_or("undefined offset normalization")?[1]};
                    amount*=if unit_y.abs()<k(0x7574){k(0x74fc)}else{k(0x7580)};
                }else if local_x<=run{amount*=k(0x75cc);}
                else if status&3!=0{amount*=local_x/dash;}
            }
        }
        y-=amount;
    }else if status&0x2000!=0{y-=amount;}
    if !y.is_finite(){return Err("nonfinite offset result".into());}
    *output=[k(0x75f8),y,k(0x75f8)];Ok(())
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn rejects_without_partial_output(){
  let math=OriginalMath::from_dol(std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../source/main.dol")).unwrap()).unwrap();
  let mut c=[0;CHARACTER_BYTES];c[0xc4..0xc8].copy_from_slice(&f32::NAN.to_be_bytes());
  let mut output=[1.,2.,3.];assert!(update(&[0;12],&c,[0.,1.,0.],[0.,-1.,0.],&mut output,&math).is_err());assert_eq!(output,[1.,2.,3.]);
  c[0xc4..0xc8].copy_from_slice(&f32::MAX.to_be_bytes());output=[1.,-f32::MAX,3.];
  assert!(update(&[0;12],&c,[0.,0.,0.],[0.,-1.,0.],&mut output,&math).is_err());assert_eq!(output,[1.,-f32::MAX,3.]);
 }
}
