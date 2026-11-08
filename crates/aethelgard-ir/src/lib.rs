//! The specification schema, the solved topology, validation, and
//! serialization (`docs/architecture.md` §3). Every shared specification type
//! lives here and nowhere else (`INV-01`).
//!
//! [`Specification`] is what a person declares; [`SolvedTopology`] is what the
//! solver derives (`RUL-16`). They are separate types and nothing converts one
//! into the other. Every type refuses unknown fields and accepts only an
//! object, never serde's positional-sequence form, and `validate` refuses
//! rather than defaults (`INV-06`).

#![forbid(unsafe_code)]

mod solve;

pub use solve::{NODE_LIMIT, RESERVED_PINS, SolveError, solve, solve_with_limit};

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The top-level specification, version 1 (`RUL-14`). Arrays keep the order a
/// person wrote them in, and nothing is sorted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Specification {
    pub schema_version: u32,
    pub project_name: String,
    pub mcu: Mcu,
    pub voltage_domains: Vec<VoltageDomain>,
    pub i2c_buses: Vec<I2cBus>,
    pub spi_buses: Vec<SpiBus>,
    pub i2c_devices: Vec<I2cDevice>,
    pub spi_devices: Vec<SpiDevice>,
    pub net_constraints: Vec<NetConstraint>,
}

/// The microcontroller: a part name and the domain that supplies it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Mcu {
    pub part: String,
    pub supply: String,
}

/// A supply rail and how far it may stray from nominal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct VoltageDomain {
    pub id: String,
    pub nominal_millivolts: u32,
    pub tolerance_permille: u16,
}

/// One signal: the net it rides and, when a person pinned it by hand, the pin.
/// `pin` is required as a key; `null` means the solver chooses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Signal {
    pub net: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub pin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct I2cBus {
    pub id: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub peripheral: Option<String>,
    pub speed_hz: u32,
    pub pull_up_ohms: u32,
    pub domain: String,
    pub sda: Signal,
    pub scl: Signal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SpiBus {
    pub id: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub peripheral: Option<String>,
    pub speed_hz: u32,
    pub domain: String,
    pub sck: Signal,
    pub mosi: Signal,
    pub miso: Signal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct I2cDevice {
    pub reference: String,
    pub part: String,
    pub supply: String,
    pub bus: String,
    pub address: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SpiDevice {
    pub reference: String,
    pub part: String,
    pub supply: String,
    pub bus: String,
    pub cs: Signal,
}

/// The electrical bounds one net must stay within.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct NetConstraint {
    pub net_name: String,
    pub max_capacitance_femtofarads: u64,
    pub max_rise_time_nanoseconds: u32,
    /// Required as a key: `null` means unconstrained, and a missing key is a
    /// refusal rather than a silent `None` (`INV-06`).
    #[serde(deserialize_with = "Option::deserialize")]
    pub impedance_ohms: Option<u32>,
}

/// What the solver derived from one specification (`RUL-16`). Keyed by net, in
/// `BTreeMap`s, so serialization is a function of content. `assignments` holds
/// the signals on an alternate function; `gpio` holds each net driven as a
/// plain GPIO output (an SPI device's `cs`), net to pin. `i2c_ceilings` holds
/// each I2C bus's maximum speed in Hz, keyed by bus id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedTopology {
    pub schema_version: u32,
    /// SHA-256 of the specification solved, 64 lowercase hex digits.
    pub specification_sha256: String,
    #[serde(deserialize_with = "unique_keys")]
    pub assignments: BTreeMap<String, PinAssignment>,
    #[serde(deserialize_with = "unique_keys")]
    pub gpio: BTreeMap<String, String>,
    #[serde(deserialize_with = "unique_keys")]
    pub i2c_ceilings: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct PinAssignment {
    pub pin: String,
    pub peripheral: String,
    pub signal: String,
    pub alternate_function: u8,
}

/// serde's derived `BTreeMap` keeps the last value of a repeated key and drops
/// the first without a word; this refuses it instead (`INV-06`).
fn unique_keys<'de, D: Deserializer<'de>, V: Deserialize<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, V>, D::Error> {
    struct Unique<V>(std::marker::PhantomData<V>);

    impl<'de, V: Deserialize<'de>> Visitor<'de> for Unique<V> {
        type Value = BTreeMap<String, V>;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("an object keyed by net")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut out = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, V>()? {
                if out.contains_key(&key) {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate assignment for net `{key}`"
                    )));
                }
                out.insert(key, value);
            }
            Ok(out)
        }
    }

    deserializer.deserialize_map(Unique(std::marker::PhantomData))
}

