//! AET-WO-2304 spike (restored from AET-WO-2303's `spike/2303/lockstep` at
//! cd43b207): the battery chain under the ESCs on Sentinel-03, under two
//! candidates: (b) per-quantum lock-step (`lockstep`) and (a) a declared
//! profile solved once (`declared`). The deck is Sentinel-03's declared tree
//! (`deck`, FEEDBACK "The model, as designed"); the ESC current is `I_RUN_A`.
//! The text below is 2303's, for the modes it kept.
//!
//! Modes:
//! - `lockstep CASE SPAN_NS SCRIPT OUT`: libngspice in this process (AET-WO-2101's
//!   `ioc` route, a breakpoint at every boundary), Renode as this process's
//!   child, run one quantum at a time from `GetSyncData`. At each boundary the
//!   firmware's motor timer is read (TIM1 `CR1`, `CCR1`) and sets the motors'
//!   current for the next quantum; ngspice's battery and `VDD_3V3` are sampled;
//!   a BOR3 crossing is rendered as the CLI renders it (held reset, nudge).
//!   `SCRIPT` is the unmodified CLI's Renode argv, captured from /proc.
//!   Three traces are written with the real `aethelgard-chaos` writer: per-
//!   quantum samples (kinds 3 and 0), per-quantum stand-in records (until
//!   `MAX_RECORDS` refuses), and the events only.
//! - `replay DIR`: `replay_dir` on each trace, this process's peak RSS, and
//!   the bridge's actions re-derived from the recorded `VDD_3V3` samples alone.
//! - `flip DIR OUT`: a copy of `DIR` with one byte of a mid-run sample's value
//!   flipped, then `replay_dir` on it.
//! - `splice RUNDIR OUTBASE`: a real campaign run's events re-written, sealed,
//!   with (a) one kind 3 sample and (b) 1,000 per-quantum stand-in records
//!   inserted after its rail records, for the CLI's `replay`.
//!
//! **Spike-declared, not sourced:** the pack's voltage and resistance, the
//! bulk capacitor, the motors' current while frames go out and its lag, the
//! regulator's dropout. Nothing in the repository declares an ESC's supply
//! current; the generated firmware sends a zero-throttle pattern, so a real
//! ESC would draw only its idle current. The map closes the loop through what
//! the firmware does (whether its timer runs), which is what is measured.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use aethelgard_chaos::cbor;
use aethelgard_chaos::record::{Reading, Record};
use aethelgard_chaos::replay::MAX_RECORDS;
use aethelgard_chaos::{Event, Header, TraceWriter, replay_dir};
use ng_probe::{Hooks, Spice};

const WAIT: Duration = Duration::from_secs(300);
const BOOT_WORD: u32 = 0x2000_0000;
const TIM1_CR1: u32 = 0x4001_0000;
const TIM1_CCR1: u32 = 0x4001_0034;
/// The quantum, 10 µs unless `Q_NS` says otherwise (option (b) of §1.3).
fn quantum_ns() -> u64 {
    static Q: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    *Q.get_or_init(|| {
        std::env::var("Q_NS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10_000)
    })
}
const NUDGE_NS: u64 = 10;

// The deck (AET-WO-2304 §1.5; FEEDBACK "The model, as designed"). Sentinel-03's
// declared tree, with the spike's model where nothing is declared.
/// The pack's floor: cells.min (3) x cell_min_millivolts (3000), the fixture.
const PACK_V: f64 = 9.0;
/// C1, UCZ1H470MCL1GS: 47 uF, ESR max 250 mOhm (Nichicon UCZ p. 2, the parts table).
const HOTPLUG_F: f64 = 47e-6;
const HOTPLUG_ESR: f64 = 0.25;
/// C10, C20 (10 uF) and C11, C21 (0.22 uF): the bucks' input ceramics, nominal.
const CIN_F: f64 = 20.44e-6;
/// The bucks' input power at the declared loads and the fixture's declared
/// efficiencies: U10 5.0 V x 2.000 A / 0.90, U11 3.3 V x 0.748 A / 0.81 (W).
const P_BUCKS_W: f64 = 5.0 * 2.0 / 0.90 + 3.3 * 0.748 / 0.81;
/// U11's duty-limited output, IR's `headroom` arithmetic (power.rs L1462-1491):
/// D 0.9609, RDS_hi 160 mOhm, RDS_lo 110 mOhm, DCR 16 mOhm, at 0.748 A, in V.
const D: f64 = 0.9609;
const I3: f64 = 0.748;
/// U11's enable, with the datasheet's hysteresis (SNVSAN3F p. 15, 8.3.4, typical:
/// starts at about 3.7 V, shuts down below about 3 V); decided at each boundary.
const EN_ON_V: f64 = 3.7;
const EN_OFF_V: f64 = 3.0;
const REG_OHMS: f64 = 0.05;
/// VDD_3V3: the emitter's decoupling (10.81 uF) plus U11's four 22 uF outputs
/// at nominal (C24-C27), FEEDBACK §1.8.
const DECOUPLING_F: f64 = 98.81e-6;
const LAG_S: f64 = 1e-3; // the ESC current's first-order rise (2303's; unsourced)
/// DS12117 Table 16 typical, as aethelgard-chaos (falling, rising).
const BOR3: (f64, f64) = (2.61, 2.70);
/// The declared (a) profile's step time: a scenario's number, not the artifact's.
const DECLARED_STEP_NS: u64 = 40_000_000;

fn pack_ohms(case: &str) -> f64 {
    match case {
        "healthy" => 0.05,
        "weak" => 1.0,
        // `r<ohms>`: a pack R named in the case (runs added after the
        // predictions: D1's band, either side of it).
        other => other
            .strip_prefix('r')
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| panic!("unknown case {other}")),
    }
}

/// The ESC current while the firmware's timer runs, A: from the run's
/// environment, never a default (INV-06).
fn run_a() -> f64 {
    static A: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *A.get_or_init(|| {
        std::env::var("I_RUN_A")
            .expect("I_RUN_A: the ESC current while CEN is set")
            .parse()
            .unwrap()
    })
}

