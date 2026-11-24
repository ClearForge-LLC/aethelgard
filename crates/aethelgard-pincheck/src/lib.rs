//! The pinout check (`AET-WO-4400a`, ruling D7 on PR #130): what a linked
//! STM32H7 firmware writes to its GPIO registers at reset ([`elf`], with
//! [`header`] as a cross-check), judged against the `SolvedTopology`, a
//! declared allow-list and the package's pins by [`judge`]. The rules, the
//! definition of a configured pin and the blind spots are in `README.md`.

#![forbid(unsafe_code)]

pub mod elf;
pub mod header;
pub mod identity;

use std::collections::BTreeMap;
use std::fmt;

use aethelgard_ir::{PinTable, SolvedTopology};
use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use header::HeaderPort;

/// `STM32_HAS_GPIOA` to `K` on the H753: the ports `gpio_default_config` holds, in order.
pub const PORTS: [char; 11] = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K'];

/// The one part whose register layout this check knows.
pub const PART: &str = "STM32H753ZITx";

/// The SWD port, which every judged image keeps at its reset state, alternate
/// AF0 (RM0433 Rev 9 §11.4.1 and §11.4.9–10; [`PinId::reset`]), so a debugger
/// attaches after boot (`AET-WO-4410` (3), review 3 A11). The expected value
/// is this check's own reset table, never a generator's line (`INV-07`).
// belt-and-suspenders: the judge's own copy of the SWD pins, never
// `aethelgard-hwdef`'s `SWD_LINES` (INV-07, RUL-20: no shared table).
pub const SWD: [&str; 2] = ["PA13", "PA14"];

/// What writes `SYSCFG_PMCR` in the ArduPilot this check judges, a cited
/// source fact (`AET-WO-4410` D1, ruled 2026-11-23). LQFP144 pads 28 and 29
/// are `PC2_C` and `PC3_C` only while PC2SO and PC3SO hold their reset value
/// (RM0433 Rev 9 p. 549). No image the check reads holds `PMCR`: the one
/// writer at the tag is code, whose operands are instruction immediates, and
/// it never touches either bit. The twin's access log owns the check on a
/// running image (a Tviburi row, `SYSCFG_PMCR`).
pub mod pmcr {
    /// The source read: ArduPilot and its ChibiOS submodule.
    pub const SOURCE: &str = "ArduPilot Copter-4.7.2 (fb773567), ChibiOS d103983f";
    /// `SYSCFG_PMCR_PC2SO_Pos` 26 and `_PC3SO_Pos` 27
    /// (`os/common/ext/ST/STM32H7xx/stm32h753xx.h:19071`, `:19074`).
    pub const PC2SO_PC3SO: u32 = (1 << 26) | (1 << 27);

    /// One writer: where it is, its text, and every bit it may change.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Writer {
        pub at: &'static str,
        pub text: &'static str,
        pub mask: u32,
    }

    /// Every writer of `SYSCFG->PMCR` in the source at the tag, outside the
    /// vendor headers (`spike/4410/RESULTS.md` M3): ChibiOS's `mac_lld_init`,
    /// under `#if defined(STM32H7XX)`, linked only with `HAL_USE_MAC`. Each is
    /// a masked read-modify-write: PA1SO (`_Pos` 25, `stm32h753xx.h:19068`)
    /// and EPIS (bits 23:21, `:19059-19060`).
    pub const WRITERS: [Writer; 3] = [
        Writer {
            at: "os/hal/ports/STM32/LLD/MACv2/hal_mac_lld.c:306",
            text: "SYSCFG->PMCR |= SYSCFG_PMCR_PA1SO;",
            mask: 1 << 25,
        },
        Writer {
            at: "os/hal/ports/STM32/LLD/MACv2/hal_mac_lld.c:308",
            text: "SYSCFG->PMCR = (SYSCFG->PMCR & ~SYSCFG_PMCR_EPIS_SEL_Msk) | SYSCFG_PMCR_EPIS_SEL_2;",
            mask: 0x7 << 21,
        },
        Writer {
            at: "os/hal/ports/STM32/LLD/MACv2/hal_mac_lld.c:310",
            text: "SYSCFG->PMCR &= ~SYSCFG_PMCR_EPIS_SEL_Msk;",
            mask: 0x7 << 21,
        },
    ];
}

/// One port's seven words, in `gpio_default_config`'s order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PortRegisters {
    pub moder: u32,
    pub otyper: u32,
    pub ospeedr: u32,
    pub pupdr: u32,
    pub odr: u32,
    pub afrl: u32,
    pub afrh: u32,
}