/// The derives above are `remote = "Self"`, so they generate inherent
/// functions; these trait impls wrap them so a type deserializes only from an
/// object. The derived form would also accept a positional sequence such as
/// `["p",[],[]]`, which bypasses field names entirely.
macro_rules! object_only {
    ($($ty:ident),*) => {$(
        impl Serialize for $ty {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                $ty::serialize(self, serializer)
            }
        }

        impl<'de> Deserialize<'de> for $ty {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct ObjectVisitor;

                impl<'de> Visitor<'de> for ObjectVisitor {
                    type Value = $ty;

                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str(concat!("a ", stringify!($ty), " object"))
                    }

                    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<$ty, A::Error> {
                        $ty::deserialize(MapAccessDeserializer::new(map))
                    }
                }

                deserializer.deserialize_map(ObjectVisitor)
            }
        }
    )*};
}

object_only!(
    Specification,
    Mcu,
    VoltageDomain,
    Signal,
    I2cBus,
    SpiBus,
    I2cDevice,
    SpiDevice,
    NetConstraint,
    SolvedTopology,
    PinAssignment
);

/// The pins of one part and package, built from a vendored vendor source and
/// never read from a user, so it is `Serialize` and not `Deserialize`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PinTable {
    pub part: String,
    pub package: String,
    pub source: PinSource,
    /// Keyed by canonical pin name, such as `PB8`.
    pub pins: BTreeMap<String, Pin>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pin {
    /// The package pin number.
    pub position: u16,
    /// Signal name to alternate-function number, such as `I2C1_SCL` to 4.
    pub alternate_functions: BTreeMap<String, u8>,
}

/// Where a [`PinTable`] came from: a repository, a commit, and each file's sha256.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PinSource {
    pub repository: String,
    pub commit: String,
    pub files: BTreeMap<String, String>,
}

impl PinTable {
    /// `RUL-14` over every pin key and signal name; `path` is `pins.<key>` or
    /// `pins.<key>.<signal>`.
    pub fn validate(&self) -> Result<(), IrValidationError> {
        for (name, pin) in &self.pins {
            identifier(name, || format!("pins.{name}"))?;
            for signal in pin.alternate_functions.keys() {
                identifier(signal, || format!("pins.{name}.{signal}"))?;
            }
        }
        Ok(())
    }
}