fn deck(case: &str, span_ns: u64) -> Vec<String> {
    let avail = format!(
        "({D}*(v(bat)-{i}*0.160) - {k}*{i}*0.110 - {i}*0.016)",
        i = I3,
        k = 1.0 - D
    );
    vec![
        format!(
            "* aet2304 Sentinel-03 battery chain, {case}, I_RUN {} A",
            run_a()
        ),
        format!("vpack src 0 dc {PACK_V}"),
        format!("rpack src bat {}", pack_ohms(case)),
        format!("chot hot 0 {HOTPLUG_F} ic={PACK_V}"),
        format!("resr bat hot {HOTPLUG_ESR}"),
        format!("ccin bat 0 {CIN_F} ic={PACK_V}"),
        "vctl ctl 0 dc 0 external".to_owned(),
        // An external source adds the callback value to its dc value (measured), so dc 0.
        "ven en 0 dc 0 external".to_owned(),
        "rlag ctl lag 1k".to_owned(),
        format!("clag lag 0 {} ic=0", LAG_S / 1e3),
        "besc bat 0 i = v(lag)".to_owned(),
        // Both bucks: constant power from VBAT while enabled.
        format!("bbuck bat 0 i = v(en) * {P_BUCKS_W} / max(v(bat), 1.0)"),
        // U11: toward min(3.3 V, avail) through REG_OHMS while enabled.
        format!("breg 0 rail i = v(en) * (min(3.3, max(0, {avail})) - v(rail)) / {REG_OHMS}"),
        format!("cdec rail 0 {DECOUPLING_F} ic=3.3"),
        // The declared 748 mA, smoothed to zero below 0.5 V.
        format!("bload rail 0 i = {I3} * min(1, max(0, v(rail)) / 0.5)"),
        ".options reltol=1e-3 abstol=1e-12 vntol=1e-6 method=trap".to_owned(),
        // A 1 us step cap (2301 F12).
        format!(".tran 10n {span_ns}n 0 1000n uic"),
        ".end".to_owned(),
    ]
}

// ---------------------------------------------------------------- Renode

/// The CLI's setup commands from its captured argv: everything before its
/// first watchpoint hook (the judge's witness machinery: the TIM1 anchor and
/// its +2 ms stop, the boot word's serial injection), with the CLI's scratch
/// paths mapped to this run's copies, the peripheral-access logging, the
/// capture backends and the noisy log levels left out (deviations, FEEDBACK).
struct Setup {
    commands: Vec<String>,
    cold: Vec<String>,
    rearm: Vec<String>,
    vtor: String,
}

fn setup_from(script: &Path, tool: &Path) -> Setup {
    let argv = fs::read_to_string(script).unwrap();
    let line = argv
        .split('\n')
        .find(|a| a.contains("LoadPlatformDescription"))
        .expect("no monitor script in the argv");
    let all: Vec<&str> = line.split("; ").collect();
    let prefix = all[2]
        .split('@')
        .nth(1)
        .and_then(|p| p.split("/models/").next())
        .expect("no tool prefix")
        .to_owned();
    let mut commands = Vec::new();
    let (mut cold, mut rearm, mut vtor) = (Vec::new(), Vec::new(), String::new());
    for c in all {
        if c.starts_with("sysbus AddWatchpointHook") || c.starts_with("emulation RunFor") {
            break;
        }
        // FULL_SETUP keeps the CLI's witness logging and capture backends.
        if std::env::var_os("FULL_SETUP").is_none()
            && (c.starts_with("logLevel -1")
                || c.starts_with("sysbus LogPeripheralAccess")
                || c.contains("CreateFileBackend"))
        {
            continue;
        }
        // FULL_SETUP silences the motor observers as `2302` did ("observers
        // silenced"), so their edges do not split the console (2301 F8).
        let c = if c.starts_with("logLevel -1") && c.contains("obs") {
            c.replace("logLevel -1", "logLevel 3")
        } else {
            c.to_owned()
        };
        let c = c.replace(&prefix, &tool.display().to_string());
        if c.starts_with("sysbus WriteDoubleWord 0x5802") {
            cold.push(c.clone());
        } else if c.starts_with("sysbus WriteDoubleWord 0x2000") {
            rearm.push(c.clone());
        } else if c.starts_with("cpu VectorTableOffset") {
            vtor = c.clone();
        }
        commands.push(c);
    }
    Setup {
        commands,
        cold,
        rearm,
        vtor,
    }
}

struct Renode {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<String>,
    busy: Duration,
}

