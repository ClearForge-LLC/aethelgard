//! Serial ports (`AET-WO-4300b`): what a specification declares, the `BRR`
//! the solver derives on the solved clock, and the static framing check that
//! judges it against RM0433's receiver tolerance (`4300` D8: framing is
//! static, bytes are witnessed, and there is no framing fault).
//!
//! A serial port is one USART or UART with a TX and an RX pin, each on an
//! alternate function. Its two nets are entries of
//! `SolvedTopology::assignments` (`USARTn_TX` / `USARTn_RX`, `UARTn_…`), so
//! every generator that draws or configures solved pins sees them.
//! [`SolvedSerialPort`] adds the role, the framing and the solved `BRR`.
//!
//! The generated firmware leaves `OVER8` and `ONEBIT` at 0 and `USART_PRESC`
//! at its reset value (÷1), so the kernel clock divided by `BRR` is the baud
//! (RM0433 Rev 9 §48.5.7, "In case of oversampling by 16, the equation is
//! Tx/Rx baud = usart_ker_ckpres / USARTDIV", `BRR = USARTDIV`).

use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use std::collections::{BTreeMap, BTreeSet};

use crate::motor::SolvedClock;
use crate::{IrValidationError, Signal, SolvedTopology};

/// What a port carries. The framing is declared, never inferred from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SerialRole {
    Console,
    Mavlink,
    Gps,
    RcCrsf,
    RcSbus,
}

impl SerialRole {
    /// The name the specification and the trace use.
    pub fn name(self) -> &'static str {
        match self {
            Self::Console => "console",
            Self::Mavlink => "mavlink",
            Self::Gps => "gps",
            Self::RcCrsf => "rc_crsf",
            Self::RcSbus => "rc_sbus",
        }
    }
}

/// A frame's parity bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Parity {
    None,
    Even,
    Odd,
}

impl Parity {
    pub fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Even => "even",
            Self::Odd => "odd",
        }
    }
}

/// A port's declared framing. Every key is required (`INV-06`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Framing {
    pub baud: u32,
    /// Data bits, not counting the parity bit: 7, 8 or 9, with the word
    /// (data and parity) 7 to 9 bits long.
    pub data_bits: u8,
    pub parity: Parity,
    /// 1 or 2.
    pub stop_bits: u8,
    /// Both lines inverted (`CR2.TXINV` and `CR2.RXINV`).
    pub inverted: bool,
}

impl Framing {
    /// The word length the receiver samples: data bits plus a parity bit.
    /// Saturating, so a deserialized `data_bits` of 255 gives no word
    /// rather than an overflow.
    pub fn word_bits(&self) -> u8 {
        self.data_bits
            .saturating_add(u8::from(self.parity != Parity::None))
    }

    /// `USART_CR1.M[1:0]` for the word (RM0433 Rev 9 §48.8.1): `00` 8 bits,
    /// `01` 9 bits, `10` 7 bits; `None` for any other word.
    pub fn m_bits(&self) -> Option<u8> {
        match self.word_bits() {
            8 => Some(0b00),
            9 => Some(0b01),
            7 => Some(0b10),
            _ => None,
        }
    }

    /// `USART_CR2.STOP[1:0]` (RM0433 Rev 9 §48.8.2): `00` 1 stop bit, `10` 2;
    /// `None` for any other count.
    pub fn stop_field(&self) -> Option<u8> {
        match self.stop_bits {
            1 => Some(0b00),
            2 => Some(0b10),
            _ => None,
        }
    }

    /// `8N1`, `8E2`: how a finding names the framing.
    pub fn short(&self) -> String {
        let p = match self.parity {
            Parity::None => 'N',
            Parity::Even => 'E',
            Parity::Odd => 'O',
        };
        format!("{}{p}{}", self.data_bits, self.stop_bits)
    }
}

/// One declared serial port. Every key is required (`INV-06`): `peripheral`
/// and each signal's `pin` are `null` for the solver to choose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SerialPort {
    pub label: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub peripheral: Option<String>,
    pub tx: Signal,
    pub rx: Signal,
    pub role: SerialRole,
    pub framing: Framing,
}

/// The USARTs and UARTs a port may use (the work order's list): LPUART1 is
/// refused by name, because its enable bit is in `RCC_APB4ENR`, which the
/// generated firmware does not own, and its baud generator differs.
pub const PERIPHERALS: [&str; 8] = [
    "USART1", "USART2", "USART3", "UART4", "UART5", "USART6", "UART7", "UART8",
];

