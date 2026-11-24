//! A campaign run's records (`AET-WO-2300`): event kinds 5 to 37, one per
//! record `aethelgard campaign` archives (kind 15 since `AET-WO-3100`; 16 to
//! 20, a declared supply droop, since `AET-WO-2301`; 31, a declared battery
//! sag, since `AET-WO-2304b`; 32, a droop's recovery bound, since
//! `AET-WO-2305`; 33 to 37, a pin device's GPIO and ADC inputs and a motor
//! output held at a static level, since `AET-WO-1211c`). Kinds 38 to 40 are a
//! foreign run's, which `simulate --foreign --input --archive` archives
//! (`AET-WO-4400i`): its declared input, its stimuli and what the firmware
//! transmitted. Every field is an integer, a byte
//! string, a text string or an array of them; nothing is a float or a map
//! (`RUL-06`). An absent value is an empty array, a present one an array of
//! one element.

use crate::cbor::Value;
use crate::event::{name_ok, text_ok};

/// The first campaign kind; [`Record::kind`] is this plus the variant's index.
pub const FIRST_KIND: u64 = 5;
/// The last kind (40, a foreign run's transmitted bytes, since
/// `AET-WO-4400i`).
pub const LAST_KIND: u64 = 40;
/// The most bytes kind 38 holds of an input file, and kind 40 of a capture
/// (`AET-WO-4400i`): far past a 5 s window's frames at 115200 Bd.
pub const MAX_FOREIGN_BYTES: usize = 1 << 20;
/// The most stimuli kind 39 holds.
pub const MAX_STIMULI: usize = 100_000;

/// A pin's level, as the `pin_held` override and kinds 33 and 34 name it
/// (`AET-WO-1211c`). Chaos-owned: this crate reads no IR type (`RUL-16`).
/// A scenario file names it `low` or `high`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Low,
    High,
}

/// The highest voltage an `analog_value` override may set, in mV
/// (`AET-WO-1211c`): the twin's ADC reference. Renode 1.17.0's
/// `stm32h7.repl` gives `adcM1S2` and `adc3` `referenceVoltage: 3.3`
/// (`AET-WO-1211` §4).
pub const ANALOG_MAX_MILLIVOLTS: u32 = 3300;

/// The code the twin's ADC reads for `microvolts` at `bits` of resolution
/// (`AET-WO-1211c`): round-half-up of `microvolts` × (2^`bits` − 1) /
/// [`ANALOG_MAX_MILLIVOLTS`]·1000, in integers (no floats, `RUL-06`). The
/// conversion Renode's model applies (`STM32_ADC_Common.cs` L451-460,
/// `AET-WO-1211` §4: 1.65 V reads 0x8000 and 0.825 V 0x4000 at 16 bits;
/// measured, 1.5 V reads 0x745 = 1861 at 12 bits, `spike/1211c/RESULTS.md`
/// P4e). Only 12 bits is measured; `bits` above 32 are computed as 32 and a
/// code past `u32` saturates, so no input panics.
pub fn adc_code(microvolts: u64, bits: u8) -> u32 {
    let full_scale = u128::from(ANALOG_MAX_MILLIVOLTS) * 1000;
    let top = (1u128 << u32::from(bits.min(32))) - 1;
    let code = (u128::from(microvolts) * top * 2 + full_scale) / (2 * full_scale);
    u32::try_from(code).unwrap_or(u32::MAX)
}

/// A PWM output's declared idle as kind 24 carries it (`AET-WO-1211c`, from
/// `pwm.idle`, `AET-WO-1211a`): a static level, or a pulse in µs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotorIdle {
    Low,
    High,
    PulseUs(u32),
}

/// The protocols a motor output may declare (`AET-WO-4300a`), as kind 24
/// names them.
pub const MOTOR_PROTOCOLS: [&str; 3] = ["pwm", "dshot300", "dshot600"];

/// The time Renode 1.17.0 runs `ticks` cycles of a timer counting at
/// `kernel_hz`, in whole ns: ⌈`ticks` × 10⁹ / `kernel_hz`⌉. Virtual time has
/// 1 ns resolution, and at each limit Renode drops the residue and rounds
/// the time to the next limit up (`BaseClockSource.cs` L315–L325;
/// `AET-WO-4400f` F3), so a DShot600 bit of 106 ticks at 64 MHz runs 1,657
/// ns, not 1,656.25. A model convention, as `ARR` ticks against silicon's
/// `ARR + 1`: the silicon check (kind 25) judges the exact period, and this
/// rounding never reaches it. `None` for a zero clock or a period past
/// `u64` ns.
///
/// The one definition (`AET-WO-4400g` §1.3): the twin predicts its
/// observers with it and replay checks kind 26 against it, since replay
/// reads an archive without the twin.
pub fn realized_ns(ticks: u64, kernel_hz: u64) -> Option<u64> {
    if kernel_hz == 0 {
        return None;
    }
    let ns = (u128::from(ticks) * 1_000_000_000).div_ceil(u128::from(kernel_hz));
    u64::try_from(ns).ok()
}

/// What kind 26's `status` may say (`AET-WO-4300a`): the observer's edges
/// were the model's sequence with an edge off the grid; were, on the grid
/// alone; showed fewer than two rises; could not be read whole; did not fit
/// the sequence; shared stamps; stopped more than a period before the
/// window's end; were logged with no anchor observed (`AET-WO-2302`); or, an
/// output whose idle is a static level, were judged by its kind 37 instead
/// (`level`, `AET-WO-1211c`).
pub const MOTOR_STATUSES: [&str; 9] = [
    "witnessed",
    "static_only",
    "unwitnessed",
    "unreadable",
    "mismatch",
    "batched",
    "stopped",
    "unanchored",
    "level",
];

/// What kind 34's `status` may say (`AET-WO-1211c`): the firmware's GPIO
/// access was what the declaration says; was not; was never seen; or the
/// access log could not be read whole.
pub const GPIO_STATUSES: [&str; 4] = ["witnessed", "mismatch", "unwitnessed", "unreadable"];

/// What kind 36's `status` may say (`AET-WO-1211c`): the firmware read the
/// code the declared value gives; read another; or read the ADC's data
/// register not at all.
pub const ADC_STATUSES: [&str; 3] = ["witnessed", "mismatch", "unwitnessed"];

/// What kind 37's `status` may say (`AET-WO-1211c`): a motor output held at
/// a static level stayed at it after its timer settled; did not; had no
/// timer anchor to settle from; or its observer could not be read.
pub const LEVEL_STATUSES: [&str; 4] = ["witnessed", "mismatch", "unanchored", "unreadable"];

/// The roles a serial port may declare (`AET-WO-4300b`), as kind 27 names
/// them.
pub const SERIAL_ROLES: [&str; 5] = ["console", "mavlink", "gps", "rc_crsf", "rc_sbus"];

/// The parities a serial port's framing may declare, as kind 27 names them.
pub const SERIAL_PARITIES: [&str; 3] = ["none", "even", "odd"];

/// The framing fields Renode's USART stores nowhere (`AET-WO-4300b`), seen
/// only in the words its access log shows written, in the order kind 28
/// lists the ones a framing needs.
pub const SERIAL_TAGS: [&str; 4] = ["CR1.M0", "CR1.M1", "CR2.RXINV", "CR2.TXINV"];

/// What kind 29's `echo` may say (`AET-WO-4300b`): the frame came back
/// whole with the RX pin on its AF from the injection on; did not come back;
/// came back in part; came back with the RX pin off its AF at the injection
/// or changed after it was solved (adversarial pass 4, ADV4-4); or was not judged,
/// because the boot word was never set.
pub const SERIAL_ECHO_STATUSES: [&str; 5] = [
    "witnessed",
    "missing",
    "partial",
    "not_counted",
    "not_judged",
];

/// A finding's severities in a campaign verdict (kind 14), closed
/// (`AET-WO-3201` B3): the judge writes `error` or `info`, and KiCad's
/// `warning` never reaches a campaign. Checked exactly, so another case or
/// whitespace is refused, on write and on read; the bytes are unchanged.
pub const SEVERITIES: [&str; 2] = ["error", "info"];
/// The injected frame's length, and the bytes each port's pattern adds to
/// its label (`00 FE 7F 80 FF`).
// belt-and-suspenders: `aethelgard_twin::serial::{FRAME_LEN, PATTERN_TAIL}`
// state these; replay checks an archive without the twin, so it states them
// again.
pub const SERIAL_FRAME_LEN: usize = 8;
pub const SERIAL_TAIL_LEN: usize = 5;

/// The most unexpected writes kind 28 carries: the CLI cuts a longer list,
/// which still fails the port's tags, so the event fits the bound.
pub const MAX_SERIAL_UNEXPECTED: usize = 16;

/// The longest text of one of them, in bytes of printable ASCII: the access
/// log's judge writes at most 80 (the interval rule's `PK15 MODER/AFR not
/// rebuilt, OTYPER/OSPEEDR/PUPDR not rebuilt after it was solved`), past the
/// 64 the other names keep (`AET-WO-4300b` adversarial pass 4, ADV4-4: at
/// 64, a run in which the rule fired could not be archived).
pub const MAX_SERIAL_UNEXPECTED_BYTES: usize = 128;

/// A serial port's framing as kind 27 carries it, from the specification
/// through the solved topology (`AET-WO-4300b` §1.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerialFraming {
    pub baud: u32,
    /// Data bits, not counting the parity bit.
    pub data_bits: u8,
    /// One of [`SERIAL_PARITIES`].
    pub parity: String,
    pub stop_bits: u8,
    pub inverted: bool,
}

impl SerialFraming {
    /// The word the receiver samples: data bits and a parity bit if any.
    pub fn word_bits(&self) -> u8 {
        self.data_bits
            .saturating_add(u8::from(self.parity != "none"))
    }

    /// The tags of [`SERIAL_TAGS`] this framing needs set: `M0` for a 9-bit
    /// word, `M1` for a 7-bit one (RM0433 Rev 9 §48.8.1), both inversions
    /// when inverted (§48.8.2).
    // belt-and-suspenders: `aethelgard_twin::serial::needed_tags` is the
    // judge's; replay never depends on the twin, so it states the rule again.
    pub fn tags(&self) -> Vec<String> {
        let word = self.word_bits();
        [word == 9, word == 7, self.inverted, self.inverted]
            .iter()
            .zip(SERIAL_TAGS)
            .filter(|(needed, _)| **needed)
            .map(|(_, t)| t.to_owned())
            .collect()
    }
}

/// What one port's capture held, as kind 29 carries it: the whole patterns,
/// the ones among them cut short by a reset, the frame's bytes after them,
/// whether anything else was there, and the file's length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SerialCapture {
    pub patterns: u64,
    pub cut: u64,
    pub echoed: u64,
    pub stray: bool,
    pub len: u64,
}

/// What a port's writes showed, rebuilt from Renode's access log, as kind 28
/// carries it (`AET-WO-4300b` adversarial F1–F3): its USART's writes in the
/// last boot, each needed tag not set in its word at `UE`, and everything
/// else the writes did that the bring-up does not (at most
/// [`MAX_SERIAL_UNEXPECTED`], each at most [`MAX_SERIAL_UNEXPECTED_BYTES`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SerialTags {
    pub writes: u64,
    pub missing: Vec<String>,
    pub unexpected: Vec<String>,
}

impl SerialTags {
    pub fn holds(&self) -> bool {
        self.missing.is_empty() && self.unexpected.is_empty()
    }
}

/// The first [`SERIAL_FRAME_LEN`] bytes of
/// `sha256("aethelgard-serial-frame" ‖ firmware ‖ label)`: the frame the
/// judge injects into the port labelled `label` of the firmware whose file's
/// sha256 is `firmware` (`AET-WO-4300b`).
// belt-and-suspenders: `aethelgard_twin::serial::frame` is the judge's;
// replay checks an archive without the twin (`aethelgard-twin` depends on
// `aethelgard-ir`, `RUL-16`), so it states the formula again, and the CLI's
// tests hold the two equal.
pub fn serial_frame(firmware: &[u8; 32], label: &str) -> [u8; SERIAL_FRAME_LEN] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"aethelgard-serial-frame");
    h.update(firmware);
    h.update(label.as_bytes());
    let digest = h.finalize();
    let mut out = [0u8; SERIAL_FRAME_LEN];
    out.copy_from_slice(&digest[..SERIAL_FRAME_LEN]);
    out
}

/// One override a scenario applies to its specification (`AET-WO-2300` §1.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Override {
    /// `[0, bus, ohms]`.
    PullUpOhms { bus: String, ohms: u32 },
    /// `[1, net, femtofarads]`.
    MaxCapacitance { net: String, femtofarads: u64 },
    /// `[2, bus, line]`, `line` 0 for SDA and 1 for SCL.
    LineHeldLow { bus: String, line: Line },
    /// `[3, domain, floor_mv, from_boot_word_ns, fall_ns, hold_ns, rise_ns,
    /// source_milliohms, load_milliamps]` (`AET-WO-2301`; trace 19,
    /// `AET-WO-2310`: the instant signed from the undisturbed boot word, and
    /// no BOR level, which is the specification's).
    Droop(Droop),
    /// `[4, reference]`: an SPI device left absent behind its chip select,
    /// which stays wired (`AET-WO-4200` D4).
    SpiDeviceAbsent { reference: String },
    /// `[5, port]`: a serial port's RX line open on the board (`AET-WO-2302`
    /// D5, ruled (ii)): the composition injects nothing into that port, and
    /// the port after it in the sequential chain goes in after the port
    /// before it instead.
    SerialRxOpen { port: String },
    /// `[6, series_milliohms, from_boot_word_ns]`: the battery's path R steps
    /// from the specification's healthy R to this at its instant, and holds
    /// to the run's end (`AET-WO-2304b` §1.6). Scenario version 4; its
    /// instant signed from the undisturbed boot word since scenario 6
    /// (`AET-WO-2310`).
    BatterySag(BatterySag),
    /// `[7, domain]`: the named rail held below its band for the whole run,
    /// rendered as each declared consumer's absence through the
    /// `serial_rx_open` and `spi_device_absent` overrides the scenario lists
    /// beside it (`AET-WO-2304b` §1.7, K1). A modelling assumption, named in
    /// every verdict that uses it. Scenario version 4.
    RailOutOfBand { domain: String },
    /// `[8, net, level]`, `level` 0 for low and 1 for high: a pin device's
    /// GPIO input held at that level for the whole run, in place of its
    /// declared pull's (`AET-WO-1211c`). Scenario version 5.
    PinHeld { net: String, level: Level },
    /// `[9, net, millivolts]`: a pin device's ADC input held at that voltage,
    /// from 0 to [`ANALOG_MAX_MILLIVOLTS`] (`AET-WO-1211c`). Scenario
    /// version 5.
    AnalogValue { net: String, millivolts: u32 },
}

