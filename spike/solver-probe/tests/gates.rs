//! The spike's measurements, repeated inside CI's `gates` job (`cargo test
//! --workspace`, debug profile). The digests were measured on Furnace in
//! release (`spike/results/furnace-release.txt`); a green run here means
//! `gates` produced the same bytes. Timings go to stderr unbuffered, outside
//! libtest's capture, so they appear in the job log.

use std::io::Write as _;
use std::time::Instant;

use aethelgard_spike_solver::{Pruning, Stats, sha256_hex, solve};

const RECOMMENDED: Pruning = Pruning {
    instance_matching: true,
    pin_matching: true,
    memo: false,
    node_limit: 10_000_000,
};
const WO_BASELINE: Pruning = Pruning {
    instance_matching: false,
    pin_matching: false,
    memo: false,
    node_limit: 10_000_000,
};

/// (file, sha256 of the output line, whether the WO's search finishes fast)
const CASES: &[(&str, &str, bool)] = &[
    (
        "examples/sentinel-01-minimal.json",
        "c7f59a497e8ed989e83491a3a55af2a86a076f06470c8ac13c64e987e561f0cd",
        true,
    ),
    (
        "spike/specs/stress.json",
        "759f41463aaf205dec2eed472c0929e38a300f749cfe51ff49f3937c23e4b578",
        true,
    ),
    (
        "spike/specs/ref-pin-twice.json",
        "3ba63bb3390e5ce7a8cff6daa29e0a9f74741703ba608e6532657ad8d1e6de7c",
        true,
    ),
    (
        "spike/specs/unknown-pin.json",
        "0f9a823ab1545f8c8338adf4ab9ba38042ab972e1857e6cd0d1bd7a4843b57ac",
        true,
    ),
    (
        "spike/specs/reserved-pin.json",
        "223c4e1cb8372d2c0ea35f6f4226f373e18fa003a4517d6ec3056979e16233a7",
        true,
    ),
    (
        "spike/specs/unknown-peripheral.json",
        "eb9848bf14e70dcfa40714adcf92167bc689d322dc7fbf0f28fb72f1c01de7dc",
        true,
    ),
    (
        "spike/specs/wrong-kind-peripheral.json",
        "3a777283205b390cdacb0dbb00a01941818e57548efc2199db7aa751ff2d8a6a",
        true,
    ),
    (
        "spike/specs/no-instance-pin.json",
        "3df8c9ad2b14bbabf9d50750a708c4ac247d8218b9b3a41cd98fa71610168f9c",
        true,
    ),
    (
        "spike/specs/no-instance-peripheral.json",
        "ad8816eb9dc6accbe2a4b42eb379d8d7ed1e58b075571acafed90e013e1a62b8",
        true,
    ),
    (
        "spike/specs/no-instance-held.json",
        "664127d1d472e982ac85e6b3ffd2ba52045e1e8602d0a92dfaa5e4a1a9b64d22",
        true,
    ),
    (
        "spike/specs/peripheral-twice.json",
        "98e7e84f7a8348107c7150c68a5210ae9c8efd6887abddfef7c120e4b7f94e18",
        true,
    ),
    (
        "spike/specs/infeasible-shared.json",
        "f93c5f3fbc663a796668dce2444321ee8134d4a88635561b1c509e0aac405eb2",
        true,
    ),
    (
        "spike/specs/pigeonhole-i2c.json",
        "853b63335b99602f3a904d75f73814689d7c0a1f9a89256705cbbe05e7d81e7e",
        true,
    ),
    (
        "spike/specs/pigeonhole-spi.json",
        "9015bf98b98d00130be1b7430142323fa8c4022be394a371350c9884675ff3e4",
        false,
    ),
    (
        "spike/specs/thrash-pins.json",
        "2157e20359067facf4dc99fdcd99a2e6fa9879ad5358015643c5646619c58596",
        false,
    ),
];

fn line(bytes: &[u8], pruning: Pruning) -> (String, f64, u64) {
    let table = aethelgard_svd_hal::pin_table("STM32H753ZITx").expect("table");
    let mut stats = Stats::default();
    let t = Instant::now();
    let r = solve(bytes, &table, pruning, &mut stats);
    let us = t.elapsed().as_secs_f64() * 1e6;
    let out = match r {
        Ok(topo) => serde_json::to_string(&topo).expect("serialize"),
        Err(e) => format!("REFUSED {}: {}", e.kind(), e.message()),
    };
    (out, us, stats.nodes)
}

#[test]
fn every_spec_gives_the_furnace_bytes_twice() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let mut log = std::io::stderr();
    for &(file, digest, baseline_fast) in CASES {
        let bytes = std::fs::read(format!("{root}/{file}")).expect("read");
        let mut settings = vec![("fc+im+pm", RECOMMENDED)];
        if baseline_fast {
            settings.push(("fc", WO_BASELINE));
        }
        for (label, pruning) in settings {
            let (a, us, nodes) = line(&bytes, pruning);
            let (b, _, _) = line(&bytes, pruning);
            let _ = writeln!(
                log,
                "SPIKE-MEASURE {file} {label}: {us:.1}us nodes={nodes} out_sha256={}",
                sha256_hex(a.as_bytes())
            );
            assert_eq!(a, b, "{file} {label}: two solves differ");
            assert_eq!(sha256_hex(a.as_bytes()), digest, "{file} {label}: {a}");
        }
    }
}

#[test]
fn clause_2_names_both_claimants() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let bytes = std::fs::read(format!("{root}/spike/specs/ref-pin-twice.json")).expect("read");
    let (out, _, _) = line(&bytes, RECOMMENDED);
    assert_eq!(
        out,
        "REFUSED PinClaimedTwice: pin `PB9` is claimed by both `i2c_buses[0].sda` and `spi_devices[0].cs`"
    );
}
