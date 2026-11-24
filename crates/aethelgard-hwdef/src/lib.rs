//! ArduPilot's board definition, `hwdef.dat`, generated from a
//! [`SolvedTopology`] and the parts tables alone (`AET-WO-4400b`, `RUL-16`).
//! Unmodified ArduPilot `Copter-4.7.2` (`fb773567`) builds it as an untracked
//! board directory; `aethelgard check pinout` then judges the ELF it links,
//! sharing no code with this crate (`INV-07`, `RUL-20`).
//!
//! Every emitted line carries a comment naming its source: `[T]` the
//! topology, `[P]` a parts-table row, `[C]` a constant of this generator,
//! each named in the file's header. A fact with no source is never emitted
//! as a guess (`INV-06`): the generator refuses the topology by name, or
//! names the line it leaves out. The rules, the constants and what is not
//! emitted are in `README.md`.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use aethelgard_ir::motor::PullFitment;
use aethelgard_ir::orientation::{Orientation, Toward};
use aethelgard_ir::parts::{self, ArduPilotKind};
use aethelgard_ir::power_parts::{self, ArduPilotOscillator, Kind as PowerKind, McuArduPilot};
use aethelgard_ir::serial::{Parity, SerialRole};
use aethelgard_ir::{IrValidationError, SolvedTopology};

/// The ArduPilot release the emitted file is for.
pub const ARDUPILOT: &str = "ArduPilot Copter-4.7.2 (fb773567896014da007bca58358704d4a0d8516f)";

/// `APJ_BOARD_ID`, a number (`4400a` D4: accepted with `GITHUB_ACTIONS`
/// unset, `libraries/AP_HAL/hwdef/scripts/hwdef.py:129-139`). No row of
/// `Tools/AP_Bootloader/board_types.txt` at the tag holds it, and it is
/// outside every reserved range: 1000 to 19999 is ArduPilot's (lines 72-77),
/// and the file allocates nothing above 19,999 (line 619) but ModalAI's
/// 41775-41777 (lines 38-40).
pub const APJ_BOARD_ID: u32 = 24_400;

/// `HAL_STORAGE_SIZE`, the parameter storage in bytes: less than one flash
/// page (`chibios_hwdef.py:849-865`), as step G of the `4400` spike built.
pub const HAL_STORAGE_SIZE: u32 = 16_384;

/// An STM32H7's flash sector, KB (`chibios_hwdef.py:799-801`; DS12117 Rev 10
/// §3.3.1: two banks of eight 128-Kbyte sectors).
pub const H7_SECTOR_KB: i64 = 128;

/// The SWD port, kept on (`AET-WO-4410` (3), review 3 A11): ArduPilot makes an
/// unlisted pin `INPUT FLOATING` (`chibios_hwdef.py:47`, `:145`), which turns
/// SWD off at boot. Each line is ArduPilot's own text (197 boards at the tag)
/// with the pull its script gives the label written out (`:407-413`), which
/// is RM0433's reset pull (Rev 9 §11.4.4 p. 542, `GPIOA_PUPDR` `0x6400 0000`).
/// The label alone picks AF0 (`STM32H753xx.py:247`, `:249`).
// belt-and-suspenders: `aethelgard-pincheck` keeps its own SWD pins, never
// this table (INV-07, RUL-20: the judge shares no table with the generator).
pub const SWD_LINES: [(&str, &str); 2] = [
    ("PA13", "PA13 JTMS-SWDIO SWD PULLUP"),
    ("PA14", "PA14 JTCK-SWCLK SWD PULLDOWN"),
];

/// The timers the system tick may take, in order of preference: TIM5, the
/// H7 default (`stm32h7_mcuconf.h:605-606`), then TIM2, the other 32-bit
/// general-purpose timer (RM0433 Rev 9 §39.1).
pub const SYSTEM_TIMERS: [u8; 2] = [5, 2];

/// The `MODE` token of every `SPIDEV`: the mode the generated firmware drives
/// and `validate` holds every SPI device to (`parts::DRIVEN_SPI_MODE`).
fn spi_mode() -> String {
    format!("MODE{}", parts::DRIVEN_SPI_MODE)
}

/// Every refusal, by name (`INV-06`). None of these is a partial file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("the topology is not a valid SolvedTopology: {0}")]
    InvalidTopology(IrValidationError),
    #[error("{0}")]
    PartsTable(IrValidationError),
    #[error(
        "the power parts table holds no `mcu` row for `{part}`, so FLASH_SIZE_KB and the MCU line have no source"
    )]
    UnknownMcu { part: String },
    #[error(
        "`{part}` is a `{series}` part; this generator knows the flash sectors of `STM32H7xx` alone"
    )]
    UnsupportedSeries { part: String, series: String },
    #[error(
        "the topology's clock has a {hse_hz} Hz oscillator; the MCU row cites ArduPilot's SPI kernel clocks only with OSCILLATOR_HZ {}",
        cited.iter().map(u32::to_string).collect::<Vec<_>>().join(" or ")
    )]
    OscillatorNotCited { hse_hz: u32, cited: Vec<u32> },
    #[error(
        "the topology has no serial port; ArduPilot builds only with a USART or UART (AET-WO-4400b §1.1, measured)"
    )]
    NoSerialPort,
    #[error("no serial port has the role `console`; ArduPilot's SERIAL0 is its console")]
    NoConsole,
    #[error(
        "serial ports `{first}` and `{second}` are both the console; ArduPilot has one SERIAL0"
    )]
    ConsoleTwice { first: String, second: String },
    #[error(
        "serial ports `{first}` and `{second}` are both RC input; ArduPilot attaches one RCIN port and drops the next (AP_SerialManager.cpp:536-541)"
    )]
    RcInputTwice { first: String, second: String },
    #[error("SPI controller `{peripheral}` has no ArduPilot kernel clock in the MCU row")]
    SpiKernelNotCited { peripheral: String },
    #[error("{count} serial ports; ArduPilot's DEFAULT_SERIALn defines run SERIAL0 to SERIAL9")]
    TooManySerialPorts { count: usize },
    #[error(
        "serial port `{port}` is the console and inverted; ArduPilot has no DEFAULT_SERIAL0_OPTIONS"
    )]
    InvertedConsole { port: String },
    #[error(
        "serial port `{port}` ({role}) declares {declared}, not {implied}, the framing the role takes in ArduPilot"
    )]
    FramingNotImplied {
        port: String,
        role: &'static str,
        declared: String,
        implied: String,
    },
    #[error(
        "serial port `{port}`: the console at {baud} baud is not a SERIAL0 baud ArduPilot can hold (the parameter is baud / 1000 and maps back to {mapped}, AP_SerialManager.cpp:172, 737-770)"
    )]
    ConsoleBaud {
        port: String,
        baud: u32,
        mapped: u32,
    },
    #[error(
        "serial port `{port}` at {baud} baud: ArduPilot reads a SERIALn baud of 2000 or less as kbaud (AP_SerialManager.cpp:762-769)"
    )]
    SerialBaud { port: String, baud: u32 },
    #[error("`{net}` is on `{signal}`, a peripheral this generator emits no line for")]
    UnsupportedSignal { net: String, signal: String },
    #[error(
        "`{net}` is a chip select whose name is also the label of an emitted pin; ArduPilot keys pins by label"
    )]
    LabelCollision { net: String },
    #[error(
        "`{net}` is a chip select whose name starts with `{prefix}`; ArduPilot looks such a label up as an alternate function of its pin and refuses it at configure (chibios_hwdef.py:33, 173-178)"
    )]
    LabelTakesAfPrefix { net: String, prefix: String },
    #[error("`{first}` and `{second}` both take the SPIDEV name `{name}`")]
    DuplicateSpidev {
        name: String,
        first: String,
        second: String,
    },
    #[error(
        "SPI controller `{peripheral}` at {hz} Hz is below {floor} Hz, ArduPilot's slowest on its {kernel_hz} Hz kernel (SPIDevice.cpp:259-276 halves kernel / 2 at most seven times), so ArduPilot would clock it faster"
    )]
    SpiBelowArduPilotDivider {
        peripheral: String,
        hz: u64,
        kernel_hz: i64,
        floor: i64,
    },
    #[error("motors take both TIM5 and TIM2, so no system timer is left for ArduPilot")]
    NoSystemTimer,
    #[error("`{net}` is solved on `{pin}`, but no emitted line configures it")]
    PinNotEmitted { net: String, pin: String },
    /// `AET-WO-1211a`: ArduPilot's hwdef has no `GPIO(n)` line for a pin
    /// device's pins until a work order writes it.
    #[error(
        "`{reference}` ({part}) is a pin device; hwdef.dat has no GPIO(n) line this generator writes for its pins (AET-WO-1211a)"
    )]
    GpioNotEmitted { reference: String, part: String },
    /// `AET-WO-4410` (1): ArduPilot reads one analog battery voltage,
    /// `HAL_BATT_VOLT_PIN` (`AP_BattMonitor_Analog.cpp:30-36`).
    #[error(
        "battery senses `{first}` and `{second}` are both solved; ArduPilot's hwdef has one HAL_BATT_VOLT_PIN"
    )]
    SenseTwice { first: String, second: String },
    /// `AET-WO-4410` D6: the scale is written as the exact quotient of two
    /// integers a `float` holds exactly (2^24 and below).
    #[error(
        "battery sense `{net}`: the divider {top_ohms} ohm over {bottom_ohms} ohm has no exact float quotient (both sums at most 16777216, the bottom above 0)"
    )]
    DividerNotExact {
        net: String,
        top_ohms: u32,
        bottom_ohms: u32,
    },
    /// `AET-WO-4410` S2: ArduPilot's H7 build enables ADC12 whatever the
    /// board holds, and ChibiOS then needs its DMA stream, which ArduPilot
    /// allocates only from an ADC1 or ADC2 pin line.
    #[error(
        "battery sense `{net}` is solved on {peripheral}, not ADC1: ArduPilot's H7 build enables ADC12 (stm32h7_mcuconf.h:420) and fails without an ADC1 pin's DMA stream (ChibiOS hal_adc_lld.h:339, measured in AET-WO-4410)"
    )]
    SenseNotOnAdc1 { net: String, peripheral: String },
    /// `AET-WO-4410` (3): PA13 and PA14 are the SWD port's, always emitted.
    #[error(
        "`{net}` is solved on {pin}, a pin of the SWD port this generator always emits (AET-WO-4410)"
    )]
    SwdPinSolved { net: String, pin: String },
    /// `AET-WO-4400d` (`INV-06`): an IMU's rotation is the declared
    /// orientation's, never a default.
    #[error(
        "`{reference}` ({part}) is an IMU with no declared orientation; ArduPilot's IMU line needs its rotation (AET-WO-4400d)"
    )]
    ImuOrientationUndeclared { reference: String, part: String },
    /// `AET-WO-4400d`: the driver's sample frame is a cited fact.
    #[error(
        "`{reference}` ({part}): the parts row cites no ardupilot.frame, so the driver's frame is unknown (AET-WO-4400d)"
    )]
    ImuFrameNotCited { reference: String, part: String },
}

