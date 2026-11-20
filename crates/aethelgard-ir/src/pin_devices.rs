//! A device with no bus (`AET-WO-1211a`, `1211` §2 as ruled Q1–Q10): a
//! driver IC bound to a busless row of the parts table
//! ([`crate::pin_parts`]), with its supplies, straps, GPIO signals, ADC
//! inputs and bridges. The specification's `pin_devices` list is private and
//! read through [`Specification::declared_pin_devices`] (`RUL-16`).
//!
//! [`validate`] is `validate`'s step 24: every key is required and nothing
//! substitutes a default (`INV-06`). A bridge judges its **idle** only (Q3):
//! each input idles at a `level` (`BridgeIdlePulsed`), and the static idle
//! tuple (the enable's `initial`, each input's level) must be a non-driving
//! row of the part's truth table in its strapped mode (`BridgeIdleDrives`);
//! the tuple the part's own pulls hold from reset to the first write must be
//! one too (`UnheldDuringReset`, `1211` §2.6). Nothing is judged at run time:
//! no drive API exists.
//!
//! The solver places each signal like an SPI chip select and each ADC input
//! on an ADC1 or ADC3 channel ([`place`]); every other crate refuses a
//! topology that carries a pin device, by name, until its work order lands.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::motor::{Idle, Level, MotorProtocol};
use crate::pin_parts::{PinKind, PinPart};
use crate::{IrValidationError, Specification, VoltageDomain};

// ---------------------------------------------------------------- the MCU

/// The MCU's output levels at 8 mA, `2.7 V <= VDD <= 3.6 V` (DS12117 Rev 10
/// Table 50 rev Y p. 139 = Table 147 rev V p. 243, CMOS port): `VOL` at most
/// 0.4 V, `VOH` at least `VDD - 0.4 V`.
pub const MCU_VOL_MAX_MV: u32 = 400;
pub const MCU_VOH_DROP_MV: u32 = 400;
pub const MCU_IO_LEVELS: &str = "DS12117 Rev 10 Table 50 p. 139 (rev Y) = Table 147 p. 243 (rev V): VOL <= 0.4 V, VOH >= VDD - 0.4 V at 8 mA";
/// The MCU's weak pull-up, at least (DS12117 Rev 10 Table 49 rev Y p. 137 =
/// Table 146 rev V p. 241: `RPU` 30 to 50 kΩ).
pub const MCU_RPU_MIN_OHMS: u32 = 30_000;

// ---------------------------------------------------------------- the declaration

/// One device with no bus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct PinDevice {
    pub reference: String,
    /// A busless row of the parts table (`bus: none`).
    pub part: String,
    /// Each supply pin of the row to a declared voltage domain.
    #[serde(deserialize_with = "crate::unique_keys")]
    pub supplies: BTreeMap<String, String>,
    /// Each strap pin of the row to its strap.
    #[serde(deserialize_with = "crate::unique_keys")]
    pub straps: BTreeMap<String, Strap>,
    pub signals: Vec<GpioSignal>,
    pub analog: Vec<AnalogInput>,
    pub bridges: Vec<Bridge>,
    /// A limit the row does not state, declared with its citation (`1211`
    /// §7 as ruled); one the row states is refused here
    /// (`PartLimitDeclaredTwice`).
    pub part_limits: PartLimits,
    /// The range the device's resistors are judged over: `null` when the
    /// specification's `power.conditions` declares the board temperature,
    /// else required (`TemperatureUndeclared`, `TemperatureDeclaredTwice`).
    #[serde(deserialize_with = "Option::deserialize")]
    pub temperature_celsius: Option<crate::power::Temperature>,
}

/// A strap: tied to `GND` or a voltage domain, or a resistor to ground.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Strap {
    Tie(String),
    Resistor(crate::power::Resistor),
}

/// Which way a GPIO signal points, seen from the MCU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Output,
    Input,
}

/// The MCU's own pull (`PUPDR`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McuPull {
    Up,
    Down,
    None,
}

/// A GPIO signal bound to one device pin. An output states `initial`, the
/// level the firmware writes before `MODER` = output; an input states
/// `active` and `pull`. `pin` is `null` for the solver to choose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct GpioSignal {
    pub device_pin: String,
    pub net: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub pin: Option<String>,
    pub direction: Direction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial: Option<Level>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<Level>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull: Option<SignalPull>,
}

/// An input's pull: the MCU's (`mcu`) or a resistor (`resistor`), exactly
/// one, with its source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SignalPull {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcu: Option<McuPull>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resistor: Option<PullResistor>,
    pub source: String,
}

/// A pull resistor from the signal's net to a domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct PullResistor {
    pub reference: String,
    pub part: String,
    pub ohms: u32,
    pub to: String,
}

/// An ADC input bound to one device pin. `pin` and `channel` are `null` for
/// the solver to choose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct AnalogInput {
    pub device_pin: String,
    pub net: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub pin: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub channel: Option<AdcChannel>,
    pub transfer: Transfer,
    pub full_scale: FullScale,
    pub adc_resolution_bits: u8,
}

/// An ADC channel by hand: `{"peripheral": "ADC3", "input": "INP5"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct AdcChannel {
    pub peripheral: String,
    pub input: String,
}

/// How the quantity reaches the pin: a current output into a resistor to
/// ground, with a reservoir capacitor at the pin when the resistor is above
/// RAIN (the rule `power.sense` takes, `AET-WO-4300d` D10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Transfer {
    CurrentOutput(CurrentOutput),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct CurrentOutput {
    pub resistor: crate::power::Resistor,
    #[serde(deserialize_with = "Option::deserialize")]
    pub reservoir: Option<crate::power::Reservoir>,
}

/// The largest quantity the input converts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum FullScale {
    Milliamps(u32),
}

/// A bridge the device owns (`1211` Q2): its mode, each input pin to a motor
/// output's label or a signal's net (Q7), and each output pin to a net.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Bridge {
    pub id: String,
    pub mode: String,
    #[serde(deserialize_with = "crate::unique_keys")]
    pub inputs: BTreeMap<String, String>,
    #[serde(deserialize_with = "crate::unique_keys")]
    pub outputs: BTreeMap<String, String>,
}

/// Limits the row does not state, each declared with its citation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct PartLimits {
    #[serde(deserialize_with = "Option::deserialize")]
    pub input_pwm_max_hz: Option<SourcedValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SourcedValue {
    pub value: u32,
    pub source: String,
}

// ---------------------------------------------------------------- solved

/// One solved GPIO: its pin and direction, an output's initial level or an
/// input's MCU pull (`AET-WO-1211a`, solved version 14). An SPI chip select
/// is an output at `high`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedGpio {
    pub pin: String,
    pub direction: Direction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial: Option<Level>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull: Option<McuPull>,
}

impl SolvedGpio {
    /// An output at `initial`.
    pub fn output(pin: impl Into<String>, initial: Level) -> Self {
        Self {
            pin: pin.into(),
            direction: Direction::Output,
            initial: Some(initial),
            pull: None,
        }
    }

