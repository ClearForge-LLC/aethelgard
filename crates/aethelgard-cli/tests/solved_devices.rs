//! `AET-WO-2203` §1.3: no consumer learns the I2C devices from a
//! `Specification` (`RUL-16`); `aethelgard-ir`'s `solve` copies them into
//! `SolvedTopology::i2c_devices` and everyone else reads them there. A grep
//! over the crates' sources, so a new reader cannot be added quietly.

use std::fs;
use std::path::{Path, PathBuf};

fn crates() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Every `.rs` file under `dir`, with its text.
fn sources(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = fs::read_to_string(&path).unwrap();
                out.push((path, text));
            }
        }
    }
    out.sort();
    out
}

/// Lines of `text` that are not `//` comments, numbered from 1.
fn code(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l))
        .filter(|(_, l)| !l.trim_start().starts_with("//"))
}

/// True when `line` holds `word` as a whole identifier.
fn has_word(line: &str, word: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    line.match_indices(word).any(|(at, _)| {
        !line[..at].chars().next_back().is_some_and(ident)
            && !line[at + word.len()..].chars().next().is_some_and(ident)
    })
}

#[test]
fn the_interim_derivation_is_gone() {
    // The names are assembled so that this file does not match itself.
    let names = [format!("i2c_{}", "targets"), format!("I2c{}", "Target")];
    for (path, text) in sources(&crates()) {
        for (n, line) in code(&text) {
            for name in &names {
                assert!(
                    !line.contains(name.as_str()),
                    "{}:{n} names `{name}`, which `AET-WO-2203` deleted",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn nothing_outside_solve_reads_devices_from_a_specification() {
    // The crates that generate or judge from a topology never name the
    // `Specification` type at all (`RUL-16`) ...
    for krate in ["aethelgard-svd-hal", "aethelgard-twin", "aethelgard-chaos"] {
        for (path, text) in sources(&crates().join(krate).join("src")) {
            for (n, line) in code(&text) {
                assert!(
                    !has_word(line, "Specification"),
                    "{}:{n} names `Specification`: {line}",
                    path.display()
                );
            }
        }
    }
    // ... and no crate but `ir` and `eda` reads `.i2c_devices` off anything
    // called a specification. The CLI holds the `Specification` it parsed,
    // so it is the one a quiet reader would be added to.
    for krate in [
        "aethelgard-svd-hal",
        "aethelgard-twin",
        "aethelgard-chaos",
        "aethelgard-cli",
    ] {
        for dir in ["src", "tests"] {
            for (path, text) in sources(&crates().join(krate).join(dir)) {
                if path.ends_with("solved_devices.rs") {
                    continue;
                }
                for (n, line) in code(&text) {
                    for held in ["spec", "specification", "spec()", "specification()"] {
                        assert!(
                            !line.contains(&format!("{held}.i2c_devices")),
                            "{}:{n} reads a specification's devices: {line}",
                            path.display()
                        );
                    }
                }
            }
        }
    }
}

/// `aethelgard-eda`'s schematic and layout read the specification: they are
/// design intent, not a generator of the firmware or the platform, and are
/// outside `AET-WO-2203`'s fence. Pinned at exactly these readers, so a third
/// one fails here and goes to the architect (FEEDBACK).
#[test]
fn the_eda_readers_are_exactly_the_known_two() {
    let mut readers = Vec::new();
    for (path, text) in sources(&crates().join("aethelgard-eda").join("src")) {
        for (_, line) in code(&text) {
            if line.contains(".i2c_devices") {
                readers.push(path.file_name().unwrap().to_string_lossy().into_owned());
            }
        }
    }
    assert_eq!(readers, ["schematic.rs", "schematic.rs"]);
}