/// One variant per refusal. Each names what `id()` returns, or nothing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IrValidationError {
    #[error("schema_version is {version}, not 1")]
    UnsupportedSchemaVersion { version: u32 },
    #[error(
        "`{path}` is not an identifier: 1 to 64 bytes of printable ASCII 0x21-0x7E, none of them `/`, `{{`, `}}`, `$`, `[` or `]`"
    )]
    InvalidIdentifier { path: String },
    #[error("voltage_domains is empty")]
    NoVoltageDomain,
    #[error("voltage domain id `{id}` appears more than once")]
    DuplicateDomainId { id: String },
    #[error("voltage domain `{id}` has nominal_millivolts 0")]
    ZeroNominalVoltage { id: String },
    #[error("voltage domain `{id}` has tolerance_permille {value}, outside 1..=500")]
    ToleranceOutOfRange { id: String, value: u16 },
    #[error("`{id}` is not a declared voltage domain")]
    UnknownDomain { id: String },
    #[error("`{id}` runs on a different voltage domain from its bus or the microcontroller")]
    SupplyMismatch { id: String },
    #[error("bus id `{id}` appears more than once")]
    DuplicateBusId { id: String },
    #[error("I2C bus `{id}` has speed_hz {value}, not 100000, 400000 or 1000000")]
    UnsupportedI2cSpeed { id: String, value: u32 },
    #[error("I2C bus `{id}` has pull_up_ohms 0")]
    ZeroPullUp { id: String },
    #[error("SPI bus `{id}` has speed_hz 0")]
    ZeroSpiSpeed { id: String },
    #[error("device reference `{id}` appears more than once")]
    DuplicateReference { id: String },
    #[error("`{id}` is not a bus of the device's kind")]
    UnknownBus { id: String },
    #[error("device `{id}` has I2C address {value}, outside 0x08..=0x77")]
    I2cAddressOutOfRange { id: String, value: u8 },
    #[error("device `{id}` repeats I2C address {value} on its bus")]
    DuplicateI2cAddress { id: String, value: u8 },
    #[error("net `{id}` is declared by more than one signal")]
    DuplicateNetName { id: String },
    #[error("signal net `{id}` has the name of a voltage domain")]
    NetShadowsDomain { id: String },
    #[error("net_name `{id}` appears more than once in net_constraints")]
    DuplicateNetConstraint { id: String },
    #[error("net_constraints names `{id}`, which is no signal net or voltage domain")]
    UnknownNet { id: String },
    #[error("net `{id}` has max_capacitance_femtofarads 0")]
    ZeroCapacitance { id: String },
    #[error("net `{id}` has max_rise_time_nanoseconds 0")]
    ZeroRiseTime { id: String },
    #[error("net `{id}` has impedance_ohms 0")]
    ZeroImpedance { id: String },
    #[error("specification_sha256 is not 64 lowercase hex digits")]
    MalformedSpecificationHash,
    #[error("net `{id}` has alternate_function {value}, above 15")]
    AlternateFunctionOutOfRange { id: String, value: u8 },
    #[error("I2C bus `{id}` has a ceiling other than 100000, 400000 or 1000000 Hz")]
    InvalidCeiling { id: String },
    #[error("net `{id}` is in both assignments and gpio")]
    NetAssignedTwice { id: String },
    #[error("pin `{id}` is assigned to more than one net")]
    DuplicatePin { id: String },
}

