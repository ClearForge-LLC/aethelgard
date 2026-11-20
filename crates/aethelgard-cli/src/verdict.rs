//! The one shape every tool verdict takes (`crates/aethelgard-cli/README.md`).

use serde::Serialize;

use crate::input::Input;
use crate::run::{SandboxReport, Tool};

#[derive(Serialize)]
pub struct Verdict {
    pub ok: bool,
    pub verdict: &'static str,
    pub check: &'static str,
    /// `simulate --spec`: the lifecycle banner, one line per `eol` or `nrnd`
    /// part the board uses (`aethelgard_ir::lifecycle_banner`; `AET-WO-4210`
    /// D1). Absent for a board of `active` parts.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lifecycle: Vec<String>,
    pub input: InputReport,
    pub tool: ToolReport,
    pub sandbox: SandboxReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub included_severities: Option<&'static [&'static str]>,
    /// `check drc` only: whose rules judged, `specification` or `kicad-default`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<&'static str>,
    /// `check drc --spec` only: the sha256 of the specification file as read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specification_sha256: Option<String>,
    /// `check drc --schematic` only: the schematic parity was judged against.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schematic: Option<InputReport>,
    /// `check drc --spec` of a specification that declares coupled pairs
    /// (`AET-WO-1308` §1.3): each pair as the board's copper measures it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pairs: Option<Vec<PairRecord>>,
    /// `check drc --spec` of a specification with a power tree
    /// (`AET-WO-4300d2`): its loop rules as the board measures them, each
    /// class with its width, and what the board cannot show.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power: Option<PowerRecord>,
    /// `check support` only (`AET-WO-4300g`): the MCU's support network as
    /// the schematic's netlist and the board's pads show it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support: Option<SupportRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness: Option<String>,
    /// `prove` only (`AET-WO-3301` §1.4): `generated` when the crate copied is
    /// byte for byte the board crate `generate` writes for `--spec`, else
    /// `supplied`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness_provenance: Option<&'static str>,
    /// `prove` only: the cover properties Kani reported for the harness, 0
    /// when it reported none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_count: Option<u32>,
    /// `build` only: the ELF written, and its sha256.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elf: Option<InputReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine: Option<Machine>,
    /// `simulate --firmware` only: what the boot was judged by.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot: Option<Boot>,
    /// `simulate --foreign` only (`AET-WO-4400e` D3): the foreign judge's
    /// report, under the same `boot` key with `judge: "foreign"`. Exactly one
    /// of the two is set, so a generated boot's verdict is byte for byte what
    /// it was.
    #[serde(rename = "boot", skip_serializing_if = "Option::is_none")]
    pub foreign_boot: Option<crate::renode::foreign::ForeignBoot>,
    /// `simulate --firmware` only: the analog pre-pass's measurements and its
    /// fault decision, each its own record, apart from `boot` (`AET-WO-2200`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analog: Option<Vec<AnalogRecord>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faults: Option<Vec<FaultRecord>>,
    /// A campaign droop only (`AET-WO-2301`): the declared rail, ngspice's
    /// crossings, the resets the bridge rendered and what Renode read after
    /// each release, each its own record with its source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub droop: Option<DroopReport>,
    /// `simulate --firmware` only: the boot's peak tree RSS, as the reaper's
    /// memory bound sampled it, and its share of the bound (`AET-WO-2302`
    /// D6). A sample, so it differs run to run: kept apart from `boot`, from
    /// every finding and from the campaign's archive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryReport>,
    pub findings: Vec<Finding>,
    /// `simulate --firmware` only: each tool that judged the boot and the
    /// version it reported, for a campaign's archive header (`AET-WO-2205`
    /// §1.1). Not part of the verdict's JSON.
    #[serde(skip)]
    pub tools: Vec<(&'static str, String)>,
}

/// One coupled pair, measured on the board's copper by
/// `aethelgard_eda::route::pairs`: per conductor (positive, negative) its
/// length, its vias and its uncoupled length at each end, and the bounds it
/// was judged by.
#[derive(Serialize)]
pub struct PairRecord {
    pub name: String,
    pub nets: [String; 2],
    pub gap_nm: i64,
    pub layers: Vec<&'static str>,
    pub length_nm: [i64; 2],
    pub length_difference_nm: i64,
    pub length_difference_max_nm: i64,
    pub vias: [usize; 2],
    pub uncoupled_nm: [[i64; 2]; 2],
    pub uncoupled_max_nm: i64,
}

