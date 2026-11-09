//! The mechanical half of the `INV-01` dependency direction and the `unsafe`
//! forbid: every member opts into the workspace lints and states the forbid at
//! its root, the five library crates declare nothing and depend only on the
//! allowed crates, and `aethelgard-ir` depends on no other member. A library
//! crate is exactly a manifest and a `src/lib.rs`, so no other target or
//! module can hide a declaration from the check.

use std::fs;
use std::path::{Path, PathBuf};

use toml::{Table, Value};

const MEMBERS: [&str; 8] = [
    "crates/aethelgard-ir",
    "crates/aethelgard-eda",
    "crates/aethelgard-kicad-lib",
    "crates/aethelgard-svd-hal",
    "crates/aethelgard-twin",
    "crates/aethelgard-chaos",
    "crates/aethelgard-evolve",
    "crates/aethelgard-cli",
];

/// The per-crate copies of the root licences (`AET-WO-1800`); the byte-identity
/// check is `package_licences.rs`.
const LICENCE_FILES: [&str; 2] = ["LICENSE-APACHE", "LICENSE-MIT"];

const CHAOS: &str = "crates/aethelgard-chaos";
const SVD_HAL: &str = "crates/aethelgard-svd-hal";
const EDA: &str = "crates/aethelgard-eda";
const TWIN: &str = "crates/aethelgard-twin";
const KICAD_LIB: &str = "crates/aethelgard-kicad-lib";

/// Every member but `ir` (first), `cli` (last), `chaos` (graduated in
/// `AET-WO-0400`), `svd-hal` (graduated in `AET-WO-1101`), `eda`
/// (graduated in `AET-WO-1300`), `kicad-lib` (`AET-WO-1304`, a data crate)
/// and `twin` (graduated in `AET-WO-2100`); the order is pinned by
/// `members_are_exactly_the_eight_parts`.
fn libraries() -> Vec<&'static str> {
    MEMBERS[1..7]
        .iter()
        .copied()
        .filter(|m| ![CHAOS, SVD_HAL, EDA, KICAD_LIB, TWIN].contains(m))
        .collect()
}

const LIBRARY_DEPENDENCIES: [&str; 3] = ["aethelgard-ir", "serde", "thiserror"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn manifest(member: &str) -> Table {
    let path = root().join(member).join("Cargo.toml");
    fs::read_to_string(&path).unwrap().parse().unwrap()
}

/// Every dependency's package name (after any `package =` rename) in every
/// dependency table, target-specific ones included.
fn dependency_names(manifest: &Table) -> Vec<String> {
    let mut tables = vec![manifest];
    if let Some(Value::Table(targets)) = manifest.get("target") {
        tables.extend(targets.values().filter_map(Value::as_table));
    }
    let mut names = Vec::new();
    for table in tables {
        for key in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(Value::Table(deps)) = table.get(key) {
                names.extend(deps.iter().map(|(key, dep)| {
                    dep.get("package")
                        .and_then(Value::as_str)
                        .unwrap_or(key)
                        .to_owned()
                }));
            }
        }
    }
    names
}

/// Every file under `dir`, as paths relative to `dir`, sorted.
fn files(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let relative = path.strip_prefix(dir).unwrap();
                found.push(relative.to_string_lossy().into_owned());
            }
        }
    }
    found.sort();
    found
}

fn crate_root(member: &str) -> syn::File {
    let src = root().join(member).join("src");
    let path = ["lib.rs", "main.rs"]
        .iter()
        .map(|name| src.join(name))
        .find(|path| path.exists())
        .unwrap_or_else(|| panic!("{member} has neither src/lib.rs nor src/main.rs"));
    syn::parse_file(&fs::read_to_string(path).unwrap()).unwrap()
}

fn forbids_unsafe(file: &syn::File) -> bool {
    file.attrs.iter().any(|attr| {
        attr.path().is_ident("forbid")
            && attr
                .parse_args::<syn::Ident>()
                .is_ok_and(|ident| ident == "unsafe_code")
    })
}

