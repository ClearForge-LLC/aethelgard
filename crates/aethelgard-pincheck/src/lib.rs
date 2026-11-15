//! The pinout check (`AET-WO-4400a`, ruling D7 on PR #130): what a linked
//! STM32H7 firmware writes to its GPIO registers at reset ([`elf`], with
//! [`header`] as a cross-check), judged against the `SolvedTopology`, a
//! declared allow-list and the package's pins by [`judge`]. The rules, the
//! definition of a configured pin and the blind spots are in `README.md`.

#![forbid(unsafe_code)]

pub mod elf;
pub mod header;

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
    #[error("the topology's {net} is on `{pin}`, which is not a pin P[A-K]0-15")]
    InvalidPin { net: String, pin: String },
}

impl Refusal {
    /// The refusal's name for `error.kind`.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Elf { kind, .. } | Self::Allow { kind, .. } => kind,
            Self::Header { .. } => "MalformedHeader",
            Self::DecodesDisagree { .. } => "DecodesDisagree",
            Self::UnsupportedPart { .. } => "UnsupportedPart",
            Self::InvalidPin { .. } => "InvalidPin",
        }
    }

    /// What `error.id` names: the macro, the register, the pin or the net.
    pub fn id(&self) -> Option<String> {
        match self {
            Self::Header { name, .. } => Some(name.clone()),
            Self::DecodesDisagree { port, register, .. } => Some(format!("GPIO{port}_{register}")),
            Self::InvalidPin { net, .. } => Some(net.clone()),
            Self::UnsupportedPart { part } => Some(part.clone()),
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

/// Judges `elf` (and `header`, if given) against `topology`, `package` and `allow`.
pub fn judge(
    topology: &SolvedTopology,
    package: &PinTable,
    elf: &GpioConfig,
    header: Option<&[HeaderPort; PORTS.len()]>,
    allow: &AllowList,
) -> Result<Judgement, Refusal> {
    if package.part != PART {
        return Err(Refusal::UnsupportedPart {
            part: package.part.clone(),
        });
    }
    if let Some(header) = header {
        cross_check(elf, header)?;
    }
    let parse = |net: &String, pin: &String| {
        PinId::parse(pin).ok_or_else(|| Refusal::InvalidPin {
            net: net.clone(),
            pin: pin.clone(),
        })
    };
    // Each named pin: its net, and the AF it must carry (None for a GPIO output).
    let mut named: BTreeMap<PinId, (String, Option<u8>)> = BTreeMap::new();
    for (net, a) in &topology.assignments {
        named.insert(
            parse(net, &a.pin)?,
            (net.clone(), Some(a.alternate_function)),
        );
    }
    for (net, pin) in &topology.gpio {
        named.insert(parse(net, pin)?, (net.clone(), None));
    }
    if let Some((pin, _)) = allow.0.iter().find(|(pin, _)| named.contains_key(pin)) {
        return allow_refusal(
            "AllowNamesTopologyPin",
            format!(
                "{pin} is the topology's {}; the topology judges it",
                named[pin].0
            ),
        );
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
    Ok(Judgement {
        ok: findings.is_empty(),
        decodes: if header.is_some() {
            "elf and hwdef.h agree"
        } else {
            "elf only"
        },
        configured,
        findings,
    })
}