impl From<aethelgard_eda::route::PairReport> for PairRecord {
    fn from(p: aethelgard_eda::route::PairReport) -> PairRecord {
        PairRecord {
            length_difference_nm: p.length_difference_nm(),
            name: p.name,
            nets: p.nets,
            gap_nm: p.gap_nm,
            layers: p.layers,
            length_nm: p.length_nm,
            length_difference_max_nm: aethelgard_eda::route::PAIR_SKEW_NM,
            vias: p.vias,
            uncoupled_nm: p.uncoupled_nm,
            uncoupled_max_nm: aethelgard_eda::route::PAIR_BREAKOUT_NM,
        }
    }
}

/// A boot's peak tree RSS and its share of the reaper's bound
/// (`AET-WO-2302` D6), in thousandths, rounded down.
#[derive(Serialize)]
pub struct MemoryReport {
    pub peak_rss_kib: u64,
    pub bound_kib: u64,
    pub share_permille: u64,
}

impl MemoryReport {
    pub fn of(peak_rss_kib: u64, bound_kib: u64) -> Self {
        MemoryReport {
            peak_rss_kib,
            bound_kib,
            share_permille: peak_rss_kib.saturating_mul(1000) / bound_kib.max(1),
        }
    }
}

/// The residual every verdict of a board with a DShot output names
/// (`AET-WO-2302` D2, accepted): the judge clears `CEN` at the end of the
/// output's window.
pub const DSHOT_STOP_RESIDUAL: &str = "after its window a firmware reading its timer sees it stopped (CEN cleared by the judge): a twin artifact";

/// One judge's stop (`AET-WO-2302` D2): a DShot timer at its window's end,
/// its solved registers as the stop read them back, and `CR1` after it
/// cleared `CEN`. Kind 30 archives it.
#[derive(Serialize)]
pub struct StopReport {
    pub timer: String,
    pub anchor_ns: u64,
    pub at_ns: u64,
    pub read_back: Vec<RegisterReport>,
    pub cr1_after: Option<String>,
}

/// A droop's records, as `campaign` archives them (kinds 16 to 19).
#[derive(Serialize)]
pub struct DroopReport {
    pub rail: RailReport,
    pub crossings: Vec<CrossingReport>,
    pub resets: Vec<ResetReport>,
    pub observed: Vec<ObservedReport>,
}

/// `analog.rail_declared`: every value the rail was computed from.
#[derive(Serialize)]
pub struct RailReport {
    pub kind: &'static str,
    pub source: &'static str,
    pub domain: String,
    pub nominal_millivolts: u32,
    pub tolerance_permille: u32,
    pub floor_millivolts: u32,
    pub start_ns: u64,
    pub fall_ns: u64,
    pub hold_ns: u64,
    pub rise_ns: u64,
    pub source_milliohms: u32,
    pub load_milliamps: u32,
    pub decoupling_picofarads: u64,
    pub bor_level: u8,
    /// The rail's lowest value, as ngspice computed it.
    pub minimum_microvolts: u64,
    /// Where the thresholds come from (`AET-WO-2301` D4; Rev 10 since `AET-WO-2205`).
    pub thresholds: &'static str,
}

/// `analog.rail_crossing_computed`.
#[derive(Serialize)]
pub struct CrossingReport {
    pub kind: &'static str,
    pub source: &'static str,
    pub level: String,
    pub microvolts: u64,
    pub at_ps: u64,
    pub falling: bool,
}

/// `bridge.reset_rendered`: the model's rendering, never a measured reset.
#[derive(Serialize)]
pub struct ResetReport {
    pub kind: &'static str,
    pub source: &'static str,
    pub at_ns: u64,
    pub released: bool,
    pub cause: Vec<String>,
    pub rendering: &'static str,
}

/// `firmware.observed` at the recovery bound after a release.
#[derive(Serialize)]
pub struct ObservedReport {
    pub kind: &'static str,
    pub source: &'static str,
    pub at_ns: u64,
    pub boot_word: Word,
    pub status: Word,
    pub value: Word,
    /// What Renode logged for the first identity read's device from the
    /// release to this read: only a transfer the controller runs calls the
    /// model (`AET-WO-2301` adversarial F1, `AET-WO-2204`). `null` when the
    /// boot has no identity read, or the log could not be read whole. The
    /// lines kind 19 archives, and nothing else (`AET-WO-2205` §1.8).
    pub witness: Option<Vec<String>>,
}

/// The identity reads of the devices the twin has no model for
/// (`AET-WO-4241`): each named `identity_not_witnessable`, never judged
/// against the part, and the twin's own cost of them, a limit of the twin
/// (the architect's D3), never the board's.
#[derive(Serialize)]
pub struct NotWitnessableReport {
    pub count: usize,
    pub reads: Vec<NotWitnessableRead>,
    /// Every read's cost, summed; `null` when any was not measured.
    pub total_ns: Option<u64>,
    pub twin_limit: &'static str,
}