/// A declared battery sag (`AET-WO-2304b` §1.6): the whole path R under the
/// fault, mΩ, and when the path steps to it from the healthy R, signed ns
/// from the undisturbed boot word (`AET-WO-2310`; [`resolve_instant`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatterySag {
    pub series_milliohms: u32,
    pub from_boot_word_ns: i64,
}

/// The bound of a planted instant's offset from the boot word, either way:
/// a run's length (`AET-WO-2310`).
pub const MAX_PLANT_OFFSET_NS: i64 = crate::campaign::TWIN_MAX_VIRTUAL_NS as i64;

/// A planted event's offset as a scenario may state it (`AET-WO-2310`):
/// within [`MAX_PLANT_OFFSET_NS`] either way of the boot word and a whole
/// number of 10 ns instructions, as the boot word is.
pub fn plant_offset_ok(v: i64) -> bool {
    (-MAX_PLANT_OFFSET_NS..=MAX_PLANT_OFFSET_NS).contains(&v) && v % 10 == 0
}

/// A planted event's instant from reset: the undisturbed boot word's plus
/// the signed offset (`AET-WO-2310`, ruled Q-A (a)). `None` when it lies
/// before reset (at or before 0) or does not fit; the caller refuses either
/// by name.
pub fn resolve_instant(boot_word_ns: u64, from_boot_word_ns: i64) -> Option<u64> {
    let at = i128::from(boot_word_ns) + i128::from(from_boot_word_ns);
    u64::try_from(at).ok().filter(|&at| at > 0)
}

/// One battery-fed regulator as kind 31 carries it: a constant-power sink
/// and its two floors (`AET-WO-2304b` §1.5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SagSink {
    /// The regulator's reference.
    pub name: String,
    pub input_milliwatts: u64,
    /// Its stated minimum operating input, mV (F5).
    pub shutdown_millivolts: u64,
    /// The input from which its regulation band is stated, mV (D-B9).
    pub regulation_millivolts: u64,
}

impl SagSink {
    /// The higher of its two floors, µV.
    pub fn floor_microvolts(&self) -> u64 {
        self.shutdown_millivolts
            .max(self.regulation_millivolts)
            .saturating_mul(1000)
    }
}

/// A declared sag of a voltage domain's source (`AET-WO-2301` P2, ruled D2
/// and D7): from the domain's nominal down to `floor_millivolts` and back,
/// through `source_milliohms` into the emitter's decoupling, with a constant
/// `load_milliamps`, starting `from_boot_word_ns` from the undisturbed boot
/// word, signed (`AET-WO-2310`). The BOR level is the specification's
/// `mcu.bor_level`, never the scenario's ([`RailDroop`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Droop {
    pub domain: String,
    pub floor_millivolts: u32,
    pub from_boot_word_ns: i64,
    pub fall_ns: u64,
    pub hold_ns: u64,
    pub rise_ns: u64,
    pub source_milliohms: u32,
    pub load_milliamps: u32,
}

/// A droop as the run rendered it (kind 16, `AET-WO-2310`): the scenario's,
/// its instant from reset ([`resolve_instant`] of the undisturbed boot word),
/// and the specification's BOR level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RailDroop {
    pub droop: Droop,
    pub start_ns: u64,
    pub bor_level: u8,
}

impl Droop {
    /// The I·R drop the load makes across the source, in µV.
    pub fn drop_microvolts(&self) -> u64 {
        u64::from(self.load_milliamps) * u64::from(self.source_milliohms)
    }
}

/// One line of an I2C bus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    Sda,
    Scl,
}

/// One line of a solved I2C bus, as kind 15 names it: the solver's signal
/// (`I2C1_SCL`), its net, its pin and alternate function, and the net's
/// rise-time ceiling, which `analog_lines` holds per net.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BusLine {
    pub signal: String,
    pub net: String,
    pub pin: String,
    pub alternate_function: u8,
    pub ceiling_ps: u64,
}

/// A solved pin's port letter and index, `PB6` as (`B`, 6): `P`, one capital
/// letter, then 0 to 15 in decimal with no leading zero, the form the solver
/// names pins in. Anything else is not a pin.
pub fn split_pin(pin: &str) -> Option<(char, u8)> {
    let rest = pin.strip_prefix('P')?;
    let mut chars = rest.chars();
    let port = chars.next().filter(char::is_ascii_uppercase)?;
    let digits = chars.as_str();
    let canonical = matches!(digits.len(), 1 | 2)
        && digits.bytes().all(|b| b.is_ascii_digit())
        && !(digits.len() == 2 && digits.starts_with('0'));
    let index: u8 = digits.parse().ok().filter(|_| canonical)?;
    (index <= 15).then_some((port, index))
}

/// The highest alternate function a GPIO pin has (`AFSEL` is four bits).
pub const MAX_ALTERNATE_FUNCTION: u8 = 15;

/// The deck an analog record was computed in (`AET-WO-2205` §1.3, review
/// A10.4): one byte window at the deck's fixed bit rate, whatever the bus's
/// declared speed, stated beside it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    /// The deck's bit rate: 400 kHz for every bus.
    pub deck_hz: u32,
    /// The bus's declared maximum speed, from the solved topology.
    pub declared_hz: u32,
}

/// The most executable segments kind 10 carries; the CLI refuses a firmware
/// with more.
pub const MAX_EXECUTABLE_SEGMENTS: usize = 16;

/// Whether `pc` is in one of `executable`'s `[start, end)` ranges.
pub fn inside(executable: &[(u32, u32)], pc: u32) -> bool {
    executable.iter().any(|&(s, e)| pc >= s && pc < e)
}

/// A register as the boot read it: what the topology expects under `mask`,
/// and what Renode read, absent when the boot was killed before the reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    pub address: u32,
    pub mask: u32,
    pub expected: u32,
    pub observed: Option<u32>,
}