impl IrValidationError {
    /// The variant name, as callers see it in the CLI's error envelope.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion { .. } => "UnsupportedSchemaVersion",
            Self::InvalidIdentifier { .. } => "InvalidIdentifier",
            Self::NoVoltageDomain => "NoVoltageDomain",
            Self::DuplicateDomainId { .. } => "DuplicateDomainId",
            Self::ZeroNominalVoltage { .. } => "ZeroNominalVoltage",
            Self::ToleranceOutOfRange { .. } => "ToleranceOutOfRange",
            Self::UnknownDomain { .. } => "UnknownDomain",
            Self::SupplyMismatch { .. } => "SupplyMismatch",
            Self::DuplicateBusId { .. } => "DuplicateBusId",
            Self::UnsupportedI2cSpeed { .. } => "UnsupportedI2cSpeed",
            Self::ZeroPullUp { .. } => "ZeroPullUp",
            Self::ZeroSpiSpeed { .. } => "ZeroSpiSpeed",
            Self::DuplicateReference { .. } => "DuplicateReference",
            Self::UnknownBus { .. } => "UnknownBus",
            Self::I2cAddressOutOfRange { .. } => "I2cAddressOutOfRange",
            Self::DuplicateI2cAddress { .. } => "DuplicateI2cAddress",
            Self::DuplicateNetName { .. } => "DuplicateNetName",
            Self::NetShadowsDomain { .. } => "NetShadowsDomain",
            Self::DuplicateNetConstraint { .. } => "DuplicateNetConstraint",
            Self::UnknownNet { .. } => "UnknownNet",
            Self::ZeroCapacitance { .. } => "ZeroCapacitance",
            Self::ZeroRiseTime { .. } => "ZeroRiseTime",
            Self::ZeroImpedance { .. } => "ZeroImpedance",
            Self::MalformedSpecificationHash => "MalformedSpecificationHash",
            Self::AlternateFunctionOutOfRange { .. } => "AlternateFunctionOutOfRange",
            Self::InvalidCeiling { .. } => "InvalidCeiling",
            Self::NetAssignedTwice { .. } => "NetAssignedTwice",
            Self::DuplicatePin { .. } => "DuplicatePin",
        }
    }

    /// The offending name, reference, net or pin. `None` for the refusals that
    /// name no single thing: a version, a path (see the variant), a hash, an
    /// empty list. Nothing is trimmed.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::UnsupportedSchemaVersion { .. }
            | Self::InvalidIdentifier { .. }
            | Self::NoVoltageDomain
            | Self::MalformedSpecificationHash => None,
            Self::DuplicateDomainId { id }
            | Self::ZeroNominalVoltage { id }
            | Self::ToleranceOutOfRange { id, .. }
            | Self::UnknownDomain { id }
            | Self::SupplyMismatch { id }
            | Self::DuplicateBusId { id }
            | Self::UnsupportedI2cSpeed { id, .. }
            | Self::ZeroPullUp { id }
            | Self::ZeroSpiSpeed { id }
            | Self::DuplicateReference { id }
            | Self::UnknownBus { id }
            | Self::I2cAddressOutOfRange { id, .. }
            | Self::DuplicateI2cAddress { id, .. }
            | Self::DuplicateNetName { id }
            | Self::NetShadowsDomain { id }
            | Self::DuplicateNetConstraint { id }
            | Self::UnknownNet { id }
            | Self::ZeroCapacitance { id }
            | Self::ZeroRiseTime { id }
            | Self::ZeroImpedance { id }
            | Self::AlternateFunctionOutOfRange { id, .. }
            | Self::InvalidCeiling { id }
            | Self::NetAssignedTwice { id }
            | Self::DuplicatePin { id } => Some(id),
        }
    }

    /// The JSON path of the field `InvalidIdentifier` refused, such as
    /// `i2c_buses[0].sda.net`.
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::InvalidIdentifier { path } => Some(path),
            _ => None,
        }
    }
}

/// `RUL-14`: 1 to 64 bytes of printable ASCII `0x21`-`0x7E`. Whitespace, control
/// characters and everything outside ASCII are refused, so two identifiers
/// that look alike are the same bytes. Six more are refused because KiCad
/// reads them as more than text (`AET-WO-1300` S5, ruled D6 on PR #40): `/` is
/// its sheet-path separator, `{` and `}` delimit its escape tokens and bus
/// groups, `$` starts a text variable, and `[` and `]` a bus vector.
pub const REFUSED_IN_IDENTIFIERS: [u8; 6] = *b"/{}$[]";

fn identifier(value: &str, path: impl FnOnce() -> String) -> Result<(), IrValidationError> {
    let ok = (1..=64).contains(&value.len())
        && value
            .bytes()
            .all(|b| (0x21..=0x7E).contains(&b) && !REFUSED_IN_IDENTIFIERS.contains(&b));
    if ok {
        Ok(())
    } else {
        Err(IrValidationError::InvalidIdentifier { path: path() })
    }
}

fn optional_identifier(
    value: &Option<String>,
    path: impl FnOnce() -> String,
) -> Result<(), IrValidationError> {
    value.as_deref().map_or(Ok(()), |v| identifier(v, path))
}

fn signal_identifiers(signal: &Signal, path: &str) -> Result<(), IrValidationError> {
    identifier(&signal.net, || format!("{path}.net"))?;
    optional_identifier(&signal.pin, || format!("{path}.pin"))
}