impl Refusal {
    /// The refusal's name, as the CLI's envelope carries it.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidTopology(_) => "InvalidTopology",
            Self::PartsTable(e) => e.kind(),
            Self::UnknownMcu { .. } => "UnknownMcu",
            Self::UnsupportedSeries { .. } => "UnsupportedSeries",
            Self::OscillatorNotCited { .. } => "OscillatorNotCited",
            Self::NoSerialPort => "NoSerialPort",
            Self::NoConsole => "NoConsole",
            Self::RcInputTwice { .. } => "RcInputTwice",
            Self::SpiKernelNotCited { .. } => "SpiKernelNotCited",
            Self::ConsoleTwice { .. } => "ConsoleTwice",
            Self::TooManySerialPorts { .. } => "TooManySerialPorts",
            Self::InvertedConsole { .. } => "InvertedConsole",
            Self::FramingNotImplied { .. } => "FramingNotImplied",
            Self::ConsoleBaud { .. } => "ConsoleBaud",
            Self::SerialBaud { .. } => "SerialBaud",
            Self::UnsupportedSignal { .. } => "UnsupportedSignal",
            Self::LabelCollision { .. } => "LabelCollision",
            Self::LabelTakesAfPrefix { .. } => "LabelTakesAfPrefix",
            Self::DuplicateSpidev { .. } => "DuplicateSpidev",
            Self::SpiBelowArduPilotDivider { .. } => "SpiBelowArduPilotDivider",
            Self::NoSystemTimer => "NoSystemTimer",
            Self::SenseTwice { .. } => "SenseTwice",
            Self::DividerNotExact { .. } => "DividerNotExact",
            Self::SenseNotOnAdc1 { .. } => "SenseNotOnAdc1",
            Self::SwdPinSolved { .. } => "SwdPinSolved",
            Self::PinNotEmitted { .. } => "PinNotEmitted",
            Self::GpioNotEmitted { .. } => "GpioNotEmitted",
            Self::ImuOrientationUndeclared { .. } => "ImuOrientationUndeclared",
            Self::ImuFrameNotCited { .. } => "ImuFrameNotCited",
        }
    }

    /// The net, port, peripheral or part the refusal names, if one.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::UnknownMcu { part } | Self::UnsupportedSeries { part, .. } => Some(part),
            Self::ConsoleTwice { second, .. }
            | Self::RcInputTwice { second, .. }
            | Self::SenseTwice { second, .. } => Some(second),
            Self::SpiKernelNotCited { peripheral } => Some(peripheral),
            Self::InvertedConsole { port }
            | Self::FramingNotImplied { port, .. }
            | Self::ConsoleBaud { port, .. }
            | Self::SerialBaud { port, .. } => Some(port),
            Self::UnsupportedSignal { net, .. }
            | Self::LabelCollision { net }
            | Self::LabelTakesAfPrefix { net, .. }
            | Self::DividerNotExact { net, .. }
            | Self::SenseNotOnAdc1 { net, .. }
            | Self::SwdPinSolved { net, .. }
            | Self::PinNotEmitted { net, .. } => Some(net),
            Self::GpioNotEmitted { reference, .. } => Some(reference),
            Self::DuplicateSpidev { second, .. } => Some(second),
            Self::ImuOrientationUndeclared { reference, .. }
            | Self::ImuFrameNotCited { reference, .. } => Some(reference),
            Self::SpiBelowArduPilotDivider { peripheral, .. } => Some(peripheral),
            Self::InvalidTopology(e) | Self::PartsTable(e) => e.id(),
            Self::OscillatorNotCited { .. }
            | Self::NoSerialPort
            | Self::NoConsole
            | Self::TooManySerialPorts { .. }
            | Self::NoSystemTimer => None,
        }
    }
}

/// The generated file and what it leaves out, each named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hwdef {
    /// `hwdef.dat`'s bytes.
    pub text: String,
    /// Each fact the topology holds that no line states, and why.
    pub not_emitted: Vec<String>,
}

/// One emitted line: the `hwdef.dat` text and its source comment. A line
/// that is itself a comment (`#`, whose text ArduPilot's parser discards,
/// `libraries/AP_HAL/hwdef/scripts/hwdef.py:209-212`) has no source, and is
/// written as it is.
struct Lines(Vec<(String, String)>);

impl Lines {
    fn push(&mut self, line: impl Into<String>, source: impl Into<String>) {
        self.0.push((line.into(), source.into()));
    }

    /// A comment line, written verbatim between the emitted lines: its text
    /// names its own source.
    fn comment(&mut self, text: impl AsRef<str>) {
        self.0.push((format!("# {}", text.as_ref()), String::new()));
    }
}

