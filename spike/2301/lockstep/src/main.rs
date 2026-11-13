//! AET-WO-2301 spike, P2 and P4: a rail droop computed by libngspice in this
//! process, and what Renode's generated firmware sees of it.
//!
//! Modes:
//! - `rail CASE QUANTUM_NS SPAN_NS OUT`: ngspice alone. One `tran`, a
//!   breakpoint at every quantum boundary, the rail read at each boundary from
//!   `GetSyncData` (AET-WO-2101's `ioc` route). Reports each threshold's
//!   crossing as ngspice's accepted points give it (and ngspice's own `meas`),
//!   and as the boundary samples see it.
//! - `lockstep CASE QUANTUM_NS SPAN_NS OUT`: the same, with Renode as this
//!   process's child, stepped from inside `GetSyncData`: at each boundary
//!   Renode runs the quantum and one word is read (the boot word); then the
//!   bridge decides from the boundary sample whether the next quantum is held
//!   in reset. Writes the per-quantum traces for P4 with the real
//!   `aethelgard-chaos` writer.
//! - `schedule CASE SPAN_NS OUT`: the rail is solved first, ngspice's
//!   crossings become events, and Renode is run segment by segment to each.
//! - `replay DIR`: `replay_dir`, then this process's peak RSS (P4's fold).
//!
//! A case is a declared source sag (nominal 3.3 V, floor, ramps) through a
//! declared source resistance into the emitted BOM's decoupling on `VDD_3V3`,
//! with a declared constant load. The thresholds are DS12117 Rev 7 Table 26
//! typical values; which BOR level applies is a scenario parameter here.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use aethelgard_chaos::record::Record;
use aethelgard_chaos::{Event, Header, TraceWriter, replay_dir};
use ng_probe::{Hooks, Spice};

const NOMINAL_V: f64 = 3.3;
const SOURCE_OHMS: f64 = 0.5;
const LOAD_A: f64 = 0.1;
/// Four 100 nF, one per IC on VDD_3V3 (`schematic.rs` L512, L610, L649).
const DECOUPLING_F: f64 = 400e-9;
/// The domain's floor: 3,300 mV less 50 per mille.
const TOLERANCE_FLOOR_V: f64 = 3.135;
/// DS12117 Rev 7 Table 26, typical: (falling, rising).
const BOR3: (f64, f64) = (2.61, 2.70);
const BOR0: (f64, f64) = (1.62, 1.67);
/// "Reset temporization after BOR0 released", typical, Table 26.
const T_RSTTEMPO_NS: u64 = 377_000;

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    floor_v: f64,
    start_ns: u64,
    fall_ns: u64,
    hold_ns: u64,
    rise_ns: u64,
    /// The BOR level the part runs at: its thresholds, and the release delay.
    bor: (f64, f64),
    release_delay_ns: u64,
}

fn case(name: &str) -> Case {
    let base = |name, floor_v, bor, delay| Case {
        name,
        floor_v,
        start_ns: 1_000_000,
        fall_ns: 20_000,
        hold_ns: 200_000,
        rise_ns: 20_000,
        bor,
        release_delay_ns: delay,
    };
    match name {
        "tolerance" => base("tolerance", 3.20, BOR3, 0),
        "bor3" => base("bor3", 2.40, BOR3, 0),
        "pdr" => base("pdr", 1.20, BOR0, T_RSTTEMPO_NS),
        // A 2 µs fall, 1 µs at the floor, a 2 µs rise: about 3 µs below BOR3.
        // Between two 10 µs boundaries, then straddling one.
        "dip-between" => Case {
            start_ns: 1_003_000,
            fall_ns: 2_000,
            hold_ns: 1_000,
            rise_ns: 2_000,
            ..base("dip-between", 2.40, BOR3, 0)
        },
        "dip-straddle" => Case {
            start_ns: 1_008_000,
            fall_ns: 2_000,
            hold_ns: 1_000,
            rise_ns: 2_000,
            ..base("dip-straddle", 2.40, BOR3, 0)
        },
        other => panic!("unknown case {other}"),
    }
}