impl Specification {
    /// Refuses the first malformed value found, in the order of
    /// `crates/aethelgard-ir/README.md`, and in document order within each step.
    /// It does not check that a part, peripheral or pin exists, nor that two
    /// signals ask for one pin: both need the pin table (`AET-WO-1200`).
    pub fn validate(&self) -> Result<(), IrValidationError> {
        use IrValidationError as E;

        // 1
        if self.schema_version != 1 {
            return Err(E::UnsupportedSchemaVersion {
                version: self.schema_version,
            });
        }
        // 2
        self.identifiers()?;
        // 3
        if self.voltage_domains.is_empty() {
            return Err(E::NoVoltageDomain);
        }
        // 4
        let mut domains = BTreeSet::new();
        for domain in &self.voltage_domains {
            let id = &domain.id;
            if !domains.insert(id.as_str()) {
                return Err(E::DuplicateDomainId { id: id.clone() });
            }
            if domain.nominal_millivolts == 0 {
                return Err(E::ZeroNominalVoltage { id: id.clone() });
            }
            if !(1..=500).contains(&domain.tolerance_permille) {
                return Err(E::ToleranceOutOfRange {
                    id: id.clone(),
                    value: domain.tolerance_permille,
                });
            }
        }
        // 5
        let known = |name: &String| {
            if domains.contains(name.as_str()) {
                Ok(())
            } else {
                Err(E::UnknownDomain { id: name.clone() })
            }
        };
        known(&self.mcu.supply)?;
        for bus in &self.i2c_buses {
            known(&bus.domain)?;
        }
        for bus in &self.spi_buses {
            known(&bus.domain)?;
        }
        for device in &self.i2c_devices {
            known(&device.supply)?;
        }
        for device in &self.spi_devices {
            known(&device.supply)?;
        }
        // 5a: one rail. A device whose bus is unknown is left to step 9.
        for bus in &self.i2c_buses {
            if bus.domain != self.mcu.supply {
                return Err(E::SupplyMismatch { id: bus.id.clone() });
            }
        }
        for bus in &self.spi_buses {
            if bus.domain != self.mcu.supply {
                return Err(E::SupplyMismatch { id: bus.id.clone() });
            }
        }
        for device in &self.i2c_devices {
            let bus = self.i2c_buses.iter().find(|b| b.id == device.bus);
            if bus.is_some_and(|b| b.domain != device.supply) {
                return Err(E::SupplyMismatch {
                    id: device.reference.clone(),
                });
            }
        }
        for device in &self.spi_devices {
            let bus = self.spi_buses.iter().find(|b| b.id == device.bus);
            if bus.is_some_and(|b| b.domain != device.supply) {
                return Err(E::SupplyMismatch {
                    id: device.reference.clone(),
                });
            }
        }
        // 6
        let mut i2c_ids = BTreeSet::new();
        let mut spi_ids = BTreeSet::new();
        let mut bus_ids = BTreeSet::new();
        for id in self.i2c_buses.iter().map(|b| &b.id) {
            if !bus_ids.insert(id.as_str()) {
                return Err(E::DuplicateBusId { id: id.clone() });
            }
            i2c_ids.insert(id.as_str());
        }
        for id in self.spi_buses.iter().map(|b| &b.id) {
            if !bus_ids.insert(id.as_str()) {
                return Err(E::DuplicateBusId { id: id.clone() });
            }
            spi_ids.insert(id.as_str());
        }
        // 7
        for bus in &self.i2c_buses {
            if ![100_000, 400_000, 1_000_000].contains(&bus.speed_hz) {
                return Err(E::UnsupportedI2cSpeed {
                    id: bus.id.clone(),
                    value: bus.speed_hz,
                });
            }
            if bus.pull_up_ohms == 0 {
                return Err(E::ZeroPullUp { id: bus.id.clone() });
            }
        }
        for bus in &self.spi_buses {
            if bus.speed_hz == 0 {
                return Err(E::ZeroSpiSpeed { id: bus.id.clone() });
            }
        }
        // 8
        let mut references = BTreeSet::new();
        let all_references = self
            .i2c_devices
            .iter()
            .map(|d| &d.reference)
            .chain(self.spi_devices.iter().map(|d| &d.reference));
        for reference in all_references {
            if !references.insert(reference.as_str()) {
                return Err(E::DuplicateReference {
                    id: reference.clone(),
                });
            }
        }
        // 9
        for device in &self.i2c_devices {
            if !i2c_ids.contains(device.bus.as_str()) {
                return Err(E::UnknownBus {
                    id: device.bus.clone(),
                });
            }
        }
        for device in &self.spi_devices {
            if !spi_ids.contains(device.bus.as_str()) {
                return Err(E::UnknownBus {
                    id: device.bus.clone(),
                });
            }
        }
        // 10
        let mut addresses = BTreeSet::new();
        for device in &self.i2c_devices {
            if !(0x08..=0x77).contains(&device.address) {
                return Err(E::I2cAddressOutOfRange {
                    id: device.reference.clone(),
                    value: device.address,
                });
            }
            if !addresses.insert((device.bus.as_str(), device.address)) {
                return Err(E::DuplicateI2cAddress {
                    id: device.reference.clone(),
                    value: device.address,
                });
            }
        }
        // 11
        let mut nets = BTreeSet::new();
        let signals = self
            .i2c_buses
            .iter()
            .flat_map(|b| [&b.sda, &b.scl])
            .chain(
                self.spi_buses
                    .iter()
                    .flat_map(|b| [&b.sck, &b.mosi, &b.miso]),
            )
            .chain(self.spi_devices.iter().map(|d| &d.cs));
        for signal in signals {
            let net = &signal.net;
            if !nets.insert(net.as_str()) {
                return Err(E::DuplicateNetName { id: net.clone() });
            }
            if domains.contains(net.as_str()) {
                return Err(E::NetShadowsDomain { id: net.clone() });
            }
        }
        // 12
        let mut constrained = BTreeSet::new();
        for net in &self.net_constraints {
            let id = &net.net_name;
            if !constrained.insert(id.as_str()) {
                return Err(E::DuplicateNetConstraint { id: id.clone() });
            }
            if !nets.contains(id.as_str()) && !domains.contains(id.as_str()) {
                return Err(E::UnknownNet { id: id.clone() });
            }
            if net.max_capacitance_femtofarads == 0 {
                return Err(E::ZeroCapacitance { id: id.clone() });
            }
            if net.max_rise_time_nanoseconds == 0 {
                return Err(E::ZeroRiseTime { id: id.clone() });
            }
            if net.impedance_ohms == Some(0) {
                return Err(E::ZeroImpedance { id: id.clone() });
            }
        }
        Ok(())
    }