/// The peripheral refused by name.
pub const LPUART: &str = "LPUART1";

/// The kernel clock of `peripheral` at reset, in Hz, from `clock`:
/// `RCC_D2CCIP2R` resets to 0 (RM0433 Rev 9 §8.7.21), so `USART16SEL` = `000`
/// selects `rcc_pclk2` for USART1 and USART6, and `USART234578SEL` = `000`
/// selects `rcc_pclk1` for the rest. `rcc_pclkN` is `rcc_hclk1` divided by
/// `D2PPREN` (RM0433 Rev 9 Figure 49). `None` for a peripheral not in
/// [`PERIPHERALS`].
pub fn kernel_hz(clock: &SolvedClock, peripheral: &str) -> Option<u64> {
    let ppre = match peripheral {
        "USART1" | "USART6" => clock.d2ppre2,
        p if PERIPHERALS.contains(&p) => clock.d2ppre1,
        _ => return None,
    };
    Some(clock.hclk_hz() / u64::from(ppre.max(1)))
}

/// `BRR` for `baud` on `kernel_hz` with oversampling by 16: the nearest whole
/// `USARTDIV`, half up (`(2·kernel + baud) / (2·baud)`), which is not always
/// the `BRR` of least baud error, since that error, `|USARTDIV − BRR| / BRR`,
/// weighs each side by its own `BRR`: when `USARTDIV`'s fraction lies in
/// `[1/2 − 1/(4·BRR + 2), 1/2)` this gives `BRR` and the least error
/// `BRR + 1`. On the 64 MHz kernel clock some whole baud falls there at every
/// `BRR` from 16 to 265 and at a few up to 2606 (computed over every baud);
/// no declared baud does (FEEDBACK §9 Q5, accepted on PR #152). `None` outside `BRR`'s range: at least 16 (RM0433 Rev 9 §48.8.5,
/// "BRR[15:0] … must be greater than or equal to 16") and at most `0xFFFF`.
pub fn solve_brr(kernel_hz: u64, baud: u32) -> Option<u32> {
    let baud = u64::from(baud);
    if baud == 0 {
        return None;
    }
    let brr = (2 * kernel_hz + baud) / (2 * baud);
    u32::try_from(brr)
        .ok()
        .filter(|b| (16..=0xFFFF).contains(b))
}

/// One row of RM0433 Rev 9 Tables 406 and 407 (§48.5.8): the receiver's
/// tolerance to clock deviation, in parts per million, at `OVER8` = 0 and
/// `ONEBIT` = 0, the configuration the generated firmware writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ToleranceRow {
    pub table: &'static str,
    /// `USART_CR1.M[1:0]`.
    pub m_bits: u8,
    pub tolerance_ppm: u32,
}

/// The source of [`TOLERANCE`].
pub const TOLERANCE_SOURCE: &str =
    "RM0433 Rev 9 §48.5.8, Tables 406 and 407, column OVER8 = 0, ONEBIT = 0";

/// RM0433 Rev 9 Table 406 (`BRR[3:0]` = `0000`) and Table 407 (`BRR[3:0]` ≠
/// `0000`), column `OVER8` = 0, `ONEBIT` = 0, by `M` bits: as data, as
/// `4300c`'s Fmax table is.
pub const TOLERANCE: [ToleranceRow; 6] = [
    ToleranceRow {
        table: "Table 406",
        m_bits: 0b00,
        tolerance_ppm: 37_500,
    },
    ToleranceRow {
        table: "Table 406",
        m_bits: 0b01,
        tolerance_ppm: 34_100,
    },
    ToleranceRow {
        table: "Table 406",
        m_bits: 0b10,
        tolerance_ppm: 41_600,
    },
    ToleranceRow {
        table: "Table 407",
        m_bits: 0b00,
        tolerance_ppm: 33_300,
    },
    ToleranceRow {
        table: "Table 407",
        m_bits: 0b01,
        tolerance_ppm: 30_300,
    },
    ToleranceRow {
        table: "Table 407",
        m_bits: 0b10,
        tolerance_ppm: 37_000,
    },
];