#[test]
fn members_are_exactly_the_eight_parts() {
    let root = manifest("");
    let members: Vec<&str> = root["workspace"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.as_str().unwrap())
        .collect();
    assert_eq!(members, MEMBERS);
    assert_eq!(
        root["workspace"]["lints"]["rust"]["unsafe_code"].as_str(),
        Some("forbid")
    );
}

#[test]
fn every_member_opts_into_workspace_lints() {
    for member in MEMBERS {
        let lints = manifest(member).get("lints").cloned();
        assert_eq!(
            lints.and_then(|l| l.get("workspace").and_then(Value::as_bool)),
            Some(true),
            "{member} lacks [lints] workspace = true"
        );
    }
}

#[test]
fn every_crate_root_forbids_unsafe() {
    for member in MEMBERS {
        assert!(
            forbids_unsafe(&crate_root(member)),
            "{member} lacks #![forbid(unsafe_code)]"
        );
    }
}

#[test]
fn no_member_has_a_build_script() {
    for member in MEMBERS {
        assert!(
            !root().join(member).join("build.rs").exists(),
            "{member} has build.rs"
        );
        assert!(
            manifest(member)["package"].get("build").is_none(),
            "{member} sets package.build"
        );
    }
}

#[test]
fn library_crates_declare_nothing_and_only_re_export() {
    for member in libraries() {
        for item in crate_root(member).items {
            assert!(
                matches!(item, syn::Item::Use(_)),
                "{member} declares something other than a re-export"
            );
        }
    }
}

#[test]
fn library_crates_are_only_a_manifest_and_lib_rs() {
    for member in libraries() {
        assert_eq!(
            files(&root().join(member)),
            ["Cargo.toml", "README.md", "src/lib.rs"],
            "{member} has files beyond Cargo.toml, README.md and src/lib.rs"
        );
        for key in manifest(member).keys() {
            assert!(
                ["package", "dependencies", "lints"].contains(&key.as_str()),
                "{member} sets [{key}]"
            );
        }
    }
}

#[test]
fn library_crates_depend_only_on_allowed_crates() {
    for member in libraries() {
        for name in dependency_names(&manifest(member)) {
            assert!(
                LIBRARY_DEPENDENCIES.contains(&name.as_str()),
                "{member} depends on {name}"
            );
        }
    }
}

#[test]
fn ir_depends_on_no_other_member() {
    for name in dependency_names(&manifest("crates/aethelgard-ir")) {
        assert!(
            !name.starts_with("aethelgard"),
            "aethelgard-ir depends on {name}"
        );
    }
}

/// `aethelgard-chaos` holds the trace archive (`AET-WO-0400`), so it is no
/// longer a re-export stub. It is held to a smaller rule instead: its files,
/// its manifest keys and its dependencies are fixed. `AET-WO-2300` added the
/// scenario model, which reads and changes a `Specification`: so
/// `aethelgard-ir`, `serde` and `serde_json`.
#[test]
fn chaos_has_only_its_known_files_keys_and_dependencies() {
    for file in files(&root().join(CHAOS)) {
        let allowed = file == "Cargo.toml"
            || file == "README.md"
            || LICENCE_FILES.contains(&file.as_str())
            || (file.starts_with("src/") && file.ends_with(".rs"))
            || (file.starts_with("tests/") && file.ends_with(".rs"))
            || (file.starts_with("tests/ui/") && file.ends_with(".stderr"));
        assert!(allowed, "chaos has a file outside the allowed set: {file}");
    }
    let chaos = manifest(CHAOS);
    for key in chaos.keys() {
        assert!(
            ["package", "dependencies", "lints"].contains(&key.as_str()),
            "chaos sets [{key}]"
        );
    }
    let mut names = dependency_names(&chaos);
    names.sort();
    assert_eq!(
        names,
        ["aethelgard-ir", "serde", "serde_json", "sha2", "thiserror"]
    );
    for (name, dep) in chaos["dependencies"].as_table().unwrap() {
        assert_eq!(
            dep.get("workspace").and_then(Value::as_bool),
            Some(true),
            "chaos dependency {name} is not from the workspace table"
        );
    }
}