/// [`NotWitnessableReport::twin_limit`].
pub const NOT_WITNESSABLE_LIMIT: &str = "no model in the twin answers these reads, so each waits out the firmware's bounded wait (TXIS on I2C) that silicon, which answers, does not; the firmware makes them after every modelled step, so no modelled record's place moves, and the boot word and every instant after it are later in the twin than on silicon by about total_ns (measured from the status write before the first read: on a board with no modelled step, the brown-out word's, so it also counts the bring-up between)";

/// One unmodelled identity read: what it asked for, what its record holds
/// (information only) and its cost in the twin.
#[derive(Serialize)]
pub struct NotWitnessableRead {
    pub reference: String,
    pub part: String,
    /// `I2C1 0x48`, or `SPI1 PA0`: the bus and where the device sits on it.
    pub on: String,
    pub register: String,
    pub width: u8,
    /// The part table's value, which only the bench reads.
    pub table_value: String,
    pub status: Option<&'static str>,
    pub observed: Option<String>,
    pub cost_ns: Option<u64>,
}

/// A boot's inputs, its expectations and what Renode read (`AET-WO-2100`).
/// Every expected value comes from the specification's solved topology and
/// the CLI's boot contract, never from the firmware (`INV-07`).
#[derive(Serialize)]
pub struct Boot {
    pub firmware: FirmwareReport,
    pub specification_sha256: String,
    pub seed: u32,
    pub virtual_time_ns: u64,
    /// The constant that bounds the boot: `MAX_VIRTUAL_NS` on every board
    /// (`AET-WO-2302`); each witness's own window is in its entry.
    pub window: &'static str,
    /// `null` until Renode has read it, as is every `observed` below.
    pub mips: Option<u64>,
    pub instructions: Expected<u64>,
    /// Where the core was when the run ended, for a reader; not a witness.
    pub pc: Option<String>,
    pub boot_word: Word,
    /// The brown-out level the boot recorded (`AET-WO-4230` D10).
    pub brown_out: BrownOutReport,
    pub registers: Vec<RegisterReport>,
    pub unobservable: Vec<String>,
    /// One per step the judge expects (`AET-WO-2202`; one per step of a
    /// device's script since `AET-WO-4210`).
    pub i2c: Vec<I2cReport>,
    /// What Renode logged for each identity read's device (`AET-WO-2204` D1).
    pub i2c_witness: Vec<WitnessReport>,
    /// One per SPI identity read the judge expects, after the I2C ones in the
    /// records (`AET-WO-4200`).
    pub spi: Vec<SpiReport>,
    /// The unmodelled devices' identity reads (`AET-WO-4241`); absent from
    /// the JSON when the board has none, so such a board reports as before.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_witnessable: Option<NotWitnessableReport>,
    /// What Renode logged for each SPI read's device, and what its
    /// controller's multiplexer logged when a byte reached no single device.
    pub spi_witness: Vec<SpiWitnessReport>,
    /// `ICSR` after the run: `VECTACTIVE` [8:0] is 0 when the core is in
    /// thread mode (`AET-WO-2204` D3).
    pub icsr: Option<String>,
    /// One per motor output, in the specification's order (`AET-WO-4300a`):
    /// the static check for silicon and the twin's witness, apart. Absent
    /// from the JSON when the topology has none, so a board without motor
    /// outputs reports as before.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub motor: Vec<MotorReport>,
    /// One per serial port, in the specification's order (`AET-WO-4300b`):
    /// the silicon check and the twin's judgement, apart. Absent from the
    /// JSON when the topology has none, so a board without serial ports
    /// reports as before.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub serial: Vec<SerialReport>,
    /// The instant of the boot word's write in the boot the judge reads, as
    /// the CLI's hook observed it (`AET-WO-2302`): the serial frames go in
    /// one quantum after it. `null` until Renode has run, or when none was
    /// observed.
    pub boot_word_ns: Option<u64>,
    /// Each judge's stop (`AET-WO-2302` D2), absent from the JSON on a board
    /// without a DShot output.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stops: Vec<StopReport>,
    /// What the twin cannot show of this board, each named
    /// ([`DSHOT_STOP_RESIDUAL`] on every board with a DShot output); absent
    /// when none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub residuals: Vec<&'static str>,
    /// The firmware's executable segments, `[start, end)` at their physical
    /// address, that the judge placed `pc` against: kind 10 archives them
    /// (`AET-WO-2205` §1.5). Not part of the verdict's JSON.
    #[serde(skip)]
    pub executable: Vec<(u64, u64)>,
}