/// What `gpio_init` writes at reset, port A first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpioConfig {
    pub ports: [PortRegisters; PORTS.len()],
}

/// A `MODER` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Input,
    Output,
    Alternate,
    Analog,
}

impl Mode {
    fn from_bits(bits: u32) -> Self {
        match bits & 3 {
            0 => Self::Input,
            1 => Self::Output,
            2 => Self::Alternate,
            _ => Self::Analog,
        }
    }
}

/// A pin `P<port><n>`, `n` 0 to 15.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PinId {
    port: usize,
    n: u32,
}

impl PinId {
    /// Exactly `P[A-K](0|[1-9]|1[0-5])`.
    pub fn parse(name: &str) -> Option<Self> {
        let b = name.as_bytes();
        let port = PORTS.iter().position(|&p| b.get(1) == Some(&(p as u8)))?;
        let digits = name.get(2..)?;
        let canonical = !digits.is_empty()
            && digits.bytes().all(|d| d.is_ascii_digit())
            && (digits.len() == 1 || !digits.starts_with('0'));
        let n: u32 = digits.parse().ok().filter(|&n| canonical && n < 16)?;
        (b[0] == b'P').then_some(Self { port, n })
    }

    fn all() -> impl Iterator<Item = Self> {
        (0..PORTS.len()).flat_map(|port| (0..16).map(move |n| Self { port, n }))
    }

    /// RM0433 Rev 9 §11.4.1 and §11.4.9–10: the debug pins reset to alternate
    /// AF0, every other pin to analog.
    fn reset(self) -> (Mode, u8) {
        match (PORTS[self.port], self.n) {
            ('A', 13..=15) | ('B', 3 | 4) => (Mode::Alternate, 0),
            _ => (Mode::Analog, 0),
        }
    }
}

impl fmt::Display for PinId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P{}{}", PORTS[self.port], self.n)
    }
}

impl GpioConfig {
    /// A pin's mode and alternate function.
    pub fn pin(&self, pin: PinId) -> (Mode, u8) {
        let r = &self.ports[pin.port];
        let afr = if pin.n < 8 { r.afrl } else { r.afrh };
        let af = (afr >> (4 * (pin.n % 8))) & 0xf;
        (Mode::from_bits(r.moder >> (2 * pin.n)), af as u8)
    }

    /// A pin's `ODR` bit (RM0433 Rev 9 §11.4.6): the level an output drives
    /// from its first write (`AET-WO-4410` (4)).
    pub fn odr(&self, pin: PinId) -> u32 {
        (self.ports[pin.port].odr >> pin.n) & 1
    }

    /// A pin's `OTYPER` bit, `OSPEEDR` field and `PUPDR` field (RM0433
    /// Rev 9 §11.4.2–11.4.4), as the bytes hold them (`AET-WO-4400b` §1.3).
    pub fn drive(&self, pin: PinId) -> (u32, u32, u32) {
        let r = &self.ports[pin.port];
        (
            (r.otyper >> pin.n) & 1,
            (r.ospeedr >> (2 * pin.n)) & 3,
            (r.pupdr >> (2 * pin.n)) & 3,
        )
    }
}

/// A two-bit field as RM0433 writes it, `00` to `11`.
fn bits2(v: u32) -> String {
    format!("{:02b}", v & 3)
}

fn decoded(mode: Mode, af: u8) -> String {
    match mode {
        Mode::Alternate => format!("alternate AF{af}"),
        Mode::Input => "input".into(),
        Mode::Output => "output".into(),
        Mode::Analog => "analog".into(),
    }
}