/// The row that applies to `brr` and `m_bits`: Table 406 when `BRR[3:0]` is
/// `0000`, else Table 407.
pub fn tolerance(brr: u32, m_bits: u8) -> Option<&'static ToleranceRow> {
    let table = if brr & 0xF == 0 {
        "Table 406"
    } else {
        "Table 407"
    };
    TOLERANCE
        .iter()
        .find(|r| r.table == table && r.m_bits == m_bits)
}

/// Whether the error of `kernel_hz / brr` against `baud` is strictly below
/// `tolerance_ppm` (RM0433's "<"), in exact integers:
/// `|kernel − BRR·baud| · 10⁶ < tolerance · BRR·baud`.
///
/// With `BRR` the nearest whole `USARTDIV` and at least 16, the error is at
/// most `1 / (2·BRR)`: 3.125 % at `BRR` 16 (Table 406, whose tightest row is
/// 3.41 %) and 2.94 % from `BRR` 17 on (Table 407, tightest 3.03 %). So no
/// baud [`solve_brr`] admits fails this on any kernel clock; the check
/// refuses only a `BRR` that is not the solved one.
pub fn within(kernel_hz: u64, brr: u32, baud: u32, tolerance_ppm: u32) -> bool {
    let d = u128::from(brr) * u128::from(baud);
    u128::from(kernel_hz).abs_diff(d) * 1_000_000 < u128::from(tolerance_ppm) * d
}

/// The baud a `BRR` gives on `kernel_hz`, in thousandths of a baud, rounded
/// down.
pub fn actual_millibaud(kernel_hz: u64, brr: u32) -> u64 {
    kernel_hz.saturating_mul(1000) / u64::from(brr.max(1))
}

/// The error of the actual baud against the declared one, in parts per
/// million, rounded half away from zero: `(kernel / BRR − baud) / baud`.
pub fn error_ppm(kernel_hz: u64, brr: u32, baud: u32) -> i64 {
    let (k, d) = (i128::from(kernel_hz), i128::from(brr) * i128::from(baud));
    if d == 0 {
        return i64::MAX;
    }
    let num = (k - d) * 1_000_000;
    let q = (2 * num.abs() + d) / (2 * d);
    let q = if num < 0 { -q } else { q };
    i64::try_from(q).unwrap_or(i64::MAX)
}

/// One serial port as the generators and the judges need it: its place in
/// the specification, its peripheral and nets (each a key of `assignments`),
/// its declared role and framing (copied by `solve`, `RUL-16`), and the
/// `BRR` solved for the topology's clock with the baud it gives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedSerialPort {
    /// Position in the specification's `serial_ports`.
    pub index: u32,
    /// `USART1` … `UART8`.
    pub peripheral: String,
    /// The TX and RX nets: keys of `assignments` on `<peripheral>_TX` and
    /// `<peripheral>_RX`.
    pub tx: String,
    pub rx: String,
    pub role: SerialRole,
    pub framing: Framing,
    /// The kernel clock [`kernel_hz`] gives on the topology's clock.
    pub kernel_hz: u64,
    /// `USART_BRR` as the firmware writes it.
    pub brr: u32,
    /// [`actual_millibaud`] and [`error_ppm`] for `brr`.
    pub actual_millibaud: u64,
    pub error_ppm: i64,
}

crate::object_only!(Framing, SerialPort, SolvedSerialPort);

/// What [`check_silicon`] found for one port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SiliconFraming {
    pub kernel_hz: u64,
    pub brr: u32,
    pub actual_millibaud: u64,
    pub error_ppm: i64,
    /// The row of [`TOLERANCE`] that applies, with [`TOLERANCE_SOURCE`].
    pub tolerance: Option<ToleranceRow>,
    /// Every rule held: the kernel clock is the one the topology's clock
    /// gives; `BRR` is in range and is the nearest whole `USARTDIV`; the
    /// word is 7 to 9 bits and the stop bits 1 or 2; and the error is
    /// strictly below the row's tolerance (RM0433's "<").
    pub holds: bool,
}

