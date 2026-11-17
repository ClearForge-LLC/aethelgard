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

use common::verify::{
    SPEC, aethelgard, board, build, copy_tree, gate_reads, generated, one, package_files, root,
    sha, write_files,
};

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Instant;

use serde_json::Value;

const HARNESS: &str = "proofs::every_valid_sequence";
/// The cheapest harness that runs SPI1's whole bring-up (the reset clock,
/// GPIOA, the kernel selects, `enable_spi1`, `init`) through the image, so a
/// seed there fails it in seconds where [`HARNESS`] takes minutes (`AET-WO-4230`
/// Q-K, measured in the image: the missing clock enable fails in 3 s here,
/// 228 s there).
const SPI_SETUP: &str = "proofs::spi1_transfer_refuses_too_many_frames";

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn prove(label: &str, crate_dir: &Path, harness: &str, code: i32) -> Value {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let start = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args([
            "prove",
            "--crate",
            crate_dir.to_str().unwrap(),
            "--harness",
            harness,
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
    assert_eq!(v["harness"], harness);
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

fn fails_on(label: &str, harness: &str, from: &str, to: &str, kind: &str, message: &str) {
    let dir = seeded(label, from, to);
    let v = prove(label, &dir, harness, 1);
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
    let v = prove("board", &board(), HARNESS, 0);
    assert_eq!(v["verdict"], "pass");
    assert_eq!(v["findings"], serde_json::json!([]));
}

/// `AET-WO-4210` §2.2: the 1.8 V IMU and barometer fixture's board crate
/// (generated as the CLI generates it; KiCad cannot draw the stand-ins). Its
/// transfer proofs are bounded by the largest step of each bus's scripts
/// (the IMU's data read: one byte out, six in; the ICP-10111's 16-bit
/// commands), so every transfer the firmware makes is inside what Kani
/// proves bounded and ordered.
#[test]
fn the_imu_fixture_s_transfers_prove_within_its_scripts() {
    let spec_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/imu-baro-1v8.json");
    let bytes = fs::read(&spec_path).unwrap();
    let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &sha(&bytes)).unwrap();
    let answers = |part: &str| matches!(aethelgard_twin::model::model(part), Ok(Some(_)));
    let files =
        aethelgard_svd_hal::board::generate_with("imu-baro-board", &solved, &table, &answers)
            .unwrap();
    let dir = std::env::temp_dir().join(format!("aet-4210-prove-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    for (rel, text) in &files {
        let to = dir.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(&to, text).unwrap();
    }
    let proofs = &files["src/proofs.rs"];
    assert!(
        proofs.contains("kani::assume(wl <= 2 && rl <= 6);"),
        "{proofs}"
    );
    assert!(
        proofs.contains("Spi1 made a data step's frames"),
        "{proofs}"
    );
    for harness in [
        "proofs::spi1_transfer_is_bounded_and_ordered",
        "proofs::i2c1_transfer_is_bounded_and_ordered",
    ] {
        let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let start = Instant::now();
        let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
            .args([
                "prove",
                "--crate",
                dir.to_str().unwrap(),
                "--harness",
                harness,
            ])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        let mut log = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
        log.write_all(
            format!(
                "ANSWER 4210-PB10: {harness}: exit={:?} wall_s={:.2} verdict={}\n",
                out.status.code(),
                start.elapsed().as_secs_f64(),
                stdout.trim()
            )
            .as_bytes(),
        )
        .unwrap();
        let v: Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(out.status.code(), Some(0), "{v}");
        assert_eq!(v["verdict"], "pass", "{v}");
    }
}

/// `AET-WO-4220` PB10: the Sentinel-02 fixture with the parts of our own.
/// The ICM-42688-P's data read (one byte out, six in) and the BMP581's
/// configuration (three out) bound their buses' transfer proofs, so every
/// transfer its firmware makes is inside what Kani proves.
#[test]
fn the_own_parts_fixture_s_transfers_prove_within_its_scripts() {
    let spec_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sentinel-02-own.json");
    let (solved, files) = generated(&spec_path, "own-parts-board");
    let dir = std::env::temp_dir().join(format!("aet-4220-prove-{}", std::process::id()));
    write_files(&dir, &files);
    let proofs = &files["src/proofs.rs"];
    let imu = solved.spi_devices["U5"].peripheral.to_ascii_lowercase();
    let harnesses: Vec<String> = proofs
        .lines()
        .filter_map(|l| l.trim().strip_prefix("fn "))
        .filter_map(|l| l.split_once('(').map(|(n, _)| n.to_owned()))
        .filter(|n| n.ends_with("_transfer_is_bounded_and_ordered"))
        .collect();
    assert!(
        harnesses.contains(&format!("{imu}_transfer_is_bounded_and_ordered")),
        "{harnesses:?}"
    );
    for harness in harnesses {
        let harness = format!("proofs::{harness}");
        let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
        let start = Instant::now();
        let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
            .args([
                "prove",
                "--crate",
                dir.to_str().unwrap(),
                "--harness",
                &harness,
            ])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        let mut log = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
        log.write_all(
            format!(
                "ANSWER 4220-PB10: {harness}: exit={:?} wall_s={:.2} verdict={}\n",
                out.status.code(),
                start.elapsed().as_secs_f64(),
                stdout.trim()
            )
            .as_bytes(),
        )
        .unwrap();
        let v: Value = serde_json::from_str(stdout.trim()).unwrap();
        assert_eq!(out.status.code(), Some(0), "{v}");
        assert_eq!(v["verdict"], "pass", "{v}");
    }
}

#[test]
fn a_wrong_alternate_function_fails() {
    fails_on(
        "seed-wrong-af",
        HARNESS,
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
        SPI_SETUP,
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
        HARNESS,
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
        "r.write(TOKEN, Block::Spi1, 0x0c, 0x84400000);",
        "#[cfg(kani)]\n        kani::assume(false);\n        r.write(TOKEN, Block::Spi1, 0x0c, 0x84400000);",
    );
    let v = prove("seed-vacuous", &dir, SPI_SETUP, 1);
    fs::remove_dir_all(&dir).unwrap();
    assert_eq!(v["verdict"], "fail");
    assert_eq!(v["tool"]["exit_code"], 0, "{v}");
    assert_eq!(v["findings"][0]["kind"], "cover_unsatisfied", "{v}");
}

// ---------------------------------------------------------------------------
// `AET-WO-3301`: `build`, `prove --spec` and `verify`, with the real tools.

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
    // cargo configuration other than the generated one, exit 3 before cargo
    // runs (`AET-WO-1603` §1.2), never a design fail.
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
        // The pre-flight, before cargo runs (`AET-WO-1603` §1.2).
        "CrateHasBuildScript",
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
    // Refused by name before cargo runs (`AET-WO-1603` §1.2); it was
    // cargo's `ToolError` before.
    refused(
        "bad-config",
        &|d| append(&d.join("firmware/.cargo/config.toml"), "\nnot toml [[\n"),
        3,
        "CargoConfigRefused",
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
/// and the support gate copied it, and before `extract` reads it, is refused
/// by the extract gate: `BindingMismatch`, exit 5, naming `extract`. The swap
/// is timed by the support gate's own copy of the board (`AET-WO-4300g`: it
/// runs after DRC and reads the board too) reaching the board's length; both
/// earlier gates judge the generated bytes, so `extract` is the first to see
/// the swap.
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
        let roots: Vec<PathBuf> = fs::read_dir(&tmp)
            .map(|r| r.filter_map(|e| Some(e.ok()?.path())).collect())
            .unwrap_or_default();
        let dirs: Vec<PathBuf> = roots.iter().map(|r| r.join("tool")).collect();
        let ours = dirs
            .iter()
            .map(|d| d.join("tree/kicad/sentinel-01.kicad_pcb"))
            .find(|p| p.is_file());
        let support = roots
            .iter()
            .map(|d| d.join("board.kicad_pcb"))
            .find(|p| p.is_file());
        if let (Some(ours), Some(support)) = (ours, support) {
            let bytes = fs::read(&ours).unwrap();
            // The support gate copies the board byte for byte, so at the
            // board's length it has read it all.
            if fs::metadata(&support).is_ok_and(|m| m.len() >= bytes.len() as u64) {
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
    assert!(
        swapped,
        "verify ended before the support gate copied the board: {v}"
    );
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

/// Replaces every string value equal to `from` in `v` with `to`.
fn rename_value(v: &mut Value, from: &str, to: &str) {
    match v {
        Value::String(s) if s == from => *s = to.to_owned(),
        Value::Array(a) => a.iter_mut().for_each(|x| rename_value(x, from, to)),
        Value::Object(m) => m.values_mut().for_each(|x| rename_value(x, from, to)),
        _ => {}
    }
}

/// `AET-WO-1102` §1.3, and the image half of the §1.1 spike (the architect's
/// ruling on PR #106): one specification carries every character `RUL-14`
/// admits in the positions of every kind, and `verify --scenarios --fab-out`
/// passes it. Then every output the run writes carries each identifier
/// byte for byte: the schematic and board (read back by the gates), the BOM,
/// the placement file, each Gerber's net attributes, the drill file, the file
/// names, the manifest, and the verdict. A character an output damages makes
/// this test fail at that assertion, and is then added to the base set's
/// refusals with the failing output named in the work order's FEEDBACK.
///
/// The base set is 83 characters (`0x21`-`0x7E` but `/ { } $ [ ] " % * , \`). A net
/// carries 13 or 14 of them in ASCII order, so each starts and ends with a
/// punctuation character or a digit of its own, and the six sort in the
/// order the router needs; the six cover all 83. The domain id, each bus
/// id and the scenario name carry as many as fit in 64 bytes. A reference is
/// a Renode identifier: three of four bytes, each holding `_`, and every
/// letter and digit is in the nets. The project name is a crate name, with
/// all of `a-z0-9-`.
#[test]
fn every_admitted_identifier_character_survives_every_output() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let admitted: Vec<char> = (0x21u8..=0x7e)
        .filter(|b| !aethelgard_ir::REFUSED_IN_IDENTIFIERS.contains(b))
        .map(char::from)
        .collect();
    assert_eq!(admitted.len(), 83);
    let slice = |from: usize, to: usize| admitted[from..to].iter().collect::<String>();
    let nets: Vec<String> = admitted
        .chunks(14)
        .enumerate()
        .map(|(i, c)| {
            // Net names are unique after case folding, and chunks 2 and 4 are
            // the upper and lower case of one run of letters: end chunk 4 on a digit.
            let tail = if i == 4 { "0" } else { "" };
            format!("{}{tail}", c.iter().collect::<String>())
        })
        .collect();
    assert_eq!(nets.len(), 6);
    let domain = format!("V{}", slice(0, 62));
    let sensors = format!("B1{}", slice(10, 60));
    let flash = format!("B2{}", slice(30, 80));
    let scenario_name = format!("S{}", slice(0, 60));
    // A reference's text is drawn on the silkscreen, and a 22-byte one overlaps
    // its own part's outline (DRC `silk_overlap`, CI run 43510641685), so the
    // references are short. Their letters and digits are the nets' too; `_` is
    // the one character only a reference admits beyond them.
    let refs: Vec<String> = ["Ua_9", "Vb_8", "Wc_7"].map(String::from).to_vec();
    let project = "abcdefghijklmnopqrstuvwxyz0123456789-x";

    let mut spec: Value = serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    for (from, to) in [
        ("sentinel-01", project),
        ("VDD_3V3", &domain),
        ("SENSORS", &sensors),
        ("FLASH", &flash),
        // The router completes nets in name order and cannot complete this
        // board in every order (`Unroutable` for a plain `N0`..`N5` in the
        // reference's own pairing), so the new names keep the reference's
        // order: FLASH_CS < I2C_SCL < I2C_SDA < SPI_MISO < SPI_MOSI < SPI_SCK.
        ("FLASH_CS", &nets[0]),
        ("I2C_SCL", &nets[1]),
        ("I2C_SDA", &nets[2]),
        ("SPI_MISO", &nets[3]),
        ("SPI_MOSI", &nets[4]),
        ("SPI_SCK", &nets[5]),
        ("U2", &refs[0]),
        ("U3", &refs[1]),
        ("U4", &refs[2]),
    ] {
        rename_value(&mut spec, from, to);
    }
    let base = std::env::temp_dir().join(format!("aet-1102-names-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let spec_path = base.join("spec.json");
    fs::write(&spec_path, serde_json::to_vec(&spec).unwrap()).unwrap();
    // The seeded set, renamed (`AET-WO-3303` D1, ruled): the first scenario
    // carries the admitted characters into the manifest.
    let droop = serde_json::json!({"droop": {"domain": domain, "floor_millivolts": 2400,
        "start_ns": 1000000, "fall_ns": 20000, "hold_ns": 200000, "rise_ns": 20000,
        "source_milliohms": 500, "load_milliamps": 100, "bor_level": 3}});
    let seeded = [
        (
            "scenario.json",
            scenario_name.as_str(),
            "pass",
            serde_json::json!([]),
            serde_json::json!([{ "bus": sensors, "pull_up_ohms": 3300 }]),
        ),
        (
            "droop.json",
            "droop",
            "pass",
            serde_json::json!([]),
            serde_json::json!([droop]),
        ),
        (
            "absent.json",
            "absent",
            "fail",
            serde_json::json!(["spi_identity_mismatch", "spi_transaction_unwitnessed"]),
            serde_json::json!([{ "spi_device_absent": refs[2] }]),
        ),
        (
            "baseline.json",
            "baseline",
            "pass",
            serde_json::json!([]),
            serde_json::json!([{ "bus": sensors, "pull_up_ohms": 2200 }]),
        ),
        // The console's open RX line, detected (`AET-WO-2302` D5).
        (
            "open.json",
            "open",
            "fail",
            serde_json::json!(["serial_echo_missing"]),
            serde_json::json!([{ "serial_rx_open": "CONSOLE" }]),
        ),
    ];
    for (file, name, expect, findings, overrides) in &seeded {
        let scenario = serde_json::json!({
            "schema_version": 3,
            "name": name,
            // Only a relative `base` inside the current directory is accepted
            // (`BaseOutsideRoot`): here, next to the scenario.
            "base": "spec.json",
            "expect": expect,
            "expect_findings": findings,
            "overrides": overrides,
        });
        fs::write(base.join(file), serde_json::to_vec(&scenario).unwrap()).unwrap();
    }
    let pkg = base.join("pkg");

    // A scenario's `base` is confined to the current directory, so this run
    // starts in the directory that holds the specification (`aethelgard()`
    // starts in the repository root).
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args([
            "verify",
            "--spec",
            "spec.json",
            "--scenarios",
            "scenario.json",
            "droop.json",
            "absent.json",
            "baseline.json",
            "open.json",
            "--fab-out",
            pkg.to_str().unwrap(),
        ])
        .current_dir(&base)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let (code, stdout) = (out.status.code(), String::from_utf8(out.stdout).unwrap());
    let v = one(&stdout);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["verdict"], "pass", "{v}");
    assert_eq!(v["fab"]["status"], "written", "{v}");
    assert_eq!(v["campaign"][0]["verdict"], "pass", "{v}");

    let files = package_files(&pkg);
    // File names: every file but the two notes starts with the project name.
    for name in files.keys() {
        assert!(
            name == "FAB-NOTES.md" || name == "MANIFEST.json" || name.starts_with(project),
            "{name}"
        );
    }
    let text = |name: &str| String::from_utf8(files[&format!("{project}{name}")].clone()).unwrap();
    // Every difference is collected, so one run names every damaged output
    // and character (the spike's table, measured in CI), and then fails.
    let mut damage: Vec<String> = vec![];
    // The BOM: one CRLF row per part, the reference its first field. No
    // admitted reference holds `,` or `"`, so a plain split reads it.
    let bom = text(".bom.csv");
    let col0: Vec<&str> = bom
        .split("\r\n")
        .filter(|l| !l.is_empty())
        .map(|l| l.split(',').next().unwrap())
        .collect();
    for r in &refs {
        if !col0.contains(&r.as_str()) {
            damage.push(format!("{r} is not in the BOM: {col0:?}"));
        }
    }
    // The placement file: KiCad quotes the reference.
    let pos = text("-pos.csv");
    for r in &refs {
        if !pos.lines().any(|l| l.starts_with(&format!("\"{r}\","))) {
            damage.push(format!("{r} is not in the placement file:\n{pos}"));
        }
    }
    // The Gerbers: every net's attribute, byte for byte; every reference
    // somewhere. What KiCad wrote is printed, whatever the outcome.
    let gerbers: Vec<String> = files
        .iter()
        .filter(|(n, _)| n.rsplit('.').next().is_some_and(|e| e.starts_with('g')))
        .map(|(_, b)| String::from_utf8_lossy(b).into_owned())
        .collect();
    assert!(!gerbers.is_empty());
    let written: std::collections::BTreeSet<&str> = gerbers
        .iter()
        .flat_map(|g| g.lines())
        .filter(|l| l.starts_with("%TO.N,"))
        .collect();
    eprintln!("AET-WO-1102 gerber net attributes: {written:?}");
    // KiCad writes a signal net as `/<net>`; a rail and GND are global, bare.
    let wanted = nets
        .iter()
        .map(|n| format!("/{n}"))
        .chain([domain.clone(), "GND".to_owned()]);
    for net in wanted {
        let want = format!("%TO.N,{net}*%");
        if !gerbers.iter().any(|g| g.contains(&want)) {
            damage.push(format!("no Gerber carries {want}"));
        }
    }
    for r in &refs {
        if !gerbers.iter().any(|g| g.contains(r.as_str())) {
            damage.push(format!("no Gerber carries {r}"));
        }
    }
    // The drill file: every net it names is one of ours, whole.
    let known: Vec<&str> = nets
        .iter()
        .chain([&domain])
        .map(String::as_str)
        .chain(["GND"])
        .collect();
    for line in text(".drl").lines() {
        if let Some(at) = line.find("TO.N,") {
            eprintln!("AET-WO-1102 drill net attribute: {line:?}");
            let name = line[at + 5..].trim_end_matches(['*', '%']);
            if !known.contains(&name) {
                damage.push(format!("the drill file names `{name}`"));
            }
        }
    }
    // The manifest and the verdict name the project and the scenario.
    let manifest: Value = serde_json::from_slice(&files["MANIFEST.json"]).unwrap();
    if manifest["project"] != project {
        damage.push(format!("the manifest names {}", manifest["project"]));
    }
    if manifest["scenarios"][0]["scenario"] != scenario_name.as_str() {
        damage.push("the manifest names another scenario".into());
    }
    if v["campaign"][0]["scenario"] != scenario_name.as_str() {
        damage.push("the verdict names another scenario".into());
    }
    assert!(damage.is_empty(), "{}", damage.join("\n"));
    // `AET-WO-2302` D5: the console's open RX line, detected beside the
    // baseline, covers its port; every precondition is met, in the verdict
    // and the manifest alike.
    let met = serde_json::json!([
        {"precondition": "every_harness_proved", "state": "met"},
        {"precondition": "campaign_faults_every_bus_and_rail", "state": "met"},
        {"precondition": "mcu_support_network_judged", "state": "met"},
        {"precondition": "every_gate_passed", "state": "met"},
    ]);
    assert_eq!(v["fab"]["preconditions"], met, "{v}");
    assert_eq!(manifest["preconditions"], met, "{manifest}");
    fs::remove_dir_all(&base).unwrap();
}

const FOUR: &str = "crates/aethelgard-cli/tests/fixtures/four-layer.json";
/// The 4-layer fixture's seeded set (`AET-WO-3303` D1, ruled): a tolerated
/// pull-up and droop, the absent flash detected, and the 2.2 kΩ baseline.
const FOUR_SCENARIOS: [&str; 4] = [
    "crates/aethelgard-cli/tests/fixtures/four-layer-3k3.json",
    "crates/aethelgard-cli/tests/fixtures/four-layer-droop.json",
    "crates/aethelgard-cli/tests/fixtures/four-layer-spi-absent.json",
    "crates/aethelgard-cli/tests/fixtures/four-layer-pullup.json",
];

/// `verify` of the 4-layer fixture with its seeded set and `flag <dir>`.
fn four_verify(dir: &Path, flag: &str) -> (Option<i32>, String) {
    let mut args = vec!["verify", "--spec", FOUR, "--scenarios"];
    args.extend(FOUR_SCENARIOS);
    args.extend([flag, dir.to_str().unwrap()]);
    aethelgard(&args)
}

/// `AET-WO-4100` §2.6 and §4: the 4-layer fixture (OSH Park's stackup, ruled
/// D2) passes every gate, DRC with schematic parity among them, by the
/// specification's rules; `--fab-out` writes both inner Gerbers, each
/// carrying its plane's region and no clock line; `FAB-NOTES.md` states the
/// layer order, each plane's net and every dielectric with its source; and
/// `--against-fab` derives the package again identical.
#[test]
fn four_layer_fixture_passes_every_gate_and_its_package_holds() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let base = std::env::temp_dir().join(format!("aet-4100-four-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let dir = base.join("pkg");
    let (code, out) = four_verify(&dir, "--fab-out");
    let v = one(&out);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["verdict"], "pass");
    for g in v["gates"].as_array().unwrap() {
        assert_eq!(g["verdict"], "pass", "{g}");
    }
    let package = package_files(&dir);
    for (name, net) in [
        ("sentinel-01-4l-In1_Cu.g1", "GND"),
        ("sentinel-01-4l-In2_Cu.g2", "VDD_3V3"),
    ] {
        let text = String::from_utf8(package[name].clone()).unwrap();
        assert!(
            text.contains(&format!("%TO.N,{net}*%\nG36*\n")),
            "{name} carries no {net} region"
        );
        assert!(!text.contains("CreationDate") && !text.contains("Created by"));
    }
    assert!(package.keys().all(|n| !n.contains("gbrjob")));
    let notes = String::from_utf8(package["FAB-NOTES.md"].clone()).unwrap();
    for want in [
        "- 4 copper layers, top to bottom, joined only by through vias:\n",
        "  - In1.Cu: plane, net GND\n",
        "  - In2.Cu: plane, net VDD_3V3\n",
        "  - F.Cu to In1.Cu: 0.179 mm (OSH Park 4-layer",
        "  - In1.Cu to In2.Cu: 0.891 mm (OSH Park 4-layer",
        "rounded down; every maximum, rounded up.",
        "Finished board from 1.25 mm to 1.851 mm thick.",
        "Finished copper at most 70 um thick on each layer.",
        "- `sentinel-01-4l-In1_Cu.g1`: Gerber, inner copper layer 1",
    ] {
        assert!(notes.contains(want), "{want}");
    }
    let manifest: Value = serde_json::from_slice(&package["MANIFEST.json"]).unwrap();
    assert!(manifest["files"].get("sentinel-01-4l-In2_Cu.g2").is_some());
    let (code, again) = four_verify(&dir, "--against-fab");
    let w = one(&again);
    assert_eq!(code, Some(0), "{w}");
    assert_eq!(w["against_fab"], "identical");
    fs::remove_dir_all(&base).unwrap();
}

/// `AET-WO-4100` §5.4: a package missing an inner Gerber, and one whose
/// `FAB-NOTES.md` states a stackup the specification does not, fail
/// `--against-fab`, naming each file.
#[test]
fn against_fab_names_a_missing_inner_layer_and_a_stackup_that_lies() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let base = std::env::temp_dir().join(format!("aet-4100-lie-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let dir = base.join("pkg");
    let (code, out) = four_verify(&dir, "--fab-out");
    assert_eq!(code, Some(0), "{out}");
    fs::remove_file(dir.join("sentinel-01-4l-In2_Cu.g2")).unwrap();
    let notes = fs::read_to_string(dir.join("FAB-NOTES.md")).unwrap();
    let lie = notes.replace("F.Cu to In1.Cu: 0.179 mm", "F.Cu to In1.Cu: 0.25 mm");
    assert_ne!(lie, notes);
    fs::write(dir.join("FAB-NOTES.md"), lie).unwrap();
    let (code, again) = four_verify(&dir, "--against-fab");
    let w = one(&again);
    assert_eq!(code, Some(1), "{w}");
    assert_eq!(w["against_fab"], "differs");
    let named: Vec<String> = w["fab"]["differences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["message"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        named
            .iter()
            .any(|m| m.starts_with("sentinel-01-4l-In2_Cu.g2")),
        "{named:?}"
    );
    assert!(
        named.iter().any(|m| m.starts_with("FAB-NOTES.md")),
        "{named:?}"
    );
    assert_eq!(named.len(), 2, "{named:?}");
    fs::remove_dir_all(&base).unwrap();
}

const CONNECTORS: &str = "crates/aethelgard-cli/tests/fixtures/connectors.json";

/// `AET-WO-4300f`: the connectors fixture (JST GH headers at the bottom
/// edge, an unconnected pin, mounting pins on GND) emits a schematic that
/// passes ERC and a board that passes DRC by the specification's rules with
/// schematic parity, with no finding of any kind, on two copper layers and
/// on four (the 4-layer fixture's board, with its planes).
#[test]
fn connectors_pass_erc_and_drc_with_parity_at_two_and_four_layers() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let base = std::env::temp_dir().join(format!("aet-4300f-gate-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let read =
        |p: &str| -> Value { serde_json::from_slice(&fs::read(root().join(p)).unwrap()).unwrap() };
    let mut four = read(CONNECTORS);
    four["board"] = read(FOUR)["board"].clone();
    let four_path = base.join("connectors-4l.json");
    fs::write(&four_path, serde_json::to_vec_pretty(&four).unwrap()).unwrap();
    for (label, spec, layers) in [("2l", root().join(CONNECTORS), 2), ("4l", four_path, 4)] {
        let out = base.join(label);
        let (code, text) = aethelgard(&[
            "emit",
            "--file",
            spec.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ]);
        let r = one(&text);
        assert_eq!(code, Some(0), "{label}: emit: {r}");
        let board = r["board"]["path"].as_str().unwrap().to_owned();
        let sch = r["schematic"]["path"].as_str().unwrap().to_owned();
        let board_text = fs::read_to_string(&board).unwrap();
        assert_eq!(
            board_text.matches(" signal)").count(),
            layers,
            "{label}: the emitted board does not have {layers} copper layers"
        );

        let (code, text) = aethelgard(&["check", "erc", "--schematic", &sch]);
        let erc = one(&text);
        assert_eq!(code, Some(0), "{label}: erc: {erc}");
        assert_eq!(erc["verdict"], "pass", "{label}: erc: {erc}");
        assert_eq!(
            erc["findings"],
            Value::Array(vec![]),
            "{label}: erc has findings"
        );

        let (code, text) = aethelgard(&[
            "check",
            "drc",
            "--board",
            &board,
            "--spec",
            spec.to_str().unwrap(),
            "--schematic",
            &sch,
        ]);
        let drc = one(&text);
        assert_eq!(code, Some(0), "{label}: drc: {drc}");
        assert_eq!(drc["verdict"], "pass", "{label}: drc: {drc}");
        assert_eq!(drc["rules"], "specification", "{label}: drc: {drc}");
        assert_eq!(
            drc["findings"],
            Value::Array(vec![]),
            "{label}: drc has findings, unconnected items included"
        );
    }
    fs::remove_dir_all(&base).unwrap();
}

/// `AET-WO-4300d2` §1.9: the Sentinel-02 power fixture (slice A's copy without
/// its crossing, which waits for the ICM-20948's footprint, FEEDBACK
/// D2) emits a schematic that passes ERC and a board that passes DRC by the
/// specification's rules with schematic parity, with no finding of any kind,
/// its loop rules measured from the board, every class named with its width,
/// and `extract` passing on every line. Then one loop part moved off its pin
/// (§5.1) fails the same gate, by its rule, and by nothing else.
#[test]
fn the_power_fixture_passes_erc_drc_its_loop_rules_and_extract() {
    use aethelgard_eda::schematic;
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let base = std::env::temp_dir().join(format!("aet-4300d2-gate-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let v = common::power::power_tree();
    let spec_path = base.join("sentinel-02-power.json");
    fs::write(&spec_path, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
    let spec_arg = spec_path.to_str().unwrap();
    let out = base.join("emitted");
    let (code, text) = aethelgard(&["emit", "--file", spec_arg, "--out", out.to_str().unwrap()]);
    let r = one(&text);
    assert_eq!(code, Some(0), "emit: {r}");
    let board = r["board"]["path"].as_str().unwrap().to_owned();
    let sch = r["schematic"]["path"].as_str().unwrap().to_owned();

    let (code, text) = aethelgard(&["check", "erc", "--schematic", &sch]);
    let erc = one(&text);
    assert_eq!(code, Some(0), "erc: {erc}");
    assert_eq!(erc["findings"], Value::Array(vec![]), "erc has findings");

    let drc = |board: &str| {
        let (code, text) = aethelgard(&[
            "check",
            "drc",
            "--board",
            board,
            "--spec",
            spec_arg,
            "--schematic",
            &sch,
        ]);
        (code, one(&text))
    };
    let (code, v) = drc(&board);
    assert_eq!(code, Some(0), "drc: {v}");
    assert_eq!(v["verdict"], "pass", "drc: {v}");
    assert_eq!(v["findings"], Value::Array(vec![]), "drc has findings");
    let loops = v["power"]["loops"].as_array().unwrap();
    assert_eq!(loops.len(), 17, "{v}");
    assert!(loops.iter().all(|l| l["verdict"] == "pass"), "{v}");
    let classes: Vec<(&str, i64)> = v["power"]["classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["name"].as_str().unwrap(), c["width_um"].as_i64().unwrap()))
        .collect();
    assert_eq!(classes, [("POWER", 500), ("SUPPLY_MCU", 200)], "{v}");
    let conditions = v["power"]["conditions"].as_array().unwrap();
    assert!(
        conditions
            .iter()
            .any(|c| c.as_str().unwrap().contains("item 5")),
        "{v}"
    );

    let (code, text) = aethelgard(&["extract", "--board", &board, "--spec", spec_arg]);
    let x = one(&text);
    assert_eq!(code, Some(0), "extract: {x}");
    assert_eq!(x["verdict"], "pass", "extract: {x}");

    // §5.1: the high-frequency input capacitor of U10 moved 25 mm off.
    let spec: aethelgard_ir::Specification =
        serde_json::from_value(common::power::power_tree()).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    let tree = aethelgard_eda::strict::board(&fs::read_to_string(&board).unwrap()).unwrap();
    let (b, _) =
        aethelgard_eda::route::read(&tree, &aethelgard_eda::stack::Stack::of(&spec)).unwrap();
    let moves = vec![(
        "C11".to_owned(),
        (b.outline.x1 + 25_000_000, b.outline.y0),
        0,
    )];
    let moved = schematic::emit_moved(&spec, &solved, &table, &moves).unwrap();
    let moved_path = base.join("moved.kicad_pcb");
    fs::write(&moved_path, moved.board).unwrap();
    let (code, v) = drc(moved_path.to_str().unwrap());
    assert_eq!(code, Some(1), "moved drc: {v}");
    let kinds: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["loop_rule"], "{v}");
    assert!(
        v["findings"][0]["message"]
            .as_str()
            .unwrap()
            .starts_with("C11 "),
        "{v}"
    );
    fs::remove_dir_all(&base).unwrap();
}

/// `AET-WO-1310` §2: the committed Sentinel-02 power fixture, its crossing
/// and the ICM-20948 included, routes (U5's 0.2 mm pads at 0.4 mm pitch leave
/// by escapes off the grid) and passes every gate it reaches: ERC with no
/// finding; DRC by the specification's rules with schematic parity, with no
/// finding of any kind (U5's keepout in board coordinates, so no
/// `lib_footprint_mismatch`: D4) and every loop rule passing; the support
/// gate. `extract` then refuses it by name (`4300d2` D6 (b)): the
/// translator's data pins have no stated maximum capacitance.
#[test]
fn the_full_power_fixture_routes_and_passes_every_gate_it_reaches() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let base = std::env::temp_dir().join(format!("aet-1310-power-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let spec = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../aethelgard-ir/tests/fixtures/sentinel-02-power.json");
    let spec_arg = spec.to_str().unwrap();
    let out = base.join("emitted");
    let (code, text) = aethelgard(&["emit", "--file", spec_arg, "--out", out.to_str().unwrap()]);
    let r = one(&text);
    assert_eq!(code, Some(0), "emit: {r}");
    let board = r["board"]["path"].as_str().unwrap().to_owned();
    let sch = r["schematic"]["path"].as_str().unwrap().to_owned();

    let (code, text) = aethelgard(&["check", "erc", "--schematic", &sch]);
    let erc = one(&text);
    assert_eq!(code, Some(0), "erc: {erc}");
    assert_eq!(erc["findings"], Value::Array(vec![]), "erc has findings");

    let (code, text) = aethelgard(&[
        "check",
        "drc",
        "--board",
        &board,
        "--spec",
        spec_arg,
        "--schematic",
        &sch,
    ]);
    let v = one(&text);
    assert_eq!(code, Some(0), "drc: {v}");
    assert_eq!(v["verdict"], "pass", "drc: {v}");
    assert_eq!(v["findings"], Value::Array(vec![]), "drc has findings: {v}");
    let loops = v["power"]["loops"].as_array().unwrap();
    assert!(
        !loops.is_empty() && loops.iter().all(|l| l["verdict"] == "pass"),
        "{v}"
    );

    let (code, text) = aethelgard(&[
        "check",
        "support",
        "--schematic",
        &sch,
        "--board",
        &board,
        "--spec",
        spec_arg,
    ]);
    let support = one(&text);
    assert_eq!(code, Some(0), "support: {support}");
    assert_eq!(support["verdict"], "pass", "support: {support}");

    let (code, text) = aethelgard(&["extract", "--board", &board, "--spec", spec_arg]);
    let x = one(&text);
    assert_ne!(code, Some(0), "extract: {x}");
    assert_eq!(x["error"]["kind"], "TranslatorPinUnbounded", "extract: {x}");
    fs::remove_dir_all(&base).unwrap();
}

/// `AET-WO-4300g` §1.5, on KiCad's own netlist: the reference's schematic,
/// exported by `kicad-cli sch export netlist` in the sandbox, passes the
/// support gate with every role in its bound, J2 the programming path and J3
/// the source. Then the board with one VDD capacitor moved 25 mm off its pin
/// (the schematic unchanged) fails it, by that role and by nothing else.
#[test]
fn the_reference_passes_the_support_gate_and_a_capacitor_moved_off_fails() {
    use aethelgard_eda::schematic;
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let base = std::env::temp_dir().join(format!("aet-4300g-support-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let spec_path = root().join("examples/sentinel-01-minimal.json");
    let spec_arg = spec_path.to_str().unwrap();
    let out = base.join("emitted");
    let (code, text) = aethelgard(&["emit", "--file", spec_arg, "--out", out.to_str().unwrap()]);
    let r = one(&text);
    assert_eq!(code, Some(0), "emit: {r}");
    let board = r["board"]["path"].as_str().unwrap().to_owned();
    let sch = r["schematic"]["path"].as_str().unwrap().to_owned();
    let support = |board: &str| {
        let (code, text) = aethelgard(&[
            "check",
            "support",
            "--schematic",
            &sch,
            "--board",
            board,
            "--spec",
            spec_arg,
        ]);
        (code, one(&text))
    };
    let (code, v) = support(&board);
    assert_eq!(code, Some(0), "support: {v}");
    assert_eq!(v["verdict"], "pass", "{v}");
    assert_eq!(v["findings"], Value::Array(vec![]), "{v}");
    let s = &v["support"];
    let roles = s["roles"].as_array().unwrap();
    assert_eq!(roles.len(), 19, "{v}");
    assert!(roles.iter().all(|x| x["verdict"] == "pass"), "{v}");
    assert_eq!(s["programming"]["connector"], "J2", "{v}");
    assert_eq!(s["supply_source"], "J3 pin 1", "{v}");
    let moved_part = roles
        .iter()
        .find(|x| x["role"] == "vdd_bypass" && x["pin"] == "52")
        .unwrap()["part"]
        .as_str()
        .unwrap()
        .to_owned();

    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&fs::read(&spec_path).unwrap()).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    let tree = aethelgard_eda::strict::board(&fs::read_to_string(&board).unwrap()).unwrap();
    let (b, _) =
        aethelgard_eda::route::read(&tree, &aethelgard_eda::stack::Stack::of(&spec)).unwrap();
    let moves = vec![(
        moved_part.clone(),
        (b.outline.x1 + 25_000_000, b.outline.y0),
        0,
    )];
    let moved = schematic::emit_moved(&spec, &solved, &table, &moves).unwrap();
    let moved_path = base.join("moved.kicad_pcb");
    fs::write(&moved_path, moved.board).unwrap();
    let (code, v) = support(moved_path.to_str().unwrap());
    assert_eq!(code, Some(1), "moved: {v}");
    let kinds: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["support_capacitor_missing"], "{v}");
    assert!(
        v["findings"][0]["message"]
            .as_str()
            .unwrap()
            .contains("VDD 52"),
        "{v}"
    );
    fs::remove_dir_all(&base).unwrap();
}

const BENCH: &str = "crates/aethelgard-cli/tests/fixtures/sentinel-01-bench.json";

/// `AET-WO-1312` §2: Sentinel-01 with a test point on every net §1.1 lists,
/// BOOT0 and NRST on its SWD connector, and a reset switch passes `validate`,
/// `generate`, `emit`, `check erc` and `check drc --spec` with no finding,
/// parity included, at two layers and at four (the 4-layer fixture's board).
/// The support gate judges BOOT0's pull-down and the switch from KiCad's own
/// netlist, and `extract` bounds every line with its test pad on it.
#[test]
fn the_bench_board_passes_every_gate_at_two_and_four_layers() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let base = std::env::temp_dir().join(format!("aet-1312-bench-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let read =
        |p: &str| -> Value { serde_json::from_slice(&fs::read(root().join(p)).unwrap()).unwrap() };
    let mut four = read(BENCH);
    four["board"] = read(FOUR)["board"].clone();
    let four_path = base.join("bench-4l.json");
    fs::write(&four_path, serde_json::to_vec_pretty(&four).unwrap()).unwrap();
    for (label, spec, layers) in [("2l", root().join(BENCH), 2), ("4l", four_path, 4)] {
        let spec_arg = spec.to_str().unwrap();
        let (code, text) = aethelgard(&["validate", "--file", spec_arg]);
        assert_eq!(code, Some(0), "{label}: validate: {text}");
        let generated = base.join(format!("{label}-gen"));
        let (code, text) = aethelgard(&[
            "generate",
            "--spec",
            spec_arg,
            "--out",
            generated.to_str().unwrap(),
        ]);
        assert_eq!(code, Some(0), "{label}: generate: {text}");
        let out = base.join(label);
        let (code, text) =
            aethelgard(&["emit", "--file", spec_arg, "--out", out.to_str().unwrap()]);
        let r = one(&text);
        assert_eq!(code, Some(0), "{label}: emit: {r}");
        let board = r["board"]["path"].as_str().unwrap().to_owned();
        let sch = r["schematic"]["path"].as_str().unwrap().to_owned();
        let board_text = fs::read_to_string(&board).unwrap();
        assert_eq!(board_text.matches(" signal)").count(), layers, "{label}");
        for tp in 1..=12 {
            assert!(
                board_text.contains(&format!("\"Reference\" \"TP{tp}\"")),
                "{label}: TP{tp} is not on the board"
            );
        }
        assert!(board_text.contains("\"Reference\" \"SW1\""), "{label}");

        let (code, text) = aethelgard(&["check", "erc", "--schematic", &sch]);
        let erc = one(&text);
        assert_eq!(code, Some(0), "{label}: erc: {erc}");
        assert_eq!(erc["verdict"], "pass", "{label}: erc: {erc}");
        assert_eq!(erc["findings"], Value::Array(vec![]), "{label}: erc");

        for parity in [false, true] {
            let mut args = vec!["check", "drc", "--board", &board, "--spec", spec_arg];
            if parity {
                args.extend(["--schematic", &sch]);
            }
            let (code, text) = aethelgard(&args);
            let drc = one(&text);
            assert_eq!(code, Some(0), "{label} parity={parity}: drc: {drc}");
            assert_eq!(drc["verdict"], "pass", "{label}: drc: {drc}");
            assert_eq!(drc["rules"], "specification", "{label}: drc: {drc}");
            assert_eq!(drc["findings"], Value::Array(vec![]), "{label}: drc");
        }

        let (code, text) = aethelgard(&[
            "check",
            "support",
            "--schematic",
            &sch,
            "--board",
            &board,
            "--spec",
            spec_arg,
        ]);
        let v = one(&text);
        assert_eq!(code, Some(0), "{label}: support: {v}");
        assert_eq!(v["findings"], Value::Array(vec![]), "{label}: {v}");
        let straps = v["support"]["straps"].to_string();
        assert!(
            straps.contains("BOOT0 (pin 138) on `/BOOT0`, pulled to `GND` by")
                && straps.contains("(10k)"),
            "{label}: {straps}"
        );

        let (code, text) = aethelgard(&["extract", "--board", &board, "--spec", spec_arg]);
        let x = one(&text);
        assert_eq!(code, Some(0), "{label}: extract: {x}");
        assert_eq!(x["verdict"], "pass", "{label}: extract: {x}");
        for line in x["lines"].as_array().unwrap() {
            let pins = line["pins"].to_string();
            assert!(
                !pins.contains("TP"),
                "{label}: a test pad counted as a pin: {line}"
            );
        }
    }
    fs::remove_dir_all(&base).unwrap();
}

/// `AET-WO-4220b` §2: the Sentinel-02-shaped fixture, its ICM-42688-P and
/// BMP581 drawn from our own library (`Aethelgard_Sensor`), passes ERC with
/// no finding, DRC by the specification's rules with schematic parity and no
/// finding of any kind (each footprint's keepouts judged, no
/// `lib_footprint_mismatch`), and `extract`, each sensor's pins counted at
/// its declared value and named as declared. A track drawn across the
/// BMP581's keepout, on either layer, fails DRC (§5.3).
#[test]
fn the_own_parts_fixture_passes_erc_drc_with_parity_and_extract() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let base = std::env::temp_dir().join(format!("aet-4220b-gate-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let spec_path = root().join("crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json");
    let spec_arg = spec_path.to_str().unwrap();
    let out = base.join("emitted");
    let (code, text) = aethelgard(&["emit", "--file", spec_arg, "--out", out.to_str().unwrap()]);
    let r = one(&text);
    assert_eq!(code, Some(0), "emit: {r}");
    let board = r["board"]["path"].as_str().unwrap().to_owned();
    let sch = r["schematic"]["path"].as_str().unwrap().to_owned();
    let pcb = fs::read_to_string(&board).unwrap();
    for (reference, footprint) in [
        ("U5", "Aethelgard_Sensor:TDK_LGA-14_3x2.5mm_P0.5mm"),
        ("U7", "Aethelgard_Sensor:Bosch_LGA-10_2x2mm_P0.5mm"),
    ] {
        let at = pcb
            .find(&format!("(property \"Reference\" \"{reference}\""))
            .unwrap();
        let start = pcb[..at].rfind("(footprint ").unwrap();
        assert!(
            pcb[start..].starts_with(&format!("(footprint \"{footprint}\"")),
            "{reference}"
        );
    }

    let (code, text) = aethelgard(&["check", "erc", "--schematic", &sch]);
    let erc = one(&text);
    assert_eq!(code, Some(0), "erc: {erc}");
    assert_eq!(erc["findings"], Value::Array(vec![]), "erc has findings");

    let drc = |board: &str| {
        let (code, text) = aethelgard(&[
            "check",
            "drc",
            "--board",
            board,
            "--spec",
            spec_arg,
            "--schematic",
            &sch,
        ]);
        (code, one(&text))
    };
    let (code, v) = drc(&board);
    assert_eq!(code, Some(0), "drc: {v}");
    assert_eq!(v["verdict"], "pass", "drc: {v}");
    assert_eq!(v["findings"], Value::Array(vec![]), "drc has findings");

    let (code, text) = aethelgard(&["extract", "--board", &board, "--spec", spec_arg]);
    let x = one(&text);
    assert_eq!(code, Some(0), "extract: {x}");
    assert_eq!(x["verdict"], "pass", "extract: {x}");
    let declared: Vec<&str> = x["lines"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|l| l["declared_pins"].as_array().into_iter().flatten())
        .map(|p| p.as_str().unwrap())
        .collect();
    assert_eq!(declared, ["U7.4", "U7.2", "U5.13", "U5.14"], "{x}");

    // §5.3: a 0.2 mm track of U7's own SDA net straight across U7's centre,
    // on F.Cu and then on B.Cu, is a track inside its keepout.
    let (u7x, u7y) = {
        let at = pcb.find("(property \"Reference\" \"U7\"").unwrap();
        let start = pcb[..at].rfind("(footprint ").unwrap();
        let at = &pcb[start..][pcb[start..].find("(at ").unwrap() + 4..];
        let mut it = at[..at.find(')').unwrap()].split_whitespace();
        let x: f64 = it.next().unwrap().parse().unwrap();
        let y: f64 = it.next().unwrap().parse().unwrap();
        (x, y)
    };
    for layer in ["F.Cu", "B.Cu"] {
        let track = format!(
            "\t(segment\n\t\t(start {:.4} {:.4})\n\t\t(end {:.4} {:.4})\n\t\t(width 0.2)\n\t\t(layer \"{layer}\")\n\t\t(net \"/I2C_SDA\")\n\t\t(uuid \"4220b000-0000-4000-8000-00000000000{}\")\n\t)\n",
            u7x - 0.3,
            u7y,
            u7x + 0.3,
            u7y,
            if layer == "F.Cu" { 1 } else { 2 },
        );
        let end = pcb.rfind(')').unwrap();
        let seeded = format!("{}{track}{}", &pcb[..end], &pcb[end..]);
        let path = base.join(format!("under-{layer}.kicad_pcb"));
        fs::write(&path, seeded).unwrap();
        let (code, v) = drc(path.to_str().unwrap());
        assert_eq!(code, Some(1), "a track under U7 on {layer}: {v}");
        // KiCad names a track inside a keepout `items_not_allowed`; the
        // seeded track's loose ends add `track_dangling` and
        // `unconnected_items`, which are not the claim.
        let uuid = format!(
            "4220b000-0000-4000-8000-00000000000{}",
            if layer == "F.Cu" { 1 } else { 2 }
        );
        let in_keepout = v["findings"].as_array().unwrap().iter().any(|f| {
            f["kind"] == "items_not_allowed"
                && f["where"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|w| w["uuid"] == uuid.as_str())
        });
        assert!(in_keepout, "{layer}: {v}");
    }
    fs::remove_dir_all(&base).unwrap();
}

/// `AET-WO-1313` §3: the Sentinel-03 fixture, inside its declared
/// `board.max_outline_mm` (100 x 100 mm) at four layers, with its connectors
/// on the left and right edges, passes `validate`, `generate`, `emit`, ERC,
/// DRC by the specification's rules with and without schematic parity,
/// `check support` and `extract`, each with no finding.
#[test]
fn the_sentinel_03_fixture_passes_every_gate_inside_its_bound() {
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let base = std::env::temp_dir().join(format!("aet-1313-outline-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    let spec_path = root().join("crates/aethelgard-cli/tests/fixtures/sentinel-03.json");
    let spec_arg = spec_path.to_str().unwrap();
    let (code, text) = aethelgard(&["validate", "--file", spec_arg]);
    assert_eq!(code, Some(0), "validate: {text}");
    let generated = base.join("gen");
    let (code, text) = aethelgard(&[
        "generate",
        "--spec",
        spec_arg,
        "--out",
        generated.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "generate: {text}");
    let out = base.join("emitted");
    let (code, text) = aethelgard(&["emit", "--file", spec_arg, "--out", out.to_str().unwrap()]);
    let r = one(&text);
    assert_eq!(code, Some(0), "emit: {r}");
    let board = r["board"]["path"].as_str().unwrap().to_owned();
    let sch = r["schematic"]["path"].as_str().unwrap().to_owned();
    let board_text = fs::read_to_string(&board).unwrap();
    assert_eq!(board_text.matches(" signal)").count(), 4);
    // The outline, from the board's one Edge.Cuts rectangle.
    let rect = &board_text[board_text.find("(gr_rect").unwrap()..];
    let num = |key: &str| -> (f64, f64) {
        let s = &rect[rect.find(key).unwrap() + key.len()..];
        let mut it = s.split([' ', ')']).filter(|t| !t.is_empty());
        (
            it.next().unwrap().parse().unwrap(),
            it.next().unwrap().parse().unwrap(),
        )
    };
    let ((x0, y0), (x1, y1)) = (num("(start "), num("(end "));
    assert!(
        x1 - x0 <= 100.0 && y1 - y0 <= 100.0,
        "{} x {}",
        x1 - x0,
        y1 - y0
    );

    let (code, text) = aethelgard(&["check", "erc", "--schematic", &sch]);
    let erc = one(&text);
    assert_eq!(code, Some(0), "erc: {erc}");
    assert_eq!(erc["findings"], Value::Array(vec![]), "erc: {erc}");
    for parity in [false, true] {
        let mut args = vec!["check", "drc", "--board", &board, "--spec", spec_arg];
        if parity {
            args.extend(["--schematic", &sch]);
        }
        let (code, text) = aethelgard(&args);
        let drc = one(&text);
        assert_eq!(code, Some(0), "parity={parity}: drc: {drc}");
        assert_eq!(drc["verdict"], "pass", "drc: {drc}");
        assert_eq!(drc["rules"], "specification", "drc: {drc}");
        assert_eq!(drc["findings"], Value::Array(vec![]), "drc: {drc}");
    }
    let (code, text) = aethelgard(&[
        "check",
        "support",
        "--schematic",
        &sch,
        "--board",
        &board,
        "--spec",
        spec_arg,
    ]);
    let v = one(&text);
    assert_eq!(code, Some(0), "support: {v}");
    assert_eq!(v["findings"], Value::Array(vec![]), "support: {v}");
    let (code, text) = aethelgard(&["extract", "--board", &board, "--spec", spec_arg]);
    let x = one(&text);
    assert_eq!(code, Some(0), "extract: {x}");
    assert_eq!(x["verdict"], "pass", "extract: {x}");
    assert_eq!(x["findings"], Value::Array(vec![]), "extract: {x}");
    fs::remove_dir_all(&base).unwrap();
}