fn deck(c: &Case, span_ns: u64) -> Vec<String> {
    let s = |ns: u64| format!("{}n", ns);
    let t1 = c.start_ns;
    let t2 = t1 + c.fall_ns;
    let t3 = t2 + c.hold_ns;
    let t4 = t3 + c.rise_ns;
    vec![
        format!("* aet2301 rail {}", c.name),
        format!(
            "vsrc src 0 pwl(0 {NOMINAL_V} {} {NOMINAL_V} {} {} {} {} {} {NOMINAL_V})",
            s(t1),
            s(t2),
            c.floor_v,
            s(t3),
            c.floor_v,
            s(t4)
        ),
        format!("rs src rail {SOURCE_OHMS}"),
        format!("cdec rail 0 {DECOUPLING_F}"),
        format!("iload rail 0 dc {LOAD_A}"),
        ".options reltol=1e-3 abstol=1e-12 vntol=1e-6 method=trap".to_owned(),
        // TMAX (ns) caps ngspice's step; unset, ngspice caps it at the 10 ns
        // print step (measured: 300,905 accepted points in 3 ms).
        match std::env::var("TMAX_NS") {
            Ok(m) => format!(".tran 10n {} 0 {m}n", s(span_ns)),
            Err(_) => format!(".tran 10n {}", s(span_ns)),
        },
        ".end".to_owned(),
    ]
}

/// Every crossing of `level` on piecewise-linear points: (time, falling).
fn crossings(t: &[f64], v: &[f64], level: f64) -> Vec<(f64, bool)> {
    let mut out = Vec::new();
    for i in 1..t.len().min(v.len()) {
        let (a, b) = (v[i - 1], v[i]);
        if (a >= level) != (b >= level) {
            let x = t[i - 1] + (level - a) * (t[i] - t[i - 1]) / (b - a);
            out.push((x, b < level));
        }
    }
    out
}

/// The reset intervals a BOR with thresholds (fall, rise) asserts on a
/// waveform: from each falling crossing of `fall` to the next rising crossing
/// of `rise`, plus the release delay. Times in ns.
fn reset_intervals(t: &[f64], v: &[f64], bor: (f64, f64), delay_ns: u64) -> Vec<(u64, u64)> {
    let mut fall: Vec<_> = crossings(t, v, bor.0).into_iter().filter(|c| c.1).collect();
    let rise: Vec<_> = crossings(t, v, bor.1)
        .into_iter()
        .filter(|c| !c.1)
        .collect();
    let mut out = Vec::new();
    fall.reverse();
    while let Some((tf, _)) = fall.pop() {
        let tr = rise.iter().find(|r| r.0 > tf).map(|r| r.0);
        let up = |x: f64| ((x * 1e9 / 10.0).ceil() as u64) * 10;
        let release = tr.map_or(u64::MAX, |tr| up(tr) + delay_ns);
        out.push((up(tf), release));
        fall.retain(|f| f.0 > tr.unwrap_or(f64::MAX));
    }
    out
}

// ---------------------------------------------------------------- Renode

const WAIT: Duration = Duration::from_secs(300);
const FW: &str = "/cache/target/thumbv7em-none-eabihf/release/sentinel-01-board-firmware";
const REPL: &str = "/repo/boards/sentinel-01/sentinel-01.repl";
const BOOT_WORD: u32 = 0x2000_0000;

struct Renode {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<String>,
    serial: u64,
    busy: Duration,
}

