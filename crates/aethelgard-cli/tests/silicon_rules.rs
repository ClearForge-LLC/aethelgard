//! `AET-WO-4230` §2: every fixture that solves today, solved against the real
//! STM32H753ZITx pin table, has SPI controllers and I2C timings that hold
//! under `aethelgard_ir::silicon`'s independent rules. It lives here, not in
//! `aethelgard-ir`, because the real pin table is `aethelgard-svd-hal`'s and
//! `aethelgard-ir` may depend on no other member.

use std::path::{Path, PathBuf};

use aethelgard_ir::{silicon, solve};
use sha2::{Digest, Sha256};

#[path = "common/spec.rs"]
mod spec;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.json` under `dir`, recursively, sorted.
fn json_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries {
        let p = e.unwrap().path();
        if p.is_dir() {
            json_under(&p, out);
        } else if p.extension().is_some_and(|x| x == "json") {
            out.push(p);
        }
    }
}

/// `examples/` and every crate's `tests/fixtures/`.
fn fixtures() -> Vec<PathBuf> {
    let mut out = Vec::new();
    json_under(&root().join("examples"), &mut out);
    for c in std::fs::read_dir(root().join("crates")).unwrap() {
        json_under(&c.unwrap().path().join("tests/fixtures"), &mut out);
    }
    out.sort();
    out
}

#[test]
fn every_solved_fixture_s_controllers_hold_under_the_independent_rules() {
    let table = aethelgard_svd_hal::pin_table("STM32H753ZITx").unwrap();
    let (mut solved, mut skipped, mut spi, mut i2c) = (Vec::new(), Vec::new(), 0, 0);
    for path in fixtures() {
        let bytes = std::fs::read(&path).unwrap();
        let Some(spec) = spec::committed(&path, &bytes) else {
            continue;
        };
        let hash: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let name = path.strip_prefix(root()).unwrap().display().to_string();
        let t = match solve(&spec, &table, &hash) {
            Ok(t) => t,
            Err(e) => {
                // A fixture refused today is skipped, but never for a reason
                // this work order added.
                let new = [
                    "SpiSpeedBelowKernelRange",
                    "SpiKernelClockUnavailable",
                    "SpiControllerMismatch",
                    "I2cTimingMismatch",
                ];
                assert!(!new.contains(&e.kind()), "{name}: {e}");
                skipped.push(format!("{name}: {}", e.kind()));
                continue;
            }
        };
        assert_eq!(t.bor_level, spec.mcu.bor_level, "{name}");
        for (p, c) in &t.spi_controllers {
            let f = silicon::check_spi(p, c, &t.clock);
            assert!(f.holds, "{name} {p}: {f:#?}");
            assert!(f.achieved_max_hz <= u64::from(f.declared_hz), "{name} {p}");
            let w = silicon::spi_words(&t.spi_controllers, p).expect("words");
            assert_eq!(
                w.cfg2 & (1 << 26 | 1 << 22),
                1 << 26 | 1 << 22,
                "{name} {p}"
            );
            assert_eq!(w.cr1_idle & 1 << 12, 1 << 12, "{name} {p}");
            spi += 1;
        }
        for (p, timing) in &t.i2c_timings {
            let f = silicon::check_i2c_on(t.clock.budget(), p, timing);
            assert!(f.holds, "{name} {p}: {f:#?}");
            i2c += 1;
        }
        solved.push(name);
    }
    eprintln!(
        "{} fixtures solved, {spi} SPI controllers, {i2c} I2C controllers: {solved:?}; refused: {skipped:?}",
        solved.len()
    );
    assert!(
        solved.contains(&"examples/sentinel-01-minimal.json".to_owned()),
        "{solved:?}"
    );
    // Measured on this branch: 17 fixtures, 26 SPI and 26 I2C controllers.
    // A floor, so the test cannot pass by solving nothing.
    assert!(
        solved.len() >= 17 && spi >= 26 && i2c >= 26,
        "{} {spi} {i2c}: {solved:?}",
        solved.len()
    );
}

/// `AET-WO-1213`: `aethelgard_ir::serial::map_baudrate`, the specification's
/// copy, and `aethelgard_pincheck::elf::map_baudrate`, the judge's, agree on
/// every input (`RUL-20`'s pair). Both take ArduPilot's `int32_t`; the
/// IR's returns its `uint32_t`, `pincheck`'s a `u64`. The sweep covers every
/// listed code, 0 and the negatives (57), 1 to 2000 (kbaud), the 2000/2001
/// edge, every `b / 1000` of a `u32` baud, and the `int32_t` extremes; and
/// the console rule, `console_mapped`, is `pincheck`'s map of `b / 1000`
/// against `b` over every baud to 5,000,000 and the `u32` extremes.
#[test]
fn the_ir_s_map_baudrate_is_pincheck_s() {
    use aethelgard_ir::serial::{console_mapped, map_baudrate};
    use aethelgard_pincheck::elf::map_baudrate as judge;
    let codes = [
        1, 2, 4, 9, 19, 38, 57, 100, 111, 115, 230, 256, 460, 500, 921, 1500, 2000,
    ];
    let edges = [i32::MIN, i32::MIN + 1, -1, 0, 2001, i32::MAX - 1, i32::MAX];
    for rate in codes.into_iter().chain(edges).chain(-1000..=4_300_000) {
        assert_eq!(u64::from(map_baudrate(rate)), judge(rate), "{rate}");
    }
    for baud in (0..=5_000_000).chain([u32::MAX - 1, u32::MAX]) {
        let mapped = judge(i32::try_from(baud / 1000).unwrap());
        let expect = (mapped != u64::from(baud)).then_some(mapped);
        assert_eq!(console_mapped(baud).map(u64::from), expect, "{baud}");
    }
}