/// One campaign record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    /// Kind 5: the scenario file's sha256, its name, what it expects, and its overrides.
    Scenario {
        name: String,
        sha256: [u8; 32],
        expect_pass: bool,
        /// The finding kinds a failing scenario expects, sorted and unique.
        expect_findings: Vec<String>,
        overrides: Vec<Override>,
    },
    /// Kind 6: the overridden specification's and the solved topology's sha256.
    Solved {
        specification_sha256: [u8; 32],
        topology_sha256: [u8; 32],
    },
    /// Kind 7: `analog.rise_time_computed`, by ngspice from the declared
    /// pull-up and capacitance (`RUL-19` (1)). With `complete` false, the rise
    /// is a lower bound.
    RiseComputed {
        bus: String,
        net: String,
        rise_30_70_ps: u64,
        complete: bool,
        ceiling_ps: u64,
        exceeds: bool,
        window: Window,
    },
    /// Kind 8: `analog.line_stuck_high`, computed as kind 7 is.
    LineStuckHigh {
        bus: String,
        net: String,
        microvolts_at_release: i64,
        ceiling_ps: u64,
        window: Window,
    },
    /// Kind 9: `bridge.fault_state`.
    BusFault {
        bus: String,
        fault: String,
        source: String,
        from: Vec<String>,
        nets: Vec<String>,
        firmware_sees: String,
    },
    /// Kind 10: the boot's witnesses (`firmware.observed`), and the
    /// firmware's executable segments the judge placed `pc` against
    /// (`AET-WO-2205` §1.5).
    BootObserved {
        image_sha256: [u8; 32],
        boot_word: Reading,
        instructions_expected: u64,
        instructions_observed: Option<u64>,
        pc: Option<u32>,
        /// `[start, end)` at each segment's physical address, which is its
        /// virtual one (`AET-WO-2204`), sorted, at most
        /// [`MAX_EXECUTABLE_SEGMENTS`].
        executable: Vec<(u32, u32)>,
        /// The boot record's word 1, `FLASH_OPTSR_CUR` as the firmware read
        /// it, judged under `BOR_LEV`'s mask against the declared level in
        /// place (`AET-WO-4230` D10); read with the boot word.
        brown_out: Reading,
        /// What the flash controller's access log showed that the bring-up
        /// never does, at most [`MAX_WITNESS_LINES`] (a cut list still fails):
        /// empty when nothing; `None` when unread or when the log could not
        /// be read whole.
        flash_access: Option<Vec<String>>,
    },
    /// Kind 11: one solved register after the boot.
    RegisterObserved { name: String, reading: Reading },
    /// Kind 12: one step of a device's script, as the firmware recorded it
    /// (`AET-WO-4210`): the bytes it writes, then the read of `width` bytes
    /// (none for a `configure` step) against the part's value under `mask`.
    I2cCheck {
        reference: String,
        address: u8,
        step: String,
        write: Vec<u8>,
        width: u8,
        expected: u64,
        mask: u64,
        observed: Option<u64>,
        status: Option<String>,
    },
    /// Kind 13: one identity read's witness (`AET-WO-2204` D1): the whole
    /// log lines the device's model writes for the read, as the judge
    /// expects them, and every line Renode logged for the device with a
    /// witness's shape, in order, at most [`MAX_WITNESS_LINES`]; `None` when
    /// the log could not be read whole (past the filter's budget).
    I2cWitness {
        peripheral: String,
        reference: String,
        /// The model's path in Renode, `i2c1.u2`.
        device: String,
        step: String,
        expected: Vec<String>,
        observed: Option<Vec<String>>,
        /// The addresses the device's controller found no device at during
        /// this step (`AET-WO-3305`, trace 16; the whole run before), in
        /// order, at most [`MAX_WITNESS_LINES`]: attribution's evidence of a
        /// transfer to a wrong address, never a witness.
        absent: Vec<u8>,
    },
    /// Kind 14: the verdict, with each distinct finding kind, its severity and
    /// how many times it was found, sorted.
    Verdict {
        pass: bool,
        findings: Vec<(String, String, u64)>,
    },
    /// Kind 15: one solved I2C bus, from the solved topology alone
    /// (`AET-WO-3100` R1): its controller, its two lines, SCL then SDA, and
    /// its devices by reference, sorted. It joins a device or a controller to
    /// the bus kinds 7 to 9 name. Archived after kind 6, before any kind 7.
    I2cBusSolved {
        bus: String,
        peripheral: String,
        lines: Vec<BusLine>,
        devices: Vec<(String, u8)>,
    },
    /// Kind 16: the declared rail (`AET-WO-2301`, ruled D8): every value the
    /// rail was computed from, so the waveform is re-derivable from the trace.
    /// The droop's own values, then the domain's nominal and tolerance and the
    /// decoupling the emitter places on it.
    RailDeclared {
        droop: RailDroop,
        nominal_millivolts: u32,
        tolerance_permille: u32,
        decoupling_picofarads: u64,
    },
    /// Kind 17: `analog.rail_crossing_computed`, one crossing of one
    /// threshold's level, computed by ngspice from the declared values.
    RailCrossing {
        domain: String,
        level: String,
        microvolts: u64,
        at_ps: u64,
        falling: bool,
    },
    /// Kind 18: `bridge.reset_rendered`, a reset asserted or released: the
    /// model's rendering of the supervisor, never a measured reset. Renode
    /// models no BOR, PVD or reset cause; `rendering` names the monitor
    /// commands that stood in for it.
    ResetRendered {
        at_ns: u64,
        released: bool,
        cause: Vec<String>,
        rendering: String,
    },
    /// Kind 19: `firmware.observed` after a release, at the recovery bound:
    /// the boot word and the identity read's two words, as Renode read them,
    /// and the first identity read's witness lines from the release to the
    /// probe (`AET-WO-2301` adversarial F1, `AET-WO-2204`).
    FirmwareObserved {
        at_ns: u64,
        boot_word: Reading,
        status: Reading,
        value: Reading,
        /// The first identity read's witness-shaped lines from the release
        /// to the probe (`AET-WO-2204`): `None` when the boot has no identity
        /// read, or when the log could not be read whole.
        witness: Option<Vec<String>>,
    },
    /// Kind 20: the scenario's `expect_counts`, sorted by kind, archived only
    /// when it declares them.
    ExpectedCounts { counts: Vec<(String, u64)> },
    /// Kind 21: one SPI identity read, as the firmware recorded it
    /// (`AET-WO-4200`, kind 12's analogue): the device on its controller, at
    /// its multiplexer address, selected by `select`, the command it writes
    /// first, and the part's value against the record's.
    SpiCheck {
        reference: String,
        peripheral: String,
        select: String,
        address: u8,
        step: String,
        write: Vec<u8>,
        width: u8,
        expected: u64,
        mask: u64,
        observed: Option<u64>,
        status: Option<String>,
    },
    /// Kind 22: one SPI read's witness (`AET-WO-4200` D2, kind 13's
    /// analogue): the model's expected `Transmit`-path lines and what Renode
    /// logged for the device, as kind 13 holds them; and every line the
    /// controller's multiplexer logged when a byte reached no device, several,
    /// or an address with none, at most [`MAX_WITNESS_LINES`]: attribution's
    /// evidence (D4), never a witness.
    SpiWitness {
        peripheral: String,
        reference: String,
        device: String,
        step: String,
        expected: Vec<String>,
        observed: Option<Vec<String>>,
        /// The multiplexer's lines inside this step's chip-select window
        /// (`AET-WO-4210` D5).
        unrouted: Vec<String>,
    },
    /// Kind 23: one solved SPI controller, from the solved topology alone
    /// (`AET-WO-4200` adversarial A3, kind 15's analogue): its devices in
    /// the specification's order, each with its multiplexer address (its
    /// place on the controller, from 0) and its chip select's pin. Kinds 21
    /// and 22 and an absent device's override join to it. Archived beside
    /// kind 15, before the boot.
    SpiBusSolved {
        peripheral: String,
        devices: Vec<(String, u8, String)>,
    },
    /// Kind 24: one solved motor output, from the solved topology alone
    /// (`AET-WO-4300a`): its label (its net), timer and channel, pin and
    /// alternate function, protocol, the solved `PSC`, `ARR` and `CCR`, and
    /// for PWM its declared rate and pulse range in µs. Kinds 25 and 26 join
    /// to it. Archived beside kinds 15 and 23, before the boot. Since trace
    /// 18 (`AET-WO-1211c`), for PWM also its declared idle and tick
    /// (`pwm.idle`, `pwm.resolution_ps`, `AET-WO-1211a`), which kind 25's
    /// check has judged since then: replay re-judges kind 25 from them.
    MotorOutputSolved {
        label: String,
        timer: String,
        channel: u8,
        pin: String,
        alternate_function: u8,
        protocol: String,
        psc: u32,
        arr: u32,
        ccr: u32,
        pwm: Option<(u32, u32, u32)>,
        /// Present exactly for PWM, as `pwm` is.
        idle: Option<MotorIdle>,
        resolution_ps: Option<u64>,
    },
    /// Kind 25: the static check for silicon of one motor output (`4300`
    /// D3): its timer's kernel clock from the solved clock, the period and
    /// high time the solved registers give with silicon's `ARR + 1`, the
    /// declared ones, in ps, and whether they agree within the check's rules.
    /// Decided from the topology alone, before the boot.
    MotorTimingCheck {
        label: String,
        kernel_hz: u64,
        period_ps: u64,
        high_ps: u64,
        nominal_period_ps: u64,
        nominal_high_ps: u64,
        holds: bool,
    },
    /// Kind 26: one motor output's twin witness (`4300` D4): its observer,
    /// the model's period and high time (since trace 14, `AET-WO-4400g`, as
    /// [`realized_ns`] gives them at kind 25's `kernel_hz`, the frequency the
    /// twin builds the timer at), the stamp grain and the quantum, in ns, and what the observer showed (`status`, one of
    /// [`MOTOR_STATUSES`]; the counts; the first rise; the deviations'
    /// spread; edges off the grid; edges sharing a stamp). Since trace 13
    /// (`AET-WO-2302`) its window: `anchor_ns`, its timer's first `CEN` rise
    /// in the boot the judge reads (absent when none was observed), and
    /// `end_ns`, `DSHOT_WINDOW_NS` after it for DShot and the boot's end for
    /// PWM; replay derives both from the records. And the edges the window
    /// left out, `edges_before` its anchor and `edges_after` its end, and
    /// `cen_rises`, its timer's `CEN` rises in that boot (0 exactly when no
    /// anchor was observed; one timer's outputs all carry the same), from
    /// which replay binds `motor_edge_before_anchor`,
    /// `motor_edge_after_window` and `motor_anchor_repeated` (adversarial
    /// pass 1, L3).
    MotorWitness {
        label: String,
        observer: String,
        period_ns: u64,
        high_ns: u64,
        grain_ns: u64,
        quantum_ns: u64,
        status: String,
        rises: u64,
        falls: u64,
        first_ns: Option<u64>,
        /// The last rise, and the end of the window it must reach.
        last_ns: Option<u64>,
        end_ns: u64,
        span_ns: Option<u64>,
        off_grid: u64,
        batched: u64,
        anchor_ns: Option<u64>,
        edges_before: u64,
        edges_after: u64,
        cen_rises: u64,
    },
    /// Kind 27: one solved serial port, from the solved topology alone
    /// (`AET-WO-4300b`): its label, its place in the specification, its
    /// USART, its TX and RX pins and their alternate functions, its role and
    /// framing, and the `BRR` solved on its kernel clock with the baud it
    /// gives (in thousandths) and its error (ppm). Kinds 28 and 29 join to
    /// it. Archived beside kinds 15, 23 and 24, before the boot.
    SerialPortSolved {
        label: String,
        index: u32,
        peripheral: String,
        tx: String,
        tx_af: u8,
        rx: String,
        rx_af: u8,
        role: String,
        framing: SerialFraming,
        kernel_hz: u64,
        brr: u32,
        actual_millibaud: u64,
        error_ppm: i64,
    },
    /// Kind 28: one port's static framing judgement, its two halves apart
    /// (`4300` D8). Silicon's: the kernel clock, `BRR`, the baud and error it
    /// gives, the receiver-tolerance row that applies (RM0433 Rev 9 Tables
    /// 406 and 407: the table, `M[1:0]`, the tolerance in ppm) and whether
    /// every rule held. The twin's: the tags the framing needs (of
    /// [`SERIAL_TAGS`]) and what the USART's writes showed ([`SerialTags`]),
    /// absent until
    /// Renode has run or when its log could not be read whole. The stored
    /// fields (`CR1`, `CR2.STOP`, `CR3`, `BRR`) are read back as kind 11s
    /// and judged there. Archived after the boot.
    SerialFramingCheck {
        label: String,
        kernel_hz: u64,
        brr: u32,
        actual_millibaud: u64,
        error_ppm: i64,
        tolerance: Option<(String, u8, u32)>,
        holds: bool,
        tags_needed: Vec<String>,
        tags: Option<SerialTags>,
    },
    /// Kind 29: one port's twin witness (`AET-WO-4300b` §1.5): what its
    /// capture held (absent when Renode left no file), the frame injected,
    /// the echo's status (one of [`SERIAL_ECHO_STATUSES`]), and the RX pin's
    /// `MODER` and `AFSEL` fields at the injection (absent when no frame went
    /// in), the TX pin's at the end (as kind 11 reads its registers), and
    /// the TX pin's at the first `TDR` write of each boot, rebuilt from the
    /// access log: the first boot's that is not routed, else the first that
    /// left the AF after it was solved, else the last's (absent
    /// when no boot wrote `TDR`, a boot's could not be rebuilt, or the log
    /// could not be read whole; adversarial F4, pass 4 ADV4-4). The echo is
    /// `not_counted` too when the RX pin changed after it was solved,
    /// which kind 28's tags name. Archived after kind 28, once Renode has
    /// run. Since trace 13 (`AET-WO-2302` D4) `injected_ns`: the first frame
    /// byte's instant, one quantum after the boot word for the first port
    /// and after the port before's last echoed byte for each later one,
    /// present exactly when `rx_at_injection` is.
    SerialWitness {
        label: String,
        capture: Option<SerialCapture>,
        frame: [u8; SERIAL_FRAME_LEN],
        echo: String,
        rx_at_injection: Option<(u8, u8)>,
        tx_at_end: Option<(u8, u8)>,
        tx_at_send: Option<(u8, u8)>,
        injected_ns: Option<u64>,
    },
    /// Kind 30: one judge's stop (`AET-WO-2302` D2, accepted): a DShot
    /// timer (`TIM1`, kind 24's `timer`) at its window's end, `at_ns`,
    /// `DSHOT_WINDOW_NS` after its anchor, `anchor_ns`; each of its solved
    /// registers as the stop read it back before clearing `CEN` (as kind 11
    /// reads one, at most [`MAX_STOP_REGISTERS`]); and `CR1` after it
    /// (absent when the stop logged no read after it). The judge's own act
    /// on the machine: after it, a firmware reading the timer sees it
    /// stopped, a residual every such verdict names. Archived after kind 26.
    JudgeStop {
        timer: String,
        anchor_ns: u64,
        at_ns: u64,
        read_back: Vec<Reading>,
        cr1_after: Option<u32>,
    },
    /// Kind 31: the battery sag's declaration (`AET-WO-2304b` §1.9), every
    /// value the chain was judged from, so replay re-judges it without IR:
    /// the sag; the pack's open voltage, µV, and the healthy path R, mΩ;
    /// each battery-fed sink in the specification's order; the capacitance
    /// on `VBAT` the deck holds, pF; the static operating point at the
    /// fault R, µV; the deck's solved minimum, µV; and the sink whose floor
    /// binds. Archived before the boot, after the bus faults.
    BatterySagDeclared {
        sag: BatterySag,
        /// The sag's instant from reset, resolved from the undisturbed boot
        /// word (`AET-WO-2310`).
        start_ns: u64,
        v_open_microvolts: u64,
        healthy_milliohms: u32,
        sinks: Vec<SagSink>,
        capacitance_picofarads: u64,
        static_microvolts: u64,
        solved_minimum_microvolts: u64,
        binding: String,
    },
    /// Kind 32: a droop's recovery bound (`AET-WO-2305` §1.2, ruled
    /// 2026-11-20): the instant the same prepared boot, run first with no
    /// reset, wrote its boot word, counted from 0 (`None` when it wrote none,
    /// or its log could not be read whole); the margin,
    /// [`crate::power::RECOVERY_MARGIN_NS`]; and the bound, their sum (`None`
    /// with no instant: no probe is read, and every release is
    /// unrecovered, by name). Archived once, when the droop releases a reset.
    RecoveryBound {
        boot_word_ns: Option<u64>,
        margin_ns: u64,
        bound_ns: Option<u64>,
    },
    /// Kind 33: one GPIO signal of a pin device, as the twin declares it
    /// before the boot (`AET-WO-1211c`, virtual time 0): its net and pin.
    /// An output: `level` its declared initial level, `active` absent,
    /// `held` false. An input: `level` the level the twin drives it at for
    /// this run, `active` its declared active level, and `held` whether a
    /// `pin_held` override set `level` (otherwise it is the declared pull's).
    /// Kind 34 joins to it by net.
    PinGpioDeclared {
        net: String,
        pin: String,
        output: bool,
        level: u8,
        active: Option<u8>,
        held: bool,
    },
    /// Kind 34: one GPIO signal's twin witness, after the boot
    /// (`AET-WO-1211c`), `status` one of [`GPIO_STATUSES`]. For an output,
    /// `bsrr` is the first `BSRR` word the firmware wrote that touches the
    /// pin's set or reset bit and `ordered` whether it came before the first
    /// `MODER` write making the pin an output; `level` is absent. For an
    /// input, `level` is the pin's `IDR` bit read at the boot's end; `bsrr`
    /// and `ordered` are absent. Whether the signal is an output or an input
    /// is its kind 33's.
    PinGpioWitness {
        net: String,
        status: String,
        bsrr: Option<u32>,
        ordered: Option<bool>,
        level: Option<u8>,
    },
    /// Kind 35: one ADC input of a pin device, as the twin declares it
    /// before the boot (`AET-WO-1211c`, virtual time 0): its net and pin,
    /// the ADC (`ADC1` or `ADC3`) and channel, the resolution in bits (12,
    /// the only one measured in the twin), the voltage it is driven at in
    /// µV, whether an `analog_value` override set it, and the `code` that
    /// voltage reads as, [`adc_code`]. Kind 36 joins to it by net.
    PinAdcDeclared {
        net: String,
        pin: String,
        peripheral: String,
        channel: u8,
        bits: u8,
        microvolts: u64,
        held: bool,
        code: u32,
    },
    /// Kind 36: one ADC input's twin witness, after the boot
    /// (`AET-WO-1211c`), `status` one of [`ADC_STATUSES`]: how many `DR`
    /// reads of that ADC the judge saw, and the `code` it matched to this
    /// input (absent exactly when `unwitnessed`).
    PinAdcWitness {
        net: String,
        status: String,
        reads: u64,
        code: Option<u32>,
    },
    /// Kind 37: a motor output whose idle is a static level, judged after
    /// the boot beside its kind 26 (`AET-WO-1211c`; the kind 26 says
    /// `level`): `level` the declared idle level; `settle_ns` its timer's
    /// `CEN` anchor plus one period (`AET-WO-1211` Q10: an idle-high output
    /// reads low until the first update event), absent with no anchor; the
    /// edges seen after `settle_ns`; `end_level` the observer's level at the
    /// window's end (absent when unread); and `status`, one of
    /// [`LEVEL_STATUSES`].
    MotorLevelWitness {
        label: String,
        level: u8,
        settle_ns: Option<u64>,
        rises_after: u64,
        falls_after: u64,
        end_level: Option<u8>,
        status: String,
    },
    /// Kind 38: a foreign run's declared input (`AET-WO-4400i`): the serial
    /// port's label and its USART, the time between two bytes in ns (the
    /// port's character time, a whole number of quanta), and the input file's
    /// bytes verbatim. Replay recomputes the file's sha256, its frames and
    /// every byte's instant from it, with no file (`INV-04`).
    ForeignInput {
        label: String,
        peripheral: String,
        spacing_ns: u64,
        file: Vec<u8>,
    },
    /// Kind 39: every stimulus a foreign input run set, in time order, as
    /// `[at_ns, device, property, value]`, the value exactly as Renode was
    /// given it (the board at rest, `AET-WO-4400i` D4).
    Stimuli {
        entries: Vec<(u64, String, String, String)>,
    },
    /// Kind 40: the bytes the firmware transmitted on the input's port, in
    /// order, read from the port's file backend at the run's end, `end_ns`
    /// (`AET-WO-4400i`): the witness's only source.
    SerialCaptured {
        label: String,
        end_ns: u64,
        bytes: Vec<u8>,
    },
}

/// The most registers kind 30 reads back: a timer's solved registers
/// (`CR1`, `CR2`, `SMCR`, `DIER`, `CCMR1`, `CCMR2`, `CCER`, `PSC`, `ARR`,
/// `RCR`, four `CCRx`) with room.
pub const MAX_STOP_REGISTERS: usize = 32;

/// A pin's `MODER` and `AFSEL` fields, present or absent.
fn mux(m: Option<(u8, u8)>) -> Value {
    Value::Array(
        m.iter()
            .map(|(mode, af)| Value::Array(vec![u((*mode).into()), u((*af).into())]))
            .collect(),
    )
}

fn u(n: u64) -> Value {
    Value::Uint(n)
}

fn t(s: &str) -> Value {
    Value::Text(s.to_owned())
}

fn flag(b: bool) -> Value {
    u(u64::from(b))
}

fn opt(v: Option<u64>) -> Value {
    Value::Array(v.map(u).into_iter().collect())
}

