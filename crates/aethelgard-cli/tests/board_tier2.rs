//! Gate clause 5 (`AET-WO-1400`), tier 2: `aethelgard prove` on the generated
//! reference board, inside the tools image. The harness was generated with
//! the crate from the specification side, and the regeneration test guards it
//! (`INV-07`). Three seeded generator bugs, each a one-line edit to a copy of
//! the crate, must each fail on the rule they break, and a seeded `assume`
//! that cuts every path must fail as `cover_unsatisfied`. Each call writes an `ANSWER 1400-C5`
//! line to `/dev/stderr`. The proofs run one at a time: each peaks at about
//! 3.5 GB (measured on Furnace), and the `tools` runner has 15 GiB.

#![cfg(feature = "with-tools")]

mod common;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Instant;

use serde_json::Value;

const HARNESS: &str = "proofs::every_valid_sequence";

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn board() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../boards/sentinel-01")
}

fn prove(label: &str, crate_dir: &Path, code: i32) -> Value {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let start = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args([
            "prove",
            "--crate",
            crate_dir.to_str().unwrap(),
            "--harness",
            HARNESS,
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let line = format!(
        "ANSWER 1400-C5: {label}: exit={:?} wall_s={:.2} verdict={}\n",
        out.status.code(),
        start.elapsed().as_secs_f64(),
        stdout.trim()
    );
    let mut log = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
    log.write_all(line.as_bytes()).unwrap();
    assert_eq!(stdout.lines().count(), 1, "stdout: {stdout}");
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out.status.code(), Some(code), "{v}");
    assert_eq!(v["check"], "prove");
    assert_eq!(v["harness"], HARNESS);
    assert_eq!(
        v["tool"]["version"],
        "Kani Rust Verifier 0.68.0 (cargo plugin)"
    );
    v
}

/// A copy of the board with `from` replaced by `to` in `src/board.rs`, once.
fn seeded(label: &str, from: &str, to: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-1400-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    for file in [
        "Cargo.toml",
        "Cargo.lock",
        "src/lib.rs",
        "src/board.rs",
        "src/proofs.rs",
    ] {
        fs::copy(board().join(file), dir.join(file)).unwrap();
    }
    let lib = dir.join("src/board.rs");
    let text = fs::read_to_string(&lib).unwrap();
    assert_eq!(
        text.matches(from).count(),
        1,
        "seed site `{from}` is not unique"
    );
    fs::write(&lib, text.replacen(from, to, 1)).unwrap();
    dir
}

fn fails_on(label: &str, from: &str, to: &str, kind: &str, message: &str) {
    let dir = seeded(label, from, to);
    let v = prove(label, &dir, 1);
    fs::remove_dir_all(&dir).unwrap();
    assert_eq!(v["verdict"], "fail");
    let messages: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .inspect(|f| assert_eq!(f["kind"], kind))
        .map(|f| f["message"].as_str().unwrap())
        .collect();
    assert!(messages.contains(&message), "{messages:?}");
}

#[test]
fn the_generated_board_proves() {
    let v = prove("board", &board(), 0);
    assert_eq!(v["verdict"], "pass");
    assert_eq!(v["findings"], serde_json::json!([]));
}

#[test]
fn a_wrong_alternate_function_fails() {
    fails_on(
        "seed-wrong-af",
        "(v & !0x0f000000) | 0x04000000",
        "(v & !0x0f000000) | 0x05000000",
        "proof_failed",
        "GpioB word 8: a solved pin's field is not what the specification fixes",
    );
}

#[test]
fn a_missing_clock_enable_fails() {
    fails_on(
        "seed-no-clock",
        "r.modify(TOKEN, Block::Rcc, 0x0f0, |v| v | 0x00001000);",
        "",
        "proof_failed",
        "Spi1 accessed before RCC_APB2ENR bit 12 is set",
    );
}

#[test]
fn a_write_past_the_block_fails() {
    fails_on(
        "seed-past-block",
        // `write_byte`'s write, named by its signature: `transfer` (AET-WO-2202)
        // also writes TXDR with the same expression.
        "r: &mut R, byte: u8) {\n        r.write(TOKEN, Block::I2c1, 0x28, u32::from(byte));",
        "r: &mut R, byte: u8) {\n        r.write(TOKEN, Block::I2c1, 0x2c, u32::from(byte));",
        "proof_failed",
        "register access past the end of its block",
    );
}

/// Adversarial E1: an `assume` that cuts every path in SPI1's start makes
/// every assertion hold and Kani exit 0; only the harness's covers show it.
#[test]
fn a_vacuous_harness_fails() {
    let dir = seeded(
        "seed-vacuous",
        "r.modify(TOKEN, Block::Spi1, 0x0c, |v| v | 0x00400000);",
        "#[cfg(kani)]\n        kani::assume(false);\n        r.modify(TOKEN, Block::Spi1, 0x0c, |v| v | 0x00400000);",
    );
    let v = prove("seed-vacuous", &dir, 1);
    fs::remove_dir_all(&dir).unwrap();
    assert_eq!(v["verdict"], "fail");
    assert_eq!(v["tool"]["exit_code"], 0, "{v}");
    assert_eq!(v["findings"][0]["kind"], "cover_unsatisfied", "{v}");
}

// ---------------------------------------------------------------------------
// `AET-WO-3301`: `build`, `prove --spec` and `verify`, with the real tools.

const SPEC: &str = "examples/sentinel-01-minimal.json";

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

/// Runs the binary from the repository root: its exit code and stdout.
fn aethelgard(args: &[&str]) -> (Option<i32>, String) {
    let start = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let mut log = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
    let line = format!(
        "ANSWER 3301: {} exit={:?} wall_s={:.2}\n",
        args[0],
        out.status.code(),
        start.elapsed().as_secs_f64()
    );
    log.write_all(line.as_bytes()).unwrap();
    (out.status.code(), String::from_utf8(out.stdout).unwrap())
}

fn one(stdout: &str) -> Value {
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    serde_json::from_str(stdout).unwrap()
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&src, &dst);
        } else {
            fs::copy(&src, &dst).unwrap();
        }
    }
}