/// `AP_SerialManager::map_baudrate` (`AP_SerialManager.cpp:737-770`), the
/// value SERIAL0's parameter `DEFAULT_SERIAL0_BAUD / 1000` maps back to
/// (line 172).
fn map_baudrate(rate: u32) -> u32 {
    let rate = if rate == 0 { 57 } else { rate };
    match rate {
        1 => 1200,
        2 => 2400,
        4 => 4800,
        9 => 9600,
        19 => 19200,
        38 => 38400,
        57 => 57600,
        100 => 100_000,
        111 => 111_100,
        115 => 115_200,
        230 => 230_400,
        256 => 256_000,
        460 => 460_800,
        500 => 500_000,
        921 => 921_600,
        1500 => 1_500_000,
        2000 => 2_000_000,
        r if r > 2000 => r,
        r => r.saturating_mul(1000),
    }
}

/// ArduPilot's `af_labels` at the tag (`chibios_hwdef.py:33`): a pin label
/// that starts with one of these, case-sensitive (Python's `startswith`; the
/// parser keeps the label's case, `:2840`), is looked up in the MCU's
/// alternate-function table (`:173-178`), and configure fails when the pin
/// has none (`AET-WO-4400b` A9). A chip select is emitted under its net's
/// name, so a net that starts with one is refused by name.
const AF_LABEL_PREFIXES: [&str; 14] = [
    "USART", "UART", "SPI", "I2C", "SDIO", "SDMMC", "OTG", "JT", "TIM", "CAN", "QUADSPI",
    "OCTOSPI", "ETH", "MCO",
];

/// ArduPilot's protocol for a role (`AP_SerialManager.h:39-62`): MAVLink 2
/// on the console (SERIAL0's own default, `AP_SerialManager.cpp:37-38`) and
/// a MAVLink port, GPS, and RC input for CRSF and SBUS, whose UART framing
/// AP_RCProtocol sets itself (`AP_RCProtocol.cpp:326-352`).
fn protocol(role: SerialRole) -> &'static str {
    match role {
        SerialRole::Console | SerialRole::Mavlink => "SerialProtocol_MAVLink2",
        SerialRole::Gps => "SerialProtocol_GPS",
        SerialRole::RcCrsf | SerialRole::RcSbus => "SerialProtocol_RCIN",
    }
}

/// The framing a role takes in ArduPilot: (data bits, parity, stop bits,
/// inverted, the one baud if the role fixes it). MAVLink, GPS and the
/// console are 8N1, not inverted, at the declared baud: ArduPilot sets no
/// parity or stop bits for them. CRSF and SBUS are AP_RCProtocol's own
/// serial configurations (`AP_RCProtocol.cpp:339-352`): CRSF 8N1, not
/// inverted (its baud is `ir`'s to judge, `serial::contradiction`); SBUS at
/// 100,000 baud, even parity, two stop bits, inverted.
fn implied(role: SerialRole) -> (u8, Parity, u8, bool, Option<u32>) {
    match role {
        SerialRole::RcSbus => (8, Parity::Even, 2, true, Some(100_000)),
        _ => (8, Parity::None, 1, false, None),
    }
}

/// What ArduPilot's RC input runs on a port of `role`, from its own serial
/// configurations (`AP_RCProtocol.cpp` at the tag: SBUS line 344, CRSF line
/// 351), or `None` for a port that is not RC input.
fn rc_runs(role: SerialRole) -> Option<&'static str> {
    match role {
        SerialRole::RcCrsf => Some("CRSF at 416666 baud, 8N1 (AP_RCProtocol.cpp:351)"),
        SerialRole::RcSbus => Some("SBUS at 100000 baud, 8E2, RX inverted (AP_RCProtocol.cpp:344)"),
        _ => None,
    }
}

fn parity_letter(p: Parity) -> char {
    match p {
        Parity::None => 'N',
        Parity::Even => 'E',
        Parity::Odd => 'O',
    }
}

/// The per-pin speed token for a stored `OSPEEDR` (`chibios_hwdef.py:365-385`:
/// `SPEED_VERYLOW` is `00`, `SPEED_LOW` `01`). `validate` holds every stored
/// value to `0` or `1`.
fn speed_token(ospeedr: u8) -> &'static str {
    if ospeedr == 0 {
        "SPEED_VERYLOW"
    } else {
        "SPEED_LOW"
    }
}

/// A speed SPIDEV states (`chibios_hwdef.py:1473-1477`: `*MHZ` or `*KHZ`),
/// rounded down to a whole kHz, so ArduPilot is never asked for more.
fn spidev_speed(hz: u64) -> String {
    let khz = hz / 1000;
    if khz.is_multiple_of(1000) {
        format!("{}*MHZ", khz / 1000)
    } else {
        format!("{khz}*KHZ")
    }
}

/// The peripheral family of an assignment's signal, from its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    I2c,
    Spi,
    Serial,
    Timer,
}

fn family(signal: &str) -> Option<Family> {
    let head = signal.split('_').next().unwrap_or("");
    let digits = |p: &str| {
        head.strip_prefix(p)
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    };
    if digits("I2C") {
        Some(Family::I2c)
    } else if digits("SPI") {
        Some(Family::Spi)
    } else if digits("USART") || digits("UART") {
        Some(Family::Serial)
    } else if digits("TIM") {
        Some(Family::Timer)
    } else {
        None
    }
}

/// The MCU row: its flash and ArduPilot's names and kernel clocks.
fn mcu_row(part: &str) -> Result<(i64, &'static McuArduPilot), Refusal> {
    let table = power_parts::table().map_err(Refusal::PartsTable)?;
    let row = table.get(part).ok_or_else(|| Refusal::UnknownMcu {
        part: part.to_owned(),
    })?;
    let PowerKind::Mcu(m) = &row.kind else {
        return Err(Refusal::UnknownMcu {
            part: part.to_owned(),
        });
    };
    Ok((m.flash_kilobytes.value, &m.ardupilot))
}