/// `aethelgard-svd-hal` holds the vendored pin data and its parser
/// (`AET-WO-1101`), so it is no longer a re-export stub. Its files, manifest
/// keys and dependencies are fixed. `AET-WO-1400` added ST's CMSIS header and
/// its licence and source note, the trybuild snapshots under `tests/ui/`, and
/// two dev-dependencies: `trybuild` and the generated reference board.
/// `AET-WO-2100` added a third, `aethelgard-twin`, so the regeneration guard
/// also covers the emitted platform description (ruled on PR #56, R6).
#[test]
fn svd_hal_has_only_its_known_files_keys_and_dependencies() {
    const DATA: [&str; 7] = [
        "data/st-cmsis-device-h7/LICENSE.md",
        "data/st-cmsis-device-h7/SOURCE.md",
        "data/st-cmsis-device-h7/stm32h753xx.h",
        "data/st-open-pin-data/GPIO-STM32H747_gpio_v1_0_Modes.xml",
        "data/st-open-pin-data/LICENSE",
        "data/st-open-pin-data/SOURCE.md",
        "data/st-open-pin-data/STM32H753ZITx.xml",
    ];
    for file in files(&root().join(SVD_HAL)) {
        let allowed = file == "Cargo.toml"
            || file == "README.md"
            || LICENCE_FILES.contains(&file.as_str())
            || (file.starts_with("src/") && file.ends_with(".rs"))
            || (file.starts_with("tests/") && file.ends_with(".rs"))
            || (file.starts_with("tests/ui/") && file.ends_with(".stderr"))
            || DATA.contains(&file.as_str());
        assert!(
            allowed,
            "svd-hal has a file outside the allowed set: {file}"
        );
    }
    for file in DATA {
        assert!(root().join(SVD_HAL).join(file).exists(), "missing {file}");
    }
    let svd = manifest(SVD_HAL);
    for key in svd.keys() {
        assert!(
            ["package", "dependencies", "dev-dependencies", "lints"].contains(&key.as_str()),
            "svd-hal sets [{key}]"
        );
    }
    let table = |key: &str| -> Vec<String> {
        let mut names: Vec<String> = svd
            .get(key)
            .and_then(Value::as_table)
            .map(|deps| deps.keys().cloned().collect())
            .unwrap_or_default();
        names.sort();
        names
    };
    assert_eq!(
        table("dependencies"),
        ["aethelgard-ir", "roxmltree", "sha2", "thiserror"]
    );
    assert_eq!(
        table("dev-dependencies"),
        [
            "aethelgard-twin",
            "sentinel-01-board",
            "serde_json",
            "trybuild"
        ]
    );
    for key in ["dependencies", "dev-dependencies"] {
        for (name, dep) in svd[key].as_table().unwrap() {
            if key == "dev-dependencies" && name == "aethelgard-twin" {
                // `twin` depends on `svd-hal`, so a versioned dev-dependency
                // back would make the first publish a cycle (AET-WO-1800).
                // Cargo strips a dev-dependency that is path-only.
                assert_eq!(
                    dep.get("path").and_then(Value::as_str),
                    Some("../aethelgard-twin")
                );
                assert!(dep.get("version").is_none() && dep.get("workspace").is_none());
                continue;
            }
            assert_eq!(
                dep.get("workspace").and_then(Value::as_bool),
                Some(true),
                "svd-hal dependency {name} is not from the workspace table"
            );
        }
    }
}

/// `aethelgard-eda` holds the schematic emitter and the gate's rule files
/// (`AET-WO-1300`). The vendored KiCad files moved out in `AET-WO-1304`, so
/// it is `MIT OR Apache-2.0` and holds no `data/`. Its files, manifest keys,
/// licence and dependencies are fixed.
#[test]
fn eda_has_only_its_known_files_keys_and_dependencies() {
    for file in files(&root().join(EDA)) {
        let allowed = file == "Cargo.toml"
            || file == "README.md"
            || LICENCE_FILES.contains(&file.as_str())
            || (file.starts_with("src/") && file.ends_with(".rs"))
            || (file.starts_with("tests/") && file.ends_with(".rs"));
        assert!(allowed, "eda has a file outside the allowed set: {file}");
    }
    let eda = manifest(EDA);
    for key in eda.keys() {
        assert!(
            ["package", "dependencies", "lints"].contains(&key.as_str()),
            "eda sets [{key}]"
        );
    }
    assert_eq!(
        eda["package"]["license"].as_str(),
        Some("MIT OR Apache-2.0")
    );
    let mut names = dependency_names(&eda);
    names.sort();
    assert_eq!(
        names,
        ["aethelgard-ir", "aethelgard-kicad-lib", "sha2", "thiserror"]
    );
    for (name, dep) in eda["dependencies"].as_table().unwrap() {
        assert_eq!(
            dep.get("workspace").and_then(Value::as_bool),
            Some(true),
            "eda dependency {name} is not from the workspace table"
        );
    }
}