fn sha(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Builds `board` with `aethelgard build`; the verdict and the ELF's bytes.
fn build(board: &Path, elf: &Path) -> (Value, Vec<u8>) {
    let (code, stdout) = aethelgard(&[
        "build",
        "--crate",
        board.to_str().unwrap(),
        "--out",
        elf.to_str().unwrap(),
    ]);
    let v = one(&stdout);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["check"], "build");
    (v, fs::read(elf).unwrap())
}

/// `AET-WO-3200` F11, closed: the same crate built at two paths of different
/// lengths gives one sha256, and the verdict names it. Then what `build`
/// refuses, end to end.
#[test]
fn build_gives_one_sha256_at_two_paths() {
    let base = std::env::temp_dir().join(format!("aet-3301-paths-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let (a, b) = (
        base.join("a"),
        base.join("a-much-longer-name/and/deeper/board"),
    );
    copy_tree(&board(), &a);
    copy_tree(&board(), &b);
    let (va, ea) = build(&a, &base.join("a.elf"));
    let (vb, eb) = build(&b, &base.join("b.elf"));

    // End to end, what `build` refuses (adversarial B5, B7): a build script,
    // a source read from outside the crate, each exit 3 with no ELF; and a
    // cargo configuration cargo cannot load, exit 5, not a design fail.
    let refused = |label: &str, edit: &dyn Fn(&Path), code: i32, kind: &str| {
        let dir = base.join(label);
        copy_tree(&board(), &dir);
        edit(&dir);
        let elf = base.join(format!("{label}.elf"));
        let (got, stdout) = aethelgard(&[
            "build",
            "--crate",
            dir.to_str().unwrap(),
            "--out",
            elf.to_str().unwrap(),
        ]);
        let v = one(&stdout);
        assert_eq!(got, Some(code), "{label}: {v}");
        assert_eq!(v["error"]["kind"], kind, "{label}: {v}");
        assert!(!elf.exists(), "{label}: an ELF was written");
        v
    };
    let append = |file: &Path, text: &str| {
        let mut f = OpenOptions::new().append(true).open(file).unwrap();
        f.write_all(text.as_bytes()).unwrap();
    };
    refused(
        "build-script",
        &|d| {
            fs::write(d.join("build.rs"), "fn main() {}\n").unwrap();
            let manifest = d.join("Cargo.toml");
            let text = fs::read_to_string(&manifest).unwrap();
            fs::write(
                &manifest,
                text.replacen(
                    "publish = false",
                    "publish = false\nbuild = \"build.rs\"",
                    1,
                ),
            )
            .unwrap();
        },
        3,
        "CrateHasDependency",
    );
    let v = refused(
        "outside-source",
        &|d| {
            append(
                &d.join("src/lib.rs"),
                "\npub static OUTSIDE: &[u8] = include_bytes!(\"/etc/passwd\");\n",
            )
        },
        3,
        "CrateHasDependency",
    );
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("/etc/passwd"),
        "{v}"
    );
    refused(
        "bad-config",
        &|d| append(&d.join("firmware/.cargo/config.toml"), "\nnot toml [[\n"),
        5,
        "ToolError",
    );
    fs::remove_dir_all(&base).unwrap();
    assert_eq!(sha(&ea), sha(&eb));
    assert_eq!(va["elf"]["sha256"], sha(&ea));
    assert_eq!(va["elf"]["sha256"], vb["elf"]["sha256"]);
    assert_eq!(va["input"]["sha256"], vb["input"]["sha256"]);
    assert_eq!(va["verdict"], "pass");
    assert_eq!(&ea[..4], b"\x7fELF");
}