/// Generates `hwdef.dat` from `t` and the embedded parts tables.
pub fn generate(t: &SolvedTopology) -> Result<Hwdef, Refusal> {
    t.validate().map_err(Refusal::InvalidTopology)?;
    let parts_table = parts::table().map_err(Refusal::PartsTable)?;
    let (flash_kb, ap) = mcu_row(&t.mcu)?;
    if ap.series != "STM32H7xx" {
        return Err(Refusal::UnsupportedSeries {
            part: t.mcu.clone(),
            series: ap.series.clone(),
        });
    }
    let osc = ap
        .oscillator(t.clock.hse_hz)
        .ok_or_else(|| Refusal::OscillatorNotCited {
            hse_hz: t.clock.hse_hz,
            cited: ap.oscillators.iter().map(|o| o.oscillator_hz).collect(),
        })?;
    if let Some((reference, d)) = t.pin_devices.iter().next() {
        return Err(Refusal::GpioNotEmitted {
            reference: reference.clone(),
            part: d.part.clone(),
        });
    }
    // Every solved pin, by net: none may be an SWD pin. A sense's pin is its
    // ADC input's (`validate` refuses a disagreement, `AET-WO-1211a`).
    let solved_pins = t
        .assignments
        .iter()
        .map(|(net, a)| (net, &a.pin))
        .chain(t.gpio.iter().map(|(net, g)| (net, &g.pin)))
        .chain(t.adc_inputs.iter().map(|(net, a)| (net, &a.pin)));
    for (net, pin) in solved_pins {
        if SWD_LINES.iter().any(|(swd, _)| swd == pin) {
            return Err(Refusal::SwdPinSolved {
                net: net.clone(),
                pin: pin.clone(),
            });
        }
    }
    let battery = battery(t)?;
    let mut not_emitted = Vec::new();
    let mut l = Lines(Vec::new());
    let speed = |net: &str| speed_token(t.pin_speeds[net]);

    // Every assignment belongs to a family this generator emits.
    for (net, a) in &t.assignments {
        if family(&a.signal).is_none() {
            return Err(Refusal::UnsupportedSignal {
                net: net.clone(),
                signal: a.signal.clone(),
            });
        }
    }
    let labels: BTreeSet<&str> = t
        .assignments
        .values()
        .map(|a| a.signal.as_str())
        .chain(battery.as_ref().map(|_| "BATT_VOLTAGE_SENS"))
        .collect();
    if let Some(net) = t.gpio.keys().find(|n| labels.contains(n.as_str())) {
        return Err(Refusal::LabelCollision { net: net.clone() });
    }
    for net in t.gpio.keys() {
        if let Some(prefix) = AF_LABEL_PREFIXES.iter().find(|p| net.starts_with(*p)) {
            return Err(Refusal::LabelTakesAfPrefix {
                net: net.clone(),
                prefix: (*prefix).to_owned(),
            });
        }
    }

    // The MCU and its memory.
    l.push(
        format!("MCU {} {}", ap.series, ap.mcu),
        format!(
            "[T] mcu {}; [P] its power-parts row's ardupilot names",
            t.mcu
        ),
    );
    l.push(
        format!("OSCILLATOR_HZ {}", t.clock.hse_hz),
        "[T] clock.hse_hz (0: HSI drives the PLLs)",
    );
    l.push(
        format!("FLASH_SIZE_KB {flash_kb}"),
        "[P] the MCU row's flash_kilobytes",
    );
    l.push(format!("APJ_BOARD_ID {APJ_BOARD_ID}"), "[C] APJ_BOARD_ID");
    l.push(
        "define AP_BOOTLOADER_FLASHING_ENABLED 0",
        "[C] no bootloader (4400 D3)",
    );
    l.push(
        "FLASH_RESERVE_START_KB 0",
        "[C] no bootloader: the image starts at 0x08000000",
    );
    l.push(
        format!("define HAL_STORAGE_SIZE {HAL_STORAGE_SIZE}"),
        "[C] HAL_STORAGE_SIZE",
    );
    l.push(
        format!("STORAGE_FLASH_PAGE {}", flash_kb / H7_SECTOR_KB - 2),
        "[P] flash_kilobytes; [C] the last two 128 KB sectors (storage uses page and page+1)",
    );
    for (_, line) in SWD_LINES {
        l.push(
            line,
            "[C] SWD_LINES: the debug port kept on, alternate AF0 with RM0433's reset pull",
        );
    }

    // The system timer, off every motor timer.
    let motor_timers: BTreeSet<&str> = t
        .motor_outputs
        .keys()
        .map(|net| t.assignments[net].peripheral.as_str())
        .collect();
    let timer = SYSTEM_TIMERS
        .iter()
        .find(|n| !motor_timers.contains(format!("TIM{n}").as_str()))
        .ok_or(Refusal::NoSystemTimer)?;
    l.push(
        format!("STM32_ST_USE_TIMER {timer}"),
        "[C] TIM5, else TIM2: a timer no motor output takes",
    );

    serial(t, &mut l, &mut not_emitted, speed)?;
    i2c(t, parts_table, &mut l, &mut not_emitted, speed);
    spi(t, parts_table, osc, &mut l, &mut not_emitted, speed)?;

    // Motor outputs, in the specification's order. Each line's pull follows
    // the output's declared fitment (`AET-WO-4300j` D6): `fitted`, `FLOATING`,
    // no internal pull, since the declared external resistor holds the line
    // (`AET-WO-4300i`, ruled D0 `external`, which reversed `4400b` D3's
    // `PULLDOWN`); `open`, the link not fitted, `PULLDOWN`, the firmware's
    // pull. `FLOATING` is ArduPilot's default for a PWM line with no pull
    // token (`chibios_hwdef.py:387-427`, at the pinned tag); it is written,
    // as on every other line here, so the line states the fact rather than
    // leaning on the default. A comment line above each names the fitment
    // and the bound its resistor was judged against (`AET-WO-4300j` §1 (4)),
    // so whoever reads the file sees why BLHeli passthrough is safe. No
    // `HAL_SERIAL_ESC_COMM_ENABLED` is written.
    let mut motors: Vec<(&String, &aethelgard_ir::SolvedMotorOutput)> =
        t.motor_outputs.iter().collect();
    motors.sort_by_key(|(_, m)| m.index);
    for (net, m) in &motors {
        let a = &t.assignments[*net];
        let p = &m.pull;
        // `to` is `GND`: `validate` refuses any other (`MotorPullNotDown`).
        let (pull, source) = match p.fitment {
            PullFitment::Fitted => {
                l.comment(format!(
                    "{net} pull: fitted, {} {} ohm to GND via link {}; bound {}..{} ohm, checked for BLHeli passthrough idle with it fitted, the ESC threshold a stand-in (AET-WO-4300j R1)",
                    p.resistor, p.ohms, p.link, p.floor_ohms, p.ceiling_ohms
                ));
                (
                    "FLOATING",
                    "no internal pull: the pull-down is the declared resistor (4300i D0)",
                )
            }
            PullFitment::Open => {
                l.comment(format!(
                    "{net} pull: open (link {} not fitted), PULLDOWN in firmware; {} {} ohm, bound {}..{} ohm (AET-WO-4300j)",
                    p.link, p.resistor, p.ohms, p.floor_ohms, p.ceiling_ohms
                ));
                (
                    "PULLDOWN",
                    "pull.fitment open: the link is not fitted, so the firmware pulls down (4300j D6)",
                )
            }
        };
        l.push(
            format!(
                "{} {} {} PWM({}) {} PUSHPULL {pull}",
                a.pin,
                a.signal,
                a.peripheral,
                m.index + 1,
                speed(net)
            ),
            format!(
                "[T] motor_outputs.{net} (index {}), assignments, pin_speeds; {source}",
                m.index
            ),
        );
        not_emitted.push(format!(
            "{net}: its protocol ({}) and PWM range are ArduPilot parameters (MOT_PWM_TYPE, MOT_PWM_MIN/MAX), not hwdef.dat lines; they wait for the defaults.parm work order (D10)",
            m.protocol.name()
        ));
    }
    if !motors.is_empty() {
        not_emitted.push("BIDIR: nothing declares bidirectional DShot (D7)".to_owned());
    }
    match battery {
        Some(b) => b.lines(&mut l),
        None => l.push("define HAL_USE_ADC FALSE", "[T] no power.sense pin"),
    }

    // belt-and-suspenders: every solved pin is on exactly one emitted line,
    // so no fact of the topology is dropped without a name.
    let emitted: BTreeSet<&str> =
        l.0.iter()
            .filter_map(|(line, _)| line.split(' ').next())
            .filter(|w| {
                w.len() >= 3 && w.starts_with('P') && w[2..].bytes().all(|b| b.is_ascii_digit())
            })
            .collect();
    let solved = t
        .assignments
        .iter()
        .map(|(net, a)| (net, &a.pin))
        .chain(t.gpio.iter().map(|(net, g)| (net, &g.pin)));
    for (net, pin) in solved {
        if !emitted.contains(pin.as_str()) {
            return Err(Refusal::PinNotEmitted {
                net: net.clone(),
                pin: pin.clone(),
            });
        }
    }

    Ok(Hwdef {
        text: render(t, &l, &not_emitted),
        not_emitted,
    })
}

/// The largest integer a `float` holds exactly, `2^24`: below it the
/// quotient `a.0f / b.0f` is the correctly rounded ratio (`AET-WO-4410` D6).
const EXACT_IN_FLOAT: u64 = 1 << 24;

/// The one battery voltage ArduPilot reads (`AET-WO-4410` (1)): the sense
/// pin on its solved ADC1 input, and the divider's exact ratio.
struct Battery<'t> {
    net: &'t str,
    pin: &'t str,
    channel: u32,
    top: u64,
    bottom: u64,
}

