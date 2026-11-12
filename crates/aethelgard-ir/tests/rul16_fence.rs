//! `RUL-16` as a compiled fence (`AET-WO-1204`). The two accessors are the
//! only way to read a specification's device lists; `cli`, `svd-hal` and
//! `twin` each carry a `clippy.toml` that disallows both. Clippy resolves a
//! `disallowed-methods` path through the compiler, but a path that no longer
//! names a function is only a warning, so a renamed accessor would leave the
//! fence silently dead. This test is the other half: it names each accessor as
//! a compiled item, so a rename fails to build until it is edited, and it
//! reads the three files for exactly those two paths.

use std::fs;
use std::path::Path;

use aethelgard_ir::{I2cDevice, Specification, SpiDevice};

/// Binds the accessor as a typed item and returns the path a `clippy.toml`
/// must spell for it. One macro makes both from one name, so they cannot drift.
macro_rules! accessor {
    ($name:ident, $device:ty) => {{
        let _: fn(&Specification) -> &[$device] = Specification::$name;
        concat!("aethelgard_ir::Specification::", stringify!($name))
    }};
}

const FENCED: [&str; 3] = ["aethelgard-cli", "aethelgard-svd-hal", "aethelgard-twin"];

/// Every `path = "..."` value of a `clippy.toml`, in file order.
fn paths(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .flat_map(|l| l.split("path = \"").skip(1))
        .filter_map(|rest| rest.split('"').next())
        .map(str::to_owned)
        .collect()
}

#[test]
fn each_runner_disallows_exactly_the_two_accessors() {
    let mut want = vec![
        accessor!(declared_i2c_devices, I2cDevice).to_owned(),
        accessor!(declared_spi_devices, SpiDevice).to_owned(),
    ];
    want.sort();
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for name in FENCED {
        let file = crates.join(name).join("clippy.toml");
        let text = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("{} is the fence of {name}: {e}", file.display()));
        let mut got = paths(&text);
        got.sort();
        assert_eq!(got, want, "{name}/clippy.toml");
        assert_eq!(
            text.matches("reason = \"RUL-16").count(),
            want.len(),
            "{name}/clippy.toml: every entry gives a `RUL-16` reason"
        );
        assert!(
            text.contains("disallowed-methods"),
            "{name}/clippy.toml: the entries are not under `disallowed-methods`"
        );
    }
}

#[test]
fn the_schematic_crate_is_not_fenced() {
    // It draws declared intent (`RUL-16`, scope of 2026-11-09).
    let eda = Path::new(env!("CARGO_MANIFEST_DIR")).join("../aethelgard-eda");
    assert!(!eda.join("clippy.toml").exists());
}