/// §1.4 and §3.3: the review's vacuous hand-written harness still passes
/// Kani, and `prove --spec` now says it is `supplied`, with no cover; the
/// committed crate is `generated`, with its covers counted.
#[test]
fn prove_with_spec_tells_a_hand_written_harness_from_the_generated_one() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("aet-3301-vacuous-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    copy_tree(&board(), &dir);
    fs::write(
        dir.join("src/proofs.rs"),
        "#[kani::proof]\nfn every_valid_sequence() {\n    let x: u8 = kani::any();\n    kani::assume(x != x);\n    assert!(1 == 2);\n}\n",
    )
    .unwrap();
    let args = |c: &Path| {
        [
            "prove".to_owned(),
            "--crate".to_owned(),
            c.to_str().unwrap().to_owned(),
            "--harness".to_owned(),
            HARNESS.to_owned(),
            "--spec".to_owned(),
            SPEC.to_owned(),
        ]
    };
    let a = args(&dir);
    let (code, stdout) = aethelgard(&a.iter().map(String::as_str).collect::<Vec<_>>());
    fs::remove_dir_all(&dir).unwrap();
    let v = one(&stdout);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["verdict"], "pass", "the vacuous harness passes Kani: {v}");
    assert_eq!(v["harness_provenance"], "supplied", "{v}");
    assert_eq!(v["cover_count"], 0, "{v}");

    let mut a = args(&board());
    a[4] = "proofs::i2c1_transfer_is_bounded_and_ordered".to_owned();
    let (code, stdout) = aethelgard(&a.iter().map(String::as_str).collect::<Vec<_>>());
    let v = one(&stdout);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["harness_provenance"], "generated", "{v}");
    assert_eq!(v["cover_count"], 7, "{v}");
}

/// The committed board and `emit`'s files, as the tree `generate` writes.
fn against(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-3301-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    copy_tree(&board(), &dir.join("board"));
    let kicad = dir.join("kicad");
    let (code, stdout) = aethelgard(&["emit", "--file", SPEC, "--out", kicad.to_str().unwrap()]);
    assert_eq!(code, Some(0), "{stdout}");
    dir
}

/// §3.2 and §3.4: `verify` with the 2.2 kΩ scenario and `--against` the
/// committed board passes; its verdict names every file's hash, the ELF's,
/// every gate's result and the run; two runs print the same bytes.
#[test]
fn verify_binds_the_reference_and_two_runs_print_the_same_bytes() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let dir = against("verify-2k2");
    let args = [
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        "scenarios/sentinel-01/pullup-2k2.json",
        "--against",
        dir.to_str().unwrap(),
    ];
    let (code, first) = aethelgard(&args);
    let (code2, second) = aethelgard(&args);
    let v = one(&first);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(code2, Some(0));
    assert_eq!(first, second, "two runs disagree");

    assert_eq!(v["verdict"], "pass");
    assert_eq!(v["against"], "identical");
    assert_eq!(v["rise_times"], "computed from declared values");
    assert_eq!(v["engine_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        v["specification_sha256"],
        sha(&fs::read(root().join(SPEC)).unwrap())
    );
    // Every file of the tree, by path, with the hash of the bytes compared.
    let mut want = serde_json::Map::new();
    let mut stack = vec![dir.clone()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else {
                let rel = p.strip_prefix(&dir).unwrap().to_str().unwrap().to_owned();
                want.insert(rel, sha(&fs::read(&p).unwrap()).into());
            }
        }
    }
    assert_eq!(v["files"], Value::Object(want));
    let gates: Vec<(&str, &str)> = v["gates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| (g["gate"].as_str().unwrap(), g["verdict"].as_str().unwrap()))
        .collect();
    assert_eq!(
        gates,
        [
            ("erc", "pass"),
            ("drc", "pass"),
            ("extract", "pass"),
            ("prove", "pass"),
            ("prove", "pass"),
            ("build", "pass"),
            ("simulate", "pass"),
            ("campaign", "pass"),
        ]
    );
    // Each gate's boundary is what its tools' read grants make it (ruled on
    // PR #96): in the pinned image, Kani's kit is uid 1001's, so `prove` says
    // `toolchain`; under rootless podman every gate says `user_namespace`.
    for g in v["gates"].as_array().unwrap() {
        if g["gate"] == "prove" {
            assert_eq!(g["harness_provenance"], "generated");
        }
        // `AET-WO-1305`: extract is bound to the generated board, and runs no
        // tool, so it has no uid boundary to report (null, never a value).
        if g["gate"] == "extract" {
            assert_eq!(g["input_sha256"], v["files"]["kicad/sentinel-01.kicad_pcb"]);
            assert_eq!(g["uid_boundary"], Value::Null, "{g}");
            assert!(g.get("uid_boundary_unmet").is_none(), "{g}");
            continue;
        }
        let (holds, unmet) = common::expected_boundary(&gate_reads(g["gate"].as_str().unwrap()));
        assert_eq!(g["uid_boundary"], holds, "{g}");
        assert_eq!(g["uid_boundary_unmet"].as_str(), unmet, "{g}");
    }
    // The ELF is the one `build` gives for the committed board anywhere.
    let elf = dir.join("reference.elf");
    let (_, bytes) = build(&board(), &elf);
    assert_eq!(v["elf_sha256"], sha(&bytes));
    let runs = v["campaign"].as_array().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["scenario"], "pullup-2k2");
    assert_eq!(runs[0]["verdict"], "pass");
    assert_eq!(runs[0]["run_id"].as_str().unwrap().len(), 32);
    // The same run id `campaign` gives that ELF and scenario directly.
    let archive = dir.join("archive");
    let (_, lines) = aethelgard(&[
        "campaign",
        "--scenarios",
        "scenarios/sentinel-01/pullup-2k2.json",
        "--firmware",
        elf.to_str().unwrap(),
        "--archive",
        archive.to_str().unwrap(),
    ]);
    let line: Value = serde_json::from_str(lines.lines().next().unwrap()).unwrap();
    assert_eq!(runs[0]["run_id"], line["run_id"]);
    assert_eq!(runs[0]["state_sequence_hash"], line["state_sequence_hash"]);
    fs::remove_dir_all(&dir).unwrap();
}