/// Every input refused by name (`INV-06`). None of these is a verdict.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("{message}")]
    Elf { kind: &'static str, message: String },
    #[error("hwdef.h {name}: {message}")]
    Header { name: String, message: String },
    /// One of the 33 macros is named inside a conditional region the reader
    /// cannot evaluate exactly (`AET-WO-1603` §1.4).
    #[error(
        "hwdef.h {name}: named at line {line} inside a conditional other than exactly `#if 0` or `#if 1`, which the cross-check does not evaluate"
    )]
    Conditional { name: String, line: usize },
    #[error(
        "GPIO{port}_{register} is {elf:#010x} in the ELF and {header:#010x} in hwdef.h: two decodes of one image disagree"
    )]
    DecodesDisagree {
        port: char,
        register: &'static str,
        elf: u32,
        header: u32,
    },
    #[error("allow-list: {message}")]
    Allow { kind: &'static str, message: String },
    #[error("the pin table is for {part}; the check knows only {PART}")]
    UnsupportedPart { part: String },
    /// `AET-WO-1218` (`1217` R2): the topology names an MCU other than the
    /// pin table's, exactly. `SolvedTopology::validate` takes the analog
    /// pads from `topology.mcu`, so a renamed MCU would drop them unseen.
    #[error("the topology is for `{topology}`; the pin table is for `{table}`")]
    PartMismatch { topology: String, table: String },
    #[error("the topology's {net} is on `{pin}`, which is not a pin P[A-K]0-15")]
    InvalidPin { net: String, pin: String },
    /// A valid topology holds a speed for every pin; one that does not was
    /// not validated.
    #[error("the topology holds no pin speed for {net}")]
    NoPinSpeed { net: String },
    /// `AET-WO-1211a`: a pin device, or a gpio entry that is not a chip
    /// select, is a fact this check cannot yet judge from the image.
    #[error("`{id}` ({detail}) is a gpio fact the pinout check does not judge (AET-WO-1211a)")]
    GpioNotJudged { id: String, detail: String },
    /// `AET-WO-4410` (3): the SWD port is the check's to hold at reset.
    #[error("the topology's {net} is on {pin}, a pin of the SWD port the check holds at reset")]
    SwdPinNamed { net: String, pin: String },
}

/// Refuses a topology the check would mis-read: a pin device, or a gpio entry
/// that is not a chip select (`AET-WO-1211a`). Every gpio entry the check
/// reads after this is a chip select.
pub fn refuse_unjudged_gpio(t: &SolvedTopology) -> Result<(), Refusal> {
    if let Some((reference, d)) = t.pin_devices.iter().next() {
        return Err(Refusal::GpioNotJudged {
            id: reference.clone(),
            detail: d.part.clone(),
        });
    }
    if let Some((net, _)) = t.gpio.iter().find(|(_, g)| !g.is_high_output()) {
        return Err(Refusal::GpioNotJudged {
            id: net.clone(),
            detail: "not a chip select".to_owned(),
        });
    }
    Ok(())
}

impl Refusal {
    /// The refusal's name for `error.kind`.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Elf { kind, .. } | Self::Allow { kind, .. } => kind,
            Self::Header { .. } => "MalformedHeader",
            Self::Conditional { .. } => "HwdefConditionalUnsupported",
            Self::DecodesDisagree { .. } => "DecodesDisagree",
            Self::UnsupportedPart { .. } => "UnsupportedPart",
            Self::PartMismatch { .. } => "PartMismatch",
            Self::InvalidPin { .. } => "InvalidPin",
            Self::NoPinSpeed { .. } => "NoPinSpeed",
            Self::GpioNotJudged { .. } => "GpioNotJudged",
            Self::SwdPinNamed { .. } => "SwdPinNamed",
        }
    }

    /// What `error.id` names: the macro, the register, the pin or the net.
    pub fn id(&self) -> Option<String> {
        match self {
            Self::Header { name, .. } | Self::Conditional { name, .. } => Some(name.clone()),
            Self::DecodesDisagree { port, register, .. } => Some(format!("GPIO{port}_{register}")),
            Self::InvalidPin { net, .. }
            | Self::NoPinSpeed { net }
            | Self::SwdPinNamed { net, .. } => Some(net.clone()),
            Self::GpioNotJudged { id, .. } => Some(id.clone()),
            Self::UnsupportedPart { part } => Some(part.clone()),
            Self::PartMismatch { topology, .. } => Some(topology.clone()),
            Self::Elf { .. } | Self::Allow { .. } => None,
        }
    }
}

/// One pin of a declared allow-list: a pin the image may configure that the
/// topology does not name, exactly as declared, and why.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllowedPin {
    pub mode: Mode,
    #[serde(default)]
    pub alternate_function: Option<u8>,
    pub why: String,
}

/// A declared allow-list, keyed by pin. The check holds none of its own.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AllowList(BTreeMap<PinId, AllowedPin>);

struct UniqueMap(Vec<(String, AllowedPin)>);

impl<'de> Deserialize<'de> for UniqueMap {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = UniqueMap;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a map of pin to its declared configuration")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<UniqueMap, A::Error> {
                let mut out: Vec<(String, AllowedPin)> = Vec::new();
                while let Some((k, v)) = map.next_entry::<String, AllowedPin>()? {
                    if out.iter().any(|(seen, _)| *seen == k) {
                        return Err(de::Error::custom(format!("pin `{k}` is given twice")));
                    }
                    out.push((k, v));
                }
                Ok(UniqueMap(out))
            }
        }
        d.deserialize_map(V)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AllowFile {
    allow: UniqueMap,
}

