//! AET-WO-2101 S4 probe: one owner (this process, standing in for the CLI)
//! drives Renode and ngspice in lock-step, quantum by quantum:
//!
//! 1. Renode, through its monitor on a piped stdin: `emulation RunFor <q>`,
//!    then the per-quantum reads between whole-line sentinels;
//! 2. ngspice, to the same virtual time;
//! 3. the fault state for the next quantum, set in the sensor model by a
//!    monitor command, from the measured rise time against the ceiling.
//!
//! It writes three files: the state sequence (simulator state only, no wall
//! clock: hashed for INV-04), the per-quantum wall-clock costs, and the event
//! log for S6, where the measured rise time and the firmware's observed
//! effect are separate records with separate sources.
//!
//! ngspice backends (`--spice`):
//! - `pipe`: route b, a long-lived `ngspice -p` under `script`'s pty (44.2's
//!   pipe mode segfaults on a non-tty stdin), advanced by `stop when time > T`
//!   and `resume`;
//! - `lib`: route c, libngspice in-process, the same commands through
//!   `ngSpice_Command`;
//! - `ioc`: route c by inversion of control: one `tran`, breakpoints at every
//!   quantum boundary, and Renode advanced from inside `GetSyncData` when the
//!   solver lands on one, so the analog side never pauses;
//! - `none`: Renode alone, the baseline.
//!
//! This file has no `unsafe`; the FFI is `ng-probe`'s (`../ngspice/src/ffi.rs`).

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread;
use std::time::{Duration, Instant};

use ng_probe::{Hooks, Spice};

const WAIT: Duration = Duration::from_secs(120);
const RISE_CMD: &str =
    "meas tran aetrise TRIG v(line) VAL=0.99 RISE=1 TARG v(line) VAL=2.31 RISE=1";

/// ngspice prints its prompt (`ngspice 12 -> `) with no newline, so the
/// next output shares its line; every leading prompt is removed.
fn strip_prompt(mut line: &str) -> &str {
    while let Some(rest) = line.strip_prefix("ngspice ") {
        match rest.split_once(" -> ") {
            Some((n, after)) if n.bytes().all(|b| b.is_ascii_digit()) => line = after,
            _ => break,
        }
    }
    line
}

/// A child driven line by line: commands on stdin, stdout and stderr merged
/// into one channel of lines, each with stray carriage returns removed.
struct Driven {
    name: &'static str,
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<String>,
    serial: u64,
}

fn pump(pipe: impl Read + Send + 'static, tx: Sender<String>) {
    thread::spawn(move || {
        for line in BufReader::new(pipe).lines() {
            let Ok(line) = line else { break };
            if tx
                .send(strip_prompt(line.trim_matches('\r')).to_owned())
                .is_err()
            {
                break;
            }
        }
    });
}

impl Driven {
    fn spawn(name: &'static str, cmd: &mut Command) -> Self {
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot start {name}: {e}"));
        let (tx, rx) = channel();
        pump(child.stdout.take().expect("stdout"), tx.clone());
        pump(child.stderr.take().expect("stderr"), tx);
        let stdin = child.stdin.take().expect("stdin");
        Self {
            name,
            child,
            stdin,
            rx,
            serial: 0,
        }
    }

    /// Sends the commands, then a sentinel, and returns every line up to the
    /// sentinel's own output line (a whole-line match, so the echo of the
    /// command that prints it never matches).
    fn exec(&mut self, cmds: &[String], echo: fn(&str) -> String) -> Vec<String> {
        self.serial += 1;
        let sentinel = format!("AET2101-{}-{}", self.name.to_uppercase(), self.serial);
        let mut text = String::new();
        for c in cmds {
            text.push_str(c);
            text.push('\n');
        }
        text.push_str(&echo(&sentinel));
        text.push('\n');
        self.stdin
            .write_all(text.as_bytes())
            .and_then(|()| self.stdin.flush())
            .unwrap_or_else(|e| panic!("{}: write failed: {e}", self.name));
        let mut lines = Vec::new();
        loop {
            match self.rx.recv_timeout(WAIT) {
                Ok(line) if line.trim() == sentinel => return lines,
                Ok(line) => lines.push(line),
                Err(RecvTimeoutError::Timeout) => panic!(
                    "{}: no sentinel {sentinel} in {WAIT:?}; last lines: {:?}",
                    self.name,
                    lines.iter().rev().take(10).collect::<Vec<_>>()
                ),
                Err(RecvTimeoutError::Disconnected) => panic!(
                    "{}: output ended before {sentinel}; last lines: {:?}",
                    self.name,
                    lines.iter().rev().take(20).collect::<Vec<_>>()
                ),
            }
        }
    }