/// The boot record's word 1, `FLASH_OPTSR_CUR` as the firmware read it,
/// against the declared level (`AET-WO-4230` §1.6, D10): `BOR_LEV` (bits
/// 3:2, `mask`) must be `expected`, the specification's `level` in place;
/// and what the flash controller's access log showed that the bring-up never
/// does (`flash_access`: empty when nothing, `null` until read or when the
/// log could not be read whole). On silicon only the bench reads the level.
#[derive(Serialize)]
pub struct BrownOutReport {
    pub level: u8,
    pub address: String,
    pub mask: String,
    pub expected: String,
    pub observed: Option<String>,
    pub flash_access: Option<Vec<String>>,
}

/// One motor output's two judgements (`AET-WO-4300a`, `4300` D3), never
/// blended: `silicon`, the solved registers on the topology's clock with
/// silicon's `ARR + 1`; and `twin`, the observer's edges against the model's
/// conventions (`ARR` ticks at a fixed 250 MHz).
#[derive(Serialize)]
pub struct MotorReport {
    pub label: String,
    pub protocol: &'static str,
    pub timer: String,
    pub channel: u32,
    pub pin: String,
    pub psc: u32,
    pub arr: u32,
    pub ccr: u32,
    pub silicon: aethelgard_ir::motor::SiliconTiming,
    pub twin: TwinReport,
    /// What the verdict says of the pin's output speed (`AET-WO-4300a` Q3,
    /// ruled: owned by `AET-WO-4500`'s backlog).
    pub speed: &'static str,
}

/// The motor pin's speed is not judged: its load is off-board and no
/// specification declares it (`AET-WO-4300a` Q3, ruled).
pub const MOTOR_SPEED_NOT_JUDGED: &str = "motor pin speed not judged: off-board load undeclared";

/// One serial port's judgements (`AET-WO-4300b`, `4300` D8), never blended:
/// `silicon`, the solved `BRR` on the topology's clock against RM0433's
/// receiver tolerance; and `twin`, what Renode's USART left: the capture,
/// the pins' mux, and the USART's writes from Renode's access log.
#[derive(Serialize)]
pub struct SerialReport {
    pub label: String,
    pub role: &'static str,
    pub peripheral: String,
    pub tx: PinReport,
    pub rx: PinReport,
    pub framing: aethelgard_ir::Framing,
    pub brr: u32,
    pub actual_millibaud: u64,
    pub error_ppm: i64,
    pub silicon: aethelgard_ir::serial::SiliconFraming,
    /// [`aethelgard_ir::serial::TOLERANCE_SOURCE`].
    pub tolerance_source: &'static str,
    pub twin: SerialTwinReport,
    /// What the verdict says of the pins' output speed (as motors).
    pub speed: &'static str,
}

/// A serial pin and its solved alternate function.
#[derive(Serialize)]
pub struct PinReport {
    pub pin: String,
    pub alternate_function: u8,
}

/// The serial pin's speed is not judged: its load is off-board and no
/// specification declares it (as `MOTOR_SPEED_NOT_JUDGED`).
pub const SERIAL_SPEED_NOT_JUDGED: &str = "serial pin speed not judged: off-board load undeclared";

/// The twin's half of a serial port. `null` until Renode has run, as every
/// field after `tags_needed`.
#[derive(Serialize)]
pub struct SerialTwinReport {
    /// The USART's Renode name, `usart1`: the log source and the capture's.
    pub source: &'static str,
    /// The pattern's bytes, upper-case hexadecimal.
    pub pattern: String,
    /// The injected frame's bytes, upper-case hexadecimal.
    pub frame: String,
    /// The tags the framing needs set (`CR1.M0`) at `UE`, each seen only in
    /// the words Renode's access log shows written; the register image holds
    /// the stored fields.
    pub tags_needed: Vec<String>,
    /// Whole patterns before the frame.
    pub patterns: Option<usize>,
    /// Patterns among them cut short, each by a rendered reset.
    pub cut: Option<usize>,
    /// The frame's bytes that came back after them.
    pub echoed: Option<usize>,
    /// The first frame byte's instant: one quantum after the boot word for
    /// the first port, after the port before's last echoed byte for each
    /// later one (`AET-WO-2302` D4); and the frame's bytes that went in.
    /// `null` when none went in.
    pub injected_ns: Option<u64>,
    pub injected_bytes: Option<usize>,
    /// The capture's length, and whether anything in it was not a pattern
    /// or the frame.
    pub capture_bytes: Option<usize>,
    pub stray: Option<bool>,
    /// `witnessed`, `missing`, `partial`, `not_counted` or `not_judged`.
    pub echo: Option<&'static str>,
    /// The RX pin at the injection, the TX pin at the end, and the TX pin at
    /// a boot's first `TDR` write (the first boot's not routed, else the
    /// first that left the AF after it was solved, else the last's): `PA10 MODER 0b10,
    /// AF7`.
    pub rx_at_injection: Option<String>,
    pub tx_at_end: Option<String>,
    pub tx_at_send: Option<String>,
    /// The USART's writes in the last boot (Renode's access log), and what
    /// they lacked at `UE` or did beyond the bring-up; `null` when the log
    /// could not be read whole.
    pub usart_writes: Option<usize>,
    pub tags_missing: Option<Vec<String>>,
    pub tags_unexpected: Option<Vec<String>>,
    /// What the campaign archives (kind 29) and the JSON says in words: the
    /// frame's bytes, and each pin's `MODER` and `AFSEL` fields.
    #[serde(skip)]
    pub frame_bytes: [u8; aethelgard_twin::serial::FRAME_LEN],
    #[serde(skip)]
    pub rx_fields: Option<(u8, u8)>,
    #[serde(skip)]
    pub tx_fields: Option<(u8, u8)>,
    #[serde(skip)]
    pub tx_send_fields: Option<(u8, u8)>,
}