/// `aethelgard-kicad-lib` holds KiCad 10.0.7's vendored files as bytes and
/// nothing else (`AET-WO-1304`, `RUL-11`): its files are named one by one, it
/// has no dependency, it is `CC-BY-SA-4.0`, and `src/lib.rs` is its only Rust.
#[test]
fn kicad_lib_has_only_its_known_files_keys_and_no_dependencies() {
    const DATA: [&str; 18] = [
        "data/kicad-10.0.7/LICENSE.md",
        "data/kicad-10.0.7/README.md",
        "data/kicad-10.0.7/footprints/Capacitor_SMD.pretty/C_0402_1005Metric.kicad_mod",
        "data/kicad-10.0.7/footprints/Package_LGA.pretty/Bosch_LGA-8_2.5x2.5mm_P0.65mm_ClockwisePinNumbering.kicad_mod",
        "data/kicad-10.0.7/footprints/Package_QFP.pretty/LQFP-144_20x20mm_P0.5mm.kicad_mod",
        "data/kicad-10.0.7/footprints/Package_SO.pretty/SOIC-8_5.3x5.3mm_P1.27mm.kicad_mod",
        "data/kicad-10.0.7/footprints/Package_SON.pretty/WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm.kicad_mod",
        "data/kicad-10.0.7/footprints/Resistor_SMD.pretty/R_0402_1005Metric.kicad_mod",
        "data/kicad-10.0.7/symbols/Device.kicad_symdir/C.kicad_sym",
        "data/kicad-10.0.7/symbols/Device.kicad_symdir/R.kicad_sym",
        "data/kicad-10.0.7/symbols/MCU_ST_STM32H7.kicad_symdir/STM32H753ZITx.kicad_sym",
        "data/kicad-10.0.7/symbols/Memory_Flash.kicad_symdir/W25Q32JVSS.kicad_sym",
        "data/kicad-10.0.7/symbols/Sensor.kicad_symdir/BME280.kicad_sym",
        "data/kicad-10.0.7/symbols/Sensor_Temperature.kicad_symdir/TMP102xxDRL.kicad_sym",
        "data/kicad-10.0.7/symbols/Sensor_Temperature.kicad_symdir/TMP117xxDRV.kicad_sym",
        "data/kicad-10.0.7/symbols/power.kicad_symdir/+3V3.kicad_sym",
        "data/kicad-10.0.7/symbols/power.kicad_symdir/GND.kicad_sym",
        "data/kicad-10.0.7/symbols/power.kicad_symdir/PWR_FLAG.kicad_sym",
    ];
    let mut expected: Vec<&str> = DATA.to_vec();
    expected.extend(["Cargo.toml", "README.md", "src/lib.rs"]);
    expected.sort();
    assert_eq!(files(&root().join(KICAD_LIB)), expected);
    let kicad = manifest(KICAD_LIB);
    for key in kicad.keys() {
        assert!(
            ["package", "lints"].contains(&key.as_str()),
            "kicad-lib sets [{key}]"
        );
    }
    assert_eq!(kicad["package"]["license"].as_str(), Some("CC-BY-SA-4.0"));
    assert!(dependency_names(&kicad).is_empty());
}