impl Renode {
    fn start(scratch: &Path) -> Self {
        let home = scratch.join("home");
        fs::create_dir_all(home.join(".config/renode")).unwrap();
        fs::write(
            home.join(".config/renode/config"),
            "[general]\nhistory-path = /dev/null\n",
        )
        .unwrap();
        let mut child = Command::new("renode")
            .args(["--disable-gui", "--console", "--plain"])
            .env_clear()
            .env("HOME", &home)
            .env("PATH", "/usr/local/bin:/usr/bin:/bin")
            .env("DOTNET_EnableDiagnostics", "0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("renode");
        let (tx, rx) = channel();
        let out = child.stdout.take().unwrap();
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines() {
                let Ok(line) = line else { break };
                if tx.send(line.trim_matches('\r').to_owned()).is_err() {
                    break;
                }
            }
        });
        let stdin = child.stdin.take().unwrap();
        let mut r = Self {
            child,
            stdin,
            rx,
            serial: 0,
            busy: Duration::ZERO,
        };
        let log = scratch.join("renode.log");
        let mut setup = vec![
            format!("logFile @{}", log.display()),
            "logLevel 3 console".to_owned(),
            "emulation SetSeed 2100".to_owned(),
            "mach create".to_owned(),
            format!("machine LoadPlatformDescription @{REPL}"),
            format!("sysbus LoadELF @{FW}"),
        ];
        setup.extend(cold_and_vtor());
        setup.extend(rearm());
        r.exec(&setup);
        r
    }

    /// The commands, then a sentinel; every whole value line before it.
    fn exec(&mut self, cmds: &[String]) -> Vec<String> {
        let t0 = Instant::now();
        self.serial += 1;
        // ONE_LINE: the commands as one `; `-joined line with a constant
        // sentinel, so that each quantum's line repeats the last one and
        // AntShell's history (`CommandHistory.Add` skips a repeat) stays put.
        // Safe because each exec waits for its own sentinel before the next.
        let one_line = std::env::var_os("ONE_LINE").is_some();
        let sentinel = if one_line {
            "AET2301-Q".to_owned()
        } else {
            format!("AET2301-{}", self.serial)
        };
        let text = if one_line {
            format!("{}; echo \"{sentinel}\"\n", cmds.join("; "))
        } else {
            format!("{}\necho \"{sentinel}\"\n", cmds.join("\n"))
        };
        self.stdin.write_all(text.as_bytes()).unwrap();
        self.stdin.flush().unwrap();
        let mut vals = Vec::new();
        loop {
            match self.rx.recv_timeout(WAIT) {
                Ok(l) if l.trim() == sentinel => break,
                Ok(l) => {
                    let l = l.trim();
                    if l.starts_with("0x") || l == "True" || l == "False" {
                        vals.push(l.to_owned());
                    }
                }
                Err(RecvTimeoutError::Timeout) => panic!("renode: no sentinel in {WAIT:?}"),
                Err(RecvTimeoutError::Disconnected) => panic!("renode exited"),
            }
        }
        self.busy += t0.elapsed();
        vals
    }

    fn quit(mut self) {
        let _ = self.stdin.write_all(b"quit\n");
        let _ = self.child.wait();
    }
}

fn cold_and_vtor() -> Vec<String> {
    let mut c: Vec<String> = [0x5802_44E0u32, 0x5802_44E8, 0x5802_44F0]
        .iter()
        .map(|a| format!("sysbus WriteDoubleWord {a:#010x} 0x0"))
        .collect();
    c.push("cpu VectorTableOffset 0x08000000".to_owned());
    c
}

/// The bridge's re-arm, as the CLI's first boot does: only the firmware may
/// set the boot word and the record again.
fn rearm() -> Vec<String> {
    vec![
        format!("sysbus WriteDoubleWord {BOOT_WORD:#010x} 0x0"),
        "sysbus WriteDoubleWord 0x20000004 0xffffffff".to_owned(),
        "sysbus WriteDoubleWord 0x20000008 0xffffffff".to_owned(),
    ]
}

/// Asserting a held reset: P1's `held-halted` means (measured deterministic).
fn assert_reset() -> Vec<String> {
    let mut c = vec!["machine Reset".to_owned()];
    c.extend(cold_and_vtor());
    c.push("cpu IsHalted true".to_owned());
    c.extend(rearm());
    c
}

/// The release, then a 10 ns `RunFor`: a release takes effect only at the end
/// of the next `RunFor`, so the nudge bounds the loss to 10 ns (P1, F5).
const NUDGE_NS: u64 = 10;
fn release() -> Vec<String> {
    vec!["cpu IsHalted false".to_owned(), run_for(NUDGE_NS)]
}

fn run_for(ns: u64) -> String {
    format!(
        "emulation RunFor \"{}.{:09}\"",
        ns / 1_000_000_000,
        ns % 1_000_000_000
    )
}

const FINAL_READS: [&str; 5] = [
    "sysbus ReadDoubleWord 0x20000000",
    "sysbus ReadDoubleWord 0x20000004",
    "sysbus ReadDoubleWord 0x20000008",
    "cpu ExecutedInstructions",
    "cpu PC",
];

// ---------------------------------------------------------------- traces

struct Traces {
    samples: TraceWriter,
    records: TraceWriter,
    events: TraceWriter,
    refused: Option<String>,
}

fn header() -> Header {
    Header {
        run_id: [0x23; 16],
        artifacts: vec![("probe".to_owned(), [0; 32])],
    }
}

