//! AET-WO-2304 §1.8.
//! - `decouple SPEC SHA256`: `aethelgard_eda::schematic::decoupling_picofarads`
//!   on the specification, as the CLI's `resolve_droop` calls it
//!   (cli/src/campaign.rs L305-309), printed per rail.
//! - `droop PF...`: `aethelgard_chaos::campaign::{check_droop, tau_ns,
//!   droop_end_ns}` for `scenarios/motors/dshot600-droop.json`'s droop at each
//!   decoupling figure, with the boot's 100 ms span.
use aethelgard_chaos::campaign::{check_droop, droop_end_ns, tau_ns};
use aethelgard_chaos::record::Droop;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a[0].as_str() {
        "decouple" => {
            let text = std::fs::read_to_string(&a[1]).unwrap();
            let spec: aethelgard_ir::Specification = serde_json::from_str(&text).unwrap();
            spec.validate().unwrap();
            let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
            let topology = aethelgard_ir::solve(&spec, &table, &a[2]).unwrap();
            let d =
                aethelgard_eda::schematic::decoupling_picofarads(&spec, &topology, &table).unwrap();
            println!("{}", serde_json::to_string(&d).unwrap());
        }
        "droop" => {
            let droop = Droop {
                domain: "VDD_3V3".to_owned(),
                floor_millivolts: 2400,
                start_ns: 100_000,
                fall_ns: 20_000,
                hold_ns: 200_000,
                rise_ns: 20_000,
                source_milliohms: 500,
                load_milliamps: 100,
                bor_level: 3,
            };
            for pf in &a[1..] {
                let pf: u64 = pf.parse().unwrap();
                println!(
                    "{{\"decoupling_pf\":{pf},\"tau_ns\":{},\"ten_tau_ns\":{},\"droop_end_ns\":{},\"check_droop\":{:?}}}",
                    tau_ns(droop.source_milliohms, pf),
                    tau_ns(droop.source_milliohms, pf) * 10,
                    droop_end_ns(&droop, pf),
                    format!("{:?}", check_droop(&droop, 3300, pf, 100_000_000))
                );
            }
        }
        _ => panic!("usage: decouple SPEC SHA256 | droop PF..."),
    }
}