impl SerialReport {
    pub fn new(s: &crate::renode::Serial, w: Option<&crate::renode::SerialWitness>) -> Self {
        use crate::renode::hex_bytes;
        let f = &s.silicon;
        let c = w.and_then(|w| w.capture.as_ref());
        let tags = w.and_then(|w| w.tags.as_ref());
        SerialReport {
            label: s.port.label.clone(),
            role: s.role.name(),
            peripheral: s.port.peripheral.clone(),
            tx: PinReport {
                pin: s.port.tx.clone(),
                alternate_function: s.port.tx_af,
            },
            rx: PinReport {
                pin: s.port.rx.clone(),
                alternate_function: s.port.rx_af,
            },
            framing: s.port.framing.clone(),
            brr: f.brr,
            actual_millibaud: f.actual_millibaud,
            error_ppm: f.error_ppm,
            silicon: f.clone(),
            tolerance_source: aethelgard_ir::serial::TOLERANCE_SOURCE,
            twin: SerialTwinReport {
                source: s.port.renode,
                pattern: hex_bytes(&s.pattern),
                frame: hex_bytes(&s.frame),
                tags_needed: s
                    .port
                    .tags
                    .iter()
                    .map(|t| format!("{}.{}", t.register, t.field))
                    .collect(),
                patterns: c.map(|c| c.patterns),
                cut: c.map(|c| c.cut),
                echoed: c.map(|c| c.echoed),
                injected_ns: w.and_then(|w| w.injected_ns),
                injected_bytes: w.map(|w| w.injected),
                capture_bytes: c.map(|c| c.len),
                stray: c.map(|c| c.stray),
                echo: w.map(|w| w.echo),
                rx_at_injection: w
                    .and_then(|w| w.rx_at_injection)
                    .map(|(m, a)| s.rx.describe(m, a)),
                tx_at_end: w
                    .and_then(|w| w.tx_at_end)
                    .map(|(m, a)| s.tx.describe(m, a)),
                tx_at_send: w
                    .and_then(|w| w.tx_at_send)
                    .map(|(m, a)| s.tx.describe(m, a)),
                usart_writes: tags.map(|t| t.writes),
                tags_missing: tags.map(|t| t.missing.clone()),
                tags_unexpected: tags.map(|t| t.unexpected.clone()),
                frame_bytes: s.frame,
                rx_fields: w
                    .and_then(|w| w.rx_at_injection)
                    .map(|(m, a)| s.rx.fields(m, a)),
                tx_fields: w.and_then(|w| w.tx_at_end).map(|(m, a)| s.tx.fields(m, a)),
                tx_send_fields: w.and_then(|w| w.tx_at_send).map(|(m, a)| s.tx.fields(m, a)),
            },
            speed: SERIAL_SPEED_NOT_JUDGED,
        }
    }
}

#[derive(Serialize)]
pub struct TwinReport {
    /// The observer's log source, `gpioPortA.pa8obs`.
    pub observer: String,
    pub period_ns: u64,
    pub high_ns: u64,
    pub grain_ns: u64,
    pub quantum_ns: u64,
    /// The window's anchor (its timer's first `CEN` rise in the boot the
    /// judge reads) and end: `DSHOT_WINDOW_NS` after it for DShot, the
    /// boot's end for PWM (`AET-WO-2302`). `null` until Renode has run, as
    /// every field below; the anchor `null` too when none was observed.
    pub anchor_ns: Option<u64>,
    /// Edges before the anchor and after the window's end, each named, and
    /// the timer's `CEN` rises in the boot (more than one named).
    pub edges_before: Option<u64>,
    pub edges_after: Option<u64>,
    pub cen_rises: Option<u64>,
    pub status: Option<&'static str>,
    pub rises: Option<u64>,
    pub falls: Option<u64>,
    pub first_ns: Option<u64>,
    pub last_ns: Option<u64>,
    pub end_ns: Option<u64>,
    pub span_ns: Option<u64>,
    pub off_grid: Option<u64>,
    pub batched: Option<u64>,
}