/// The static check for silicon: `port`'s `BRR` and framing on `clock`, in
/// exact integers. It never reads the generated firmware or board (`INV-07`).
pub fn check_silicon(clock: &SolvedClock, port: &SolvedSerialPort) -> SiliconFraming {
    let kernel = kernel_hz(clock, &port.peripheral);
    let tolerance = port
        .framing
        .m_bits()
        .and_then(|m| tolerance(port.brr, m))
        .copied();
    let brr_ok = kernel
        .is_some_and(|k| k == port.kernel_hz && solve_brr(k, port.framing.baud) == Some(port.brr));
    let shape_ok = port.framing.stop_field().is_some();
    let within = tolerance.is_some_and(|row| {
        within(
            port.kernel_hz,
            port.brr,
            port.framing.baud,
            row.tolerance_ppm,
        )
    });
    SiliconFraming {
        kernel_hz: port.kernel_hz,
        brr: port.brr,
        actual_millibaud: actual_millibaud(port.kernel_hz, port.brr),
        error_ppm: error_ppm(port.kernel_hz, port.brr, port.framing.baud),
        tolerance,
        holds: brr_ok && shape_ok && within,
    }
}

/// The lowest baud TBS's CRSF specification gives a dual-wire port (a port
/// with TX and RX): "416666 baud 8N1 … but baudrate can be negotiated to be
/// higher" ([`CRSF_SOURCE`]).
pub const CRSF_MIN_BAUD: u32 = 416_666;

/// TBS's CRSF specification as held (`AET-WO-4300b` FEEDBACK §0).
pub const CRSF_SOURCE: &str = "TBS CRSF Protocol, github.com/tbs-fpv/tbs-crsf-spec crsf.md at a73f7dc5 (sha256 58293cef…f918), Hardware: Dual-Wire Full-Duplex UART";

/// Where a declared framing contradicts the role's published framing, the
/// field and what the source requires; `None` when it does not, or when the
/// role has no published framing (SBUS has no vendor-published source; the
/// console, MAVLink and GPS roles declare their own).
pub fn contradiction(role: SerialRole, framing: &Framing) -> Option<(&'static str, String)> {
    if role != SerialRole::RcCrsf {
        return None;
    }
    if framing.data_bits != 8 || framing.parity != Parity::None || framing.stop_bits != 1 {
        return Some(("framing", format!("8N1, not {}", framing.short())));
    }
    if framing.inverted {
        return Some((
            "inverted",
            "false: only non-inverted UART is supported on a dual-wire port".to_owned(),
        ));
    }
    if framing.baud < CRSF_MIN_BAUD {
        return Some((
            "baud",
            format!(
                "at least {CRSF_MIN_BAUD} (the default, negotiable higher), not {}",
                framing.baud
            ),
        ));
    }
    None
}

/// Step 13c of [`crate::Specification::validate`], each port in document
/// order: a label unique with case folded; a peripheral that is `null` or one
/// of [`PERIPHERALS`] exactly (`LPUART1`, in any case, refused by its own
/// name), and not another port's; the framing's shape; and no contradiction
/// of the role's published framing ([`contradiction`]). The nets are step 11's
/// and a constraint on one step 14's.
pub(crate) fn validate(ports: &[SerialPort]) -> Result<(), IrValidationError> {
    use IrValidationError as E;

    let mut labels: BTreeMap<String, &str> = BTreeMap::new();
    let mut peripherals: BTreeMap<&str, &str> = BTreeMap::new();
    for port in ports {
        let id = || port.label.clone();
        if labels
            .insert(port.label.to_lowercase(), &port.label)
            .is_some()
        {
            return Err(E::DuplicateSerialLabel { id: id() });
        }
        if let Some(p) = port.peripheral.as_deref() {
            if p.eq_ignore_ascii_case(LPUART) {
                return Err(E::SerialPeripheralLpuart {
                    id: id(),
                    peripheral: p.to_owned(),
                });
            }
            if !PERIPHERALS.contains(&p) {
                return Err(E::UnknownSerialPeripheral {
                    id: id(),
                    peripheral: p.to_owned(),
                });
            }
            if let Some(first) = peripherals.insert(p, &port.label) {
                return Err(E::SerialPeripheralTwice {
                    id: id(),
                    first: first.to_owned(),
                    peripheral: p.to_owned(),
                });
            }
        }
        if let Some((field, value, rule)) = shape_error(&port.framing) {
            return Err(E::SerialFramingShape {
                id: id(),
                field,
                value,
                rule,
            });
        }
        if let Some((field, required)) = contradiction(port.role, &port.framing) {
            return Err(E::SerialFramingContradictsRole {
                id: id(),
                role: port.role.name(),
                field,
                required,
                citation: CRSF_SOURCE,
            });
        }
    }
    Ok(())
}