impl Battery<'_> {
    /// The pin line, `HAL_BATT_VOLT_PIN`, `HAL_BATT_VOLT_SCALE` and
    /// `HAL_BATT_MONITOR_DEFAULT`, each line ArduPilot's own
    /// (`spike/4410/RESULTS.md` (1), built on Sentinel-03 in the pinned image).
    fn lines(&self, l: &mut Lines) {
        let net = self.net;
        l.push(
            format!("{} BATT_VOLTAGE_SENS ADC1 SCALE(1)", self.pin),
            format!(
                "[T] power.sense.{net}, adc_inputs.{net} (ADC1 INP{}); SCALE(1): the pin's own multiplier stays 1, the divider is HAL_BATT_VOLT_SCALE (chibios_hwdef.py:2203-2204, 2241-2247)",
                self.channel
            ),
        );
        l.push(
            format!("define HAL_BATT_VOLT_PIN {}", self.channel),
            format!(
                "[T] adc_inputs.{net}.channel: the analog pin is the named ADC's channel (chibios_hwdef.py:2204)"
            ),
        );
        l.push(
            format!(
                "define HAL_BATT_VOLT_SCALE ({}.0f/{}.0f)",
                self.top + self.bottom,
                self.bottom
            ),
            format!(
                "[T] power.sense.{net}: (top_ohms + bottom_ohms) / bottom_ohms, exact in float (AP_BattMonitor_Analog.cpp:30-36, 160)"
            ),
        );
        l.push(
            "define HAL_BATT_MONITOR_DEFAULT 3",
            "[C] voltage only: no current sense is declared (AP_BattMonitor_Params.h:17-18; AP_BattMonitor.cpp:568-569)",
        );
    }
}

// belt-and-suspenders: `aethelgard-pincheck`'s identity judge parses the same
// channel and computes the same ratio by its own code (INV-07, RUL-20).
/// The solved battery sense, if one: at most one, on ADC1 (`AET-WO-4410`
/// S2: ArduPilot's H7 build fails without an ADC1 pin line), with a divider
/// whose quotient a `float` holds exactly.
fn battery(t: &SolvedTopology) -> Result<Option<Battery<'_>>, Refusal> {
    let Some(sense) = t.power.as_ref().map(|p| &p.sense) else {
        return Ok(None);
    };
    let mut nets = sense.iter();
    let Some((net, s)) = nets.next() else {
        return Ok(None);
    };
    if let Some((second, _)) = nets.next() {
        return Err(Refusal::SenseTwice {
            first: net.clone(),
            second: second.clone(),
        });
    }
    let input = t.adc_inputs.get(net);
    let channel = input
        .filter(|a| a.peripheral == "ADC1")
        .and_then(|a| a.channel.strip_prefix("INP"))
        .and_then(|n| n.parse::<u32>().ok())
        .ok_or_else(|| Refusal::SenseNotOnAdc1 {
            net: net.clone(),
            peripheral: input.map_or_else(|| "no ADC input".to_owned(), |a| a.peripheral.clone()),
        })?;
    let (top, bottom) = (u64::from(s.top_ohms), u64::from(s.bottom_ohms));
    if bottom == 0 || top + bottom > EXACT_IN_FLOAT {
        return Err(Refusal::DividerNotExact {
            net: net.clone(),
            top_ohms: s.top_ohms,
            bottom_ohms: s.bottom_ohms,
        });
    }
    Ok(Some(Battery {
        net,
        pin: &s.pin,
        channel,
        top,
        bottom,
    }))
}

/// The serial ports: the console first as SERIAL0, then the rest in the
/// specification's order, with each port's protocol, baud and options.
fn serial(
    t: &SolvedTopology,
    l: &mut Lines,
    not_emitted: &mut Vec<String>,
    speed: impl Fn(&str) -> &'static str,
) -> Result<(), Refusal> {
    let ports = t.serial_ports_in_order();
    if ports.is_empty() {
        return Err(Refusal::NoSerialPort);
    }
    let consoles: Vec<&str> = ports
        .iter()
        .filter(|(_, p, _, _)| p.role == SerialRole::Console)
        .map(|(label, ..)| *label)
        .collect();
    match consoles.as_slice() {
        [] => return Err(Refusal::NoConsole),
        [_] => {}
        [first, second, ..] => {
            return Err(Refusal::ConsoleTwice {
                first: (*first).to_owned(),
                second: (*second).to_owned(),
            });
        }
    }
    let rc: Vec<&str> = ports
        .iter()
        .filter(|(_, p, _, _)| protocol(p.role) == "SerialProtocol_RCIN")
        .map(|(label, ..)| *label)
        .collect();
    if let [first, second, ..] = rc.as_slice() {
        return Err(Refusal::RcInputTwice {
            first: (*first).to_owned(),
            second: (*second).to_owned(),
        });
    }
    if ports.len() > 10 {
        return Err(Refusal::TooManySerialPorts { count: ports.len() });
    }
    let mut order: Vec<_> = ports
        .iter()
        .filter(|(_, p, _, _)| p.role == SerialRole::Console)
        .collect();
    order.extend(
        ports
            .iter()
            .filter(|(_, p, _, _)| p.role != SerialRole::Console),
    );
    for (n, (label, p, _, _)) in order.iter().enumerate() {
        let f = &p.framing;
        let (data, parity, stop, inverted, baud) = implied(p.role);
        if (f.data_bits, f.parity, f.stop_bits, f.inverted) != (data, parity, stop, inverted)
            || baud.is_some_and(|b| b != f.baud)
        {
            let shape = |d: u8, p: Parity, s: u8, i: bool| {
                format!(
                    "{d}{}{s}{}",
                    parity_letter(p),
                    if i { " inverted" } else { "" }
                )
            };
            return Err(Refusal::FramingNotImplied {
                port: (*label).to_owned(),
                role: p.role.name(),
                declared: format!(
                    "{} at {} baud",
                    shape(f.data_bits, f.parity, f.stop_bits, f.inverted),
                    f.baud
                ),
                implied: format!(
                    "{}{}",
                    shape(data, parity, stop, inverted),
                    baud.map(|b| format!(" at {b} baud")).unwrap_or_default()
                ),
            });
        }
        if n == 0 {
            if f.inverted {
                return Err(Refusal::InvertedConsole {
                    port: (*label).to_owned(),
                });
            }
            let mapped = map_baudrate(f.baud / 1000);
            if mapped != f.baud {
                return Err(Refusal::ConsoleBaud {
                    port: (*label).to_owned(),
                    baud: f.baud,
                    mapped,
                });
            }
        } else if f.baud <= 2000 {
            return Err(Refusal::SerialBaud {
                port: (*label).to_owned(),
                baud: f.baud,
            });
        }
    }
    let peripherals: Vec<&str> = order
        .iter()
        .map(|(_, p, _, _)| p.peripheral.as_str())
        .collect();
    l.push(
        format!("SERIAL_ORDER {}", peripherals.join(" ")),
        "[T] serial_ports: the console first, then the specification's order",
    );
    for (n, (label, p, tx, rx)) in order.iter().enumerate() {
        for (a, net) in [(tx, &p.tx), (rx, &p.rx)] {
            l.push(
                format!(
                    "{} {} {} {} PUSHPULL FLOATING",
                    a.pin,
                    a.signal,
                    a.peripheral,
                    speed(net)
                ),
                format!(
                    "[T] serial_ports.{label} ({}), assignments.{net}, pin_speeds",
                    p.role.name()
                ),
            );
        }
        // SERIAL0's protocol default is a literal, MAVLink 2, and
        // DEFAULT_SERIAL0_PROTOCOL is read only by its own `#ifndef`
        // (AP_SerialManager.cpp:37-44, 180): emitted, it would be a line
        // ArduPilot never reads (adversarial A4).
        if n > 0 {
            l.push(
                format!("define DEFAULT_SERIAL{n}_PROTOCOL {}", protocol(p.role)),
                format!("[T] serial_ports.{label}.role {}", p.role.name()),
            );
        }
        // F3 (ruled on PR #189): an RC-input port's UART is configured by
        // AP_RCProtocol itself, never by its SERIALn baud or options
        // (AP_RCProtocol.cpp:326-336, AP_SerialManager.cpp:536-543), so
        // neither is emitted; the header names what ArduPilot runs instead.
        if let Some(runs) = rc_runs(p.role) {
            not_emitted.push(format!(
                "{label} ({}) on SERIAL{n}: no DEFAULT_SERIAL{n}_BAUD or _OPTIONS; ArduPilot's RC input sets the UART itself and runs {runs}, not the declared {} baud",
                p.role.name(),
                p.framing.baud
            ));
            continue;
        }
        l.push(
            format!("define DEFAULT_SERIAL{n}_BAUD {}", p.framing.baud),
            format!("[T] serial_ports.{label}.framing.baud"),
        );
        // SERIAL7 and 8 read a literal 0, never their define
        // (AP_SerialManager.cpp:357, 376; adversarial A4's precedent).
        if matches!(n, 7 | 8) {
            not_emitted.push(format!(
                "DEFAULT_SERIAL{n}_OPTIONS for {label}: ArduPilot reads a literal 0 for SERIAL{n}_OPTIONS (AP_SerialManager.cpp:{})",
                if n == 7 { 357 } else { 376 }
            ));
        } else if n > 0 {
            // Not inverted: only SBUS may be (FramingNotImplied above), and an
            // RC-input port took its `continue`.
            l.push(
                format!("define DEFAULT_SERIAL{n}_OPTIONS 0"),
                format!("[T] serial_ports.{label}.framing.inverted false (UARTDriver.h:90-91)"),
            );
        }
    }
    Ok(())
}

