//! AET-WO-1600 spike S3: the distinct system calls each tool made, from one
//! `strace -f -qq` log of the tier-2 suite (or any run of the CLI).
//!
//! A process belongs to a tool from the moment the Landlock trampoline
//! (`aethelgard --aethelgard-internal-confine`) execs it; everything it
//! starts, and everything they exec, inherits the label. The label is the
//! tool's file name (`kicad-cli`, `renode`, `cargo`, …). Processes outside a
//! tool (the test runner, the CLI, the reaper, the trampoline before exec)
//! are counted as `outside`, so nothing is silently dropped.
//!
//! Usage: `aet-1600-census TRACE [--calls]` prints, per label, the number of
//! distinct calls and with `--calls` the sorted list; then which of S1's calls
//! each label made, and with what results.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::exit;

const S1: &[&str] = &[
    "shmget", "shmat", "shmdt", "shmctl", "msgget", "msgsnd", "msgrcv", "msgctl", "semget",
    "semop", "semtimedop", "semctl", "unshare", "setns", "clone3", "pidfd_open", "pidfd_getfd",
    "pidfd_send_signal", "ptrace", "process_vm_readv", "process_vm_writev", "kcmp", "keyctl",
    "add_key", "request_key", "bpf", "perf_event_open", "userfaultfd",
];
const CLONE_NEW: &[&str] = &[
    "CLONE_NEWNS", "CLONE_NEWCGROUP", "CLONE_NEWUTS", "CLONE_NEWIPC", "CLONE_NEWUSER",
    "CLONE_NEWPID", "CLONE_NEWNET", "CLONE_NEWTIME",
];

#[derive(Default)]
struct Census {
    label: HashMap<u32, String>,
    /// A pid's `execve` path while its result is pending.
    pending_exec: HashMap<u32, (String, bool)>,
    calls: BTreeMap<String, BTreeMap<String, u64>>,
    /// label → S1 call (or `clone+NEW`) → result text → count.
    s1: BTreeMap<String, BTreeMap<String, BTreeMap<String, u64>>>,
    lines: u64,
    unparsed: u64,
}

/// The text after the last `)` that is followed by padding and `= `: strace
/// pads results to a column, and an errno's description has its own `)`.
fn raw_result(rest: &str) -> Option<&str> {
    rest.match_indices(')')
        .filter_map(|(i, _)| rest[i + 1..].trim_start().strip_prefix("= "))
        .next_back()
        .map(str::trim)
}

fn result_of(rest: &str) -> Option<String> {
    let r = raw_result(rest)?;
    // "-1 EPERM (Operation not permitted)" → "-1 EPERM"; "3" → "ok".
    Some(if r.starts_with('-') {
        r.split_whitespace().take(2).collect::<Vec<_>>().join(" ")
    } else if r.starts_with('?') {
        "?".to_owned()
    } else {
        "ok".to_owned()
    })
}

impl Census {
    fn label(&self, pid: u32) -> String {
        self.label.get(&pid).cloned().unwrap_or_else(|| "outside".to_owned())
    }

    fn line(&mut self, l: &str) {
        self.lines += 1;
        let Some((pid, rest)) = l.split_once(' ') else {
            self.unparsed += 1;
            return;
        };
        let Ok(pid) = pid.parse::<u32>() else {
            self.unparsed += 1;
            return;
        };
        let rest = rest.trim_start();
        if rest.starts_with("+++") || rest.starts_with("---") {
            return;
        }
        let (name, resumed) = if let Some(r) = rest.strip_prefix("<... ") {
            (r.split(' ').next().unwrap_or(""), true)
        } else {
            (rest.split('(').next().unwrap_or(""), false)
        };
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            self.unparsed += 1;
            return;
        }
        let label = self.label(pid);
        if !resumed {
            *self
                .calls
                .entry(label.clone())
                .or_default()
                .entry(name.to_owned())
                .or_default() += 1;
        }
        let result = result_of(rest);
        // Labels follow forks and execs.
        if matches!(name, "clone" | "clone3" | "fork" | "vfork")
            && let Some(r) = raw_result(rest)
            && let Ok(child) = r.split_whitespace().next().unwrap_or("").parse::<u32>()
            && child > 0
        {
            self.label.insert(child, label.clone());
        }
        if name == "execve" {
            if !resumed {
                let path = rest
                    .split('"')
                    .nth(1)
                    .unwrap_or("")
                    .to_owned();
                let confine = rest.contains("--aethelgard-internal-confine");
                self.pending_exec.insert(pid, (path, confine));
            }
            if result.as_deref() == Some("ok")
                && let Some((path, confine)) = self.pending_exec.remove(&pid)
            {
                let new = if confine {
                    "trampoline".to_owned()
                } else if label == "trampoline" {
                    path.rsplit('/').next().unwrap_or(&path).to_owned()
                } else {
                    label.clone()
                };
                self.label.insert(pid, new);
            }
        }
        // S1's calls, and clone with a namespace bit, with their results.
        let s1_name = if S1.contains(&name) {
            Some(name.to_owned())
        } else if name == "clone" && !resumed && CLONE_NEW.iter().any(|f| rest.contains(f)) {
            Some("clone+NEW".to_owned())
        } else {
            None
        };
        if let (Some(n), Some(r)) = (s1_name, result.clone()) {
            *self
                .s1
                .entry(label)
                .or_default()
                .entry(n)
                .or_default()
                .entry(r)
                .or_default() += 1;
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("usage: aet-1600-census TRACE [--calls]");
        exit(64);
    };
    let list = args.iter().any(|a| a == "--calls");
    let file = File::open(path).unwrap_or_else(|e| {
        eprintln!("{path}: {e}");
        exit(66)
    });
    let mut c = Census::default();
    for l in BufReader::new(file).lines() {
        match l {
            Ok(l) => c.line(&l),
            Err(_) => c.unparsed += 1,
        }
    }
    println!("ANSWER census lines={} unparsed={}", c.lines, c.unparsed);
    let mut union = BTreeSet::new();
    for (label, calls) in &c.calls {
        if label != "outside" && label != "trampoline" {
            union.extend(calls.keys().cloned());
        }
        let names: Vec<&str> = calls.keys().map(String::as_str).collect();
        println!(
            "ANSWER census label={label} distinct={} calls={}",
            names.len(),
            calls.values().sum::<u64>()
        );
        if list {
            println!("ANSWER census label={label} list={}", names.join(","));
        }
    }
    println!("ANSWER census tools-union distinct={}", union.len());
    if list {
        println!(
            "ANSWER census tools-union list={}",
            union.into_iter().collect::<Vec<_>>().join(",")
        );
    }
    for (label, calls) in &c.s1 {
        let parts: Vec<String> = calls
            .iter()
            .map(|(n, rs)| {
                let rs: Vec<String> = rs.iter().map(|(r, k)| format!("{r}x{k}")).collect();
                format!("{n}[{}]", rs.join("|"))
            })
            .collect();
        println!("ANSWER census s1 label={label} {}", parts.join(" "));
    }
}
