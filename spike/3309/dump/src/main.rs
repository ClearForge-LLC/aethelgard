// AET-WO-3309 §2, scratch: each run directory's kind 13 and kind 14 records, by index, as `Debug`
// prints them, for the sweep's line-by-line comparison (sh/compare.py).
use aethelgard_chaos::record::Record;

fn main() {
    for d in std::env::args().skip(1) {
        match aethelgard_chaos::archive::replay_dir(std::path::Path::new(&d)) {
            Ok(r) => {
                for (i, rec) in r.state.records.iter().enumerate() {
                    if matches!(rec, Record::I2cWitness { .. } | Record::Verdict { .. }) {
                        println!("[{i}] kind {}: {rec:?}", rec.kind());
                    }
                }
            }
            Err(e) => println!("REPLAY-REFUSED {e:?}"),
        }
    }
}
