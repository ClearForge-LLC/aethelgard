//! AET-WO-4210 spike: `aet-4210-gen <spec.json> <out-dir>` writes the board
//! crate the generator emits for the specification (firmware included) and
//! the twin's platform description beside it, as `twin_tier2.rs` does for
//! `spi-imu.json`. Prints the platform's path.
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf};

fn main() {
    let args: Vec<String> = env::args().collect();
    let (spec_path, out) = (PathBuf::from(&args[1]), PathBuf::from(&args[2]));
    let bytes = fs::read(&spec_path).expect("spec");
    let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).expect("parse");
    let sha: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).expect("pin table");
    let solved = aethelgard_ir::solve(&spec, &table, &sha).expect("solve");
    let answers = |part: &str| matches!(aethelgard_twin::model::model(part), Ok(Some(_)));
    let name = format!("{}-board", spec.project_name);
    let files = aethelgard_svd_hal::board::generate_with(&name, &solved, &table, &answers).expect("generate");
    let _ = fs::remove_dir_all(&out);
    for (rel, text) in &files {
        let to = out.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(&to, text).unwrap();
    }
    let platform = out.join(format!("{}.repl", spec.project_name));
    fs::write(&platform, aethelgard_twin::platform::emit(&solved).expect("emit")).unwrap();
    println!("{}", platform.display());
}
