// Scratch: print each I2C record (kinds 12, 13, 15) and the verdict of a run directory, by index.
fn main() {
    for d in std::env::args().skip(1) {
        let r = aethelgard_chaos::archive::replay_dir(std::path::Path::new(&d)).expect("replay");
        println!("== {d}");
        for (i, rec) in r.state.records.iter().enumerate() {
            let k = rec.kind();
            if matches!(k, 7 | 8 | 9 | 12 | 13 | 14 | 15) {
                println!("[{i}] kind {k}: {rec:?}");
            }
        }
    }
}
