//! AET-WO-1600 spike S4: can safe `rustix` empty the bounding set, clear the
//! effective, permitted and inheritable sets and the ambient set, with
//! `no_new_privs` still set afterwards? No `unsafe` (forbidden in Cargo.toml).
//!
//! - `report`: one `ANSWER caps state …` line from `/proc/self/status` and
//!   `rustix::thread::capabilities`.
//! - `drop forward|reverse`: sets `no_new_privs` (as Landlock already has),
//!   then runs the three drops in that order, prints each step's result and
//!   the state, then execs itself with `report`, so the last line is what a
//!   program started after the drop holds. Forward is bounding, ambient,
//!   capset; reverse is capset, ambient, bounding.
//! - `exec PROGRAM ARGS…`: the forward drop, then exec. Any failed step exits
//!   70 without exec, so nothing runs half-dropped.

use std::env;
use std::fs;
use std::os::unix::process::CommandExt;
use std::process::{Command, exit};

use rustix::thread::{
    CapabilitySet, CapabilitySets, capabilities, capability_is_in_bounding_set,
    clear_ambient_capability_set, no_new_privs, remove_capability_from_bounding_set,
    set_capabilities, set_no_new_privs,
};

fn status_field(key: &str) -> String {
    fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find_map(|l| l.strip_prefix(key)?.strip_prefix(':'))
        .map(|v| v.split_whitespace().next().unwrap_or("").to_owned())
        .unwrap_or_else(|| "?".to_owned())
}

fn state() -> String {
    let api = match capabilities(None) {
        Ok(s) => format!(
            "api_eff={:x} api_prm={:x} api_inh={:x}",
            s.effective.bits(),
            s.permitted.bits(),
            s.inheritable.bits()
        ),
        Err(e) => format!("api=errno={}", e.raw_os_error()),
    };
    format!(
        "uid={} inh={} prm={} eff={} bnd={} amb={} nnp={} nnp_api={} {api}",
        status_field("Uid"),
        status_field("CapInh"),
        status_field("CapPrm"),
        status_field("CapEff"),
        status_field("CapBnd"),
        status_field("CapAmb"),
        status_field("NoNewPrivs"),
        no_new_privs().map_or_else(|e| format!("errno={}", e.raw_os_error()), |b| b.to_string()),
    )
}

fn last_cap() -> u32 {
    fs::read_to_string("/proc/sys/kernel/cap_last_cap")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(40)
}

/// Drops every capability still in the bounding set, including any number
/// past rustix's named ones. `ok(n)` or `errno=E(first failure, n tried)`.
fn bounding() -> Result<String, String> {
    let (mut tried, mut failed) = (0, None);
    for n in 0..=last_cap() {
        let cap = CapabilitySet::from_bits_retain(1u64 << n);
        match capability_is_in_bounding_set(cap) {
            Ok(false) => continue,
            Ok(true) => {}
            Err(e) => {
                failed.get_or_insert(format!("read cap {n} errno={}", e.raw_os_error()));
                continue;
            }
        }
        tried += 1;
        if let Err(e) = remove_capability_from_bounding_set(cap) {
            failed.get_or_insert(format!("drop cap {n} errno={}", e.raw_os_error()));
        }
    }
    match failed {
        None => Ok(format!("ok({tried})")),
        Some(f) => Err(format!("{f}({tried} tried)")),
    }
}

fn ambient() -> Result<String, String> {
    clear_ambient_capability_set()
        .map(|()| "ok".to_owned())
        .map_err(|e| format!("errno={}", e.raw_os_error()))
}

fn capset() -> Result<String, String> {
    let empty = CapabilitySets {
        effective: CapabilitySet::empty(),
        permitted: CapabilitySet::empty(),
        inheritable: CapabilitySet::empty(),
    };
    set_capabilities(None, empty)
        .map(|()| "ok".to_owned())
        .map_err(|e| format!("errno={}", e.raw_os_error()))
}

/// Runs the drop; every step's result, and whether all succeeded.
fn drop_all(reverse: bool) -> (String, bool) {
    let nnp = set_no_new_privs(true).map_or_else(|e| format!("errno={}", e.raw_os_error()), |()| "ok".to_owned());
    let mut steps: Vec<(&str, fn() -> Result<String, String>)> =
        vec![("bounding", bounding), ("ambient", ambient), ("capset", capset)];
    if reverse {
        steps.reverse();
    }
    let mut all_ok = nnp == "ok";
    let mut line = format!("set_nnp={nnp}");
    for (name, step) in steps {
        let r = step();
        all_ok &= r.is_ok();
        line.push_str(&format!(" {name}={}", r.unwrap_or_else(|e| e)));
    }
    (line, all_ok)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let me = env::current_exe().unwrap_or_else(|_| exit(70));
    match args.get(1).map(String::as_str) {
        Some("report") => println!("ANSWER caps after-exec {}", state()),
        Some("drop") => {
            let reverse = args.get(2).map(String::as_str) == Some("reverse");
            let order = if reverse { "reverse" } else { "forward" };
            println!("ANSWER caps {order} before {}", state());
            let (steps, ok) = drop_all(reverse);
            println!("ANSWER caps {order} steps {steps} all_ok={ok}");
            println!("ANSWER caps {order} after {}", state());
            let err = Command::new(me).arg("report").exec();
            println!("ANSWER caps {order} exec-error {err}");
            exit(70);
        }
        Some("exec") if args.len() > 2 => {
            let (steps, ok) = drop_all(false);
            if !ok {
                eprintln!("aet-1600-caps: drop failed: {steps}");
                exit(70);
            }
            let err = Command::new(&args[2]).args(&args[3..]).exec();
            eprintln!("aet-1600-caps: exec {}: {err}", args[2]);
            exit(70);
        }
        _ => {
            eprintln!("usage: aet-1600-caps report | drop forward|reverse | exec PROGRAM ARGS…");
            exit(64);
        }
    }
}
