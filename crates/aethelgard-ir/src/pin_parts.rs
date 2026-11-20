//! The parts table's busless rows (`AET-WO-1211a`, parts schema 11, `1211`
//! Q6 as ruled): a part with `bus: "none"` and `identity: null`, bound by a
//! specification's `pin_devices`, never by a bus. A row carries what
//! `validate` judges a pin device's signals against: each pin's kind, its
//! internal pull and levels, the part's timing, and the truth table of each
//! strapped mode with its `drives` and `forbidden` set (Q3).
//!
//! Every value cites the held maker's document. **A value no held document
//! states is not entered** (`1211` §7, ruled): its key is absent, and a check
//! that needs it refuses by name (`PartLimitUndeclared`) unless the
//! specification declares it with a citation (`RUL-14`). Values printed in
//! the TYP column only are named `*_typical` and never used as bounds
//! (`RUL-19`). Symbols, footprints and passives are `AET-WO-1211d`'s.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::motor::Level;

/// A pin's role on its part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinKind {
    /// A logic input the MCU may drive.
    LogicInput,
    /// A logic input that enables the part, asserted at `active`.
    EnableInput {
        active: Level,
    },
    /// An open-drain output: its `VOL` at a sink current, and the largest
    /// sink current and pull-up voltage the part allows, when stated.
    OpenDrainOutput {
        vol_max_mv: u32,
        at_ma: u32,
        iod_max_ma: Option<u32>,
        pull_up_max_mv: Option<u32>,
    },
    /// A current output proportional to the load current: `A` (typical
    /// only), the largest current it may source, and its error bands.
    CurrentOutput {
        ua_per_a_typical: u32,
        output_max_ua: u32,
        error_band: Vec<ErrorBand>,
        /// The supply pin, and its range in mV, the error bands are stated
        /// for (SLVSDS7B p. 6: 5.5 V <= VVM <= 37 V): outside it no error is
        /// stated (`PartLimitUndeclared`).
        band_supply: (String, (u32, u32)),
    },
    /// A strap with discrete levels.
    Strap(StrapLevels),
    /// A strap read as a voltage, inside `range_mv`.
    StrapAnalog {
        range_mv: (u32, u32),
    },
    BridgeOutput,
    Supply {
        range_mv: (u32, u32),
    },
    Ground,
    /// A pin for a passive part only (a charge pump, a regulator's
    /// capacitor): `1211d`'s.
    Passive,
    /// A current-sense resistor's return.
    SenseReturn,
}

impl PinKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::LogicInput => "logic_input",
            Self::EnableInput { .. } => "enable_input",
            Self::OpenDrainOutput { .. } => "open_drain_output",
            Self::CurrentOutput { .. } => "current_output",
            Self::Strap(_) => "strap",
            Self::StrapAnalog { .. } => "strap_analog",
            Self::BridgeOutput => "bridge_output",
            Self::Supply { .. } => "supply",
            Self::Ground => "ground",
            Self::Passive => "passive",
            Self::SenseReturn => "sense_return",
        }
    }
}

/// The levels a discrete strap reads: tied to ground (`low`), tied at or
/// above `high_min_mv` (when the part has a high level), or a resistor to
/// ground inside one of `resistors` (min, max ohms).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrapLevels {
    pub high_min_mv: Option<u32>,
    pub resistors: Vec<(u32, u32)>,
}

/// One error band of a current output: over `load_ma` (inclusive), a
/// relative error in percent or an absolute one in µA of load current.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorBand {
    pub load_ma: (u32, u32),
    pub percent: Option<u32>,
    pub absolute_ua: Option<u32>,
}

/// One pin of a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowPin {
    /// The package pin, `None` for an exposed pad.
    pub number: Option<u16>,
    pub kind: PinKind,
    /// The part's internal pull-down on this pin, typical only (`RUL-19`).
    pub pull_down_ohms_typical: Option<u32>,
    /// This pin's own input levels where they differ from the part's.
    pub levels: Option<Levels>,
    pub page: String,
}

/// Input levels, in millivolts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Levels {
    pub vil_max_mv: u32,
    pub vih_min_mv: u32,
}

/// An output of a bridge in one state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Out {
    Z,
    L,
    H,
}