impl Traces {
    fn create(out: &Path) -> Self {
        let w = |n: &str| {
            let d = out.join(n);
            let _ = fs::remove_dir_all(&d);
            TraceWriter::create(&d, &header()).unwrap()
        };
        Self {
            samples: w("trace-samples"),
            records: w("trace-records"),
            events: w("trace-events"),
            refused: None,
        }
    }

    /// One boundary sample, twice: as kind 3 (`AnalogSample`), and as a
    /// labelled record. No rail record kind exists; kind 8 (`LineStuckHigh`:
    /// two texts, an i64 and a u64) stands in for {source, net, µV, quantum}.
    fn sample(&mut self, t_ns: u64, k: u64, volts: f64) {
        let uv = (volts * 1e6).round() as i64;
        self.samples
            .append(&Event::AnalogSample {
                timestamp: t_ns,
                net: "VDD_3V3".to_owned(),
                microvolts: uv,
            })
            .unwrap();
        if self.refused.is_none()
            && let Err(e) = self.records.append(&Event::Record {
                timestamp: t_ns,
                record: Record::LineStuckHigh {
                    bus: "ngspice".to_owned(),
                    net: "VDD_3V3".to_owned(),
                    microvolts_at_release: uv,
                    ceiling_ps: k,
                },
            })
        {
            // The writer folds as it writes, so MAX_RECORDS refuses here.
            self.refused = Some(format!("quantum {k}: {e}"));
        }
    }

    /// An event of the crossings-only record set: a stand-in record each.
    fn event(&mut self, t_ns: u64, what: &str, source: &str, detail: i64) {
        self.events
            .append(&Event::Record {
                timestamp: t_ns,
                record: Record::LineStuckHigh {
                    bus: source.to_owned(),
                    net: what.to_owned(),
                    microvolts_at_release: detail,
                    ceiling_ps: 0,
                },
            })
            .unwrap();
    }

    fn commit(self, out: &Path) -> String {
        let mut s = format!(
            ",\"records_refused\":{}",
            self.refused.map_or("null".to_owned(), |r| format!("{r:?}"))
        );
        for (name, w) in [
            ("samples", self.samples),
            ("records", self.records),
            ("events", self.events),
        ] {
            let m = w.commit().unwrap();
            let bytes = fs::metadata(out.join(format!("trace-{name}/trace.cbor")))
                .map(|m| m.len())
                .unwrap_or(0);
            s.push_str(&format!(
                ",\"trace_{name}_bytes\":{bytes},\"trace_{name}_events\":{}",
                m.event_count
            ));
        }
        s
    }
}

// ---------------------------------------------------------------- the hook

struct Ioc {
    q_ns: u64,
    quanta: u64,
    next_k: u64,
    samples: Vec<(u64, f64)>,
    renode: Option<Renode>,
    traces: Option<Traces>,
    bor: (f64, f64),
    delay_ns: u64,
    held: bool,
    release_at: Option<u64>,
    /// ns of the next quantum already run by a release's nudge.
    nudged: u64,
    last_boot: Option<String>,
    actions: Vec<String>,
    since: Instant,
    ng_busy: Duration,
}

