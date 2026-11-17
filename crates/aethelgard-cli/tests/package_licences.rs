//! AET-WO-1800: each published crate carries its own copy of the two licence
//! files, byte-identical to the repository root's (the canonical ones), so the
//! `.crate` crates.io serves holds them.

use std::fs;
use std::path::{Path, PathBuf};

const PUBLISHED: [&str; 11] = [
    "crates/aethelgard-ir",
    // Our own symbols and footprints (`AET-WO-4220b`) are MIT OR Apache-2.0.
    "crates/aethelgard-kicad-lib",
    "crates/aethelgard-svd-hal",
    "crates/aethelgard-eda",
    "crates/aethelgard-twin",
    "crates/aethelgard-chaos",
    "crates/aethelgard-evolve",
    "crates/aethelgard-pincheck",
    "crates/aethelgard-hwdef",
    "crates/aethelgard-cli",
    "models/renode",
];

/// `CC-BY-SA-4.0 AND (MIT OR Apache-2.0)`: KiCad's files with KiCad's own
/// `LICENSE.md`, and ours (`AET-WO-4220b`) with the two above.
const KICAD_LIB: &str = "aethelgard-kicad-lib";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn every_published_crate_holds_byte_identical_licence_files() {
    for name in PUBLISHED {
        for file in ["LICENSE-MIT", "LICENSE-APACHE"] {
            let canonical = fs::read(root().join(file)).unwrap();
            let copy = fs::read(root().join(name).join(file))
                .unwrap_or_else(|e| panic!("{name}/{file}: {e}"));
            assert_eq!(copy, canonical, "{name}/{file} differs from the root's");
        }
    }
}

#[test]
fn kicad_lib_carries_kicads_licence_beside_ours() {
    let dir = root().join("crates").join(KICAD_LIB);
    assert!(dir.join("data/kicad-10.0.7/LICENSE.md").is_file());
    assert!(dir.join("data/aethelgard/README.md").is_file());
}