/// No KiCad library file sits anywhere under `crates/` but in the data crate,
/// so no crate can carry a share-alike file the licence fields do not show.
#[test]
fn kicad_library_files_live_only_in_the_data_crate() {
    let crates = root().join("crates");
    let data = format!("{KICAD_LIB}/data/");
    for file in files(&crates) {
        if file.ends_with(".kicad_mod") || file.ends_with(".kicad_sym") {
            assert!(
                format!("crates/{file}").starts_with(&data),
                "a KiCad library file outside {KICAD_LIB}/data/: {file}"
            );
        }
    }
}

/// `aethelgard-twin` lowers a solved topology to a Renode platform and holds
/// the judge's half of a boot (`AET-WO-2100`, ruled on PR #56, R6), so it is
/// no longer a re-export stub. Its files, manifest keys and dependencies are
/// fixed: the new edge `twin -> svd-hal`, for the register facts, is the
/// ruled one, beside `twin -> ir`.
#[test]
fn twin_has_only_its_known_files_keys_and_dependencies() {
    for file in files(&root().join(TWIN)) {
        let allowed = file == "Cargo.toml"
            || file == "README.md"
            || LICENCE_FILES.contains(&file.as_str())
            || (file.starts_with("src/") && file.ends_with(".rs"))
            || (file.starts_with("tests/") && file.ends_with(".rs"));
        assert!(allowed, "twin has a file outside the allowed set: {file}");
    }
    let twin = manifest(TWIN);
    for key in twin.keys() {
        assert!(
            ["package", "dependencies", "dev-dependencies", "lints"].contains(&key.as_str()),
            "twin sets [{key}]"
        );
    }
    let table = |key: &str| -> Vec<String> {
        let mut names: Vec<String> = twin
            .get(key)
            .and_then(Value::as_table)
            .map(|deps| deps.keys().cloned().collect())
            .unwrap_or_default();
        names.sort();
        names
    };
    assert_eq!(
        table("dependencies"),
        ["aethelgard-ir", "aethelgard-svd-hal", "thiserror"]
    );
    assert_eq!(table("dev-dependencies"), ["serde_json"]);
    for key in ["dependencies", "dev-dependencies"] {
        for (name, dep) in twin[key].as_table().unwrap() {
            assert_eq!(
                dep.get("workspace").and_then(Value::as_bool),
                Some(true),
                "twin dependency {name} is not from the workspace table"
            );
        }
    }
}