/// One row of a mode's truth table: the enable's level, each input's
/// (`None` is "don't care"), what the outputs do, and whether the row drives
/// the load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub name: String,
    pub enable: Level,
    pub inputs: [Option<Level>; 2],
    pub outputs: [Out; 2],
    pub drives: bool,
}

/// A pair of inputs and the pair of outputs they drive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeChannel {
    pub inputs: [String; 2],
    pub outputs: [String; 2],
}

/// One strapped mode of a bridge part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mode {
    /// `ph_en` or `in_in`.
    pub name: String,
    /// The level of the part's mode strap that selects it; `None` when the
    /// part has one mode and no strap.
    pub strap_level: Option<Level>,
    pub channels: Vec<BridgeChannel>,
    pub states: Vec<State>,
    /// The names of states no input may reach (empty for both parts held).
    pub forbidden: Vec<String>,
    pub page: String,
}

/// A value and where it was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stated {
    pub value: u32,
    pub page: String,
}

/// A busless part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinPart {
    pub part: String,
    /// The held maker's document: its name, revision and sha256.
    pub document: String,
    pub revision: String,
    pub sha256: String,
    pub logic_levels: Levels,
    /// The highest voltage a logic input takes.
    pub vin_max_mv: u32,
    pub logic_levels_page: String,
    /// The largest wake time after the enable asserts; absent when no held
    /// document states one.
    pub t_wake_max_us: Option<Stated>,
    /// The highest input PWM rate; absent when no held document states one
    /// (the DRV8833: `PartLimitUndeclared` unless declared).
    pub input_pwm_max_hz: Option<Stated>,
    /// The enable pin a bridge's idle reads.
    pub enable: Option<String>,
    /// The pin whose strap selects a mode, when the part has more than one.
    pub mode_strap: Option<String>,
    pub pins: BTreeMap<String, RowPin>,
    pub modes: Vec<Mode>,
}

impl PinPart {
    /// The input levels of `pin`: its own, else the part's.
    pub fn levels(&self, pin: &str) -> Levels {
        self.pins
            .get(pin)
            .and_then(|p| p.levels)
            .unwrap_or(self.logic_levels)
    }

    pub fn mode(&self, name: &str) -> Option<&Mode> {
        self.modes.iter().find(|m| m.name == name)
    }
}

impl Mode {
    /// The row a static tuple selects: the first whose enable matches and
    /// whose inputs match where they care.
    pub fn state(&self, enable: Level, inputs: [Level; 2]) -> Option<&State> {
        self.states.iter().find(|s| {
            s.enable == enable
                && s.inputs
                    .iter()
                    .zip(inputs)
                    .all(|(want, got)| want.is_none_or(|w| w == got))
        })
    }
}