fn texts(v: &[String]) -> Value {
    Value::Array(v.iter().map(|s| t(s)).collect())
}

fn u8s(v: &[u8]) -> Value {
    Value::Array(v.iter().map(|&b| u(b.into())).collect())
}

fn opt_texts(v: Option<&[String]>) -> Value {
    Value::Array(v.map(texts).into_iter().collect())
}

/// Witness lines as a record carries them: at most [`MAX_WITNESS_LINES`],
/// each 1 to 256 printable ASCII bytes.
fn witness_lines_ok(v: &[String]) -> bool {
    v.len() <= MAX_WITNESS_LINES && v.iter().all(|s| text_ok(s, 1, 256))
}

/// A step's place among a device's steps, from its name, in the order a
/// script runs them: `identity` 0; one configuration, `configure`, 1, or one
/// of several, `configure 1` to `configure 4`, its number (`AET-WO-4220`:
/// each step has its own name, so a record is joined to one step alone);
/// `data` 5. `None` for any other name.
// belt-and-suspenders: the names restate `aethelgard_ir::parts::CONFIGURE_STEPS`,
// which this crate may not depend on (`workspace_shape`); the archive reads a
// run without the table (`INV-07`). `own_models_tier1.rs` holds the two equal.
pub fn step_index(step: &str) -> Option<usize> {
    match step {
        "identity" => Some(0),
        "configure" | "configure 1" => Some(1),
        "configure 2" => Some(2),
        "configure 3" => Some(3),
        "configure 4" => Some(4),
        "data" => Some(5),
        _ => None,
    }
}

/// Whether a step is a configuration write, which reads nothing.
fn is_configure(step: &str) -> bool {
    matches!(step_index(step), Some(1..=4))
}

/// A step's fields as kinds 12 and 21 carry them (`AET-WO-4210`): a known
/// step, a write of 1 to 4 bytes, a read of 0 to 8 bytes (0 exactly for
/// `configure`), and an expected value and mask that fit the read.
fn step_ok(
    step: &str,
    write: &[u8],
    width: u8,
    expected: u64,
    mask: u64,
) -> Result<(), &'static str> {
    let fits = |v: u64| width >= 8 || v >> (8 * u32::from(width)) == 0;
    if step_index(step).is_none() {
        Err("step is not identity, a configuration or data")
    } else if write.is_empty() || write.len() > 4 {
        Err("write is not 1 to 4 bytes")
    } else if width > 8 || (width == 0) != is_configure(step) {
        Err("width is not 0 to 8, or is 0 for a step that reads or not 0 for configure")
    } else if !fits(mask) || !fits(expected) {
        Err("expected or mask does not fit the width")
    } else {
        Ok(())
    }
}

/// The most witness lines a record carries: a judge sees two per identity
/// read, so a longer observation already differs from what it expects, and
/// cutting it here cannot make it match.
pub const MAX_WITNESS_LINES: usize = 16;

fn window_value(w: &Window) -> Value {
    Value::Array(vec![u(w.deck_hz.into()), u(w.declared_hz.into())])
}

fn reading(r: &Reading) -> Value {
    Value::Array(vec![
        u(r.address.into()),
        u(r.mask.into()),
        u(r.expected.into()),
        opt(r.observed.map(u64::from)),
    ])
}

/// Reads fields in order; every accessor is `None` on a wrong type.
struct Fields<'a>(std::slice::Iter<'a, Value>);

impl<'a> Fields<'a> {
    fn next(&mut self) -> Option<&'a Value> {
        self.0.next()
    }
    fn uint(&mut self) -> Option<u64> {
        match self.next()? {
            Value::Uint(n) => Some(*n),
            _ => None,
        }
    }
    fn int(&mut self) -> Option<i64> {
        self.next()?.as_i64()
    }
    fn u32(&mut self) -> Option<u32> {
        u32::try_from(self.uint()?).ok()
    }
    fn u8(&mut self) -> Option<u8> {
        u8::try_from(self.uint()?).ok()
    }
    fn flag(&mut self) -> Option<bool> {
        match self.uint()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
    fn text(&mut self) -> Option<String> {
        match self.next()? {
            Value::Text(s) => Some(s.clone()),
            _ => None,
        }
    }
    fn hash(&mut self) -> Option<[u8; 32]> {
        match self.next()? {
            Value::Bytes(b) => <[u8; 32]>::try_from(b.as_slice()).ok(),
            _ => None,
        }
    }
    fn array(&mut self) -> Option<&'a [Value]> {
        match self.next()? {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
    fn opt(&mut self) -> Option<Option<u64>> {
        match self.array()? {
            [] => Some(None),
            [Value::Uint(n)] => Some(Some(*n)),
            _ => None,
        }
    }
    fn opt_u8(&mut self) -> Option<Option<u8>> {
        match self.opt()? {
            None => Some(None),
            Some(n) => u8::try_from(n).ok().map(Some),
        }
    }
    /// An array of zero or one flag, `0` or `1` and nothing else.
    fn opt_flag(&mut self) -> Option<Option<bool>> {
        match self.opt()? {
            None => Some(None),
            Some(0) => Some(Some(false)),
            Some(1) => Some(Some(true)),
            Some(_) => None,
        }
    }
    fn opt_u32(&mut self) -> Option<Option<u32>> {
        match self.opt()? {
            None => Some(None),
            Some(n) => u32::try_from(n).ok().map(Some),
        }
    }
    fn texts(&mut self) -> Option<Vec<String>> {
        let mut inner = Fields(self.array()?.iter());
        let mut out = Vec::new();
        while inner.0.len() > 0 {
            out.push(inner.text()?);
        }
        Some(out)
    }
    fn bytes(&mut self) -> Option<Vec<u8>> {
        match self.next()? {
            Value::Bytes(b) => Some(b.clone()),
            _ => None,
        }
    }

    fn u8s(&mut self) -> Option<Vec<u8>> {
        let mut inner = Fields(self.array()?.iter());
        let mut out = Vec::new();
        while inner.0.len() > 0 {
            out.push(inner.u8()?);
        }
        Some(out)
    }
    fn opt_texts(&mut self) -> Option<Option<Vec<String>>> {
        let mut inner = Fields(self.array()?.iter());
        if inner.0.len() == 0 {
            return Some(None);
        }
        let v = inner.texts()?;
        inner.end()?;
        Some(Some(v))
    }
    fn window(&mut self) -> Option<Window> {
        let mut w = Fields(self.array()?.iter());
        let window = Window {
            deck_hz: w.u32()?,
            declared_hz: w.u32()?,
        };
        w.end()?;
        Some(window)
    }
    fn reading(&mut self) -> Option<Reading> {
        let mut r = Fields(self.array()?.iter());
        let reading = Reading {
            address: r.u32()?,
            mask: r.u32()?,
            expected: r.u32()?,
            observed: r.opt_u32()?,
        };
        r.end()?;
        Some(reading)
    }
    fn end(&mut self) -> Option<()> {
        self.0.next().is_none().then_some(())
    }
    /// An array of zero or one element, the one read by `inner`.
    fn opt_with<T>(
        &mut self,
        inner: impl FnOnce(&mut Fields<'a>) -> Option<T>,
    ) -> Option<Option<T>> {
        match self.array()? {
            [] => Some(None),
            [Value::Array(items)] => {
                let mut g = Fields(items.iter());
                let v = inner(&mut g)?;
                g.end()?;
                Some(Some(v))
            }
            _ => None,
        }
    }
    fn mux(&mut self) -> Option<Option<(u8, u8)>> {
        self.opt_with(|g| Some((g.u8()?, g.u8()?)))
    }
}

fn override_value(o: &Override) -> Value {
    Value::Array(match o {
        Override::PullUpOhms { bus, ohms } => vec![u(0), t(bus), u((*ohms).into())],
        Override::MaxCapacitance { net, femtofarads } => vec![u(1), t(net), u(*femtofarads)],
        Override::LineHeldLow { bus, line } => vec![
            u(2),
            t(bus),
            u(match line {
                Line::Sda => 0,
                Line::Scl => 1,
            }),
        ],
        Override::Droop(d) => {
            let mut v = vec![u(3)];
            v.extend(droop_fields(d));
            v
        }
        Override::SpiDeviceAbsent { reference } => vec![u(4), t(reference)],
        Override::SerialRxOpen { port } => vec![u(5), t(port)],
        Override::BatterySag(b) => vec![
            u(6),
            u(b.series_milliohms.into()),
            Value::from_i64(b.from_boot_word_ns),
        ],
        Override::RailOutOfBand { domain } => vec![u(7), t(domain)],
        Override::PinHeld { net, level } => vec![
            u(8),
            t(net),
            u(match level {
                Level::Low => 0,
                Level::High => 1,
            }),
        ],
        Override::AnalogValue { net, millivolts } => vec![u(9), t(net), u((*millivolts).into())],
    })
}

fn droop_fields(d: &Droop) -> Vec<Value> {
    vec![
        t(&d.domain),
        u(d.floor_millivolts.into()),
        Value::from_i64(d.from_boot_word_ns),
        u(d.fall_ns),
        u(d.hold_ns),
        u(d.rise_ns),
        u(d.source_milliohms.into()),
        u(d.load_milliamps.into()),
    ]
}

impl Fields<'_> {
    fn droop(&mut self) -> Option<Droop> {
        Some(Droop {
            domain: self.text()?,
            floor_millivolts: self.u32()?,
            from_boot_word_ns: self.int()?,
            fall_ns: self.uint()?,
            hold_ns: self.uint()?,
            rise_ns: self.uint()?,
            source_milliohms: self.u32()?,
            load_milliamps: self.u32()?,
        })
    }
}

fn override_from(v: &Value) -> Option<Override> {
    let Value::Array(items) = v else { return None };
    let mut f = Fields(items.iter());
    let o = match f.uint()? {
        0 => Override::PullUpOhms {
            bus: f.text()?,
            ohms: f.u32()?,
        },
        1 => Override::MaxCapacitance {
            net: f.text()?,
            femtofarads: f.uint()?,
        },
        2 => Override::LineHeldLow {
            bus: f.text()?,
            line: match f.uint()? {
                0 => Line::Sda,
                1 => Line::Scl,
                _ => return None,
            },
        },
        3 => Override::Droop(f.droop()?),
        4 => Override::SpiDeviceAbsent {
            reference: f.text()?,
        },
        5 => Override::SerialRxOpen { port: f.text()? },
        6 => Override::BatterySag(BatterySag {
            series_milliohms: f.u32()?,
            from_boot_word_ns: f.int()?,
        }),
        7 => Override::RailOutOfBand { domain: f.text()? },
        8 => Override::PinHeld {
            net: f.text()?,
            level: match f.uint()? {
                0 => Level::Low,
                1 => Level::High,
                _ => return None,
            },
        },
        9 => Override::AnalogValue {
            net: f.text()?,
            millivolts: f.u32()?,
        },
        _ => return None,
    };
    f.end()?;
    Some(o)
}

impl Window {
    fn check(&self) -> Result<(), &'static str> {
        if self.deck_hz > 0 && self.declared_hz > 0 {
            Ok(())
        } else {
            Err("a window's deck and declared speeds are not both above 0")
        }
    }
}

impl Record {
    pub fn kind(&self) -> u64 {
        FIRST_KIND
            + match self {
                Self::Scenario { .. } => 0,
                Self::Solved { .. } => 1,
                Self::RiseComputed { .. } => 2,
                Self::LineStuckHigh { .. } => 3,
                Self::BusFault { .. } => 4,
                Self::BootObserved { .. } => 5,
                Self::RegisterObserved { .. } => 6,
                Self::I2cCheck { .. } => 7,
                Self::I2cWitness { .. } => 8,
                Self::Verdict { .. } => 9,
                Self::I2cBusSolved { .. } => 10,
                Self::RailDeclared { .. } => 11,
                Self::RailCrossing { .. } => 12,
                Self::ResetRendered { .. } => 13,
                Self::FirmwareObserved { .. } => 14,
                Self::ExpectedCounts { .. } => 15,
                Self::SpiCheck { .. } => 16,
                Self::SpiWitness { .. } => 17,
                Self::SpiBusSolved { .. } => 18,
                Self::MotorOutputSolved { .. } => 19,
                Self::MotorTimingCheck { .. } => 20,
                Self::MotorWitness { .. } => 21,
                Self::SerialPortSolved { .. } => 22,
                Self::SerialFramingCheck { .. } => 23,
                Self::SerialWitness { .. } => 24,
                Self::JudgeStop { .. } => 25,
                Self::BatterySagDeclared { .. } => 26,
                Self::RecoveryBound { .. } => 27,
                Self::PinGpioDeclared { .. } => 28,
                Self::PinGpioWitness { .. } => 29,
                Self::PinAdcDeclared { .. } => 30,
                Self::PinAdcWitness { .. } => 31,
                Self::MotorLevelWitness { .. } => 32,
                Self::ForeignInput { .. } => 33,
                Self::Stimuli { .. } => 34,
                Self::SerialCaptured { .. } => 35,
            }
    }

    /// The virtual time a record is archived at: 0 for what is decided
    /// before the boot, its own time for the droop's crossings, resets and
    /// observations, and `end_ns` for what the boot's end reads.
    pub fn timestamp(&self, end_ns: u64) -> u64 {
        match self {
            Self::RailCrossing { at_ps, .. } => at_ps / 1000,
            Self::ResetRendered { at_ns, .. } | Self::FirmwareObserved { at_ns, .. } => *at_ns,
            Self::SerialCaptured { end_ns, .. } => *end_ns,
            _ if self.before_boot() => 0,
            _ => end_ns,
        }
    }

    /// Whether the record is decided before the boot starts, at virtual time
    /// 0: the scenario, the topology, the solved buses, the analog records and
    /// the bus faults. The rest are read at the boot's end.
    pub fn before_boot(&self) -> bool {
        matches!(
            self,
            Self::Scenario { .. }
                | Self::Solved { .. }
                | Self::I2cBusSolved { .. }
                | Self::SpiBusSolved { .. }
                | Self::MotorOutputSolved { .. }
                | Self::MotorTimingCheck { .. }
                | Self::SerialPortSolved { .. }
                | Self::RiseComputed { .. }
                | Self::LineStuckHigh { .. }
                | Self::BusFault { .. }
                | Self::RailDeclared { .. }
                | Self::BatterySagDeclared { .. }
                | Self::RecoveryBound { .. }
                | Self::PinGpioDeclared { .. }
                | Self::PinAdcDeclared { .. }
                | Self::ExpectedCounts { .. }
                | Self::ForeignInput { .. }
                | Self::Stimuli { .. }
        )
    }