impl Ioc {
    fn boundary(&mut self, _time: f64) {
        self.ng_busy += self.since.elapsed();
        let k = self.next_k;
        let t_ns = k * self.q_ns;
        let v = ng_probe::ffi::vec_last("rail").map_or(f64::NAN, |(_, v)| v);
        self.samples.push((t_ns, v));
        if let Some(tr) = self.traces.as_mut() {
            tr.sample(t_ns, k, v);
        }
        if let Some(r) = self.renode.as_mut() {
            // 1. Renode runs the quantum that ends here; one word is read.
            let got = r.exec(&[
                run_for(self.q_ns - std::mem::take(&mut self.nudged)),
                format!("sysbus ReadDoubleWord {BOOT_WORD:#010x}"),
            ]);
            let boot = got.first().cloned().unwrap_or_default();
            if self.last_boot.as_ref() != Some(&boot) {
                if let Some(tr) = self.traces.as_mut() {
                    tr.event(
                        t_ns,
                        "boot_word",
                        "renode",
                        i64::from_str_radix(boot.trim_start_matches("0x"), 16).unwrap_or(-1),
                    );
                }
                self.actions.push(format!("{t_ns} renode boot_word {boot}"));
                self.last_boot = Some(boot);
            }
            // 2. The bridge's decision for the next quantum, from the sample.
            let mut cmds = Vec::new();
            if !self.held && v < self.bor.0 {
                self.held = true;
                self.release_at = None;
                cmds = assert_reset();
                self.actions
                    .push(format!("{t_ns} bridge reset_asserted v={v:.4}"));
                if let Some(tr) = self.traces.as_mut() {
                    tr.event(t_ns, "VDD_3V3", "ngspice", (v * 1e6) as i64);
                    tr.event(t_ns, "reset_asserted", "bridge", 0);
                }
            } else if self.held && self.release_at.is_none() && v > self.bor.1 {
                self.release_at = Some(t_ns + self.delay_ns);
                if let Some(tr) = self.traces.as_mut() {
                    tr.event(t_ns, "VDD_3V3", "ngspice", (v * 1e6) as i64);
                }
            }
            if self.held && self.release_at.is_some_and(|at| t_ns >= at) {
                self.held = false;
                self.release_at = None;
                cmds = release();
                self.nudged = NUDGE_NS;
                self.actions
                    .push(format!("{t_ns} bridge reset_released v={v:.4}"));
                if let Some(tr) = self.traces.as_mut() {
                    tr.event(t_ns, "reset_released", "bridge", 0);
                }
            }
            if !cmds.is_empty() {
                r.exec(&cmds);
            }
        }
        self.next_k += 1;
        self.since = Instant::now();
    }
}

impl Hooks for Ioc {
    fn vsrc(&mut self, _time: f64, _name: &str) -> f64 {
        0.0
    }
    fn sync(&mut self, time: f64, _delta: &mut f64, _old: f64, _redo: i32, loc: i32) -> i32 {
        if loc == 0
            && self.next_k <= self.quanta
            && time >= (self.next_k * self.q_ns) as f64 * 1e-9 * (1.0 - 1e-12)
        {
            self.boundary(time);
        }
        0
    }
}

struct Quiet;
impl Hooks for Quiet {
    fn vsrc(&mut self, _time: f64, _name: &str) -> f64 {
        0.0
    }
    fn sync(&mut self, _t: f64, _d: &mut f64, _o: f64, _r: i32, _l: i32) -> i32 {
        0
    }
}

fn meas_when(sp: &mut Spice<impl Hooks>, level: f64, dir: &str) -> String {
    sp.take_log();
    let name = format!("w{}{}", dir.to_lowercase(), (level * 1000.0) as u64);
    sp.command(&format!("meas tran {name} WHEN v(rail)={level} {dir}=1"));
    let log = sp.take_log();
    match sp.vec(&name) {
        Some(v) if v.len() == 1 => format!("{:.3}", v[0] * 1e9),
        _ => format!(
            "\"none: {}\"",
            log.last().cloned().unwrap_or_default().replace('"', "'")
        ),
    }
}

fn levels() -> Vec<(&'static str, f64)> {
    vec![
        ("tolerance_floor", TOLERANCE_FLOOR_V),
        ("bor3_fall", BOR3.0),
        ("bor3_rise", BOR3.1),
        ("bor0_fall", BOR0.0),
        ("bor0_rise", BOR0.1),
    ]
}

fn args() -> Vec<String> {
    std::env::args().skip(1).collect()
}

fn main() {
    let a = args();
    match a.first().map(String::as_str) {
        Some("rail") | Some("lockstep") => ioc_mode(&a),
        Some("schedule") => schedule_mode(&a),
        Some("replay") => replay_mode(&a[1]),
        _ => panic!(
            "usage: lockstep rail|lockstep CASE QUANTUM_NS SPAN_NS OUT | schedule CASE SPAN_NS OUT | replay DIR"
        ),
    }
}