/// §3.2: the same run with the 47 kΩ scenario fails, with the campaign's
/// findings, although that scenario's own expectation (`fail`) is met.
#[test]
fn verify_fails_the_47k_pull_up_with_the_campaigns_findings() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (code, stdout) = aethelgard(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        "scenarios/sentinel-01/pullup-47k.json",
    ]);
    let v = one(&stdout);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["verdict"], "fail");
    let gates = v["gates"].as_array().unwrap();
    let campaign = gates.last().unwrap();
    assert_eq!(campaign["gate"], "campaign");
    assert_eq!(campaign["verdict"], "fail");
    assert_eq!(
        campaign["exit_code"], 0,
        "the scenario met its own expectation"
    );
    let mut kinds: Vec<&str> = campaign["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["kind"].as_str().unwrap())
        .collect();
    kinds.sort_unstable();
    assert_eq!(kinds, ["analog_rise_exceeds_ceiling", "i2c_timeout"], "{v}");
    assert!(
        gates[..gates.len() - 1]
            .iter()
            .all(|g| g["verdict"] == "pass"),
        "{v}"
    );
    assert_eq!(v["campaign"][0]["verdict"], "fail");
}

/// The paths each gate's tools add to their read grant beyond the system set,
/// as the CLI grants them.
fn gate_reads(gate: &str) -> Vec<PathBuf> {
    let var = |name: &str| PathBuf::from(std::env::var_os(name).expect(name));
    let bridge = Path::new(env!("CARGO_BIN_EXE_aethelgard"))
        .parent()
        .unwrap()
        .to_path_buf();
    let mut reads = Vec::new();
    match gate {
        "erc" | "drc" => reads.extend(common::tool_dirs("kicad-cli")),
        "prove" => {
            reads.extend(common::tool_dirs("cargo"));
            reads.extend(common::tool_dirs("cargo-kani"));
            reads.extend([var("KANI_HOME"), var("RUSTUP_HOME")]);
        }
        "build" => {
            reads.extend(common::tool_dirs("cargo"));
            reads.push(var("RUSTUP_HOME"));
        }
        "simulate" | "campaign" => {
            reads.extend(common::tool_dirs("renode"));
            reads.push(bridge);
        }
        other => panic!("no read grant known for gate {other}"),
    }
    reads
}