    /// Whether this is what every reader before `AET-WO-1211a` read: an
    /// output that starts high (a chip select).
    pub fn is_high_output(&self) -> bool {
        self.direction == Direction::Output && self.initial == Some(Level::High)
    }
}

/// One solved ADC input: its pin, the peripheral chosen, and the channel
/// (`INPm`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedAdcInput {
    pub pin: String,
    pub peripheral: String,
    pub channel: String,
}

/// A pin device as the topology carries it: its part. Every crate but the
/// IR refuses a topology with one, by name, until its work order lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedPinDevice {
    pub part: String,
}

crate::object_only!(
    PinDevice,
    GpioSignal,
    SignalPull,
    PullResistor,
    AnalogInput,
    AdcChannel,
    CurrentOutput,
    Bridge,
    PartLimits,
    SourcedValue,
    SolvedGpio,
    SolvedAdcInput,
    SolvedPinDevice
);

// ---------------------------------------------------------------- refusals

/// Each refusal of a pin device is its own kind ([`PinDeviceRefusal::kind`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinKindOfRefusal {
    PartHasBus,
    PartWithoutBus,
    SupplyPinUndeclared,
    UnknownDomain,
    SupplyOutOfRange,
    StrapUndeclared,
    StrapLevelInvalid,
    UnknownDevicePin,
    DevicePinBoundTwice,
    DevicePinUnbound,
    SignalKindMismatch,
    SignalShape,
    LogicLevelIncompatible,
    UnheldDuringReset,
    OpenDrainUnpulled,
    PullOverDrive,
    PartLimitUndeclared,
    PartLimitDeclaredTwice,
    TemperatureUndeclared,
    TemperatureDeclaredTwice,
    PassivePartUnknown,
    AdcFullScaleExceeded,
    AdcOutputCurrentExceeded,
    AdcSourceImpedance,
    BridgeModeUnknown,
    BridgeModeStrapMismatch,
    BridgeChannelUnknown,
    BridgeInputUnknown,
    BridgeInputShared,
    BridgeIdlePulsed,
    BridgeIdleDrives,
    BridgeForbiddenState,
    InputRateAbovePart,
    PinDeviceUndeclared,
}

impl PinKindOfRefusal {
    pub fn name(self) -> &'static str {
        use PinKindOfRefusal as K;
        match self {
            K::PartHasBus => "PartHasBus",
            K::PartWithoutBus => "PartWithoutBus",
            K::SupplyPinUndeclared => "SupplyPinUndeclared",
            K::UnknownDomain => "UnknownDomain",
            K::SupplyOutOfRange => "SupplyOutOfRange",
            K::StrapUndeclared => "StrapUndeclared",
            K::StrapLevelInvalid => "StrapLevelInvalid",
            K::UnknownDevicePin => "UnknownDevicePin",
            K::DevicePinBoundTwice => "DevicePinBoundTwice",
            K::DevicePinUnbound => "DevicePinUnbound",
            K::SignalKindMismatch => "SignalKindMismatch",
            K::SignalShape => "SignalShape",
            K::LogicLevelIncompatible => "LogicLevelIncompatible",
            K::UnheldDuringReset => "UnheldDuringReset",
            K::OpenDrainUnpulled => "OpenDrainUnpulled",
            K::PullOverDrive => "PullOverDrive",
            K::PartLimitUndeclared => "PartLimitUndeclared",
            K::PartLimitDeclaredTwice => "PartLimitDeclaredTwice",
            K::TemperatureUndeclared => "TemperatureUndeclared",
            K::TemperatureDeclaredTwice => "TemperatureDeclaredTwice",
            K::PassivePartUnknown => "PowerPartUnknown",
            K::AdcFullScaleExceeded => "AdcFullScaleExceeded",
            K::AdcOutputCurrentExceeded => "AdcOutputCurrentExceeded",
            K::AdcSourceImpedance => "AdcSourceImpedance",
            K::BridgeModeUnknown => "BridgeModeUnknown",
            K::BridgeModeStrapMismatch => "BridgeModeStrapMismatch",
            K::BridgeChannelUnknown => "BridgeChannelUnknown",
            K::BridgeInputUnknown => "BridgeInputUnknown",
            K::BridgeInputShared => "BridgeInputShared",
            K::BridgeIdlePulsed => "BridgeIdlePulsed",
            K::BridgeIdleDrives => "BridgeIdleDrives",
            K::BridgeForbiddenState => "BridgeForbiddenState",
            K::InputRateAbovePart => "InputRateAbovePart",
            K::PinDeviceUndeclared => "PinDeviceUndeclared",
        }
    }
}

/// A pin device refused: its kind, what it names (`id`), where (`path`), and
/// why.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{path}`: {message}")]
pub struct PinDeviceRefusal {
    pub kind: PinKindOfRefusal,
    pub id: String,
    pub path: String,
    pub message: String,
}

impl PinDeviceRefusal {
    pub fn kind(&self) -> &'static str {
        self.kind.name()
    }
}

fn refuse(
    kind: PinKindOfRefusal,
    id: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
) -> IrValidationError {
    IrValidationError::PinDevice(PinDeviceRefusal {
        kind,
        id: id.into(),
        path: path.into(),
        message: message.into(),
    })
}

use PinKindOfRefusal as K;

// ---------------------------------------------------------------- validate

/// Every net a pin device declares: its signals', its ADC inputs' and its
/// bridges' outputs, in document order.
pub(crate) fn nets(spec: &Specification) -> impl Iterator<Item = &String> {
    spec.declared_pin_devices().iter().flat_map(|d| {
        d.signals
            .iter()
            .map(|s| &s.net)
            .chain(d.analog.iter().map(|a| &a.net))
            .chain(d.bridges.iter().flat_map(|b| b.outputs.values()))
    })
}

/// Every reference a pin device declares: its own, then its resistors' and
/// capacitors', in document order.
pub(crate) fn references(spec: &Specification) -> Vec<&str> {
    let mut out = Vec::new();
    for d in spec.declared_pin_devices() {
        out.push(d.reference.as_str());
        for s in d.straps.values() {
            if let Strap::Resistor(r) = s {
                out.push(r.reference.as_str());
            }
        }
        for s in &d.signals {
            if let Some(r) = s.pull.as_ref().and_then(|p| p.resistor.as_ref()) {
                out.push(r.reference.as_str());
            }
        }
        for a in &d.analog {
            let Transfer::CurrentOutput(c) = &a.transfer;
            out.push(c.resistor.reference.as_str());
            if let Some(r) = &c.reservoir {
                out.push(r.reference.as_str());
            }
        }
    }
    out
}

