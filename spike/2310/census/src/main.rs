//! AET-WO-2310 §1.1 census, through `replay_dir` alone (the strict reader, no run rules): for each run
//! directory given, one line of what item 3 (planted instants), item 1 (the droop's level), item 5 (the
//! header's tools) and item 7 (an `ok` read whose witness is unread, and its finding) need.
use aethelgard_chaos::record::{Override, Record};
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
        let mut out = vec![];
        let (mut read, mut booted) = (false, false);
        let mut findings: Vec<String> = vec![];
        let mut statuses = std::collections::BTreeSet::new();
        for x in recs {
            match x {
                Record::BootObserved { boot_word, instructions_observed, instructions_expected, brown_out, .. } => {
                    read = boot_word.observed.is_some();
                    booted = boot_word.observed == Some(boot_word.expected);
                    out.push(format!("instr={instructions_observed:?}/{instructions_expected} bor_word={:?}&{:#x}=={:#x}", brown_out.observed, brown_out.mask, brown_out.expected));
                }
                Record::Verdict { findings: f, .. } => findings = f.iter().map(|(k, _, _)| k.clone()).collect(),
                Record::Scenario { name, overrides, .. } => {
                    out.push(format!("scenario={name}"));
                    for o in overrides {
                        match o {
                            Override::Droop(d) => out.push(format!("droop.start_ns={} droop.bor_level={} fall/hold/rise={}/{}/{}", d.start_ns, d.bor_level, d.fall_ns, d.hold_ns, d.rise_ns)),
                            Override::BatterySag(s) => out.push(format!("sag.start_ns={}", s.start_ns)),
                            _ => {}
                        }
                    }
                }
                Record::RailDeclared { droop, .. } => out.push(format!("k16.start_ns={} k16.bor_level={}", droop.start_ns, droop.bor_level)),
                Record::ResetRendered { at_ns, released, .. } => out.push(format!("reset@{at_ns}{}", if *released { "R" } else { "A" })),
                Record::RecoveryBound { boot_word_ns, bound_ns, .. } => out.push(format!("k32.boot_word_ns={boot_word_ns:?} bound={bound_ns:?}")),
                Record::BatterySagDeclared { sag, .. } => out.push(format!("k31.start_ns={}", sag.start_ns)),
                // Kind 29's injection is one quantum after the judged boot's boot word (`AET-WO-2302`).
                Record::SerialWitness { injected_ns: Some(at), .. } => out.push(format!("k29.injected_ns={at}")),
                _ => {}
            }
        }
        // Item 7: on a read boot, an `ok` read (kind 12 or 21) whose own witness (kind 13 or 22, same
        // reference and step) is unread, and whether the verdict names `{bus}_transaction_unwitnessed`.
        let (mut ok_unread, mut ok_unread_named) = (0u32, 0u32);
        for x in recs {
            let (bus, reference, step, status) = match x {
                Record::I2cCheck { reference, step, status, .. } => ("i2c", reference, step, status),
                Record::SpiCheck { reference, step, status, .. } => ("spi", reference, step, status),
                _ => continue,
            };
            statuses.insert(format!("{bus}:{status:?}"));
            if !read || status.as_deref() != Some("ok") {
                continue;
            }
            let unread = recs.iter().any(|w| match (bus, w) {
                ("i2c", Record::I2cWitness { reference: r2, step: s2, observed, .. }) |
                ("spi", Record::SpiWitness { reference: r2, step: s2, observed, .. }) => r2 == reference && s2 == step && observed.is_none(),
                _ => false,
            });
            if unread {
                ok_unread += 1;
                if findings.iter().any(|f| *f == format!("{bus}_transaction_unwitnessed")) {
                    ok_unread_named += 1;
                }
            }
        }
        let tools: Vec<String> = r.header.tools.iter().map(|(n, v)| format!("{n}={v}")).collect();
        println!(
            "{dir} read={read} booted={booted} {} ok_unread_witness={ok_unread} named={ok_unread_named} statuses={statuses:?} tools={tools:?}",
            out.join(" ")
        );
    }
}