impl Renode {
    fn start(out: &Path, setup: &Setup) -> Self {
        let home = out.join("home");
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
        let o = child.stdout.take().unwrap();
        std::thread::spawn(move || {
            for line in BufReader::new(o).lines() {
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
            busy: Duration::ZERO,
        };
        let mut cmds = vec![
            format!("logFile @{}", out.join("renode.log").display()),
            "logLevel 3 console".to_owned(),
        ];
        cmds.extend(setup.commands.iter().cloned());
        for c in cmds {
            r.exec(&[c]);
        }
        r
    }

    /// One `;`-joined line and a constant sentinel (2301 F15): a quantum's
    /// line repeats the last, so AntShell's history does not grow.
    fn exec(&mut self, cmds: &[String]) -> Vec<String> {
        let t0 = Instant::now();
        let sentinel = "AET2303-Q";
        let text = format!("{}; echo \"{sentinel}\"\n", cmds.join("; "));
        self.stdin.write_all(text.as_bytes()).unwrap();
        self.stdin.flush().unwrap();
        let mut vals = Vec::new();
        loop {
            match self.rx.recv_timeout(WAIT) {
                Ok(l) if l.trim().ends_with(sentinel) => break,
                Ok(l) => {
                    let l = l.trim();
                    // 2301 F8: only whole value lines; the log is in a file.
                    if l.starts_with("0x") && l[2..].chars().all(|c| c.is_ascii_hexdigit()) {
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

    fn rss_kib(&self) -> u64 {
        tree_rss_kib(self.child.id())
    }

    fn quit(mut self) {
        let _ = self.stdin.write_all(b"quit\n");
        let _ = self.child.wait();
    }
}

/// VmRSS of a process and its descendants (Renode's launcher may fork).
fn tree_rss_kib(pid: u32) -> u64 {
    let mut total = status_kib(pid, "VmRSS:");
    if let Ok(kids) = fs::read_to_string(format!("/proc/{pid}/task/{pid}/children")) {
        for k in kids.split_whitespace().filter_map(|k| k.parse().ok()) {
            total += tree_rss_kib(k);
        }
    }
    total
}

fn status_kib(pid: u32, key: &str) -> u64 {
    fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with(key))
                .and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse().ok()))
        })
        .unwrap_or(0)
}

fn self_hwm_kib() -> u64 {
    status_kib(std::process::id(), "VmHWM:")
}

fn run_for(ns: u64) -> String {
    format!(
        "emulation RunFor \"{}.{:09}\"",
        ns / 1_000_000_000,
        ns % 1_000_000_000
    )
}

fn hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap_or(u32::MAX)
}

// ---------------------------------------------------------------- traces

fn header() -> Header {
    Header {
        run_id: [0x23; 16],
        artifacts: vec![("probe".to_owned(), [0; 32])],
        tools: Vec::new(),
    }
}

struct Traces {
    samples: TraceWriter,
    records: TraceWriter,
    events: TraceWriter,
    records_written: u64,
    refused: Option<String>,
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
            records_written: 0,
            refused: None,
        }
    }

    /// One boundary, two shapes. Samples: `V_bat` and `VDD_3V3` as kind 3,
    /// the firmware's command (TIM1 `CR1`) as kind 0. Records: the same three
    /// as stand-ins (no per-quantum record kind exists): kind 17
    /// (`RailCrossing`) for each rail, kind 9 (`RegisterObserved`) for `CR1`.
    fn quantum(&mut self, t_ns: u64, vbat: f64, vdd: f64, cr1: u32) {
        let uv = |v: f64| (v * 1e6).round() as i64;
        for (net, v) in [("VBAT", vbat), ("VDD_3V3", vdd)] {
            self.samples
                .append(&Event::AnalogSample {
                    timestamp: t_ns,
                    net: net.to_owned(),
                    microvolts: uv(v),
                })
                .unwrap();
        }
        self.samples
            .append(&Event::RegisterWrite {
                timestamp: t_ns,
                address: TIM1_CR1,
                value: cr1,
            })
            .unwrap();
        if self.refused.is_some() {
            return;
        }
        let recs = [
            Record::RailCrossing {
                domain: "VBAT".to_owned(),
                level: "quantum".to_owned(),
                microvolts: uv(vbat).max(0) as u64,
                at_ps: t_ns * 1000,
                falling: false,
            },
            Record::RailCrossing {
                domain: "VDD_3V3".to_owned(),
                level: "quantum".to_owned(),
                microvolts: uv(vdd).max(0) as u64,
                at_ps: t_ns * 1000,
                falling: false,
            },
            Record::RegisterObserved {
                name: "TIM1_CR1".to_owned(),
                reading: Reading {
                    address: TIM1_CR1,
                    mask: 1,
                    expected: 1,
                    observed: Some(cr1),
                },
            },
        ];
        for record in recs {
            match self.records.append(&Event::Record {
                timestamp: t_ns,
                record,
            }) {
                Ok(()) => self.records_written += 1,
                Err(e) => {
                    self.refused = Some(format!(
                        "at {t_ns} ns, record {}: {e}",
                        self.records_written + 1
                    ));
                    return;
                }
            }
        }
    }

    fn event(&mut self, t_ns: u64, record: Record) {
        self.events
            .append(&Event::Record {
                timestamp: t_ns,
                record,
            })
            .unwrap();
    }

    fn commit(self) -> String {
        let mut s = format!(
            ",\"records_written\":{},\"records_refused\":{}",
            self.records_written,
            self.refused.map_or("null".to_owned(), |r| format!("{r:?}"))
        );
        for (name, w) in [
            ("samples", self.samples),
            ("records", self.records),
            ("events", self.events),
        ] {
            let m = w.commit().unwrap();
            s.push_str(&format!(
                ",\"trace_{name}_bytes\":{},\"trace_{name}_events\":{}",
                m.trace_bytes, m.event_count
            ));
        }
        s
    }
}

// ---------------------------------------------------------------- the hook

struct Ioc {
    quanta: u64,
    next_k: u64,
    renode: Option<Renode>,
    setup: Setup,
    traces: Option<Traces>,
    /// The motors' commanded current for the next quantum, A.
    ctl: f64,
    /// U11/U10's enable (hysteresis, decided at each boundary).
    en: bool,
    /// Declared mode (candidate (a)): no Renode; the profile is the load.
    declared: bool,
    held: bool,
    nudged: u64,
    last: (u32, u32, u32),
    actions: Vec<String>,
    cen_time_ns: u64,
    rss: Vec<(u64, u64)>,
    since: Instant,
    ng_busy: Duration,
}