    /// The payload after the kind and the timestamp.
    pub fn fields(&self) -> Vec<Value> {
        match self {
            Self::Scenario {
                name,
                sha256,
                expect_pass,
                expect_findings,
                overrides,
            } => vec![
                t(name),
                Value::Bytes(sha256.to_vec()),
                flag(*expect_pass),
                texts(expect_findings),
                Value::Array(overrides.iter().map(override_value).collect()),
            ],
            Self::Solved {
                specification_sha256,
                topology_sha256,
            } => vec![
                Value::Bytes(specification_sha256.to_vec()),
                Value::Bytes(topology_sha256.to_vec()),
            ],
            Self::RiseComputed {
                bus,
                net,
                rise_30_70_ps,
                complete,
                ceiling_ps,
                exceeds,
                window,
            } => vec![
                t(bus),
                t(net),
                u(*rise_30_70_ps),
                flag(*complete),
                u(*ceiling_ps),
                flag(*exceeds),
                window_value(window),
            ],
            Self::LineStuckHigh {
                bus,
                net,
                microvolts_at_release,
                ceiling_ps,
                window,
            } => vec![
                t(bus),
                t(net),
                Value::from_i64(*microvolts_at_release),
                u(*ceiling_ps),
                window_value(window),
            ],
            Self::BusFault {
                bus,
                fault,
                source,
                from,
                nets,
                firmware_sees,
            } => vec![
                t(bus),
                t(fault),
                t(source),
                texts(from),
                texts(nets),
                t(firmware_sees),
            ],
            Self::BootObserved {
                image_sha256,
                boot_word,
                instructions_expected,
                instructions_observed,
                pc,
                executable,
                brown_out,
                flash_access,
            } => vec![
                Value::Bytes(image_sha256.to_vec()),
                reading(boot_word),
                u(*instructions_expected),
                opt(*instructions_observed),
                opt(pc.map(u64::from)),
                Value::Array(
                    executable
                        .iter()
                        .map(|&(s, e)| Value::Array(vec![u(s.into()), u(e.into())]))
                        .collect(),
                ),
                reading(brown_out),
                opt_texts(flash_access.as_deref()),
            ],
            Self::RegisterObserved { name, reading: r } => vec![t(name), reading(r)],
            Self::I2cCheck {
                reference,
                address,
                step,
                write,
                width,
                expected,
                mask,
                observed,
                status,
            } => vec![
                t(reference),
                u((*address).into()),
                t(step),
                u8s(write),
                u((*width).into()),
                u(*expected),
                u(*mask),
                opt(*observed),
                Value::Array(status.iter().map(|s| t(s)).collect()),
            ],
            Self::I2cWitness {
                peripheral,
                reference,
                device,
                step,
                expected,
                observed,
                absent,
            } => vec![
                t(peripheral),
                t(reference),
                t(device),
                t(step),
                texts(expected),
                opt_texts(observed.as_deref()),
                u8s(absent),
            ],
            Self::Verdict { pass, findings } => vec![
                flag(*pass),
                Value::Array(
                    findings
                        .iter()
                        .map(|(kind, severity, count)| {
                            Value::Array(vec![t(kind), t(severity), u(*count)])
                        })
                        .collect(),
                ),
            ],
            Self::I2cBusSolved {
                bus,
                peripheral,
                lines,
                devices,
            } => vec![
                t(bus),
                t(peripheral),
                Value::Array(
                    lines
                        .iter()
                        .map(|l| {
                            Value::Array(vec![
                                t(&l.signal),
                                t(&l.net),
                                t(&l.pin),
                                u(l.alternate_function.into()),
                                u(l.ceiling_ps),
                            ])
                        })
                        .collect(),
                ),
                Value::Array(
                    devices
                        .iter()
                        .map(|(reference, address)| {
                            Value::Array(vec![t(reference), u((*address).into())])
                        })
                        .collect(),
                ),
            ],
            Self::RailDeclared {
                droop,
                nominal_millivolts,
                tolerance_permille,
                decoupling_picofarads,
            } => {
                let mut v = droop_fields(&droop.droop);
                v.extend([
                    u(droop.start_ns),
                    u(droop.bor_level.into()),
                    u((*nominal_millivolts).into()),
                    u((*tolerance_permille).into()),
                    u(*decoupling_picofarads),
                ]);
                v
            }
            Self::RailCrossing {
                domain,
                level,
                microvolts,
                at_ps,
                falling,
            } => vec![
                t(domain),
                t(level),
                u(*microvolts),
                u(*at_ps),
                flag(*falling),
            ],
            Self::ResetRendered {
                at_ns,
                released,
                cause,
                rendering,
            } => vec![u(*at_ns), flag(*released), texts(cause), t(rendering)],
            Self::FirmwareObserved {
                at_ns,
                boot_word,
                status,
                value,
                witness,
            } => vec![
                u(*at_ns),
                reading(boot_word),
                reading(status),
                reading(value),
                opt_texts(witness.as_deref()),
            ],
            Self::ExpectedCounts { counts } => vec![Value::Array(
                counts
                    .iter()
                    .map(|(kind, n)| Value::Array(vec![t(kind), u(*n)]))
                    .collect(),
            )],
            Self::SpiCheck {
                reference,
                peripheral,
                select,
                address,
                step,
                write,
                width,
                expected,
                mask,
                observed,
                status,
            } => vec![
                t(reference),
                t(peripheral),
                t(select),
                u((*address).into()),
                t(step),
                u8s(write),
                u((*width).into()),
                u(*expected),
                u(*mask),
                opt(*observed),
                Value::Array(status.iter().map(|s| t(s)).collect()),
            ],
            Self::SpiWitness {
                peripheral,
                reference,
                device,
                step,
                expected,
                observed,
                unrouted,
            } => vec![
                t(peripheral),
                t(reference),
                t(device),
                t(step),
                texts(expected),
                opt_texts(observed.as_deref()),
                texts(unrouted),
            ],
            Self::SpiBusSolved {
                peripheral,
                devices,
            } => vec![
                t(peripheral),
                Value::Array(
                    devices
                        .iter()
                        .map(|(r, a, s)| Value::Array(vec![t(r), u((*a).into()), t(s)]))
                        .collect(),
                ),
            ],
            Self::MotorOutputSolved {
                label,
                timer,
                channel,
                pin,
                alternate_function,
                protocol,
                psc,
                arr,
                ccr,
                pwm,
                idle,
                resolution_ps,
            } => vec![
                t(label),
                t(timer),
                u((*channel).into()),
                t(pin),
                u((*alternate_function).into()),
                t(protocol),
                u((*psc).into()),
                u((*arr).into()),
                u((*ccr).into()),
                Value::Array(
                    pwm.iter()
                        .map(|(r, lo, hi)| {
                            Value::Array(vec![u((*r).into()), u((*lo).into()), u((*hi).into())])
                        })
                        .collect(),
                ),
                Value::Array(
                    idle.iter()
                        .map(|i| {
                            Value::Array(match i {
                                MotorIdle::Low => vec![u(0)],
                                MotorIdle::High => vec![u(1)],
                                MotorIdle::PulseUs(us) => vec![u(2), u((*us).into())],
                            })
                        })
                        .collect(),
                ),
                Value::Array(resolution_ps.iter().map(|ps| u(*ps)).collect()),
            ],
            Self::MotorTimingCheck {
                label,
                kernel_hz,
                period_ps,
                high_ps,
                nominal_period_ps,
                nominal_high_ps,
                holds,
            } => vec![
                t(label),
                u(*kernel_hz),
                u(*period_ps),
                u(*high_ps),
                u(*nominal_period_ps),
                u(*nominal_high_ps),
                flag(*holds),
            ],
            Self::MotorWitness {
                label,
                observer,
                period_ns,
                high_ns,
                grain_ns,
                quantum_ns,
                status,
                rises,
                falls,
                first_ns,
                last_ns,
                end_ns,
                span_ns,
                off_grid,
                batched,
                anchor_ns,
                edges_before,
                edges_after,
                cen_rises,
            } => vec![
                t(label),
                t(observer),
                u(*period_ns),
                u(*high_ns),
                u(*grain_ns),
                u(*quantum_ns),
                t(status),
                u(*rises),
                u(*falls),
                opt(*first_ns),
                opt(*last_ns),
                u(*end_ns),
                opt(*span_ns),
                u(*off_grid),
                u(*batched),
                opt(*anchor_ns),
                u(*edges_before),
                u(*edges_after),
                u(*cen_rises),
            ],
            Self::SerialPortSolved {
                label,
                index,
                peripheral,
                tx,
                tx_af,
                rx,
                rx_af,
                role,
                framing,
                kernel_hz,
                brr,
                actual_millibaud,
                error_ppm,
            } => vec![
                t(label),
                u((*index).into()),
                t(peripheral),
                t(tx),
                u((*tx_af).into()),
                t(rx),
                u((*rx_af).into()),
                t(role),
                Value::Array(vec![
                    u(framing.baud.into()),
                    u(framing.data_bits.into()),
                    t(&framing.parity),
                    u(framing.stop_bits.into()),
                    flag(framing.inverted),
                ]),
                u(*kernel_hz),
                u((*brr).into()),
                u(*actual_millibaud),
                Value::from_i64(*error_ppm),
            ],
            Self::SerialFramingCheck {
                label,
                kernel_hz,
                brr,
                actual_millibaud,
                error_ppm,
                tolerance,
                holds,
                tags_needed,
                tags,
            } => vec![
                t(label),
                u(*kernel_hz),
                u((*brr).into()),
                u(*actual_millibaud),
                Value::from_i64(*error_ppm),
                Value::Array(
                    tolerance
                        .iter()
                        .map(|(table, m, ppm)| {
                            Value::Array(vec![t(table), u((*m).into()), u((*ppm).into())])
                        })
                        .collect(),
                ),
                flag(*holds),
                texts(tags_needed),
                Value::Array(
                    tags.iter()
                        .map(|j| {
                            Value::Array(vec![u(j.writes), texts(&j.missing), texts(&j.unexpected)])
                        })
                        .collect(),
                ),
            ],
            Self::SerialWitness {
                label,
                capture,
                frame,
                echo,
                rx_at_injection,
                tx_at_end,
                tx_at_send,
                injected_ns,
            } => vec![
                t(label),
                Value::Array(
                    capture
                        .iter()
                        .map(|c| {
                            Value::Array(vec![
                                u(c.patterns),
                                u(c.cut),
                                u(c.echoed),
                                flag(c.stray),
                                u(c.len),
                            ])
                        })
                        .collect(),
                ),
                Value::Bytes(frame.to_vec()),
                t(echo),
                mux(*rx_at_injection),
                mux(*tx_at_end),
                mux(*tx_at_send),
                opt(*injected_ns),
            ],
            Self::JudgeStop {
                timer,
                anchor_ns,
                at_ns,
                read_back,
                cr1_after,
            } => vec![
                t(timer),
                u(*anchor_ns),
                u(*at_ns),
                Value::Array(read_back.iter().map(reading).collect()),
                opt(cr1_after.map(u64::from)),
            ],
            Self::RecoveryBound {
                boot_word_ns,
                margin_ns,
                bound_ns,
            } => vec![opt(*boot_word_ns), u(*margin_ns), opt(*bound_ns)],
            Self::PinGpioDeclared {
                net,
                pin,
                output,
                level,
                active,
                held,
            } => vec![
                t(net),
                t(pin),
                flag(*output),
                u((*level).into()),
                opt(active.map(u64::from)),
                flag(*held),
            ],
            Self::PinGpioWitness {
                net,
                status,
                bsrr,
                ordered,
                level,
            } => vec![
                t(net),
                t(status),
                opt(bsrr.map(u64::from)),
                opt(ordered.map(u64::from)),
                opt(level.map(u64::from)),
            ],
            Self::PinAdcDeclared {
                net,
                pin,
                peripheral,
                channel,
                bits,
                microvolts,
                held,
                code,
            } => vec![
                t(net),
                t(pin),
                t(peripheral),
                u((*channel).into()),
                u((*bits).into()),
                u(*microvolts),
                flag(*held),
                u((*code).into()),
            ],
            Self::PinAdcWitness {
                net,
                status,
                reads,
                code,
            } => vec![t(net), t(status), u(*reads), opt(code.map(u64::from))],
            Self::MotorLevelWitness {
                label,
                level,
                settle_ns,
                rises_after,
                falls_after,
                end_level,
                status,
            } => vec![
                t(label),
                u((*level).into()),
                opt(*settle_ns),
                u(*rises_after),
                u(*falls_after),
                opt(end_level.map(u64::from)),
                t(status),
            ],
            Self::ForeignInput {
                label,
                peripheral,
                spacing_ns,
                file,
            } => vec![t(label), t(peripheral), u(*spacing_ns), Value::Bytes(file.clone())],
            Self::Stimuli { entries } => vec![Value::Array(
                entries
                    .iter()
                    .map(|(at, device, property, value)| {
                        Value::Array(vec![u(*at), t(device), t(property), t(value)])
                    })
                    .collect(),
            )],
            Self::SerialCaptured {
                label,
                end_ns,
                bytes,
            } => vec![t(label), u(*end_ns), Value::Bytes(bytes.clone())],
            Self::BatterySagDeclared {
                sag,
                start_ns,
                v_open_microvolts,
                healthy_milliohms,
                sinks,
                capacitance_picofarads,
                static_microvolts,
                solved_minimum_microvolts,
                binding,
            } => vec![
                u(sag.series_milliohms.into()),
                Value::from_i64(sag.from_boot_word_ns),
                u(*start_ns),
                u(*v_open_microvolts),
                u((*healthy_milliohms).into()),
                Value::Array(
                    sinks
                        .iter()
                        .map(|k| {
                            Value::Array(vec![
                                t(&k.name),
                                u(k.input_milliwatts),
                                u(k.shutdown_millivolts),
                                u(k.regulation_millivolts),
                            ])
                        })
                        .collect(),
                ),
                u(*capacitance_picofarads),
                u(*static_microvolts),
                u(*solved_minimum_microvolts),
                t(binding),
            ],
        }
    }