/// The first field of `framing` outside the shape the firmware can write:
/// a baud above 0, 7 to 9 data bits, a word (data and parity) of 7 to 9 bits
/// (`M[1:0]`), and 1 or 2 stop bits (`STOP`).
fn shape_error(framing: &Framing) -> Option<(&'static str, u32, &'static str)> {
    if framing.baud == 0 {
        return Some(("baud", 0, "a baud is above 0"));
    }
    if !(7..=9).contains(&framing.data_bits) {
        return Some((
            "data_bits",
            u32::from(framing.data_bits),
            "7 to 9 data bits",
        ));
    }
    if framing.m_bits().is_none() {
        return Some((
            "parity",
            u32::from(framing.word_bits()),
            "the data bits and a parity bit make a word of 7 to 9 bits (RM0433 Rev 9 §48.8.1, `USART_CR1.M[1:0]`), and this many is the word",
        ));
    }
    if framing.stop_field().is_none() {
        return Some((
            "stop_bits",
            u32::from(framing.stop_bits),
            "1 or 2 stop bits (RM0433 Rev 9 §48.8.2, `USART_CR2.STOP` `00` or `10`)",
        ));
    }
    None
}

/// Whether `peripheral` is a USART, UART or LPUART name: an assignment on
/// one belongs to a serial port.
fn is_serial_peripheral(peripheral: &str) -> bool {
    ["USART", "UART", "LPUART"]
        .iter()
        .any(|prefix| peripheral.starts_with(prefix))
}

