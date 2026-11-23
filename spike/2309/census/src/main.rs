//! AET-WO-2309 §1.1 sweep B census: for each run directory given, what its records hold of the
//! three rules, as one JSON-ish line. It reads through `replay_dir` (the strict reader, no run
//! rules), so it describes an archive without judging it.
use aethelgard_chaos::record::Record;
use aethelgard_chaos::replay_dir;

fn main() {
    for dir in std::env::args().skip(1) {
        let r = match replay_dir(std::path::Path::new(&dir)) {
            Ok(r) => r,
            Err(e) => {
                println!("{dir} refused={}", e.kind());
                continue;
            }
        };
        let recs = &r.state.records;
        let (mut read, mut booted) = (false, false);
        // Per kind: (read, unread) counts; for 13 and 22 also how many unread hold evidence.
        let mut k = std::collections::BTreeMap::<&str, (u32, u32)>::new();
        let mut evidence_unread = 0u32;
        let (mut solved, mut witnessed) = (0u32, 0u32);
        for x in recs {
            if let Record::BootObserved { boot_word, .. } = x {
                read = boot_word.observed.is_some();
                booted = boot_word.observed == Some(boot_word.expected);
            }
        }
        let mut tally = |name: &'static str, is_read: bool| {
            let e = k.entry(name).or_default();
            if is_read { e.0 += 1 } else { e.1 += 1 }
        };
        for x in recs {
            match x {
                Record::RegisterObserved { reading, .. } => tally("11", reading.observed.is_some()),
                Record::I2cCheck {
                    observed, status, ..
                } => {
                    tally("12v", observed.is_some());
                    tally("12s", status.is_some());
                }
                Record::SpiCheck {
                    observed, status, ..
                } => {
                    tally("21v", observed.is_some());
                    tally("21s", status.is_some());
                }
                Record::I2cWitness {
                    observed, absent, ..
                } => {
                    tally("13", observed.is_some());
                    evidence_unread += u32::from(observed.is_none() && !absent.is_empty());
                }
                Record::SpiWitness {
                    observed, unrouted, ..
                } => {
                    tally("22", observed.is_some());
                    evidence_unread += u32::from(observed.is_none() && !unrouted.is_empty());
                }
                Record::MotorOutputSolved { .. } => solved += 1,
                Record::MotorWitness { .. } => witnessed += 1,
                _ => {}
            }
        }
        let kinds: Vec<String> = k.iter().map(|(n, (a, b))| format!("{n}:{a}/{b}")).collect();
        println!(
            "{dir} read={read} booted={booted} kinds(read/unread)={} unread_with_evidence={evidence_unread} k24={solved} k26={witnessed} trace_bytes={}",
            kinds.join(","),
            r.manifest.trace_bytes
        );
    }
}