    /// The record of `kind` from its payload, or `None` if a field has the
    /// wrong type or the payload the wrong length. [`check`](Self::check)
    /// judges the values.
    pub fn from_fields(kind: u64, payload: &[Value]) -> Option<Self> {
        let mut f = Fields(payload.iter());
        let record = match kind {
            5 => Self::Scenario {
                name: f.text()?,
                sha256: f.hash()?,
                expect_pass: f.flag()?,
                expect_findings: f.texts()?,
                overrides: f
                    .array()?
                    .iter()
                    .map(override_from)
                    .collect::<Option<_>>()?,
            },
            6 => Self::Solved {
                specification_sha256: f.hash()?,
                topology_sha256: f.hash()?,
            },
            7 => Self::RiseComputed {
                bus: f.text()?,
                net: f.text()?,
                rise_30_70_ps: f.uint()?,
                complete: f.flag()?,
                ceiling_ps: f.uint()?,
                exceeds: f.flag()?,
                window: f.window()?,
            },
            8 => Self::LineStuckHigh {
                bus: f.text()?,
                net: f.text()?,
                microvolts_at_release: f.int()?,
                ceiling_ps: f.uint()?,
                window: f.window()?,
            },
            9 => Self::BusFault {
                bus: f.text()?,
                fault: f.text()?,
                source: f.text()?,
                from: f.texts()?,
                nets: f.texts()?,
                firmware_sees: f.text()?,
            },
            10 => Self::BootObserved {
                image_sha256: f.hash()?,
                boot_word: f.reading()?,
                instructions_expected: f.uint()?,
                instructions_observed: f.opt()?,
                pc: f.opt_u32()?,
                executable: {
                    let mut v = Vec::new();
                    for item in f.array()? {
                        let Value::Array(pair) = item else {
                            return None;
                        };
                        let mut g = Fields(pair.iter());
                        v.push((g.u32()?, g.u32()?));
                        g.end()?;
                    }
                    v
                },
                brown_out: f.reading()?,
                flash_access: f.opt_texts()?,
            },
            11 => Self::RegisterObserved {
                name: f.text()?,
                reading: f.reading()?,
            },
            12 => Self::I2cCheck {
                reference: f.text()?,
                address: f.u8()?,
                step: f.text()?,
                write: f.u8s()?,
                width: f.u8()?,
                expected: f.uint()?,
                mask: f.uint()?,
                observed: f.opt()?,
                status: match f.array()? {
                    [] => None,
                    [Value::Text(s)] => Some(s.clone()),
                    _ => return None,
                },
            },
            13 => Self::I2cWitness {
                peripheral: f.text()?,
                reference: f.text()?,
                device: f.text()?,
                step: f.text()?,
                expected: f.texts()?,
                observed: f.opt_texts()?,
                absent: f.u8s()?,
            },
            14 => {
                let pass = f.flag()?;
                let mut findings = Vec::new();
                for item in f.array()? {
                    let Value::Array(triple) = item else {
                        return None;
                    };
                    let mut g = Fields(triple.iter());
                    findings.push((g.text()?, g.text()?, g.uint()?));
                    g.end()?;
                }
                Self::Verdict { pass, findings }
            }
            15 => {
                let bus = f.text()?;
                let peripheral = f.text()?;
                let mut lines = Vec::new();
                for item in f.array()? {
                    let Value::Array(line) = item else {
                        return None;
                    };
                    let mut g = Fields(line.iter());
                    lines.push(BusLine {
                        signal: g.text()?,
                        net: g.text()?,
                        pin: g.text()?,
                        alternate_function: g.u8()?,
                        ceiling_ps: g.uint()?,
                    });
                    g.end()?;
                }
                let mut devices = Vec::new();
                for item in f.array()? {
                    let Value::Array(pair) = item else {
                        return None;
                    };
                    let mut g = Fields(pair.iter());
                    devices.push((g.text()?, g.u8()?));
                    g.end()?;
                }
                Self::I2cBusSolved {
                    bus,
                    peripheral,
                    lines,
                    devices,
                }
            }
            16 => Self::RailDeclared {
                droop: RailDroop {
                    droop: f.droop()?,
                    start_ns: f.uint()?,
                    bor_level: f.u8()?,
                },
                nominal_millivolts: f.u32()?,
                tolerance_permille: f.u32()?,
                decoupling_picofarads: f.uint()?,
            },
            17 => Self::RailCrossing {
                domain: f.text()?,
                level: f.text()?,
                microvolts: f.uint()?,
                at_ps: f.uint()?,
                falling: f.flag()?,
            },
            18 => Self::ResetRendered {
                at_ns: f.uint()?,
                released: f.flag()?,
                cause: f.texts()?,
                rendering: f.text()?,
            },
            19 => Self::FirmwareObserved {
                at_ns: f.uint()?,
                boot_word: f.reading()?,
                status: f.reading()?,
                value: f.reading()?,
                witness: f.opt_texts()?,
            },
            20 => {
                let mut counts = Vec::new();
                for item in f.array()? {
                    let Value::Array(pair) = item else {
                        return None;
                    };
                    let mut g = Fields(pair.iter());
                    counts.push((g.text()?, g.uint()?));
                    g.end()?;
                }
                Self::ExpectedCounts { counts }
            }
            21 => Self::SpiCheck {
                reference: f.text()?,
                peripheral: f.text()?,
                select: f.text()?,
                address: f.u8()?,
                step: f.text()?,
                write: f.u8s()?,
                width: f.u8()?,
                expected: f.uint()?,
                mask: f.uint()?,
                observed: f.opt()?,
                status: match f.array()? {
                    [] => None,
                    [Value::Text(s)] => Some(s.clone()),
                    _ => return None,
                },
            },
            22 => Self::SpiWitness {
                peripheral: f.text()?,
                reference: f.text()?,
                device: f.text()?,
                step: f.text()?,
                expected: f.texts()?,
                observed: f.opt_texts()?,
                unrouted: f.texts()?,
            },
            23 => {
                let peripheral = f.text()?;
                let mut devices = Vec::new();
                for item in f.array()? {
                    let Value::Array(triple) = item else {
                        return None;
                    };
                    let mut g = Fields(triple.iter());
                    devices.push((g.text()?, g.u8()?, g.text()?));
                    g.end()?;
                }
                Self::SpiBusSolved {
                    peripheral,
                    devices,
                }
            }
            24 => Self::MotorOutputSolved {
                label: f.text()?,
                timer: f.text()?,
                channel: f.u8()?,
                pin: f.text()?,
                alternate_function: f.u8()?,
                protocol: f.text()?,
                psc: f.u32()?,
                arr: f.u32()?,
                ccr: f.u32()?,
                pwm: match f.array()? {
                    [] => None,
                    [Value::Array(triple)] => {
                        let mut g = Fields(triple.iter());
                        let v = (g.u32()?, g.u32()?, g.u32()?);
                        g.end()?;
                        Some(v)
                    }
                    _ => return None,
                },
                idle: match f.array()? {
                    [] => None,
                    [Value::Array(i)] => {
                        let mut g = Fields(i.iter());
                        let v = match g.uint()? {
                            0 => MotorIdle::Low,
                            1 => MotorIdle::High,
                            2 => MotorIdle::PulseUs(g.u32()?),
                            _ => return None,
                        };
                        g.end()?;
                        Some(v)
                    }
                    _ => return None,
                },
                resolution_ps: match f.array()? {
                    [] => None,
                    [Value::Uint(ps)] => Some(*ps),
                    _ => return None,
                },
            },
            25 => Self::MotorTimingCheck {
                label: f.text()?,
                kernel_hz: f.uint()?,
                period_ps: f.uint()?,
                high_ps: f.uint()?,
                nominal_period_ps: f.uint()?,
                nominal_high_ps: f.uint()?,
                holds: f.flag()?,
            },
            26 => Self::MotorWitness {
                label: f.text()?,
                observer: f.text()?,
                period_ns: f.uint()?,
                high_ns: f.uint()?,
                grain_ns: f.uint()?,
                quantum_ns: f.uint()?,
                status: f.text()?,
                rises: f.uint()?,
                falls: f.uint()?,
                first_ns: f.opt()?,
                last_ns: f.opt()?,
                end_ns: f.uint()?,
                span_ns: f.opt()?,
                off_grid: f.uint()?,
                batched: f.uint()?,
                anchor_ns: f.opt()?,
                edges_before: f.uint()?,
                edges_after: f.uint()?,
                cen_rises: f.uint()?,
            },
            27 => Self::SerialPortSolved {
                label: f.text()?,
                index: f.u32()?,
                peripheral: f.text()?,
                tx: f.text()?,
                tx_af: f.u8()?,
                rx: f.text()?,
                rx_af: f.u8()?,
                role: f.text()?,
                framing: {
                    let mut g = Fields(f.array()?.iter());
                    let framing = SerialFraming {
                        baud: g.u32()?,
                        data_bits: g.u8()?,
                        parity: g.text()?,
                        stop_bits: g.u8()?,
                        inverted: g.flag()?,
                    };
                    g.end()?;
                    framing
                },
                kernel_hz: f.uint()?,
                brr: f.u32()?,
                actual_millibaud: f.uint()?,
                error_ppm: f.int()?,
            },
            28 => Self::SerialFramingCheck {
                label: f.text()?,
                kernel_hz: f.uint()?,
                brr: f.u32()?,
                actual_millibaud: f.uint()?,
                error_ppm: f.int()?,
                tolerance: f.opt_with(|g| Some((g.text()?, g.u8()?, g.u32()?)))?,
                holds: f.flag()?,
                tags_needed: f.texts()?,
                tags: f.opt_with(|g| {
                    Some(SerialTags {
                        writes: g.uint()?,
                        missing: g.texts()?,
                        unexpected: g.texts()?,
                    })
                })?,
            },
            29 => Self::SerialWitness {
                label: f.text()?,
                capture: f.opt_with(|g| {
                    Some(SerialCapture {
                        patterns: g.uint()?,
                        cut: g.uint()?,
                        echoed: g.uint()?,
                        stray: g.flag()?,
                        len: g.uint()?,
                    })
                })?,
                frame: match f.next()? {
                    Value::Bytes(b) => <[u8; SERIAL_FRAME_LEN]>::try_from(b.as_slice()).ok()?,
                    _ => return None,
                },
                echo: f.text()?,
                rx_at_injection: f.mux()?,
                tx_at_end: f.mux()?,
                tx_at_send: f.mux()?,
                injected_ns: f.opt()?,
            },
            30 => Self::JudgeStop {
                timer: f.text()?,
                anchor_ns: f.uint()?,
                at_ns: f.uint()?,
                read_back: {
                    let mut g = Fields(f.array()?.iter());
                    let mut out = Vec::new();
                    while g.0.len() > 0 {
                        out.push(g.reading()?);
                    }
                    out
                },
                cr1_after: f.opt_u32()?,
            },
            32 => Self::RecoveryBound {
                boot_word_ns: f.opt()?,
                margin_ns: f.uint()?,
                bound_ns: f.opt()?,
            },
            33 => Self::PinGpioDeclared {
                net: f.text()?,
                pin: f.text()?,
                output: f.flag()?,
                level: f.u8()?,
                active: f.opt_u8()?,
                held: f.flag()?,
            },
            34 => Self::PinGpioWitness {
                net: f.text()?,
                status: f.text()?,
                bsrr: f.opt_u32()?,
                ordered: f.opt_flag()?,
                level: f.opt_u8()?,
            },
            35 => Self::PinAdcDeclared {
                net: f.text()?,
                pin: f.text()?,
                peripheral: f.text()?,
                channel: f.u8()?,
                bits: f.u8()?,
                microvolts: f.uint()?,
                held: f.flag()?,
                code: f.u32()?,
            },
            36 => Self::PinAdcWitness {
                net: f.text()?,
                status: f.text()?,
                reads: f.uint()?,
                code: f.opt_u32()?,
            },
            37 => Self::MotorLevelWitness {
                label: f.text()?,
                level: f.u8()?,
                settle_ns: f.opt()?,
                rises_after: f.uint()?,
                falls_after: f.uint()?,
                end_level: f.opt_u8()?,
                status: f.text()?,
            },
            38 => Self::ForeignInput {
                label: f.text()?,
                peripheral: f.text()?,
                spacing_ns: f.uint()?,
                file: f.bytes()?,
            },
            39 => {
                let mut entries = Vec::new();
                for item in f.array()? {
                    let Value::Array(e) = item else {
                        return None;
                    };
                    let mut g = Fields(e.iter());
                    entries.push((g.uint()?, g.text()?, g.text()?, g.text()?));
                    g.end()?;
                }
                Self::Stimuli { entries }
            }
            40 => Self::SerialCaptured {
                label: f.text()?,
                end_ns: f.uint()?,
                bytes: f.bytes()?,
            },
            31 => {
                let sag = BatterySag {
                    series_milliohms: f.u32()?,
                    from_boot_word_ns: f.int()?,
                };
                let start_ns = f.uint()?;
                let v_open_microvolts = f.uint()?;
                let healthy_milliohms = f.u32()?;
                let mut sinks = Vec::new();
                for item in f.array()? {
                    let Value::Array(sink) = item else {
                        return None;
                    };
                    let mut g = Fields(sink.iter());
                    sinks.push(SagSink {
                        name: g.text()?,
                        input_milliwatts: g.uint()?,
                        shutdown_millivolts: g.uint()?,
                        regulation_millivolts: g.uint()?,
                    });
                    g.end()?;
                }
                Self::BatterySagDeclared {
                    sag,
                    start_ns,
                    v_open_microvolts,
                    healthy_milliohms,
                    sinks,
                    capacitance_picofarads: f.uint()?,
                    static_microvolts: f.uint()?,
                    solved_minimum_microvolts: f.uint()?,
                    binding: f.text()?,
                }
            }
            _ => return None,
        };
        f.end()?;
        Some(record)
    }