    fn quit(mut self, cmd: &str) {
        let _ = writeln!(self.stdin, "{cmd}");
        drop(self.stdin);
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn renode_echo(s: &str) -> String {
    format!("echo \"{s}\"")
}

fn spice_echo(s: &str) -> String {
    format!("echo {s}")
}

fn run_for(ns: u64) -> String {
    format!(
        "emulation RunFor \"{}.{:09}\"",
        ns / 1_000_000_000,
        ns % 1_000_000_000
    )
}

/// The value after the first `=` on the first line that starts with `key`.
fn spice_value(lines: &[String], key: &str) -> Option<String> {
    lines.iter().find_map(|l| {
        let rest = l.trim().strip_prefix(key)?;
        let v = rest.trim_start().strip_prefix('=')?;
        Some(v.split_whitespace().next()?.to_owned())
    })
}

/// A Renode read's value: a whole line that is a bare number (hex or
/// decimal); command echoes and log lines are skipped.
fn renode_values(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|l| l.trim())
        .filter(|l| {
            l.strip_prefix("0x")
                .map(|h| !h.is_empty() && h.bytes().all(|b| b.is_ascii_hexdigit()))
                .unwrap_or_else(|| !l.is_empty() && l.bytes().all(|b| b.is_ascii_digit()))
        })
        .map(str::to_owned)
        .collect()
}

/// The owner's state: Renode, the fault policy, and the three records.
struct Owner {
    renode: Driven,
    reads: Vec<String>,
    fault_cmd: Option<String>,
    /// Sent only when the fault state changes (a removal cannot be repeated).
    fault_on: Option<String>,
    fault_off: Option<String>,
    quantum_ns: u64,
    ceiling_ns: f64,
    trise: Option<f64>,
    fault: bool,
    last_fw: Vec<String>,
    seq: String,
    costs: String,
    events: String,
}

impl Owner {
    /// Step 1: Renode runs one quantum, then the reads.
    fn renode_step(&mut self) -> (Vec<String>, u128) {
        let t0 = Instant::now();
        let mut cmds = vec![run_for(self.quantum_ns)];
        cmds.extend(self.reads.iter().cloned());
        let out = self.renode.exec(&cmds, renode_echo);
        (renode_values(&out), t0.elapsed().as_micros())
    }

