//! AET-WO-1800: each published crate carries its own copy of the two licence
//! files, byte-identical to the repository root's (the canonical ones), so the
//! `.crate` crates.io serves holds them.

use std::fs;
use std::path::{Path, PathBuf};

const PUBLISHED: [&str; 7] = [
    "aethelgard-ir",
    "aethelgard-kicad-lib",
    "aethelgard-svd-hal",
    "aethelgard-eda",
    "aethelgard-twin",
    "aethelgard-chaos",
    "aethelgard-cli",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn every_published_crate_holds_byte_identical_licence_files() {
    for name in PUBLISHED {
        for file in ["LICENSE-MIT", "LICENSE-APACHE"] {
            let canonical = fs::read(root().join(file)).unwrap();
            let copy = fs::read(root().join("crates").join(name).join(file))
                .unwrap_or_else(|e| panic!("{name}/{file}: {e}"));
            assert_eq!(copy, canonical, "{name}/{file} differs from the root's");
        }
    }
}