    /// The field-level rules; the writer and the reader both apply them.
    pub fn check(&self) -> Result<(), &'static str> {
        let names = |v: &[String]| v.iter().all(|s| name_ok(s));
        let sorted = |v: &[String]| v.windows(2).all(|w| w[0] < w[1]);
        match self {
            Self::Scenario {
                name,
                expect_pass,
                expect_findings,
                overrides,
                ..
            } => {
                if !name_ok(name) {
                    return Err("scenario name is not 1 to 64 printable ASCII bytes");
                }
                if !names(expect_findings) || !sorted(expect_findings) {
                    return Err("expected finding kinds are not sorted unique names");
                }
                if *expect_pass != expect_findings.is_empty() {
                    return Err(
                        "a passing scenario expects no finding, a failing one at least one",
                    );
                }
                let ids_ok = overrides.iter().all(|o| match o {
                    Override::PullUpOhms { bus, .. } | Override::LineHeldLow { bus, .. } => {
                        name_ok(bus)
                    }
                    Override::MaxCapacitance { net, .. } => name_ok(net),
                    Override::Droop(d) => name_ok(&d.domain),
                    Override::SpiDeviceAbsent { reference } => name_ok(reference),
                    Override::SerialRxOpen { port } => name_ok(port),
                    Override::BatterySag(_) => true,
                    Override::RailOutOfBand { domain } => name_ok(domain),
                    Override::PinHeld { net, .. } => name_ok(net),
                    Override::AnalogValue { net, .. } => name_ok(net),
                });
                if !ids_ok {
                    return Err("an override's id is not 1 to 64 printable ASCII bytes");
                }
                if overrides.iter().any(|o| {
                    matches!(o, Override::AnalogValue { millivolts, .. }
                        if *millivolts > ANALOG_MAX_MILLIVOLTS)
                }) {
                    return Err("an analog_value override is above the ADC's 3300 mV reference");
                }
                Ok(())
            }
            Self::Solved { .. } => Ok(()),
            Self::RiseComputed {
                bus,
                net,
                rise_30_70_ps,
                complete,
                ceiling_ps,
                exceeds,
                window,
            } => {
                if !name_ok(bus) || !name_ok(net) {
                    Err("bus or net name is not 1 to 64 printable ASCII bytes")
                } else if *exceeds != (!complete || rise_30_70_ps > ceiling_ps) {
                    Err("exceeds is not the rise against the ceiling")
                } else {
                    window.check()
                }
            }
            Self::LineStuckHigh {
                bus, net, window, ..
            } => {
                if name_ok(bus) && name_ok(net) {
                    window.check()
                } else {
                    Err("bus or net name is not 1 to 64 printable ASCII bytes")
                }
            }
            Self::BusFault {
                bus,
                fault,
                source,
                from,
                nets,
                firmware_sees,
            } => {
                if !name_ok(bus) || !name_ok(fault) {
                    Err("bus or fault name is not 1 to 64 printable ASCII bytes")
                } else if !text_ok(source, 1, 128) || !text_ok(firmware_sees, 0, 1024) {
                    Err("fault source or rendering is not printable ASCII of its length")
                } else if from.is_empty() || !from.iter().all(|s| text_ok(s, 1, 64)) {
                    Err("fault causes are not 1 to 64 printable ASCII bytes each")
                } else if nets.is_empty() || !names(nets) || !sorted(nets) {
                    Err("fault nets are not sorted unique names")
                } else {
                    Ok(())
                }
            }
            Self::BootObserved {
                boot_word,
                instructions_observed,
                pc,
                executable,
                brown_out,
                flash_access,
                ..
            } => {
                let ordered = executable.iter().all(|&(s, e)| s < e)
                    && executable.windows(2).all(|w| w[0].1 <= w[1].0);
                // One read gives all four (`AET-WO-2205` adversarial 1): a
                // `pc` absent beside a boot that was read hides where it ended.
                let read = [
                    boot_word.observed.is_some(),
                    instructions_observed.is_some(),
                    pc.is_some(),
                    brown_out.observed.is_some(),
                ];
                if executable.len() > MAX_EXECUTABLE_SEGMENTS || !ordered {
                    Err("executable segments are not at most 16 sorted, disjoint, nonempty ranges")
                } else if read.contains(&true) && read.contains(&false) {
                    Err(
                        "the boot word, the instruction count, pc and the brown-out word are not all read or all unread",
                    )
                } else if brown_out.expected & !brown_out.mask != 0 {
                    Err("the brown-out word's expected level lies outside its mask")
                } else if flash_access.is_some() && !read[0] {
                    Err("a boot that read nothing holds flash controller accesses")
                } else if flash_access
                    .as_deref()
                    .is_some_and(|v| !witness_lines_ok(v))
                {
                    Err(
                        "flash controller accesses are not at most 16 lines of 1 to 256 printable ASCII bytes",
                    )
                } else {
                    Ok(())
                }
            }
            Self::RegisterObserved { name, .. } => {
                if name_ok(name) {
                    Ok(())
                } else {
                    Err("register name is not 1 to 64 printable ASCII bytes")
                }
            }
            Self::I2cCheck {
                reference,
                status,
                step,
                write,
                width,
                expected,
                mask,
                ..
            } => {
                step_ok(step, write, *width, *expected, *mask)?;
                if !name_ok(reference) {
                    Err("device reference is not 1 to 64 printable ASCII bytes")
                } else if status.as_deref().is_some_and(|s| !name_ok(s)) {
                    Err("status is not 1 to 64 printable ASCII bytes")
                } else {
                    Ok(())
                }
            }
            Self::I2cWitness {
                peripheral,
                reference,
                device,
                step,
                expected,
                observed,
                absent,
            } => {
                if step_index(step).is_none() {
                    Err("step is not identity, a configuration or data")
                } else if absent.len() > MAX_WITNESS_LINES || absent.iter().any(|&a| a > 0x7F) {
                    Err("absent addresses are not at most 16 seven-bit addresses")
                } else if !(name_ok(peripheral) && name_ok(reference) && name_ok(device)) {
                    Err("peripheral, reference or device is not 1 to 64 printable ASCII bytes")
                } else if !witness_lines_ok(expected)
                    || observed.as_deref().is_some_and(|o| !witness_lines_ok(o))
                {
                    Err("witness lines are not at most 16 lines of 1 to 256 printable ASCII bytes")
                } else {
                    Ok(())
                }
            }
            Self::Verdict { pass, findings } => {
                let pairs_sorted = findings
                    .windows(2)
                    .all(|w| (&w[0].0, &w[0].1) < (&w[1].0, &w[1].1));
                if !findings
                    .iter()
                    .all(|(k, s, n)| name_ok(k) && SEVERITIES.contains(&s.as_str()) && *n > 0)
                {
                    Err("a finding is not a kind, a severity of error or info, and a count above 0")
                } else if !pairs_sorted {
                    Err("findings are not sorted and unique by kind and severity")
                } else if *pass != findings.iter().all(|(_, s, _)| s == "info") {
                    Err("pass is not the absence of a finding above info")
                } else {
                    Ok(())
                }
            }
            Self::I2cBusSolved {
                bus,
                peripheral,
                lines,
                devices,
            } => {
                let ends = |l: &BusLine, line: &str| {
                    l.signal.strip_prefix(peripheral.as_str()) == Some(line)
                };
                if !name_ok(bus) || !name_ok(peripheral) {
                    Err("bus or peripheral name is not 1 to 64 printable ASCII bytes")
                } else if !lines
                    .iter()
                    .all(|l| name_ok(&l.signal) && name_ok(&l.net) && name_ok(&l.pin))
                {
                    Err("a line's signal, net or pin is not 1 to 64 printable ASCII bytes")
                } else if !(lines.len() == 2 && ends(&lines[0], "_SCL") && ends(&lines[1], "_SDA"))
                {
                    Err("a solved bus's lines are not its peripheral's SCL, then its SDA")
                } else if lines[0].net == lines[1].net || lines[0].pin == lines[1].pin {
                    Err("a solved bus's two lines share a net or a pin")
                } else if !lines.iter().all(|l| {
                    split_pin(&l.pin).is_some() && l.alternate_function <= MAX_ALTERNATE_FUNCTION
                }) {
                    Err(
                        "a line's pin is not P, a port letter and 0 to 15, or its alternate function is over 15",
                    )
                } else if !devices.iter().all(|(r, _)| name_ok(r))
                    || !devices.windows(2).all(|w| w[0].0 < w[1].0)
                {
                    Err("a solved bus's devices are not sorted unique references")
                } else {
                    Ok(())
                }
            }
            Self::RailDeclared {
                droop,
                nominal_millivolts,
                decoupling_picofarads,
                ..
            } => {
                if !name_ok(&droop.droop.domain) {
                    Err("a droop's domain is not 1 to 64 printable ASCII bytes")
                } else if droop.bor_level > crate::power::MAX_BOR_LEVEL {
                    // The level is the specification's (`AET-WO-2310`); the
                    // part has levels 0 to 3 only.
                    Err("a declared rail's BOR level is not 0 to 3")
                } else if droop.droop.floor_millivolts >= *nominal_millivolts
                    || *decoupling_picofarads == 0
                {
                    Err("a declared rail's floor is not below its nominal, or it has no decoupling")
                } else {
                    Ok(())
                }
            }
            Self::RailCrossing { domain, level, .. } => {
                if name_ok(domain) && name_ok(level) {
                    Ok(())
                } else {
                    Err("a crossing's domain or level is not 1 to 64 printable ASCII bytes")
                }
            }
            Self::ResetRendered {
                cause, rendering, ..
            } => {
                if cause.is_empty() || !names(cause) || !sorted(cause) {
                    Err("a reset's causes are not sorted unique threshold names")
                } else if !text_ok(rendering, 1, 1024) {
                    Err("a reset's rendering is not 1 to 1024 printable ASCII bytes")
                } else {
                    Ok(())
                }
            }
            Self::FirmwareObserved { witness, .. } => {
                if witness.as_deref().is_some_and(|o| !witness_lines_ok(o)) {
                    Err("witness lines are not at most 16 lines of 1 to 256 printable ASCII bytes")
                } else {
                    Ok(())
                }
            }
            Self::ExpectedCounts { counts } => {
                if counts.is_empty() || !counts.iter().all(|(k, _)| name_ok(k)) {
                    Err("expected counts are not one or more finding kinds")
                } else if !counts.windows(2).all(|w| w[0].0 < w[1].0) {
                    Err("expected counts are not sorted unique by kind")
                } else {
                    Ok(())
                }
            }
            Self::SpiCheck {
                reference,
                peripheral,
                select,
                status,
                step,
                write,
                width,
                expected,
                mask,
                ..
            } => {
                step_ok(step, write, *width, *expected, *mask)?;
                if !(name_ok(reference) && name_ok(peripheral) && name_ok(select)) {
                    Err(
                        "device reference, peripheral or select is not 1 to 64 printable ASCII bytes",
                    )
                } else if status.as_deref().is_some_and(|s| !name_ok(s)) {
                    Err("status is not 1 to 64 printable ASCII bytes")
                } else {
                    Ok(())
                }
            }
            Self::SpiWitness {
                peripheral,
                reference,
                device,
                step,
                expected,
                observed,
                unrouted,
            } => {
                if step_index(step).is_none() {
                    Err("step is not identity, a configuration or data")
                } else if !(name_ok(peripheral) && name_ok(reference) && name_ok(device)) {
                    Err("peripheral, reference or device is not 1 to 64 printable ASCII bytes")
                } else if !witness_lines_ok(expected)
                    || observed.as_deref().is_some_and(|o| !witness_lines_ok(o))
                    || !witness_lines_ok(unrouted)
                {
                    Err("witness lines are not at most 16 lines of 1 to 256 printable ASCII bytes")
                } else {
                    Ok(())
                }
            }
            Self::SpiBusSolved {
                peripheral,
                devices,
            } => {
                let addresses_in_order = devices
                    .iter()
                    .enumerate()
                    .all(|(i, d)| usize::from(d.1) == i);
                let mut selects: Vec<&str> = devices.iter().map(|d| d.2.as_str()).collect();
                selects.sort_unstable();
                selects.dedup();
                if !name_ok(peripheral) || !peripheral.starts_with("SPI") {
                    Err("a solved SPI controller is not named SPIn")
                } else if devices.is_empty() || devices.len() > MAX_WITNESS_LINES {
                    Err("a solved SPI controller has 1 to 16 devices")
                } else if !devices
                    .iter()
                    .all(|d| name_ok(&d.0) && split_pin(&d.2).is_some())
                {
                    Err("a solved SPI device's reference or chip select is malformed")
                } else if !addresses_in_order || selects.len() != devices.len() {
                    Err(
                        "a solved SPI controller's addresses are not 0 to n in order, or two devices share a chip select",
                    )
                } else {
                    Ok(())
                }
            }
            Self::MotorOutputSolved {
                label,
                timer,
                channel,
                pin,
                alternate_function,
                protocol,
                psc,
                arr,
                ccr,
                pwm,
                idle,
                resolution_ps,
            } => {
                let timer_ok = timer
                    .strip_prefix("TIM")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
                if !name_ok(label) || !timer_ok || split_pin(pin).is_none() {
                    Err("a solved motor output's label, timer or pin is malformed")
                } else if !(1..=4).contains(channel) || *alternate_function > 15 {
                    Err("a solved motor output's channel is not 1 to 4 or its AF above 15")
                } else if !MOTOR_PROTOCOLS.contains(&protocol.as_str())
                    || pwm.is_some() != (protocol == "pwm")
                    || idle.is_some() != (protocol == "pwm")
                    || resolution_ps.is_some() != (protocol == "pwm")
                    || *resolution_ps == Some(0)
                {
                    Err(
                        "a solved motor output's protocol is unknown, or its PWM timing, idle or tick is not present exactly for PWM",
                    )
                } else if *psc > 0xFFFF || *arr == 0 || *arr > 0xFFFF {
                    Err("a solved motor output's PSC or ARR is outside the timer")
                } else if !match idle {
                    // An idle level is CCR 0 (low) or ARR + 1 (high), RM0433
                    // Rev 9 p. 1558 (`AET-WO-1211a`); every other word is
                    // inside the period.
                    Some(MotorIdle::Low) => *ccr == 0,
                    Some(MotorIdle::High) => u64::from(*ccr) == u64::from(*arr) + 1,
                    Some(MotorIdle::PulseUs(_)) | None => *ccr != 0 && ccr < arr,
                } {
                    Err(
                        "a solved motor output's CCR is not its idle's: 0 for low, ARR + 1 for high, else 1 to ARR - 1",
                    )
                } else {
                    Ok(())
                }
            }
            Self::MotorTimingCheck {
                label, kernel_hz, ..
            } => {
                if !name_ok(label) || *kernel_hz == 0 {
                    Err("a motor timing check's label is malformed or its timer clock 0")
                } else {
                    Ok(())
                }
            }
            Self::MotorWitness {
                label,
                observer,
                grain_ns,
                quantum_ns,
                status,
                anchor_ns,
                edges_before,
                edges_after,
                cen_rises,
                ..
            } => {
                if !name_ok(label) || !name_ok(observer) {
                    Err("a motor witness's label or observer is malformed")
                } else if !MOTOR_STATUSES.contains(&status.as_str()) {
                    Err("a motor witness's status is not one the judge gives")
                } else if *grain_ns == 0 || *quantum_ns == 0 {
                    Err("a motor witness's grain or quantum is 0")
                } else if anchor_ns.is_some() != (*cen_rises > 0)
                    || (anchor_ns.is_none() && (*edges_before > 0 || *edges_after > 0))
                {
                    Err(
                        "a motor witness names an anchor without a CEN rise, a rise without an anchor, or edges outside a window it does not have",
                    )
                } else {
                    Ok(())
                }
            }
            Self::SerialPortSolved {
                label,
                peripheral,
                tx,
                tx_af,
                rx,
                rx_af,
                role,
                framing,
                kernel_hz,
                brr,
                ..
            } => {
                let pins = split_pin(tx).is_some() && split_pin(rx).is_some() && tx != rx;
                if !name_ok(label) || !name_ok(peripheral) || !pins {
                    Err(
                        "a solved serial port's label, peripheral or pins are malformed, or its TX and RX share a pin",
                    )
                } else if *tx_af > MAX_ALTERNATE_FUNCTION || *rx_af > MAX_ALTERNATE_FUNCTION {
                    Err("a solved serial port's alternate function is above 15")
                } else if !SERIAL_ROLES.contains(&role.as_str())
                    || !SERIAL_PARITIES.contains(&framing.parity.as_str())
                {
                    Err("a solved serial port's role or parity is not one a specification declares")
                } else if framing.baud == 0
                    || !(7..=9).contains(&framing.word_bits())
                    || !(1..=2).contains(&framing.stop_bits)
                {
                    Err(
                        "a solved serial port's framing is not a baud above 0, a 7- to 9-bit word and 1 or 2 stop bits",
                    )
                } else if *kernel_hz == 0 || !(16..=0xFFFF).contains(brr) {
                    Err("a solved serial port's kernel clock is 0 or its BRR outside 16 to 0xFFFF")
                } else {
                    Ok(())
                }
            }
            Self::SerialFramingCheck {
                label,
                kernel_hz,
                tolerance,
                tags_needed,
                tags,
                ..
            } => {
                let canonical = |v: &[String]| {
                    let mut at = SERIAL_TAGS.iter();
                    v.iter().all(|t| at.any(|s| s == t))
                };
                if !name_ok(label) || *kernel_hz == 0 {
                    Err("a serial framing check's label is malformed or its kernel clock 0")
                } else if tolerance
                    .as_ref()
                    .is_some_and(|(table, m, _)| !text_ok(table, 1, 64) || *m > 2)
                {
                    Err("a serial framing check's tolerance row is malformed")
                } else if !canonical(tags_needed) {
                    Err("a serial framing check's needed tags are not tags, in order, once each")
                } else if tags.as_ref().is_some_and(|j| {
                    !j.missing.iter().all(|m| tags_needed.contains(m))
                        || !canonical(&j.missing)
                        || j.unexpected.len() > MAX_SERIAL_UNEXPECTED
                        || !j
                            .unexpected
                            .iter()
                            .all(|x| text_ok(x, 1, MAX_SERIAL_UNEXPECTED_BYTES))
                }) {
                    Err(
                        "a serial framing check's tag judgement names a tag the framing does not need as missing, or too many or malformed unexpected ones",
                    )
                } else {
                    Ok(())
                }
            }
            Self::SerialWitness {
                label,
                capture,
                echo,
                rx_at_injection,
                tx_at_end,
                tx_at_send,
                injected_ns,
                ..
            } => {
                let mux_ok = |m: &Option<(u8, u8)>| {
                    m.is_none_or(|(mode, af)| mode <= 3 && af <= MAX_ALTERNATE_FUNCTION)
                };
                if !name_ok(label) {
                    Err("a serial witness's label is malformed")
                } else if !SERIAL_ECHO_STATUSES.contains(&echo.as_str()) {
                    Err("a serial witness's echo is not one the judge gives")
                } else if !mux_ok(rx_at_injection) || !mux_ok(tx_at_end) || !mux_ok(tx_at_send) {
                    Err("a serial witness's pin mode is above 3 or its alternate function above 15")
                } else if capture.is_some_and(|c| !capture_fits(&c, label)) {
                    Err(
                        "a serial witness's counts do not fit its capture's length and its pattern's",
                    )
                } else if injected_ns.is_some() != rx_at_injection.is_some() {
                    Err(
                        "a serial witness names its injection's instant without the RX pin's mux there, or the mux without the instant",
                    )
                } else {
                    Ok(())
                }
            }
            Self::JudgeStop {
                timer,
                anchor_ns,
                at_ns,
                read_back,
                ..
            } => {
                let tim = timer
                    .strip_prefix("TIM")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
                if !tim {
                    Err("a judge's stop does not name a timer")
                } else if at_ns <= anchor_ns {
                    Err("a judge's stop is not after its anchor")
                } else if read_back.is_empty() || read_back.len() > MAX_STOP_REGISTERS {
                    Err("a judge's stop reads back no register, or too many")
                } else {
                    Ok(())
                }
            }
            // The bound is the instant plus the margin, and only the ruled
            // margin; the probes are placed against it by
            // `campaign::check_droop_records`.
            Self::RecoveryBound {
                boot_word_ns,
                margin_ns,
                bound_ns,
            } => {
                if *margin_ns != crate::power::RECOVERY_MARGIN_NS {
                    Err("a recovery bound's margin is not the ruled one quantum")
                } else if *bound_ns != boot_word_ns.and_then(crate::power::recovery_bound) {
                    Err(
                        "a recovery bound is not its undisturbed boot word's instant plus the margin",
                    )
                } else {
                    Ok(())
                }
            }
            // The shape of each pin-device record alone (`AET-WO-1211c`); the
            // joins to each other and to the scenario are
            // `campaign::check_pin_device_records`.
            Self::PinGpioDeclared {
                net,
                pin,
                output,
                level,
                active,
                held,
            } => {
                if !name_ok(net) || split_pin(pin).is_none() {
                    Err("a declared GPIO signal's net or pin is malformed")
                } else if *level > 1 || active.is_some_and(|a| a > 1) {
                    Err("a declared GPIO signal's level or active level is not 0 or 1")
                } else if *output && (active.is_some() || *held) {
                    Err("a declared GPIO output has an active level or is held")
                } else if !*output && active.is_none() {
                    Err("a declared GPIO input has no active level")
                } else {
                    Ok(())
                }
            }
            Self::PinGpioWitness {
                net,
                status,
                bsrr,
                ordered,
                level,
            } => {
                if !name_ok(net) {
                    Err("a GPIO witness's net is malformed")
                } else if !GPIO_STATUSES.contains(&status.as_str()) {
                    Err("a GPIO witness's status is not one the judge gives")
                } else if level.is_some_and(|l| l > 1) {
                    Err("a GPIO witness's level is not 0 or 1")
                } else if level.is_some() && (bsrr.is_some() || ordered.is_some()) {
                    Err("a GPIO witness holds both an output's fields and an input's")
                } else if status == "unreadable"
                    && (bsrr.is_some() || ordered.is_some() || level.is_some())
                {
                    Err("an unreadable GPIO witness holds a reading")
                } else {
                    Ok(())
                }
            }
            Self::PinAdcDeclared {
                net,
                pin,
                peripheral,
                channel,
                bits,
                microvolts,
                code,
                ..
            } => {
                if !name_ok(net) || split_pin(pin).is_none() {
                    Err("a declared ADC input's net or pin is malformed")
                } else if peripheral != "ADC1" && peripheral != "ADC3" {
                    Err("a declared ADC input's peripheral is not ADC1 or ADC3")
                } else if *channel > 19 {
                    Err("a declared ADC input's channel is above 19")
                } else if *bits != 12 {
                    Err("a declared ADC input's resolution is not 12 bits")
                } else if *microvolts > u64::from(ANALOG_MAX_MILLIVOLTS) * 1000 {
                    Err("a declared ADC input's voltage is above the ADC's 3300 mV reference")
                } else if *code != adc_code(*microvolts, *bits) {
                    Err("a declared ADC input's code is not the one its voltage reads as")
                } else {
                    Ok(())
                }
            }
            Self::PinAdcWitness {
                net,
                status,
                reads,
                code,
            } => {
                if !name_ok(net) {
                    Err("an ADC witness's net is malformed")
                } else if !ADC_STATUSES.contains(&status.as_str()) {
                    Err("an ADC witness's status is not one the judge gives")
                } else if code.is_none() != (status == "unwitnessed") {
                    Err("an ADC witness has a code exactly when it is not unwitnessed")
                } else if code.is_some() && *reads == 0 {
                    Err("an ADC witness matched a code from no read")
                } else {
                    Ok(())
                }
            }
            Self::MotorLevelWitness {
                label,
                level,
                settle_ns,
                rises_after,
                falls_after,
                end_level,
                status,
            } => {
                let holds = settle_ns.is_some()
                    && *rises_after == 0
                    && *falls_after == 0
                    && *end_level == Some(*level);
                if !name_ok(label) {
                    Err("a level witness's label is malformed")
                } else if *level > 1 || end_level.is_some_and(|l| l > 1) {
                    Err("a level witness's level or end level is not 0 or 1")
                } else if !LEVEL_STATUSES.contains(&status.as_str()) {
                    Err("a level witness's status is not one the judge gives")
                } else if (status == "witnessed") != holds {
                    Err(
                        "a level witness is witnessed exactly when it settled, saw no edge after and ended at its level",
                    )
                } else if status == "unanchored" && settle_ns.is_some() {
                    Err("an unanchored level witness has a settle time")
                } else if status == "unreadable" && end_level.is_some() {
                    Err("an unreadable level witness has an end level")
                } else {
                    Ok(())
                }
            }
            Self::ForeignInput {
                label,
                peripheral,
                spacing_ns,
                file,
            } => {
                if !name_ok(label) || !name_ok(peripheral) {
                    Err("a foreign input's port label or peripheral is malformed")
                } else if *spacing_ns == 0 {
                    Err("a foreign input's byte spacing is 0")
                } else if file.is_empty() || file.len() > MAX_FOREIGN_BYTES {
                    Err("a foreign input's file is empty or over MAX_FOREIGN_BYTES")
                } else {
                    Ok(())
                }
            }
            Self::Stimuli { entries } => {
                if entries.len() > MAX_STIMULI {
                    Err("a run's stimuli are over MAX_STIMULI")
                } else if !entries.windows(2).all(|w| w[0].0 <= w[1].0) {
                    Err("a run's stimuli are not in time order")
                } else if !entries
                    .iter()
                    .all(|(_, d, p, v)| name_ok(d) && name_ok(p) && name_ok(v))
                {
                    Err("a stimulus's device, property or value is malformed")
                } else {
                    Ok(())
                }
            }
            Self::SerialCaptured { label, bytes, .. } => {
                if !name_ok(label) {
                    Err("a capture's port label is malformed")
                } else if bytes.len() > MAX_FOREIGN_BYTES {
                    Err("a capture is over MAX_FOREIGN_BYTES")
                } else {
                    Ok(())
                }
            }
            // The shape only; the chain is re-judged against the scenario by
            // `campaign::check_battery_sag_records`.
            Self::BatterySagDeclared { sinks, binding, .. } => {
                let mut seen: Vec<&str> = sinks.iter().map(|k| k.name.as_str()).collect();
                seen.sort_unstable();
                seen.dedup();
                if sinks.is_empty()
                    || sinks.len() > crate::battery::MAX_SINKS
                    || seen.len() != sinks.len()
                    || !sinks.iter().all(|k| name_ok(&k.name))
                {
                    Err("a battery sag's sinks are not 1 to 8 distinct names")
                } else if !sinks.iter().any(|k| k.name == *binding) {
                    Err("a battery sag's binding floor names no sink")
                } else {
                    Ok(())
                }
            }
        }
    }
}

/// Whether a capture's counts fit its length, for the pattern of a port
/// labelled `label` (`P`, the label's bytes and [`SERIAL_TAIL_LEN`]): at
/// most the whole frame after the patterns; a stray capture read no cut and
/// no frame, and holds its whole patterns; otherwise its length is the whole
/// patterns, each cut one's 1 to `P - 1` bytes, and the frame's bytes.
fn capture_fits(c: &SerialCapture, label: &str) -> bool {
    let p = (label.len() + SERIAL_TAIL_LEN) as u64;
    let whole = c.patterns.checked_mul(p);
    if c.echoed > SERIAL_FRAME_LEN as u64 {
        return false;
    }
    if c.stray {
        return c.cut == 0 && c.echoed == 0 && whole.is_some_and(|w| w <= c.len);
    }
    let least = whole
        .and_then(|w| w.checked_add(c.cut))
        .and_then(|n| n.checked_add(c.echoed));
    let most = whole
        .zip(c.cut.checked_mul(p - 1))
        .and_then(|(w, k)| w.checked_add(k))
        .and_then(|n| n.checked_add(c.echoed));
    least
        .zip(most)
        .is_some_and(|(lo, hi)| lo <= c.len && c.len <= hi)
}