fn ioc_mode(a: &[String]) {
    let with_renode = a[0] == "lockstep";
    let c = case(&a[1]);
    let q_ns: u64 = a[2].parse().unwrap();
    let span_ns: u64 = a[3].parse().unwrap();
    let out = PathBuf::from(&a[4]);
    fs::create_dir_all(&out).unwrap();
    let quanta = span_ns / q_ns;
    let t_start = Instant::now();
    let renode = with_renode.then(|| Renode::start(&out));
    let renode_start = t_start.elapsed();
    let traces = with_renode.then(|| Traces::create(&out));
    let mut sp = Spice::init(Ioc {
        q_ns,
        quanta,
        next_k: 1,
        samples: Vec::with_capacity(quanta as usize),
        renode,
        traces,
        bor: c.bor,
        delay_ns: c.release_delay_ns,
        held: false,
        release_at: None,
        nudged: 0,
        last_boot: None,
        actions: Vec::new(),
        since: Instant::now(),
        ng_busy: Duration::ZERO,
    })
    .unwrap();
    sp.circ(&deck(&c, span_ns)).unwrap();
    let t_bkpt = Instant::now();
    for k in 1..=quanta {
        assert!(sp.set_bkpt((k * q_ns) as f64 * 1e-9), "breakpoint {k}");
    }
    let bkpt_s = t_bkpt.elapsed().as_secs_f64();
    let t_run = Instant::now();
    sp.hooks().since = Instant::now();
    let r = sp.command("run");
    let run_s = t_run.elapsed().as_secs_f64();
    assert_eq!(r, 0, "run: {:?}", sp.take_log());
    let h = sp.hooks();
    // ngspice does not call GetSyncData at tstop itself: the last boundary
    // runs here, with the solver at the end time (measured: 300 of 301).
    if h.next_k == quanta {
        h.since = Instant::now();
        h.boundary(span_ns as f64 * 1e-9);
    }
    assert_eq!(h.next_k, quanta + 1, "missed boundaries");
    let samples = std::mem::take(&mut h.samples);
    let actions = std::mem::take(&mut h.actions);
    let ng_busy = h.ng_busy.as_secs_f64();
    let renode = h.renode.take();
    let traces = h.traces.take();
    let time = sp.vec("time").unwrap();
    let rail = sp.vec("rail").unwrap();
    let vmin = rail.iter().cloned().fold(f64::INFINITY, f64::min);
    let mut lv = String::new();
    let st: Vec<f64> = samples.iter().map(|s| s.0 as f64 * 1e-9).collect();
    let sv: Vec<f64> = samples.iter().map(|s| s.1).collect();
    for (name, level) in levels() {
        let exact: Vec<_> = crossings(&time, &rail, level);
        let seen: Vec<_> = crossings(&st, &sv, level);
        // A crossing between boundaries is seen at the later boundary.
        let seen_at: Vec<u64> = seen
            .iter()
            .map(|(x, _)| ((x * 1e9 / q_ns as f64).ceil() as u64) * q_ns)
            .collect();
        let exact_ns: Vec<String> = exact
            .iter()
            .map(|(x, f)| format!("[{:.3},{}]", x * 1e9, f))
            .collect();
        lv.push_str(&format!(
            ",\"{name}\":{{\"level_v\":{level},\"exact_ns\":[{}],\"seen_at_boundary_ns\":{:?},\"meas_fall_ns\":{},\"meas_rise_ns\":{}}}",
            exact_ns.join(","),
            seen_at,
            meas_when(&mut sp, level, "FALL"),
            meas_when(&mut sp, level, "RISE"),
        ));
    }
    let resets = reset_intervals(&time, &rail, c.bor, c.release_delay_ns);
    let mut tail = String::new();
    if let Some(mut r) = renode {
        let fin = r.exec(
            &FINAL_READS
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
        );
        tail.push_str(&format!(
            ",\"renode_start_s\":{:.3},\"renode_busy_s\":{:.3},\"final\":{:?},\"actions\":{:?}",
            renode_start.as_secs_f64(),
            r.busy.as_secs_f64(),
            fin,
            actions
        ));
        r.quit();
    }
    if let Some(t) = traces {
        tail.push_str(&t.commit(&out));
    }
    let tsv: String = samples
        .iter()
        .map(|(t, v)| format!("{t}\t{v:.6}\n"))
        .collect();
    fs::write(out.join("boundary-samples.tsv"), tsv).unwrap();
    println!(
        "{{\"mode\":\"{}\",\"case\":\"{}\",\"tmax_ns\":\"{}\",\"quantum_ns\":{q_ns},\"span_ns\":{span_ns},\"quanta\":{quanta},\"accepted_points\":{},\"rail_min_v\":{vmin:.6},\"bkpt_s\":{bkpt_s:.3},\"run_s\":{run_s:.3},\"ngspice_between_boundaries_s\":{ng_busy:.3},\"wall_s\":{:.3},\"reset_intervals_ns\":{}{lv}{tail}}}",
        a[0],
        c.name,
        std::env::var("TMAX_NS").unwrap_or_default(),
        time.len(),
        t_start.elapsed().as_secs_f64(),
        pairs(&resets),
    );
}