    /// Step 2, in the order the README lists.
    fn identifiers(&self) -> Result<(), IrValidationError> {
        identifier(&self.project_name, || "project_name".into())?;
        identifier(&self.mcu.part, || "mcu.part".into())?;
        identifier(&self.mcu.supply, || "mcu.supply".into())?;
        for (i, domain) in self.voltage_domains.iter().enumerate() {
            identifier(&domain.id, || format!("voltage_domains[{i}].id"))?;
        }
        for (i, bus) in self.i2c_buses.iter().enumerate() {
            let at = format!("i2c_buses[{i}]");
            identifier(&bus.id, || format!("{at}.id"))?;
            optional_identifier(&bus.peripheral, || format!("{at}.peripheral"))?;
            identifier(&bus.domain, || format!("{at}.domain"))?;
            signal_identifiers(&bus.sda, &format!("{at}.sda"))?;
            signal_identifiers(&bus.scl, &format!("{at}.scl"))?;
        }
        for (i, bus) in self.spi_buses.iter().enumerate() {
            let at = format!("spi_buses[{i}]");
            identifier(&bus.id, || format!("{at}.id"))?;
            optional_identifier(&bus.peripheral, || format!("{at}.peripheral"))?;
            identifier(&bus.domain, || format!("{at}.domain"))?;
            signal_identifiers(&bus.sck, &format!("{at}.sck"))?;
            signal_identifiers(&bus.mosi, &format!("{at}.mosi"))?;
            signal_identifiers(&bus.miso, &format!("{at}.miso"))?;
        }
        for (i, device) in self.i2c_devices.iter().enumerate() {
            let at = format!("i2c_devices[{i}]");
            identifier(&device.reference, || format!("{at}.reference"))?;
            identifier(&device.part, || format!("{at}.part"))?;
            identifier(&device.supply, || format!("{at}.supply"))?;
            identifier(&device.bus, || format!("{at}.bus"))?;
        }
        for (i, device) in self.spi_devices.iter().enumerate() {
            let at = format!("spi_devices[{i}]");
            identifier(&device.reference, || format!("{at}.reference"))?;
            identifier(&device.part, || format!("{at}.part"))?;
            identifier(&device.supply, || format!("{at}.supply"))?;
            identifier(&device.bus, || format!("{at}.bus"))?;
            signal_identifiers(&device.cs, &format!("{at}.cs"))?;
        }
        for (i, net) in self.net_constraints.iter().enumerate() {
            identifier(&net.net_name, || format!("net_constraints[{i}].net_name"))?;
        }
        Ok(())
    }
}