// ---------------------------------------------------------------- the wire

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPinPart {
    part: String,
    bus: String,
    #[serde(deserialize_with = "Option::deserialize")]
    identity: Option<serde_json::Value>,
    source: RawSource,
    logic_levels: RawLevels,
    #[serde(default)]
    t_wake_max_us: Option<RawStated>,
    #[serde(default)]
    input_pwm_max_hz: Option<RawStated>,
    #[serde(deserialize_with = "Option::deserialize")]
    enable: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    mode_strap: Option<String>,
    #[serde(deserialize_with = "crate::unique_keys")]
    pins: BTreeMap<String, RawPin>,
    modes: Vec<RawMode>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSource {
    document: String,
    revision: String,
    sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLevels {
    vil_max_mv: u32,
    vih_min_mv: u32,
    vin_max_mv: u32,
    page: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStated {
    value: u32,
    page: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPin {
    #[serde(deserialize_with = "Option::deserialize")]
    number: Option<u16>,
    kind: String,
    page: String,
    #[serde(default)]
    active: Option<Level>,
    #[serde(default)]
    pull_down_ohms_typical: Option<u32>,
    #[serde(default)]
    vil_max_mv: Option<u32>,
    #[serde(default)]
    vih_min_mv: Option<u32>,
    #[serde(default)]
    vol_max_mv: Option<u32>,
    #[serde(default)]
    at_ma: Option<u32>,
    #[serde(default)]
    iod_max_ma: Option<u32>,
    #[serde(default)]
    pull_up_max_mv: Option<u32>,
    #[serde(default)]
    ua_per_a_typical: Option<u32>,
    #[serde(default)]
    output_max_ua: Option<u32>,
    #[serde(default)]
    error_band: Option<Vec<RawBand>>,
    #[serde(default)]
    error_band_supply_pin: Option<String>,
    #[serde(default)]
    error_band_supply_mv: Option<(u32, u32)>,
    #[serde(default)]
    range_mv: Option<(u32, u32)>,
    #[serde(default)]
    high_min_mv: Option<u32>,
    #[serde(default)]
    resistors: Option<Vec<(u32, u32)>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBand {
    load_ma: (u32, u32),
    #[serde(default)]
    percent: Option<u32>,
    #[serde(default)]
    absolute_ua: Option<u32>,
    #[serde(rename = "condition")]
    _condition: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMode {
    name: String,
    #[serde(deserialize_with = "Option::deserialize")]
    strap_level: Option<Level>,
    channels: Vec<RawChannel>,
    states: Vec<RawState>,
    forbidden: Vec<String>,
    page: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawChannel {
    inputs: [String; 2],
    outputs: [String; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawState {
    name: String,
    enable: Level,
    inputs: [String; 2],
    outputs: [String; 2],
    drives: bool,
}

/// One busless row, every key checked against its kind; the reason for a
/// refusal, which the caller names with the part.
pub(crate) fn pin_part(raw: RawPinPart) -> Result<PinPart, String> {
    if raw.bus != "none" {
        return Err(format!("`bus` is `{}`: a pin part's is `none`", raw.bus));
    }
    if raw.identity.is_some() {
        return Err("`identity` is not null: a part with no bus has no identity read".into());
    }
    if raw.source.sha256.len() != 64
        || !raw
            .source
            .sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("`source.sha256` is not 64 lowercase hex digits".into());
    }
    let l = &raw.logic_levels;
    if l.vil_max_mv >= l.vih_min_mv || l.vih_min_mv > l.vin_max_mv {
        return Err("`logic_levels` needs VIL max < VIH min <= VIN max".into());
    }
    let mut pins = BTreeMap::new();
    for (name, p) in raw.pins {
        crate::check_identifier(crate::IdentifierKind::Plain, &name.replace('/', "_"))
            .map_err(|rule| format!("pin `{name}`: {rule}"))?;
        let pin = row_pin(p).map_err(|e| format!("pin `{name}`: {e}"))?;
        pins.insert(name, pin);
    }
    let kind_of = |pin: &str| pins.get(pin).map(|p| &p.kind);
    for (name, p) in &pins {
        if let PinKind::CurrentOutput { band_supply, .. } = &p.kind
            && !matches!(kind_of(&band_supply.0), Some(PinKind::Supply { .. }))
        {
            return Err(format!(
                "pin `{name}`: `error_band_supply_pin` `{}` is not a supply pin",
                band_supply.0
            ));
        }
    }
    if let Some(e) = &raw.enable
        && !matches!(kind_of(e), Some(PinKind::EnableInput { .. }))
    {
        return Err(format!("`enable` `{e}` is not an enable_input pin"));
    }
    if let Some(s) = &raw.mode_strap
        && !matches!(kind_of(s), Some(PinKind::Strap(_)))
    {
        return Err(format!("`mode_strap` `{s}` is not a strap pin"));
    }
    let mut modes = Vec::new();
    for m in raw.modes {
        if !matches!(m.name.as_str(), "ph_en" | "in_in") {
            return Err(format!("mode `{}` is not `ph_en` or `in_in`", m.name));
        }
        if m.strap_level.is_some() != raw.mode_strap.is_some() {
            return Err(format!(
                "mode `{}`: `strap_level` is given exactly when the part has a `mode_strap`",
                m.name
            ));
        }
        if raw.enable.is_none() {
            return Err("a part with modes names its `enable` pin".into());
        }
        let mut channels = Vec::new();
        for c in m.channels {
            for i in &c.inputs {
                if !matches!(kind_of(i), Some(PinKind::LogicInput)) {
                    return Err(format!("mode `{}`: `{i}` is not a logic_input pin", m.name));
                }
            }
            for o in &c.outputs {
                if !matches!(kind_of(o), Some(PinKind::BridgeOutput)) {
                    return Err(format!(
                        "mode `{}`: `{o}` is not a bridge_output pin",
                        m.name
                    ));
                }
            }
            channels.push(BridgeChannel {
                inputs: c.inputs,
                outputs: c.outputs,
            });
        }
        if channels.is_empty() || m.states.is_empty() {
            return Err(format!("mode `{}` has no channel or no state", m.name));
        }
        let mut states = Vec::new();
        for s in m.states {
            let input = |t: &str| match t {
                "x" => Ok(None),
                "0" => Ok(Some(Level::Low)),
                "1" => Ok(Some(Level::High)),
                _ => Err(format!("state `{}`: input `{t}` is not 0, 1 or x", s.name)),
            };
            let out = |t: &str| match t {
                "Z" => Ok(Out::Z),
                "L" => Ok(Out::L),
                "H" => Ok(Out::H),
                _ => Err(format!("state `{}`: output `{t}` is not Z, L or H", s.name)),
            };
            let outputs = [out(&s.outputs[0])?, out(&s.outputs[1])?];
            // A row drives exactly when its outputs differ and neither floats.
            let drives = outputs[0] != outputs[1] && !outputs.contains(&Out::Z);
            if drives != s.drives {
                return Err(format!(
                    "state `{}`: `drives` is {} but its outputs say {drives}",
                    s.name, s.drives
                ));
            }
            states.push(State {
                inputs: [input(&s.inputs[0])?, input(&s.inputs[1])?],
                outputs,
                drives,
                enable: s.enable,
                name: s.name,
            });
        }
        if let Some(f) = m
            .forbidden
            .iter()
            .find(|f| !states.iter().any(|s| &s.name == *f))
        {
            return Err(format!(
                "mode `{}`: forbidden state `{f}` is not a row",
                m.name
            ));
        }
        modes.push(Mode {
            name: m.name,
            strap_level: m.strap_level,
            channels,
            states,
            forbidden: m.forbidden,
            page: m.page,
        });
    }
    Ok(PinPart {
        part: raw.part,
        document: raw.source.document,
        revision: raw.source.revision,
        sha256: raw.source.sha256,
        logic_levels: Levels {
            vil_max_mv: l.vil_max_mv,
            vih_min_mv: l.vih_min_mv,
        },
        vin_max_mv: l.vin_max_mv,
        logic_levels_page: raw.logic_levels.page,
        t_wake_max_us: raw.t_wake_max_us.map(|s| Stated {
            value: s.value,
            page: s.page,
        }),
        input_pwm_max_hz: raw.input_pwm_max_hz.map(|s| Stated {
            value: s.value,
            page: s.page,
        }),
        enable: raw.enable,
        mode_strap: raw.mode_strap,
        pins,
        modes,
    })
}

/// Every field of a current output, each required.
const CURRENT_OUTPUT_FIELDS: &[&str] = &[
    "ua_per_a_typical",
    "output_max_ua",
    "error_band",
    "error_band_supply_pin",
    "error_band_supply_mv",
];

/// A pin's fields, exactly those its kind takes.
fn row_pin(p: RawPin) -> Result<RowPin, String> {
    let given: Vec<&str> = [
        ("active", p.active.is_some()),
        ("pull_down_ohms_typical", p.pull_down_ohms_typical.is_some()),
        ("vil_max_mv", p.vil_max_mv.is_some()),
        ("vih_min_mv", p.vih_min_mv.is_some()),
        ("vol_max_mv", p.vol_max_mv.is_some()),
        ("at_ma", p.at_ma.is_some()),
        ("iod_max_ma", p.iod_max_ma.is_some()),
        ("pull_up_max_mv", p.pull_up_max_mv.is_some()),
        ("ua_per_a_typical", p.ua_per_a_typical.is_some()),
        ("output_max_ua", p.output_max_ua.is_some()),
        ("error_band", p.error_band.is_some()),
        ("error_band_supply_pin", p.error_band_supply_pin.is_some()),
        ("error_band_supply_mv", p.error_band_supply_mv.is_some()),
        ("range_mv", p.range_mv.is_some()),
        ("high_min_mv", p.high_min_mv.is_some()),
        ("resistors", p.resistors.is_some()),
    ]
    .into_iter()
    .filter(|(_, set)| *set)
    .map(|(k, _)| k)
    .collect();
    let (allowed, required): (&[&str], &[&str]) = match p.kind.as_str() {
        "logic_input" => (&["pull_down_ohms_typical"], &[]),
        "enable_input" => (
            &[
                "active",
                "pull_down_ohms_typical",
                "vil_max_mv",
                "vih_min_mv",
            ],
            &["active"],
        ),
        "open_drain_output" => (
            &["vol_max_mv", "at_ma", "iod_max_ma", "pull_up_max_mv"],
            &["vol_max_mv", "at_ma"],
        ),
        "current_output" => (CURRENT_OUTPUT_FIELDS, CURRENT_OUTPUT_FIELDS),
        "strap" => (&["high_min_mv", "resistors"], &["resistors"]),
        "strap_analog" | "supply" => (&["range_mv"], &["range_mv"]),
        "bridge_output" | "ground" | "passive" | "sense_return" => (&[], &[]),
        other => return Err(format!("kind `{other}` is not known")),
    };
    if let Some(k) = given.iter().find(|k| !allowed.contains(k)) {
        return Err(format!("`{k}` is not a field of a {} pin", p.kind));
    }
    if let Some(k) = required.iter().find(|k| !given.contains(k)) {
        return Err(format!("a {} pin states `{k}`", p.kind));
    }
    let range = |r: (u32, u32)| {
        if r.0 <= r.1 {
            Ok(r)
        } else {
            Err("`range_mv` runs low to high".to_owned())
        }
    };
    let kind = match p.kind.as_str() {
        "logic_input" => PinKind::LogicInput,
        "enable_input" => PinKind::EnableInput {
            active: p.active.expect("required"),
        },
        "open_drain_output" => PinKind::OpenDrainOutput {
            vol_max_mv: p.vol_max_mv.expect("required"),
            at_ma: p.at_ma.expect("required"),
            iod_max_ma: p.iod_max_ma,
            pull_up_max_mv: p.pull_up_max_mv,
        },
        "current_output" => {
            let bands = p.error_band.expect("required");
            if bands.is_empty() {
                return Err("`error_band` is empty".into());
            }
            let mut error_band = Vec::new();
            for b in bands {
                if b.percent.is_some() == b.absolute_ua.is_some() || b.load_ma.0 > b.load_ma.1 {
                    return Err(
                        "each error band runs low to high and states one of `percent` and `absolute_ua`"
                            .into(),
                    );
                }
                error_band.push(ErrorBand {
                    load_ma: b.load_ma,
                    percent: b.percent,
                    absolute_ua: b.absolute_ua,
                });
            }
            PinKind::CurrentOutput {
                ua_per_a_typical: p.ua_per_a_typical.expect("required"),
                output_max_ua: p.output_max_ua.expect("required"),
                error_band,
                band_supply: (
                    p.error_band_supply_pin.expect("required"),
                    range(p.error_band_supply_mv.expect("required"))?,
                ),
            }
        }
        "strap" => {
            let resistors = p.resistors.expect("required");
            if resistors.iter().any(|r| r.0 == 0 || r.0 > r.1) {
                return Err("each strap resistor level runs low to high, above 0".into());
            }
            PinKind::Strap(StrapLevels {
                high_min_mv: p.high_min_mv,
                resistors,
            })
        }
        "strap_analog" => PinKind::StrapAnalog {
            range_mv: range(p.range_mv.expect("required"))?,
        },
        "supply" => PinKind::Supply {
            range_mv: range(p.range_mv.expect("required"))?,
        },
        "bridge_output" => PinKind::BridgeOutput,
        "ground" => PinKind::Ground,
        "passive" => PinKind::Passive,
        _ => PinKind::SenseReturn,
    };
    let levels = match (p.vil_max_mv, p.vih_min_mv) {
        (None, None) => None,
        (Some(vil_max_mv), Some(vih_min_mv)) if vil_max_mv < vih_min_mv => Some(Levels {
            vil_max_mv,
            vih_min_mv,
        }),
        _ => return Err("a pin's own levels state both VIL max and VIH min, VIL below".into()),
    };
    Ok(RowPin {
        number: p.number,
        kind,
        pull_down_ohms_typical: p.pull_down_ohms_typical,
        levels,
        page: p.page,
    })
}