fn pairs(v: &[(u64, u64)]) -> String {
    let items: Vec<String> = v.iter().map(|(a, b)| format!("[{a},{b}]")).collect();
    format!("[{}]", items.join(","))
}

fn schedule_mode(a: &[String]) {
    let c = case(&a[1]);
    let span_ns: u64 = a[2].parse().unwrap();
    let out = PathBuf::from(&a[3]);
    fs::create_dir_all(&out).unwrap();
    let t0 = Instant::now();
    let mut sp = Spice::init(Quiet).unwrap();
    sp.circ(&deck(&c, span_ns)).unwrap();
    assert_eq!(sp.command("run"), 0);
    let time = sp.vec("time").unwrap();
    let rail = sp.vec("rail").unwrap();
    let ng_s = t0.elapsed().as_secs_f64();
    let resets = reset_intervals(&time, &rail, c.bor, c.release_delay_ns);
    // The events, in order: (time, commands, label).
    let mut events: Vec<(u64, Vec<String>, String)> = Vec::new();
    for (on, off) in &resets {
        events.push((*on, assert_reset(), format!("reset_asserted@{on}")));
        if *off < span_ns {
            events.push((*off, release(), format!("reset_released@{off}")));
            // A read 30 µs after the release, for the recovery (P1: 20 µs).
            events.push((*off + 30_000, vec![], format!("probe@{}", off + 30_000)));
        }
    }
    let t1 = Instant::now();
    let mut r = Renode::start(&out);
    let start_s = t1.elapsed().as_secs_f64();
    let traces = Traces::create(&out);
    let mut traces = traces;
    let mut now = 0u64;
    let mut seen = Vec::new();
    for (at, cmds, label) in &events {
        if *at > now {
            let got = r.exec(&[
                run_for(at - now),
                FINAL_READS[0].to_owned(),
                FINAL_READS[1].to_owned(),
            ]);
            now = *at;
            seen.push(format!("{label} before: {got:?}"));
            traces.event(now, "boot_word", "renode", 0);
        }
        if !cmds.is_empty() {
            r.exec(cmds);
            traces.event(now, label, "bridge", 0);
            if label.starts_with("reset_released") {
                now += NUDGE_NS;
            }
        }
    }
    let fin = r.exec(
        &[run_for(span_ns - now)]
            .into_iter()
            .chain(FINAL_READS.iter().map(|s| s.to_string()))
            .collect::<Vec<_>>(),
    );
    let busy = r.busy.as_secs_f64();
    r.quit();
    let tr = traces.commit(&out);
    println!(
        "{{\"mode\":\"schedule\",\"case\":\"{}\",\"span_ns\":{span_ns},\"accepted_points\":{},\"ngspice_s\":{ng_s:.3},\"renode_start_s\":{start_s:.3},\"renode_busy_s\":{busy:.3},\"wall_s\":{:.3},\"reset_intervals_ns\":{},\"seen\":{seen:?},\"final\":{fin:?}{tr}}}",
        c.name,
        time.len(),
        t0.elapsed().as_secs_f64(),
        pairs(&resets)
    );
}

fn replay_mode(dir: &str) {
    let t0 = Instant::now();
    let r = replay_dir(Path::new(dir)).unwrap();
    let mut status = String::new();
    fs::File::open("/proc/self/status")
        .unwrap()
        .read_to_string(&mut status)
        .unwrap();
    let hwm = status
        .lines()
        .find(|l| l.starts_with("VmHWM"))
        .unwrap_or("VmHWM: ?")
        .split_whitespace()
        .nth(1)
        .unwrap_or("?")
        .to_owned();
    println!(
        "{{\"dir\":\"{dir}\",\"events\":{},\"records\":{},\"vmhwm_kib\":{hwm},\"replay_s\":{:.3}}}",
        r.manifest.event_count,
        r.state.records.len(),
        t0.elapsed().as_secs_f64()
    );
}