    /// Steps 2's result and 3: record the analog side, set the fault state for
    /// the next quantum, and append the quantum to every record.
    #[allow(clippy::too_many_arguments)]
    fn finish(
        &mut self,
        k: u64,
        fw: Vec<String>,
        renode_us: u128,
        spice_t: String,
        spice_v: String,
        measured: Option<f64>,
        spice_us: u128,
    ) {
        let t_ns = k * self.quantum_ns;
        if self.trise.is_none()
            && let Some(ns) = measured
        {
            self.trise = Some(ns);
            // S6: the evidence, from the analog solver.
            self.events.push_str(&format!(
                "{{\"kind\":\"analog.rise_time_measured\",\"source\":\"ngspice\",\"quantum\":{k},\"t_virtual_ns\":{t_ns},\"net\":\"line\",\"rise_30_70_ns\":{ns:.4},\"ceiling_ns\":{},\"exceeds\":{}}}\n",
                self.ceiling_ns,
                ns > self.ceiling_ns
            ));
        }
        let t2 = Instant::now();
        let want = self.trise.is_some_and(|ns| ns > self.ceiling_ns);
        if let Some(fc) = self.fault_cmd.clone() {
            let cmd = fc.replace("{}", if want { "1" } else { "0" });
            self.renode.exec(&[cmd], renode_echo);
        }
        if want != self.fault {
            let edge = if want { &self.fault_on } else { &self.fault_off };
            if let Some(cmd) = edge.clone() {
                self.renode.exec(&[cmd], renode_echo);
            }
            if self.fault_cmd.is_some() || edge.is_some() {
                self.events.push_str(&format!(
                    "{{\"kind\":\"bridge.fault_state\",\"source\":\"owner\",\"quantum\":{k},\"t_virtual_ns\":{t_ns},\"fault\":{want},\"from\":\"analog.rise_time_measured > ceiling_ns\"}}\n"
                ));
            }
        }
        self.fault = want;
        let fault_us = t2.elapsed().as_micros();
        // S6: the effect, from the firmware's own result block in RAM: the
        // reads that are not the core's (`sysbus.cpu ...`), so the event marks
        // a change in what the firmware recorded, not every executed instruction.
        let effect: Vec<String> = fw
            .iter()
            .zip(&self.reads)
            .filter(|(_, r)| !r.starts_with("sysbus.cpu"))
            .map(|(v, _)| v.clone())
            .collect();
        if effect != self.last_fw {
            self.events.push_str(&format!(
                "{{\"kind\":\"firmware.observed\",\"source\":\"renode\",\"quantum\":{k},\"t_virtual_ns\":{t_ns},\"reads\":[{}]}}\n",
                effect
                    .iter()
                    .map(|v| format!("\"{v}\""))
                    .collect::<Vec<_>>()
                    .join(",")
            ));
            self.last_fw = effect;
        }
        self.seq.push_str(&format!(
            "q={k} t_ns={t_ns} renode={} spice_t={spice_t} spice_v={spice_v} trise={} fault={}\n",
            fw.join(","),
            self.trise.map_or("-".to_owned(), |v| format!("{v:.6}")),
            u8::from(self.fault)
        ));
        self.costs
            .push_str(&format!("{k},{renode_us},{spice_us},{fault_us}\n"));
    }
}

/// A command-driven ngspice: the pipe child, or the library through
/// `ngSpice_Command` with its output captured by `SendChar`.
enum Stepped {
    Pipe(Driven),
    Lib(Spice<Quiet>),
}

/// Hooks for the command-driven library: the deck's sources are its own.
struct Quiet;

impl Hooks for Quiet {
    fn vsrc(&mut self, _time: f64, _name: &str) -> f64 {
        0.0
    }
    fn sync(&mut self, _t: f64, _d: &mut f64, _o: f64, _r: i32, _l: i32) -> i32 {
        0
    }
}

impl Stepped {
    fn exec(&mut self, cmds: &[String]) -> Vec<String> {
        match self {
            Stepped::Pipe(d) => d.exec(cmds, spice_echo),
            Stepped::Lib(sp) => {
                for c in cmds {
                    sp.command(c);
                }
                sp.take_log()
                    .iter()
                    .map(|l| {
                        l.strip_prefix("stdout ")
                            .or_else(|| l.strip_prefix("stderr "))
                            .unwrap_or(l)
                            .to_owned()
                    })
                    .collect()
            }
        }
    }
}

/// The inversion-of-control hooks: they own the owner, and run its quantum
/// from inside `GetSyncData` whenever the solver lands on a boundary.
struct Ioc {
    owner: Option<Owner>,
    quanta: u64,
    next_k: u64,
    q_s: f64,
    since: Instant,
    // the online 30 %-70 % detector over accepted points
    prev: Option<(f64, f64)>,
    t30: Option<f64>,
    measured: Option<f64>,
}

impl Ioc {
    /// One quantum at a boundary the solver has reached: Renode to the same
    /// time, then the analog record and the fault state.
    fn boundary(&mut self, time: f64) {
        let (Some(owner), true) = (self.owner.as_mut(), self.next_k <= self.quanta) else {
            return;
        };
        let spice_us = self.since.elapsed().as_micros();
        let k = self.next_k;
        let (fw, renode_us) = owner.renode_step();
        let v = ng_probe::ffi::vec_last("line").map_or("?".to_owned(), |(_, v)| format!("{v:.12e}"));
        owner.finish(k, fw, renode_us, format!("{time:.12e}"), v, self.measured, spice_us);
        self.next_k += 1;
        self.since = Instant::now();
    }
}

impl Hooks for Ioc {
    fn vsrc(&mut self, _time: f64, _name: &str) -> f64 {
        0.0
    }
    fn sync(&mut self, time: f64, _delta: &mut f64, _old: f64, _redo: i32, loc: i32) -> i32 {
        // loc 1 reports the step just taken: feed the newest accepted point.
        if loc == 1
            && let (Some((_, t)), Some((_, v))) =
                (ng_probe::ffi::vec_last("time"), ng_probe::ffi::vec_last("line"))
        {
            if let Some((pt, pv)) = self.prev {
                let cross = |lvl: f64| {
                    (pv < lvl && v >= lvl).then(|| pt + (lvl - pv) * (t - pt) / (v - pv))
                };
                if self.t30.is_none() {
                    self.t30 = cross(0.99);
                }
                if let (Some(a), None) = (self.t30, self.measured)
                    && let Some(b) = cross(2.31)
                {
                    self.measured = Some((b - a) * 1e9);
                }
            }
            self.prev = Some((t, v));
        }
        // loc 0 reports the time reached: a boundary runs the quantum.
        if loc == 0 && time >= self.next_k as f64 * self.q_s * (1.0 - 1e-12) {
            self.boundary(time);
        }
        0
    }
}

struct Args {
    renode_setup: String,
    reads: String,
    fault_cmd: Option<String>,
    fault_on: Option<String>,
    fault_off: Option<String>,
    quantum_ns: u64,
    span_ns: u64,
    spice: String,
    deck: Option<String>,
    ceiling_ns: f64,
    out: String,
    renode: String,
}

fn parse() -> Args {
    let mut a = Args {
        renode_setup: String::new(),
        reads: String::new(),
        fault_cmd: None,
        fault_on: None,
        fault_off: None,
        quantum_ns: 10_000,
        span_ns: 10_000_000,
        spice: "pipe".into(),
        deck: None,
        ceiling_ns: 300.0,
        out: "lockstep".into(),
        renode: "renode".into(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let mut v = || it.next().unwrap_or_else(|| panic!("{k} needs a value"));
        match k.as_str() {
            "--renode-setup" => a.renode_setup = v(),
            "--reads" => a.reads = v(),
            "--fault-cmd" => a.fault_cmd = Some(v()),
            "--fault-on" => a.fault_on = Some(v()),
            "--fault-off" => a.fault_off = Some(v()),
            "--quantum-ns" => a.quantum_ns = v().parse().expect("quantum"),
            "--span-ns" => a.span_ns = v().parse().expect("span"),
            "--spice" => a.spice = v(),
            "--deck" => a.deck = Some(v()),
            "--ceiling-ns" => a.ceiling_ns = v().parse().expect("ceiling"),
            "--out" => a.out = v(),
            "--renode" => a.renode = v(),
            _ => panic!("unknown argument {k}"),
        }
    }
    a
}

fn lines_of(path: &str) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path}: {e}"))
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

fn main() {
    let a = parse();
    let started = Instant::now();
    let mut renode = Driven::spawn(
        "renode",
        Command::new(&a.renode)
            .args(["--disable-gui", "--console", "--plain"])
            .env("DOTNET_EnableDiagnostics", "0"),
    );
    let setup = renode.exec(&lines_of(&a.renode_setup), renode_echo);
    let setup_errors: Vec<_> = setup
        .iter()
        .filter(|l| l.contains("There was an error") || l.contains("[ERROR]"))
        .collect();
    assert!(setup_errors.is_empty(), "renode setup failed: {setup_errors:?}");
    let mut owner = Owner {
        renode,
        reads: lines_of(&a.reads),
        fault_cmd: a.fault_cmd.clone(),
        fault_on: a.fault_on.clone(),
        fault_off: a.fault_off.clone(),
        quantum_ns: a.quantum_ns,
        ceiling_ns: a.ceiling_ns,
        trise: None,
        fault: false,
        last_fw: Vec::new(),
        seq: String::new(),
        costs: String::from("q,renode_us,spice_us,fault_us\n"),
        events: String::new(),
    };
    let quanta = a.span_ns / a.quantum_ns;
    let span_s = a.span_ns as f64 * 1e-9;
    let deck = || a.deck.clone().expect("--deck");
    let setup_wall;
    let loop_start;
    match a.spice.as_str() {
        "ioc" => {
            let mut sp = Spice::init(Ioc {
                owner: Some(owner),
                quanta,
                next_k: 1,
                q_s: a.quantum_ns as f64 * 1e-9,
                since: Instant::now(),
                prev: None,
                t30: None,
                measured: None,
            })
            .expect("ngspice init");
            sp.command(&format!("source {}", deck()));
            for k in 1..=quanta {
                sp.set_bkpt(k as f64 * a.quantum_ns as f64 * 1e-9);
            }
            sp.take_log();
            setup_wall = started.elapsed();
            loop_start = Instant::now();
            sp.hooks().since = Instant::now();
            let rc = sp.command(&format!("tran 1n {span_s:e} 0 5n"));
            assert_eq!(rc, 0, "tran: {:?}", sp.take_log());
            let ioc = sp.hooks();
            // The last boundary is the transient's end, after which no sync
            // call fires: run it now, at the same virtual time.
            if let Some((_, t)) = ng_probe::ffi::vec_last("time") {
                ioc.boundary(t);
            }
            assert_eq!(ioc.next_k, quanta + 1, "the solver missed quantum boundaries");
            owner = ioc.owner.take().expect("the owner");
            drop(sp);
        }
        "pipe" | "lib" | "none" => {
            let mut spice = match a.spice.as_str() {
                "pipe" => Some(Stepped::Pipe(Driven::spawn(
                    "spice",
                    Command::new("script").args([
                        "-qec",
                        "stty -echo; exec ngspice -p -t dumb",
                        "/dev/null",
                    ]),
                ))),
                "lib" => Some(Stepped::Lib(Spice::init(Quiet).expect("ngspice init"))),
                _ => None,
            };
            if let Some(s) = spice.as_mut() {
                s.exec(&[format!("source {}", deck())]);
            }
            setup_wall = started.elapsed();
            loop_start = Instant::now();
            for k in 1..=quanta {
                let (fw, renode_us) = owner.renode_step();
                let t1 = Instant::now();
                let (mut st, mut sv, mut measured) = ("-".to_owned(), "-".to_owned(), None);
                if let Some(s) = spice.as_mut() {
                    let t_s = format!("{:e}", (k * a.quantum_ns) as f64 * 1e-9);
                    let mut c = vec!["delete all".to_owned(), format!("stop when time > {t_s}")];
                    c.push(if k == 1 {
                        format!("tran 1n {span_s:e} 0 5n")
                    } else {
                        "resume".to_owned()
                    });
                    c.push("let aetn = length(time) - 1".to_owned());
                    c.push("print time[aetn] v(line)[aetn]".to_owned());
                    if owner.trise.is_none() {
                        c.push(RISE_CMD.to_owned());
                    }
                    let out = s.exec(&c);
                    st = spice_value(&out, "time[aetn]").unwrap_or_else(|| "?".into());
                    sv = spice_value(&out, "v(line)[aetn]").unwrap_or_else(|| "?".into());
                    measured = spice_value(&out, "aetrise")
                        .and_then(|v| v.parse::<f64>().ok())
                        .map(|v| v * 1e9);
                }
                let spice_us = t1.elapsed().as_micros();
                owner.finish(k, fw, renode_us, st, sv, measured, spice_us);
            }
            if let Some(Stepped::Pipe(d)) = spice {
                d.quit("quit");
            }
        }
        other => panic!("unknown spice backend {other}"),
    }
    let loop_wall = loop_start.elapsed();
    fs::write(format!("{}.seq", a.out), &owner.seq).expect("seq");
    fs::write(format!("{}.costs.csv", a.out), &owner.costs).expect("costs");
    fs::write(format!("{}.events.jsonl", a.out), &owner.events).expect("events");
    let trise = owner.trise;
    owner.renode.quit("quit");
    println!(
        "quanta={quanta} quantum_ns={} span_ns={} setup_s={:.3} loop_s={:.3} per_quantum_ms={:.4} spice={} trise_ns={}",
        a.quantum_ns,
        a.span_ns,
        setup_wall.as_secs_f64(),
        loop_wall.as_secs_f64(),
        loop_wall.as_secs_f64() * 1e3 / quanta as f64,
        a.spice,
        trise.map_or("-".to_owned(), |v| format!("{v:.4}"))
    );
}