/// What binds a device pin.
#[derive(Clone, Copy)]
enum Binding<'s> {
    Supply,
    Strap,
    Signal(&'s GpioSignal),
    Analog,
    BridgeOutput,
    /// A bridge input from a motor output.
    Motor(&'s crate::MotorOutput),
}

/// Step 24 (`AET-WO-1211a`): each pin device, in document order, against the
/// embedded parts table.
pub(crate) fn validate(spec: &Specification) -> Result<(), IrValidationError> {
    validate_against(spec, crate::parts::table()?)
}

/// Step 24 against `table`: a row the embedded table does not hold (a
/// forbidden state, an input with no pull) is judged by the same code.
pub fn validate_against(
    spec: &Specification,
    table: &crate::parts::PartsTable,
) -> Result<(), IrValidationError> {
    let mut motor_inputs: BTreeMap<&str, String> = BTreeMap::new();
    for (i, d) in spec.declared_pin_devices().iter().enumerate() {
        let path = format!("pin_devices[{i}]");
        let row = match table.pin_part(&d.part) {
            Some(r) => r,
            None if table.get(&d.part).is_some() => {
                return Err(refuse(
                    K::PartHasBus,
                    &d.reference,
                    format!("{path}.part"),
                    format!(
                        "`{}` is a part with a bus: declare it in `i2c_devices` or `spi_devices`, not in `pin_devices`",
                        d.part
                    ),
                ));
            }
            None => {
                return Err(IrValidationError::PartWithoutRow {
                    reference: d.reference.clone(),
                    part: d.part.clone(),
                });
            }
        };
        Device {
            spec,
            d,
            row,
            path,
            bound: BTreeMap::new(),
        }
        .check(&mut motor_inputs)?;
    }
    Ok(())
}

/// A bus device whose part is a busless row (`PartWithoutBus`), checked
/// where a bus device's part is looked up.
pub(crate) fn part_without_bus(
    reference: &str,
    part: &str,
    path: &str,
) -> Option<IrValidationError> {
    let table = crate::parts::table().ok()?;
    table.pin_part(part).map(|_| {
        refuse(
            K::PartWithoutBus,
            reference,
            path,
            format!("`{part}` has no bus (`bus: none`): declare it in `pin_devices`"),
        )
    })
}

struct Device<'s> {
    spec: &'s Specification,
    d: &'s PinDevice,
    row: &'s PinPart,
    path: String,
    bound: BTreeMap<&'s str, Binding<'s>>,
}

impl<'s> Device<'s> {
    fn err(
        &self,
        kind: PinKindOfRefusal,
        at: &str,
        message: impl Into<String>,
    ) -> IrValidationError {
        refuse(
            kind,
            &self.d.reference,
            format!("{}{at}", self.path),
            message,
        )
    }

