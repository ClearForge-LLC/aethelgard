//! AET-WO-1600 spike: measurements, printed as `ANSWER` lines on the real
//! stderr (libtest captures `io::stderr`, not an appended `/dev/stderr`), so
//! they show in the CI log. The build deletes this file and `spike/1600/`.
//!
//! Every build (`gates`, `tools`, Furnace): the candidate programs checked by a
//! BPF interpreter against the kernel's own headers; the probe run inside the
//! real sandbox (the baseline: what is open today); the S4 capability crate,
//! built offline beside the workspace and run.
//!
//! With `with-tools` (the seam in `run.rs` exists only then): the probe under
//! every variant, and the whole tier-2 suite, `with_tools`, under every
//! variant and under the capability drop.

use std::collections::BTreeMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

#[allow(dead_code)]
#[path = "../../../spike/1600/hook.rs"]
mod hook;

fn answer(line: &str) {
    let mut err = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
    writeln!(err, "ANSWER {line}").unwrap();
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn where_am_i() -> String {
    let kernel = fs::read_to_string("/proc/sys/kernel/osrelease").unwrap_or_default();
    let uid = fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find_map(|l| {
            l.strip_prefix("Uid:")
                .map(|v| v.split_whitespace().next().unwrap_or("?").to_owned())
        })
        .unwrap_or_default();
    let image = Path::new("/.dockerenv").exists() || Path::new("/run/.containerenv").exists();
    format!("kernel={} uid={uid} container={image}", kernel.trim())
}

// ---------------------------------------------------------------- the BPF

/// A classic-BPF interpreter for exactly the opcodes these programs use.
fn run_bpf(p: &[seccompiler::sock_filter], nr: u32, arch: u32, args: [u64; 6]) -> u32 {
    let mut data = [0u8; 64];
    data[0..4].copy_from_slice(&nr.to_le_bytes());
    data[4..8].copy_from_slice(&arch.to_le_bytes());
    for (i, a) in args.iter().enumerate() {
        data[16 + 8 * i..24 + 8 * i].copy_from_slice(&a.to_le_bytes());
    }
    let (mut pc, mut acc) = (0usize, 0u32);
    loop {
        let op = &p[pc];
        let jump = |taken: bool| pc + 1 + usize::from(if taken { op.jt } else { op.jf });
        pc = match op.code {
            0x20 => {
                let k = op.k as usize;
                acc = u32::from_le_bytes(data[k..k + 4].try_into().unwrap());
                pc + 1
            }
            0x54 => {
                acc &= op.k;
                pc + 1
            }
            0x15 => jump(acc == op.k),
            0x45 => jump(acc & op.k != 0),
            0x06 => return op.k,
            other => panic!("opcode {other:#x} at {pc}"),
        };
        assert!(pc < p.len(), "fell off the program");
    }
}

/// `name → number` from the kernel's x86_64 table, `__NR_name number`.
fn kernel_numbers() -> Option<BTreeMap<String, u32>> {
    let text = fs::read_to_string("/usr/include/x86_64-linux-gnu/asm/unistd_64.h").ok()?;
    Some(
        text.lines()
            .filter_map(|l| {
                let mut w = l.strip_prefix("#define __NR_")?.split_whitespace();
                Some((w.next()?.to_owned(), w.next()?.parse().ok()?))
            })
            .collect(),
    )
}

#[test]
fn every_variant_does_what_its_spec_says_under_an_interpreter() {
    const X86_64: u32 = 0xc000_003e;
    const I386: u32 = 0x4000_0003;
    const ALLOW: u32 = 0x7fff_0000;
    const KILL: u32 = 0x8000_0000;
    let errno = |e: u32| 0x0005_0000 | e;
    // Every name in the spec is the kernel's number for that name.
    match kernel_numbers() {
        Some(table) => {
            let mut checked = 0;
            for set in [hook::SYSV, hook::NS, hook::PIDFD, hook::KERNEL] {
                for &(name, nr) in set {
                    assert_eq!(table.get(name), Some(&nr), "{name}");
                    checked += 1;
                }
            }
            assert_eq!(table.get("clone"), Some(&hook::NR_CLONE));
            assert_eq!(table.get("clone3"), Some(&hook::NR_CLONE3));
            answer(&format!(
                "bpf numbers checked={} against unistd_64.h ({})",
                checked + 2,
                where_am_i()
            ));
        }
        None => answer("bpf numbers SKIP: no /usr/include/x86_64-linux-gnu/asm/unistd_64.h"),
    }
    // The clone mask is exactly the kernel's CLONE_NEW* bits that fit clone(2).
    if let Ok(sched) = fs::read_to_string("/usr/include/linux/sched.h") {
        let mut mask = 0u32;
        for l in sched.lines() {
            let mut w = l.split_whitespace();
            if w.next() == Some("#define")
                && let (Some(name), Some(v)) = (w.next(), w.next())
                && name.starts_with("CLONE_NEW")
            {
                let v = u64::from_str_radix(v.trim_start_matches("0x"), 16).unwrap();
                if v & 0xff == 0 && v <= u64::from(u32::MAX) {
                    mask |= v as u32;
                }
            }
        }
        assert_eq!(mask, hook::CLONE_NEW_MASK);
        answer(&format!(
            "bpf clone_new_mask={mask:#x} matches linux/sched.h"
        ));
    }
    let variants = [
        "allow",
        "sysv",
        "ns",
        "pidfd",
        "kernel",
        "s1",
        "s2-enosys",
        "s2-eperm",
        "clone3-enosys",
        "clone3-eperm",
    ];
    for v in variants {
        let spec = hook::spec(v).unwrap();
        let p = hook::program(v).unwrap();
        for nr in 0..1024u32 {
            let want = if spec.eperm.contains(&nr) {
                errno(1)
            } else if nr == hook::NR_CLONE3 && spec.clone3.is_some() {
                errno(spec.clone3.unwrap())
            } else {
                ALLOW
            };
            if nr == hook::NR_CLONE {
                // A thread, a fork and a vfork are allowed; each namespace bit is not.
                for flags in [0x11u64, 0x003d_0f00, 0x4111] {
                    assert_eq!(
                        run_bpf(&p, nr, X86_64, [flags, 0, 0, 0, 0, 0]),
                        ALLOW,
                        "{v} clone {flags:#x}"
                    );
                }
                for bit in 0..32 {
                    let flags = 0x11u64 | (1 << bit);
                    let refused = spec.clone_new && hook::CLONE_NEW_MASK & (1 << bit) != 0;
                    let got = run_bpf(&p, nr, X86_64, [flags | (0xffff_ffff << 32), 0, 0, 0, 0, 0]);
                    assert_eq!(
                        got,
                        if refused { errno(1) } else { ALLOW },
                        "{v} clone bit {bit}"
                    );
                }
                continue;
            }
            assert_eq!(run_bpf(&p, nr, X86_64, [0; 6]), want, "{v} nr {nr}");
            // x32 and i386 are killed, whatever the number.
            assert_eq!(
                run_bpf(&p, nr | 0x4000_0000, X86_64, [0; 6]),
                KILL,
                "{v} x32 {nr}"
            );
            assert_eq!(run_bpf(&p, nr, I386, [0; 6]), KILL, "{v} i386 {nr}");
        }
        answer(&format!(
            "bpf variant={v} instructions={} eperm={} clone_new={} clone3={:?} interpreter=agrees",
            p.len(),
            spec.eperm.len(),
            spec.clone_new,
            spec.clone3
        ));
    }
    assert!(hook::program("bogus").is_none());
}

// ---------------------------------------------------------------- the probe

fn probe_bin() -> PathBuf {
    let p = Path::new(env!("CARGO_BIN_EXE_aethelgard"))
        .parent()
        .unwrap()
        .join("examples/spike_1600_probe");
    assert!(
        p.exists(),
        "{} is missing: `cargo test` builds it",
        p.display()
    );
    p
}

struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let d = env::temp_dir().join(format!("aet-1600-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.join("toolbox")).unwrap();
        symlink(probe_bin(), d.join("toolbox/kicad-cli")).unwrap();
        symlink(probe_bin(), d.join("toolbox/renode")).unwrap();
        fs::write(d.join("probe.kicad_pcb"), "probe\n").unwrap();
        fs::write(d.join("p.repl"), "cpu: CPU.CortexM @ sysbus\n").unwrap();
        Self(d)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn cli(s: &Scratch, args: &[&str], variant: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aethelgard"));
    cmd.args(args)
        .current_dir(&s.0)
        .env_clear()
        .env("PATH", s.0.join("toolbox"))
        .stdin(Stdio::null());
    if let Some(v) = variant {
        cmd.env(hook::VAR, v);
    }
    cmd.output().unwrap()
}

/// The probe's findings as `name=result`, or the refusal the CLI gave.
fn probe_line(s: &Scratch, variant: Option<&str>) -> String {
    let out = cli(s, &["check", "drc", "--board", "probe.kicad_pcb"], variant);
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stdout)));
    match v["findings"].as_array() {
        Some(f) => f
            .iter()
            .map(|f| f["message"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
            .join(" "),
        None => format!("refused exit={:?} {}", out.status.code(), v),
    }
}

/// The renode persona's `[ERROR]` line: the tool's own capability sets.
fn status_line(s: &Scratch, variant: Option<&str>) -> String {
    let out = cli(s, &["simulate", "--platform", "p.repl"], variant);
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stdout)));
    let msg = v["error"]["message"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| v.to_string());
    match msg.find("spike-1600 ") {
        Some(i) => msg[i + 11..].to_owned(),
        None => format!("unexpected {msg}"),
    }
}

