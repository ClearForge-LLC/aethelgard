//! S3 probe: run CMD confined by Landlock.
//!
//!   confine --ro P1:P2:... --rw P3:... -- CMD...
//!
//! Read-only access beneath each --ro path, full access beneath each --rw
//! path, nothing elsewhere. Missing paths are skipped and listed. Prints one
//! `landlock ...` line to stderr with the kernel's status and how fully the
//! ruleset was enforced, then becomes CMD.

#![forbid(unsafe_code)]

use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, exit};

use landlock::{
    ABI, Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr, path_beneath_rules,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(sep) = args.iter().position(|a| a == "--") else {
        eprintln!("usage: confine --ro P:... --rw P:... -- CMD...");
        exit(64);
    };
    let (mut ro, mut rw) = (Vec::new(), Vec::new());
    let mut i = 1;
    while i < sep {
        let list = args.get(i + 1).map(String::as_str).unwrap_or("");
        let target = match args[i].as_str() {
            "--ro" => &mut ro,
            "--rw" => &mut rw,
            other => {
                eprintln!("unknown flag {other}");
                exit(64);
            }
        };
        for p in list.split(':').filter(|p| !p.is_empty()) {
            if Path::new(p).exists() {
                target.push(p.to_owned());
            } else {
                eprintln!("landlock-skip missing {p}");
            }
        }
        i += 2;
    }
    let abi = ABI::V8;
    let status = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .and_then(|r| r.create())
        .and_then(|r| r.add_rules(path_beneath_rules(&ro, AccessFs::from_read(abi))))
        .and_then(|r| r.add_rules(path_beneath_rules(&rw, AccessFs::from_all(abi))))
        .and_then(|r| r.restrict_self());
    match status {
        Ok(s) => eprintln!(
            "landlock {:?} enforced={:?} no_new_privs={}",
            s.landlock, s.ruleset, s.no_new_privs
        ),
        Err(e) => {
            eprintln!("landlock error {e}");
            exit(70);
        }
    }
    let err = Command::new(&args[sep + 1]).args(&args[sep + 2..]).exec();
    eprintln!("exec: {err}");
    exit(70);
}