impl Ioc {
    fn boundary(&mut self) {
        self.ng_busy += self.since.elapsed();
        let k = self.next_k;
        let t_ns = k * quantum_ns();
        let vbat = ng_probe::ffi::vec_last("bat").map_or(f64::NAN, |(_, v)| v);
        let vdd = ng_probe::ffi::vec_last("rail").map_or(f64::NAN, |(_, v)| v);
        // The bucks' enable, from the boundary sample (both candidates).
        if self.en && vbat < EN_OFF_V {
            self.en = false;
            self.actions
                .push(format!("{t_ns} bucks off vbat={vbat:.4}"));
        } else if !self.en && vbat >= EN_ON_V {
            self.en = true;
            self.actions.push(format!("{t_ns} bucks on vbat={vbat:.4}"));
        }
        if self.declared {
            // Candidate (a): the declared profile, nothing from a firmware.
            self.ctl = if t_ns >= DECLARED_STEP_NS {
                run_a()
            } else {
                0.0
            };
            self.next_k += 1;
            self.since = Instant::now();
            return;
        }
        // 1. Renode runs the quantum that ends here; three words are read.
        let got = self.renode.as_mut().unwrap().exec(&[
            run_for(quantum_ns() - std::mem::take(&mut self.nudged)),
            format!("sysbus ReadDoubleWord {BOOT_WORD:#010x}"),
            format!("sysbus ReadDoubleWord {TIM1_CR1:#010x}"),
            format!("sysbus ReadDoubleWord {TIM1_CCR1:#010x}"),
        ]);
        assert_eq!(got.len(), 3, "reads at {t_ns}: {got:?}");
        let now = (hex(&got[0]), hex(&got[1]), hex(&got[2]));
        let cen = now.1 & 1;
        if cen == 1 {
            self.cen_time_ns += quantum_ns();
        }
        self.traces
            .as_mut()
            .unwrap()
            .quantum(t_ns, vbat, vdd, now.1);
        if now != self.last {
            self.actions.push(format!(
                "{t_ns} renode boot={:#x} cr1={:#x} ccr1={:#x}",
                now.0, now.1, now.2
            ));
            self.traces.as_mut().unwrap().event(
                t_ns,
                Record::RegisterObserved {
                    name: "TIM1_CR1".to_owned(),
                    reading: Reading {
                        address: TIM1_CR1,
                        mask: 1,
                        expected: 1,
                        observed: Some(now.1),
                    },
                },
            );
            self.last = now;
        }
        // 2. The bridge's decision, from the boundary sample (2301's rule).
        if !self.held && vdd < BOR3.0 {
            self.held = true;
            let mut c = vec!["machine Reset".to_owned()];
            c.extend(self.setup.cold.iter().cloned());
            c.push(self.setup.vtor.clone());
            c.push("cpu IsHalted true".to_owned());
            c.extend(self.setup.rearm.iter().cloned());
            self.renode.as_mut().unwrap().exec(&c);
            self.actions.push(format!(
                "{t_ns} bridge reset_asserted vdd={vdd:.4} vbat={vbat:.4}"
            ));
            self.traces
                .as_mut()
                .unwrap()
                .event(t_ns, rendered(t_ns, false));
        } else if self.held && vdd > BOR3.1 {
            self.held = false;
            self.renode
                .as_mut()
                .unwrap()
                .exec(&["cpu IsHalted false".to_owned(), run_for(NUDGE_NS)]);
            self.nudged = NUDGE_NS;
            self.actions.push(format!(
                "{t_ns} bridge reset_released vdd={vdd:.4} vbat={vbat:.4}"
            ));
            self.traces
                .as_mut()
                .unwrap()
                .event(t_ns, rendered(t_ns, true));
        }
        // 3. The load for the next quantum: what the firmware's timer does now
        // (a reset has just cleared it, if one was asserted).
        self.ctl = if self.held {
            0.0
        } else {
            f64::from(cen) * run_a()
        };
        if t_ns % 10_000_000 == 0 {
            self.rss
                .push((t_ns, self.renode.as_ref().unwrap().rss_kib()));
        }
        self.next_k += 1;
        self.since = Instant::now();
    }
}

fn rendered(t_ns: u64, released: bool) -> Record {
    Record::ResetRendered {
        at_ns: t_ns,
        released,
        cause: vec![
            if released {
                "VBOR3_rising"
            } else {
                "VBOR3_falling"
            }
            .to_owned(),
        ],
        rendering: "held reset, the CLI's rendering".to_owned(),
    }
}

impl Hooks for Ioc {
    fn vsrc(&mut self, _time: f64, name: &str) -> f64 {
        if name.eq_ignore_ascii_case("ven") {
            if self.en { 1.0 } else { 0.0 }
        } else {
            self.ctl
        }
    }
    fn sync(&mut self, time: f64, _delta: &mut f64, _old: f64, _redo: i32, loc: i32) -> i32 {
        if loc == 0
            && self.next_k <= self.quanta
            && time >= (self.next_k * quantum_ns()) as f64 * 1e-9 * (1.0 - 1e-12)
        {
            self.boundary();
        }
        0
    }
}

