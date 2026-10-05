use sa2_runtime::{disc::digest,original_asset_bundle::{load_sonic,ArchiveInput,ArchiveKind}};
fn run()->Result<(),Box<dyn std::error::Error>> {
    let args:Vec<_>=std::env::args().collect();
    if args.len()!=3{return Err("usage: animation-bundle-audit SOURCE_DIRECTORY OUTPUT_JSON".into());}
    let root=std::path::Path::new(&args[1]);
    let dol=std::fs::read(root.join("main.dol"))?;
    let common=std::fs::read(root.join("plcommtn.prs"))?;
    let sonic=std::fs::read(root.join("sonicmtn.prs"))?;
    let model=std::fs::read(root.join("sonicmdl.prs"))?;
    let inputs=[ArchiveInput{name:"plcommtn.prs",kind:ArchiveKind::Motions,compressed:&common},ArchiveInput{name:"sonicmtn.prs",kind:ArchiveKind::Motions,compressed:&sonic},ArchiveInput{name:"sonicmdl.prs",kind:ArchiveKind::Models,compressed:&model}];
    let bundle=load_sonic(&dol,&inputs)?;
    let view=sa2_runtime::original_contact::Memory{base:bundle.base,bytes:&bundle.memory};
    let mut trees=std::collections::BTreeMap::new();
    for entry in &bundle.report.entries {
        if entry.model_pointer!=0 && !trees.contains_key(&entry.model) {
            let mut nodes=vec![0xa55a5aa5;111];
            let count=sa2_runtime::original_animation_graph::enumerate(entry.model_pointer,&view,&mut nodes)?;
            trees.insert(entry.model,serde_json::json!({"root":entry.model_pointer,"count":count,"nodes":nodes}));
        }
    }
    let report=serde_json::json!({"scope":"asset reference audit only; no rendered or playable claim","base":bundle.base,"registry":bundle.registry,"entry_table":bundle.entry_table,"heap_table":bundle.heap_table,"heap_start":bundle.heap_start,"memory_sha256":digest(&bundle.memory),"trees":trees,"bundle":bundle.report});
    std::fs::write(format!("{}.bin",args[2]),&bundle.memory)?;
    std::fs::write(&args[2],serde_json::to_vec_pretty(&report)?)?;
    println!("Asset reference audit written: {}",args[2]);
    Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("{e}");std::process::exit(1);}}