fn manifest_name(dir: &Path) -> String {
    let manifest: Table = fs::read_to_string(dir.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    manifest["package"]["name"].as_str().unwrap().to_owned()
}

/// Generated board crates (`AET-WO-1400`, ruled on PR #42, R1 and R3) live under
/// `boards/`, which the root excludes, so the workspace's lints never reach
/// their `unsafe` `mmio` module. Each is its own crate with no `[workspace]`
/// key, and depends on nothing, because `aethelgard prove` judges it with no
/// network and an empty cargo home.
#[test]
fn boards_are_excluded_and_dependency_free() {
    let root_manifest: Table = fs::read_to_string(root().join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let exclude: Vec<&str> = root_manifest["workspace"]["exclude"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(exclude, ["boards", "bridge"]);
    // The trybuild suite judges the board the regeneration test owns, and no
    // other (adversarial E8).
    assert_eq!(
        root_manifest["workspace"]["dependencies"]["sentinel-01-board"]["path"].as_str(),
        Some("boards/sentinel-01")
    );
    let mut boards = Vec::new();
    for entry in fs::read_dir(root().join("boards")).unwrap() {
        let dir = entry.unwrap().path();
        let manifest: Table = fs::read_to_string(dir.join("Cargo.toml"))
            .unwrap()
            .parse()
            .unwrap();
        for key in manifest.keys() {
            assert!(
                ["package", "lints"].contains(&key.as_str()),
                "{} sets [{key}]",
                dir.display()
            );
        }
        // No host code runs when a board builds.
        let package = manifest["package"].as_table().unwrap();
        for key in ["build", "links"] {
            assert!(
                !package.contains_key(key),
                "{} sets package.{key}",
                dir.display()
            );
        }
        assert!(
            !dir.join("build.rs").exists(),
            "{} has a build.rs",
            dir.display()
        );
        // The reference firmware beside the board (`AET-WO-2100`, R1): its
        // own crate, no host code, and one dependency, the board beside it.
        let firmware = dir.join("firmware");
        let fw: Table = fs::read_to_string(firmware.join("Cargo.toml"))
            .unwrap()
            .parse()
            .unwrap();
        for key in fw.keys() {
            assert!(
                ["package", "dependencies", "profile"].contains(&key.as_str()),
                "{} sets [{key}]",
                firmware.display()
            );
        }
        let package = fw["package"].as_table().unwrap();
        for key in ["build", "links"] {
            assert!(
                !package.contains_key(key),
                "the firmware sets package.{key}"
            );
        }
        assert!(
            !firmware.join("build.rs").exists(),
            "the firmware has a build.rs"
        );
        let deps = fw["dependencies"].as_table().unwrap();
        let board = manifest_name(&dir);
        assert_eq!(deps.keys().collect::<Vec<_>>(), [&board]);
        assert_eq!(deps[&board]["path"].as_str(), Some(".."));
        boards.push(dir.file_name().unwrap().to_str().unwrap().to_owned());
    }
    // Exactly the boards `aethelgard-svd-hal/tests/board.rs` regenerates.
    assert_eq!(boards, ["sentinel-01"]);
}

/// The analog bridge (`AET-WO-2200`, `RUL-17`) is excluded with the boards, so
/// the workspace's `forbid(unsafe_code)` does not reach it. In its place: the
/// crate root denies `unsafe_code`, the one `allow` sits on `mod ffi`, the
/// keyword `unsafe` appears in `src/ffi.rs` and nowhere else, and the crate
/// depends on `serde_json` alone and is never published.
#[test]
fn the_bridge_keeps_its_unsafe_in_ffi_alone() {
    let dir = root().join("bridge");
    let manifest: Table = fs::read_to_string(dir.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        !manifest.contains_key("workspace"),
        "bridge sets [workspace]"
    );
    assert_eq!(manifest["package"]["publish"].as_bool(), Some(false));
    assert_eq!(dependency_names(&manifest), ["serde_json"]);
    for key in ["dev-dependencies", "build-dependencies", "target"] {
        assert!(!manifest.contains_key(key), "bridge sets [{key}]");
    }

    let main = fs::read_to_string(dir.join("src/main.rs")).unwrap();
    let file = syn::parse_file(&main).unwrap();
    let denies = file.attrs.iter().any(|attr| {
        attr.path().is_ident("deny")
            && attr
                .parse_args::<syn::Ident>()
                .is_ok_and(|ident| ident == "unsafe_code")
    });
    assert!(denies, "bridge/src/main.rs does not deny unsafe_code");
    let allowed: Vec<String> = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Mod(m) => Some(m),
            _ => None,
        })
        .filter(|m| {
            m.attrs.iter().any(|attr| {
                attr.path().is_ident("allow")
                    && attr
                        .parse_args::<syn::Ident>()
                        .is_ok_and(|ident| ident == "unsafe_code")
            })
        })
        .map(|m| m.ident.to_string())
        .collect();
    assert_eq!(allowed, ["ffi"]);
    // That allow is the only one: an allow on a function would override the deny.
    assert_eq!(main.matches("allow(unsafe_code)").count(), 1);

    let mut with_unsafe = Vec::new();
    // Integration tests are crates of their own, outside the root's deny.
    let sources = ["src", "tests"]
        .into_iter()
        .flat_map(|d| {
            files(&dir.join(d))
                .into_iter()
                .map(move |f| format!("{d}/{f}"))
        })
        .chain(["build.rs".to_owned()]);
    for name in sources {
        let text = fs::read_to_string(dir.join(&name)).unwrap();
        if name != "src/main.rs" {
            assert!(!text.contains("unsafe_code"), "{name} names unsafe_code");
        }
        if has_unsafe_keyword(&text) {
            with_unsafe.push(name);
        }
    }
    assert_eq!(with_unsafe, ["src/ffi.rs"]);
}

/// The vendored header and licence are the bytes `SOURCE.md` pins (`AET-WO-2200`
/// §1.3), so `ffi.rs` and the header cannot drift together unseen.
#[test]
fn the_vendored_ngspice_files_are_the_pinned_ones() {
    use sha2::{Digest, Sha256};

    let dir = root().join("bridge/vendor/ngspice-44.2");
    let source = fs::read_to_string(dir.join("SOURCE.md")).unwrap();
    let mut files = files(&dir);
    files.retain(|f| f != "SOURCE.md");
    assert_eq!(files, ["COPYING", "sharedspice.h"]);
    for name in files {
        let digest = Sha256::digest(fs::read(dir.join(&name)).unwrap());
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        let row = source
            .lines()
            .find(|l| l.starts_with(&format!("| `{name}` |")))
            .unwrap_or_else(|| panic!("SOURCE.md has no row for {name}"));
        assert!(row.contains(&format!("`{hex}`")), "{name} is {hex}: {row}");
    }
}

/// The keyword `unsafe` outside comments and string literals. `unsafe_code`
/// is another identifier, so it does not count.
fn has_unsafe_keyword(src: &str) -> bool {
    let b = src.as_bytes();
    let mut code = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(b"//") {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if b[i..].starts_with(b"/*") {
            i += 2;
            while i < b.len() && !b[i..].starts_with(b"*/") {
                i += 1;
            }
            i += 2;
        } else if let Some(hashes) = raw_string_at(b, i) {
            let close: Vec<u8> = std::iter::once(b'"')
                .chain(std::iter::repeat_n(b'#', hashes))
                .collect();
            i += 2 + hashes;
            while i < b.len() && !b[i..].starts_with(&close) {
                i += 1;
            }
            i += close.len();
        } else if b[i] == b'\'' && b.get(i + 1) == Some(&b'\\') {
            // An escaped char literal such as '\'' or '\\'.
            i += 3;
            while i < b.len() && b[i] != b'\'' {
                i += 1;
            }
            i += 1;
        } else if b[i] == b'\'' && b.get(i + 2) == Some(&b'\'') {
            i += 3;
        } else if b[i] == b'"' {
            i += 1;
            while i < b.len() && b[i] != b'"' {
                i += if b[i] == b'\\' { 2 } else { 1 };
            }
            i += 1;
        } else {
            code.push(b[i]);
            i += 1;
            continue;
        }
        // What was skipped still separates the words around it.
        code.push(b' ');
    }
    let code = String::from_utf8_lossy(&code);
    code.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|w| w == "unsafe")
}