impl MotorReport {
    pub fn new(m: &crate::renode::Motor, w: Option<&crate::renode::MotorWitness>) -> Self {
        let o = &m.observer;
        MotorReport {
            label: o.label.clone(),
            protocol: o.protocol.name(),
            timer: format!("TIM{}", o.timer),
            channel: o.channel,
            pin: o.pin.clone(),
            psc: m.psc,
            arr: m.arr,
            ccr: m.ccr,
            silicon: m.silicon.clone(),
            speed: MOTOR_SPEED_NOT_JUDGED,
            twin: TwinReport {
                observer: o.source.clone(),
                period_ns: o.period_ns,
                high_ns: o.high_ns,
                grain_ns: crate::renode::STAMP_GRAIN_NS,
                quantum_ns: aethelgard_twin::boot::QUANTUM_NS,
                anchor_ns: w.and_then(|w| w.anchor_ns),
                edges_before: w.map(|w| w.edges_before),
                edges_after: w.map(|w| w.edges_after),
                cen_rises: w.map(|w| w.cen_rises),
                status: w.map(|w| w.status),
                rises: w.map(|w| w.rises),
                falls: w.map(|w| w.falls),
                first_ns: w.and_then(|w| w.first_ns),
                last_ns: w.and_then(|w| w.last_ns),
                end_ns: w.map(|w| w.end_ns),
                span_ns: w.and_then(|w| w.span_ns),
                off_grid: w.map(|w| w.off_grid),
                batched: w.map(|w| w.batched),
            },
        }
    }
}

/// One identity read's witness: the device model's whole log lines the judge
/// expects, and every line Renode logged for the device with a witness's
/// shape (`AET-WO-2204` D1).
#[derive(Serialize)]
pub struct WitnessReport {
    pub peripheral: String,
    pub reference: String,
    /// `identity`, `configure` or `data` (`AET-WO-4210`).
    pub step: &'static str,
    /// The model's path in Renode, `i2c1.u2`.
    pub device: String,
    /// Empty when the model has no known witness format (a named failure).
    pub expected: Vec<String>,
    /// `null` until Renode has run, or when the log could not be read whole.
    pub observed: Option<Vec<String>>,
    /// The addresses the device's controller found no device at during this
    /// step (`AET-WO-3305`), in order: attribution's evidence, never part of
    /// the witness.
    pub absent: Vec<String>,
}

/// One SPI identity read: what the parts table says, and what the
/// firmware's record held (`AET-WO-4200`).
#[derive(Serialize)]
pub struct SpiReport {
    pub reference: String,
    pub peripheral: String,
    /// The chip select's pin, `PA0`.
    pub select: String,
    /// The multiplexer address the chip select drives, `0x00`.
    pub address: String,
    /// `identity`, `configure` or `data` (`AET-WO-4210`).
    pub step: &'static str,
    /// The bytes written first, space-separated: `0x9f`.
    pub command: String,
    /// The bytes read back; 0 for a configuration.
    pub width: u8,
    pub expected: String,
    /// The bits `expected` fixes (`AET-WO-4210` F6).
    pub mask: String,
    pub observed: Option<String>,
    /// `ok`, `timeout_rxp` or `invalid`; `null` until Renode has read the record.
    pub status: Option<&'static str>,
}

/// One SPI read's witness, and the multiplexer's lines for its controller.
#[derive(Serialize)]
pub struct SpiWitnessReport {
    pub peripheral: String,
    pub reference: String,
    /// `identity`, `configure` or `data` (`AET-WO-4210`).
    pub step: &'static str,
    /// The model's path in Renode, `spi1.spi1mux.u4`.
    pub device: String,
    pub expected: Vec<String>,
    pub observed: Option<Vec<String>>,
    /// Each line the controller's multiplexer logged when a byte reached no
    /// device, several, or an address with none, inside this step's
    /// chip-select window, or inside none for the identity step
    /// (`AET-WO-4210` D5): attribution's evidence, never part of the witness.
    pub unrouted: Vec<String>,
}

impl WitnessReport {
    /// `check`'s witness as the judge expects it, with what Renode logged.
    pub fn of(
        check: &aethelgard_twin::boot::StepCheck,
        observed: Option<Vec<String>>,
        absent: Vec<String>,
    ) -> Self {
        WitnessReport {
            peripheral: check.peripheral.clone(),
            reference: check.reference.clone(),
            step: check.label,
            device: check.device.clone(),
            expected: check.witness.clone().unwrap_or_default(),
            observed,
            absent,
        }
    }
}