impl SolvedTopology {
    /// Refuses the first malformed value found: version, hash, identifiers (in
    /// `assignments` a net, then its `pin`, `peripheral`, `signal`; then in
    /// `gpio` a net, then its pin; then each `i2c_ceilings` bus id; keys in
    /// sorted order), then alternate functions, then ceilings, then a net in
    /// both maps, then a pin used twice across both.
    pub fn validate(&self) -> Result<(), IrValidationError> {
        use IrValidationError as E;

        if self.schema_version != 1 {
            return Err(E::UnsupportedSchemaVersion {
                version: self.schema_version,
            });
        }
        let hash = self.specification_sha256.as_bytes();
        if hash.len() != 64 || !hash.iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return Err(E::MalformedSpecificationHash);
        }
        for (net, a) in &self.assignments {
            identifier(net, || format!("assignments.{net}"))?;
            identifier(&a.pin, || format!("assignments.{net}.pin"))?;
            identifier(&a.peripheral, || format!("assignments.{net}.peripheral"))?;
            identifier(&a.signal, || format!("assignments.{net}.signal"))?;
        }
        for (net, pin) in &self.gpio {
            identifier(net, || format!("gpio.{net}"))?;
            identifier(pin, || format!("gpio.{net}.pin"))?;
        }
        for bus in self.i2c_ceilings.keys() {
            identifier(bus, || format!("i2c_ceilings.{bus}"))?;
        }
        for (net, a) in &self.assignments {
            if a.alternate_function > 15 {
                return Err(E::AlternateFunctionOutOfRange {
                    id: net.clone(),
                    value: a.alternate_function,
                });
            }
        }
        for (bus, hz) in &self.i2c_ceilings {
            if ![100_000, 400_000, 1_000_000].contains(hz) {
                return Err(E::InvalidCeiling { id: bus.clone() });
            }
        }
        if let Some(net) = self.gpio.keys().find(|n| self.assignments.contains_key(*n)) {
            return Err(E::NetAssignedTwice { id: net.clone() });
        }
        let mut pins = BTreeSet::new();
        let all = self
            .assignments
            .values()
            .map(|a| &a.pin)
            .chain(self.gpio.values());
        for pin in all {
            if !pins.insert(pin.as_str()) {
                return Err(E::DuplicatePin { id: pin.clone() });
            }
        }
        Ok(())
    }
}