/// The I2C buses: `I2C_ORDER` by each bus's first device in the
/// specification's order, then buses with no device by peripheral; the
/// slowest declared bus speed as the one clock ArduPilot has; the pins; and a
/// `BARO` line for each barometer the parts table names.
fn i2c(
    t: &SolvedTopology,
    parts_table: &parts::PartsTable,
    l: &mut Lines,
    not_emitted: &mut Vec<String>,
    speed: impl Fn(&str) -> &'static str,
) {
    let devices = t.i2c_devices_in_order();
    let mut order: Vec<&str> = Vec::new();
    for (_, d) in &devices {
        if !order.contains(&d.peripheral.as_str()) {
            order.push(&d.peripheral);
        }
    }
    let all: BTreeSet<&str> = t
        .assignments
        .values()
        .filter(|a| family(&a.signal) == Some(Family::I2c))
        .map(|a| a.peripheral.as_str())
        .collect();
    for p in all {
        if !order.contains(&p) {
            order.push(p);
        }
    }
    if order.is_empty() {
        return;
    }
    l.push(
        format!("I2C_ORDER {}", order.join(" ")),
        "[T] i2c_devices in index order, then each device-less bus by peripheral",
    );
    // `validate` gives every solved I2C bus a declared speed, so a bus in
    // `order` has one.
    let slowest = t
        .i2c_speeds
        .values()
        .min()
        .copied()
        .expect("an I2C bus has a declared speed");
    l.push(
        format!("define HAL_I2C_MAX_CLOCK {slowest}"),
        "[T] the minimum of i2c_speeds: no bus runs faster than declared (ArduPilot has one clock, I2CDevice.cpp:53, 209)",
    );
    for p in &order {
        for suffix in ["SCL", "SDA"] {
            if let Some((net, a)) = t
                .assignments
                .iter()
                .find(|(_, a)| a.peripheral == *p && a.signal.ends_with(&format!("_{suffix}")))
            {
                l.push(
                    format!(
                        "{} {} {} {} OPENDRAIN FLOATING",
                        a.pin,
                        a.signal,
                        a.peripheral,
                        speed(net)
                    ),
                    format!("[T] assignments.{net}, pin_speeds; open-drain, pulled up off-chip"),
                );
            }
        }
    }
    for (reference, d) in &devices {
        let row = parts_table.get(&d.part).and_then(|p| p.ardupilot.as_ref());
        match row {
            Some(a) if a.kind == ArduPilotKind::Baro => {
                let bus = order.iter().position(|p| *p == d.peripheral).expect("listed above");
                l.push(
                    format!(
                        "BARO {} I2C:{bus}:{:#04x}",
                        a.driver.as_deref().expect("a baro row names its driver"),
                        d.address
                    ),
                    format!("[T] i2c_devices.{reference} ({}, I2C_ORDER index {bus}); [P] its ardupilot driver", d.part),
                );
            }
            _ => not_emitted.push(format!(
                "{reference} ({}): no ArduPilot line; the parts table holds no ardupilot column for it",
                d.part
            )),
        }
    }
}

/// ArduPilot's axis-aligned rotations, as `Vector3<T>::rotate` computes them
/// at the tag (`libraries/AP_Math/vector3.cpp`, evaluated case by case,
/// `AET-WO-4400d` FEEDBACK §1.1): each name with its matrix, rows x, y, z, so
/// that `v_board = M · v_driver`. Of two names for one matrix (26 is 14, 27 is
/// 10, `rotations.h` L54-55) the lower-numbered is written (ruled D4).
const ROTATIONS: [(&str, [[i8; 3]; 3]); 24] = [
    ("ROTATION_NONE", [[1, 0, 0], [0, 1, 0], [0, 0, 1]]),
    ("ROTATION_YAW_90", [[0, -1, 0], [1, 0, 0], [0, 0, 1]]),
    ("ROTATION_YAW_180", [[-1, 0, 0], [0, -1, 0], [0, 0, 1]]),
    ("ROTATION_YAW_270", [[0, 1, 0], [-1, 0, 0], [0, 0, 1]]),
    ("ROTATION_ROLL_180", [[1, 0, 0], [0, -1, 0], [0, 0, -1]]),
    (
        "ROTATION_ROLL_180_YAW_90",
        [[0, 1, 0], [1, 0, 0], [0, 0, -1]],
    ),
    ("ROTATION_PITCH_180", [[-1, 0, 0], [0, 1, 0], [0, 0, -1]]),
    (
        "ROTATION_ROLL_180_YAW_270",
        [[0, -1, 0], [-1, 0, 0], [0, 0, -1]],
    ),
    ("ROTATION_ROLL_90", [[1, 0, 0], [0, 0, -1], [0, 1, 0]]),
    ("ROTATION_ROLL_90_YAW_90", [[0, 0, 1], [1, 0, 0], [0, 1, 0]]),
    ("ROTATION_ROLL_270", [[1, 0, 0], [0, 0, 1], [0, -1, 0]]),
    (
        "ROTATION_ROLL_270_YAW_90",
        [[0, 0, -1], [1, 0, 0], [0, -1, 0]],
    ),
    ("ROTATION_PITCH_90", [[0, 0, 1], [0, 1, 0], [-1, 0, 0]]),
    ("ROTATION_PITCH_270", [[0, 0, -1], [0, 1, 0], [1, 0, 0]]),
    (
        "ROTATION_ROLL_90_PITCH_90",
        [[0, 1, 0], [0, 0, -1], [-1, 0, 0]],
    ),
    (
        "ROTATION_ROLL_180_PITCH_90",
        [[0, 0, -1], [0, -1, 0], [-1, 0, 0]],
    ),
    (
        "ROTATION_ROLL_270_PITCH_90",
        [[0, -1, 0], [0, 0, 1], [-1, 0, 0]],
    ),
    (
        "ROTATION_ROLL_90_PITCH_180",
        [[-1, 0, 0], [0, 0, -1], [0, -1, 0]],
    ),
    (
        "ROTATION_ROLL_270_PITCH_180",
        [[-1, 0, 0], [0, 0, 1], [0, 1, 0]],
    ),
    (
        "ROTATION_ROLL_90_PITCH_270",
        [[0, -1, 0], [0, 0, -1], [1, 0, 0]],
    ),
    (
        "ROTATION_ROLL_180_PITCH_270",
        [[0, 0, 1], [0, -1, 0], [1, 0, 0]],
    ),
    (
        "ROTATION_ROLL_270_PITCH_270",
        [[0, 1, 0], [0, 0, 1], [1, 0, 0]],
    ),
    (
        "ROTATION_ROLL_90_PITCH_180_YAW_90",
        [[0, 0, 1], [-1, 0, 0], [0, -1, 0]],
    ),
    (
        "ROTATION_ROLL_90_YAW_270",
        [[0, 0, -1], [-1, 0, 0], [0, 1, 0]],
    ),
];