fn allow_refusal<T>(kind: &'static str, message: impl Into<String>) -> Result<T, Refusal> {
    Err(Refusal::Allow {
        kind,
        message: message.into(),
    })
}

impl AllowList {
    /// `{"allow": {"PD8": {"mode": "alternate", "alternate_function": 7, "why": "..."}}}`.
    /// A pin is canonical; the mode is `output` or `alternate`, with an
    /// alternate function 0–15 exactly when alternate; `why` is not empty.
    pub fn parse(bytes: &[u8]) -> Result<Self, Refusal> {
        let file: AllowFile = match serde_json::from_slice(bytes) {
            Ok(f) => f,
            Err(e) => return allow_refusal("MalformedAllowList", e.to_string()),
        };
        let mut out = BTreeMap::new();
        for (name, pin) in file.allow.0 {
            let Some(id) = PinId::parse(&name) else {
                return allow_refusal(
                    "MalformedAllowList",
                    format!("`{name}` is not a pin P[A-K]0-15"),
                );
            };
            let shaped = match (pin.mode, pin.alternate_function) {
                (Mode::Alternate, Some(af)) => af < 16,
                (Mode::Output, None) => true,
                _ => false,
            };
            if !shaped || pin.why.trim().is_empty() {
                return allow_refusal(
                    "MalformedAllowList",
                    format!(
                        "{name}: an allowed pin is `output`, or `alternate` with an alternate_function 0-15, and says why"
                    ),
                );
            }
            out.insert(id, pin);
        }
        Ok(Self(out))
    }
}

/// One finding, named by pin, net and decode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    /// `forward`, `reverse` or `package`.
    pub check: &'static str,
    pub kind: &'static str,
    pub pin: String,
    pub net: Option<String>,
    pub expected: String,
    pub decoded: String,
}

/// A pin the image configures, and what names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConfiguredPin {
    pub pin: String,
    pub decoded: String,
    /// `topology <net>`, `allow-list`, or absent.
    pub named_by: Option<String>,
}

/// The verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Judgement {
    pub ok: bool,
    /// Whether `hwdef.h` was read and agreed, or only the ELF was read.
    pub decodes: &'static str,
    /// Whether the identity judge ran (`AET-WO-4400b2`): it needs `hwdef.h`.
    pub identity: &'static str,
    pub configured: Vec<ConfiguredPin>,
    pub findings: Vec<Finding>,
}

fn cross_check(elf: &GpioConfig, header: &[HeaderPort; PORTS.len()]) -> Result<(), Refusal> {
    for ((port, e), h) in PORTS.iter().zip(&elf.ports).zip(header) {
        for (register, a, b) in [
            ("MODER", e.moder, h.moder),
            ("AFRL", e.afrl, h.afrl),
            ("AFRH", e.afrh, h.afrh),
        ] {
            if a != b {
                return Err(Refusal::DecodesDisagree {
                    port: *port,
                    register,
                    elf: a,
                    header: b,
                });
            }
        }
    }
    Ok(())
}

/// Each pin the topology names: its net, and the AF it must carry (None for
/// a GPIO output).
type Named = BTreeMap<PinId, (String, Option<u8>)>;

/// Every refusal the topology and the package alone decide (`AET-WO-4410`
/// (6), `1218` F4): `check pinout` runs this before it reads the ELF or
/// `hwdef.h`, so a topology refusal is never hidden behind an unreadable
/// image. [`judge`] runs it first too.
pub fn admit(topology: &SolvedTopology, package: &PinTable) -> Result<(), Refusal> {
    named(topology, package)?;
    sensed(topology).map(|_| ())
}

/// Each battery sense pin, by pin, with its net (`AET-WO-4410` (1)): an
/// analog pin, so in neither `assignments` nor `gpio`. A pin outside
/// P[A-K]0-15, or an SWD pin, is refused as any topology pin is.
fn sensed(topology: &SolvedTopology) -> Result<BTreeMap<PinId, String>, Refusal> {
    let mut out = BTreeMap::new();
    for (net, s) in topology.power.iter().flat_map(|p| &p.sense) {
        out.insert(topology_pin(net, &s.pin)?, net.clone());
    }
    Ok(out)
}

