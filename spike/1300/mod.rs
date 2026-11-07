//! AET-WO-1300 spike: the probes that answer §0 S1-S5 inside the tools image.
//! Spike only; the build half deletes `spike/1300/` and its test hook.
//!
//! Run by `crates/aethelgard-cli/tests/spike_1300.rs` (feature `with-tools`),
//! so CI's `tools` job runs it inside the image pinned by `container/IMAGE`
//! with `--network none`. Every measurement is one `ANSWER <Sn> ...` line on
//! the job's stderr, written past libtest's capture. A measurement never
//! panics; a probe that cannot measure prints `blocked: <reason>`.

// Spike code: signatures are shaped by the probes, not an API.
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

pub mod kilib;
pub mod pcb;
pub mod sch;
pub mod sexpr;

mod s1;
mod s2;
mod s3;
mod s4;
mod s5;

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;
use sha2::{Digest, Sha256};

/// A UUID derived from content: sha256 of a domain-separated key, the first
/// 16 bytes, with RFC 9562's version 8 (custom) and variant bits set.
pub fn uuid(key: &str) -> String {
    let h = Sha256::digest(format!("aethelgard/spike-1300\0{key}").as_bytes());
    let mut b = [0u8; 16];
    b.copy_from_slice(&h[..16]);
    b[6] = (b[6] & 0x0f) | 0x80;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// One `ANSWER` line per call, on the process's real stderr (libtest
/// captures only the print macros).
pub fn answer(id: &str, text: impl AsRef<str>) {
    let flat = text.as_ref().replace('\n', " | ");
    let mut err = std::io::stderr().lock();
    let _ = err.write_all(format!("ANSWER {id} {flat}\n").as_bytes());
}

pub struct Ran {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub ms: u128,
}

impl Ran {
    pub fn code_text(&self) -> String {
        self.code
            .map_or_else(|| "signal".to_owned(), |c| c.to_string())
    }
}

/// Runs a program to completion (5 min cap), output to files so no pipe can
/// fill and stall it.
pub fn run(program: &str, args: &[&str], cwd: &Path) -> Result<Ran, String> {
    let out = cwd.join(".spike-stdout");
    let err = cwd.join(".spike-stderr");
    let started = Instant::now();
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(fs::File::create(&out).map_err(|e| e.to_string())?)
        .stderr(fs::File::create(&err).map_err(|e| e.to_string())?)
        .spawn()
        .map_err(|e| format!("{program}: {e}"))?;
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|e| e.to_string())? {
            break s;
        }
        if started.elapsed() > Duration::from_secs(300) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("{program} {args:?}: killed after 300 s"));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let read = |p: &Path| String::from_utf8_lossy(&fs::read(p).unwrap_or_default()).into_owned();
    let ran = Ran {
        code: status.code(),
        stdout: read(&out),
        stderr: read(&err),
        ms: started.elapsed().as_millis(),
    };
    let _ = fs::remove_file(out);
    let _ = fs::remove_file(err);
    Ok(ran)
}

#[derive(Clone, Debug)]
pub struct Finding {
    pub kind: String,
    pub severity: String,
    pub description: String,
    /// (description, uuid) per item.
    pub items: Vec<(String, String)>,
}