/// A board-frame direction as a unit vector: x forward, y right, z down
/// (the engine's frame, F.Cu up; ruled D3).
fn unit(d: Toward) -> [i8; 3] {
    match d {
        Toward::Forward => [1, 0, 0],
        Toward::Aft => [-1, 0, 0],
        Toward::Right => [0, 1, 0],
        Toward::Left => [0, -1, 0],
        Toward::Down => [0, 0, 1],
        Toward::Up => [0, 0, -1],
    }
}

/// The `ROTATION_…` for a declared orientation, with the driver passing the
/// chip's axes unchanged (`ardupilot.frame` `chip`, the one frame the parts
/// table accepts): the matrix whose columns are the chip's X, Y = Z × X and
/// Z in the board frame. `None` only for axes not at right angles, which
/// `SolvedTopology::validate` refuses first.
fn rotation(o: &Orientation) -> Option<&'static str> {
    let x = unit(o.x);
    let z = unit(o.z);
    let y = [
        z[1] * x[2] - z[2] * x[1],
        z[2] * x[0] - z[0] * x[2],
        z[0] * x[1] - z[1] * x[0],
    ];
    let m = [[x[0], y[0], z[0]], [x[1], y[1], z[1]], [x[2], y[2], z[2]]];
    ROTATIONS.iter().find(|(_, r)| *r == m).map(|(n, _)| *n)
}

/// The SPI buses: each controller's pins, each device's chip select and
/// `SPIDEV` (from the parts table's name, on its controller's achieved
/// rate), the logger's define, an SPI barometer's `BARO` line, and an IMU's
/// `IMU` line, its rotation computed from the declared orientation
/// (`AET-WO-4400d`); an IMU with none is refused (`INV-06`).
fn spi(
    t: &SolvedTopology,
    parts_table: &parts::PartsTable,
    osc: &ArduPilotOscillator,
    l: &mut Lines,
    not_emitted: &mut Vec<String>,
    speed: impl Fn(&str) -> &'static str,
) -> Result<(), Refusal> {
    let controllers: BTreeMap<&str, Vec<(&String, &aethelgard_ir::PinAssignment)>> = t
        .assignments
        .iter()
        .filter(|(_, a)| family(&a.signal) == Some(Family::Spi))
        .fold(BTreeMap::new(), |mut m, (net, a)| {
            m.entry(a.peripheral.as_str())
                .or_insert_with(Vec::new)
                .push((net, a));
            m
        });
    for pins in controllers.values() {
        for suffix in ["SCK", "MISO", "MOSI"] {
            for (net, a) in pins
                .iter()
                .filter(|(_, a)| a.signal.ends_with(&format!("_{suffix}")))
            {
                l.push(
                    format!(
                        "{} {} {} {} PUSHPULL FLOATING",
                        a.pin,
                        a.signal,
                        a.peripheral,
                        speed(net)
                    ),
                    format!("[T] assignments.{net}, pin_speeds"),
                );
            }
        }
    }
    let devices = t.spi_devices_in_order();
    for (reference, d) in &devices {
        l.push(
            format!(
                "{} {} CS {} PUSHPULL FLOATING",
                t.gpio[&d.cs].pin,
                d.cs,
                speed(&d.cs)
            ),
            format!(
                "[T] gpio.{}, the chip select of spi_devices.{reference}, pin_speeds",
                d.cs
            ),
        );
    }
    let mut names: BTreeMap<String, &str> = BTreeMap::new();
    let mut on_bus: BTreeMap<&str, u32> = BTreeMap::new();
    let mut dataflash = false;
    let mut baros = Vec::new();
    let mut imus = Vec::new();
    for (reference, d) in &devices {
        let Some(a) = parts_table.get(&d.part).and_then(|p| p.ardupilot.as_ref()) else {
            not_emitted.push(format!(
                "{reference} ({}): no SPIDEV; the parts table holds no ardupilot column for it",
                d.part
            ));
            continue;
        };
        let name = a.spidev.clone().expect("an SPI row names its SPIDEV");
        if let Some(first) = names.insert(name.clone(), reference) {
            return Err(Refusal::DuplicateSpidev {
                name,
                first: first.to_owned(),
                second: (*reference).to_owned(),
            });
        }
        let c = &t.spi_controllers[&d.peripheral];
        let kernel = osc
            .spi_kernel_hz
            .get(&d.peripheral)
            .ok_or_else(|| Refusal::SpiKernelNotCited {
                peripheral: d.peripheral.clone(),
            })?
            .value;
        let asked = c.achieved_hz / 1000 * 1000;
        if u128::from(asked) * 256
            < u128::try_from(kernel).expect("the parser holds every kernel above zero")
        {
            return Err(Refusal::SpiBelowArduPilotDivider {
                peripheral: d.peripheral.clone(),
                hz: asked,
                kernel_hz: kernel,
                floor: kernel / 256,
            });
        }
        let devid = on_bus.entry(&d.peripheral).or_insert(0);
        *devid += 1;
        let hz = spidev_speed(c.achieved_hz);
        l.push(
            format!("SPIDEV {name} {} DEVID{devid} {} {} {hz} {hz}", d.peripheral, d.cs, spi_mode()),
            format!(
                "[T] spi_devices.{reference} ({}), spi_controllers.{}.achieved_hz rounded down to a kHz, DRIVEN_SPI_MODE; [P] its ardupilot spidev",
                d.part, d.peripheral
            ),
        );
        match a.kind {
            ArduPilotKind::Dataflash => dataflash = true,
            ArduPilotKind::Imu => {
                let refused = |undeclared: bool| {
                    let (reference, part) = ((*reference).to_owned(), d.part.clone());
                    if undeclared {
                        Refusal::ImuOrientationUndeclared { reference, part }
                    } else {
                        Refusal::ImuFrameNotCited { reference, part }
                    }
                };
                let o = d.orientation.ok_or_else(|| refused(true))?;
                // belt-and-suspenders: `validate` refuses an orientation on
                // a part whose row cites no frame (`ImuAxesNotCited`), so a
                // topology from `solve` never reaches this; one written by
                // hand can, and its rotation would then rest on an assumed
                // frame.
                if a.frame.is_none() {
                    return Err(refused(false));
                }
                let r = rotation(&o)
                    .expect("validate refuses axes not at right angles; the 24 cover the rest");
                imus.push((reference, d, a.driver.clone(), name, o, r));
            }
            ArduPilotKind::Baro => baros.push((reference, d, a.driver.clone(), name)),
        }
    }
    if dataflash {
        l.push(
            "define HAL_LOGGING_DATAFLASH_ENABLED 1",
            "[P] a dataflash part on SPIDEV dataflash (AP_Logger_config.h:15-17 defaults it off)",
        );
    }
    for (reference, d, driver, name) in baros {
        l.push(
            format!(
                "BARO {} SPI:{name}",
                driver.expect("a baro row names its driver")
            ),
            format!(
                "[T] spi_devices.{reference} ({}); [P] its ardupilot driver",
                d.part
            ),
        );
    }
    for (reference, d, driver, name, o, r) in imus {
        l.push(
            format!(
                "IMU {} SPI:{name} {r}",
                driver.expect("an imu row names its driver")
            ),
            format!(
                "[T] spi_devices.{reference}.orientation (x {}, z {}) in the board frame [C]; [P] its ardupilot driver and frame ({})",
                o.x.name(),
                o.z.name(),
                d.part
            ),
        );
    }
    Ok(())
}