fn lockstep(a: &[String]) {
    let case = a[1].clone();
    let span_ns: u64 = a[2].parse().unwrap();
    let script = PathBuf::from(&a[3]);
    let out = PathBuf::from(&a[4]);
    let tool = PathBuf::from(std::env::var("TOOL").expect("TOOL: the run's copies"));
    fs::create_dir_all(&out).unwrap();
    let quanta = span_ns / quantum_ns();
    let t0 = Instant::now();
    let setup = setup_from(&script, &tool);
    fs::write(out.join("setup.cmds"), setup.commands.join("\n")).unwrap();
    let renode = Renode::start(&out, &setup);
    let renode_start_s = t0.elapsed().as_secs_f64();
    let rss_start = renode.rss_kib();
    let mut sp = Spice::init(Ioc {
        quanta,
        next_k: 1,
        renode: Some(renode),
        setup,
        traces: Some(Traces::create(&out)),
        ctl: 0.0,
        en: true,
        declared: false,
        held: false,
        nudged: 0,
        last: (u32::MAX, u32::MAX, u32::MAX),
        actions: Vec::new(),
        cen_time_ns: 0,
        rss: Vec::new(),
        since: Instant::now(),
        ng_busy: Duration::ZERO,
    })
    .unwrap();
    sp.circ(&deck(&case, span_ns)).unwrap();
    for k in 1..=quanta {
        assert!(
            sp.set_bkpt((k * quantum_ns()) as f64 * 1e-9),
            "breakpoint {k}"
        );
    }
    let t_run = Instant::now();
    sp.hooks().since = Instant::now();
    let r = sp.command("run");
    assert_eq!(r, 0, "run: {:?}", sp.take_log());
    let h = sp.hooks();
    // ngspice does not call GetSyncData at tstop itself (2301).
    if h.next_k == quanta {
        h.since = Instant::now();
        h.boundary();
    }
    assert_eq!(h.next_k, quanta + 1, "missed boundaries");
    let run_s = t_run.elapsed().as_secs_f64();
    let rss_end = h.renode.as_ref().unwrap().rss_kib();
    let fin = h.renode.as_mut().unwrap().exec(&[
        "sysbus ReadDoubleWord 0x20000000".to_owned(),
        "sysbus ReadDoubleWord 0x20000004".to_owned(),
        "cpu PC".to_owned(),
    ]);
    let busy = h.renode.as_ref().unwrap().busy.as_secs_f64();
    let ng_busy = h.ng_busy.as_secs_f64();
    let cen_ms = h.cen_time_ns as f64 / 1e6;
    let actions = std::mem::take(&mut h.actions);
    let rss = std::mem::take(&mut h.rss);
    let renode = h.renode.take().unwrap();
    let traces = h.traces.take().unwrap();
    renode.quit();
    let tail = traces.commit();
    fs::write(out.join("actions.txt"), actions.join("\n") + "\n").unwrap();
    let rss_s: Vec<String> = rss.iter().map(|(t, k)| format!("[{t},{k}]")).collect();
    println!(
        "{{\"case\":\"{case}\",\"i_run_a\":{},\"pack_ohms\":{},\"quantum_ns\":{},\"full_setup\":{},\"span_ns\":{span_ns},\"quanta\":{quanta},\"renode_start_s\":{renode_start_s:.3},\"run_s\":{run_s:.3},\"renode_busy_s\":{busy:.3},\"ngspice_between_boundaries_s\":{ng_busy:.3},\"wall_s\":{:.3},\"cen_set_ms\":{cen_ms:.3},\"renode_rss_kib_start\":{rss_start},\"renode_rss_kib_end\":{rss_end},\"probe_hwm_kib\":{},\"final\":{fin:?},\"resets\":{},\"actions\":{}{tail},\"rss_every_10ms\":[{}]}}",
        run_a(),
        pack_ohms(&case),
        quantum_ns(),
        std::env::var_os("FULL_SETUP").is_some(),
        t0.elapsed().as_secs_f64(),
        self_hwm_kib(),
        actions
            .iter()
            .filter(|a| a.contains("reset_asserted"))
            .count(),
        actions.len(),
        rss_s.join(",")
    );
}

// ---------------------------------------------------------------- candidate (a)

