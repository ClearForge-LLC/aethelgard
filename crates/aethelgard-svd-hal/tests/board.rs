//! Gate clause 4, board-crate half: the committed `boards/sentinel-01` is
//! exactly what the generator writes for `examples/sentinel-01-minimal.json`,
//! every byte of every file, the Kani harness included. That is what guards
//! the harness (`INV-07`, ruled on PR #42, R3): a hand edit fails here.
//!
//! `AETHELGARD_BLESS=1 cargo test -p aethelgard-svd-hal --test board` writes
//! the tree instead of comparing it; nothing else ever writes `boards/`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use aethelgard_ir::Specification;
use aethelgard_svd_hal::board::generate;
use aethelgard_svd_hal::pin_table;
use sha2::{Digest, Sha256};

const CRATE: &str = "sentinel-01-board";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn board_dir() -> PathBuf {
    root().join("boards/sentinel-01")
}

fn generated() -> BTreeMap<String, String> {
    let bytes = fs::read(root().join("examples/sentinel-01-minimal.json")).unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &sha).unwrap();
    generate(CRATE, &solved, &table).unwrap()
}

/// Every regular file under `dir`, relative, with `/` separators. `target/`
/// at the top is skipped (a local build of the crate); anything else that is
/// not a regular file is reported as a difference by [`compare`].
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let rel = path
                .strip_prefix(dir)
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/");
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                if rel != "target" {
                    stack.push(path);
                }
            } else if kind.is_file() {
                out.insert(rel, fs::read(&path).unwrap());
            } else {
                out.insert(rel, b"<not a regular file>".to_vec());
            }
        }
    }
    out
}

/// The files that differ between `dir` and `files`, missing and extra ones
/// included. Empty means byte-identical.
fn compare(dir: &Path, files: &BTreeMap<String, String>) -> Vec<String> {
    let on_disk = tree(dir);
    let mut diffs = Vec::new();
    for (path, text) in files {
        match on_disk.get(path) {
            None => diffs.push(format!("missing: {path}")),
            Some(bytes) if bytes != text.as_bytes() => diffs.push(format!("differs: {path}")),
            Some(_) => {}
        }
    }
    for path in on_disk.keys() {
        if !files.contains_key(path) {
            diffs.push(format!("extra: {path}"));
        }
    }
    diffs
}

#[test]
fn the_committed_board_crate_is_exactly_the_generated_one() {
    let files = generated();
    if std::env::var_os("AETHELGARD_BLESS").is_some() {
        let dir = board_dir();
        if dir.exists() {
            fs::remove_dir_all(&dir).unwrap();
        }
        for (path, text) in &files {
            let p = dir.join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        }
    }
    assert_eq!(compare(&board_dir(), &files), Vec::<String>::new());
}

#[test]
fn two_generations_are_byte_identical() {
    assert_eq!(generated(), generated());
}

/// The negative control: a copy of the committed tree with one oracle
/// assertion weakened is caught, and so are an extra and a missing file.
#[test]
fn a_hand_edited_harness_fails_the_comparison() {
    let files = generated();
    let copy = std::env::temp_dir().join(format!("aet-1400-board-{}", std::process::id()));
    let _ = fs::remove_dir_all(&copy);
    for (path, bytes) in tree(&board_dir()) {
        let p = copy.join(&path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }
    assert_eq!(compare(&copy, &files), Vec::<String>::new());

    let proofs = copy.join("src/proofs.rs");
    let text = fs::read_to_string(&proofs).unwrap();
    let weakened = text.replacen("kani::assert(", "kani::cover!(", 1);
    assert_ne!(text, weakened);
    fs::write(&proofs, weakened).unwrap();
    assert_eq!(compare(&copy, &files), ["differs: src/proofs.rs"]);

    fs::write(&proofs, &text).unwrap();
    fs::write(copy.join("src/extra.rs"), "").unwrap();
    fs::remove_file(copy.join("Cargo.lock")).unwrap();
    assert_eq!(
        compare(&copy, &files),
        ["missing: Cargo.lock", "extra: src/extra.rs"]
    );
    fs::remove_dir_all(&copy).unwrap();
}