/// The file: a header naming the source, the constants and what is not
/// emitted, then each line with its source comment.
fn render(t: &SolvedTopology, l: &Lines, not_emitted: &[String]) -> String {
    let mut header: Vec<String> = Vec::new();
    let mut c = |s: &str| header.push(format!("# {s}"));
    c(&format!(
        "hwdef.dat for {ARDUPILOT}, generated by aethelgard-hwdef {} (AET-WO-4400b).",
        env!("CARGO_PKG_VERSION")
    ));
    c(&format!(
        "From the SolvedTopology of specification sha256 {} and the parts tables alone (RUL-16). Do not edit: regenerate.",
        t.specification_sha256
    ));
    c("Sources: [T] the topology; [P] a parts-table row; [C] a constant of this generator.");
    c(&format!(
        "Constants: APJ_BOARD_ID {APJ_BOARD_ID} (no row of board_types.txt holds it); no bootloader (AP_BOOTLOADER_FLASHING_ENABLED 0, FLASH_RESERVE_START_KB 0); HAL_STORAGE_SIZE {HAL_STORAGE_SIZE} in the last two flash sectors; the system timer TIM5, else TIM2; the SWD port PA13/PA14 kept on."
    ));
    c("Never emitted: DEFAULTGPIO (4400a D-1).");
    if l.0.iter().any(|(line, _)| line.starts_with("IMU ")) {
        c(
            "Board frame (AET-WO-4400d D3): x toward board.forward, y to its right, z down through F.Cu to B.Cu; the board is mounted F.Cu up, and any other mounting is AHRS_ORIENTATION's.",
        );
    }
    if l.0
        .iter()
        .any(|(line, _)| line.starts_with("define DEFAULT_SERIAL") && line.contains("_PROTOCOL "))
    {
        c(
            "Serial pulls: FLOATING holds from reset; ArduPilot's set_options then pulls up TX and RX on every port it opens (UARTDriver.cpp:1611, 1624): SERIAL1 to SERIAL9 at init (AP_SerialManager.cpp:451-457), and SERIAL0 when GCS looks up its MAVLink console (GCS_Common.cpp:2810; AP_SerialManager.cpp:605, 656). It pulls down a line it inverts, and no line here removes that.",
        );
    }
    for n in not_emitted {
        c(&format!("Not emitted: {n}."));
    }
    let mut out = header.join("\n");
    out.push_str("\n\n");
    // A comment line is written as it is, and sets no width.
    let width =
        l.0.iter()
            .filter(|(_, source)| !source.is_empty())
            .map(|(line, _)| line.len())
            .max()
            .unwrap_or(0);
    for (line, source) in &l.0 {
        if source.is_empty() {
            let _ = writeln!(out, "{line}");
        } else {
            let _ = writeln!(out, "{line:width$}  # {source}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `AET-WO-4410` (2), §5: each oscillator's own kernel clocks judge the
    /// SPI floor. SPI4 runs at 72 MHz on `OSCILLATOR_HZ` 0 and at 100 MHz on
    /// 16000000, so 300 kHz passes the first (floor 281,250 Hz) and is
    /// refused on the second (floor 390,625 Hz): a 16 MHz topology is never
    /// judged by the `0` row's clocks, nor the reverse.
    #[test]
    fn each_oscillator_judges_the_spi_floor_by_its_own_kernel_clocks() {
        let mut t: SolvedTopology =
            serde_json::from_str(include_str!("../tests/fixtures/sentinel-01.topology.json"))
                .unwrap();
        let mut c = t.spi_controllers.remove("SPI1").unwrap();
        c.achieved_hz = 300_000;
        t.spi_controllers.insert("SPI4".into(), c);
        for d in t.spi_devices.values_mut() {
            d.peripheral = "SPI4".into();
        }
        let (_, ap) = mcu_row(&t.mcu).unwrap();
        let run = |hz: u32| {
            spi(
                &t,
                parts::table().unwrap(),
                ap.oscillator(hz).unwrap(),
                &mut Lines(Vec::new()),
                &mut Vec::new(),
                |_| "",
            )
        };
        assert_eq!(run(0), Ok(()));
        assert_eq!(
            run(16_000_000),
            Err(Refusal::SpiBelowArduPilotDivider {
                peripheral: "SPI4".into(),
                hz: 300_000,
                kernel_hz: 100_000_000,
                floor: 390_625,
            })
        );
        assert!(ap.oscillator(8_000_000).is_none());
    }

    /// D6: SPIDEV states whole kHz, rounded down, as MHz when exact.
    #[test]
    fn spidev_speeds_round_down_to_a_whole_khz() {
        assert_eq!(spidev_speed(16_000_000), "16*MHZ");
        assert_eq!(spidev_speed(15_999_999), "15999*KHZ");
        assert_eq!(spidev_speed(2_000_000), "2*MHZ");
        assert_eq!(spidev_speed(312_500), "312*KHZ");
        assert_eq!(spidev_speed(1_000), "1*KHZ");
    }

    /// `AP_SerialManager::map_baudrate` at the bauds a console declares:
    /// SERIAL0's `baud / 1000` maps back for these, and not for 14,400.
    #[test]
    fn serial0_bauds_map_back_as_ardupilot_maps_them() {
        for baud in [
            9_600, 57_600, 115_200, 230_400, 460_800, 921_600, 100_000, 420_000,
        ] {
            assert_eq!(map_baudrate(baud / 1000), baud, "{baud}");
        }
        assert_eq!(map_baudrate(14_400 / 1000), 14_000);
        assert_eq!(map_baudrate(0), 57_600);
    }

    /// `AET-WO-1214` (`1213` F3): `generate` validates the topology first, and
    /// `SolvedTopology::validate` refuses both bauds below (`ConsoleBaudNotRoundTrip`,
    /// `SerialBaudReadAsKbaud`, each a `SerialPortMismatch` there), so the
    /// generator's own guards are reached only by calling `serial` directly.
    /// They stay as `RUL-20`'s pair, for a topology that reaches the
    /// generator by another path. Each fires here, and names its port.
    #[test]
    fn the_serial_baud_guards_fire_when_called_directly() {
        let fixture: SolvedTopology = serde_json::from_str(include_str!(
            "../tests/fixtures/serial-dshot-9600.topology.json"
        ))
        .unwrap();
        let serial =
            |t: &SolvedTopology| serial(t, &mut Lines(Vec::new()), &mut Vec::new(), |_| "");
        assert_eq!(serial(&fixture), Ok(()));

        // The console at 76,800: SERIAL0 holds 76, which maps to 76,000.
        let mut t = fixture.clone();
        t.serial_ports.get_mut("CONSOLE").unwrap().framing.baud = 76_800;
        assert_eq!(
            serial(&t),
            Err(Refusal::ConsoleBaud {
                port: "CONSOLE".into(),
                baud: 76_800,
                mapped: 76_000
            })
        );
        let Err(Refusal::InvalidTopology(e)) = generate(&t) else {
            panic!("validate refuses first")
        };
        assert_eq!((e.kind(), e.id()), ("SerialPortMismatch", Some("CONSOLE")));

        // A second port, `mavlink` on SERIAL1, at 1200: read as kbaud.
        let mut t = fixture.clone();
        let mut telem = t.serial_ports["CONSOLE"].clone();
        (telem.index, telem.role) = (1, SerialRole::Mavlink);
        (telem.tx, telem.rx) = ("TELEM_TX".into(), "TELEM_RX".into());
        telem.framing.baud = 1200;
        for net in ["TX", "RX"] {
            let a = t.assignments[&format!("CONSOLE_{net}")].clone();
            t.assignments.insert(format!("TELEM_{net}"), a);
        }
        t.serial_ports.insert("TELEM".into(), telem);
        assert_eq!(
            serial(&t),
            Err(Refusal::SerialBaud {
                port: "TELEM".into(),
                baud: 1200
            })
        );
        t.serial_ports.get_mut("TELEM").unwrap().framing.baud = 2001;
        assert_eq!(serial(&t), Ok(()), "2001 is not kbaud");
    }
}