    fn domain(&self, id: &str, at: &str) -> Result<&'s VoltageDomain, IrValidationError> {
        self.spec
            .voltage_domains
            .iter()
            .find(|v| v.id == id)
            .ok_or_else(|| {
                self.err(
                    K::UnknownDomain,
                    at,
                    format!("`{id}` is not a declared voltage domain"),
                )
            })
    }

    fn mcu_domain(&self) -> Result<&'s VoltageDomain, IrValidationError> {
        self.domain(&self.spec.mcu.supply, ".mcu.supply")
    }

    /// The row pin `name`, refused by name when the row has none.
    fn pin(&self, name: &str, at: &str) -> Result<&'s crate::pin_parts::RowPin, IrValidationError> {
        self.row.pins.get(name).ok_or_else(|| {
            self.err(
                K::UnknownDevicePin,
                at,
                format!(
                    "`{name}` is not a pin of `{}`; its pins: {}",
                    self.row.part,
                    self.row.pins.keys().cloned().collect::<Vec<_>>().join(", ")
                ),
            )
        })
    }

    fn bind(&mut self, pin: &'s str, how: Binding<'s>, at: &str) -> Result<(), IrValidationError> {
        if self.bound.insert(pin, how).is_some() {
            return Err(self.err(
                K::DevicePinBoundTwice,
                at,
                format!("device pin `{pin}` is bound twice"),
            ));
        }
        Ok(())
    }

    fn kind_mismatch(&self, pin: &str, kind: &PinKind, want: &str, at: &str) -> IrValidationError {
        self.err(
            K::SignalKindMismatch,
            at,
            format!("device pin `{pin}` is a {} pin; {want}", kind.name()),
        )
    }

    fn check(
        mut self,
        motor_inputs: &mut BTreeMap<&'s str, String>,
    ) -> Result<(), IrValidationError> {
        self.supplies()?;
        let strap_levels = self.straps()?;
        self.signals()?;
        self.analog()?;
        self.bridges(&strap_levels, motor_inputs)?;
        // Every input of the row that the MCU or a strap could hold is bound.
        for (name, p) in &self.row.pins {
            if matches!(p.kind, PinKind::LogicInput | PinKind::EnableInput { .. })
                && !self.bound.contains_key(name.as_str())
            {
                return Err(self.err(
                    K::DevicePinUnbound,
                    "",
                    format!(
                        "`{}`'s {} pin `{name}` is bound by no signal or bridge: an input left to its pull is not declared (`INV-06`)",
                        self.row.part,
                        p.kind.name()
                    ),
                ));
            }
        }
        self.limits()
    }

    fn supplies(&mut self) -> Result<(), IrValidationError> {
        let d = self.d;
        for (pin, domain) in &d.supplies {
            let at = format!(".supplies.{pin}");
            let p = self.pin(pin, &at)?;
            let PinKind::Supply { range_mv } = p.kind else {
                return Err(self.kind_mismatch(
                    pin,
                    &p.kind,
                    "only a supply pin takes a domain",
                    &at,
                ));
            };
            let v = self.domain(domain, &at)?;
            let (lo, hi) = (crate::minimum_millivolts(v), crate::maximum_millivolts(v));
            if lo < range_mv.0 || hi > range_mv.1 {
                return Err(self.err(
                    K::SupplyOutOfRange,
                    &at,
                    format!(
                        "`{domain}` spans {lo} to {hi} mV, outside `{pin}`'s {} to {} mV ({}, {})",
                        range_mv.0, range_mv.1, self.row.document, p.page
                    ),
                ));
            }
            self.bind(pin, Binding::Supply, &at)?;
        }
        for (name, p) in &self.row.pins {
            if matches!(p.kind, PinKind::Supply { .. }) && !d.supplies.contains_key(name) {
                return Err(self.err(
                    K::SupplyPinUndeclared,
                    ".supplies",
                    format!("`{}`'s supply pin `{name}` names no domain", self.row.part),
                ));
            }
        }
        Ok(())
    }

    /// Each strap's level, by pin: `low` or `high` for a tie, `None` for a
    /// resistor level or an analog strap.
    fn straps(&mut self) -> Result<BTreeMap<&'s str, Option<Level>>, IrValidationError> {
        let d = self.d;
        let mut levels = BTreeMap::new();
        for (pin, strap) in &d.straps {
            let at = format!(".straps.{pin}");
            let p = self.pin(pin, &at)?;
            let invalid = |why: String| {
                self.err(
                    K::StrapLevelInvalid,
                    &at,
                    format!("{why} ({}, {})", self.row.document, p.page),
                )
            };
            let level = match (&p.kind, strap) {
                (PinKind::Strap(_) | PinKind::StrapAnalog { .. }, Strap::Tie(to))
                    if to == crate::GND =>
                {
                    Some(Level::Low)
                }
                // A current-sense return tied to ground: no current control.
                (PinKind::SenseReturn, Strap::Tie(to)) if to == crate::GND => None,
                (PinKind::SenseReturn, _) => {
                    return Err(invalid(format!(
                        "`{pin}` is a current-sense return: only a tie to `GND` (no current control) is judged; a sense resistor is not modelled"
                    )));
                }
                (PinKind::Strap(s), Strap::Tie(to)) => {
                    let v = self.domain(to, &at)?;
                    let (lo, hi) = (crate::minimum_millivolts(v), crate::maximum_millivolts(v));
                    match s.high_min_mv {
                        Some(min) if lo >= min && hi <= self.row.vin_max_mv => Some(Level::High),
                        Some(min) => {
                            return Err(invalid(format!(
                                "`{to}` spans {lo} to {hi} mV; `{pin}` reads high from {min} mV to {} mV",
                                self.row.vin_max_mv
                            )));
                        }
                        None => return Err(invalid(format!("`{pin}` has no high level"))),
                    }
                }
                (PinKind::StrapAnalog { range_mv }, Strap::Tie(to)) => {
                    let v = self.domain(to, &at)?;
                    let (lo, hi) = (crate::minimum_millivolts(v), crate::maximum_millivolts(v));
                    if lo < range_mv.0 || hi > range_mv.1 {
                        return Err(invalid(format!(
                            "`{to}` spans {lo} to {hi} mV, outside `{pin}`'s {} to {} mV",
                            range_mv.0, range_mv.1
                        )));
                    }
                    None
                }
                (PinKind::Strap(s), Strap::Resistor(r)) => {
                    let e = resistor_error_ppm(self, r, &at)?;
                    let (lo, hi) = band(r.ohms, e);
                    if !s
                        .resistors
                        .iter()
                        .any(|(min, max)| lo >= i128::from(*min) && hi <= i128::from(*max))
                    {
                        return Err(invalid(format!(
                            "`{}` is {} ohms within {lo} to {hi} at its worst, inside none of `{pin}`'s resistor levels {:?}",
                            r.reference, r.ohms, s.resistors
                        )));
                    }
                    None
                }
                (PinKind::StrapAnalog { .. }, Strap::Resistor(_)) => {
                    return Err(invalid(format!(
                        "`{pin}` is read as a voltage; a resistor alone sets none"
                    )));
                }
                (kind, _) => {
                    return Err(self.kind_mismatch(
                        pin,
                        kind,
                        "only a strap pin takes a strap",
                        &at,
                    ));
                }
            };
            self.bind(pin, Binding::Strap, &at)?;
            levels.insert(pin.as_str(), level);
        }
        for (name, p) in &self.row.pins {
            if matches!(
                p.kind,
                PinKind::Strap(_) | PinKind::StrapAnalog { .. } | PinKind::SenseReturn
            ) && !d.straps.contains_key(name)
            {
                return Err(self.err(
                    K::StrapUndeclared,
                    ".straps",
                    format!("`{}`'s strap pin `{name}` is not declared", self.row.part),
                ));
            }
        }
        Ok(levels)
    }

    fn signals(&mut self) -> Result<(), IrValidationError> {
        let d = self.d;
        let mcu = self.mcu_domain()?;
        let mcu_hi = crate::maximum_millivolts(mcu);
        for (k, s) in d.signals.iter().enumerate() {
            let at = format!(".signals[{k}]");
            let p = self.pin(&s.device_pin, &format!("{at}.device_pin"))?;
            let shape = |why: &str| self.err(K::SignalShape, &at, why.to_owned());
            match s.direction {
                Direction::Output => {
                    if s.initial.is_none() || s.active.is_some() || s.pull.is_some() {
                        return Err(shape(
                            "an output states `initial` and neither `active` nor `pull`",
                        ));
                    }
                    if !matches!(p.kind, PinKind::LogicInput | PinKind::EnableInput { .. }) {
                        return Err(self.kind_mismatch(
                            &s.device_pin,
                            &p.kind,
                            "an output drives a logic_input or enable_input pin",
                            &at,
                        ));
                    }
                    self.drive_levels(&s.device_pin, mcu, &at)?;
                }
                Direction::Input => {
                    if s.initial.is_some() || s.active.is_none() || s.pull.is_none() {
                        return Err(shape(
                            "an input states `active` and `pull` and no `initial`",
                        ));
                    }
                    let PinKind::OpenDrainOutput {
                        at_ma,
                        iod_max_ma,
                        pull_up_max_mv,
                        ..
                    } = p.kind
                    else {
                        return Err(self.kind_mismatch(
                            &s.device_pin,
                            &p.kind,
                            "an input reads an open_drain_output pin",
                            &at,
                        ));
                    };
                    let pull = s.pull.as_ref().expect("checked");
                    let sink_ma = iod_max_ma.unwrap_or(at_ma);
                    match (&pull.mcu, &pull.resistor) {
                        (Some(McuPull::Up), None) => {
                            // RPU at least 30 kΩ to the MCU's supply.
                            self.pulled(
                                &at,
                                mcu_hi,
                                i128::from(MCU_RPU_MIN_OHMS),
                                sink_ma,
                                pull_up_max_mv,
                                &s.device_pin,
                            )?;
                        }
                        (Some(McuPull::Down | McuPull::None), None) => {
                            return Err(self.err(
                                K::OpenDrainUnpulled,
                                &format!("{at}.pull"),
                                format!(
                                    "`{}` is open drain: with no pull-up it never reads high",
                                    s.device_pin
                                ),
                            ));
                        }
                        (None, Some(r)) => {
                            let v = self.domain(&r.to, &format!("{at}.pull.resistor.to"))?;
                            let e = resistor_error_ppm(
                                self,
                                &crate::power::Resistor {
                                    reference: r.reference.clone(),
                                    part: r.part.clone(),
                                    ohms: r.ohms,
                                },
                                &format!("{at}.pull.resistor"),
                            )?;
                            let (r_lo, _) = band(r.ohms, e);
                            self.pulled(
                                &at,
                                crate::maximum_millivolts(v),
                                r_lo,
                                sink_ma,
                                pull_up_max_mv,
                                &s.device_pin,
                            )?;
                        }
                        _ => {
                            return Err(shape("`pull` states exactly one of `mcu` and `resistor`"));
                        }
                    }
                }
            }
            self.bind(&s.device_pin, Binding::Signal(s), &at)?;
        }
        Ok(())
    }

    /// The MCU drives `pin`: its `VOH` at the supply's low edge clears the
    /// pin's `VIH`, its `VOL` the pin's `VIL`, and its supply's high edge is
    /// inside the pin's input range; and the pin is held from reset to the
    /// first write (`1211` §2.6).
    fn drive_levels(
        &self,
        pin: &str,
        mcu: &VoltageDomain,
        at: &str,
    ) -> Result<(), IrValidationError> {
        let l = self.row.levels(pin);
        let (lo, hi) = (
            crate::minimum_millivolts(mcu),
            crate::maximum_millivolts(mcu),
        );
        let voh = lo.saturating_sub(MCU_VOH_DROP_MV);
        if voh < l.vih_min_mv || MCU_VOL_MAX_MV > l.vil_max_mv || hi > self.row.vin_max_mv {
            return Err(self.err(
                K::LogicLevelIncompatible,
                at,
                format!(
                    "the MCU on `{}` drives VOH >= {voh} mV and VOL <= {MCU_VOL_MAX_MV} mV up to {hi} mV ({MCU_IO_LEVELS}); `{pin}` needs VIH >= {} mV, VIL <= {} mV and at most {} mV ({}, {})",
                    mcu.id, l.vih_min_mv, l.vil_max_mv, self.row.vin_max_mv, self.row.document, self.row.logic_levels_page
                ),
            ));
        }
        if self.row.pins[pin].pull_down_ohms_typical.is_none() {
            return Err(self.err(
                K::UnheldDuringReset,
                at,
                format!(
                    "`{pin}` has no internal pull and no strap: from reset to the first write its MCU pin is analog (RM0433 Rev 9 p. 541) and nothing holds it"
                ),
            ));
        }
        Ok(())
    }

    /// A pull-up of at least `r_min` ohms to `v_max` mV on an open-drain
    /// pin: its current within what the part sinks, and its voltage within
    /// what the pin takes.
    fn pulled(
        &self,
        at: &str,
        v_max: u32,
        r_min: i128,
        sink_ma: u32,
        pull_up_max_mv: Option<u32>,
        pin: &str,
    ) -> Result<(), IrValidationError> {
        let ua = i128::from(v_max) * 1000 / r_min.max(1);
        if ua > i128::from(sink_ma) * 1000 || pull_up_max_mv.is_some_and(|m| v_max > m) {
            return Err(self.err(
                K::PullOverDrive,
                &format!("{at}.pull"),
                format!(
                    "the pull-up draws up to {ua} uA at {v_max} mV through at least {r_min} ohms; `{pin}` sinks {sink_ma} mA{} ({})",
                    pull_up_max_mv.map_or(String::new(), |m| format!(" and takes at most {m} mV")),
                    self.row.document
                ),
            ));
        }
        Ok(())
    }

    fn analog(&mut self) -> Result<(), IrValidationError> {
        let d = self.d;
        let pp = crate::power_parts::table()?;
        let mcu = self.mcu_domain()?;
        let vref_min = crate::minimum_millivolts(mcu);
        for (k, a) in d.analog.iter().enumerate() {
            let at = format!(".analog[{k}]");
            let p = self.pin(&a.device_pin, &format!("{at}.device_pin"))?;
            let PinKind::CurrentOutput {
                ua_per_a_typical,
                output_max_ua,
                error_band,
            } = &p.kind
            else {
                return Err(self.kind_mismatch(
                    &a.device_pin,
                    &p.kind,
                    "an ADC input reads a current_output pin",
                    &at,
                ));
            };
            let Transfer::CurrentOutput(c) = &a.transfer;
            let FullScale::Milliamps(fs_ma) = a.full_scale;
            if fs_ma == 0 || c.resistor.ohms == 0 {
                return Err(self.err(
                    K::SignalShape,
                    &at,
                    "a full scale or resistor of 0 declares nothing",
                ));
            }
            // The worst error band that covers the full scale (`RUL-19`: the
            // typical gain with its stated error, never the typical alone).
            let mut worst_ua: Option<i128> = None;
            for b in error_band
                .iter()
                .filter(|b| (b.load_ma.0..=b.load_ma.1).contains(&fs_ma))
            {
                let ua = i128::from(fs_ma) * 1000;
                let with = match (b.percent, b.absolute_ua) {
                    (Some(pc), _) => ua * (100 + i128::from(pc)) / 100,
                    (None, Some(abs)) => ua + i128::from(abs),
                    (None, None) => ua,
                };
                worst_ua = Some(worst_ua.map_or(with, |w| w.max(with)));
            }
            let Some(load_ua) = worst_ua else {
                return Err(self.err(
                    K::PartLimitUndeclared,
                    &format!("{at}.full_scale"),
                    format!(
                        "{fs_ma} mA is in none of `{}`'s error bands ({}, {}): its error there is not stated",
                        a.device_pin, self.row.document, p.page
                    ),
                ));
            };
            // I_out = I_load × A, µA.
            let out_ua = ceil(load_ua * i128::from(*ua_per_a_typical), 1_000_000);
            if out_ua > i128::from(*output_max_ua) {
                return Err(self.err(
                    K::AdcOutputCurrentExceeded,
                    &format!("{at}.full_scale"),
                    format!(
                        "at {fs_ma} mA with its error, `{}` sources up to {out_ua} uA, above its {output_max_ua} uA ({}, {})",
                        a.device_pin, self.row.document, p.page
                    ),
                ));
            }
            let e = resistor_error_ppm(
                self,
                &c.resistor,
                &format!("{at}.transfer.current_output.resistor"),
            )?;
            let (_, r_hi) = band(c.resistor.ohms, e);
            let full_uv = out_ua * r_hi;
            if full_uv > i128::from(vref_min) * 1000 {
                return Err(self.err(
                    K::AdcFullScaleExceeded,
                    &format!("{at}.full_scale"),
                    format!(
                        "{fs_ma} mA reads up to {full_uv} uV at the error band's worst edge and the resistor's highest, above VREF+'s minimum of {vref_min} mV (VDDA on `{}`)",
                        mcu.id
                    ),
                ));
            }
            // RAIN at the declared resolution, or the reservoir AN2834 needs.
            let Some(row) = pp.get(&self.spec.mcu.part) else {
                return Err(self.err(
                    K::PassivePartUnknown,
                    ".mcu",
                    format!("`{}` has no power-parts row", self.spec.mcu.part),
                ));
            };
            let crate::power_parts::Kind::Mcu(m) = &row.kind else {
                return Err(self.err(
                    K::PassivePartUnknown,
                    ".mcu",
                    "the MCU's row is not an mcu row",
                ));
            };
            let Some((_, rain)) = m
                .rain_max_ohms
                .iter()
                .find(|(b, _)| *b == a.adc_resolution_bits)
            else {
                return Err(self.err(
                    K::AdcSourceImpedance,
                    &format!("{at}.adc_resolution_bits"),
                    format!(
                        "the MCU's row states no RAIN for {} bits",
                        a.adc_resolution_bits
                    ),
                ));
            };
            if r_hi > i128::from(rain.value) {
                let Some(res) = &c.reservoir else {
                    return Err(self.err(
                        K::AdcSourceImpedance,
                        &format!("{at}.transfer"),
                        format!(
                            "the source is up to {r_hi} ohms, above RAIN's {} ohms at {} bits ({}), and no reservoir capacitor is declared",
                            rain.value, a.adc_resolution_bits, pp.cite(&rain.at)
                        ),
                    ));
                };
                let cap = match pp.get(&res.part).map(|r| &r.kind) {
                    Some(crate::power_parts::Kind::Capacitor(cap)) => cap,
                    _ => {
                        return Err(self.err(
                            K::PassivePartUnknown,
                            &format!("{at}.transfer.current_output.reservoir"),
                            format!(
                                "`{}` is not a capacitor row of the power parts table",
                                res.part
                            ),
                        ));
                    }
                };
                let need = crate::power::reservoir_min_picofarads(m, a.adc_resolution_bits);
                if i128::from(cap.rated_picofarads.value) < need
                    || cap.rated_millivolts.value < i64::from(vref_min)
                {
                    return Err(self.err(
                        K::AdcSourceImpedance,
                        &format!("{at}.transfer.current_output.reservoir"),
                        format!(
                            "the reservoir is {} pF rated {} mV; the rule needs {need} pF at {} bits ({}) rated at least {vref_min} mV",
                            cap.rated_picofarads.value,
                            cap.rated_millivolts.value,
                            a.adc_resolution_bits,
                            pp.cite(&m.reservoir_rule.at)
                        ),
                    ));
                }
            }
            self.bind(&a.device_pin, Binding::Analog, &at)?;
        }
        Ok(())
    }

    fn bridges(
        &mut self,
        straps: &BTreeMap<&'s str, Option<Level>>,
        motor_inputs: &mut BTreeMap<&'s str, String>,
    ) -> Result<(), IrValidationError> {
        let d = self.d;
        for (k, b) in d.bridges.iter().enumerate() {
            let at = format!(".bridges[{k}]");
            let Some(mode) = self.row.mode(&b.mode) else {
                return Err(self.err(
                    K::BridgeModeUnknown,
                    &format!("{at}.mode"),
                    format!(
                        "`{}` has no mode `{}`; its modes: {}",
                        self.row.part,
                        b.mode,
                        self.row
                            .modes
                            .iter()
                            .map(|m| m.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            };
            if let (Some(strap), Some(want)) = (&self.row.mode_strap, mode.strap_level) {
                let got = straps.get(strap.as_str()).copied().flatten();
                if got != Some(want) {
                    return Err(self.err(
                        K::BridgeModeStrapMismatch,
                        &format!("{at}.mode"),
                        format!(
                            "mode `{}` needs `{strap}` {}; it is strapped {} ({}, {})",
                            b.mode,
                            want.name(),
                            got.map_or("to no level", Level::name),
                            self.row.document,
                            mode.page
                        ),
                    ));
                }
            }
            let keys =
                |m: &BTreeMap<String, String>| m.keys().cloned().collect::<BTreeSet<String>>();
            let Some(channel) = mode.channels.iter().find(|c| {
                keys(&b.inputs) == c.inputs.iter().cloned().collect()
                    && keys(&b.outputs) == c.outputs.iter().cloned().collect()
            }) else {
                return Err(self.err(
                    K::BridgeChannelUnknown,
                    &at,
                    format!(
                        "inputs {:?} and outputs {:?} are no channel of `{}` mode `{}`",
                        keys(&b.inputs),
                        keys(&b.outputs),
                        self.row.part,
                        b.mode
                    ),
                ));
            };
            for out in &channel.outputs {
                self.bind(out, Binding::BridgeOutput, &format!("{at}.outputs.{out}"))?;
            }
            // Each input's idle level, in the channel's order.
            let mut idle = [Level::Low; 2];
            for (n, pin) in channel.inputs.iter().enumerate() {
                let label = &b.inputs[pin];
                let at_in = format!("{at}.inputs.{pin}");
                let motor = self.spec.motor_outputs.iter().find(|m| &m.label == label);
                let level = if let Some(m) = motor {
                    if let Some(first) =
                        motor_inputs.insert(m.label.as_str(), format!("{}{at_in}", self.path))
                    {
                        return Err(self.err(
                            K::BridgeInputShared,
                            &at_in,
                            format!("motor output `{label}` is already the input at `{first}`"),
                        ));
                    }
                    let pulsed = |why: String| self.err(K::BridgeIdlePulsed, &at_in, why);
                    let level = match (m.protocol, &m.pwm) {
                        (MotorProtocol::Pwm, Some(p)) => match p.idle {
                            Idle::Level(l) => l,
                            Idle::PulseUs(us) => {
                                return Err(pulsed(format!(
                                    "`{label}` idles at a {us} us pulse: a pulsed bridge input visits two rows of the truth table every period, so its idle is no one state (`1211` Q3)"
                                )));
                            }
                        },
                        _ => {
                            return Err(pulsed(format!(
                                "`{label}` is {}: its idle is a frame stream, no level",
                                m.protocol.name()
                            )));
                        }
                    };
                    self.drive_levels(pin, self.mcu_domain()?, &at_in)?;
                    self.bind(pin, Binding::Motor(m), &at_in)?;
                    level
                } else {
                    match self.bound.get(pin.as_str()) {
                        Some(Binding::Signal(s))
                            if &s.net == label && s.direction == Direction::Output =>
                        {
                            s.initial.expect("an output states initial")
                        }
                        _ => {
                            return Err(self.err(
                                K::BridgeInputUnknown,
                                &at_in,
                                format!(
                                    "`{label}` is neither a PWM motor output nor this device's output signal on `{pin}`"
                                ),
                            ));
                        }
                    }
                };
                idle[n] = level;
            }
            let enable_pin = self
                .row
                .enable
                .as_deref()
                .expect("a part with modes names its enable");
            let enable = match self.bound.get(enable_pin) {
                Some(Binding::Signal(s)) if s.direction == Direction::Output => {
                    s.initial.expect("an output states initial")
                }
                _ => {
                    return Err(self.err(
                        K::DevicePinUnbound,
                        &at,
                        format!(
                            "the bridge's idle reads `{enable_pin}`, which no output signal of this device drives"
                        ),
                    ));
                }
            };
            self.judge_state(mode, enable, idle, &at, K::BridgeIdleDrives, "idles")?;
            // From reset to the first write, each input and the enable sit
            // at the part's own pulls (`1211` §2.6): every driven input has
            // one (`drive_levels` refuses it otherwise), and a row states
            // only pull-downs.
            let held = |pin: &str| {
                self.row.pins[pin]
                    .pull_down_ohms_typical
                    .map(|_| Level::Low)
                    .expect("drive_levels refused an input with no pull")
            };
            self.judge_state(
                mode,
                held(enable_pin),
                [held(&channel.inputs[0]), held(&channel.inputs[1])],
                &at,
                K::UnheldDuringReset,
                "sits from reset to the first write",
            )?;
        }
        Ok(())
    }

    fn judge_state(
        &self,
        mode: &crate::pin_parts::Mode,
        enable: Level,
        inputs: [Level; 2],
        at: &str,
        kind: PinKindOfRefusal,
        what: &str,
    ) -> Result<(), IrValidationError> {
        let tuple = format!(
            "{} {}, inputs {} {}",
            self.row.enable.as_deref().unwrap_or("enable"),
            enable.name(),
            inputs[0].name(),
            inputs[1].name()
        );
        let Some(state) = mode.state(enable, inputs) else {
            return Err(self.err(
                kind,
                at,
                format!(
                    "the bridge {what} at ({tuple}), which is no row of `{}` mode `{}` ({}, {})",
                    self.row.part, mode.name, self.row.document, mode.page
                ),
            ));
        };
        if mode.forbidden.contains(&state.name) {
            return Err(self.err(
                K::BridgeForbiddenState,
                at,
                format!(
                    "the bridge {what} in `{}`, a forbidden state of `{}` ({})",
                    state.name, self.row.part, mode.page
                ),
            ));
        }
        if state.drives {
            return Err(self.err(
                kind,
                at,
                format!(
                    "the bridge {what} at ({tuple}): `{}`, which drives the load ({}, {})",
                    state.name, self.row.document, mode.page
                ),
            ));
        }
        Ok(())
    }

    /// Each declared limit the row lacks, and each PWM bridge input's rate
    /// against the part's maximum.
    fn limits(&self) -> Result<(), IrValidationError> {
        let declared = &self.d.part_limits.input_pwm_max_hz;
        if let (Some(_), Some(row)) = (declared, &self.row.input_pwm_max_hz) {
            return Err(self.err(
                K::PartLimitDeclaredTwice,
                ".part_limits.input_pwm_max_hz",
                format!(
                    "`{}` states it ({} Hz, {}): declare it once",
                    self.row.part, row.value, row.page
                ),
            ));
        }
        for (pin, how) in &self.bound {
            let Binding::Motor(m) = how else { continue };
            let rate = m.pwm.as_ref().map_or(0, |p| p.rate_hz);
            let max = match (&self.row.input_pwm_max_hz, declared) {
                (Some(r), _) => r.value,
                (None, Some(d)) => d.value,
                (None, None) => {
                    return Err(self.err(
                        K::PartLimitUndeclared,
                        ".part_limits.input_pwm_max_hz",
                        format!(
                            "`{}` takes `{}` at {rate} Hz on `{pin}`, and no held document states its maximum input PWM rate ({} {}): declare one with its citation (`1211` §7)",
                            self.row.part, m.label, self.row.document, self.row.revision
                        ),
                    ));
                }
            };
            if rate > max {
                return Err(self.err(
                    K::InputRateAbovePart,
                    "",
                    format!(
                        "`{}` drives `{pin}` at {rate} Hz, above `{}`'s {max} Hz",
                        m.label, self.row.part
                    ),
                ));
            }
        }
        Ok(())
    }
}

/// `ceil(a / b)` for `a >= 0`, `b > 0`.
fn ceil(a: i128, b: i128) -> i128 {
    (a + b - 1) / b
}

/// `ohms` at `±ppm`: (lowest, highest), whole ohms, outward.
fn band(ohms: u32, ppm: i128) -> (i128, i128) {
    let r = i128::from(ohms);
    (
        r * (1_000_000 - ppm) / 1_000_000,
        ceil(r * (1_000_000 + ppm), 1_000_000),
    )
}

/// A resistor's worst error over the device's temperature: its row's
/// tolerance, TCR over the excursion and drift (`power.sense`'s rule).
fn resistor_error_ppm(
    dev: &Device<'_>,
    r: &crate::power::Resistor,
    at: &str,
) -> Result<i128, IrValidationError> {
    let t = match (&dev.d.temperature_celsius, &dev.spec.power) {
        (Some(_), Some(_)) => {
            return Err(dev.err(
                K::TemperatureDeclaredTwice,
                ".temperature_celsius",
                "`power.conditions` declares the board temperature: state it once",
            ));
        }
        (Some(t), None) => t,
        (None, Some(p)) => &p.conditions.board_temperature_celsius,
        (None, None) => {
            return Err(dev.err(
                K::TemperatureUndeclared,
                ".temperature_celsius",
                format!(
                    "`{}` is judged over a temperature, and neither `power.conditions` nor the device declares one",
                    r.reference
                ),
            ));
        }
    };
    let pp = crate::power_parts::table()?;
    match pp.get(&r.part).map(|row| &row.kind) {
        Some(crate::power_parts::Kind::Resistor(s)) => Ok(crate::power::resistor_error_ppm(s, t)),
        _ => Err(dev.err(
            K::PassivePartUnknown,
            at,
            format!(
                "`{}` is not a resistor row of the power parts table",
                r.part
            ),
        )),
    }
}

/// The topology's GPIO and ADC entries against its devices: each output
/// states `initial` and no `pull`, each input `pull` and no `initial`; with
/// no pin device, every GPIO is a chip select (an output at `high`) and
/// every ADC input a `power.sense` net, so a reader that refuses any pin
/// device can read the rest as it always has.
pub(crate) fn validate_solved(t: &crate::SolvedTopology) -> Result<(), IrValidationError> {
    for (net, g) in &t.gpio {
        let ok = match g.direction {
            Direction::Output => g.initial.is_some() && g.pull.is_none(),
            Direction::Input => g.pull.is_some() && g.initial.is_none(),
        };
        if !ok {
            return Err(refuse(
                K::SignalShape,
                net,
                format!("gpio.{net}"),
                "an output states `initial` and no `pull`; an input `pull` and no `initial`",
            ));
        }
        if t.pin_devices.is_empty() && !g.is_high_output() {
            return Err(refuse(
                K::PinDeviceUndeclared,
                net,
                format!("gpio.{net}"),
                "only a pin device's signal is anything but a chip select (an output at `high`), and the topology carries no pin device",
            ));
        }
    }
    if t.pin_devices.is_empty() {
        let sense: BTreeSet<&String> = t.power.iter().flat_map(|p| p.sense.keys()).collect();
        if let Some(net) = t.adc_inputs.keys().find(|n| !sense.contains(n)) {
            return Err(refuse(
                K::PinDeviceUndeclared,
                net,
                format!("adc_inputs.{net}"),
                "only a pin device's analog input is an ADC input but a `power.sense` net, and the topology carries no pin device",
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- solve

/// The ADC peripheral this engine's twin models, in the order the solver
/// prefers them (`1211` Q8): ADC2 is unmodelled in Renode (`1211` §4, A6).
const MODELLED_ADCS: [&str; 2] = ["ADC1", "ADC3"];

/// `ADCn_INPm` as (`ADCn`, `INPm`).
fn split_channel(c: &str) -> Option<(&str, &str)> {
    c.split_once('_')
}

/// The channel the solver takes on `pin`'s channels: ADC1, then ADC3; ADC2
/// only when `hand` (pinned by the specification).
pub(crate) fn choose_channel(channels: &BTreeSet<String>, hand: bool) -> Option<(String, String)> {
    let pick = |adc: &str| {
        channels
            .iter()
            .filter_map(|c| split_channel(c))
            .find(|(p, _)| *p == adc)
            .map(|(p, i)| (p.to_owned(), i.to_owned()))
    };
    MODELLED_ADCS
        .iter()
        .find_map(|a| pick(a))
        .or_else(|| if hand { pick("ADC2") } else { None })
}

/// Places every pin device's signals, then its ADC inputs, after every other
/// pin is owned, so no pin solved before moves: each signal on its hand pin
/// or the first free unreserved pin in key order (as a chip select), each
/// ADC input on its hand pin or the free unreserved pin of lowest package
/// position with an ADC1 or ADC3 channel (`1211` Q8).
pub(crate) fn place<'t>(
    spec: &Specification,
    table: &'t crate::PinTable,
    reserved: &[&str],
    used: &mut BTreeMap<&'t str, String>,
    gpio: &mut BTreeMap<String, SolvedGpio>,
    adc: &mut BTreeMap<String, SolvedAdcInput>,
) -> Result<(), crate::SolveError> {
    let key = |pin: &str| table.pins.get_key_value(pin).map(|(k, _)| k.as_str());
    for (i, d) in spec.declared_pin_devices().iter().enumerate() {
        for (k, s) in d.signals.iter().enumerate() {
            let claimant = format!("pin_devices[{i}].signals[{k}]");
            let pin = match &s.pin {
                Some(p) => {
                    let p = key(p).ok_or_else(|| crate::SolveError::UnknownPin {
                        claimant: claimant.clone(),
                        pin: p.clone(),
                    })?;
                    hand_free(p, &claimant, reserved, used)?;
                    p
                }
                None => table
                    .pins
                    .keys()
                    .map(String::as_str)
                    .find(|p| !reserved.contains(p) && !used.contains_key(p))
                    .ok_or_else(|| crate::SolveError::NoFreePin {
                        claimant: claimant.clone(),
                    })?,
            };
            used.insert(pin, claimant);
            gpio.insert(
                s.net.clone(),
                SolvedGpio {
                    pin: pin.to_owned(),
                    direction: s.direction,
                    initial: s.initial,
                    pull: s.pull.as_ref().map(|p| p.mcu.unwrap_or(McuPull::None)),
                },
            );
        }
    }
    for (i, d) in spec.declared_pin_devices().iter().enumerate() {
        for (k, a) in d.analog.iter().enumerate() {
            let claimant = format!("pin_devices[{i}].analog[{k}]");
            let wanted = a
                .channel
                .as_ref()
                .map(|c| format!("{}_{}", c.peripheral, c.input));
            let fits = |pin: &crate::Pin| match &wanted {
                Some(w) => pin.adc_channels.contains(w),
                None => !pin.adc_channels.is_empty(),
            };
            let (name, pin) = match &a.pin {
                Some(p) => {
                    let (name, pin) = table.pins.get_key_value(p.as_str()).ok_or_else(|| {
                        crate::SolveError::UnknownPin {
                            claimant: claimant.clone(),
                            pin: p.clone(),
                        }
                    })?;
                    hand_free(name, &claimant, reserved, used)?;
                    if !fits(pin) {
                        return Err(crate::SolveError::AdcChannelNotOnPin {
                            net: a.net.clone(),
                            reason: format!(
                                "pin `{name}` carries {}",
                                wanted.as_ref().map_or_else(
                                    || "no ADC input".to_owned(),
                                    |w| format!("no `{w}`; it carries {:?}", pin.adc_channels)
                                )
                            ),
                        });
                    }
                    (name.as_str(), pin)
                }
                None => {
                    let mut free: Vec<(&str, &crate::Pin)> = table
                        .pins
                        .iter()
                        .filter(|(n, p)| {
                            fits(p)
                                && !reserved.contains(&n.as_str())
                                && !used.contains_key(n.as_str())
                        })
                        .map(|(n, p)| (n.as_str(), p))
                        .collect();
                    free.sort_by_key(|(_, p)| p.position);
                    let modelled = |p: &crate::Pin| {
                        choose_channel(&p.adc_channels, wanted.is_some()).is_some()
                    };
                    match free.iter().find(|(_, p)| modelled(p)) {
                        Some(&(n, p)) => (n, p),
                        None if !free.is_empty() => {
                            return Err(crate::SolveError::AdcUnmodelled {
                                net: a.net.clone(),
                                pins: free.iter().map(|(n, _)| (*n).to_owned()).collect(),
                            });
                        }
                        None => return Err(crate::SolveError::NoAdcPin { net: a.net.clone() }),
                    }
                }
            };
            let (peripheral, channel) = match &a.channel {
                Some(c) => (c.peripheral.clone(), c.input.clone()),
                None => choose_channel(&pin.adc_channels, a.pin.is_some()).ok_or_else(|| {
                    crate::SolveError::AdcUnmodelled {
                        net: a.net.clone(),
                        pins: vec![name.to_owned()],
                    }
                })?,
            };
            used.insert(name, claimant);
            adc.insert(
                a.net.clone(),
                SolvedAdcInput {
                    pin: name.to_owned(),
                    peripheral,
                    channel,
                },
            );
        }
    }
    Ok(())
}

/// A hand pin neither reserved nor owned by another claimant.
fn hand_free(
    pin: &str,
    claimant: &str,
    reserved: &[&str],
    used: &BTreeMap<&str, String>,
) -> Result<(), crate::SolveError> {
    if reserved.contains(&pin) {
        return Err(crate::SolveError::ReservedPin {
            pin: pin.to_owned(),
            claimant: claimant.to_owned(),
        });
    }
    // The model claimed each hand pin for its own claimant up front.
    if let Some(first) = used.get(pin).filter(|by| *by != claimant) {
        return Err(crate::SolveError::PinClaimedTwice {
            pin: pin.to_owned(),
            first: first.clone(),
            second: claimant.to_owned(),
        });
    }
    Ok(())
}

/// The bridges whose two inputs are both motor outputs in `in_in` mode,
/// as (bridge path, first label, second label): one counter must time both
/// (`BridgeSplitAcrossTimers`). A PH/EN pair is exempt (`1211` §2.7 (e)).
pub(crate) fn in_in_motor_pairs(spec: &Specification) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    for (i, d) in spec.declared_pin_devices().iter().enumerate() {
        for (k, b) in d.bridges.iter().enumerate() {
            if b.mode != "in_in" {
                continue;
            }
            let labels: Vec<&String> = b.inputs.values().collect();
            let motor = |l: &String| spec.motor_outputs.iter().any(|m| &m.label == l);
            if labels.len() == 2 && labels.iter().all(|l| motor(l)) {
                out.push((
                    format!("pin_devices[{i}].bridges[{k}]"),
                    labels[0].clone(),
                    labels[1].clone(),
                ));
            }
        }
    }
    out
}
