//! AET-WO-1800: each published crate carries its own copy of the two licence
//! files, byte-identical to the repository root's (the canonical ones), so the
//! `.crate` crates.io serves holds them.

use std::fs;
use std::path::{Path, PathBuf};

const PUBLISHED: [&str; 6] = [
    "aethelgard-ir",
    "aethelgard-svd-hal",
    "aethelgard-eda",
    "aethelgard-twin",
    "aethelgard-chaos",
    "aethelgard-cli",
];

/// CC-BY-SA-4.0, with KiCad's own `LICENSE.md` as its licence: it carries neither.
const KICAD_LIB: &str = "aethelgard-kicad-lib";

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

#[test]
fn kicad_lib_carries_its_own_licence_and_neither_of_ours() {
    let dir = root().join("crates").join(KICAD_LIB);
    assert!(dir.join("data/kicad-10.0.7/LICENSE.md").is_file());
    for file in ["LICENSE-MIT", "LICENSE-APACHE"] {
        assert!(
            !dir.join(file).exists(),
            "{KICAD_LIB} must not carry {file}"
        );
    }
}