/// `declared CASE SPAN_NS OUT`: candidate (a). The same deck, solved once by
/// ngspice with no Renode: the ESC current steps to `I_RUN_A` at the declared
/// time and stays (it does not follow any firmware). The bucks' enable is
/// decided at each 10 us boundary from V_bat, as in (b). `VDD_3V3`'s crossings
/// of the active levels (`aethelgard_chaos::power::levels(3)`) are found on
/// ngspice's own points, linearly interpolated to the ps, and scheduled by
/// `aethelgard_chaos::power::schedule(3, ..)`. The events trace holds the
/// crossings (kind 17) and the resets (kind 24) with the real writer.
fn declared(a: &[String]) {
    use aethelgard_chaos::power::{Crossing, levels, schedule};
    let case = a[1].clone();
    let span_ns: u64 = a[2].parse().unwrap();
    let out = PathBuf::from(&a[3]);
    fs::create_dir_all(&out).unwrap();
    let quanta = span_ns / quantum_ns();
    let t0 = Instant::now();
    let mut sp = Spice::init(Ioc {
        quanta,
        next_k: 1,
        renode: None,
        setup: Setup {
            commands: Vec::new(),
            cold: Vec::new(),
            rearm: Vec::new(),
            vtor: String::new(),
        },
        traces: None,
        ctl: 0.0,
        en: true,
        declared: true,
        held: false,
        nudged: 0,
        last: (u32::MAX, u32::MAX, u32::MAX),
        actions: Vec::new(),
        cen_time_ns: 0,
        rss: Vec::new(),
        since: Instant::now(),
        ng_busy: Duration::ZERO,
    })
    .unwrap();
    fs::write(out.join("deck.cir"), deck(&case, span_ns).join("\n") + "\n").unwrap();
    sp.circ(&deck(&case, span_ns)).unwrap();
    for k in 1..=quanta {
        assert!(
            sp.set_bkpt((k * quantum_ns()) as f64 * 1e-9),
            "breakpoint {k}"
        );
    }
    let r = sp.command("run");
    assert_eq!(r, 0, "run: {:?}", sp.take_log());
    let solve_s = t0.elapsed().as_secs_f64();
    let time = sp.vec("time").expect("time");
    let rail = sp.vec("rail").expect("rail");
    let bat = sp.vec("bat").expect("bat");
    if let Some(n) = std::env::var("DUMP")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
    {
        for i in (0..time.len()).step_by(n.max(1)) {
            eprintln!(
                "dump t={:.6e} bat={:.4} rail={:.4}",
                time[i], bat[i], rail[i]
            );
        }
        for v in ["en", "lag", "ctl", "src", "hot"] {
            let x = sp.vec(v).unwrap_or_default();
            eprintln!("vec {v}: first {:?} last {:?}", x.first(), x.last());
        }
        let log = sp.take_log();
        eprintln!("log: {}", log.join(" | "));
    }
    let mut crossings: Vec<Crossing> = Vec::new();
    for (level, uv) in levels(3) {
        let l = uv as f64 * 1e-6;
        for i in 1..rail.len() {
            let (v0, v1) = (rail[i - 1], rail[i]);
            if (v0 - l) * (v1 - l) < 0.0 || (v1 == l && v0 != l) {
                let f = (l - v0) / (v1 - v0);
                let t = time[i - 1] + f * (time[i] - time[i - 1]);
                crossings.push(Crossing {
                    level: level.clone(),
                    microvolts: uv,
                    at_ps: (t * 1e12).round() as u64,
                    falling: v1 < v0,
                });
            }
        }
    }
    crossings.sort_by_key(|c| c.at_ps);
    let resets = schedule(3, &crossings);
    // One time-ordered list: each crossing (kind 17) and each scheduled reset
    // edge (kind 24); at equal times the crossing comes first.
    let mut evs: Vec<(u64, u8, Record)> = crossings
        .iter()
        .map(|c| {
            (
                c.at_ps / 1000,
                0,
                Record::RailCrossing {
                    domain: "VDD_3V3".to_owned(),
                    level: c.level.clone(),
                    microvolts: c.microvolts,
                    at_ps: c.at_ps,
                    falling: c.falling,
                },
            )
        })
        .collect();
    let mut resets_json = Vec::new();
    if let Ok(rs) = &resets {
        for r in rs {
            let edge = |t: u64, released: bool| {
                (
                    t,
                    1,
                    Record::ResetRendered {
                        at_ns: t,
                        released,
                        cause: r.cause.clone(),
                        rendering: "held reset, the CLI's rendering".to_owned(),
                    },
                )
            };
            evs.push(edge(r.asserted_ns, false));
            if let Some(t) = r.released_ns {
                evs.push(edge(t, true));
            }
            resets_json.push(format!(
                "[{},{}]",
                r.asserted_ns,
                r.released_ns.map_or("null".to_owned(), |t| t.to_string())
            ));
        }
    }
    evs.sort_by_key(|e| (e.0, e.1));
    let mut traces = Traces::create(&out);
    for (t, _, rec) in evs {
        traces.event(t, rec);
    }
    let tail = traces.commit();
    let (min_i, min_v) = rail
        .iter()
        .enumerate()
        .fold((0, f64::MAX), |m, (i, &v)| if v < m.1 { (i, v) } else { m });
    let min_bat = bat.iter().cloned().fold(f64::MAX, f64::min);
    let xs: Vec<String> = crossings
        .iter()
        .map(|c| format!("[\"{}\",{},{}]", c.level, c.at_ps, c.falling))
        .collect();
    fs::write(
        out.join("actions.txt"),
        sp.hooks().actions.join("\n") + "\n",
    )
    .unwrap();
    println!(
        "{{\"mode\":\"declared\",\"case\":\"{case}\",\"i_run_a\":{},\"pack_ohms\":{},\"step_ns\":{DECLARED_STEP_NS},\"span_ns\":{span_ns},\"points\":{},\"solve_s\":{solve_s:.3},\"probe_hwm_kib\":{},\"vdd_min_v\":{min_v:.4},\"vdd_min_at_ns\":{},\"vbat_min_v\":{min_bat:.4},\"vbat_end_v\":{:.4},\"vdd_end_v\":{:.4},\"crossings\":[{}],\"schedule\":{},\"resets\":[{}]{tail}}}",
        run_a(),
        pack_ohms(&case),
        rail.len(),
        self_hwm_kib(),
        (time[min_i] * 1e9) as u64,
        bat.last().unwrap(),
        rail.last().unwrap(),
        xs.join(","),
        match &resets {
            Ok(_) => "\"ok\"".to_owned(),
            Err(e) => format!("{e:?}"),
        },
        resets_json.join(",")
    );
}

// ---------------------------------------------------------------- replay

/// Each event of a trace, with its byte range.
fn events_of(trace: &[u8]) -> (Header, Vec<(Event, usize, usize)>) {
    let (hv, mut off) = cbor::decode_prefix(trace, 0).unwrap();
    let header = Header::from_value(&hv).unwrap();
    let mut out = Vec::new();
    while off < trace.len() {
        let (v, end) = cbor::decode_prefix(trace, off).unwrap();
        out.push((Event::from_value(&v).unwrap(), off, end));
        off = end;
    }
    (header, out)
}

/// `stats DIR`: each per-quantum net's minimum and maximum (with their
/// times) and its value at the run's end, read from `trace-samples` alone.
fn stats(dir: &Path) {
    let bytes = fs::read(dir.join("trace-samples").join("trace.cbor")).unwrap();
    let (_, evs) = events_of(&bytes);
    let mut m: std::collections::BTreeMap<String, (i64, u64, i64, u64, i64)> =
        std::collections::BTreeMap::new();
    for (e, _, _) in &evs {
        if let Event::AnalogSample {
            timestamp,
            net,
            microvolts,
        } = e
        {
            let s = m
                .entry(net.clone())
                .or_insert((i64::MAX, 0, i64::MIN, 0, 0));
            if *microvolts < s.0 {
                s.0 = *microvolts;
                s.1 = *timestamp;
            }
            if *microvolts > s.2 {
                s.2 = *microvolts;
                s.3 = *timestamp;
            }
            s.4 = *microvolts;
        }
    }
    for (net, (lo, tlo, hi, thi, end)) in m {
        println!(
            "{{\"net\":\"{net}\",\"min_uv\":{lo},\"min_at_ns\":{tlo},\"max_uv\":{hi},\"max_at_ns\":{thi},\"end_uv\":{end}}}"
        );
    }
}

