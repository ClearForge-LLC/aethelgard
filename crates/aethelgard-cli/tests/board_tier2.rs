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
    SPEC, aethelgard, board, build, copy_tree, gate_reads, one, package_files, root, sha,
};

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Instant;

use serde_json::Value;

const HARNESS: &str = "proofs::every_valid_sequence";

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

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
    let scenario = serde_json::json!({
        "schema_version": 1,
        "name": scenario_name,
        // Only a relative `base` inside the current directory is accepted
        // (`BaseOutsideRoot`): here, next to the scenario.
        "base": "spec.json",
        "expect": "pass",
        "expect_findings": [],
        "overrides": [{ "bus": sensors, "pull_up_ohms": 2200 }],
    });
    let scenario_path = base.join("scenario.json");
    fs::write(&scenario_path, serde_json::to_vec(&scenario).unwrap()).unwrap();
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
    fs::remove_dir_all(&base).unwrap();
}