/// The generated board's ELF, built by `aethelgard build` from a fresh copy.
fn reference_elf(label: &str) -> String {
    let base = std::env::temp_dir().join(format!("aet-1601-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    copy_tree(&board(), &base.join("board"));
    let (_, bytes) = build(&base.join("board"), &base.join("reference.elf"));
    fs::remove_dir_all(&base).unwrap();
    sha(&bytes)
}

/// `AET-WO-1601` §3 (B1, B2 automated): as root, `build` runs cargo under the
/// tool uid on a root-owned copy. While it runs, a third uid (4242) tries, as
/// soon as the copy is hashed (`status-version` appears), to overwrite the
/// firmware's source in the copy and to plant a cargo configuration in the
/// tool's `HOME`, again and again until `build` ends. Every attempt is
/// refused, and the ELF is the reference's. Root only: a user cannot be
/// another uid.
#[test]
fn as_root_a_third_uid_cannot_change_what_build_reads() {
    use std::os::unix::process::CommandExt;
    assert!(
        rustix::process::geteuid().is_root(),
        "tier 2 runs as root in the image (ci.yml `tools`)"
    );
    let want = reference_elf("third-uid-ref");
    let base = std::env::temp_dir().join(format!("aet-1601-third-uid-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    copy_tree(&board(), &base.join("board"));
    let elf = base.join("raced.elf");
    let cli = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["build", "--crate"])
        .arg(base.join("board"))
        .arg("--out")
        .arg(&elf)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let scratch = format!("/var/lib/aethelgard/aethelgard-{}-", cli.id());
    let attempt = |script: &str| {
        Command::new("/bin/sh")
            .args(["-c", script])
            .uid(4242)
            .gid(4242)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let (mut tries, mut refused) = (0, 0);
    let mut cli = Some(cli);
    let mut out = None;
    while let Some(mut child) = cli.take() {
        if let Some(status) = child.try_wait().unwrap() {
            let mut stdout = String::new();
            std::io::Read::read_to_string(&mut child.stdout.take().unwrap(), &mut stdout).unwrap();
            out = Some((status.code(), stdout));
            break;
        }
        let roots: Vec<PathBuf> = fs::read_dir("/var/lib/aethelgard")
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.to_str().is_some_and(|s| s.starts_with(&scratch)))
            .collect();
        for root in roots.iter().filter(|r| r.join("status-version").exists()) {
            let main = root.join("tool/crate/firmware/src/main.rs");
            let home = root.join("tool");
            for script in [
                format!("echo 'compile_error!(\"raced\");' > '{}'", main.display()),
                format!(
                    "mkdir -p '{h}/.cargo' && echo '[build]' > '{h}/.cargo/config.toml'",
                    h = home.display()
                ),
                format!("mv '{m}' '{m}.aside'", m = main.display()),
            ] {
                let o = attempt(&script);
                assert!(!o.status.success(), "uid 4242 succeeded: {script}");
                let stderr = String::from_utf8_lossy(&o.stderr);
                if stderr.contains("Permission denied") {
                    refused += 1;
                } else {
                    // Only the CLI's own clean-up may beat an attempt.
                    assert!(!root.exists(), "not refused, not gone: {script}: {stderr}");
                    continue;
                }
                tries += 1;
            }
        }
        cli = Some(child);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let (code, stdout) = out.unwrap();
    let v = one(&stdout);
    assert_eq!(code, Some(0), "{v}");
    let (holds, unmet) = common::expected_boundary(&gate_reads("build"));
    assert_eq!(v["sandbox"]["uid_boundary"], holds, "{v}");
    assert_eq!(v["sandbox"]["uid_boundary_unmet"].as_str(), unmet, "{v}");
    assert_eq!(v["elf"]["sha256"], want, "{v}");
    assert_eq!(sha(&fs::read(&elf).unwrap()), want);
    assert!(
        tries > 0 && refused == tries,
        "{refused} of {tries} attempts refused"
    );
    fs::remove_dir_all(&base).unwrap();
}

/// `AET-WO-3301` F13, closed: a cargo configuration another uid plants in
/// `/tmp` is no longer an ancestor of any tool's working directory, so
/// `build` neither reads it nor fails on it. The planted file would break the
/// link if cargo read it. Root only, to plant it as another uid.
#[test]
fn a_cargo_configuration_another_uid_plants_in_tmp_is_not_read() {
    assert!(
        rustix::process::geteuid().is_root(),
        "tier 2 runs as root in the image (ci.yml `tools`)"
    );
    let want = reference_elf("planted-ref");
    let planted = Path::new("/tmp/.cargo");
    fs::create_dir(planted).expect("no /tmp/.cargo before the test");
    struct Remove<'a>(&'a Path);
    impl Drop for Remove<'_> {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(self.0);
        }
    }
    let _remove = Remove(planted);
    let config = planted.join("config.toml");
    fs::write(
        &config,
        "[target.thumbv7em-none-eabihf]\nrustflags = [\"-C\", \"link-arg=--planted-by-another-uid\"]\n",
    )
    .unwrap();
    std::os::unix::fs::chown(planted, Some(4242), Some(4242)).unwrap();
    std::os::unix::fs::chown(&config, Some(4242), Some(4242)).unwrap();
    assert_eq!(reference_elf("planted"), want);
}

/// `AET-WO-1305` §3.5: a board swapped in `verify`'s own scratch after DRC
/// copied it, and before `extract` reads it, is refused by the extract gate:
/// `BindingMismatch`, exit 5, naming `extract`. The swap is timed by DRC's
/// own copy of the board reaching the board's length; DRC itself judges
/// the generated bytes, so `extract` is the first gate to see the swap.
#[test]
fn verify_refuses_a_board_swapped_before_extract() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    // A scratch base the CLI accepts (`AET-WO-1601` §1.4), removed on drop.
    let private = common::private_tmp("1305-swap");
    let tmp = private.0.clone();
    let mut child = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["verify", "--spec", SPEC])
        .current_dir(root())
        .env("TMPDIR", &tmp)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    let mut swapped = false;
    while !swapped && child.try_wait().unwrap().is_none() {
        assert!(start.elapsed().as_secs() < 600, "verify ran 10 minutes");
        let dirs: Vec<PathBuf> = fs::read_dir(&tmp)
            .map(|r| {
                r.filter_map(|e| Some(e.ok()?.path().join("tool")))
                    .collect()
            })
            .unwrap_or_default();
        let ours = dirs
            .iter()
            .map(|d| d.join("tree/kicad/sentinel-01.kicad_pcb"))
            .find(|p| p.is_file());
        let drc = dirs
            .iter()
            .map(|d| d.join("input.kicad_pcb"))
            .find(|p| p.is_file());
        if let (Some(ours), Some(drc)) = (ours, drc) {
            let bytes = fs::read(&ours).unwrap();
            // DRC's copy is the board plus what `kicad::prepare` writes into
            // it, so at the board's length or more, DRC has read it all.
            if fs::metadata(&drc).is_ok_and(|m| m.len() >= bytes.len() as u64) {
                let next = ours.with_extension("swap");
                let mut other = bytes;
                other.push(b'\n');
                fs::write(&next, other).unwrap();
                fs::rename(&next, &ours).unwrap();
                swapped = true;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let v = one(&stdout);
    assert!(swapped, "verify ended before DRC copied the board: {v}");
    assert_eq!(out.status.code(), Some(5), "{v}");
    assert_eq!(v["error"]["kind"], "BindingMismatch", "{v}");
    assert_eq!(v["error"]["id"], "extract", "{v}");
}

/// `AET-WO-1306` §3, with KiCad: every corpus file the strict reader
/// accepts gets, from `check drc`, `check drc --spec --schematic` and
/// `check erc`, exactly the verdict its file states (measured against main's
/// binary in the pinned image; the file's `main:` line). Refusals are tier 1
/// (`strict_tier1.rs`). A board KiCad and the gate read alike keeps its
/// verdict: the gaps against §3's "refused by every gate" are in FEEDBACK.
#[test]
fn every_corpus_file_the_reader_accepts_gets_its_stated_kicad_verdict() {
    use common::corpus;
    let scratch = common::private_tmp("1306-corpus");
    let (board, schematic) = corpus::reference(&scratch.0);
    let mut ran = 0;
    for case in corpus::cases(&board, &schematic) {
        for e in case
            .expect
            .iter()
            .filter(|e| e.code != 3 && e.gate != "extract")
        {
            let (code, v) = corpus::gate(&case, &e.gate, &scratch.0, (&board, &schematic), None);
            let (what, id) = corpus::outcome(&v);
            assert_eq!(
                (code, what, id),
                (e.code, e.what.clone(), e.id.clone()),
                "{}/{} {}: {v}",
                case.class,
                case.name,
                e.gate
            );
            ran += 1;
        }
    }
    eprintln!("ANSWER 1306: {ran} KiCad verdicts as stated");
    assert!(ran > 0);
}

/// Every regular file directly in `dir`, with its bytes.
fn package_files(dir: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            assert!(e.file_type().unwrap().is_file(), "{:?}", e.path());
            (
                e.file_name().into_string().unwrap(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}

/// `verify` of the reference with the 2.2 kΩ scenario and `flag <dir>`.
fn fab_verify(dir: &Path, flag: &str) -> (Option<i32>, String) {
    aethelgard(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        "scenarios/sentinel-01/pullup-2k2.json",
        flag,
        dir.to_str().unwrap(),
    ])
}

/// A fresh package from one `--fab-out` run under `label`: the scratch base,
/// the package directory, its verdict and its files.
fn fab_package(
    label: &str,
) -> (
    PathBuf,
    PathBuf,
    Value,
    std::collections::BTreeMap<String, Vec<u8>>,
) {
    let base = std::env::temp_dir().join(format!("aet-3300-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let dir = base.join("pkg");
    let (code, out) = fab_verify(&dir, "--fab-out");
    let v = one(&out);
    assert_eq!(code, Some(0), "{v}");
    let package = package_files(&dir);
    (base, dir, v, package)
}

/// `AET-WO-3300` §3, the reference probe: `verify --fab-out` with the 2.2 kΩ
/// scenario passes and writes the package; the verdict's `fab` hashes equal
/// the files' sha256; two runs at two paths print the same verdict and write
/// the same bytes; the ELF is the one the campaign ran and the BOM is
/// `emit`'s; the manifest names the verdict the call prints without
/// `--fab-out`; B.Cu carries the GND pour and no Gerber carries KiCad's
/// clock (ruling on PR #103). `--against-fab` is the next two tests (split
/// by ruling D5, so each fits a shard).
#[test]
fn verify_fab_out_writes_the_package_its_verdict_names_at_any_path() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let base = std::env::temp_dir().join(format!("aet-3300-fab-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let a = base.join("a");
    let b = base.join("a-much/longer/path/b");
    let run = fab_verify;
    let (code, first) = run(&a, "--fab-out");
    let v = one(&first);
    assert_eq!(code, Some(0), "{v}");
    let (code, second) = run(&b, "--fab-out");
    assert_eq!(code, Some(0), "{second}");
    assert_eq!(
        first, second,
        "two runs at two paths print different verdicts"
    );
    let package = package_files(&a);
    assert_eq!(package, package_files(&b), "two runs wrote different bytes");

    assert_eq!(v["verdict"], "pass");
    let fab = &v["fab"];
    assert_eq!(fab["status"], "written");
    assert_eq!(fab["tool_version"], "10.0.7");
    assert_eq!(
        fab["board_sha256"],
        v["files"]["kicad/sentinel-01.kicad_pcb"]
    );
    let drc = v["gates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["gate"] == "drc")
        .unwrap();
    assert_eq!(fab["uid_boundary"], drc["uid_boundary"], "{fab}");
    // The verdict names every file written, and only those, by their bytes.
    let named: std::collections::BTreeMap<String, String> = fab["files"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, h)| (k.clone(), h.as_str().unwrap().to_owned()))
        .collect();
    let hashed: std::collections::BTreeMap<String, String> = package
        .iter()
        .map(|(k, bytes)| (k.clone(), sha(bytes)))
        .collect();
    assert_eq!(named, hashed);
    let mut names: Vec<&str> = package.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "FAB-NOTES.md",
            "MANIFEST.json",
            "sentinel-01-B_Cu.gbl",
            "sentinel-01-B_Mask.gbs",
            "sentinel-01-B_Paste.gbp",
            "sentinel-01-B_Silkscreen.gbo",
            "sentinel-01-Edge_Cuts.gm1",
            "sentinel-01-F_Cu.gtl",
            "sentinel-01-F_Mask.gts",
            "sentinel-01-F_Paste.gtp",
            "sentinel-01-F_Silkscreen.gto",
            "sentinel-01-pos.csv",
            "sentinel-01.bom.csv",
            "sentinel-01.drl",
            "sentinel-01.elf",
        ]
    );
    // The firmware is the ELF `build` gave, `simulate` booted and the
    // campaign's run was named by (verify binds the run id to it).
    assert_eq!(named["sentinel-01.elf"], v["elf_sha256"].as_str().unwrap());
    assert_eq!(v["campaign"][0]["verdict"], "pass");
    // The BOM is `emit`'s, byte for byte.
    let emitted = base.join("emit");
    let (code, out) = aethelgard(&["emit", "--file", SPEC, "--out", emitted.to_str().unwrap()]);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(
        package["sentinel-01.bom.csv"],
        fs::read(emitted.join("sentinel-01.bom.csv")).unwrap()
    );
    // The manifest: every other file, and the verdict without its `fab` key,
    // which is the line the same call prints without `--fab-out`.
    let manifest: Value = serde_json::from_slice(&package["MANIFEST.json"]).unwrap();
    let line = first.trim_end();
    let without = format!("{}}}", &line[..line.rfind(",\"fab\":").unwrap()]);
    assert_eq!(manifest["verdict_sha256"], sha(without.as_bytes()));
    assert_eq!(manifest["specification_sha256"], v["specification_sha256"]);
    assert_eq!(manifest["topology_sha256"], v["topology_sha256"]);
    let scenario = fs::read(root().join("scenarios/sentinel-01/pullup-2k2.json")).unwrap();
    assert_eq!(
        manifest["scenarios"],
        serde_json::json!([{"scenario": "pullup-2k2", "sha256": sha(&scenario)}])
    );
    let mut listed = named.clone();
    listed.remove("MANIFEST.json");
    assert_eq!(manifest["files"], serde_json::to_value(&listed).unwrap());
    // Condition 8: B.Cu carries a region of the GND zone. No Gerber carries
    // KiCad's clock lines.
    let bcu = String::from_utf8(package["sentinel-01-B_Cu.gbl"].clone()).unwrap();
    assert!(bcu.contains("%TO.N,GND*%\nG36*\n") && bcu.contains("\nG37*\n"));
    // FAB-NOTES.md names the two removed lines in prose, so only the Gerbers
    // are searched.
    for (name, bytes) in package.iter().filter(|(n, _)| n.contains(".g")) {
        let text = String::from_utf8_lossy(bytes);
        assert!(
            !text.contains("CreationDate") && !text.contains("Created by"),
            "{name}"
        );
    }
    let notes = String::from_utf8(package["FAB-NOTES.md"].clone()).unwrap();
    for want in [
        "70 um",
        "0.34 mm to 2 mm",
        "FR-4",
        "plated via, drilled 0.3 mm",
        "acd60081",
    ] {
        assert!(notes.contains(want), "{want}");
    }
    // The notes describe every other file, and only those.
    let described: Vec<&str> = notes
        .lines()
        .filter_map(|l| l.strip_prefix("- `")?.split('`').next())
        .collect();
    let others: Vec<&str> = names
        .iter()
        .copied()
        .filter(|n| *n != "FAB-NOTES.md" && *n != "MANIFEST.json")
        .collect();
    assert_eq!(described, others);
    fs::remove_dir_all(&base).unwrap();
}

/// §1.3: `--against-fab` on a package `--fab-out` wrote derives it again
/// and finds it identical, naming the same files (ruling D5, split out).
#[test]
fn verify_against_fab_finds_the_package_it_wrote_identical() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (base, dir, v, _) = fab_package("against-same");
    let (code, out) = fab_verify(&dir, "--against-fab");
    let w = one(&out);
    assert_eq!(code, Some(0), "{w}");
    assert_eq!(w["against_fab"], "identical");
    assert_eq!(w["fab"]["status"], "derived");
    assert_eq!(w["fab"]["files"], v["fab"]["files"]);
    assert!(w["fab"].get("differences").is_none(), "{w}");
    fs::remove_dir_all(&base).unwrap();
}

/// §1.3 and §3: one changed byte in every file of a package, and one extra
/// file, fail `--against-fab` with exit 1, naming each (ruling D5, split out).
#[test]
fn verify_against_fab_names_every_changed_and_extra_file() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (base, a, _, package) = fab_package("against-changed");
    for (name, bytes) in &package {
        let mut changed = bytes.clone();
        changed[bytes.len() / 2] ^= 0x01;
        fs::write(a.join(name), changed).unwrap();
    }
    fs::write(a.join("extra.gbr"), "").unwrap();
    let (code, out) = fab_verify(&a, "--against-fab");
    let w = one(&out);
    assert_eq!(code, Some(1), "{w}");
    assert_eq!(w["verdict"], "fail");
    assert_eq!(w["against_fab"], "differs");
    let mut found: Vec<(String, String)> = w["fab"]["differences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["kind"].as_str().unwrap().to_owned(),
                d["message"]
                    .as_str()
                    .unwrap()
                    .split(' ')
                    .next()
                    .unwrap()
                    .to_owned(),
            )
        })
        .collect();
    found.sort();
    let mut want: Vec<(String, String)> = package
        .keys()
        .map(|n| ("against_differs".to_owned(), n.clone()))
        .collect();
    want.push(("against_extra".to_owned(), "extra.gbr".to_owned()));
    want.sort();
    assert_eq!(found, want);
    assert_eq!(found.len(), 16);
    fs::remove_dir_all(&base).unwrap();
}

/// §3: the 47 kΩ scenario fails `verify`, and nothing is written into
/// `--fab-out`; the verdict says so.
#[test]
fn verify_fab_out_writes_nothing_when_a_gate_fails() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("aet-3300-fab-47k-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    }
    let (code, stdout) = aethelgard(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        "scenarios/sentinel-01/pullup-47k.json",
        "--fab-out",
        dir.to_str().unwrap(),
    ]);
    let v = one(&stdout);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["verdict"], "fail");
    assert_eq!(v["fab"]["status"], "not_written");
    assert_eq!(v["fab"]["files"], serde_json::json!({}));
    assert_eq!(
        fs::read_dir(&dir).unwrap().count(),
        0,
        "a failing run wrote into --fab-out"
    );
    fs::remove_dir_all(&dir).unwrap();
}