fn replay(dir: &Path) {
    for name in ["trace-samples", "trace-records", "trace-events"] {
        let d = dir.join(name);
        let t0 = Instant::now();
        match replay_dir(&d) {
            Ok(r) => println!(
                "{{\"trace\":\"{name}\",\"ok\":true,\"events\":{},\"trace_bytes\":{},\"records_folded\":{},\"state_hash\":\"{}\",\"replay_s\":{:.3},\"hwm_kib_so_far\":{}}}",
                r.manifest.event_count,
                r.manifest.trace_bytes,
                r.state.records.len(),
                r.manifest
                    .state_hash
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
                t0.elapsed().as_secs_f64(),
                self_hwm_kib()
            ),
            Err(e) => println!(
                "{{\"trace\":\"{name}\",\"ok\":false,\"error\":\"{}\",\"message\":{:?}}}",
                e.kind(),
                e.to_string()
            ),
        }
    }
    // The bridge's actions, re-derived from the recorded VDD_3V3 alone.
    let bytes = fs::read(dir.join("trace-samples/trace.cbor")).unwrap();
    let (_, evs) = events_of(&bytes);
    let mut held = false;
    let mut derived = Vec::new();
    for (e, _, _) in &evs {
        if let Event::AnalogSample {
            timestamp,
            net,
            microvolts,
        } = e
            && net == "VDD_3V3"
        {
            let v = *microvolts as f64 / 1e6;
            if !held && v < BOR3.0 {
                held = true;
                derived.push(format!("{timestamp} reset_asserted"));
            } else if held && v > BOR3.1 {
                held = false;
                derived.push(format!("{timestamp} reset_released"));
            }
        }
    }
    // Against the archive's own ResetRendered records (trace-events), not a
    // side file: a consistency check of the bridge's rule over its inputs.
    let ev = fs::read(dir.join("trace-events/trace.cbor")).unwrap();
    let recorded: Vec<String> = events_of(&ev)
        .1
        .into_iter()
        .filter_map(|(e, _, _)| match e {
            Event::Record {
                record:
                    Record::ResetRendered {
                        at_ns, released, ..
                    },
                ..
            } => Some(format!(
                "{at_ns} {}",
                if released {
                    "reset_released"
                } else {
                    "reset_asserted"
                }
            )),
            _ => None,
        })
        .collect();
    println!(
        "{{\"rejudge\":\"bridge actions from the VDD_3V3 samples alone, against trace-events' ResetRendered records\",\"derived\":{},\"recorded\":{},\"equal\":{}}}",
        derived.len(),
        recorded.len(),
        derived == recorded
    );
}

fn flip(dir: &Path, out: &Path) {
    let d = dir.join("trace-samples");
    let _ = fs::remove_dir_all(out);
    fs::create_dir_all(out).unwrap();
    for f in ["trace.cbor", "manifest.cbor"] {
        fs::copy(d.join(f), out.join(f)).unwrap();
    }
    let mut bytes = fs::read(out.join("trace.cbor")).unwrap();
    let (_, evs) = events_of(&bytes);
    let mid = evs.len() / 2;
    let (i, (e, _, end)) = evs
        .iter()
        .enumerate()
        .skip(mid)
        .find(|(_, (e, _, _))| matches!(e, Event::AnalogSample { .. }))
        .unwrap();
    // The last byte of a sample's encoding is the low byte of its microvolts.
    bytes[end - 1] ^= 0x01;
    fs::write(out.join("trace.cbor"), &bytes).unwrap();
    println!(
        "{{\"flipped_event\":{i},\"of\":{},\"byte\":{},\"event\":{:?}}}",
        evs.len(),
        end - 1,
        format!("{e:?}")
    );
    match replay_dir(out) {
        Ok(r) => println!("{{\"ok\":true,\"events\":{}}}", r.manifest.event_count),
        Err(e) => println!(
            "{{\"sealed\":\"no\",\"ok\":false,\"error\":\"{}\",\"message\":{:?}}}",
            e.kind(),
            e.to_string()
        ),
    }
    // The manifest re-sealed over the flipped trace (its sha256 recomputed,
    // the state hash kept): only the state-sequence hash chain can catch it.
    use sha2::{Digest, Sha256};
    let mv = cbor::decode_exact(&fs::read(out.join("manifest.cbor")).unwrap()).unwrap();
    let mut m = aethelgard_chaos::Manifest::from_value(&mv).unwrap();
    m.trace_sha256 = Sha256::digest(&bytes).into();
    fs::write(out.join("manifest.cbor"), cbor::to_bytes(&m.to_value())).unwrap();
    match replay_dir(out) {
        Ok(r) => println!(
            "{{\"sealed\":\"yes\",\"ok\":true,\"events\":{}}}",
            r.manifest.event_count
        ),
        Err(e) => println!(
            "{{\"sealed\":\"yes\",\"ok\":false,\"error\":\"{}\",\"message\":{:?}}}",
            e.kind(),
            e.to_string()
        ),
    }
}

