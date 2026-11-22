//! AET-WO-1219 scratch: the serde dump. Deserializes each named file as
//! `aethelgard_ir::Specification` and prints it back through serde, one JSON
//! line per file. `run.sh` builds it in `~/.cache/aet-1219/dump`, its own
//! workspace (no `Cargo.toml` under `spike/`, as `1216` D6).
use std::io::Write;

fn main() {
    let mut out = std::io::stdout().lock();
    for path in std::env::args().skip(1) {
        let text = std::fs::read_to_string(&path).expect("read");
        let line = match serde_json::from_str::<aethelgard_ir::Specification>(&text) {
            Ok(s) => serde_json::json!({"file": path, "dump": s}),
            Err(e) => serde_json::json!({"file": path, "error": e.to_string()}),
        };
        writeln!(out, "{line}").expect("write");
    }
}
