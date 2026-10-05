//! Reconstructed GSNE8P rev0 math dependencies, with original operation order.
//! Constants come from the verified executable; host sin/cos are not substituted.
use crate::disc::{digest,Result};
pub type Matrix=[[f32;4];3];
pub const IDENTITY:Matrix=[[1.,0.,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.]];
pub struct OriginalMath { dol:Vec<u8>, sections:Vec<(usize,u32,usize)> }
impl OriginalMath {
    pub fn from_dol(dol:Vec<u8>)->Result<Self>{
        if digest(&dol)!="a53335308d1f820be236fd2b6342fee850eaf048eb26aac21ddb1903b060db2e" {return Err("unverified math source DOL".into());}
        let word=|i|u32::from_be_bytes(dol[i..i+4].try_into().unwrap());
        let sections=(0..18).filter(|&i|word(0x90+i*4)!=0).map(|i|(word(i*4)as usize,word(0x48+i*4),word(0x90+i*4)as usize)).collect();
        Ok(Self{dol,sections})
    }
    /// Verified initialized data only; no original machine-code execution.
    pub fn from_data_bundle(data:Vec<u8>)->Result<Self>{
        if digest(&data)!="24290e49311ffd9876d960216a6ad9dd21f00ecf4e6d4c6d8e4322d19359ca24"{return Err("unverified original math data bundle".into());}
        if &data[..8]!=b"SA2DATA1"{return Err("unsupported math bundle version".into());}
        let word=|i|u32::from_be_bytes(data[i..i+4].try_into().unwrap());
        let mut at=12;let mut sections=Vec::new();
        for _ in 0..word(8){let address=word(at);let size=word(at+4)as usize;at+=8;sections.push((at,address,size));at+=size;}
        if at!=data.len(){return Err("invalid math bundle extent".into());}
        Ok(Self{dol:data,sections})
    }
    fn bytes(&self,address:u32,n:usize)->&[u8]{
        let &(off,start,_)=self.sections.iter().find(|&&(_,a,s)|a<=address && (address as u64+n as u64)<=a as u64+s as u64).expect("constant outside verified DOL");
        let i=off+(address-start)as usize;&self.dol[i..i+n]
    }
    pub fn constant(&self,address:u32)->f32{f32::from_be_bytes(self.bytes(address,4).try_into().unwrap())}
    fn double(&self,address:u32)->f64{f64::from_be_bytes(self.bytes(address,8).try_into().unwrap())}
    /// 0x80111414 cosine / 0x801115a8 sine, using startup copy 0x8011174c.
    /// The source rotation wrappers accept signed32-bit angles. Their radian
    /// arguments fit within 2^18; range reduction is unchanged across this range.
    pub fn sin_cos(&self,x:f32)->Result<(f32,f32)>{
        if !x.is_finite() || x.abs()>262144. {return Err("trig argument outside verified movement domain".into());}
        let c=|a|self.constant(a);
        let scaled=c(0x803b0ce8)*x;
        let quadrant=if x.is_sign_negative(){scaled-c(0x803b0cec)}else{c(0x803b0cec)+scaled} as i32;
        let q=(quadrant&3)as u32;
        let mut r=x-(quadrant.wrapping_mul(2)as f32);
        for offset in [0,4,8,12] {r=c(0x80176988+offset).mul_add(x,r);}
        let qs=c(0x80176da0+q*8);let qc=c(0x80176da4+q*8);
        if r.abs()<c(0x803b0cf0){
            let cos=-(r.mul_add(qs,-qc));
            let sin=c(0x80176de4).mul_add(r*qc,qs);
            return Ok((sin,cos));
        }
        let z=r*r;
        let mut even=c(0x80176dc0).mul_add(z,c(0x80176dc8));
        even=z.mul_add(even,c(0x80176dd0));
        even=z.mul_add(even,c(0x80176dd8));
        even=z.mul_add(even,c(0x80176de0));
        let mut odd=c(0x80176dc4).mul_add(z,c(0x80176dcc));
        odd=z.mul_add(odd,c(0x80176dd4));
        odd=z.mul_add(odd,c(0x80176ddc));
        let sin_poly=z.mul_add(odd,c(0x80176de4));
        if q&1!=0 {Ok((even*qs, (r*(-sin_poly))*qs))}
        else {Ok(((r*sin_poly)*qc,even*qc))}
    }
    /// 0x80115828 Y, 0x801158a8 Z, 0x80115c60 X wrappers and 0x800ed57c.
    pub fn rotate(&self,m:&Matrix,axis:u8,angle:i32)->Result<Matrix>{
        let scale=match axis{b'x'=>0x803b0ef0,b'y'=>0x803b0ed0,b'z'=>0x803b0ee0,_=>return Err("invalid rotation axis".into())};
        let radians=(self.double(scale)*(angle as f64))as f32;
        let (s,c)=self.sin_cos(radians)?;
        let r=match axis {
            b'x'=>[[1.,0.,0.,0.],[0.,c,-s,0.],[0.,s,c,0.]],
            b'y'=>[[c,0.,s,0.],[0.,1.,0.,0.],[-s,0.,c,0.]],
            b'z'=>[[c,-s,0.,0.],[s,c,0.,0.],[0.,0.,1.,0.]],_=>unreachable!()
        };
        Ok(concat(m,&r))
    }
    /// Original 0x80110e64 atan: range reduction and polynomial, not host atan.
    fn atan(&self,value:f32)->f32{
        if value.is_nan(){return value;}
        let c=|a|self.constant(a);let base=0x801768d0_u32;
        let sign=value.to_bits()&0x80000000;let x=value.abs();
        let mut index=-1_i32;let mut reciprocal=false;
        let reduced=if x>=c(0x803b0cc0){reciprocal=true;c(0x803b0cc4)/x}
        else if x>c(0x803b0cc8){
            let bits=x.to_bits();let exponent=bits&0x7f800000;
            index=match exponent {
                0x3f000000=>i32::from(bits>=0x3f08d5b9)+i32::from(bits>=0x3f521801),
                0x3f800000=>2+i32::from(bits>=0x3f9bf7ec)+i32::from(bits>=0x3fef789e),
                0x40000000=>4,_=>0,
            };
            let entry=base+(index*4)as u32;
            let low=c(entry+0x9c);let high=c(entry+0x84);
            let inv=c(0x803b0cc4)/(high+(x+low));
            let a=-(inv.mul_add(c(entry+0x1c),-high));
            let b=-(inv.mul_add(c(entry+0x34),-low));
            a+b
        }else{x};
        let square=reduced*reduced;let cube=reduced*square;
        let mut poly=square.mul_add(c(base+0x18),c(base+0x14));
        for off in [0x10,0xc,8,4]{poly=square.mul_add(poly,c(base+off));}
        let mut result=cube.mul_add(poly,reduced);
        let entry=base.wrapping_add((index*4)as u32);
        result+=c(entry+0x6c);result+=c(entry+0x50);
        if reciprocal {
            result-=c(0x803b0ccc);
            if sign==0 {-result}else{result}
        }else{f32::from_bits(result.to_bits()|sign)}
    }
    /// Original 0x801110b4 refinement of the Gekko reciprocal-square-root seed.
    fn inverse_sqrt(&self,x:f32)->f32{
        if x>0. {
            let mut estimate=crate::gekko_estimate::reciprocal_sqrt_seed(x as f64)as f32;
            let half=self.constant(0x803b0cd4);let three=self.constant(0x803b0cd8);
            for _ in 0..3 {
                let square=estimate*estimate;estimate=half*estimate;
                let correction=-(x.mul_add(square,-three));
                estimate*=correction;
            }estimate
        }else if x==0. {self.constant(0x801d308c)}else{self.constant(0x801d3088)}
    }
    pub(crate) fn asin(&self,x:f32)->f32{
        let remaining=-(x.mul_add(x,-self.constant(0x803b0cc4)));
        self.atan(x*self.inverse_sqrt(remaining))
    }
    /// Complete 80111124..80111168 acos, used by signed vector angles.
    pub(crate) fn acos(&self,x:f32)->f32{
        let remaining=-(x.mul_add(x,-self.constant(0x803b6520-0x585c)));
        let angle=self.atan(x*self.inverse_sqrt(remaining));
        self.constant(0x803b6520-0x5854)-angle
    }
    /// Original 0x80111168 retains its signed-zero branch behavior.
    pub(crate) fn atan2(&self,numerator:f32,denominator:f32)->f32{
        let nsign=numerator.to_bits()&0x80000000;
        let dsign=denominator.to_bits()&0x80000000;
        let half_pi=self.constant(0x803b0ccc);let pi=self.constant(0x803b0cdc);
        if nsign==dsign {
            if dsign!=0 {self.atan(numerator/denominator)-pi}
            else if denominator==0. {half_pi}else{self.atan(numerator/denominator)}
        }else if denominator<0. {pi+self.atan(numerator/denominator)}
        else if denominator==0. {f32::from_bits(nsign.wrapping_add(0x3fc90fdb))}
        else{self.atan(numerator/denominator)}
    }
    /// Complete vector-to-pitch/roll conversion 0x8005071c..0x80050844.
    pub fn surface_angles(&self,v:[f32;3])->Result<[u32;2]>{
        if v.iter().any(|x|!x.is_finite()){return Err("nonfinite direction vector outside verified domain".into());}
        let [x,y,z]=v;let small=self.constant(0x803af134);
        if z>self.constant(0x803af130) || (y.abs()<small && x.abs()<small && z>0.) {return Ok([0x4000,0]);}
        if z<self.constant(0x803af138) || (y.abs()<small && x.abs()<small && z<0.) {return Ok([0xc000,0]);}
        let scale=self.double(0x803aef38);
        let integer=|f:f64|if f.is_nan(){i32::MIN}else{f as i32};
        let pitch=integer(scale*(self.asin(z)as f64));
        let roll=integer(scale*(self.atan2(x,y)as f64)).wrapping_neg();
        Ok([pitch as u32,roll as u32])
    }
    /// Direction setter80039b04, including source integer conversion of NaN.
    pub fn direction_angles(&self,v:[f32;3])->Result<[u32;3]>{
        if v.iter().any(|x|!x.is_finite()){return Err("nonfinite direction setter input".into());}
        let [x,y,z]=v;
        let xy=x.mul_add(x,y*y);let length=self.sqrt_refined(z.mul_add(z,xy));
        let scale=self.double(0x803b6520-0x75e8);
        let integer=|x:f64|if x.is_nan(){i32::MIN}else{x as i32};
        let sine=|angle:i32|->Result<f32>{
            let index=((angle as u32&0xffff)*2)as f32;
            let radians=(self.constant(0x803b6520-0x55f4)*index)/self.constant(0x803b6520-0x55f0);
            Ok(self.sin_cos(radians)?.0)
        };
        let yaw=integer(scale*self.atan2(z,x)as f64);
        let projected=length*sine(yaw)?;
        let pitch=integer(scale*self.acos(-z/projected.abs())as f64);
        let xy_length=self.sqrt_refined(xy);
        let roll_a=integer(scale*self.asin(y/xy_length)as f64);
        let roll_b=integer(scale*self.asin((projected.abs()*sine(pitch)?)/xy_length)as f64);
        Ok([pitch as u32,yaw as u32,roll_a.wrapping_sub(roll_b)as u32])
    }
    /// Original inlined three-iteration double refinement used at 0x8003e910.
    /// The caller's nonpositive branch retains the input instead of taking sqrt.
    pub fn sqrt_refined(&self,x:f32)->f32{
        if x<=0. {return x;}
        let mut estimate=crate::gekko_estimate::reciprocal_sqrt_seed(x as f64);
        let half=self.double(0x803aef40);let three=self.double(0x803aef48);
        for _ in 0..3 {
            let square=estimate*estimate;estimate=half*estimate;
            let correction=-((x as f64).mul_add(square,-three));
            estimate*=correction;
        }
        ((x as f64)*estimate)as f32
    }
    /// 0x801151f4 length and 0x80115d24 normalization use raw CPU estimates,
    /// not the refined square root used elsewhere in the game.
    pub(crate) fn estimated_vector(&self,v:[f32;3])->Result<(f64,Option<[f32;3]>)>{
        let squared=v[2].mul_add(v[2],v[1].mul_add(v[1],v[0]*v[0]));
        if !squared.is_finite() || squared<0. {return Err("unsupported estimated vector domain".into());}
        if squared==0. {return Ok((0.,None));}
        let inv=crate::gekko_estimate::reciprocal_sqrt_seed(squared as f64);
        let length=crate::gekko_estimate::reciprocal_seed(inv);
        Ok((length,Some(v.map(|x|(inv*(x as f64))as f32))))
    }
    pub(crate) fn angle_from_atan2(&self,n:f32,d:f32)->i32 {
        (self.double(0x803aef38)*(self.atan2(n,d)as f64))as i32
    }
    pub(crate) fn double_constant(&self,address:u32)->f64{self.double(address)}
}
/// Complete 0x800ed368 concatenation, including paired lane operation order.
pub fn concat(a:&Matrix,b:&Matrix)->Matrix{
    let mut out=[[0.;4];3];
    for row in 0..3 {for col in 0..4 {
        let v=b[0][col]*a[row][0];
        let v=b[1][col].mul_add(a[row][1],v);
        let mut v=b[2][col].mul_add(a[row][2],v);
        if col>=2 {v=(if col==3 {1.0_f32}else{0.0_f32}).mul_add(a[row][3],v);}
        out[row][col]=v;
    }}out
}
/// Complete 0x800edb98: rotation/scale vector transform, excluding translation.
pub fn transform(m:&Matrix,v:[f32;3])->[f32;3]{
    std::array::from_fn(|row|{
        let x=m[row][0]*v[0];let y=m[row][1]*v[1];
        m[row][2].mul_add(v[2],x+y)
    })
}