/// One identity read: what the part table says, and what the firmware's record held.
#[derive(Serialize)]
pub struct I2cReport {
    pub reference: String,
    pub address: String,
    /// `identity`, `configure` or `data` (`AET-WO-4210`).
    pub step: &'static str,
    /// The bytes written first, space-separated: `0xd0`, or `0xef 0xc8`.
    pub register: String,
    /// The bytes read back; 0 for a configuration.
    pub width: u8,
    pub expected: String,
    /// The bits `expected` fixes (`AET-WO-4210` F6).
    pub mask: String,
    pub observed: Option<String>,
    /// `ok`, `timeout_txis`, `timeout_tc`, `timeout_rxne`, `timeout_stopf` or `invalid`;
    /// `null` until Renode has read the record.
    pub status: Option<&'static str>,
}

#[derive(Serialize, Clone)]
pub struct FirmwareReport {
    pub path: String,
    /// The file's bytes as copied.
    pub sha256: String,
    /// The `PT_LOAD` file bytes in physical-address order: what Renode loads.
    pub image_sha256: String,
}

#[derive(Serialize)]
pub struct Expected<T> {
    pub expected: T,
    pub observed: Option<T>,
}

#[derive(Serialize)]
pub struct Word {
    pub address: String,
    pub expected: String,
    pub observed: Option<String>,
}

#[derive(Serialize)]
pub struct RegisterReport {
    pub name: String,
    pub address: String,
    pub mask: String,
    pub expected: String,
    pub observed: Option<String>,
}

/// `0x2100b007`.
pub fn word(v: u32) -> String {
    format!("{v:#010x}")
}

#[derive(Serialize)]
pub struct InputReport {
    pub path: String,
    pub sha256: String,
}

#[derive(Serialize)]
pub struct ToolReport {
    pub name: &'static str,
    pub version: String,
    /// The build the tool reported, on `simulate`'s verdicts: Renode's
    /// `build:` line (`AET-WO-4400g` §1.5), absent where none was named.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
    pub exit_code: i32,
}

#[derive(Serialize)]
pub struct Finding {
    pub kind: String,
    pub severity: String,
    pub message: String,
    #[serde(rename = "where")]
    pub location: Vec<Where>,
}

/// The MCU's support network on a judged schematic (`AET-WO-4300g`).
#[derive(Serialize)]
pub struct SupportRecord {
    /// The board the distances were measured on.
    pub board: InputReport,
    pub mcu: String,
    pub roles: Vec<SupportRole>,
    pub straps: Vec<String>,
    /// The connector that programs the part, and whether NRST reaches it.
    pub programming: Option<Programming>,
    pub supply_source: Option<String>,
    pub conditions: Vec<String>,
}

/// One capacitor role at one pin.
#[derive(Serialize)]
pub struct SupportRole {
    pub role: &'static str,
    pub pin: String,
    pub part: Option<String>,
    pub picofarads: Option<i64>,
    pub gap_um: Option<i64>,
    pub bound_um: Option<i64>,
    pub rule: String,
    pub verdict: &'static str,
}

#[derive(Serialize)]
pub struct Programming {
    pub connector: String,
    pub nrst: bool,
}

impl From<(aethelgard_eda::support::Record, InputReport)> for SupportRecord {
    fn from((r, board): (aethelgard_eda::support::Record, InputReport)) -> Self {
        Self {
            board,
            mcu: r.mcu,
            roles: r
                .roles
                .into_iter()
                .map(|x| SupportRole {
                    role: x.role,
                    pin: x.pin,
                    part: x.part,
                    picofarads: x.picofarads,
                    gap_um: x.gap_um,
                    bound_um: x.bound_um,
                    rule: x.rule,
                    verdict: x.verdict,
                })
                .collect(),
            straps: r.straps,
            programming: r.programming.map(|p| Programming {
                connector: p.connector,
                nrst: p.nrst,
            }),
            supply_source: r.supply_source,
            conditions: r.conditions,
        }
    }
}

/// The power tree on a judged board (`AET-WO-4300d2`).
#[derive(Serialize)]
pub struct PowerRecord {
    pub loops: Vec<LoopRecord>,
    pub classes: Vec<ClassRecord>,
    pub conditions: Vec<String>,
}

/// One loop rule as the board measures it (`aethelgard_eda::loops`).
#[derive(Serialize)]
pub struct LoopRecord {
    pub part: String,
    pub ic: String,
    /// Each pad with its IC pin and the copper-edge gap, in micrometres
    /// rounded up.
    pub pads: Vec<(String, String, i64)>,
    pub bound_um: i64,
    pub rule: &'static str,
    pub verdict: &'static str,
}

