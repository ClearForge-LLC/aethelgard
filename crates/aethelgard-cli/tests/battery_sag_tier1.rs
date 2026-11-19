//! `AET-WO-2304b`: the battery-sag chain's tier-1 checks, with no tool.

use std::collections::BTreeMap;

use aethelgard_ir::Specification;

fn spec(path: &str) -> Specification {
    let bytes = std::fs::read(format!("{}/{path}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn decoupling(path: &str) -> BTreeMap<String, u64> {
    let s = spec(path);
    let table = aethelgard_svd_hal::pin_table(&s.mcu.part).unwrap();
    let t = aethelgard_ir::solve(&s, &table, &"0".repeat(64)).unwrap();
    aethelgard_eda::schematic::decoupling_picofarads(&s, &t, &table).unwrap()
}

/// F3 (`AET-WO-2304b` §1.4): a regulator's output capacitors count on its
/// rail at IR's credited figure (no maker curve is held for the four Cout
/// parts, so the fixture's 500 per mille declared derating: 4 × 11 µF a
/// rail), measured; a board without a tree moves no byte (`sentinel-01`,
/// `motors-dshot600`: 8.3 µF, as on `main`).
#[test]
fn a_regulator_s_output_capacitors_count_in_its_rail_s_decoupling() {
    let pins: [(&str, &[(&str, u64)]); 4] = [
        (
            "tests/fixtures/sentinel-03.json",
            &[("VDD_3V3", 54_810_000), ("VDD_5V", 44_000_000)],
        ),
        (
            "../aethelgard-ir/tests/fixtures/sentinel-02-power.json",
            &[
                ("VDD_1V8", 700_000),
                ("VDD_3V3", 52_300_000),
                ("VDD_5V", 44_000_000),
            ],
        ),
        ("../../examples/sentinel-01-minimal.json", &[("VDD_3V3", 8_300_000)]),
        ("tests/fixtures/motors-dshot600.json", &[("VDD_3V3", 8_300_000)]),
    ];
    for (path, want) in pins {
        let want: BTreeMap<String, u64> = want.iter().map(|(d, pf)| ((*d).to_owned(), *pf)).collect();
        assert_eq!(decoupling(path), want, "{path}");
    }
}