fn findings_of(list: &Value) -> Vec<Finding> {
    list.as_array()
        .map(|a| {
            a.iter()
                .map(|f| Finding {
                    kind: f["type"].as_str().unwrap_or("?").to_owned(),
                    severity: f["severity"].as_str().unwrap_or("?").to_owned(),
                    description: f["description"].as_str().unwrap_or("").to_owned(),
                    items: f["items"]
                        .as_array()
                        .map(|items| {
                            items
                                .iter()
                                .map(|i| {
                                    (
                                        i["description"].as_str().unwrap_or("").to_owned(),
                                        i["uuid"].as_str().unwrap_or("").to_owned(),
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// "N: kind×n (severity), ..." sorted by kind.
pub fn summary(findings: &[Finding]) -> String {
    let mut by: BTreeMap<(String, String), usize> = BTreeMap::new();
    for f in findings {
        *by.entry((f.kind.clone(), f.severity.clone())).or_default() += 1;
    }
    let parts: Vec<String> = by
        .iter()
        .map(|((k, s), n)| format!("{k}x{n}({s})"))
        .collect();
    format!("{} [{}]", findings.len(), parts.join(", "))
}

pub struct Ctx {
    pub work: PathBuf,
    pub aethelgard: PathBuf,
    pub kicad: Option<String>,
    pub libs: Option<kilib::Libs>,
    /// Symbol name to `lib:name`, from S1's scan.
    pub found: BTreeMap<String, String>,
}

impl Ctx {
    pub fn dir(&self, name: &str) -> Result<PathBuf, String> {
        let d = self.work.join(name);
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
        Ok(d)
    }

    pub fn kicad(&self, args: &[&str], cwd: &Path) -> Result<Ran, String> {
        run("kicad-cli", args, cwd)
    }

    /// `kicad-cli sch erc --exit-code-violations --format json` on `sch`.
    pub fn erc(&self, sch: &Path) -> Result<(Ran, Vec<Finding>), String> {
        let report = sch.with_extension("erc.json");
        let ran = self.kicad(
            &[
                "sch",
                "erc",
                "--exit-code-violations",
                "--format",
                "json",
                "-o",
                &report.to_string_lossy(),
                &sch.to_string_lossy(),
            ],
            sch.parent().unwrap(),
        )?;
        let doc = read_json(&report).map_err(|e| {
            format!(
                "erc exit {}: {e}; stderr: {}",
                ran.code_text(),
                ran.stderr.trim()
            )
        })?;
        let mut all = Vec::new();
        for sheet in doc["sheets"].as_array().cloned().unwrap_or_default() {
            all.extend(findings_of(&sheet["violations"]));
        }
        Ok((ran, all))
    }

    /// `kicad-cli pcb drc --exit-code-violations --format json [extra]` on
    /// `pcb`; findings by report key.
    pub fn drc(
        &self,
        pcb: &Path,
        extra: &[&str],
    ) -> Result<(Ran, BTreeMap<String, Vec<Finding>>), String> {
        let report = pcb.with_extension("drc.json");
        let report_s = report.to_string_lossy().into_owned();
        let pcb_s = pcb.to_string_lossy().into_owned();
        let mut args = vec!["pcb", "drc", "--exit-code-violations", "--format", "json"];
        args.extend_from_slice(extra);
        args.extend_from_slice(&["-o", &report_s, &pcb_s]);
        let ran = self.kicad(&args, pcb.parent().unwrap())?;
        let doc = read_json(&report).map_err(|e| {
            format!(
                "drc exit {}: {e}; stderr: {}",
                ran.code_text(),
                ran.stderr.trim()
            )
        })?;
        let mut by = BTreeMap::new();
        for key in ["violations", "unconnected_items", "schematic_parity"] {
            by.insert(key.to_owned(), findings_of(&doc[key]));
        }
        Ok((ran, by))
    }

    /// `aethelgard check <check> --<flag> <path>`: the verdict line.
    pub fn check(&self, check: &str, flag: &str, path: &Path) -> Result<(Ran, Value), String> {
        let ran = run(
            &self.aethelgard.to_string_lossy(),
            &["check", check, flag, &path.to_string_lossy()],
            path.parent().unwrap(),
        )?;
        let v: Value = serde_json::from_str(ran.stdout.trim()).map_err(|e| {
            format!(
                "aethelgard check {check}: exit {}, stdout not JSON ({e}): {}",
                ran.code_text(),
                ran.stdout.trim()
            )
        })?;
        Ok((ran, v))
    }
}

pub fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// A verdict's findings as `kind` counts.
pub fn verdict_summary(v: &Value) -> String {
    let mut by: BTreeMap<String, usize> = BTreeMap::new();
    for f in v["findings"].as_array().cloned().unwrap_or_default() {
        *by.entry(f["kind"].as_str().unwrap_or("?").to_owned())
            .or_default() += 1;
    }
    format!(
        "ok={} verdict={} exit_code={} findings={:?}",
        v["ok"], v["verdict"], v["tool"]["exit_code"], by
    )
}

pub fn write(path: &Path, text: &str) -> Result<(), String> {
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Runs every probe; returns the harness errors (not measurements).
pub fn main(aethelgard: &Path, tmp: &Path) -> Vec<String> {
    let work = tmp.join("spike-1300");
    let _ = fs::remove_dir_all(&work);
    let _ = fs::create_dir_all(&work);
    let kicad = run("kicad-cli", &["version"], &work)
        .ok()
        .filter(|r| r.code == Some(0))
        .map(|r| r.stdout.trim().to_owned());
    let libs = kilib::Libs::find();
    answer(
        "ENV",
        format!(
            "kicad-cli={} libs={} image={}",
            kicad.as_deref().unwrap_or("absent"),
            match &libs {
                Ok(l) => format!("{} + {}", l.sym_dir.display(), l.fp_dir.display()),
                Err(e) => format!("absent ({e})"),
            },
            std::env::var("AETHELGARD_IMAGE").unwrap_or_else(|_| "unnamed".into())
        ),
    );
    let mut ctx = Ctx {
        work,
        aethelgard: aethelgard.to_path_buf(),
        kicad,
        libs: libs.ok(),
        found: BTreeMap::new(),
    };
    let probes: [(&str, fn(&mut Ctx) -> Result<(), String>); 5] = [
        ("S1", s1::run),
        ("S2", s2::run),
        ("S3", s3::run),
        ("S4", s4::run),
        ("S5", s5::run),
    ];
    let mut errors = Vec::new();
    for (id, probe) in probes {
        let started = Instant::now();
        if let Err(e) = probe(&mut ctx) {
            answer(id, format!("harness error: {e}"));
            errors.push(format!("{id}: {e}"));
        }
        answer(
            id,
            format!("probe time {} ms", started.elapsed().as_millis()),
        );
    }
    errors
}