/// The same calls made outside the sandbox: a channel refused inside is the
/// sandbox's doing only where it works here.
fn controls() -> String {
    let run = |p: &str, a: &[&str]| {
        Command::new(p)
            .args(a)
            .stdin(Stdio::null())
            .output()
            .map(|o| {
                if o.status.success() {
                    (true, String::from_utf8_lossy(&o.stdout).into_owned())
                } else {
                    (
                        false,
                        String::from_utf8_lossy(&o.stderr)
                            .lines()
                            .next()
                            .unwrap_or("")
                            .to_owned(),
                    )
                }
            })
            .unwrap_or((false, "spawn failed".to_owned()))
    };
    let mut line = String::new();
    for (name, make, rm) in [
        ("shmget", &["-M", "4096"][..], "-m"),
        ("msgget", &["-Q"][..], "-q"),
        ("semget", &["-S", "1"][..], "-s"),
    ] {
        let (ok, out) = run("/usr/bin/ipcmk", make);
        if ok {
            let id = out.rsplit(':').next().unwrap_or("").trim().to_owned();
            let (rm_ok, _) = run("/usr/bin/ipcrm", &[rm, &id]);
            line.push_str(&format!("{name}=ok;rm={rm_ok} "));
        } else {
            line.push_str(&format!("{name}=fail:{out} "));
        }
    }
    let (ok, out) = run("/usr/bin/unshare", &["--user", "/usr/bin/true"]);
    line.push_str(&format!(
        "unshare_user={}",
        if ok {
            "ok".to_owned()
        } else {
            format!("fail:{out}")
        }
    ));
    line
}