/// The serial step of [`crate::SolvedTopology::validate`]: `serial_ports`
/// agrees with `assignments` and with what this module recomputes from the
/// topology's clock and each port's framing. Labels and nets were checked as
/// identifiers before.
pub(crate) fn validate_solved(topology: &SolvedTopology) -> Result<(), IrValidationError> {
    let mismatch = |id: &str| IrValidationError::SerialPortMismatch { id: id.to_owned() };
    let mut owners: BTreeMap<&str, &str> = BTreeMap::new();
    for (label, port) in &topology.serial_ports {
        for net in [&port.tx, &port.rx] {
            if owners.insert(net, label).is_some() {
                return Err(mismatch(label));
            }
        }
    }
    for (net, a) in &topology.assignments {
        let owned = owners
            .get(net.as_str())
            .is_some_and(|label| topology.serial_ports[*label].peripheral == a.peripheral);
        if is_serial_peripheral(&a.peripheral) != owned {
            return Err(mismatch(net));
        }
    }
    let mut indices = BTreeSet::new();
    let mut peripherals = BTreeSet::new();
    for (label, port) in &topology.serial_ports {
        let p = port.peripheral.as_str();
        let on = |net: &str, signal: &str| {
            topology
                .assignments
                .get(net)
                .is_some_and(|a| a.peripheral == p && a.signal == format!("{p}_{signal}"))
        };
        let in_range = usize::try_from(port.index).is_ok_and(|i| i < topology.serial_ports.len());
        let f = &port.framing;
        let solved = kernel_hz(&topology.clock, p).is_some_and(|k| {
            k == port.kernel_hz
                && solve_brr(k, f.baud) == Some(port.brr)
                && port.actual_millibaud == actual_millibaud(k, port.brr)
                && port.error_ppm == error_ppm(k, port.brr, f.baud)
        });
        if !PERIPHERALS.contains(&p)
            || !on(&port.tx, "TX")
            || !on(&port.rx, "RX")
            || !in_range
            || !indices.insert(port.index)
            || !peripherals.insert(p)
            || shape_error(f).is_some()
            || contradiction(port.role, f).is_some()
            || !solved
        {
            return Err(mismatch(label));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn framing(baud: u32, data_bits: u8, parity: Parity, stop_bits: u8) -> Framing {
        Framing {
            baud,
            data_bits,
            parity,
            stop_bits,
            inverted: false,
        }
    }

    #[test]
    fn the_reset_clock_gives_64_mhz_to_every_usart() {
        let clock = SolvedClock::reset();
        for p in PERIPHERALS {
            assert_eq!(kernel_hz(&clock, p), Some(64_000_000), "{p}");
        }
        assert_eq!(kernel_hz(&clock, LPUART), None);
    }

    #[test]
    fn the_work_order_s_bauds_solve_inside_their_rows() {
        // (baud, word, BRR, table, error ppm): FEEDBACK §0.
        let cases = [
            (
                115_200,
                framing(115_200, 8, Parity::None, 1),
                556,
                "Table 407",
                -799,
            ),
            (
                57_600,
                framing(57_600, 8, Parity::None, 1),
                1111,
                "Table 407",
                100,
            ),
            (
                420_000,
                framing(420_000, 8, Parity::None, 1),
                152,
                "Table 407",
                2506,
            ),
            (
                100_000,
                framing(100_000, 8, Parity::Even, 2),
                640,
                "Table 406",
                0,
            ),
        ];
        for (baud, f, brr, table, ppm) in cases {
            assert_eq!(solve_brr(64_000_000, baud), Some(brr), "{baud}");
            let row = tolerance(brr, f.m_bits().unwrap()).unwrap();
            assert_eq!(row.table, table, "{baud}");
            assert_eq!(error_ppm(64_000_000, brr, baud), ppm, "{baud}");
        }
    }

    #[test]
    fn brr_out_of_range_is_none() {
        // 64 MHz / 16 = 4 Mbaud is the fastest; 64 MHz / 0xFFFF ≈ 976.6 the slowest.
        assert_eq!(solve_brr(64_000_000, 4_000_000), Some(16));
        assert_eq!(solve_brr(64_000_000, 4_200_000), None);
        assert_eq!(solve_brr(64_000_000, 977), Some(65_507));
        assert_eq!(solve_brr(64_000_000, 900), None);
        assert_eq!(solve_brr(64_000_000, 0), None);
    }

    #[test]
    fn the_word_includes_parity() {
        assert_eq!(framing(100_000, 8, Parity::Even, 2).m_bits(), Some(0b01));
        assert_eq!(framing(115_200, 8, Parity::None, 1).m_bits(), Some(0b00));
        assert_eq!(framing(115_200, 7, Parity::None, 1).m_bits(), Some(0b10));
        assert_eq!(framing(115_200, 7, Parity::Odd, 1).m_bits(), Some(0b00));
        assert_eq!(framing(115_200, 9, Parity::Even, 1).m_bits(), None);
        assert_eq!(framing(115_200, 8, Parity::None, 3).stop_field(), None);
    }

    #[test]
    fn crsf_refuses_what_the_text_makes_unambiguous() {
        assert_eq!(
            contradiction(SerialRole::RcCrsf, &framing(420_000, 8, Parity::None, 1)),
            None
        );
        assert_eq!(
            contradiction(SerialRole::RcCrsf, &framing(416_666, 8, Parity::None, 1)),
            None
        );
        let refused = |f: Framing| contradiction(SerialRole::RcCrsf, &f).map(|(k, _)| k);
        assert_eq!(refused(framing(400_000, 8, Parity::None, 1)), Some("baud"));
        assert_eq!(
            refused(framing(420_000, 8, Parity::Even, 1)),
            Some("framing")
        );
        assert_eq!(
            refused(framing(420_000, 8, Parity::None, 2)),
            Some("framing")
        );
        let mut inv = framing(420_000, 8, Parity::None, 1);
        inv.inverted = true;
        assert_eq!(refused(inv), Some("inverted"));
        // SBUS has no vendor-published source: nothing is refused on its behalf.
        let mut sbus = framing(115_200, 7, Parity::Odd, 1);
        sbus.inverted = false;
        assert_eq!(contradiction(SerialRole::RcSbus, &sbus), None);
    }

    #[test]
    fn the_check_refuses_a_far_rounding_and_a_foreign_clock() {
        // The nearest BRR holds.
        let clock = SolvedClock::reset();
        let mut port = SolvedSerialPort {
            index: 0,
            peripheral: "USART2".into(),
            tx: "T".into(),
            rx: "R".into(),
            role: SerialRole::Console,
            framing: framing(115_200, 8, Parity::None, 1),
            kernel_hz: 64_000_000,
            brr: 556,
            actual_millibaud: 0,
            error_ppm: 0,
        };
        assert!(check_silicon(&clock, &port).holds);
        // The far rounding (555) is not the nearest: refused even inside the row.
        port.brr = 555;
        assert!(!check_silicon(&clock, &port).holds);
        // A kernel clock unlike the topology's is refused.
        port.brr = 556;
        port.kernel_hz = 32_000_000;
        assert!(!check_silicon(&clock, &port).holds);
    }
}