/// A pin the topology names on `net`: one of `P[A-K]0-15` (`InvalidPin`),
/// and not the SWD port's (`SwdPinNamed`).
fn topology_pin(net: &str, pin: &str) -> Result<PinId, Refusal> {
    let id = PinId::parse(pin).ok_or_else(|| Refusal::InvalidPin {
        net: net.to_owned(),
        pin: pin.to_owned(),
    })?;
    if SWD.contains(&pin) {
        return Err(Refusal::SwdPinNamed {
            net: net.to_owned(),
            pin: pin.to_owned(),
        });
    }
    Ok(id)
}

fn named(topology: &SolvedTopology, package: &PinTable) -> Result<Named, Refusal> {
    if package.part != PART {
        return Err(Refusal::UnsupportedPart {
            part: package.part.clone(),
        });
    }
    if topology.mcu != package.part {
        return Err(Refusal::PartMismatch {
            topology: topology.mcu.clone(),
            table: package.part.clone(),
        });
    }
    refuse_unjudged_gpio(topology)?;
    let mut named = Named::new();
    for (net, a) in &topology.assignments {
        named.insert(
            topology_pin(net, &a.pin)?,
            (net.clone(), Some(a.alternate_function)),
        );
    }
    for (net, g) in &topology.gpio {
        named.insert(topology_pin(net, &g.pin)?, (net.clone(), None));
    }
    if let Some((_, (net, _))) = named
        .iter()
        .find(|(_, (net, _))| !topology.pin_speeds.contains_key(net))
    {
        return Err(Refusal::NoPinSpeed { net: net.clone() });
    }
    Ok(named)
}

