//! Adapter from source polygon plans and per-mesh vertex snapshots to host draws.
//! Source primitive emitters output GX triangle strips in recorded vertex order.
use serde::Deserialize;
use sa2_runtime::animated_vertex_host::MeshSnapshot;

#[derive(Clone,Deserialize,PartialEq)]
pub struct Texture { pub id:u32, pub wrap_u:u32, pub wrap_v:u32 }
#[derive(Deserialize)]
pub struct Material { pub color:Option<[u8;4]>, pub texture:Option<Texture> }
#[derive(Deserialize)]
pub struct Corner { pub index:usize, pub uv_raw:Option<[i16;2]> }
#[derive(Deserialize)]
pub struct Strip { pub signed_length:i32, pub vertices:Vec<Corner> }
#[derive(Deserialize)]
pub struct Primitive { pub kind:u32, pub header:u32, pub mesh:u32, pub mesh_visit:usize, pub strips:Vec<Strip>, pub material_state:Material }
#[derive(PartialEq)]
pub struct Vertex { pub position:[f32;3], pub normal:[f32;3], pub uv:[f32;2] }
#[derive(PartialEq)]
pub struct Packet { pub vertices:Vec<Vertex>, pub indices:Vec<u32>, pub color:[u8;4], pub texture:Option<Texture>, pub source_flags:u8, pub generated_uv:bool }

pub fn assemble(plan:&[Primitive], snapshots:&[MeshSnapshot])->Result<Vec<Packet>,String> {
    let mut packets=Vec::with_capacity(plan.len());
    for primitive in plan {
        if !matches!(primitive.kind,0x40|0x41) {return Err("Unsupported primitive kind".into());}
        let snapshot=snapshots.get(primitive.mesh_visit).ok_or("Missing mesh visit")?;
        if snapshot.mesh!=primitive.mesh {return Err("Mesh visitation mismatch".into());}
        let color=primitive.material_state.color.ok_or("Undefined material color")?;
        let generated_uv=primitive.header&0x4000!=0;
        let texture=if primitive.kind==0x41 || generated_uv {Some(primitive.material_state.texture.clone().ok_or("Undefined texture")?)}else{None};
        if texture.as_ref().is_some_and(|t|t.wrap_u>2 || t.wrap_v>2) {return Err("Invalid texture wrapping".into());}
        let mut packet=Packet{vertices:vec![],indices:vec![],color,texture,source_flags:(primitive.header>>8)as u8,generated_uv};
        for strip in &primitive.strips {
            if strip.signed_length.unsigned_abs()as usize!=strip.vertices.len() || strip.vertices.len()<3 {return Err("Invalid triangle strip length".into());}
            let start=u32::try_from(packet.vertices.len()).map_err(|_|"Too many vertices")?;
            for corner in &strip.vertices {
                let offset=corner.index.checked_mul(32).ok_or("Vertex index overflow")?;
                let end=offset.checked_add(24).ok_or("Vertex extent overflow")?;
                let data=snapshot.vertices.get(offset..end).ok_or("Vertex outside snapshot")?;
                let mut values=[0.;6];
                for (out,bytes) in values.iter_mut().zip(data.chunks_exact(4)) {*out=f32::from_be_bytes(bytes.try_into().unwrap());}
                if values.iter().any(|x|!x.is_finite()) {return Err("Nonfinite draw vertex".into());}
                let uv=if primitive.kind==0x41 && !generated_uv {corner.uv_raw.ok_or("Missing source UV")?.map(|x|x as f32/256.)}else{[0.;2]};
                packet.vertices.push(Vertex{position:values[..3].try_into().unwrap(),normal:values[3..].try_into().unwrap(),uv});
            }
            for i in 2..strip.vertices.len() as u32 {
                let triangle=if i%2==0 {[start+i-2,start+i-1,start+i]}else{[start+i-1,start+i-2,start+i]};
                packet.indices.extend(triangle);
            }
        }
        packets.push(packet);
    }
    Ok(packets)
}