#[test]
fn the_probe_measures_the_baseline_inside_the_sandbox() {
    let s = Scratch::new("base");
    let at = where_am_i();
    answer(&format!("probe controls-outside ({at}) {}", controls()));
    answer(&format!(
        "probe variant=none ({at}) {}",
        probe_line(&s, None)
    ));
    answer(&format!(
        "status variant=none ({at}) {}",
        status_line(&s, None)
    ));
}

#[cfg(feature = "with-tools")]
#[test]
fn the_probe_under_every_variant() {
    let s = Scratch::new("variants");
    let at = where_am_i();
    for v in [
        "bogus",
        "allow",
        "sysv",
        "ns",
        "pidfd",
        "kernel",
        "s1",
        "clone3-enosys",
        "clone3-eperm",
        "s2-enosys",
        "s2-eperm",
    ] {
        answer(&format!(
            "probe variant={v} ({at}) {}",
            probe_line(&s, Some(v))
        ));
    }
    answer(&format!(
        "status variant=s2-enosys ({at}) {}",
        status_line(&s, Some("s2-enosys"))
    ));
}

// ---------------------------------------------------------------- S4 caps

/// Builds `spike/1600/caps` offline into its own target directory (a nested
/// cargo must not share the outer build's lock).
fn caps_bin() -> PathBuf {
    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo().join("target"))
        .join("spike-1600");
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let out = Command::new(cargo)
        .args([
            "build",
            "--offline",
            "--locked",
            "--quiet",
            "--manifest-path",
        ])
        .arg(repo().join("spike/1600/caps/Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "building spike/1600/caps: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    target.join("debug/aet-1600-caps")
}

#[test]
fn s4_the_capability_drop_with_safe_rustix() {
    let bin = caps_bin();
    let at = where_am_i();
    for order in ["forward", "reverse"] {
        let out = Command::new(&bin).args(["drop", order]).output().unwrap();
        for l in String::from_utf8_lossy(&out.stdout).lines() {
            answer(&format!("{} ({at})", l.trim_start_matches("ANSWER ")));
        }
    }
}

// ---------------------------------------------------------------- tier 2

/// The newest `with_tools` test binary beside this one that has tests.
#[cfg(feature = "with-tools")]
fn with_tools_bin() -> PathBuf {
    let me = env::current_exe().unwrap();
    let deps = me.parent().unwrap();
    fs::read_dir(deps)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with("with_tools-") && !n.contains('.')
        })
        // A build without `with-tools` has the same name and no tests.
        .filter(|p| {
            Command::new(p)
                .args(["--list", "--format", "terse"])
                .output()
                .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).contains(": test"))
        })
        .max_by_key(|p| fs::metadata(p).and_then(|m| m.modified()).ok())
        .expect("a with_tools test binary that lists tier-2 tests")
}

