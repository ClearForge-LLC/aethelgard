//! Usage: aet-1211d-draw <spec.json> <out dir>. Validates (unless `--unvalidated`), solves, emits at
//! library level and writes `<project>.kicad_sch`, `.kicad_pcb`, the BOM and `emit.json` (the
//! refusal or the file hashes), exit 0 on a drawing, 2 on a refusal.
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let unvalidated = args.iter().any(|a| a == "--unvalidated");
    let files: Vec<&String> = args[1..].iter().filter(|a| !a.starts_with("--")).collect();
    let (spec_path, out) = (files[0], Path::new(files[1]));
    std::fs::create_dir_all(out).unwrap();
    let bytes = std::fs::read(spec_path).unwrap();
    let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
    if !unvalidated && let Err(e) = spec.validate() {
        println!("{{\"validate\":{:?}}}", e.to_string());
        std::process::exit(2);
    }
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = match aethelgard_ir::solve(&spec, &table, &"0".repeat(64)) {
        Ok(s) => s,
        Err(e) => {
            println!("{{\"solve\":{:?}}}", format!("{e:?}"));
            std::process::exit(2);
        }
    };
    match aethelgard_eda::schematic::emit(&spec, &solved, &table) {
        Ok(e) => {
            let p = &spec.project_name;
            std::fs::write(out.join(format!("{p}.kicad_sch")), &e.schematic).unwrap();
            std::fs::write(out.join(format!("{p}.kicad_pcb")), &e.board).unwrap();
            std::fs::write(out.join(format!("{p}-bom.csv")), &e.bom).unwrap();
            println!("{{\"ok\":true,\"notes\":{:?}}}", e.notes);
        }
        Err(e) => {
            println!(
                "{{\"kind\":{:?},\"id\":{:?},\"message\":{:?}}}",
                e.kind(),
                e.id(),
                e.to_string()
            );
            std::process::exit(2);
        }
    }
}