/// Judges `elf` (and `header`, if given) against `topology`, `package` and `allow`.
pub fn judge(
    topology: &SolvedTopology,
    package: &PinTable,
    elf: &GpioConfig,
    header: Option<&[HeaderPort; PORTS.len()]>,
    allow: &AllowList,
) -> Result<Judgement, Refusal> {
    let named = named(topology, package)?;
    let sensed = sensed(topology)?;
    if let Some(header) = header {
        cross_check(elf, header)?;
    }
    if let Some(pin) = allow
        .0
        .keys()
        .find(|pin| SWD.contains(&pin.to_string().as_str()))
    {
        return allow_refusal(
            "AllowNamesSwdPin",
            format!("{pin} is the SWD port's; the check holds it at reset"),
        );
    }
    if let Some((pin, net)) = allow.0.keys().find_map(|pin| {
        named
            .get(pin)
            .map(|(net, _)| net)
            .or_else(|| sensed.get(pin))
            .map(|net| (pin, net))
    }) {
        return allow_refusal(
            "AllowNamesTopologyPin",
            format!("{pin} is the topology's {net}; the topology judges it"),
        );
    }

    let mut drive = Vec::new();
    // `AET-WO-4400b` §1.3: each topology pin's output type, speed and pull,
    // from the bytes, against the topology: the speed `solve` stored, and the
    // output type and pull `aethelgard-ir`'s rules imply, which no generator
    // calls (`RUL-20`).
    for (&pin, (net, _)) in &named {
        let (ot, speed, pupd) = elf.drive(pin);
        let want_ot = u32::from(aethelgard_ir::silicon::implied_open_drain(topology, net));
        // `named` holds a speed for every net.
        let want_speed = u32::from(topology.pin_speeds[net]);
        let want_pupd = aethelgard_ir::silicon::implied_pupd(topology, net);
        let otype = |v| if v == 1 { "open-drain" } else { "push-pull" };
        for (kind, want, got, show) in [
            (
                "WrongOutputType",
                want_ot,
                ot,
                (otype(want_ot).to_owned(), otype(ot).to_owned()),
            ),
            (
                "WrongSpeed",
                want_speed,
                speed,
                (
                    format!("OSPEEDR {}", bits2(want_speed)),
                    format!("OSPEEDR {}", bits2(speed)),
                ),
            ),
            (
                "WrongPull",
                want_pupd,
                pupd,
                (
                    format!("PUPDR {}", bits2(want_pupd)),
                    format!("PUPDR {}", bits2(pupd)),
                ),
            ),
        ] {
            if want != got {
                drive.push(Finding {
                    check: "forward",
                    kind,
                    pin: pin.to_string(),
                    net: Some(net.clone()),
                    expected: show.0,
                    decoded: show.1,
                });
            }
        }
    }
    // `AET-WO-4410` (4), review 3 A12: each GPIO output's `ODR` bit, the
    // level it drives from its first write, against its solved level.
    for (&pin, (net, _)) in &named {
        let Some(g) = topology.gpio.get(net) else {
            continue;
        };
        let want = u32::from(g.initial == Some(aethelgard_ir::motor::Level::High));
        let got = elf.odr(pin);
        if want != got {
            drive.push(Finding {
                check: "forward",
                kind: "WrongOutputLevel",
                pin: pin.to_string(),
                net: Some(net.clone()),
                expected: format!("ODR {want}"),
                decoded: format!("ODR {got}"),
            });
        }
    }
    let mut findings = Vec::new();
    let mut finding =
        |check, kind, pin: PinId, net: Option<&String>, expected: String, (mode, af)| {
            findings.push(Finding {
                check,
                kind,
                pin: pin.to_string(),
                net: net.cloned(),
                expected,
                decoded: decoded(mode, af),
            });
        };
    for (&pin, (net, want)) in &named {
        let (mode, af) = elf.pin(pin);
        match *want {
            Some(want) if mode != Mode::Alternate => {
                finding(
                    "forward",
                    "NotAlternate",
                    pin,
                    Some(net),
                    format!("alternate AF{want}"),
                    (mode, af),
                );
            }
            Some(want) if af != want => {
                finding(
                    "forward",
                    "WrongAlternateFunction",
                    pin,
                    Some(net),
                    format!("alternate AF{want}"),
                    (mode, af),
                );
            }
            None if mode != Mode::Output => {
                finding(
                    "forward",
                    "NotOutput",
                    pin,
                    Some(net),
                    "output".into(),
                    (mode, af),
                );
            }
            _ => {}
        }
    }
    // `AET-WO-4410` (1): a battery sense pin stays analog, ArduPilot's ADC
    // pin mode (an `ADC` line, `chibios_hwdef.py:335-336`).
    for (&pin, net) in &sensed {
        let (mode, af) = elf.pin(pin);
        if mode != Mode::Analog {
            finding(
                "forward",
                "NotAnalog",
                pin,
                Some(net),
                "analog".into(),
                (mode, af),
            );
        }
    }
    for name in SWD {
        let pin = PinId::parse(name).expect("a pin P[A-K]0-15");
        let (mode, af) = elf.pin(pin);
        if (mode, af) != pin.reset() {
            let (want_mode, want_af) = pin.reset();
            finding(
                "debug",
                "SwdNotKept",
                pin,
                None,
                decoded(want_mode, want_af),
                (mode, af),
            );
        }
    }
    for (&pin, declared) in &allow.0 {
        let (mode, af) = elf.pin(pin);
        let matches = mode == declared.mode
            && (mode != Mode::Alternate || Some(af) == declared.alternate_function);
        if !matches {
            let want = decoded(declared.mode, declared.alternate_function.unwrap_or(0));
            finding("reverse", "AllowedPinDiffers", pin, None, want, (mode, af));
        }
    }
    let mut configured = Vec::new();
    for pin in PinId::all() {
        let (mode, af) = elf.pin(pin);
        if !matches!(mode, Mode::Output | Mode::Alternate) || (mode, af) == pin.reset() {
            continue;
        }
        let named_by = match named.get(&pin) {
            Some((net, _)) => Some(format!("topology {net}")),
            None if allow.0.contains_key(&pin) => Some("allow-list".to_owned()),
            None => {
                finding(
                    "reverse",
                    "UnnamedPin",
                    pin,
                    None,
                    "reset or input".into(),
                    (mode, af),
                );
                None
            }
        };
        if !package.pins.contains_key(&pin.to_string()) {
            let expected = format!("a pin of {} ({})", package.part, package.package);
            finding("package", "NotInPackage", pin, None, expected, (mode, af));
        }
        configured.push(ConfiguredPin {
            pin: pin.to_string(),
            decoded: decoded(mode, af),
            named_by,
        });
    }
    findings.extend(drive);
    Ok(Judgement {
        ok: findings.is_empty(),
        decodes: if header.is_some() {
            "elf and hwdef.h agree"
        } else {
            "elf only"
        },
        identity: "not judged: no hwdef.h",
        configured,
        findings,
    })
}

impl Judgement {
    /// The judgement with the identity judge's findings added
    /// (`AET-WO-4400b2`, [`identity::judge`]).
    pub fn with_identity(mut self, findings: Vec<Finding>) -> Self {
        self.ok &= findings.is_empty();
        self.findings.extend(findings);
        self.identity = "judged";
        self
    }
}