#[cfg(feature = "with-tools")]
#[test]
fn tier2_under_every_variant_and_the_capability_drop() {
    use std::time::Instant;
    let suite = with_tools_bin();
    let caps = caps_bin();
    let at = where_am_i();
    let runs: &[(&str, Option<&str>, bool)] = &[
        ("none", None, false),
        ("bogus", Some("bogus"), false),
        ("allow", Some("allow"), false),
        ("sysv", Some("sysv"), false),
        ("ns", Some("ns"), false),
        ("pidfd", Some("pidfd"), false),
        ("kernel", Some("kernel"), false),
        ("s1", Some("s1"), false),
        ("clone3-enosys", Some("clone3-enosys"), false),
        ("clone3-eperm", Some("clone3-eperm"), false),
        ("s2-enosys", Some("s2-enosys"), false),
        ("s2-eperm", Some("s2-eperm"), false),
        ("caps", None, true),
        ("caps+s2-enosys", Some("s2-enosys"), true),
    ];
    for &(label, variant, drop_caps) in runs {
        let mut cmd = if drop_caps {
            let mut c = Command::new(&caps);
            c.arg("exec").arg(&suite);
            c
        } else {
            Command::new(&suite)
        };
        cmd.stdin(Stdio::null()).env_remove(hook::VAR);
        if let Some(v) = variant {
            cmd.env(hook::VAR, v);
        }
        let started = Instant::now();
        let out = cmd.output().unwrap();
        let secs = started.elapsed().as_secs_f32();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let (mut passed, mut failed) = (Vec::new(), Vec::new());
        for l in stdout.lines() {
            if let Some(rest) = l.strip_prefix("test ") {
                if let Some(name) = rest.strip_suffix(" ... ok") {
                    passed.push(name.to_owned());
                } else if let Some(name) = rest.strip_suffix(" ... FAILED") {
                    failed.push(name.to_owned());
                }
            }
        }
        answer(&format!(
            "tier2 run={label} ({at}) exit={:?} secs={secs:.1} passed={} failed={} failing={failed:?}",
            out.status.code(),
            passed.len(),
            failed.len()
        ));
        if passed.is_empty() && failed.is_empty() {
            answer(&format!(
                "tier2 run={label} no-tests stdout={:?} stderr={:?}",
                stdout.chars().take(400).collect::<String>(),
                String::from_utf8_lossy(&out.stderr)
                    .chars()
                    .take(400)
                    .collect::<String>()
            ));
        }
        // Why the failures failed: each one's refusal kind, and one sample.
        let mut kinds: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        let mut sample = String::new();
        for name in &failed {
            let head = format!("---- {name} stdout ----");
            let body = stdout
                .split(&head)
                .nth(1)
                .map(|b| b.split("\n---- ").next().unwrap_or(b))
                .unwrap_or("");
            let kind = [
                "SandboxUnavailable",
                "ToolError",
                "ToolUnusable",
                "ToolMissing",
            ]
            .into_iter()
            .find(|k| body.contains(k))
            .unwrap_or("other");
            kinds.entry(kind).or_default().push(name);
            if sample.is_empty() {
                let flat: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
                sample = format!("{name}: {}", flat.chars().take(300).collect::<String>());
            }
        }
        if !failed.is_empty() {
            let counts: Vec<String> = kinds
                .iter()
                .map(|(k, v)| format!("{k}={}", v.len()))
                .collect();
            answer(&format!(
                "tier2 run={label} kinds {} sample={sample:?}",
                counts.join(" ")
            ));
        }
    }
}