impl From<aethelgard_eda::loops::LoopRecord> for LoopRecord {
    fn from(l: aethelgard_eda::loops::LoopRecord) -> Self {
        Self {
            part: l.part,
            ic: l.ic,
            pads: l
                .pads
                .into_iter()
                .map(|g| (g.pad, g.pin, g.gap_um))
                .collect(),
            bound_um: l.bound_um,
            rule: l.rule,
            verdict: l.verdict,
        }
    }
}

/// A declared net class as the board is laid with it: its width, its via
/// (diameter and drill) when it declares one, and its nets by board name.
#[derive(Serialize)]
pub struct ClassRecord {
    pub name: String,
    pub width_um: i64,
    pub via_um: Option<(i64, i64)>,
    pub nets: Vec<String>,
}

#[derive(Serialize)]
pub struct Where {
    pub description: String,
    pub x: f64,
    pub y: f64,
    pub uuid: String,
}

#[derive(Serialize)]
pub struct Machine {
    pub cpus: Vec<String>,
    pub peripherals: Vec<Peripheral>,
}

#[derive(Serialize)]
pub struct Peripheral {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}

impl Finding {
    /// A finding with no location: the Kani and Renode kinds.
    pub fn plain(kind: &str, message: String) -> Self {
        Self {
            kind: kind.to_owned(),
            severity: "error".to_owned(),
            message,
            location: Vec::new(),
        }
    }

    /// A finding of severity `info`, which does not fail the verdict. Used for
    /// what the twin could not check, so it is reported and never passed
    /// silently (`AET-WO-2202` §1.2).
    pub fn info(kind: &str, message: String) -> Self {
        Self {
            severity: "info".to_owned(),
            ..Self::plain(kind, message)
        }
    }
}

impl Verdict {
    /// A verdict is pass exactly when it has no finding above severity `info`;
    /// the caller decides that only after both witnesses agreed.
    pub fn new(
        check: &'static str,
        input: &Input,
        tool: &Tool,
        version: String,
        code: i32,
        sandbox: SandboxReport,
        findings: Vec<Finding>,
    ) -> Self {
        // Only the boot path makes an `info` finding (KiCad's severities are
        // `error` and `warning`, which `kicad` refuses to widen).
        let pass = findings.iter().all(|f| f.severity == "info");
        Self {
            ok: pass,
            verdict: if pass { "pass" } else { "fail" },
            check,
            lifecycle: Vec::new(),
            input: InputReport {
                path: input.original.display().to_string(),
                sha256: input.sha256.clone(),
            },
            tool: ToolReport {
                name: tool.name,
                version,
                build: None,
                exit_code: code,
            },
            sandbox,
            included_severities: None,
            rules: None,
            specification_sha256: None,
            schematic: None,
            pairs: None,
            power: None,
            support: None,
            harness: None,
            harness_provenance: None,
            cover_count: None,
            elf: None,
            machine: None,
            boot: None,
            foreign_boot: None,
            analog: None,
            faults: None,
            droop: None,
            memory: None,
            findings,
            tools: Vec::new(),
        }
    }
}

/// One I2C line's rise time as the bridge computed it from the declared
/// values (`AET-WO-2200`, `RUL-19` (1)): the evidence. Nothing in it comes
/// from the firmware or from Renode.
#[derive(Serialize)]
pub struct AnalogRecord {
    /// `analog.rise_time_computed`, or `analog.line_stuck_high` for a line
    /// at or above 30 % when its driver released it.
    pub kind: &'static str,
    pub source: &'static str,
    pub bus: String,
    pub net: String,
    /// False when the line had not reached 70 % when the window ended, or
    /// never fell (ruled F7 (b) on PR #70).
    pub complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rise_30_70_ns: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rise_30_70_ns_lower_bound: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volts_at_release: Option<f64>,
    pub ceiling_ns: u32,
    pub exceeds: bool,
    pub method: &'static str,
    pub ngspice: &'static str,
    /// The deck's window: one byte at this bit rate, 400 kHz for every bus
    /// (`AET-WO-2205` §1.3, review A10.4).
    pub deck_hz: u32,
    /// The bus's declared speed, beside it.
    pub declared_hz: u32,
}

/// The decision a measurement past its ceiling forces on its bus, and what the
/// firmware is given to see of it, labelled as the model's rendering.
#[derive(Serialize)]
pub struct FaultRecord {
    pub kind: &'static str,
    /// `aethelgard` for a measured fault; `scenario <name>` for a line a
    /// campaign scenario holds low (`AET-WO-2300`).
    pub source: String,
    pub bus: String,
    pub fault: &'static str,
    /// Each distinct cause among the bus's lines.
    pub from: Vec<&'static str>,
    /// The bus's lines past their ceiling.
    pub nets: Vec<String>,
    pub firmware_sees: &'static str,
}