/// `r"`, `r#"`, `r##"` … starting a raw string at `i`, as its count of `#`,
/// also after a `b` or `c` prefix; not `r#type`, a raw identifier, nor an `r`
/// ending a word.
fn raw_string_at(b: &[u8], i: usize) -> Option<usize> {
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let start = match i.checked_sub(1).map(|p| b[p]) {
        Some(b'b' | b'c') => i - 1,
        _ => i,
    };
    if b[i] != b'r' || (start > 0 && word(b[start - 1])) {
        return None;
    }
    let hashes = b[i + 1..].iter().take_while(|&&c| c == b'#').count();
    (b.get(i + 1 + hashes) == Some(&b'"')).then_some(hashes)
}

#[test]
fn the_unsafe_scan_sees_only_code() {
    assert!(has_unsafe_keyword("fn f() { unsafe { g() } }"));
    assert!(has_unsafe_keyword("unsafe extern \"C\" {}"));
    assert!(!has_unsafe_keyword("#![deny(unsafe_code)]"));
    assert!(!has_unsafe_keyword(
        "// unsafe\n/* unsafe */ let s = \"unsafe\";"
    ));
    assert!(!has_unsafe_keyword("let s = r#\"unsafe \"x\"\"#;"));
    assert!(has_unsafe_keyword("let q = b'\"'; unsafe {}"));
    assert!(has_unsafe_keyword("let q = '\\''; fn f<'a>() {} unsafe {}"));
    assert!(has_unsafe_keyword("pub r#type: u8, for\"x\"; unsafe {}"));
    // Adversarial A7: a raw byte string ending in a backslash.
    assert!(has_unsafe_keyword(
        "let _ = br\"\\\"; unsafe { f() } let _ = \"\";"
    ));
    assert!(has_unsafe_keyword("let _ = cr#\"x\"#; unsafe {}"));
}