/// A real campaign run, re-written and sealed by the real writer with
/// per-quantum data inserted, into `OUTBASE/{a,b}/<run id>` for the CLI's
/// `replay`, inserted after the run's first event.
fn splice(run: &Path, base: &Path) {
    let bytes = fs::read(run.join("trace.cbor")).unwrap();
    let (header, evs) = events_of(&bytes);
    let id: String = header.run_id.iter().map(|b| format!("{b:02x}")).collect();
    // Beside the run's own rail crossings (kind 17) when it has them, else
    // after its first event.
    let at = evs
        .iter()
        .rposition(|(e, _, _)| {
            matches!(
                e,
                Event::Record {
                    record: Record::RailCrossing { .. },
                    ..
                }
            )
        })
        .map_or(1, |i| i + 1);
    let t = evs[at - 1].0.timestamp();
    // In time order after the run's last droop record (rank 7 of
    // `check_run`), so the campaign's order rule is met.
    let last_ps =
        evs.iter()
            .filter_map(|(e, _, _)| match e {
                Event::Record { record, .. } => match record {
                    Record::RailCrossing { at_ps, .. } => Some(*at_ps),
                    Record::ResetRendered { at_ns, .. }
                    | Record::FirmwareObserved { at_ns, .. } => Some(at_ns * 1000),
                    _ => None,
                },
                _ => None,
            })
            .max()
            .unwrap_or(0);
    let at = evs
        .iter()
        .rposition(|(e, _, _)| {
            matches!(
                e,
                Event::Record {
                    record: Record::RailCrossing { .. }
                        | Record::ResetRendered { .. }
                        | Record::FirmwareObserved { .. },
                    ..
                }
            )
        })
        .map_or(at, |i| i + 1);
    // The event's timestamp: the one before the insertion point (the
    // writer refuses a timestamp that goes back).
    let t = evs[at - 1].0.timestamp();
    println!("{{\"inserted_at_event\":{at},\"of\":{}}}", evs.len());
    for (variant, extra) in [
        (
            "a",
            vec![Event::AnalogSample {
                timestamp: t,
                net: "VBAT".to_owned(),
                microvolts: 11_100_000,
            }],
        ),
        (
            "b",
            (0..1000u64)
                .map(|k| Event::Record {
                    timestamp: t,
                    record: Record::RailCrossing {
                        domain: "VDD_3V3".to_owned(),
                        level: "quantum".to_owned(),
                        microvolts: 3_300_000,
                        at_ps: last_ps + (k + 1) * quantum_ns() * 1000,
                        falling: false,
                    },
                })
                .collect(),
        ),
    ] {
        let d = base.join(variant).join(&id);
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(d.parent().unwrap()).unwrap();
        let mut w = TraceWriter::create(&d, &header).unwrap();
        for (i, (e, _, _)) in evs.iter().enumerate() {
            if i == at {
                for x in &extra {
                    w.append(x).unwrap();
                }
            }
            w.append(e).unwrap();
        }
        let m = w.commit().unwrap();
        println!(
            "{{\"variant\":\"{variant}\",\"dir\":\"{}\",\"events\":{},\"inserted\":{}}}",
            d.display(),
            m.event_count,
            extra.len()
        );
    }
}

/// The samples shape past the measured run: `SRC`'s per-quantum events,
/// repeated with their timestamps moved on 100 ms each lap, written by the real
/// writer for `SECONDS`, counting the bytes as they go (the writer enforces no
/// trace bound; the reader does, `MAX_TRACE_BYTES`). Then `replay_dir`.
fn synth(seconds: f64, src: &Path, out: &Path) {
    use aethelgard_chaos::archive::MAX_TRACE_BYTES;
    let bytes = fs::read(src.join("trace-samples/trace.cbor")).unwrap();
    let (header, evs) = events_of(&bytes);
    let lap_ns = 100_000_000u64;
    let header_bytes = evs[0].1 as u64;
    let _ = fs::remove_dir_all(out);
    fs::create_dir_all(out.parent().unwrap()).unwrap();
    let t0 = Instant::now();
    let mut w = TraceWriter::create(out, &header).unwrap();
    let mut total = header_bytes;
    let mut crossed: Option<u64> = None;
    let mut per_s: Vec<(u64, u64)> = Vec::new();
    let end_ns = (seconds * 1e9) as u64;
    let mut buf = Vec::new();
    'outer: for lap in 0.. {
        for (e, _, _) in &evs {
            let shift = |t: u64| t + lap * lap_ns;
            let e = match e {
                Event::AnalogSample {
                    timestamp,
                    net,
                    microvolts,
                } => Event::AnalogSample {
                    timestamp: shift(*timestamp),
                    net: net.clone(),
                    microvolts: *microvolts,
                },
                Event::RegisterWrite {
                    timestamp,
                    address,
                    value,
                } => Event::RegisterWrite {
                    timestamp: shift(*timestamp),
                    address: *address,
                    value: *value,
                },
                other => panic!("unexpected {other:?}"),
            };
            if e.timestamp() > end_ns {
                break 'outer;
            }
            buf.clear();
            cbor::encode(&e.to_value(), &mut buf);
            total += buf.len() as u64;
            if crossed.is_none() && total > MAX_TRACE_BYTES {
                crossed = Some(e.timestamp());
            }
            if e.timestamp() % 1_000_000_000 == 0 && matches!(e, Event::RegisterWrite { .. }) {
                per_s.push((e.timestamp(), total));
            }
            w.append(&e).unwrap();
        }
    }
    let m = w.commit().unwrap();
    let write_s = t0.elapsed().as_secs_f64();
    let per: Vec<String> = per_s.iter().map(|(t, b)| format!("[{t},{b}]")).collect();
    println!(
        "{{\"seconds\":{seconds},\"events\":{},\"trace_bytes\":{},\"counted\":{total},\"crossed_256mib_at_ns\":{},\"write_s\":{write_s:.3},\"bytes_at_each_second\":[{}]}}",
        m.event_count,
        m.trace_bytes,
        crossed.map_or("null".to_owned(), |t| t.to_string()),
        per.join(",")
    );
    let t1 = Instant::now();
    match replay_dir(out) {
        Ok(r) => println!(
            "{{\"replay\":\"ok\",\"events\":{},\"replay_s\":{:.3},\"hwm_kib\":{}}}",
            r.manifest.event_count,
            t1.elapsed().as_secs_f64(),
            self_hwm_kib()
        ),
        Err(e) => println!(
            "{{\"replay\":\"refused\",\"error\":\"{}\",\"message\":{:?},\"replay_s\":{:.3},\"hwm_kib\":{}}}",
            e.kind(),
            e.to_string(),
            t1.elapsed().as_secs_f64(),
            self_hwm_kib()
        ),
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a.first().map(String::as_str) {
        Some("lockstep") => lockstep(&a),
        Some("declared") => declared(&a),
        Some("stats") => stats(Path::new(&a[1])),
        Some("replay") => replay(Path::new(&a[1])),
        Some("flip") => flip(Path::new(&a[1]), Path::new(&a[2])),
        Some("splice") => splice(Path::new(&a[1]), Path::new(&a[2])),
        Some("synth") => synth(a[1].parse().unwrap(), Path::new(&a[2]), Path::new(&a[3])),
        _ => panic!(
            "usage: lockstep CASE SPAN_NS SCRIPT OUT | replay DIR | flip DIR OUT | splice RUNDIR OUTBASE (MAX_RECORDS {MAX_RECORDS})"
        ),
    }
}
