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

pub mod bench;
pub mod board;
pub mod classes;
pub mod clock;
pub mod connectors;
mod drawn;
pub mod ds009;
pub mod electrical;
pub mod motor;
pub mod names;
pub mod orientation;
pub mod parts;
pub mod pin_devices;
pub mod pin_parts;
pub mod power;
pub mod power_parts;
pub mod serial;
pub mod silicon;
mod solve;
pub mod speeds;
pub mod thermal;
pub mod vos;

pub use motor::{MotorOutput, MotorProtocol, PwmTiming, SolvedClock, SolvedMotorOutput};
pub use serial::{Framing, Parity, PeerDeviationSource, SerialPort, SerialRole, SolvedSerialPort};
pub use solve::{NODE_LIMIT, RESERVED_PINS, SolveError, reserved_pins, solve, solve_with_limit};

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The specification's schema version (`RUL-14`). Version 2 added `board`
/// (`AET-WO-4100`); a version-1 file is refused by name, and nothing supplies a
/// board for it. Version 3 requires a `net_constraints` entry on every SPI
/// bus's `sck` and `mosi`, and names parts as the version-2 parts table does
/// (`AET-WO-4300c` D2, F11); nothing supplies either for an older file.
/// Version 4 added `connectors` (`AET-WO-4300f`). Version 5 requires
/// `net_classes` and `diff_pairs` (`AET-WO-1308`); an empty array declares
/// none. Version 6 requires `motor_outputs` and `mcu.hse_hz` (`AET-WO-4300a`
/// §1.1, `4400` D1). Version 7 requires `power` (`null` declares no power
/// tree) and `mcu.bor_level` (`AET-WO-4300d`, ruled D4). Version 8 requires
/// `accept_lifecycle`, each end-of-life or not-recommended part the board
/// uses acknowledged with a statement (`AET-WO-4210`, Scotty's ruling on
/// D1); an empty list acknowledges none. `4210` took 8 by the numbering
/// rule, merging after `4300d`. Version 9 requires `serial_ports`
/// (`AET-WO-4300b`, renumbered from 7 by the same rule); an empty list
/// declares none. Version 10 adds `power`'s judgement declarations
/// (`AET-WO-4300d3`, renumbered from 9 by the same rule): `hot_plug`,
/// `junction`, `controls` and `dc_bias_derating`. Version 11 requires
/// `mcu.supply_source` (`AET-WO-4300g`, renumbered from 9 by the same rule):
/// the connector pin the supply enters by, or `null` under a regulator.
/// Version 12 requires `peer_deviation_ppm` on every serial port, the remote
/// transmitter's clock deviation, with its `peer_deviation_source`, and
/// `acknowledge_unwitnessed` (an empty list acknowledges none), taken by
/// `AET-WO-4230` (renumbered from 10, then from 11, by the same rule, merging
/// after `4300d3` and then `4300g`). Nothing supplies any of them for an
/// older file.
/// Version 13 requires `pin_capacitance` on every I2C and SPI device, `null`
/// or a declaration (`AET-WO-4220b`).
/// Version 14 adds `test_points` and `mcu.reset_switch` (`AET-WO-1312`,
/// renumbered from 13 by the same rule, merging after `4220b`), each
/// optional as that work order states (absent is no test point and no
/// switch), so that a reader of version 13 refuses a file that may carry
/// them by its version, never by an unknown key.
/// Version 15 adds `board.max_outline_um` (`AET-WO-1313`), optional by the
/// precedent ruled on PR #191 (Q1): absent is today's placer, judged as
/// such, so that a reader of version 14 refuses a file that may carry it by
/// its version, never by an unknown key.
/// Version 16 adds `board.forward` and an SPI device's `orientation`
/// (`AET-WO-4400d`, ruled on PR #207), each optional by the same precedent.
/// Version 17 adds `mcu.foreign_clock` (`AET-WO-4400g` §1.2, Q1 ruled on PR
/// #229), optional by the same precedent: absent is no foreign clock, which
/// `simulate --foreign` refuses by name (`INV-06`).
/// Version 18 requires `power.battery.series_milliohms`, the healthy path's
/// series resistance with its source or `null` (undeclared), and each motor
/// output's `esc`, `{"none": {"source"}}` or `null` (undeclared)
/// (`AET-WO-2304b` §1.2): both are required keys, because their absence
/// would relax the battery-sag check (`RUL-14` as amended on PR #191).
/// Version 19 requires `mcu.core_supply` (`AET-WO-4240v` D-1, renumbered from
/// 18 by the same rule, merging after `2304b`), the board's supply
/// configuration with its source, and a declared foreign clock's `vos`;
/// nothing supplies either for an older file.
/// Version 20 replaces `mcu.hse_hz` with `mcu.hse` (`AET-WO-4240a`, `4240`
/// D4 ruled on #240): `null`, a crystal (`{"crystal": {part,
/// load_caps_femtofarads, pull_ppm, temperature_celsius, life_hours}}`) or an
/// external clock (`{"external": {hz_num, hz_den, ppm, source}}`). The key
/// is required; `hse_hz: 0`, the only value version 19 admitted, still reads
/// as `null`, so a committed specification moved by its version line alone.
/// Version 21 adds `board.drawn` (`AET-WO-1210`, renumbered from 20 by the
/// same rule, merging after `4240a`), optional by the same precedent: absent
/// is a drawn board, and `false` declares a twin stand-in no board is drawn
/// from.
/// Version 22 adds `pin_devices` (`AET-WO-1211a`, `1211` Q1–Q10 as ruled,
/// merging after `1210`), optional by `test_points`'s precedent (absent
/// declares none, and a specification with no driver gains no key), and
/// requires `pwm.idle` and `pwm.resolution_ps` on every PWM motor output,
/// with no default; `{"pulse_us": pulse_min_us}` and 1,000,000 restate what
/// the firmware wrote before, so no solved word of a committed specification
/// moved.
/// Version 23 requires `startup_max_us` and `startup_source` on a declared
/// `crystal` or `external` HSE (`AET-WO-4240b` §1.1, Q1 ruled 2026-11-20;
/// renumbered from 22 by the same rule, merging after `1211a`): the
/// bring-up's `HSERDY` bound with its source, which no default supplies; a
/// `null` HSE carries neither, so a committed specification moved by its
/// version line alone.
/// Version 24 requires `bulk` on every pin device (`AET-WO-1211e`, renumbered
/// from 23 by the same rule, merging after `4240b`): `null`, or the bulk
/// capacitor on the supply its part's row leaves unstated; a committed
/// specification moved by its version line alone.
pub const SPECIFICATION_SCHEMA_VERSION: u32 = 24;

/// The ground net: the emitter's ground symbol, never a declared domain.
pub const GND: &str = "GND";

/// The MCU's debug nets (`AET-WO-4300g`): the emitter puts SWDIO (`PA13`)
/// and SWCLK (`PA14`) on these, and NRST on [`NRST`], and a connector pin may
/// name any of them. No other declared net may take one, in any case.
/// `AET-WO-1312` adds [`BOOT0`]: a connector pin or test point that names it
/// puts the boot pin on its own net, through a pull-down
/// ([`bench::carries_boot0`]).
pub const SWDIO: &str = "SWDIO";
pub const SWCLK: &str = "SWCLK";
pub const NRST: &str = "NRST";
pub const BOOT0: &str = "BOOT0";
pub const DEBUG_NETS: [&str; 4] = [SWDIO, SWCLK, NRST, BOOT0];

/// The top-level specification, version 15 (`RUL-14`). Arrays keep the order a
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
    /// Private (`RUL-16`, `AET-WO-1204`): read through
    /// [`Specification::declared_i2c_devices`], which a runner's `clippy.toml`
    /// disallows.
    i2c_devices: Vec<I2cDevice>,
    /// Private as `i2c_devices` is; read through
    /// [`Specification::declared_spi_devices`].
    spi_devices: Vec<SpiDevice>,
    /// Private as the device lists are (`RUL-16`); read through
    /// [`Specification::declared_connectors`].
    connectors: Vec<connectors::Connector>,
    pub net_constraints: Vec<NetConstraint>,
    /// The copper layers, the stackup and the design rules (`board`).
    pub board: board::Board,
    /// Per-net track widths and vias (`AET-WO-1308`); empty declares none.
    pub net_classes: Vec<classes::NetClass>,
    /// Coupled pairs (`AET-WO-1308`); empty declares none.
    pub diff_pairs: Vec<classes::DiffPair>,
    /// Each motor output: a timer channel the solver assigns (`AET-WO-4300a`).
    pub motor_outputs: Vec<MotorOutput>,
    /// The power tree (`AET-WO-4300d`), or `null`: none declared. Required
    /// as a key (`INV-06`).
    #[serde(deserialize_with = "Option::deserialize")]
    pub power: Option<power::Power>,
    /// Each `eol` or `nrnd` part the board uses, acknowledged by name with its
    /// status and a statement of why (`AET-WO-4210`, ruled by Scotty on D1).
    /// `validate` refuses any such part not listed, and any entry that is not
    /// one: no end-of-life part is accepted silently (`INV-06`).
    pub accept_lifecycle: Vec<LifecycleAcceptance>,
    /// Each serial port: a USART or UART the solver assigns (`AET-WO-4300b`).
    pub serial_ports: Vec<SerialPort>,
    /// Each register's fields that no independent rule and no measured model
    /// reaction witnesses, acknowledged by register and class with the reason
    /// (`AET-WO-4230` R1, the architect's ruling on PR #161, comment
    /// 6911424608). `validate` refuses a malformed entry; the CLI refuses a
    /// stale one and `verify --fab-out` an unacknowledged ledger row. An empty
    /// list acknowledges none.
    pub acknowledge_unwitnessed: Vec<UnwitnessedAcknowledgement>,
    /// Each test point: a pad on a declared net (`AET-WO-1312`). Absent or
    /// empty declares none, as the work order states, and an empty list is
    /// not written back. Private as `connectors` is (`RUL-16`): read through
    /// [`Specification::declared_test_points`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    test_points: Vec<bench::TestPoint>,
    /// Each device with no bus (`AET-WO-1211a`): a driver IC bound to a
    /// busless parts row, with its GPIO signals, ADC inputs and bridges.
    /// Absent or empty declares none and is not written back, by
    /// `test_points`'s precedent, so a specification that declares no driver
    /// gains no key (`1211` P1c). Private (`RUL-16`): read through
    /// [`Specification::declared_pin_devices`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pin_devices: Vec<pin_devices::PinDevice>,
}

/// One acknowledged lifecycle: the part, its status as the parts table cites
/// it (`eol` or `nrnd`), and why the board uses it anyway.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct LifecycleAcceptance {
    pub part: String,
    pub status: String,
    pub statement: String,
}

/// The longest acknowledgement statement, in bytes.
pub const MAX_LIFECYCLE_STATEMENT: usize = 512;

/// One acknowledgement of unwitnessed ledger fields (`AET-WO-4230` R1, the
/// architect's ruling on PR #161, comment 6911424608): the register as the
/// ledger names it (`SPI1.CFG1`), the class its rows carry
/// ([`UnwitnessedClass`], held as written so that `validate` refuses an
/// unknown one by name), the exact fields it covers, and why the board is
/// fabricated with them unwitnessed. At most one entry per register and
/// class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct UnwitnessedAcknowledgement {
    pub register: String,
    pub class: String,
    pub fields: Vec<String>,
    pub reason: String,
}

impl UnwitnessedAcknowledgement {
    /// The class, typed; `None` for a name `validate` refuses.
    pub fn class_kind(&self) -> Option<UnwitnessedClass> {
        UnwitnessedClass::parse(&self.class)
    }
}

/// The ledger classes whose rows an acknowledgement covers (`AET-WO-4230`
/// contract D9): neither an independent rule nor a measured model reaction
/// judges them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum UnwitnessedClass {
    #[serde(rename = "ignored-by-model")]
    IgnoredByModel,
    #[serde(rename = "same-source")]
    SameSource,
    #[serde(rename = "silicon-only")]
    SiliconOnly,
}

impl UnwitnessedClass {
    /// Every class, in the order the ruling names them.
    pub const ALL: [Self; 3] = [Self::IgnoredByModel, Self::SameSource, Self::SiliconOnly];

    /// The name the specification and the ledger use.
    pub fn name(self) -> &'static str {
        match self {
            Self::IgnoredByModel => "ignored-by-model",
            Self::SameSource => "same-source",
            Self::SiliconOnly => "silicon-only",
        }
    }

    /// The class `name` names exactly, if any.
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.name() == name)
    }
}

/// Whether `register` has the ledger's shape for a register: a peripheral
/// and a register name joined by one `.`, each an upper-case letter followed
/// by upper-case letters, digits or `_` (`SPI1.CFG1`, `FLASH.OPTSR_CUR`).
fn register_name_ok(register: &str) -> bool {
    let segment = |s: &str| {
        let b = s.as_bytes();
        b.first().is_some_and(u8::is_ascii_uppercase)
            && b.iter()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == b'_')
    };
    register
        .split_once('.')
        .is_some_and(|(p, r)| segment(p) && segment(r))
}

/// The banner every verdict, generated file and fabrication package carries
/// when the board uses an `eol` or `nrnd` part (`AET-WO-4210`, ruled by Scotty
/// on D1): one line per part, naming its status, the devices and the vendor
/// page it is cited to. Empty for a board of `active` parts, so such a
/// board's outputs are unchanged. Devices in the order given; a part with no
/// table entry has no lifecycle and is not named.
pub fn lifecycle_banner<'a>(
    table: &parts::PartsTable,
    devices: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<String> {
    let mut by_part: Vec<(&parts::Part, Vec<&str>)> = Vec::new();
    for (reference, part) in devices {
        let Some(p) = table.get(part) else { continue };
        if p.lifecycle == parts::Lifecycle::Active {
            continue;
        }
        match by_part.iter_mut().find(|(q, _)| q.part == p.part) {
            Some((_, refs)) => refs.push(reference),
            None => by_part.push((p, vec![reference])),
        }
    }
    by_part
        .into_iter()
        .map(|(p, refs)| {
            let c = &p.citations.lifecycle;
            format!(
                "LIFECYCLE {}: {} ({}) is {} by its vendor ({} {}, {}), acknowledged in accept_lifecycle",
                p.lifecycle.name().to_ascii_uppercase(),
                p.part,
                refs.join(", "),
                match p.lifecycle {
                    parts::Lifecycle::Eol => "end of life",
                    _ => "not recommended for new designs",
                },
                c.document,
                c.revision,
                c.section
            )
        })
        .collect()
}

/// The microcontroller: a part name, the domain that supplies it, its HSE
/// (`mcu.hse`, `AET-WO-4240a`: a crystal or an external clock, or `null` for
/// none; [`clock::Hse`]), and the brown-out level its option bytes select (`AET-WO-4300d` D4: the
/// scenario and the supply floor both read it; the option-byte write is out
/// of scope).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Mcu {
    pub part: String,
    pub supply: String,
    /// Required as a key, `null` for none. A version-19 file's `hse_hz: 0`
    /// reads as `null` (the only value it ever admitted); `hse_hz` with any
    /// other value, or beside `hse`, is refused at parsing.
    #[serde(deserialize_with = "Option::deserialize")]
    pub hse: Option<clock::Hse>,
    /// `0` (RM0433's factory default) to `3`.
    pub bor_level: u8,
    /// The core supply configuration the board is built for (`AET-WO-4240v`
    /// D-1): required, with its source.
    pub core_supply: CoreSupply,
    /// Where the supply enters the board when no regulator of `power` drives
    /// it: one declared connector's pin on the supply (`AET-WO-4300g`).
    /// Required as a key; `null` when a regulator's output is the supply.
    #[serde(deserialize_with = "Option::deserialize")]
    pub supply_source: Option<SupplySource>,
    /// A push-button from `NRST` to `GND` (`AET-WO-1312`). Absent is `false`,
    /// as the work order states, and `false` is not written back.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reset_switch: bool,
    /// The kernel clocks a foreign firmware programs (`AET-WO-4400g` §1.2):
    /// `simulate --foreign` requires it and a run of the generated firmware
    /// refuses it. Absent is none, and none is not written back.
    #[serde(
        default,
        deserialize_with = "board::present",
        skip_serializing_if = "Option::is_none"
    )]
    pub foreign_clock: Option<motor::ForeignClock>,
}

/// The core supply configuration (`AET-WO-4240v` §1.1, D-1), as RM0433 Rev 9
/// Table 34 and §6.8.4 name the two this part has (it has no SMPS):
/// - `ldo`: the internal LDO supplies VCORE, VCAP decoupled only;
///   `PWR_CR3` is written with `LDOEN` 1 and `BYPASS` 0;
/// - `bypass`: an external supply drives VCORE through VCAP; `LDOEN` 0 and
///   `BYPASS` 1. `validate` and `solve` accept it; `generate`, `emit` and
///   `simulate` refuse it as `CoreSupplyNotGenerated`, since no board this
///   engine emits draws an external VCORE.
///
/// No other pair is offered: an invalid `LDOEN`/`BYPASS` pair locks
/// `PWR_CR3` until a power cycle (§6.8.4, p. 308).
///
/// `source` says where the mode was read, in `foreign_clock`'s shape (1–200
/// bytes of printable ASCII). A source is cited only with a document number
/// and its revision (F-CITE strict, the ruling on #240): a figure without
/// them is uncited, and nothing rests on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct CoreSupply {
    pub mode: vos::CoreSupplyMode,
    pub source: String,
}

/// A connector pin the supply enters by.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SupplySource {
    pub connector: String,
    pub pin: String,
}

/// The highest BOR level the option bytes can select (RM0433, `BOR_LEV`).
pub const MAX_BOR_LEVEL: u8 = 3;

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
    /// Its pins' capacitance as the specification declares it, or `null`.
    /// Required as a key (`INV-06`).
    #[serde(deserialize_with = "Option::deserialize")]
    pub pin_capacitance: Option<PinCapacitance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SpiDevice {
    pub reference: String,
    pub part: String,
    pub supply: String,
    pub bus: String,
    pub cs: Signal,
    /// As [`I2cDevice::pin_capacitance`].
    #[serde(deserialize_with = "Option::deserialize")]
    pub pin_capacitance: Option<PinCapacitance>,
    /// An IMU's orientation in the board frame (`AET-WO-4400d`,
    /// [`orientation`]). Optional: an undeclared IMU is placed as before and
    /// refused by `generate hwdef` (`INV-06`). An explicit `null` is refused,
    /// as `board.max_outline_um` refuses it.
    #[serde(
        default,
        deserialize_with = "board::present",
        skip_serializing_if = "Option::is_none"
    )]
    pub orientation: Option<orientation::Orientation>,
}

/// A device's pin capacitance as the specification declares it (`AET-WO-4220b`;
/// the `AET-WO-4220` D10 ruling, comment 6890080737): for a part whose
/// datasheet prints a typical only, or nothing. `extract` counts it on every
/// bounded line the part's pins sit on, under its condition, named as
/// declared and never as a maximum (`RUL-19`). A part whose parts-table row
/// cites a maximum is counted at that maximum (`AET-WO-4300c` D3), and a
/// declaration for it is refused (`PinCapacitanceAlreadyCited`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct PinCapacitance {
    pub declared: DeclaredPinCapacitance,
}

/// The value counted for each of the device's pins on a bounded line, the
/// condition under which the bound then holds, and where the value comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct DeclaredPinCapacitance {
    pub femtofarads: u64,
    pub condition: String,
    pub source: String,
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

/// The I2C speeds, in Hz, a bus may declare or have as its ceiling: Standard-mode,
/// Fast-mode and Fast-mode Plus.
const I2C_SPEEDS_HZ: [u32; 3] = [100_000, 400_000, 1_000_000];

/// The `schema_version` of a [`SolvedTopology`]. Version 2 added `i2c_speeds`
/// (`AET-WO-1205`), version 3 `spi_devices` (`AET-WO-4200`), version 4
/// `spi_lines` (`AET-WO-4300c`), version 5 `clock` and `motor_outputs`
/// (`AET-WO-4300a`), version 6 `power` (`AET-WO-4300d`), version 7
/// `serial_ports` (`AET-WO-4300b`), version 8 `spi_controllers`,
/// `i2c_timings` and `bor_level` (`AET-WO-4230`), version 9 `mcu` and
/// `pin_speeds` (`AET-WO-4400b` D2), version 10 an SPI device's
/// `orientation` (`AET-WO-4400d`), version 11 `foreign_clock`
/// (`AET-WO-4400g`), version 12 `core_supply` and the clock's `vos`
/// (`AET-WO-4240v`), version 13 the clock's `hse` (`AET-WO-4240a`: absent
/// for `mcu.hse` `null`, so such a topology differs only in its version),
/// version 14 `gpio`'s direction with an output's `initial` or an input's
/// `pull`, `adc_inputs`, a PWM output's `idle` and `resolution_ps`, and
/// `pin_devices` (`AET-WO-1211a`: absent when none is declared), version 15
/// the HSE tree's `flash_acr`, its declaration's start-up bound and the SPI
/// kernels `pll1_q` and `pclk` (`AET-WO-4240b` Q2, Q3, Q6, renumbered from
/// 14 by the same rule); an older file is refused, never defaulted
/// (`INV-06`).
pub const SOLVED_SCHEMA_VERSION: u32 = 15;

/// What the solver derived from one specification (`RUL-16`). Keyed by net, in
/// `BTreeMap`s, so serialization is a function of content. `assignments` holds
/// the signals on an alternate function; `gpio` holds each net driven as a
/// plain GPIO output (an SPI device's `cs`), net to pin. `i2c_ceilings` holds
/// each I2C bus's maximum speed in Hz, keyed by bus id, and `i2c_speeds` the
/// speed the specification declared for it, under the same keys
/// (`AET-WO-1205`). `analog_lines` holds
/// the analog facts of every solved I2C line, keyed by net, which the twin's
/// analog model reads instead of the specification (`AET-WO-2200`, `RUL-16`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedTopology {
    pub schema_version: u32,
    /// SHA-256 of the specification solved, 64 lowercase hex digits.
    pub specification_sha256: String,
    #[serde(deserialize_with = "unique_keys")]
    pub assignments: BTreeMap<String, PinAssignment>,
    /// Each net on a plain GPIO, with its direction and an output's initial
    /// level or an input's MCU pull (`AET-WO-1211a`, solved version 14): an
    /// SPI device's `cs` (an output at `high`) and a pin device's signals.
    #[serde(deserialize_with = "unique_keys")]
    pub gpio: BTreeMap<String, pin_devices::SolvedGpio>,
    /// Each ADC input, keyed by net, on its pin with the peripheral and
    /// channel the solver chose, ADC1 or ADC3 unless pinned (`AET-WO-1211a`,
    /// `1211` Q8): a pin device's analog inputs and each `power.sense` net,
    /// which keeps its place in `power` too.
    #[serde(deserialize_with = "unique_keys")]
    pub adc_inputs: BTreeMap<String, pin_devices::SolvedAdcInput>,
    #[serde(deserialize_with = "unique_keys")]
    pub i2c_ceilings: BTreeMap<String, u32>,
    #[serde(deserialize_with = "unique_keys")]
    pub i2c_speeds: BTreeMap<String, u32>,
    #[serde(deserialize_with = "unique_keys")]
    pub analog_lines: BTreeMap<String, AnalogLine>,
    /// Every I2C device of the specification, keyed by reference, on the
    /// peripheral its bus was solved to (`AET-WO-2203`, `RUL-16`). Read it
    /// through [`SolvedTopology::i2c_devices_in_order`].
    #[serde(deserialize_with = "unique_keys")]
    pub i2c_devices: BTreeMap<String, SolvedI2cDevice>,
    /// Every SPI device of the specification, keyed by reference, on the
    /// peripheral its bus was solved to, with its chip select's net, a key of
    /// `gpio` (`AET-WO-4200`, `RUL-16`). Read it through
    /// [`SolvedTopology::spi_devices_in_order`].
    #[serde(deserialize_with = "unique_keys")]
    pub spi_devices: BTreeMap<String, SolvedSpiDevice>,
    /// Each SPI line whose edges the MCU drives at a rate its peripheral sets
    /// (each bus's SCK and MOSI), keyed by net, with what the output speed
    /// check reads: copied by `solve` from the specification so the
    /// generator chooses each pin's speed from the topology alone
    /// (`AET-WO-4300c` D2, D4, `RUL-16`).
    #[serde(deserialize_with = "unique_keys")]
    pub spi_lines: BTreeMap<String, SpiLine>,
    /// The clock the generated firmware runs on, a solved fact: the reset
    /// clock (`AET-WO-4300a` §1.2, `4400` D1). The timer values in
    /// `motor_outputs` are solved for it, and the static timer check reads
    /// only it.
    pub clock: SolvedClock,
    /// Each motor output, keyed by its label, which is its net: a key of
    /// `assignments` on a `TIMn_CHk` signal (`AET-WO-4300a`, `RUL-16`).
    #[serde(deserialize_with = "unique_keys")]
    pub motor_outputs: BTreeMap<String, SolvedMotorOutput>,
    /// The power tree as the checks derived it (`AET-WO-4300d`), or `null`
    /// when the specification declares none: what `1303`'s supply bound and
    /// the slice-B emitter read, never the specification (`RUL-16`).
    #[serde(deserialize_with = "Option::deserialize")]
    pub power: Option<SolvedPower>,
    /// Each serial port, keyed by its label; its TX and RX nets are keys of
    /// `assignments` on `<peripheral>_TX` / `_RX` (`AET-WO-4300b`, `RUL-16`).
    #[serde(deserialize_with = "unique_keys")]
    pub serial_ports: BTreeMap<String, SolvedSerialPort>,
    /// Each solved SPI controller, keyed by peripheral (`SPI1` … `SPI6`):
    /// its kernel clock, the speed declared for it and the baud rate solved
    /// (`AET-WO-4230` §1.1, §1.2). [`silicon::check_spi`] judges it.
    #[serde(deserialize_with = "unique_keys")]
    pub spi_controllers: BTreeMap<String, SolvedSpiController>,
    /// Each solved I2C controller, keyed by peripheral (`I2C1` … `I2C4`):
    /// its kernel clock, `CR1` and one `TIMINGR` per mode (`AET-WO-4230`
    /// §1.4). [`silicon::check_i2c`] judges it.
    #[serde(deserialize_with = "unique_keys")]
    pub i2c_timings: BTreeMap<String, SolvedI2cTiming>,
    /// `mcu.bor_level`, copied by `solve` so the twin presents it without
    /// reading the specification (`AET-WO-4230` §1.6, `RUL-16`).
    pub bor_level: u8,
    /// `mcu.core_supply.mode`, copied by `solve` (`AET-WO-4240v`, `RUL-16`):
    /// the generator writes `PWR_CR3` from it.
    pub core_supply: vos::CoreSupplyMode,
    /// `mcu.part`, copied by `solve`, so a generator names the MCU without
    /// reading the specification (`AET-WO-4400b`, `RUL-16`).
    pub mcu: String,
    /// Each pin's output speed, keyed by its net (every key of `assignments`
    /// and of `gpio`), as `OSPEEDR` (`0` or `1`; `10` and `11` are never
    /// selected, `AET-WO-4300` D1): chosen by `solve` with
    /// [`speeds::OutputSpeeds::choose`] for each line of `spi_lines`, and `0`
    /// for every other pin (`AET-WO-4300c` D1, D3, D4). The generators and the
    /// pinout check read it; neither chooses (`AET-WO-4400b` D2, `RUL-20`).
    #[serde(deserialize_with = "unique_keys")]
    pub pin_speeds: BTreeMap<String, u8>,
    /// `mcu.foreign_clock`'s values, copied by `solve` (`AET-WO-4400g`,
    /// `RUL-16`), or `null` when none is declared. When present, the twin
    /// builds every timer and USART at them instead of at `clock`'s.
    #[serde(deserialize_with = "Option::deserialize")]
    pub foreign_clock: Option<motor::SolvedForeignClock>,
    /// Each pin device, keyed by reference, with its part (`AET-WO-1211a`).
    /// Absent when the specification declares none, and not written back
    /// then, so such a topology gains no key; every crate but the IR refuses
    /// a topology that carries one, by name, until its work order lands.
    #[serde(
        default,
        deserialize_with = "unique_keys",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub pin_devices: BTreeMap<String, pin_devices::SolvedPinDevice>,
}

/// The kernel clock an SPI controller is given (`AET-WO-4230` §1.1): on a
/// board with no HSE, `per_ck` for SPI1–3 (`RCC_D2CCIP1R.SPI123SEL` = `100`,
/// RM0433 Rev 9 §8.7.20; `per_ck` is `hsi_ker_ck` at `CKPERSEL`'s reset `00`,
/// §8.7.19), `hsi_ker_ck` for SPI4/5 (`SPI45SEL` = `011`, §8.7.20) and SPI6
/// (`RCC_D3CCIPR.SPI6SEL` = `011`, §8.7.22). On an HSE board every select is
/// left at its reset `000` (`4240` D6, ruled; `AET-WO-4240b` Q3): `pll1_q_ck`
/// for SPI1–3, the APB clock for SPI4–6 (`rcc_pclk2`, `rcc_pclk4` for SPI6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpiKernel {
    PerCk,
    HsiKerCk,
    Pll1Q,
    Pclk,
}

/// One SPI controller as solved (`AET-WO-4230` §1.2, contract D2 and
/// amendment 1). `declared_hz` is the lowest `speed_hz` of the buses on it;
/// `mbr` the least with the fastest kernel (`kernel_hz` at HSI's worst
/// +14,984 ppm, rounded up) divided by `2^(mbr+1)` at or below it;
/// `achieved_hz` the nominal kernel so divided and `achieved_max_hz` the
/// fastest, each rounded down. `check speeds` judges `achieved_max_hz`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedSpiController {
    pub kernel: SpiKernel,
    pub kernel_hz: u64,
    pub declared_hz: u32,
    pub mbr: u8,
    pub achieved_hz: u64,
    pub achieved_max_hz: u64,
}

/// One I2C controller as solved (`AET-WO-4230` §1.4, contract D2, D3):
/// its kernel clock, the whole `CR1` the bring-up writes with `PE` clear,
/// and `TIMINGR` for each of `Standard100k`, `Fast400k` and `FastPlus1M`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedI2cTiming {
    pub kernel_hz: u64,
    pub cr1: u32,
    #[serde(deserialize_with = "unique_keys")]
    pub timingr: BTreeMap<String, u32>,
}

/// The solved power tree. Every load returns through `GND`, so
/// `input_steady_milliamps` is also the total current of the shared ground
/// path that the VSS term of a supply bound counts (ruled D3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedPower {
    /// Keyed by domain id.
    #[serde(deserialize_with = "unique_keys")]
    pub domains: BTreeMap<String, SolvedDomain>,
    pub input_steady_milliamps: u64,
    pub input_fault_milliamps: u64,
    /// Every condition the checks rest on, each once, sorted.
    pub conditions: Vec<String>,
    /// Each battery sense net's ADC pin, keyed by net (`AET-WO-4300d2` §1.7):
    /// empty when the tree declares no sense. A required key, never
    /// defaulted (`INV-06`): a topology written before it is refused, not
    /// read as "no sense solved".
    #[serde(deserialize_with = "unique_keys")]
    pub sense: BTreeMap<String, SolvedSense>,
}

/// The pin a sense net was solved onto: an analog pin, so no alternate
/// function and no entry in `assignments`. `channels` is every single-ended
/// ADC input the pin carries, sorted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedSense {
    pub pin: String,
    pub position: u16,
    pub channels: Vec<String>,
}

/// One domain: its regulator, the regulator's guaranteed output at the
/// source pad (the domain's tolerance holds there, ruled D3), the source and
/// return pads (spike S5; for a buck, RFBT's pad 1 and RFBB's pad 2, `K5`),
/// the regulator's load, and each consumer's floor: the lowest supply at
/// which that consumer's own guarantees hold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedDomain {
    pub regulator: String,
    pub part: String,
    pub vout_min_microvolts: u64,
    pub vout_max_microvolts: u64,
    pub source: Pad,
    #[serde(rename = "return")]
    pub ret: Pad,
    pub load_milliamps: u64,
    pub consumers: Vec<Consumer>,
}

/// A pad: a part's reference and its pad number.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Pad {
    pub reference: String,
    pub pad: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Consumer {
    pub name: String,
    pub floor_millivolts: u64,
}

impl From<power::Checked> for SolvedPower {
    fn from(c: power::Checked) -> Self {
        let pad = |(reference, pad): (String, String)| Pad { reference, pad };
        SolvedPower {
            domains: c
                .domains
                .into_iter()
                .map(|(id, d)| {
                    (
                        id,
                        SolvedDomain {
                            regulator: d.regulator,
                            part: d.part,
                            vout_min_microvolts: d.vout_min_microvolts,
                            vout_max_microvolts: d.vout_max_microvolts,
                            source: pad(d.source),
                            ret: pad(d.ret),
                            load_milliamps: d.load_milliamps,
                            consumers: d
                                .consumers
                                .into_iter()
                                .map(|(name, floor_millivolts)| Consumer {
                                    name,
                                    floor_millivolts,
                                })
                                .collect(),
                        },
                    )
                })
                .collect(),
            input_steady_milliamps: c.input_steady_milliamps,
            input_fault_milliamps: c.input_fault_milliamps,
            conditions: c.conditions,
            sense: BTreeMap::new(),
        }
    }
}

/// One SPI line the MCU drives: the bus's declared clock (the most it
/// toggles), the declared bound on everything it drives but its own pin, the
/// declared rise time, and the minimum of the MCU's supply (nominal less its
/// tolerance, rounded down), which selects the VDD band.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SpiLine {
    pub bus: String,
    /// `SCK` or `MOSI`.
    pub signal: String,
    pub rate_hz: u32,
    pub capacitance_femtofarads: u64,
    pub rise_time_ns: u32,
    pub vdd_min_millivolts: u32,
}

/// The signals of an SPI bus whose edges the MCU drives at its peripheral's
/// rate (`AET-WO-4300c` D4): MISO is an input in master mode, and a chip
/// select is a software-driven GPIO.
pub const DRIVEN_SPI_SIGNALS: [&str; 2] = ["SCK", "MOSI"];

/// A domain's maximum, in millivolts: nominal plus its tolerance, rounded
/// up.
pub fn maximum_millivolts(domain: &VoltageDomain) -> u32 {
    let high = (u64::from(domain.nominal_millivolts)
        * (1000 + u64::from(domain.tolerance_permille)))
    .div_ceil(1000);
    u32::try_from(high).unwrap_or(u32::MAX)
}

/// A domain's minimum, in millivolts: nominal less its tolerance, rounded
/// down.
pub fn minimum_millivolts(domain: &VoltageDomain) -> u32 {
    let low = u64::from(domain.nominal_millivolts)
        * (1000 - u64::from(domain.tolerance_permille.min(1000)))
        / 1000;
    u32::try_from(low).expect("at most the nominal")
}

/// One I2C device as the generators need it, copied by `solve` from the
/// specification. A map keyed by reference sorts `U10` before `U3`, but the
/// specification's order decides the generated firmware's read order and the
/// judge's record offsets, so `index` keeps it: the indices of a topology are
/// exactly `0..n`, each once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedI2cDevice {
    /// Position in the specification's `i2c_devices`.
    pub index: u32,
    pub part: String,
    /// The bus's id, a key of `i2c_ceilings`.
    pub bus: String,
    /// ST's name of the controller the bus was solved to, such as `I2C1`.
    pub peripheral: String,
    pub address: u8,
}

/// One SPI device as the generators need it, copied by `solve` from the
/// specification, with `index` its place in the specification's order, as
/// [`SolvedI2cDevice`] has. Its chip select is the `gpio` entry of `cs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedSpiDevice {
    /// Position in the specification's `spi_devices`.
    pub index: u32,
    pub part: String,
    /// The bus's id.
    pub bus: String,
    /// ST's name of the controller the bus was solved to, such as `SPI1`.
    pub peripheral: String,
    /// The net of the device's chip select.
    pub cs: String,
    /// The IMU's declared orientation, copied by `solve` (`AET-WO-4400d`,
    /// solved schema 10); absent when the specification declares none.
    #[serde(
        default,
        deserialize_with = "board::present",
        skip_serializing_if = "Option::is_none"
    )]
    pub orientation: Option<orientation::Orientation>,
}

/// One I2C line as the analog model needs it, copied by `solve` from the
/// specification: the bus's pull-up and its domain's nominal voltage, and
/// the net constraint's capacitance and rise-time bound, which is the
/// ceiling the measured rise is judged against (`AET-WO-1202`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct AnalogLine {
    pub bus: String,
    pub pull_up_ohms: u32,
    pub capacitance_femtofarads: u64,
    pub ceiling_ns: u32,
    pub domain_millivolts: u32,
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
pub(crate) fn unique_keys<'de, D: Deserializer<'de>, V: Deserialize<'de>>(
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
pub(crate) use object_only;

object_only!(
    Specification,
    LifecycleAcceptance,
    UnwitnessedAcknowledgement,
    CoreSupply,
    SupplySource,
    VoltageDomain,
    Signal,
    I2cBus,
    SpiBus,
    I2cDevice,
    SpiDevice,
    PinCapacitance,
    DeclaredPinCapacitance,
    NetConstraint,
    PinAssignment,
    AnalogLine,
    SolvedI2cDevice,
    SolvedSpiDevice,
    SolvedSpiController,
    SolvedI2cTiming,
    SpiLine,
    SolvedPower,
    SolvedSense,
    SolvedDomain,
    Pad,
    Consumer
);

impl Serialize for Mcu {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Mcu::serialize(self, serializer)
    }
}

/// An object only, as [`object_only`]'s, read through [`McuWire`]'s derive
/// so a duplicate key anywhere under `mcu` is refused as before.
/// `hse_hz`, the version-19 key `hse` replaced (`AET-WO-4240a` §1.2), reads
/// as `"hse": null` when it is 0, so a committed specification moves by its
/// version line alone; any other value was always refused, and is refused
/// here, as is `hse_hz` beside `hse`, and neither.
impl<'de> Deserialize<'de> for Mcu {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct McuVisitor;
        impl<'de> Visitor<'de> for McuVisitor {
            type Value = Mcu;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a Mcu object")
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Mcu, A::Error> {
                use serde::de::Error;
                let w = McuWire::deserialize(MapAccessDeserializer::new(map))?;
                let hse = match (w.hse, w.hse_hz) {
                    (Some(hse), None) => hse,
                    (None, Some(0)) => None,
                    (None, Some(n)) => {
                        return Err(A::Error::custom(format!(
                            "mcu.hse_hz is {n}: a crystal frequency was always refused there; declare `mcu.hse`"
                        )));
                    }
                    (None, None) => return Err(A::Error::missing_field("hse")),
                    (Some(_), Some(_)) => {
                        return Err(A::Error::custom(
                            "duplicate field `hse` (`hse_hz` is the version-19 spelling of `hse`)",
                        ));
                    }
                };
                Ok(Mcu {
                    part: w.part,
                    supply: w.supply,
                    hse,
                    bor_level: w.bor_level,
                    core_supply: w.core_supply,
                    supply_source: w.supply_source,
                    reset_switch: w.reset_switch,
                    foreign_clock: w.foreign_clock,
                })
            }
        }
        deserializer.deserialize_map(McuVisitor)
    }
}

/// The wire form of [`Mcu`]: its fields with each one's own reading, and
/// `hse` and the version-19 `hse_hz` both optional, so the conversion names
/// what is missing or doubled.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct McuWire {
    part: String,
    supply: String,
    #[serde(default, deserialize_with = "present")]
    hse: Option<Option<clock::Hse>>,
    #[serde(default)]
    hse_hz: Option<u32>,
    bor_level: u8,
    core_supply: CoreSupply,
    #[serde(deserialize_with = "Option::deserialize")]
    supply_source: Option<SupplySource>,
    #[serde(default)]
    reset_switch: bool,
    #[serde(default, deserialize_with = "board::present")]
    foreign_clock: Option<motor::ForeignClock>,
}

/// The wire form of a [`SolvedTopology`]. It differs in one field: `i2c_speeds`
/// may be absent, so that a version-1 file (which has none) reaches
/// `validate` and is refused as `UnsupportedSchemaVersion`, not as a missing
/// key (`AET-WO-1205`, `INV-06`).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SolvedWire {
    schema_version: u32,
    specification_sha256: String,
    #[serde(deserialize_with = "unique_keys")]
    assignments: BTreeMap<String, PinAssignment>,
    #[serde(deserialize_with = "unique_keys")]
    gpio: BTreeMap<String, GpioWire>,
    #[serde(default, deserialize_with = "unique_keys_some")]
    adc_inputs: Option<BTreeMap<String, pin_devices::SolvedAdcInput>>,
    #[serde(deserialize_with = "unique_keys")]
    i2c_ceilings: BTreeMap<String, u32>,
    #[serde(default, deserialize_with = "unique_keys_some")]
    i2c_speeds: Option<BTreeMap<String, u32>>,
    #[serde(deserialize_with = "unique_keys")]
    analog_lines: BTreeMap<String, AnalogLine>,
    #[serde(deserialize_with = "unique_keys")]
    i2c_devices: BTreeMap<String, SolvedI2cDevice>,
    #[serde(default, deserialize_with = "unique_keys_some")]
    spi_devices: Option<BTreeMap<String, SolvedSpiDevice>>,
    #[serde(default, deserialize_with = "unique_keys_some")]
    spi_lines: Option<BTreeMap<String, SpiLine>>,
    #[serde(default)]
    clock: Option<SolvedClock>,
    #[serde(default, deserialize_with = "unique_keys_some")]
    motor_outputs: Option<BTreeMap<String, SolvedMotorOutput>>,
    /// Absent under an older version; `null` or a tree under this one.
    #[serde(default, deserialize_with = "present")]
    power: Option<Option<SolvedPower>>,
    #[serde(default, deserialize_with = "unique_keys_some")]
    serial_ports: Option<BTreeMap<String, SolvedSerialPort>>,
    #[serde(default, deserialize_with = "unique_keys_some")]
    spi_controllers: Option<BTreeMap<String, SolvedSpiController>>,
    #[serde(default, deserialize_with = "unique_keys_some")]
    i2c_timings: Option<BTreeMap<String, SolvedI2cTiming>>,
    #[serde(default)]
    bor_level: Option<u8>,
    #[serde(default)]
    core_supply: Option<vos::CoreSupplyMode>,
    #[serde(default)]
    mcu: Option<String>,
    #[serde(default, deserialize_with = "unique_keys_some")]
    pin_speeds: Option<BTreeMap<String, u8>>,
    /// Absent under an older version; `null` or a clock under this one.
    #[serde(default, deserialize_with = "present")]
    foreign_clock: Option<Option<motor::SolvedForeignClock>>,
    #[serde(default, deserialize_with = "unique_keys")]
    pin_devices: BTreeMap<String, pin_devices::SolvedPinDevice>,
}

/// A `gpio` entry as written: the directional shape of version 14, or the
/// bare pin of versions before it, read only so that `validate` refuses the
/// older file by its version, never by its shape.
#[derive(Deserialize)]
#[serde(untagged)]
enum GpioWire {
    Solved(pin_devices::SolvedGpio),
    Pin(String),
}

/// A key that is present, whatever its value: `null` reads as `Some(None)`.
fn present<'de, D: Deserializer<'de>, V: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<Option<V>>, D::Error> {
    Option::<V>::deserialize(deserializer).map(Some)
}

/// A foreign clock's values (`AET-WO-4400g` §1.2), in the specification
/// and in a topology alike: no zero, each USART the platform carries
/// ([`serial::PERIPHERALS`]) and no other, and the USARTs one mux selects
/// for at one clock (RM0433 Rev 9 §8.7.21, `RCC_D2CCIP2R`: `USART16SEL`
/// for USART1 and USART6, `USART234578SEL` for the other six).
fn foreign_clock_values(c: &motor::SolvedForeignClock) -> Result<(), IrValidationError> {
    use IrValidationError as E;

    for (field, hz) in [
        ("apb1_timers_hz".to_owned(), c.apb1_timers_hz),
        ("apb2_timers_hz".to_owned(), c.apb2_timers_hz),
    ]
    .into_iter()
    .chain(c.usarts.iter().map(|(k, v)| (format!("usarts.{k}"), *v)))
    {
        if hz == 0 {
            return Err(E::ForeignClockZero { field });
        }
    }
    let expected: BTreeSet<&str> = serial::PERIPHERALS.into_iter().collect();
    let named: BTreeSet<&str> = c.usarts.keys().map(String::as_str).collect();
    if named != expected {
        let list = |a: &BTreeSet<&str>, b: &BTreeSet<&str>| {
            a.difference(b).map(|s| (*s).to_owned()).collect::<Vec<_>>()
        };
        return Err(E::ForeignClockUsarts {
            expected: serial::PERIPHERALS
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            missing: list(&expected, &named),
            unknown: list(&named, &expected),
        });
    }
    for group in [
        &["USART1", "USART6"][..],
        &["USART2", "USART3", "UART4", "UART5", "UART7", "UART8"][..],
    ] {
        let first = group[0];
        for second in &group[1..] {
            if c.usarts[first] != c.usarts[*second] {
                return Err(E::ForeignClockMuxSplit {
                    first: first.to_owned(),
                    first_hz: c.usarts[first],
                    second: (*second).to_owned(),
                    second_hz: c.usarts[*second],
                });
            }
        }
    }
    Ok(())
}

fn unique_keys_some<'de, D: Deserializer<'de>, V: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<BTreeMap<String, V>>, D::Error> {
    unique_keys(deserializer).map(Some)
}

impl Serialize for SolvedTopology {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SolvedTopology::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for SolvedTopology {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;

        impl<'de> Visitor<'de> for ObjectVisitor {
            type Value = SolvedTopology;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a SolvedTopology object")
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<SolvedTopology, A::Error> {
                let w = SolvedWire::deserialize(MapAccessDeserializer::new(map))?;
                // A missing `i2c_speeds` is read as empty only under another
                // version, which `validate` refuses first; under the current
                // one it is a missing key, never a default.
                let i2c_speeds = match w.i2c_speeds {
                    Some(m) => m,
                    None if w.schema_version != SOLVED_SCHEMA_VERSION => BTreeMap::new(),
                    None => return Err(serde::de::Error::missing_field("i2c_speeds")),
                };
                // The same for `spi_devices`, which version 3 added.
                let spi_devices = match w.spi_devices {
                    Some(m) => m,
                    None if w.schema_version != SOLVED_SCHEMA_VERSION => BTreeMap::new(),
                    None => return Err(serde::de::Error::missing_field("spi_devices")),
                };
                // The same for `spi_lines`, which version 4 added.
                let spi_lines = match w.spi_lines {
                    Some(m) => m,
                    None if w.schema_version != SOLVED_SCHEMA_VERSION => BTreeMap::new(),
                    None => return Err(serde::de::Error::missing_field("spi_lines")),
                };
                // `clock`, which version 5 added, and `core_supply`, which
                // version 12 added, have no stand-in under any version: a
                // clock carries its `vos` and nothing supplies one
                // (`AET-WO-4240v` §1.1). An older file that lacks either is
                // refused here; one that has both reaches `validate`, which
                // refuses its version.
                let clock = w
                    .clock
                    .ok_or_else(|| serde::de::Error::missing_field("clock"))?;
                let core_supply = w
                    .core_supply
                    .ok_or_else(|| serde::de::Error::missing_field("core_supply"))?;
                let motor_outputs = match w.motor_outputs {
                    Some(m) => m,
                    None if w.schema_version != SOLVED_SCHEMA_VERSION => BTreeMap::new(),
                    None => return Err(serde::de::Error::missing_field("motor_outputs")),
                };
                // The same for `power`, which version 6 added.
                let power = match w.power {
                    Some(p) => p,
                    None if w.schema_version != SOLVED_SCHEMA_VERSION => None,
                    None => return Err(serde::de::Error::missing_field("power")),
                };
                // The same for `serial_ports`, which version 7 added.
                let serial_ports = match w.serial_ports {
                    Some(m) => m,
                    None if w.schema_version != SOLVED_SCHEMA_VERSION => BTreeMap::new(),
                    None => return Err(serde::de::Error::missing_field("serial_ports")),
                };
                // The same for `spi_controllers`, `i2c_timings` and
                // `bor_level`, which version 8 added; an older file's stand-in
                // is never read.
                let current = w.schema_version == SOLVED_SCHEMA_VERSION;
                let spi_controllers = match w.spi_controllers {
                    Some(m) => m,
                    None if !current => BTreeMap::new(),
                    None => return Err(serde::de::Error::missing_field("spi_controllers")),
                };
                let i2c_timings = match w.i2c_timings {
                    Some(m) => m,
                    None if !current => BTreeMap::new(),
                    None => return Err(serde::de::Error::missing_field("i2c_timings")),
                };
                let bor_level = match w.bor_level {
                    Some(b) => b,
                    None if !current => 0,
                    None => return Err(serde::de::Error::missing_field("bor_level")),
                };
                // The same for `mcu` and `pin_speeds`, which version 9 added.
                let mcu = match w.mcu {
                    Some(m) => m,
                    None if !current => String::new(),
                    None => return Err(serde::de::Error::missing_field("mcu")),
                };
                let pin_speeds = match w.pin_speeds {
                    Some(m) => m,
                    None if !current => BTreeMap::new(),
                    None => return Err(serde::de::Error::missing_field("pin_speeds")),
                };
                // The same for `foreign_clock`, which version 11 added.
                let foreign_clock = match w.foreign_clock {
                    Some(c) => c,
                    None if !current => None,
                    None => return Err(serde::de::Error::missing_field("foreign_clock")),
                };
                // `gpio`'s directional shape and `adc_inputs`, which version
                // 14 added (`AET-WO-1211a`); an older file's stand-in is
                // never read.
                let mut gpio = BTreeMap::new();
                for (net, g) in w.gpio {
                    let g = match g {
                        GpioWire::Solved(g) => g,
                        GpioWire::Pin(pin) if !current => {
                            pin_devices::SolvedGpio::output(pin, motor::Level::High)
                        }
                        GpioWire::Pin(_) => {
                            return Err(serde::de::Error::custom(format!(
                                "gpio.{net} is a bare pin; version {SOLVED_SCHEMA_VERSION} states {{pin, direction, initial | pull}}"
                            )));
                        }
                    };
                    gpio.insert(net, g);
                }
                let adc_inputs = match w.adc_inputs {
                    Some(m) => m,
                    None if !current => BTreeMap::new(),
                    None => return Err(serde::de::Error::missing_field("adc_inputs")),
                };
                Ok(SolvedTopology {
                    schema_version: w.schema_version,
                    specification_sha256: w.specification_sha256,
                    assignments: w.assignments,
                    gpio,
                    adc_inputs,
                    i2c_ceilings: w.i2c_ceilings,
                    i2c_speeds,
                    analog_lines: w.analog_lines,
                    i2c_devices: w.i2c_devices,
                    spi_devices,
                    spi_lines,
                    clock,
                    motor_outputs,
                    power,
                    serial_ports,
                    spi_controllers,
                    i2c_timings,
                    bor_level,
                    core_supply,
                    mcu,
                    pin_speeds,
                    foreign_clock,
                    pin_devices: w.pin_devices,
                })
            }
        }

        deserializer.deserialize_map(ObjectVisitor)
    }
}

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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Pin {
    /// The package pin number.
    pub position: u16,
    /// Signal name to alternate-function number, such as `I2C1_SCL` to 4.
    pub alternate_functions: BTreeMap<String, u8>,
    /// The single-ended ADC inputs the pin carries, such as `ADC1_INP3`
    /// (`AET-WO-4300d2` §1.7). Only the positive input: a single-ended
    /// measurement of a net against ground converts on `INPm`; `INNm` is the
    /// negative leg of a differential pair and no battery sense uses it. An
    /// analog function has no alternate-function number, so it is not in
    /// `alternate_functions`. Not serialized when empty.
    #[serde(skip_serializing_if = "BTreeSet::is_empty")]
    pub adc_channels: BTreeSet<String>,
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
            plain(name, || format!("pins.{name}"))?;
            for signal in pin.alternate_functions.keys() {
                plain(signal, || format!("pins.{name}.{signal}"))?;
            }
            for channel in &pin.adc_channels {
                plain(channel, || format!("pins.{name}.{channel}"))?;
            }
        }
        Ok(())
    }
}

/// A solved `CCR` of the shape its output's idle gives (`AET-WO-1211a`): 0
/// for an idle `low`, `ARR + 1` for `high`, else from 1 to below `ARR` (a
/// pulse idle, and DShot's `0` bit).
fn ccr_shaped(m: &SolvedMotorOutput) -> bool {
    match m.pwm.as_ref().map(|p| p.idle) {
        Some(motor::Idle::Level(motor::Level::Low)) => m.ccr == 0,
        Some(motor::Idle::Level(motor::Level::High)) => m.ccr == m.arr + 1,
        Some(motor::Idle::PulseUs(_)) => m.ccr != 0 && m.ccr <= m.arr,
        None => m.ccr != 0 && m.ccr < m.arr,
    }
}

/// One variant per refusal. Each names what `id()` returns, or nothing.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IrValidationError {
    #[error("schema_version is {version}, not {expected}")]
    UnsupportedSchemaVersion { version: u32, expected: u32 },
    #[error("board.copper_layers is {value}; this engine builds 2 or 4 copper layers")]
    UnsupportedLayerCount { value: u8 },
    #[error("`{path}`: {rule}")]
    LayerTable { path: String, rule: &'static str },
    #[error("`{path}`: {rule}")]
    LayerRole { path: String, rule: &'static str },
    #[error("plane net `{id}` is not `GND` or a declared voltage domain")]
    PlaneNet { id: String },
    #[error("board.layers: {rule}")]
    PlaneLayout { rule: &'static str },
    #[error("`{path}`: {rule}")]
    DielectricTable { path: String, rule: &'static str },
    #[error("board.stackup.finished_thickness_um: {rule}")]
    FinishedThickness { rule: &'static str },
    #[error("`{path}`: {rule}")]
    StackupValue { path: String, rule: &'static str },
    #[error("`{path}` is not a source: 1 to 200 bytes of printable ASCII, no space at either end")]
    InvalidSource { path: String },
    #[error("`{path}` is {value} um, below the engine floor of {floor} um")]
    RuleBelowFloor {
        path: String,
        value: u32,
        floor: u32,
    },
    #[error("`{path}` is {value} um; an outline bound is from 1 to {max} um (`AET-WO-1313`)")]
    OutlineBound { path: String, value: u32, max: u32 },
    #[error("`{path}` is {value} um; this engine builds only {required} um vias")]
    UnsupportedViaRule {
        path: String,
        value: u32,
        required: u32,
    },
    #[error("`{path}` is {value} um, below the minimum of {minimum} um ({of})")]
    ClassBelowMinimum {
        path: String,
        value: u32,
        minimum: u32,
        of: &'static str,
    },
    #[error("`{path}` is {value} um, above the maximum of {maximum} um")]
    ClassAboveMaximum {
        path: String,
        value: u32,
        maximum: u32,
    },
    #[error(
        "net class `{id}`: the via ring is {ring_um} um, under the {minimum_um} um the board rules' own via has ((diameter - drill) / 2)"
    )]
    ViaRingUnderRules {
        id: String,
        ring_um: String,
        minimum_um: String,
    },
    #[error("part `{part}`: thermal_vias `{field}` is zero")]
    ThermalViasZero { part: String, field: &'static str },
    #[error("part `{part}`: thermal_vias `{field}` has no source (empty or only whitespace)")]
    ThermalViasUnsourced { part: String, field: &'static str },
    #[error(
        "part `{part}`: the thermal via ring is {ring_um} um, under the {min_um} um the board rules' own via has ((diameter - drill) / 2, rounded up)"
    )]
    ThermalViaRingUnderRules {
        part: String,
        ring_um: u32,
        min_um: u32,
    },
    #[error(
        "part `{part}`: the thermal via pitch is {pitch_um} um, under the {min_um} um two holes need (drill plus the hole-to-hole gap)"
    )]
    ThermalViaPitch {
        part: String,
        pitch_um: u32,
        min_um: u64,
    },
    #[error("part `{part}`: thermal via layer `{layer}`: {rule}")]
    ThermalViaLayers {
        part: String,
        layer: String,
        rule: &'static str,
    },
    #[error("the name `{id}` is used more than once among net_classes and diff_pairs")]
    DuplicateClassName { id: String },
    #[error("net class `{id}` names no nets")]
    ClassNoNets { id: String },
    #[error(
        "`{class}` names the net `{id}`, which is no declared signal net, voltage domain or `GND`"
    )]
    ClassUnknownNet { id: String, class: String },
    #[error(
        "`{class}` names the net `{id}`, which holds `'` or `?`: the gate's DRC rules name a class's nets in KiCad expressions, where `'` ends the quote and `?` matches any character"
    )]
    ClassNetNotLiteral { id: String, class: String },
    #[error(
        "net `{id}` is in two classes: `{first}` and `{second}` (a diff pair counts as its nets' class)"
    )]
    NetInTwoClasses {
        id: String,
        first: String,
        second: String,
    },
    #[error("diff pair `{id}`: {rule}")]
    PairNets { id: String, rule: &'static str },
    #[error(
        "diff pair `{id}` names `{net}`, a voltage domain or `GND`: a rail, a plane or ground is not a coupled conductor"
    )]
    PairOnRail { id: String, net: String },
    #[error("`{path}` is not an identifier: {rule}")]
    InvalidIdentifier { path: String, rule: &'static str },
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
    #[error(
        "device reference `{id}` appears more than once, in any case (first spelled `{first}`)"
    )]
    DuplicateReference { id: String, first: String },
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
    #[error("analog_lines names `{id}`, which is no solved I2C line")]
    AnalogLineNotI2c { id: String },
    #[error("solved I2C line `{id}` has no entry in analog_lines")]
    MissingAnalogLine { id: String },
    #[error("analog line `{id}` does not share one bus with exactly its own peripheral's lines")]
    AnalogBusMismatch { id: String },
    #[error("i2c_devices indices are not exactly 0..n, each once (device `{id}`)")]
    I2cDeviceIndex { id: String },
    #[error("device `{id}` is on a peripheral other than the one its bus was solved to")]
    I2cDevicePeripheral { id: String },
    #[error("spi_devices indices are not exactly 0..n, each once (device `{id}`)")]
    SpiDeviceIndex { id: String },
    #[error("SPI device `{id}` is not on a solved SPI peripheral")]
    SpiDevicePeripheral { id: String },
    #[error("SPI device `{id}`'s chip select net is not a solved GPIO")]
    SpiDeviceSelect { id: String },
    #[error(
        "device reference `{id}` collides with `{with}`, a name the Renode platform would declare for another device or bus (`AET-WO-4200` D7)"
    )]
    ReferenceCollision { id: String, with: String },
    #[error("mcu.bor_level is {value}, not 0 to 3")]
    BorLevelOutOfRange { value: u8 },
    #[error("power parts table{}: {reason}", part.as_deref().map(|p| format!(", part `{p}`")).unwrap_or_default())]
    PowerPartsTable {
        part: Option<String>,
        reason: String,
    },
    #[error("{}: `{id}`: {detail} ({citation})", kind.name())]
    Power {
        kind: power::Refusal,
        id: String,
        detail: String,
        citation: String,
    },
    #[error("parts table{}: {reason}", part.as_ref().map(|p| format!(" entry `{p}`")).unwrap_or_default())]
    PartsTable {
        part: Option<String>,
        reason: String,
    },
    #[error("output speed table: {reason}")]
    OutputSpeedTable { reason: String },
    #[error(
        "device `{id}` names `{part}`, whose JEDEC identity depends on its ordering code: `W25Q64JVxIQ` answers EF 40 17, as the JQ code does; the IM and JM codes answer EF 70 17 and have no entry"
    )]
    AmbiguousPartName { id: String, part: String },
    #[error(
        "device `{id}` names `{part}`, which the parts table holds as `{table}`: part names are exact"
    )]
    PartNameCase {
        id: String,
        part: String,
        table: String,
    },
    #[error(
        "device `{id}` is a `{part}` on a supply of {min_millivolts} to {max_millivolts} mV, outside the part's {part_min_millivolts} to {part_max_millivolts} mV ({citation})"
    )]
    SupplyOutsidePart {
        id: String,
        part: String,
        min_millivolts: u32,
        max_millivolts: u32,
        part_min_millivolts: u32,
        part_max_millivolts: u32,
        citation: String,
    },
    #[error(
        "device `{id}` is a `{part}` on bus `{bus}`, whose domain runs {min_millivolts} to {max_millivolts} mV, outside the part's I/O range of {part_min_millivolts} to {part_max_millivolts} mV ({citation}); and no crossing declared on that bus puts it on another domain (`AET-WO-4210` D7, `AET-WO-4300d` D9)"
    )]
    IoOutsidePart {
        id: String,
        part: String,
        bus: String,
        min_millivolts: u32,
        max_millivolts: u32,
        part_min_millivolts: u32,
        part_max_millivolts: u32,
        /// Boxed, so the error stays small enough to return by value.
        citation: Box<str>,
    },
    #[error(
        "net constraint on `{id}`, an SPI bus's MISO or a chip select: no check reads it, so it cannot be held (`AET-WO-4300c` D3, D4)"
    )]
    UnjudgedSpiConstraint { id: String },
    #[error(
        "device `{id}` is a `{part}`, which its vendor lists as `{status}` ({citation}), and `accept_lifecycle` does not acknowledge it: name the part, its status and why the board uses it (`AET-WO-4210` D1)"
    )]
    LifecycleNotAccepted {
        id: String,
        part: String,
        status: &'static str,
        citation: String,
    },
    #[error(
        "device `{id}` is a `{part}`, which its vendor lists as `{status}` ({citation}), but `accept_lifecycle` acknowledges it as `{accepted}`"
    )]
    LifecycleStatusMismatch {
        id: String,
        part: String,
        status: &'static str,
        accepted: String,
        /// Boxed, as `IoOutsidePart`'s is.
        citation: Box<str>,
    },
    #[error("`accept_lifecycle` acknowledges `{id}` twice")]
    LifecycleAcceptedTwice { id: String },
    #[error(
        "`accept_lifecycle` acknowledges `{id}` with a statement that is empty, longer than 512 bytes, or not printable ASCII"
    )]
    LifecycleStatementInvalid { id: String },
    #[error(
        "`accept_lifecycle` acknowledges `{id}`, which no device uses as an end-of-life or not-recommended part: an acknowledgement stands only for a part the board uses"
    )]
    LifecycleAcceptanceUnused { id: String },
    #[error(
        "`acknowledge_unwitnessed` names register `{id}`, which is not a peripheral and a register joined by one `.`, each an upper-case letter then upper-case letters, digits or `_` (`SPI1.CFG1`)"
    )]
    UnwitnessedRegisterMalformed { id: String },
    #[error(
        "`acknowledge_unwitnessed` acknowledges register `{id}` as class `{class}`: the class is `ignored-by-model`, `same-source` or `silicon-only` (AET-WO-4230 R1)"
    )]
    UnwitnessedClassUnknown { id: String, class: String },
    #[error(
        "`acknowledge_unwitnessed` acknowledges register `{id}` as `{class}` with no fields: an entry names the exact fields it covers"
    )]
    UnwitnessedFieldsEmpty { id: String, class: String },
    #[error(
        "`acknowledge_unwitnessed` names field `{field}` of register `{id}` as `{class}` twice"
    )]
    UnwitnessedFieldTwice {
        id: String,
        class: String,
        field: String,
    },
    #[error(
        "`acknowledge_unwitnessed` acknowledges register `{id}` as `{class}` twice: one entry per register and class"
    )]
    UnwitnessedAcknowledgedTwice { id: String, class: String },
    #[error(
        "device `{id}` is a `{part}` on an {bus_kind} bus, but the part is an {part_kind} part ({citation})"
    )]
    PartOnWrongBus {
        id: String,
        part: String,
        bus_kind: &'static str,
        part_kind: &'static str,
        citation: String,
    },
    #[error(
        "device `{id}` is a `{part}` at address {address:#04X}, which its straps do not offer: {offered} ({citation})"
    )]
    I2cAddressNotOffered {
        id: String,
        part: String,
        address: u8,
        offered: String,
        citation: String,
    },
    #[error(
        "device `{id}` is a `{part}` on a bus clocked at {speed_hz} Hz, above the part's {max_clock_hz} Hz ({citation})"
    )]
    SpiClockAbovePart {
        id: String,
        part: String,
        speed_hz: u32,
        max_clock_hz: u32,
        citation: String,
    },
    #[error(
        "`spi_lines` and the solved SPI signals disagree at `{id}`: each solved SCK and MOSI has exactly one line, of its bus, with no zero value, and each bus one rate"
    )]
    SpiLineMismatch { id: String },
    #[error(
        "`pin_speeds` and the solved pins disagree at `{id}`: every net of `assignments` and `gpio` has exactly one speed, `0` or `1` (`10` and `11` are never selected)"
    )]
    PinSpeedMismatch { id: String },
    /// `AET-WO-4400b2` F4: a stored speed `solve`'s rule would not give.
    #[error(
        "`pin_speeds.{id}` is `{stored}`, but `solve` gives `{solved}` for that line (`speeds::OutputSpeeds::choose` on its `spi_lines` entry; every other pin `0`)"
    )]
    PinSpeedNotSolved { id: String, stored: u8, solved: u8 },
    /// `AET-WO-4400b2` F4: a solved SPI line no selectable speed serves.
    #[error("net `{id}` on `{pin}` has no output speed: {refusal}")]
    PinSpeedUnserved {
        id: String,
        pin: String,
        refusal: speeds::Refusal,
    },
    #[error(
        "I2C bus `{bus}` has ceiling {ceiling_hz} Hz, but its analog lines derive {}",
        derived_hz.map_or_else(|| "no mode (rise time slower than every mode, or no lines)".to_owned(), |hz| format!("{hz} Hz"))
    )]
    CeilingNotDerived {
        bus: String,
        ceiling_hz: u32,
        derived_hz: Option<u32>,
    },
    #[error("`i2c_speeds` and `i2c_ceilings` do not name the same buses: `{id}` is in only one")]
    SpeedBusMismatch { id: String },
    /// `solve` raises it too, wrapped (`AET-WO-1206` §1.5: one error set).
    #[error(
        "I2C bus `{bus}` runs at {speed_hz} Hz, above its rise-time ceiling of {ceiling_hz} Hz"
    )]
    BusTooFast {
        bus: String,
        speed_hz: u32,
        ceiling_hz: u32,
    },
    /// `solve` raises it too, wrapped (`AET-WO-1206` §1.5: one error set).
    #[error(
        "I2C bus `{bus}` has a pull-up of {pull_up_ohms} ohms, below the {floor_ohms} ohm floor its rail and mode set (UM10204 sink current)"
    )]
    PullUpBelowSinkFloor {
        bus: String,
        pull_up_ohms: u32,
        floor_ohms: u32,
    },
    /// `solve` raises it too, wrapped (`AET-WO-1206` §1.5: one error set).
    #[error(
        "I2C line `{net}` has a capacitance of {capacitance_femtofarads} fF, over its bus's limit of {limit_femtofarads} fF (UM10204)"
    )]
    BusCapacitanceOverLimit {
        net: String,
        capacitance_femtofarads: u64,
        limit_femtofarads: u64,
    },
    /// An HSE declaration or its solved tree refused (`AET-WO-4240a`); its
    /// kind is the inner refusal's.
    #[error("{0}")]
    Hse(clock::HseRefusal),
    #[error(
        "motor output `{id}`: `pwm` must be an object when the protocol is `pwm`, and null when it is DShot"
    )]
    MotorTimingShape { id: String },
    #[error("motor output `{id}` has rate_hz 0")]
    ZeroMotorRate { id: String },
    #[error(
        "motor output `{id}` has a pulse range of {min_us} to {max_us} us: the minimum is at least 1 and below the maximum"
    )]
    PulseRange {
        id: String,
        min_us: u32,
        max_us: u32,
    },
    #[error(
        "motor output `{id}`'s longest pulse, {max_us} us, does not fit inside its period at {rate_hz} Hz"
    )]
    PulseExceedsPeriod {
        id: String,
        max_us: u32,
        rate_hz: u32,
    },
    #[error(
        "net constraint on `{id}`, a motor output: no check reads it, so it cannot be held (the output leaves the board, and its load is off-board)"
    )]
    UnjudgedMotorConstraint { id: String },
    #[error(
        "motor output `{id}` disagrees with the solved topology: its net is one `TIMn_CHk` assignment of a timer with channels, each channel once, indices exactly 0..n, `pwm` present exactly for PWM, and PSC, ARR and CCR within the timer (CCR from 1 to ARR - 1, so a fall never meets the next update)"
    )]
    MotorOutputMismatch { id: String },
    #[error("serial port `{id}` is declared twice (labels are compared with case folded)")]
    DuplicateSerialLabel { id: String },
    #[error(
        "serial port `{id}` names `{peripheral}`: LPUART1 is refused, because its enable bit is in `RCC_APB4ENR`, outside the enable registers the generated firmware writes, and its baud generator is not a USART's; name one of USART1, USART2, USART3, UART4, UART5, USART6, UART7, UART8, or null"
    )]
    SerialPeripheralLpuart { id: String, peripheral: String },
    #[error(
        "serial port `{id}` names peripheral `{peripheral}`, which is none of USART1, USART2, USART3, UART4, UART5, USART6, UART7, UART8 (as written, in upper case), nor null"
    )]
    UnknownSerialPeripheral { id: String, peripheral: String },
    #[error(
        "serial ports `{first}` and `{id}` both name peripheral `{peripheral}`: a USART carries one port"
    )]
    SerialPeripheralTwice {
        id: String,
        first: String,
        peripheral: String,
    },
    #[error("serial port `{id}`: `framing.{field}` is {value}; {rule}")]
    SerialFramingShape {
        id: String,
        field: &'static str,
        value: u32,
        rule: &'static str,
    },
    #[error(
        "serial port `{id}` has role `{role}`: `framing.{field}` must be {required} ({citation})"
    )]
    SerialFramingContradictsRole {
        id: String,
        role: &'static str,
        field: &'static str,
        required: String,
        citation: String,
    },
    #[error(
        "serial port `{id}` has role `{role}` and declares `negotiated`: only an `rc_crsf` port's baud is negotiated (CRSF frame 0x70); declare null"
    )]
    SerialNegotiatedOffCrsf { id: String, role: &'static str },
    #[error(
        "serial port `{id}` declares `negotiated` at {baud}, a cited CRSF baud, not a negotiated one ({citation}); declare null"
    )]
    SerialNegotiatedAtCitedBaud {
        id: String,
        baud: u32,
        citation: &'static str,
    },
    #[error(
        "serial port `{id}` is an `rc_crsf` port and declares a `peer_deviation_ppm` of {declared_ppm}, below {cited_ppm} ppm, the spread of the two CRSF bauds it cites ({citation})"
    )]
    PeerDeviationBelowCited {
        id: String,
        declared_ppm: u32,
        cited_ppm: u32,
        citation: String,
    },
    #[error(
        "serial port `{id}` has role `{role}` and calls its `peer_deviation_ppm` of {declared_ppm} `cited`: only an `rc_crsf` port's figure of {cited_ppm} ppm, the spread of the two CRSF bauds the repository cites, is cited; declare any other figure `declared` (AET-WO-4230 R2)"
    )]
    PeerDeviationNotCited {
        id: String,
        role: &'static str,
        declared_ppm: u32,
        cited_ppm: u32,
    },
    #[error(
        "serial port `{id}` is the console at {baud} baud: ArduPilot opens it at boot at DEFAULT_SERIAL0_BAUD (init_console, AP_SerialManager.cpp:419) but holds SERIAL0_BAUD as that rate divided by 1000 (:172), which map_baudrate reads back as {mapped} (:737-770, Copter-4.7.2); declare a baud b with map_baudrate(b / 1000) = b, such as 115200 (AET-WO-1213)"
    )]
    ConsoleBaudNotRoundTrip { id: String, baud: u32, mapped: u32 },
    #[error(
        "net constraint on `{id}`, a serial port's TX or RX: no check reads it, so it cannot be held (the line leaves the board at a connector, and its load is off-board)"
    )]
    UnjudgedSerialConstraint { id: String },
    #[error(
        "serial port or net `{id}` disagrees with the solved topology: every assignment on a USART or UART is one port's TX or RX on `<peripheral>_TX` or `_RX` of the port's own peripheral, one of USART1 to UART8, each peripheral once, indices exactly 0..n, the framing's shape valid and not contradicting its role, a console's baud one SERIAL0_BAUD maps back to, and the kernel clock, BRR, actual baud and error what `serial` recomputes from the clock"
    )]
    SerialPortMismatch { id: String },
    #[error(
        "SPI controller `{id}` disagrees with the solved topology: one entry per SPI peripheral of `assignments` and no other, its kernel and kernel clock what the solved clock gives it (the reset selects' `pll1_q`/`pclk` on an HSE tree, `AET-WO-4240b` Q3), its declared speed the lowest of its buses', MBR at most 7 and the least whose rate at the kernel's fastest is at or below it, and the achieved rates the nominal and fastest kernel divided by 2^(MBR+1)"
    )]
    SpiControllerMismatch { id: String },
    #[error(
        "I2C controller `{id}` disagrees with the solved topology: one entry per I2C peripheral of `assignments` and no other, its kernel clock what the reset clock gives it, CR1 the bring-up's, and TIMINGR for exactly Standard100k, Fast400k and FastPlus1M, each the one `solve` synthesizes (RM0433 Rev 9 §47.4.9)"
    )]
    I2cTimingMismatch { id: String },
    #[error(
        "timer `{id}` carries outputs that differ in protocol, rate, PSC or ARR: the outputs of one timer share its counter"
    )]
    TimerShared { id: String },
    #[error(
        "the solved clock is not the reset clock (HSI at 64 MHz, every prescaler 1, TIMPRE 0, no crystal), the only one this engine generates firmware for (`AET-WO-4300a` §1.2). A firmware that programmed its clock would also leave the twin's counters at the figure Renode 1.17.0's RCC pushes on each RCC write, and their compare channels at the frequency they were built at (`AET-WO-4400f` (b), `INV-07`)"
    )]
    UnsupportedClock,
    #[error(
        "{clock} runs at {fastest_hz} Hz at its fastest, over its {limit_hz} Hz maximum at VOS {vos} ({cite}; `AET-WO-4240v` §1.3: judged at the source's fastest frequency, never at nominal)"
    )]
    VosLimitExceeded {
        clock: String,
        fastest_hz: u64,
        limit_hz: u64,
        vos: vos::Vos,
        cite: String,
    },
    #[error(
        "FLASH_ACR as the firmware leaves it (at reset, LATENCY 7 and WRHIGHFREQ 11, or an HSE tree's solved flash_acr) is below RM0433 Rev 9 Table 17's row for VOS {vos} at an AXI clock of {hclk_fastest_hz} Hz at its fastest, or sets a reserved bit (`AET-WO-4240v` D-7, `AET-WO-4240b` Q6)"
    )]
    FlashWaitStatesBelowTable { vos: vos::Vos, hclk_fastest_hz: u64 },
    #[error("mcu.foreign_clock.{field} is 0 Hz")]
    ForeignClockZero { field: String },
    #[error(
        "mcu.foreign_clock.usarts must name each USART the platform carries, {expected:?}, once: missing {missing:?}, unknown {unknown:?}"
    )]
    ForeignClockUsarts {
        expected: Vec<String>,
        missing: Vec<String>,
        unknown: Vec<String>,
    },
    #[error(
        "mcu.foreign_clock.usarts declares {first} at {first_hz} Hz and {second} at {second_hz} Hz, but one kernel clock mux selects both (RM0433 Rev 9 §8.7.21: USART16SEL for USART1 and USART6, USART234578SEL for the rest)"
    )]
    ForeignClockMuxSplit {
        first: String,
        first_hz: u64,
        second: String,
        second_hz: u64,
    },
    #[error("connector `{id}` names part `{part}`, which is not in the connectors table")]
    UnknownConnectorPart { id: String, part: String },
    #[error("connector `{id}` lists pin `{pin}`; its part's signal pins are `1` to `{pins}`")]
    ConnectorPinUnknown { id: String, pin: String, pins: u8 },
    #[error("connector `{id}` lists pin `{pin}` more than once")]
    ConnectorPinRepeated { id: String, pin: String },
    #[error(
        "connector `{id}` does not list pin `{pin}`; every signal pin is listed, `null` where it is unconnected"
    )]
    ConnectorPinMissing { id: String, pin: String },
    #[error(
        "connector `{id}` pin `{pin}` names net `{net}`, which is no declared signal, voltage domain or `GND`: a connector cannot invent a net"
    )]
    ConnectorNetUnknown {
        id: String,
        pin: String,
        net: String,
    },
    #[error(
        "test point `{id}` names net `{net}`, which is no declared signal, voltage domain, `GND` or debug net: a test point cannot invent a net (`AET-WO-1312`)"
    )]
    TestPointNetUnknown { id: String, net: String },
    #[error("test point `{id}` takes a reference that {other} already holds (`AET-WO-1312`)")]
    TestPointReferenceTaken { id: String, other: String },
    #[error(
        "net `{id}` carries two test points, `{first}` and `{second}`; one is enough on any net but GND (`AET-WO-1312`, `AET-WO-3402`)"
    )]
    TestPointNetRepeated {
        id: String,
        first: String,
        second: String,
    },
    #[error(
        "no declared connector carries `SWDIO`, `SWCLK`, `GND` and the MCU's supply `{supply}` together, so nothing can program the part: declare one, a DS-009 debug port or an explicit pin map (`AET-WO-4300g`)"
    )]
    NoProgrammingPath { supply: String },
    #[error("the MCU's supply `{supply}` has no source: {why} (`AET-WO-4300g`)")]
    SupplyHasNoSource { supply: String, why: String },
    #[error(
        "the MCU's supply `{supply}` has two declared sources: `mcu.supply_source` names a connector pin, and regulator `{regulator}` outputs it too; a supply has one (`AET-WO-4300g` D3, ruled)"
    )]
    SupplyHasTwoSources { supply: String, regulator: String },
    #[error(
        "connector `{id}` pin `{pin}` is on an I2C or SPI line (a bus line or a chip select) and declares no off-board capacitance (null or 0); ruling D8 requires one on every such pin, so that a bound over the line can count the cable and the remote pins (`RUL-19` (1))"
    )]
    ConnectorBudgetMissing { id: String, pin: String },
    #[error(
        "connector `{id}` pin `{pin}` declares an off-board capacitance, but it is on no I2C or SPI line (a ground, a domain or no net), which carries none"
    )]
    ConnectorBudgetNotApplicable { id: String, pin: String },
    #[error(
        "connector `{id}` takes a reference the emitter numbers its own parts by (`U`, `R` or `C` and a number, in any case): the emitter would renumber its own part or refuse the board; name a connector otherwise, such as `J1`"
    )]
    ConnectorReferenceReserved { id: String },
    #[error(
        "connector `{id}` pin `{pin}` declares {femtofarads} fF off the board, above the {max} fF any line's bound admits"
    )]
    ConnectorBudgetTooLarge {
        id: String,
        pin: String,
        femtofarads: u64,
        max: u64,
    },
    #[error("connector `{id}` names standard `{standard}`, which is not in the ports table")]
    UnknownConnectorStandard { id: String, standard: String },
    #[error(
        "connector `{id}` is part `{part}`, which is not the series and pin count of `{standard}`"
    )]
    ConnectorPartContradictsStandard {
        id: String,
        part: String,
        standard: String,
    },
    #[error(
        "connector `{id}` pin `{pin}` is `{role}` in its standard, and its net {} is not that{}",
        net.as_deref().map_or_else(|| "(null)".to_owned(), |n| format!("`{n}`")),
        required_millivolts.map_or_else(String::new, |mv| format!(": a voltage domain of nominal {mv} mV"))
    )]
    ConnectorSupplyContradictsStandard {
        id: String,
        pin: String,
        role: &'static str,
        net: Option<String>,
        required_millivolts: Option<u32>,
    },
    #[error(
        "connector `{id}` pin `{pin}` is `{role}` in its standard, and its net {} is not that ({rule})",
        net.as_deref().map_or_else(|| "(null)".to_owned(), |n| format!("`{n}`"))
    )]
    ConnectorPinContradictsStandard {
        id: String,
        pin: String,
        role: &'static str,
        net: Option<String>,
        /// What the role takes, in words: [`crate::ds009::Role::rule`].
        rule: &'static str,
    },
    #[error(
        "connector `{id}` pin `{pin}` is on `{standard}`, whose serial pins carry a `{}` serial port, and its net is serial port `{label}`'s, whose role is `{declared}` ({})",
        crate::ds009::port(standard).and_then(|p| p.serial_role).map_or("", SerialRole::name),
        crate::ds009::port(standard).map_or("", |p| p.section)
    )]
    ConnectorSerialRoleContradictsStandard {
        id: String,
        pin: String,
        standard: &'static str,
        label: String,
        declared: &'static str,
    },
    #[error(
        "serial port `{label}`'s {line} net `{net}` is on no connector pin; each serial port's TX and RX end at a declared connector pin (AET-WO-4300b, the architect's ruling 1)"
    )]
    SerialPortNotAtConnector {
        label: String,
        line: &'static str,
        net: String,
    },
    /// Step 19 (`AET-WO-4100` F1, `AET-WO-1206` §1.2): the refusal `solve`
    /// made, by its name, message and claimant.
    #[error(
        "I2C bus `{bus}` has no net constraint on `{net}`, so it has no rise time to bound its speed"
    )]
    MissingBusConstraint { bus: String, net: String },
    /// Step 20 (`AET-WO-1206` A4): a reference the emitter also numbers,
    /// spelled another way; the emitter's `DuplicateReference`, by kind, `id`
    /// (`U1 and u1`) and message.
    #[error("`{0}` is used twice as a reference")]
    EmitterReference(String),
    /// Step 20 (A5): a net or domain the emitter names for itself; the
    /// emitter's `ReservedName`, by kind, `id` and message.
    #[error("`{0}` names a net, rail or reference the emitter uses for itself; rename it")]
    ReservedName(String),
    /// Step 20 (A2): a device named for a peripheral instance of the MCU; the
    /// twin's `InvalidDeviceName`, by kind and message, with no `id` as the
    /// twin prints none.
    #[error(
        "device `{reference}` cannot be a Renode name (a lower-case letter, then letters, digits or `_`) that no stock instance already has"
    )]
    InvalidDeviceName { reference: String },
    /// Step 20a (`AET-WO-1209` §1.2): a load naming no part the board has, so
    /// its current would count against the rail for nothing (`4202`'s finding).
    #[error(
        "{path} on `{domain}` names `{reference}`, which is no part the board has: not one the specification declares, the MCU, the reset switch, nor a resistor or capacitor the emitter numbers"
    )]
    LoadNamesNoPart {
        path: String,
        domain: String,
        reference: String,
    },
    /// Step 20a (`AET-WO-1209c`): a load naming the MCU or a declared
    /// regulator, whose current a row of its own already counts
    /// (`power.rs`'s tree; [`power::Load::reference`]): on a regulator's
    /// input rail it would count twice, and on the rail it makes it is that
    /// rail's source, not a load. `counted_by` names the row.
    #[error(
        "{path} on `{domain}` names `{reference}`, whose current is already counted by {counted_by}"
    )]
    LoadNamesCountedPart {
        path: String,
        domain: String,
        reference: String,
        counted_by: String,
    },
    /// Step 14 (`AET-WO-1206` §1.8): a device whose part has no row in the
    /// parts table, so no address, supply or clock limit could be held; the
    /// emitter's `UnknownPart` by kind and `id`. The message names the set
    /// the refusal means, the table's rows (`AET-WO-1316`), where the
    /// emitter's names the parts it has an arm for.
    #[error(
        "`{reference}` is a `{part}`, which has no row in the parts table; known: {}",
        table_parts()
    )]
    PartWithoutRow { reference: String, part: String },
    /// Step 23 (`AET-WO-1210`, `AET-WO-1209` §1.1 rule 1): on a drawn board,
    /// a device on an SPI line `extract` bounds whose pin capacitance has no
    /// bound at all; `extract`'s `UnboundedPinCapacitance`, by kind, `id`,
    /// line and words, before the board is drawn.
    #[error(
        "the design puts {reference} ({part}) on {line}, a line the MCU drives, and the parts table cites no maximum for its pin capacitance, nor does `{path}` declare one"
    )]
    UnboundedPinCapacitance {
        path: String,
        reference: String,
        part: String,
        /// The bus's SCK net and the bus, as "`IMU_SCK` of SPI bus `IMU`".
        line: String,
    },
    /// `AET-WO-1210`, `AET-WO-1209` §1.3 rule 3, held by the CLI's
    /// `validate` with the emitter's set ([`Specification::drawable_parts`]):
    /// on a drawn board, a device whose part has a row and no emitter arm, so
    /// `emit` would refuse it (`UnknownPart`).
    #[error("`{reference}` is a `{part}`: row present, no emitter arm; the emitter draws: {arms}")]
    PartNotDrawable {
        path: String,
        reference: String,
        part: String,
        arms: String,
    },
    /// `AET-WO-4400d`: an orientation is in the board frame, whose x points
    /// to the declared forward edge.
    #[error(
        "`{reference}` declares an orientation, but `board.forward` is not declared: the board frame's x points to that edge"
    )]
    ForwardUndeclared { reference: String },
    /// `AET-WO-4400d`: the declared X and Z are the same line, so no rotation
    /// is named; nothing is rounded.
    #[error(
        "`{reference}`'s orientation puts X toward `{x}` and Z toward `{z}`, which are not at right angles"
    )]
    ImuAxesNotPerpendicular {
        reference: String,
        x: String,
        z: String,
    },
    /// `AET-WO-4400d`: only a part whose `ardupilot.kind` is `imu` takes an
    /// orientation.
    #[error("`{reference}` is a `{part}`, which is not an IMU, and declares an orientation")]
    OrientationOnNonImu { reference: String, part: String },
    /// `AET-WO-4400d`: the part's axes and its driver's frame are cited
    /// facts; an absent one is refused, never assumed.
    #[error("`{reference}` declares an orientation, but the `{part}` row cites no `{missing}`")]
    ImuAxesNotCited {
        reference: String,
        part: String,
        missing: &'static str,
    },
    /// Step 21 (`AET-WO-1207` §1.2, `1206` F3): the refusal `solve` made, by
    /// its name, message and claimant.
    #[error(
        "I2C bus `{bus}` allows a rise time of {rise_time_ns} ns, slower than any I2C mode (1000 ns at most)"
    )]
    RiseTimeTooSlow { bus: String, rise_time_ns: u32 },
    /// Step 21: as [`IrValidationError::RiseTimeTooSlow`].
    #[error(
        "SPI bus `{bus}` is on a supply of {min_millivolts} to {max_millivolts} mV, outside DS12117 Table 149's 1620 to 3600 mV, so no output speed can be judged for it"
    )]
    SupplyOutsideSpeedTable {
        bus: String,
        min_millivolts: u32,
        max_millivolts: u32,
    },
    /// Step 21: as [`IrValidationError::RiseTimeTooSlow`].
    #[error(
        "SPI bus `{bus}` has no net constraint on `{net}`, so its pin's output speed has no declared load to be chosen for (`AET-WO-4300c` D2)"
    )]
    MissingSpiConstraint { bus: String, net: String },
    /// Step 14 (`AET-WO-1207` §1.5, Grok 2026-11-16 row 5): a device whose
    /// part's row lists SPI modes without the one the generated firmware
    /// drives, [`parts::DRIVEN_SPI_MODE`].
    #[error(
        "device `{id}` is a `{part}`, which accepts SPI modes {modes:?} ({citation}), not mode {driven}, the one the generated firmware drives (CFG2 CPOL 0, CPHA 0)"
    )]
    SpiModeNotDriven {
        id: String,
        part: String,
        modes: Vec<u8>,
        driven: u8,
        citation: String,
    },
    /// Step 14 (`AET-WO-4220b` §1.2): a declared pin capacitance outside the
    /// parts table's own range ([`parts::MAX_PIN_FEMTOFARADS`]): zero would
    /// drop the device's pins from every bound they sit on, and above it is
    /// no signal pin.
    #[error("device `{id}` declares a pin capacitance of {femtofarads} fF, not 1 to {maximum} fF")]
    PinCapacitanceOutOfRange {
        id: String,
        femtofarads: u64,
        maximum: u64,
    },
    /// Step 14 (`AET-WO-4220b` §1.2, adversarial A1): a declared pin
    /// capacitance on an I2C device below UM10204's `Ci`, which every
    /// undeclared I2C pin is counted at: a declaration may name a condition
    /// or raise a pin's term, never lower it below the standard's maximum.
    #[error(
        "I2C device `{id}` declares a pin capacitance of {femtofarads} fF, below UM10204's Ci of {ci} fF (rev. 7.0 Table 10, p. 43), which every undeclared I2C pin is counted at"
    )]
    PinCapacitanceBelowI2cCi {
        id: String,
        femtofarads: u64,
        ci: u64,
    },
    /// Step 14 (`AET-WO-4220b` §1.2): a declared pin capacitance for a part
    /// whose row cites a maximum, which a declaration never replaces
    /// (`AET-WO-4300c` D3).
    #[error(
        "device `{id}` declares a pin capacitance, but its part `{part}` cites a maximum of {maximum_femtofarads} fF ({citation}), which a declaration never replaces"
    )]
    PinCapacitanceAlreadyCited {
        id: String,
        part: String,
        maximum_femtofarads: u64,
        citation: String,
    },
    /// Step 14 (`AET-WO-4220b` H2, ruled on #192 Q2): a declared pin
    /// capacitance below the typical its part's row cites, on any bus.
    #[error(
        "device `{id}` declares a pin capacitance of {femtofarads} fF, below the typical of {typical_femtofarads} fF its part `{part}` cites ({citation})"
    )]
    PinCapacitanceBelowTypical {
        id: String,
        part: String,
        femtofarads: u64,
        typical_femtofarads: u64,
        citation: String,
    },
    /// Step 21 (`AET-WO-1208` §1.3, `1207` F1): the refusal `solve` made, by
    /// its name, message and claimant. Every timer's kernel clock is one
    /// frequency on the reset clock, so the timing is the specification's.
    #[error("motor output `{output}`: {reason}")]
    MotorTiming {
        output: String,
        reason: &'static str,
    },
    /// Step 24 (`AET-WO-1211a`): a pin device; the kind is the inner
    /// refusal's.
    #[error("{0}")]
    PinDevice(pin_devices::PinDeviceRefusal),
    /// Step 13b and step 21 (`AET-WO-1211a`): a PWM output's idle or tick;
    /// the kind is the inner refusal's.
    #[error("{0}")]
    Pwm(motor::PwmRefusal),
    /// Step 21: as [`IrValidationError::MotorTiming`]. Every USART and UART
    /// has one kernel clock on the reset clock.
    #[error(
        "serial port `{port}`: {baud} baud needs a BRR outside 16 to 0xFFFF on a {kernel_hz} Hz kernel clock (RM0433 Rev 9 §48.8.5, oversampling by 16)"
    )]
    SerialBaudOutOfRange {
        port: String,
        baud: u32,
        kernel_hz: u64,
    },
    /// Step 21: as [`IrValidationError::SerialBaudOutOfRange`].
    #[error(
        "serial port `{port}`: the worst-case baud error, {worst_case_ppm} ppm (the receiver's {receiver_ppm}: quantization {quantization_ppm} with HSI at {hsi_ppm}; plus the peer's declared {peer_deviation_ppm}), is not below the receiver's tolerance of {tolerance_ppm} ppm ({row})"
    )]
    SerialBaudTolerance {
        port: String,
        row: String,
        quantization_ppm: u64,
        hsi_ppm: u32,
        receiver_ppm: u64,
        peer_deviation_ppm: u32,
        worst_case_ppm: u64,
        tolerance_ppm: u32,
    },
}

impl IrValidationError {
    /// The variant name, as callers see it in the CLI's error envelope.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion { .. } => "UnsupportedSchemaVersion",
            Self::UnsupportedLayerCount { .. } => "UnsupportedLayerCount",
            Self::LayerTable { .. } => "LayerTable",
            Self::LayerRole { .. } => "LayerRole",
            Self::PlaneNet { .. } => "PlaneNet",
            Self::PlaneLayout { .. } => "PlaneLayout",
            Self::DielectricTable { .. } => "DielectricTable",
            Self::FinishedThickness { .. } => "FinishedThickness",
            Self::StackupValue { .. } => "StackupValue",
            Self::InvalidSource { .. } => "InvalidSource",
            Self::RuleBelowFloor { .. } => "RuleBelowFloor",
            Self::UnsupportedViaRule { .. } => "UnsupportedViaRule",
            Self::OutlineBound { .. } => "OutlineBound",
            Self::ClassBelowMinimum { .. } => "ClassBelowMinimum",
            Self::ClassAboveMaximum { .. } => "ClassAboveMaximum",
            Self::ViaRingUnderRules { .. } => "ViaRingUnderRules",
            Self::ThermalViasZero { .. } => "ThermalViasZero",
            Self::ThermalViasUnsourced { .. } => "ThermalViasUnsourced",
            Self::ThermalViaRingUnderRules { .. } => "ThermalViaRingUnderRules",
            Self::ThermalViaPitch { .. } => "ThermalViaPitch",
            Self::ThermalViaLayers { .. } => "ThermalViaLayers",
            Self::DuplicateClassName { .. } => "DuplicateClassName",
            Self::ClassNoNets { .. } => "ClassNoNets",
            Self::ClassUnknownNet { .. } => "ClassUnknownNet",
            Self::ClassNetNotLiteral { .. } => "ClassNetNotLiteral",
            Self::NetInTwoClasses { .. } => "NetInTwoClasses",
            Self::PairNets { .. } => "PairNets",
            Self::PairOnRail { .. } => "PairOnRail",
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
            Self::AnalogLineNotI2c { .. } => "AnalogLineNotI2c",
            Self::MissingAnalogLine { .. } => "MissingAnalogLine",
            Self::AnalogBusMismatch { .. } => "AnalogBusMismatch",
            Self::I2cDeviceIndex { .. } => "I2cDeviceIndex",
            Self::I2cDevicePeripheral { .. } => "I2cDevicePeripheral",
            Self::SpiDeviceIndex { .. } => "SpiDeviceIndex",
            Self::SpiDevicePeripheral { .. } => "SpiDevicePeripheral",
            Self::SpiDeviceSelect { .. } => "SpiDeviceSelect",
            Self::ReferenceCollision { .. } => "ReferenceCollision",
            Self::PartsTable { .. } => "PartsTableInvalid",
            Self::BorLevelOutOfRange { .. } => "BorLevelOutOfRange",
            Self::PowerPartsTable { .. } => "PowerPartsTableInvalid",
            Self::Power { kind, .. } => kind.name(),
            Self::OutputSpeedTable { .. } => "OutputSpeedTableInvalid",
            Self::AmbiguousPartName { .. } => "AmbiguousPartName",
            Self::PartNameCase { .. } => "PartNameCase",
            Self::SupplyOutsidePart { .. } => "SupplyOutsidePart",
            Self::IoOutsidePart { .. } => "IoOutsidePart",
            Self::LifecycleNotAccepted { .. } => "LifecycleNotAccepted",
            Self::LifecycleStatusMismatch { .. } => "LifecycleStatusMismatch",
            Self::LifecycleAcceptedTwice { .. } => "LifecycleAcceptedTwice",
            Self::LifecycleStatementInvalid { .. } => "LifecycleStatementInvalid",
            Self::LifecycleAcceptanceUnused { .. } => "LifecycleAcceptanceUnused",
            Self::UnwitnessedRegisterMalformed { .. } => "UnwitnessedRegisterMalformed",
            Self::UnwitnessedClassUnknown { .. } => "UnwitnessedClassUnknown",
            Self::UnwitnessedFieldsEmpty { .. } => "UnwitnessedFieldsEmpty",
            Self::UnwitnessedFieldTwice { .. } => "UnwitnessedFieldTwice",
            Self::UnwitnessedAcknowledgedTwice { .. } => "UnwitnessedAcknowledgedTwice",
            Self::UnjudgedSpiConstraint { .. } => "UnjudgedSpiConstraint",
            Self::PartOnWrongBus { .. } => "PartOnWrongBus",
            Self::I2cAddressNotOffered { .. } => "I2cAddressNotOffered",
            Self::SpiClockAbovePart { .. } => "SpiClockAbovePart",
            Self::SpiLineMismatch { .. } => "SpiLineMismatch",
            Self::PinSpeedMismatch { .. } => "PinSpeedMismatch",
            Self::PinSpeedNotSolved { .. } => "PinSpeedNotSolved",
            Self::PinSpeedUnserved { refusal, .. } => match refusal {
                speeds::Refusal::SupplyOutsideTable { .. } => "SupplyOutsideSpeedTable",
                speeds::Refusal::LoadAboveTable { .. } => "LoadAboveSpeedTable",
                speeds::Refusal::TooFastForPin { .. } => "RateAbovePinSpeed",
                speeds::Refusal::RiseFasterThanPin { .. } => "RiseTimeBelowPinSpeed",
            },
            Self::CeilingNotDerived { .. } => "CeilingNotDerived",
            Self::SpeedBusMismatch { .. } => "SpeedBusMismatch",
            Self::BusTooFast { .. } => "BusTooFast",
            Self::PullUpBelowSinkFloor { .. } => "PullUpBelowSinkFloor",
            Self::BusCapacitanceOverLimit { .. } => "BusCapacitanceOverLimit",
            Self::Hse(r) => r.kind(),
            Self::MotorTimingShape { .. } => "MotorTimingShape",
            Self::ZeroMotorRate { .. } => "ZeroMotorRate",
            Self::PulseRange { .. } => "PulseRange",
            Self::PulseExceedsPeriod { .. } => "PulseExceedsPeriod",
            Self::UnjudgedMotorConstraint { .. } => "UnjudgedMotorConstraint",
            Self::MotorOutputMismatch { .. } => "MotorOutputMismatch",
            Self::DuplicateSerialLabel { .. } => "DuplicateSerialLabel",
            Self::SerialPeripheralLpuart { .. } => "SerialPeripheralLpuart",
            Self::UnknownSerialPeripheral { .. } => "UnknownSerialPeripheral",
            Self::SerialPeripheralTwice { .. } => "SerialPeripheralTwice",
            Self::SerialFramingShape { .. } => "SerialFramingShape",
            Self::SerialFramingContradictsRole { .. } => "SerialFramingContradictsRole",
            Self::SerialNegotiatedOffCrsf { .. } => "SerialNegotiatedOffCrsf",
            Self::SerialNegotiatedAtCitedBaud { .. } => "SerialNegotiatedAtCitedBaud",
            Self::PeerDeviationBelowCited { .. } => "PeerDeviationBelowCited",
            Self::PeerDeviationNotCited { .. } => "PeerDeviationNotCited",
            Self::ConsoleBaudNotRoundTrip { .. } => "ConsoleBaudNotRoundTrip",
            Self::UnjudgedSerialConstraint { .. } => "UnjudgedSerialConstraint",
            Self::SerialPortMismatch { .. } => "SerialPortMismatch",
            Self::SpiControllerMismatch { .. } => "SpiControllerMismatch",
            Self::I2cTimingMismatch { .. } => "I2cTimingMismatch",
            Self::TimerShared { .. } => "TimerShared",
            Self::UnsupportedClock => "UnsupportedClock",
            Self::VosLimitExceeded { .. } => "VosLimitExceeded",
            Self::FlashWaitStatesBelowTable { .. } => "FlashWaitStatesBelowTable",
            Self::ForeignClockZero { .. } => "ForeignClockZero",
            Self::ForeignClockUsarts { .. } => "ForeignClockUsarts",
            Self::ForeignClockMuxSplit { .. } => "ForeignClockMuxSplit",
            Self::UnknownConnectorPart { .. } => "UnknownConnectorPart",
            Self::ConnectorPinUnknown { .. } => "ConnectorPinUnknown",
            Self::ConnectorPinRepeated { .. } => "ConnectorPinRepeated",
            Self::ConnectorPinMissing { .. } => "ConnectorPinMissing",
            Self::ConnectorNetUnknown { .. } => "ConnectorNetUnknown",
            Self::TestPointNetUnknown { .. } => "TestPointNetUnknown",
            Self::TestPointReferenceTaken { .. } => "TestPointReferenceTaken",
            Self::TestPointNetRepeated { .. } => "TestPointNetRepeated",
            Self::NoProgrammingPath { .. } => "NoProgrammingPath",
            Self::SupplyHasNoSource { .. } => "SupplyHasNoSource",
            Self::SupplyHasTwoSources { .. } => "SupplyHasTwoSources",
            Self::ConnectorBudgetMissing { .. } => "ConnectorBudgetMissing",
            Self::ConnectorBudgetNotApplicable { .. } => "ConnectorBudgetNotApplicable",
            Self::ConnectorReferenceReserved { .. } => "ConnectorReferenceReserved",
            Self::ConnectorBudgetTooLarge { .. } => "ConnectorBudgetTooLarge",
            Self::UnknownConnectorStandard { .. } => "UnknownConnectorStandard",
            Self::ConnectorPartContradictsStandard { .. } => "ConnectorPartContradictsStandard",
            Self::ConnectorSupplyContradictsStandard { .. } => "ConnectorSupplyContradictsStandard",
            Self::ConnectorPinContradictsStandard { .. } => "ConnectorPinContradictsStandard",
            Self::ConnectorSerialRoleContradictsStandard { .. } => {
                "ConnectorSerialRoleContradictsStandard"
            }
            Self::SerialPortNotAtConnector { .. } => "SerialPortNotAtConnector",
            Self::MissingBusConstraint { .. } => "MissingBusConstraint",
            Self::EmitterReference(_) => "DuplicateReference",
            Self::ReservedName(_) => "ReservedName",
            Self::InvalidDeviceName { .. } => "InvalidDeviceName",
            Self::LoadNamesNoPart { .. } => "LoadNamesNoPart",
            Self::LoadNamesCountedPart { .. } => "LoadNamesCountedPart",
            Self::PartWithoutRow { .. } => "UnknownPart",
            Self::UnboundedPinCapacitance { .. } => "UnboundedPinCapacitance",
            Self::PartNotDrawable { .. } => "PartNotDrawable",
            Self::ForwardUndeclared { .. } => "ForwardUndeclared",
            Self::ImuAxesNotPerpendicular { .. } => "ImuAxesNotPerpendicular",
            Self::OrientationOnNonImu { .. } => "OrientationOnNonImu",
            Self::ImuAxesNotCited { .. } => "ImuAxesNotCited",
            Self::RiseTimeTooSlow { .. } => "RiseTimeTooSlow",
            Self::SupplyOutsideSpeedTable { .. } => "SupplyOutsideSpeedTable",
            Self::MissingSpiConstraint { .. } => "MissingSpiConstraint",
            Self::SpiModeNotDriven { .. } => "SpiModeNotDriven",
            Self::PinCapacitanceOutOfRange { .. } => "PinCapacitanceOutOfRange",
            Self::PinCapacitanceAlreadyCited { .. } => "PinCapacitanceAlreadyCited",
            Self::PinCapacitanceBelowI2cCi { .. } => "PinCapacitanceBelowI2cCi",
            Self::PinCapacitanceBelowTypical { .. } => "PinCapacitanceBelowTypical",
            Self::MotorTiming { .. } => "MotorTiming",
            Self::Pwm(r) => r.kind(),
            Self::PinDevice(r) => r.kind(),
            Self::SerialBaudOutOfRange { .. } => "SerialBaudOutOfRange",
            Self::SerialBaudTolerance { .. } => "SerialBaudTolerance",
        }
    }

    /// Who is in conflict, as `solve`'s refusal names them: the bus of the
    /// seven bus refusals `solve` made (`AET-WO-1206` §1.5, `AET-WO-1207`
    /// §1.2), the line of `BusCapacitanceOverLimit`, the output or port of
    /// the motor and serial refusals (`AET-WO-1208` §1.3), and nothing for
    /// every other refusal.
    pub fn claimants(&self) -> Vec<String> {
        match self {
            Self::MissingBusConstraint { bus, .. }
            | Self::BusTooFast { bus, .. }
            | Self::PullUpBelowSinkFloor { bus, .. }
            | Self::RiseTimeTooSlow { bus, .. }
            | Self::SupplyOutsideSpeedTable { bus, .. }
            | Self::MissingSpiConstraint { bus, .. } => vec![bus.clone()],
            Self::BusCapacitanceOverLimit { net, .. } => vec![net.clone()],
            Self::MotorTiming { output, .. } => vec![output.clone()],
            Self::Pwm(r) => vec![r.output().to_owned()],
            Self::SerialBaudOutOfRange { port, .. } | Self::SerialBaudTolerance { port, .. } => {
                vec![port.clone()]
            }
            _ => Vec::new(),
        }
    }

    /// The offending name, reference, net or pin. `None` for the refusals that
    /// name no single thing: a version, a path (see the variant), a hash, an
    /// empty list. Nothing is trimmed.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::UnsupportedSchemaVersion { .. }
            | Self::UnsupportedLayerCount { .. }
            | Self::LayerTable { .. }
            | Self::LayerRole { .. }
            | Self::PlaneLayout { .. }
            | Self::DielectricTable { .. }
            | Self::FinishedThickness { .. }
            | Self::StackupValue { .. }
            | Self::InvalidSource { .. }
            | Self::RuleBelowFloor { .. }
            | Self::UnsupportedViaRule { .. }
            | Self::OutlineBound { .. }
            | Self::ClassBelowMinimum { .. }
            | Self::ClassAboveMaximum { .. }
            | Self::InvalidIdentifier { .. }
            | Self::NoVoltageDomain
            | Self::OutputSpeedTable { .. }
            | Self::MalformedSpecificationHash => None,
            Self::PlaneNet { id } => Some(id),
            Self::ThermalViasZero { part, .. }
            | Self::ThermalViasUnsourced { part, .. }
            | Self::ThermalViaRingUnderRules { part, .. }
            | Self::ThermalViaPitch { part, .. }
            | Self::ThermalViaLayers { part, .. } => Some(part),
            Self::ViaRingUnderRules { id, .. }
            | Self::DuplicateClassName { id }
            | Self::ClassNoNets { id }
            | Self::ClassUnknownNet { id, .. }
            | Self::ClassNetNotLiteral { id, .. }
            | Self::NetInTwoClasses { id, .. }
            | Self::PairNets { id, .. }
            | Self::PairOnRail { id, .. } => Some(id),
            Self::DuplicateDomainId { id }
            | Self::ZeroNominalVoltage { id }
            | Self::ToleranceOutOfRange { id, .. }
            | Self::UnknownDomain { id }
            | Self::SupplyMismatch { id }
            | Self::DuplicateBusId { id }
            | Self::UnsupportedI2cSpeed { id, .. }
            | Self::ZeroPullUp { id }
            | Self::ZeroSpiSpeed { id }
            | Self::DuplicateReference { id, .. }
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
            | Self::DuplicatePin { id }
            | Self::AnalogLineNotI2c { id }
            | Self::MissingAnalogLine { id }
            | Self::AnalogBusMismatch { id }
            | Self::I2cDeviceIndex { id }
            | Self::I2cDevicePeripheral { id }
            | Self::SpiDeviceIndex { id }
            | Self::SpiDevicePeripheral { id }
            | Self::SpiDeviceSelect { id }
            | Self::ReferenceCollision { id, .. }
            | Self::AmbiguousPartName { id, .. }
            | Self::PartNameCase { id, .. }
            | Self::SupplyOutsidePart { id, .. }
            | Self::IoOutsidePart { id, .. }
            | Self::LifecycleNotAccepted { id, .. }
            | Self::LifecycleStatusMismatch { id, .. }
            | Self::LifecycleAcceptedTwice { id }
            | Self::LifecycleStatementInvalid { id }
            | Self::LifecycleAcceptanceUnused { id }
            | Self::UnwitnessedRegisterMalformed { id }
            | Self::UnwitnessedClassUnknown { id, .. }
            | Self::UnwitnessedFieldsEmpty { id, .. }
            | Self::UnwitnessedFieldTwice { id, .. }
            | Self::UnwitnessedAcknowledgedTwice { id, .. }
            | Self::UnjudgedSpiConstraint { id }
            | Self::PartOnWrongBus { id, .. }
            | Self::I2cAddressNotOffered { id, .. }
            | Self::SpiClockAbovePart { id, .. }
            | Self::SpiLineMismatch { id }
            | Self::PinSpeedMismatch { id }
            | Self::PinSpeedNotSolved { id, .. }
            | Self::PinSpeedUnserved { id, .. }
            | Self::SpeedBusMismatch { id }
            | Self::MotorTimingShape { id }
            | Self::ZeroMotorRate { id }
            | Self::PulseRange { id, .. }
            | Self::PulseExceedsPeriod { id, .. }
            | Self::UnjudgedMotorConstraint { id }
            | Self::MotorOutputMismatch { id }
            | Self::TimerShared { id }
            | Self::DuplicateSerialLabel { id }
            | Self::SerialPeripheralLpuart { id, .. }
            | Self::UnknownSerialPeripheral { id, .. }
            | Self::SerialPeripheralTwice { id, .. }
            | Self::SerialFramingShape { id, .. }
            | Self::SerialFramingContradictsRole { id, .. }
            | Self::SerialNegotiatedOffCrsf { id, .. }
            | Self::SerialNegotiatedAtCitedBaud { id, .. }
            | Self::PeerDeviationBelowCited { id, .. }
            | Self::PeerDeviationNotCited { id, .. }
            | Self::ConsoleBaudNotRoundTrip { id, .. }
            | Self::UnjudgedSerialConstraint { id }
            | Self::SerialPortMismatch { id }
            | Self::SpiControllerMismatch { id }
            | Self::I2cTimingMismatch { id } => Some(id),
            Self::Hse(_) | Self::UnsupportedClock | Self::FlashWaitStatesBelowTable { .. } => None,
            Self::VosLimitExceeded { clock, .. } => Some(clock),
            Self::ForeignClockZero { .. }
            | Self::ForeignClockUsarts { .. }
            | Self::ForeignClockMuxSplit { .. } => None,
            Self::PartsTable { part, .. } | Self::PowerPartsTable { part, .. } => part.as_deref(),
            Self::Power { id, .. } => Some(id),
            Self::BorLevelOutOfRange { .. } => None,
            Self::CeilingNotDerived { bus, .. }
            | Self::BusTooFast { bus, .. }
            | Self::PullUpBelowSinkFloor { bus, .. } => Some(bus),
            Self::BusCapacitanceOverLimit { net, .. } => Some(net),
            Self::UnknownConnectorPart { id, .. }
            | Self::ConnectorPinUnknown { id, .. }
            | Self::ConnectorPinRepeated { id, .. }
            | Self::ConnectorPinMissing { id, .. }
            | Self::ConnectorNetUnknown { id, .. }
            | Self::TestPointNetUnknown { id, .. }
            | Self::TestPointReferenceTaken { id, .. }
            | Self::TestPointNetRepeated { id, .. }
            | Self::ConnectorBudgetMissing { id, .. }
            | Self::ConnectorBudgetNotApplicable { id, .. }
            | Self::ConnectorReferenceReserved { id }
            | Self::ConnectorBudgetTooLarge { id, .. }
            | Self::UnknownConnectorStandard { id, .. }
            | Self::ConnectorPartContradictsStandard { id, .. }
            | Self::ConnectorSupplyContradictsStandard { id, .. }
            | Self::ConnectorPinContradictsStandard { id, .. }
            | Self::ConnectorSerialRoleContradictsStandard { id, .. } => Some(id),
            Self::SerialPortNotAtConnector { label, .. } => Some(label),
            Self::NoProgrammingPath { supply }
            | Self::SupplyHasNoSource { supply, .. }
            | Self::SupplyHasTwoSources { supply, .. } => Some(supply),
            Self::MissingBusConstraint { bus, .. }
            | Self::RiseTimeTooSlow { bus, .. }
            | Self::SupplyOutsideSpeedTable { bus, .. }
            | Self::MissingSpiConstraint { bus, .. } => Some(bus),
            Self::SpiModeNotDriven { id, .. }
            | Self::PinCapacitanceOutOfRange { id, .. }
            | Self::PinCapacitanceAlreadyCited { id, .. }
            | Self::PinCapacitanceBelowI2cCi { id, .. }
            | Self::PinCapacitanceBelowTypical { id, .. } => Some(id),
            Self::MotorTiming { output, .. } => Some(output),
            Self::Pwm(r) => Some(r.output()),
            Self::PinDevice(r) => Some(&r.id),
            Self::SerialBaudOutOfRange { port, .. } | Self::SerialBaudTolerance { port, .. } => {
                Some(port)
            }
            Self::EmitterReference(id) | Self::ReservedName(id) => Some(id),
            Self::InvalidDeviceName { .. } => None,
            Self::LoadNamesNoPart { reference, .. }
            | Self::LoadNamesCountedPart { reference, .. } => Some(reference),
            Self::PartWithoutRow { reference, .. }
            | Self::UnboundedPinCapacitance { reference, .. }
            | Self::PartNotDrawable { reference, .. }
            | Self::ForwardUndeclared { reference }
            | Self::ImuAxesNotPerpendicular { reference, .. }
            | Self::OrientationOnNonImu { reference, .. }
            | Self::ImuAxesNotCited { reference, .. } => Some(reference),
        }
    }

    /// The JSON path of the field `InvalidIdentifier` refused, such as
    /// `i2c_buses[0].sda.net`.
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::PinDevice(r) => Some(&r.path),
            Self::InvalidIdentifier { path, .. }
            | Self::LayerTable { path, .. }
            | Self::LayerRole { path, .. }
            | Self::DielectricTable { path, .. }
            | Self::StackupValue { path, .. }
            | Self::InvalidSource { path }
            | Self::RuleBelowFloor { path, .. }
            | Self::UnsupportedViaRule { path, .. }
            | Self::OutlineBound { path, .. }
            | Self::ClassBelowMinimum { path, .. }
            | Self::ClassAboveMaximum { path, .. }
            | Self::LoadNamesNoPart { path, .. }
            | Self::LoadNamesCountedPart { path, .. }
            | Self::UnboundedPinCapacitance { path, .. }
            | Self::PartNotDrawable { path, .. } => Some(path),
            _ => None,
        }
    }
}

/// What an identifier names, which decides the shape on top of the base set
/// (`RUL-14`, `AET-WO-1102`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierKind {
    /// A voltage domain id, net, bus id, pin, peripheral, signal, part, or
    /// scenario name: the base set alone.
    Plain,
    /// `project_name`: also a Cargo crate name, which `generate` requires
    /// (`InvalidCrateName`).
    Project,
    /// A device reference, I2C or SPI: also a Renode identifier, which the
    /// platform requires (`InvalidDeviceName`).
    Reference,
}

/// The base set, for every kind: 1 to 64 bytes of printable ASCII
/// `0x21`-`0x7E`, so two identifiers that look alike are the same bytes.
/// Refused beyond that, each measured: `/`, `{`, `}`, `$`, `[` and `]`, which
/// KiCad reads as more than text (`AET-WO-1300` D6); `"`, which KiCad does not
/// escape in a placement row (`AET-WO-3300` F15); `%`, `*`, `,` and `\`, which
/// KiCad rewrites as `\u0025`, `\u002A`, `\u002C` and `\u005C` in a Gerber net
/// attribute (measured in CI, `AET-WO-1102` FEEDBACK §7); and a date shape `dddd-dd-dd`
/// or a time shape `dd:dd:dd` anywhere inside, which KiCad's clock-line check
/// refuses at export (`AET-WO-3300` F14).
pub const REFUSED_IN_IDENTIFIERS: [u8; 11] = *b"/{}$[]\"%*,\\";

/// `generate` names the board crate `<project>-board`, and a crate name is at most 64 bytes.
const PROJECT_MAX: usize = 58;
const BASE_RULE: &str = "1 to 64 bytes of printable ASCII 0x21-0x7E, none of them `/`, `{`, `}`, `$`, `[`, `]`, `\"`, `%`, `*`, `,` or `\\`, and no date shape (4 digits, `-`, 2 digits, `-`, 2 digits) or time shape (2 digits, `:`, 2 digits, `:`, 2 digits)";
const PROJECT_RULE: &str = "a project name is a crate name: 1 to 58 bytes of `a-z`, `0-9` and `-`, starting with a letter, not ending with `-`, with no `--` (`generate` names the board crate `<project>-board`, at most 64 bytes)";
const REFERENCE_RULE: &str =
    "a device reference is a Renode identifier: a letter, then letters, digits and `_`";
const RESERVED_RULE: &str = "a device reference may not be a name the Renode platform already holds, in any case: a stock instance, `machine`, `sysbus`, a `.repl` keyword, or an SPI controller `spi1` to `spi6` or its multiplexer `spi1mux` to `spi6mux` (`RESERVED_REFERENCES`)";

/// The names a device reference may not take, compared on its ASCII-lowercase
/// form (`AET-WO-2204` D5, `RUL-14`). The emitter declares each modelled device
/// in the Renode `.repl` under its reference lowercased, after
/// `using "platforms/cpus/stm32h753.repl"`; Renode refuses the load when that
/// name is already declared (E02) or is a keyword (E00). Renode compares names
/// case-sensitively, so only an all-lowercase stock name can collide
/// (`gpioPortA` lowercases to `gpioporta`, which loads).
///
/// Sorted. It holds: the all-lowercase instance names declared by Renode
/// 1.17.0's chain `stm32h753.repl` (sha256 `13c5571e...d10ce`), `stm32h743.repl`
/// (`86b2c2c5...b231c`) and `stm32h7.repl` (`b85cb4af...251e`); `machine` and
/// `sysbus`, which `CreationDriver.cs` declares first; and the keywords of
/// `Grammar.cs` (`using`, `new`, `as`, `preinit`, `init`, `reset`, `local`,
/// `true`, `false`, `none`, `empty`) lowercased; and what the emitter declares
/// for each SPI controller: the controller itself where the chain does not
/// (`spi1` to `spi6`, `AET-WO-4300c` F21) and its multiplexer, `spi1mux` to
/// `spi6mux` (`AET-WO-4200` D7). The tier-1 test `aethelgard-twin/tests/reserved_names.rs`
/// parses the pinned files and asserts this list equal to that set and the
/// emitter's mux names, so the list moves only with the pin or the emitter.
pub const RESERVED_REFERENCES: &[&str] = &[
    "adc3",
    "as",
    "bdma",
    "cpu",
    "crc",
    "crypto",
    "dma1",
    "dma2",
    "dma2d",
    "dmamux1",
    "dtcm",
    "dwt",
    "empty",
    "ethernet",
    "exti",
    "false",
    "fdcan1",
    "fdcan2",
    "fmc",
    "hsem",
    "i2c1",
    "i2c2",
    "i2c3",
    "i2c4",
    "init",
    "itcm",
    "local",
    "lptimer1",
    "lptimer2",
    "lptimer3",
    "lptimer4",
    "lptimer5",
    "lpuart1",
    "ltdc",
    "machine",
    "new",
    "none",
    "nvic",
    "preinit",
    "pwr",
    "qspi",
    "rcc",
    "reset",
    "rng",
    "rtc",
    "sdmmc",
    "spi1",
    "spi1mux",
    "spi2",
    "spi2mux",
    "spi3",
    "spi3mux",
    "spi4",
    "spi4mux",
    "spi5",
    "spi5mux",
    "spi6",
    "spi6mux",
    "sram1",
    "sram2",
    "sram3",
    "sram4",
    "sysbus",
    "syscfg",
    "system_flash_ro",
    "timer1",
    "timer12",
    "timer13",
    "timer14",
    "timer15",
    "timer16",
    "timer17",
    "timer2",
    "timer3",
    "timer4",
    "timer5",
    "timer6",
    "timer7",
    "timer8",
    "true",
    "uart4",
    "uart5",
    "uart7",
    "uart8",
    "usart1",
    "usart2",
    "usart3",
    "usart6",
    "usb1",
    "usb2",
    "using",
    "watchdog",
];

/// Whether `value` holds `dddd-dd-dd` or `dd:dd:dd` (`d` an ASCII digit) anywhere.
fn clock_shape(value: &[u8]) -> bool {
    let shaped = |shape: &[u8]| {
        value.windows(shape.len()).any(|w| {
            w.iter().zip(shape).all(|(b, s)| match s {
                b'd' => b.is_ascii_digit(),
                _ => b == s,
            })
        })
    };
    shaped(b"dddd-dd-dd") || shaped(b"dd:dd:dd")
}

/// `RUL-14`'s one table: `Err` is the rule `value` breaks as an identifier of
/// `kind`. The base set is checked first, so a message names the broadest
/// rule broken. [`identifier`] and the scenario reader both call it.
pub fn check_identifier(kind: IdentifierKind, value: &str) -> Result<(), &'static str> {
    let bytes = value.as_bytes();
    let base = (1..=64).contains(&bytes.len())
        && bytes
            .iter()
            .all(|b| (0x21..=0x7E).contains(b) && !REFUSED_IN_IDENTIFIERS.contains(b))
        && !clock_shape(bytes);
    if !base {
        return Err(BASE_RULE);
    }
    let (ok, rule) = match kind {
        IdentifierKind::Plain => return Ok(()),
        IdentifierKind::Project => (
            bytes.len() <= PROJECT_MAX
                && bytes[0].is_ascii_lowercase()
                && !value.ends_with('-')
                && !value.contains("--")
                && bytes
                    .iter()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-'),
            PROJECT_RULE,
        ),
        IdentifierKind::Reference => (
            bytes[0].is_ascii_alphabetic()
                && bytes
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b == b'_'),
            REFERENCE_RULE,
        ),
    };
    if !ok {
        return Err(rule);
    }
    // `generate` writes `<project>.kicad_sch`, `<project>.repl` and the
    // fabrication files under the project name (`AET-WO-1102` A7).
    if kind == IdentifierKind::Project && names::windows_reserved(value) {
        return Err(WINDOWS_RULE);
    }
    if kind == IdentifierKind::Reference {
        let lowered = value.to_ascii_lowercase();
        if RESERVED_REFERENCES.binary_search(&lowered.as_str()).is_ok() {
            return Err(RESERVED_RULE);
        }
        if is_observer_name(&lowered) {
            return Err(OBSERVER_RULE);
        }
    }
    Ok(())
}

const WINDOWS_RULE: &str = "a project name may not be a Windows reserved device name (`con`, `prn`, `aux`, `nul`, `com0` to `com9`, `lpt0` to `lpt9`) in any case: `generate` names files after it, and Windows cannot open such a file (`AET-WO-1102` A7)";

const OBSERVER_RULE: &str = "a device reference may not be a motor output's observer name, `p`, a port letter `a` to `k`, a pin number 0 to 15, then `obs` (`pa8obs`), in any case: the Renode platform declares one on each motor output's pin (`AET-WO-4300a`)";

/// Whether `name` (lowercase) is the name the twin's platform gives the
/// observer of a pin, `pa8obs` (`aethelgard_twin::timer::observer_name`).
pub fn is_observer_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix('p') else {
        return false;
    };
    let mut chars = rest.chars();
    let port = chars.next().is_some_and(|c| ('a'..='k').contains(&c));
    let Some(number) = chars.as_str().strip_suffix("obs") else {
        return false;
    };
    port && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
        && (number == "0" || !number.starts_with('0'))
        && number.parse::<u32>().is_ok_and(|n| n <= 15)
}

pub(crate) fn identifier(
    kind: IdentifierKind,
    value: &str,
    path: impl FnOnce() -> String,
) -> Result<(), IrValidationError> {
    check_identifier(kind, value)
        .map_err(|rule| IrValidationError::InvalidIdentifier { path: path(), rule })
}

pub(crate) fn plain(value: &str, path: impl FnOnce() -> String) -> Result<(), IrValidationError> {
    identifier(IdentifierKind::Plain, value, path)
}

pub(crate) fn optional_identifier(
    value: &Option<String>,
    path: impl FnOnce() -> String,
) -> Result<(), IrValidationError> {
    value.as_deref().map_or(Ok(()), |v| plain(v, path))
}

/// Step 2 (`AET-WO-4220b`): a declared pin capacitance's condition and source
/// are each a source text ([`board::source`]).
fn declared_texts(c: &Option<PinCapacitance>, at: &str) -> Result<(), IrValidationError> {
    if let Some(c) = c {
        board::source(
            &c.declared.condition,
            &format!("{at}.pin_capacitance.declared.condition"),
        )?;
        board::source(
            &c.declared.source,
            &format!("{at}.pin_capacitance.declared.source"),
        )?;
    }
    Ok(())
}

fn signal_identifiers(signal: &Signal, path: &str) -> Result<(), IrValidationError> {
    plain(&signal.net, || format!("{path}.net"))?;
    optional_identifier(&signal.pin, || format!("{path}.pin"))
}

impl Specification {
    /// The I2C devices the specification declares, in document order. The
    /// runners that consume a `SolvedTopology` must not read this (`RUL-16`):
    /// `cli`, `svd-hal` and `twin` each carry a `clippy.toml` that disallows
    /// it. The schematic draws declared intent, so `aethelgard-eda` may.
    pub fn declared_i2c_devices(&self) -> &[I2cDevice] {
        &self.i2c_devices
    }

    /// The SPI devices the specification declares, in document order; fenced
    /// as [`Specification::declared_i2c_devices`] is.
    pub fn declared_spi_devices(&self) -> &[SpiDevice] {
        &self.spi_devices
    }

    /// The off-board connectors the specification declares, in document
    /// order; fenced as [`Specification::declared_i2c_devices`] is. Nothing
    /// solves a connector, so the `SolvedTopology` never carries one.
    pub fn declared_connectors(&self) -> &[connectors::Connector] {
        &self.connectors
    }

    /// The test points the specification declares, in document order; fenced
    /// as [`Specification::declared_connectors`] is (`AET-WO-1312`): nothing
    /// solves a test point, and no runner reads one.
    pub fn declared_test_points(&self) -> &[bench::TestPoint] {
        &self.test_points
    }

    /// The devices with no bus, in document order (`AET-WO-1211a`); fenced as
    /// [`Specification::declared_connectors`] is (`RUL-16`): the solver reads
    /// them, and every other reader refuses a topology that carries one.
    pub fn declared_pin_devices(&self) -> &[pin_devices::PinDevice] {
        &self.pin_devices
    }

    /// Refuses the first malformed value found, in the order of
    /// `crates/aethelgard-ir/README.md`, and in document order within each step.
    /// It does not check that a part, peripheral or pin exists, nor that two
    /// signals ask for one pin: both need the pin table (`AET-WO-1200`).
    ///
    /// Its last step reads the embedded parts table (`AET-WO-4200` D3): a
    /// malformed table is refused here, as JSON with a nonzero exit, never as
    /// a panic later.
    pub fn validate(&self) -> Result<(), IrValidationError> {
        self.validate_declared()?;
        let table = parts::table()?;
        self.validate_parts(table)?;
        // 15
        connectors::validate(self)?;
        // 15a (`AET-WO-1312`)
        bench::validate(self)?;
        // 16 (`AET-WO-4300d`)
        power::check(self)?;
        // 17, 18 (`AET-WO-4300g`)
        connectors::programming_path(self)?;
        connectors::supply_source(self)?;
        // 19 (`AET-WO-1206` §1.2)
        self.bus_constraints()?;
        // 20 (`AET-WO-1206` §1.3)
        names::check(self)?;
        // 20a (`AET-WO-1209` §1.2, D3 as ruled)
        names::loads(self)?;
        // 21 (`AET-WO-1207` §1.2, `AET-WO-1208` §1.3)
        self.solve_static()?;
        // 22 (`AET-WO-1208` §1.2, F1/F5 as ruled on PR #182)
        power::mcu_without_tree(self)?;
        // 23 (`AET-WO-1210`, `AET-WO-1209` §1.1): a drawn board only
        drawn::pin_capacitance(self, table)?;
        // 24 (`AET-WO-1211a`): each pin device
        pin_devices::validate(self)
    }

    /// Step 21: what `solve` refused from the specification and the reset
    /// clock alone before `AET-WO-1207` and `AET-WO-1208`, in the order
    /// `solve` judged it, by `solve`'s kind, `id`, claimant and message
    /// (`1206` F3, `1207` F1). Nothing is defaulted (`INV-06`):
    /// 1. each I2C bus's worse line (step 19 checked both lines carry a
    ///    constraint) has a rise time some I2C mode allows, and its speed is
    ///    at most that mode's (`RiseTimeTooSlow`, `BusTooFast`);
    /// 2. each I2C bus's pull-up is at least the sink-current floor at its
    ///    domain's maximum rail (`AET-WO-4230` §1.5, review A8), and each
    ///    line's capacitance at most its bus's limit, SDA then SCL;
    /// 3. each SPI bus has its MCU supply inside DS12117 Table 149's bands
    ///    (every SPI bus is on the MCU's supply, step 5a; the bands bound the
    ///    whole supply, not only its minimum) and its SCK and MOSI carry a
    ///    net constraint, whose load the output speed is chosen for
    ///    (`AET-WO-4300c` D2);
    /// 4. each motor output's timing fits a timer on the reset clock, which
    ///    gives every timer one kernel clock (RM0433 Rev 9 Table 57);
    /// 5. each serial port's `BRR` and worst-case baud error, on the reset
    ///    clock's one USART/UART kernel clock;
    /// 6. each I2C bus's pull-up again, at its domain's nominal rail, as
    ///    `solve`'s topology judged it last.
    fn solve_static(&self) -> Result<(), IrValidationError> {
        for bus in &self.i2c_buses {
            let rise_time_ns = [&bus.sda.net, &bus.scl.net]
                .into_iter()
                .filter_map(|net| self.net_constraints.iter().find(|c| &c.net_name == net))
                .map(|c| c.max_rise_time_nanoseconds)
                .max()
                .unwrap_or(0);
            let Some(ceiling_hz) = electrical::ceiling_hz(rise_time_ns) else {
                return Err(IrValidationError::RiseTimeTooSlow {
                    bus: bus.id.clone(),
                    rise_time_ns,
                });
            };
            if bus.speed_hz > ceiling_hz {
                return Err(IrValidationError::BusTooFast {
                    bus: bus.id.clone(),
                    speed_hz: bus.speed_hz,
                    ceiling_hz,
                });
            }
        }
        for bus in &self.i2c_buses {
            // Step 5 checked every bus's domain.
            if let Some(domain) = self.voltage_domains.iter().find(|d| d.id == bus.domain) {
                let floor_ohms =
                    electrical::i2c_pull_up_floor_ohms(bus.speed_hz, maximum_millivolts(domain));
                if bus.pull_up_ohms < floor_ohms {
                    return Err(IrValidationError::PullUpBelowSinkFloor {
                        bus: bus.id.clone(),
                        pull_up_ohms: bus.pull_up_ohms,
                        floor_ohms,
                    });
                }
            }
            let limit_femtofarads = electrical::i2c_bus_capacitance_limit_femtofarads(bus.speed_hz);
            for net in [&bus.sda.net, &bus.scl.net] {
                if let Some(c) = self.net_constraints.iter().find(|c| &c.net_name == net)
                    && c.max_capacitance_femtofarads > limit_femtofarads
                {
                    return Err(IrValidationError::BusCapacitanceOverLimit {
                        net: net.clone(),
                        capacitance_femtofarads: c.max_capacitance_femtofarads,
                        limit_femtofarads,
                    });
                }
            }
        }
        let (bottom, top) = (speeds::BANDS_MILLIVOLTS[0].0, speeds::BANDS_MILLIVOLTS[1].1);
        let supply = self
            .voltage_domains
            .iter()
            .find(|d| d.id == self.mcu.supply);
        for bus in &self.spi_buses {
            if let Some(supply) = supply {
                let (min, max) = (minimum_millivolts(supply), maximum_millivolts(supply));
                if min < bottom || max > top {
                    return Err(IrValidationError::SupplyOutsideSpeedTable {
                        bus: bus.id.clone(),
                        min_millivolts: min,
                        max_millivolts: max,
                    });
                }
            }
            for net in [&bus.sck.net, &bus.mosi.net] {
                if !self.net_constraints.iter().any(|c| &c.net_name == net) {
                    return Err(IrValidationError::MissingSpiConstraint {
                        bus: bus.id.clone(),
                        net: net.clone(),
                    });
                }
            }
        }
        let clock = solve::solved_clock(&self.mcu)?;
        solve::motor_timing(self, &clock)?;
        for (index, port) in self.serial_ports.iter().enumerate() {
            solve::serial_timing(&port.label, index, &clock, port)?;
        }
        // 6. Each I2C bus's pull-up at its domain's nominal rail, as `solve`'s
        // own topology judges it last (`SolvedTopology::validate`, the
        // `analog_lines` carry the nominal), buses in id order as the
        // topology keys them. The floor jumps at 2.0 V and 2.7 V, so a pull-up
        // can pass at the maximum (2 above) and fail here (adversarial pass,
        // `AET-WO-1208` FEEDBACK F4).
        let mut buses: Vec<&I2cBus> = self.i2c_buses.iter().collect();
        buses.sort_by(|a, b| a.id.cmp(&b.id));
        for bus in buses {
            if let Some(domain) = self.voltage_domains.iter().find(|d| d.id == bus.domain) {
                let floor_ohms =
                    electrical::i2c_pull_up_floor_ohms(bus.speed_hz, domain.nominal_millivolts);
                if bus.pull_up_ohms < floor_ohms {
                    return Err(IrValidationError::PullUpBelowSinkFloor {
                        bus: bus.id.clone(),
                        pull_up_ohms: bus.pull_up_ohms,
                        floor_ohms,
                    });
                }
            }
        }
        Ok(())
    }

    /// Step 19: each I2C bus's SDA then SCL carries a net constraint, in
    /// document order, as `solve` required before (`AET-WO-4100` F1): its
    /// rise time bounds the bus's speed, and nothing is defaulted (`INV-06`).
    fn bus_constraints(&self) -> Result<(), IrValidationError> {
        for bus in &self.i2c_buses {
            for net in [&bus.sda.net, &bus.scl.net] {
                if !self.net_constraints.iter().any(|c| &c.net_name == net) {
                    return Err(IrValidationError::MissingBusConstraint {
                        bus: bus.id.clone(),
                        net: net.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Step 14 (`AET-WO-4300c` §1.5, F11): each device's part against the
    /// table. The bare `W25Q64JV` is refused by name; a part with an entry
    /// must sit on its kind of bus, at an address its straps offer, on a bus
    /// clocked no faster than it; each refusal cites the datasheet. A part
    /// with no entry is refused (`AET-WO-1206` §1.8, `PartWithoutRow`).
    fn validate_parts(&self, table: &parts::PartsTable) -> Result<(), IrValidationError> {
        // F-a: a declaration nothing judges is refused, not silently unread.
        let unjudged = self
            .spi_buses
            .iter()
            .map(|b| &b.miso.net)
            .chain(self.spi_devices.iter().map(|d| &d.cs.net));
        for net in unjudged {
            if self.net_constraints.iter().any(|c| &c.net_name == net) {
                return Err(IrValidationError::UnjudgedSpiConstraint { id: net.clone() });
            }
        }
        for m in &self.motor_outputs {
            if self.net_constraints.iter().any(|c| c.net_name == m.label) {
                return Err(IrValidationError::UnjudgedMotorConstraint {
                    id: m.label.clone(),
                });
            }
        }
        let serial_nets = self
            .serial_ports
            .iter()
            .flat_map(|p| [&p.tx.net, &p.rx.net]);
        for net in serial_nets {
            if self.net_constraints.iter().any(|c| &c.net_name == net) {
                return Err(IrValidationError::UnjudgedSerialConstraint { id: net.clone() });
            }
        }
        // Each device's part has a row (`AET-WO-1206` §1.8), I2C then SPI in
        // document order: a part with none would pass every limit below
        // unheld, and `generate` refuses it (`UnknownPart`).
        let parts = self
            .i2c_devices
            .iter()
            .enumerate()
            .map(|(i, d)| (&d.reference, &d.part, format!("i2c_devices[{i}].part")))
            .chain(
                self.spi_devices
                    .iter()
                    .enumerate()
                    .map(|(i, d)| (&d.reference, &d.part, format!("spi_devices[{i}].part"))),
            );
        for (reference, part, path) in parts {
            // A busless row is never a bus device's (`AET-WO-1211a`).
            if let Some(e) = pin_devices::part_without_bus(reference, part, &path) {
                return Err(e);
            }
            if part_entry(table, reference, part)?.is_none() {
                return Err(IrValidationError::PartWithoutRow {
                    reference: reference.clone(),
                    part: part.clone(),
                });
            }
        }
        // `AET-WO-4400d`: an orientation only on an IMU, whose axes and
        // driver's frame are cited.
        for d in self.spi_devices.iter().filter(|d| d.orientation.is_some()) {
            let row = part_entry(table, &d.reference, &d.part)?;
            let ap = row.and_then(|r| r.ardupilot.as_ref());
            if !ap.is_some_and(|a| a.kind == parts::ArduPilotKind::Imu) {
                return Err(IrValidationError::OrientationOnNonImu {
                    reference: d.reference.clone(),
                    part: d.part.clone(),
                });
            }
            let missing = if row.is_some_and(|r| r.axes.is_none()) {
                Some("axes")
            } else if ap.is_some_and(|a| a.frame.is_none()) {
                Some("ardupilot.frame")
            } else {
                None
            };
            if let Some(missing) = missing {
                return Err(IrValidationError::ImuAxesNotCited {
                    reference: d.reference.clone(),
                    part: d.part.clone(),
                    missing,
                });
            }
        }
        // Each device's supply within its part's range (adversarial 2):
        // step 5 checked every supply names a domain.
        let devices = self
            .i2c_devices
            .iter()
            .map(|d| (&d.reference, &d.part, &d.supply))
            .chain(
                self.spi_devices
                    .iter()
                    .map(|d| (&d.reference, &d.part, &d.supply)),
            );
        for (reference, part, supply) in devices {
            let (Some(p), Some(domain)) = (
                part_entry(table, reference, part)?,
                self.voltage_domains.iter().find(|v| &v.id == supply),
            ) else {
                continue;
            };
            let (min, max) = (minimum_millivolts(domain), maximum_millivolts(domain));
            let (part_min, part_max) = p.supply_millivolts;
            if min < part_min || max > part_max {
                return Err(IrValidationError::SupplyOutsidePart {
                    id: reference.clone(),
                    part: p.part.clone(),
                    min_millivolts: min,
                    max_millivolts: max,
                    part_min_millivolts: part_min,
                    part_max_millivolts: part_max,
                    citation: cite(&p.citations.supply),
                });
            }
        }
        // Each device's bus domain within its part's I/O range, where the
        // datasheet separates it from the supply (`AET-WO-4210` D7, `4200`
        // F19). Step 9 checked every device's bus, step 5 every bus's domain.
        // An SPI device behind a declared crossing on its bus is on the
        // crossing's B side, so that domain is the one judged, as step 5's
        // supply rule reads it (`AET-WO-4300d` D9).
        let on_bus = self
            .i2c_devices
            .iter()
            .map(|d| {
                (
                    &d.reference,
                    &d.part,
                    &d.bus,
                    self.i2c_buses
                        .iter()
                        .find(|b| b.id == d.bus)
                        .map(|b| &b.domain),
                )
            })
            .chain(self.spi_devices.iter().map(|d| {
                let crossed = self.power.as_ref().and_then(|p| {
                    p.crossings
                        .iter()
                        .find(|c| c.bus == d.bus && c.b_domain == d.supply)
                });
                (
                    &d.reference,
                    &d.part,
                    &d.bus,
                    match crossed {
                        Some(c) => Some(&c.b_domain),
                        None => self
                            .spi_buses
                            .iter()
                            .find(|b| b.id == d.bus)
                            .map(|b| &b.domain),
                    },
                )
            }));
        for (reference, part, bus, domain) in on_bus {
            let (Some(p), Some(domain)) = (
                part_entry(table, reference, part)?,
                domain.and_then(|id| self.voltage_domains.iter().find(|v| &v.id == id)),
            ) else {
                continue;
            };
            let (min, max) = (minimum_millivolts(domain), maximum_millivolts(domain));
            let ((part_min, part_max), citation) = p.io_range();
            if min < part_min || max > part_max {
                return Err(IrValidationError::IoOutsidePart {
                    id: reference.clone(),
                    part: p.part.clone(),
                    bus: bus.clone(),
                    min_millivolts: min,
                    max_millivolts: max,
                    part_min_millivolts: part_min,
                    part_max_millivolts: part_max,
                    citation: cite(citation).into(),
                });
            }
        }
        for d in &self.i2c_devices {
            if let Some(p) = part_entry(table, &d.reference, &d.part)? {
                check_i2c_address(&d.reference, p, d.address)?;
            }
        }
        for d in &self.spi_devices {
            if let Some(p) = part_entry(table, &d.reference, &d.part)? {
                // Step 9 checked every device's bus.
                let speed_hz = self
                    .spi_buses
                    .iter()
                    .find(|b| b.id == d.bus)
                    .map_or(0, |b| b.speed_hz);
                check_spi_device(&d.reference, p, speed_hz)?;
            }
        }
        // `AET-WO-4220b` §1.2: a declaration is within the table's own range,
        // and never stands beside a cited maximum. I2C then SPI, in
        // document order.
        let declared = self
            .i2c_devices
            .iter()
            .map(|d| (&d.reference, &d.part, &d.pin_capacitance, true))
            .chain(
                self.spi_devices
                    .iter()
                    .map(|d| (&d.reference, &d.part, &d.pin_capacitance, false)),
            );
        for (reference, part, pin_capacitance, i2c) in declared {
            let Some(c) = pin_capacitance else { continue };
            let femtofarads = c.declared.femtofarads;
            if !(1..=parts::MAX_PIN_FEMTOFARADS).contains(&femtofarads) {
                return Err(IrValidationError::PinCapacitanceOutOfRange {
                    id: reference.clone(),
                    femtofarads,
                    maximum: parts::MAX_PIN_FEMTOFARADS,
                });
            }
            if i2c && femtofarads < electrical::I2C_PIN_CI_FEMTOFARADS {
                return Err(IrValidationError::PinCapacitanceBelowI2cCi {
                    id: reference.clone(),
                    femtofarads,
                    ci: electrical::I2C_PIN_CI_FEMTOFARADS,
                });
            }
            if let Some(p) = part_entry(table, reference, part)?
                && let Some(maximum) = p.pin_capacitance_femtofarads
            {
                return Err(IrValidationError::PinCapacitanceAlreadyCited {
                    id: reference.clone(),
                    part: p.part.clone(),
                    maximum_femtofarads: maximum,
                    citation: cite(&p.citations.pin_capacitance),
                });
            }
            if let Some(p) = part_entry(table, reference, part)?
                && let Some(typical) = p.pin_capacitance_typical_femtofarads
                && femtofarads < typical
            {
                return Err(IrValidationError::PinCapacitanceBelowTypical {
                    id: reference.clone(),
                    part: p.part.clone(),
                    femtofarads,
                    typical_femtofarads: typical,
                    citation: cite(&p.citations.pin_capacitance),
                });
            }
        }
        self.validate_lifecycle(table)
    }

    /// Step 15 (`AET-WO-4210`, ruled by Scotty on D1): each `eol` or `nrnd`
    /// part a device uses is acknowledged in `accept_lifecycle`, once, at the
    /// status the table cites, with a statement; and each entry acknowledges
    /// such a part. A part with no table entry has no lifecycle to
    /// acknowledge. Devices in the specification's order, I2C then SPI.
    fn validate_lifecycle(&self, table: &parts::PartsTable) -> Result<(), IrValidationError> {
        use IrValidationError as E;
        let mut seen = BTreeSet::new();
        for a in &self.accept_lifecycle {
            if !seen.insert(a.part.as_str()) {
                return Err(E::LifecycleAcceptedTwice { id: a.part.clone() });
            }
            let printable = a.statement.bytes().all(|b| (0x20..=0x7E).contains(&b));
            if a.statement.trim().is_empty()
                || a.statement.len() > MAX_LIFECYCLE_STATEMENT
                || !printable
            {
                return Err(E::LifecycleStatementInvalid { id: a.part.clone() });
            }
        }
        let devices = self
            .i2c_devices
            .iter()
            .map(|d| (&d.reference, &d.part))
            .chain(self.spi_devices.iter().map(|d| (&d.reference, &d.part)));
        let mut used = BTreeSet::new();
        for (reference, part) in devices {
            let Some(p) = part_entry(table, reference, part)? else {
                continue;
            };
            if p.lifecycle == parts::Lifecycle::Active {
                continue;
            }
            used.insert(p.part.as_str());
            let status = p.lifecycle.name();
            let citation = cite(&p.citations.lifecycle);
            match self.accept_lifecycle.iter().find(|a| a.part == p.part) {
                None => {
                    return Err(E::LifecycleNotAccepted {
                        id: reference.clone(),
                        part: p.part.clone(),
                        status,
                        citation,
                    });
                }
                Some(a) if a.status != status => {
                    return Err(E::LifecycleStatusMismatch {
                        id: reference.clone(),
                        part: p.part.clone(),
                        status,
                        accepted: a.status.clone(),
                        citation: citation.into(),
                    });
                }
                Some(_) => {}
            }
        }
        if let Some(a) = self
            .accept_lifecycle
            .iter()
            .find(|a| !used.contains(a.part.as_str()))
        {
            return Err(E::LifecycleAcceptanceUnused { id: a.part.clone() });
        }
        Ok(())
    }

    /// [`lifecycle_banner`] for the declared devices, in order.
    pub fn lifecycle_banner(&self) -> Result<Vec<String>, IrValidationError> {
        let devices = self
            .i2c_devices
            .iter()
            .map(|d| (d.reference.as_str(), d.part.as_str()))
            .chain(
                self.spi_devices
                    .iter()
                    .map(|d| (d.reference.as_str(), d.part.as_str())),
            );
        Ok(lifecycle_banner(parts::table()?, devices))
    }

    fn validate_declared(&self) -> Result<(), IrValidationError> {
        use IrValidationError as E;

        // 1
        if self.schema_version != SPECIFICATION_SCHEMA_VERSION {
            return Err(E::UnsupportedSchemaVersion {
                version: self.schema_version,
                expected: SPECIFICATION_SCHEMA_VERSION,
            });
        }
        // 2
        self.identifiers()?;
        // 2a: the HSE (`AET-WO-4240a`): its declaration, its tree, and the
        // judge on the tree the solver would choose.
        clock::judge_tree(&solve::solved_clock(&self.mcu)?)?;
        // 2b
        if self.mcu.bor_level > MAX_BOR_LEVEL {
            return Err(E::BorLevelOutOfRange {
                value: self.mcu.bor_level,
            });
        }
        // 2c (`AET-WO-4240v` D-1)
        board::source(&self.mcu.core_supply.source, "mcu.core_supply.source")?;
        // 2d (`AET-WO-4400g` §1.2, `AET-WO-4240v` D-4)
        if let Some(c) = &self.mcu.foreign_clock {
            for (path, s) in [
                ("mcu.foreign_clock.apb1_timers_hz.source", &c.apb1_timers_hz),
                ("mcu.foreign_clock.apb2_timers_hz.source", &c.apb2_timers_hz),
            ] {
                board::source(&s.source, path)?;
            }
            board::source(&c.vos.source, "mcu.foreign_clock.vos.source")?;
            for (usart, s) in &c.usarts {
                board::source(
                    &s.source,
                    &format!("mcu.foreign_clock.usarts.{usart}.source"),
                )?;
            }
            foreign_clock_values(&motor::SolvedForeignClock::from(c))?;
        }
        // 3
        if self.voltage_domains.is_empty() {
            return Err(E::NoVoltageDomain);
        }
        // 4
        let mut folded_ids = BTreeSet::new();
        for domain in &self.voltage_domains {
            let id = &domain.id;
            // Power names that differ only in case (or from the emitter's `GND`)
            // are refused by KiCad's ERC, as the net rule in step 11 is.
            let folded = id.to_lowercase();
            if (folded == "gnd" && id != "GND") || !folded_ids.insert(folded) {
                return Err(E::DuplicateDomainId { id: id.clone() });
            }
        }
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
            // A device behind a declared crossing sits on the crossing's
            // other side; step 16 judges that side (`AET-WO-4300d` D9).
            let crossed = self.power.as_ref().is_some_and(|p| {
                p.crossings
                    .iter()
                    .any(|c| c.bus == device.bus && c.b_domain == device.supply)
            });
            if !crossed && bus.is_some_and(|b| b.domain != device.supply) {
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
            if !I2C_SPEEDS_HZ.contains(&bus.speed_hz) {
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
        // Folded like the nets of step 11: KiCad's ERC refuses references
        // that differ only in case (`AET-WO-1204`).
        references_distinct(
            self.i2c_devices
                .iter()
                .map(|d| d.reference.as_str())
                .chain(self.spi_devices.iter().map(|d| d.reference.as_str()))
                .chain(self.connectors.iter().map(|c| c.reference.as_str()))
                .chain(pin_devices::references(self)),
        )?;
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
        // Names are compared with case folded: KiCad's ERC refuses labels that
        // differ only in case (`similar_labels`, `similar_label_and_power`).
        let folded_domains: BTreeSet<String> = domains.iter().map(|d| d.to_lowercase()).collect();
        // The MCU's debug nets are the engine's (`AET-WO-4300g`): a declared
        // net or domain spelling one, in any case, is refused.
        let mut folded_nets: BTreeSet<String> =
            DEBUG_NETS.iter().map(|n| n.to_lowercase()).collect();
        if let Some(d) = self
            .voltage_domains
            .iter()
            .find(|d| folded_nets.contains(&d.id.to_lowercase()))
        {
            return Err(E::NetShadowsDomain { id: d.id.clone() });
        }
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
            .chain(self.spi_devices.iter().map(|d| &d.cs))
            .chain(self.serial_ports.iter().flat_map(|p| [&p.tx, &p.rx]))
            .map(|s| &s.net)
            .chain(self.motor_outputs.iter().map(|m| &m.label))
            .chain(pin_devices::nets(self));
        for net in signals {
            let folded = net.to_lowercase();
            if !folded_nets.insert(folded.clone()) {
                return Err(E::DuplicateNetName { id: net.clone() });
            }
            nets.insert(net.as_str());
            // `GND` is the power name the emitter adds to every schematic.
            if folded_domains.contains(&folded) || (folded == "gnd" && net != "GND") {
                return Err(E::NetShadowsDomain { id: net.clone() });
            }
        }
        // A motor output's net, or a serial port's, is a signal that leaves
        // the board; named `GND`, it would join the emitter's ground (`4300a`
        // second adversarial review, F2).
        let mut leaving = self.motor_outputs.iter().map(|m| &m.label).chain(
            self.serial_ports
                .iter()
                .flat_map(|p| [&p.tx.net, &p.rx.net]),
        );
        if let Some(net) = leaving.find(|n| n.eq_ignore_ascii_case(GND)) {
            return Err(E::NetShadowsDomain { id: net.clone() });
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
        // 13
        self.board.validate(&domains)?;
        // 13, `AET-WO-4400d`: an orientation's axes are at right angles,
        // and its frame's forward edge is declared.
        for d in &self.spi_devices {
            let Some(o) = &d.orientation else { continue };
            if !o.perpendicular() {
                return Err(E::ImuAxesNotPerpendicular {
                    reference: d.reference.clone(),
                    x: o.x.name().to_owned(),
                    z: o.z.name().to_owned(),
                });
            }
            if self.board.forward.is_none() {
                return Err(E::ForwardUndeclared {
                    reference: d.reference.clone(),
                });
            }
        }
        // 13a. A power tree adds its battery rail, which a class may name
        // (`AET-WO-4300d2` K10).
        let mut rails = domains.clone();
        if self.power.is_some() {
            rails.insert(power::BATTERY_NET);
        }
        classes::validate(
            &self.net_classes,
            &self.diff_pairs,
            &self.board,
            &rails,
            &nets,
        )?;
        // 13b: each motor output's timing, in document order (`AET-WO-4300a`).
        for m in &self.motor_outputs {
            motor_shape(m)?;
        }
        // 13c: each serial port, in document order (`AET-WO-4300b`).
        serial::validate(&self.serial_ports)?;
        // 13d: each unwitnessed acknowledgement's shape (`AET-WO-4230` R1).
        self.validate_unwitnessed()
    }

    /// Step 13d (`AET-WO-4230` R1), each entry in document order: a register
    /// of the ledger's shape, a class of [`UnwitnessedClass`], at least one
    /// field and none twice, a reason by the repository's rule for a source
    /// ([`board::source`], `InvalidSource` naming
    /// `acknowledge_unwitnessed[i].reason`), and no second entry for the same
    /// register and class. Whether each field is one the ledger has for that
    /// register and class needs the pin table: the CLI's `validate` judges it
    /// (`UnwitnessedAcknowledgementStale`).
    fn validate_unwitnessed(&self) -> Result<(), IrValidationError> {
        use IrValidationError as E;
        let mut entries = BTreeSet::new();
        for (i, a) in self.acknowledge_unwitnessed.iter().enumerate() {
            let id = || a.register.clone();
            if !register_name_ok(&a.register) {
                return Err(E::UnwitnessedRegisterMalformed { id: id() });
            }
            let Some(class) = a.class_kind() else {
                return Err(E::UnwitnessedClassUnknown {
                    id: id(),
                    class: a.class.clone(),
                });
            };
            if a.fields.is_empty() {
                return Err(E::UnwitnessedFieldsEmpty {
                    id: id(),
                    class: a.class.clone(),
                });
            }
            let mut fields = BTreeSet::new();
            if let Some(field) = a.fields.iter().find(|f| !fields.insert(f.as_str())) {
                return Err(E::UnwitnessedFieldTwice {
                    id: id(),
                    class: a.class.clone(),
                    field: field.clone(),
                });
            }
            board::source(&a.reason, &format!("acknowledge_unwitnessed[{i}].reason"))?;
            if !entries.insert((a.register.as_str(), class)) {
                return Err(E::UnwitnessedAcknowledgedTwice {
                    id: id(),
                    class: a.class.clone(),
                });
            }
        }
        Ok(())
    }

    /// The entry acknowledging `register`'s rows of `class`, if any (at most
    /// one, once `validate` has passed).
    pub fn unwitnessed_acknowledgement(
        &self,
        register: &str,
        class: UnwitnessedClass,
    ) -> Option<&UnwitnessedAcknowledgement> {
        self.acknowledge_unwitnessed
            .iter()
            .find(|a| a.register == register && a.class_kind() == Some(class))
    }

    /// Step 2, in the order the README lists.
    fn identifiers(&self) -> Result<(), IrValidationError> {
        identifier(IdentifierKind::Project, &self.project_name, || {
            "project_name".into()
        })?;
        plain(&self.mcu.part, || "mcu.part".into())?;
        plain(&self.mcu.supply, || "mcu.supply".into())?;
        if let Some(s) = &self.mcu.supply_source {
            plain(&s.connector, || "mcu.supply_source.connector".into())?;
            plain(&s.pin, || "mcu.supply_source.pin".into())?;
        }
        for (i, domain) in self.voltage_domains.iter().enumerate() {
            plain(&domain.id, || format!("voltage_domains[{i}].id"))?;
        }
        for (i, bus) in self.i2c_buses.iter().enumerate() {
            let at = format!("i2c_buses[{i}]");
            plain(&bus.id, || format!("{at}.id"))?;
            optional_identifier(&bus.peripheral, || format!("{at}.peripheral"))?;
            plain(&bus.domain, || format!("{at}.domain"))?;
            signal_identifiers(&bus.sda, &format!("{at}.sda"))?;
            signal_identifiers(&bus.scl, &format!("{at}.scl"))?;
        }
        for (i, bus) in self.spi_buses.iter().enumerate() {
            let at = format!("spi_buses[{i}]");
            plain(&bus.id, || format!("{at}.id"))?;
            optional_identifier(&bus.peripheral, || format!("{at}.peripheral"))?;
            plain(&bus.domain, || format!("{at}.domain"))?;
            signal_identifiers(&bus.sck, &format!("{at}.sck"))?;
            signal_identifiers(&bus.mosi, &format!("{at}.mosi"))?;
            signal_identifiers(&bus.miso, &format!("{at}.miso"))?;
        }
        for (i, device) in self.i2c_devices.iter().enumerate() {
            let at = format!("i2c_devices[{i}]");
            identifier(IdentifierKind::Reference, &device.reference, || {
                format!("{at}.reference")
            })?;
            plain(&device.part, || format!("{at}.part"))?;
            plain(&device.supply, || format!("{at}.supply"))?;
            plain(&device.bus, || format!("{at}.bus"))?;
            declared_texts(&device.pin_capacitance, &at)?;
        }
        for (i, device) in self.spi_devices.iter().enumerate() {
            let at = format!("spi_devices[{i}]");
            identifier(IdentifierKind::Reference, &device.reference, || {
                format!("{at}.reference")
            })?;
            plain(&device.part, || format!("{at}.part"))?;
            plain(&device.supply, || format!("{at}.supply"))?;
            plain(&device.bus, || format!("{at}.bus"))?;
            signal_identifiers(&device.cs, &format!("{at}.cs"))?;
            declared_texts(&device.pin_capacitance, &at)?;
        }
        for (i, net) in self.net_constraints.iter().enumerate() {
            plain(&net.net_name, || format!("net_constraints[{i}].net_name"))?;
        }
        for (i, layer) in self.board.layers.iter().enumerate() {
            optional_identifier(&layer.net, || format!("board.layers[{i}].net"))?;
        }
        for (i, m) in self.motor_outputs.iter().enumerate() {
            plain(&m.label, || format!("motor_outputs[{i}].label"))?;
            optional_identifier(&m.pin, || format!("motor_outputs[{i}].pin"))?;
            if let Some(motor::Esc::NoEsc(n)) = &m.esc {
                board::source(&n.source, &format!("motor_outputs[{i}].esc.none.source"))?;
            }
        }
        for (i, p) in self.serial_ports.iter().enumerate() {
            let at = format!("serial_ports[{i}]");
            plain(&p.label, || format!("{at}.label"))?;
            optional_identifier(&p.peripheral, || format!("{at}.peripheral"))?;
            signal_identifiers(&p.tx, &format!("{at}.tx"))?;
            signal_identifiers(&p.rx, &format!("{at}.rx"))?;
        }
        connectors::identifiers(self)?;
        pin_devices::identifiers(self)?;
        for (i, t) in self.test_points.iter().enumerate() {
            plain(&t.net, || format!("test_points[{i}].net"))?;
            identifier(IdentifierKind::Reference, &t.reference, || {
                format!("test_points[{i}].reference")
            })?;
        }
        Ok(())
    }
}

/// Step 13b: `pwm` present exactly for PWM, a rate above 0, a pulse range
/// from at least 1 µs to above it, and its longest pulse inside the period.
fn motor_shape(m: &MotorOutput) -> Result<(), IrValidationError> {
    use IrValidationError as E;

    let id = || m.label.clone();
    match (m.protocol, &m.pwm) {
        (MotorProtocol::Pwm, Some(p)) => {
            if p.rate_hz == 0 {
                return Err(E::ZeroMotorRate { id: id() });
            }
            if p.idle == motor::Idle::PulseUs(0) {
                return Err(E::Pwm(motor::PwmRefusal::IdleSpelledTwice {
                    output: id(),
                    pulse_us: 0,
                    ccr: "0",
                    level: "low",
                }));
            }
            if p.pulse_min_us == 0 || p.pulse_min_us >= p.pulse_max_us {
                return Err(E::PulseRange {
                    id: id(),
                    min_us: p.pulse_min_us,
                    max_us: p.pulse_max_us,
                });
            }
            if u64::from(p.pulse_max_us) * u64::from(p.rate_hz) >= 1_000_000 {
                return Err(E::PulseExceedsPeriod {
                    id: id(),
                    max_us: p.pulse_max_us,
                    rate_hz: p.rate_hz,
                });
            }
            Ok(())
        }
        (MotorProtocol::Dshot300 | MotorProtocol::Dshot600, None) => Ok(()),
        _ => Err(E::MotorTimingShape { id: id() }),
    }
}

/// Refuses two references equal in any case (`DuplicateReference`), and a
/// reference whose lowercased name plus `_memory`, the name the emitter gives
/// a modelled flash's array, is a reserved name or another reference
/// (`AET-WO-4200` D7). A reference equal to an emitted bus name (`spi1mux`) is
/// in [`RESERVED_REFERENCES`], refused by the identifier rule.
pub(crate) fn references_distinct<'r>(
    all: impl Iterator<Item = &'r str>,
) -> Result<(), IrValidationError> {
    let mut references: BTreeMap<String, &str> = BTreeMap::new();
    for reference in all {
        if let Some(first) = references.insert(reference.to_lowercase(), reference) {
            return Err(IrValidationError::DuplicateReference {
                id: reference.to_owned(),
                first: first.to_owned(),
            });
        }
    }
    for (lowered, reference) in &references {
        let memory = format!("{lowered}{MEMORY_SUFFIX}");
        let with = match references.get(&memory) {
            Some(other) => Some((*other).to_owned()),
            None => RESERVED_REFERENCES
                .binary_search(&memory.as_str())
                .is_ok()
                .then_some(memory),
        };
        if let Some(with) = with {
            return Err(IrValidationError::ReferenceCollision {
                id: (*reference).to_owned(),
                with,
            });
        }
    }
    Ok(())
}

/// The parts table's rows, in its order, as [`IrValidationError::PartWithoutRow`]
/// names them. That refusal is built only from a table that loaded, so the
/// second arm prints the load's error rather than an empty list.
fn table_parts() -> String {
    match parts::table() {
        Ok(t) => {
            let names: Vec<&str> = t.parts().iter().map(|p| p.part.as_str()).collect();
            names.join(", ")
        }
        Err(e) => format!("none, as the parts table does not load: {e}"),
    }
}

/// A device's part entry, or `None` for a part the table does not hold; the
/// bare `W25Q64JV` is refused (`AET-WO-4300c` F11).
fn part_entry<'t>(
    table: &'t parts::PartsTable,
    reference: &str,
    part: &str,
) -> Result<Option<&'t parts::Part>, IrValidationError> {
    if part.eq_ignore_ascii_case(parts::BARE_W25Q64JV) {
        return Err(IrValidationError::AmbiguousPartName {
            id: reference.to_owned(),
            part: part.to_owned(),
        });
    }
    if let Some(p) = table.get(part) {
        return Ok(Some(p));
    }
    // A table part in another case would skip every limit (adversarial 3).
    if let Some(p) = table
        .parts()
        .iter()
        .find(|p| p.part.eq_ignore_ascii_case(part))
    {
        return Err(IrValidationError::PartNameCase {
            id: reference.to_owned(),
            part: part.to_owned(),
            table: p.part.clone(),
        });
    }
    Ok(None)
}

/// A citation as a refusal names it.
fn cite(c: &parts::Citation) -> String {
    format!("{} rev. {}, {}", c.document, c.revision, c.section)
}

fn wrong_bus(reference: &str, p: &parts::Part, bus_kind: &'static str) -> IrValidationError {
    IrValidationError::PartOnWrongBus {
        id: reference.to_owned(),
        part: p.part.clone(),
        bus_kind,
        part_kind: match p.bus.kind() {
            parts::BusKind::I2c => "I2C",
            parts::BusKind::Spi => "SPI",
        },
        citation: cite(&p.citations.bus),
    }
}

/// The part is an I2C one and offers `address`.
fn check_i2c_address(
    reference: &str,
    p: &parts::Part,
    address: u8,
) -> Result<(), IrValidationError> {
    let parts::Bus::I2c { addresses } = &p.bus else {
        return Err(wrong_bus(reference, p, "I2C"));
    };
    if addresses.contains(&address) {
        return Ok(());
    }
    Err(IrValidationError::I2cAddressNotOffered {
        id: reference.to_owned(),
        part: p.part.clone(),
        address,
        offered: addresses
            .iter()
            .map(|a| format!("{a:#04X}"))
            .collect::<Vec<_>>()
            .join(", "),
        citation: cite(&p.citations.bus),
    })
}

/// The part is an SPI one and its clock limit is not below `speed_hz`.
fn check_spi_device(
    reference: &str,
    p: &parts::Part,
    speed_hz: u32,
) -> Result<(), IrValidationError> {
    let parts::Bus::Spi {
        modes,
        max_clock_hz,
    } = &p.bus
    else {
        return Err(wrong_bus(reference, p, "SPI"));
    };
    if speed_hz > *max_clock_hz {
        return Err(IrValidationError::SpiClockAbovePart {
            id: reference.to_owned(),
            part: p.part.clone(),
            speed_hz,
            max_clock_hz: *max_clock_hz,
            citation: cite(&p.citations.bus),
        });
    }
    // The row's modes against the one the generated firmware drives
    // (`AET-WO-1207` §1.5): the table refuses an empty list, a mode above 3
    // and a mode listed twice, so this is the only judgement left.
    if !modes.contains(&parts::DRIVEN_SPI_MODE) {
        return Err(IrValidationError::SpiModeNotDriven {
            id: reference.to_owned(),
            part: p.part.clone(),
            modes: modes.clone(),
            driven: parts::DRIVEN_SPI_MODE,
            citation: cite(&p.citations.bus),
        });
    }
    Ok(())
}

/// What the emitter appends to a device's lowercased reference to name the
/// memory array a modelled flash keeps (`AET-WO-4200` D7).
pub const MEMORY_SUFFIX: &str = "_memory";

impl SolvedTopology {
    /// Refuses the first malformed value found: version, hash, identifiers (in
    /// `assignments` a net, then its `pin`, `peripheral`, `signal`; then in
    /// `gpio` a net, then its pin; then each `i2c_ceilings` bus id; then in
    /// `analog_lines` a net, then its bus; keys in sorted order), then
    /// alternate functions, then ceilings, then a net in both maps, then a pin
    /// used twice across both, then the analog lines: each key a solved I2C
    /// line whose bus has a ceiling, no zero value, one bus per peripheral, and
    /// every solved I2C line present.
    pub fn validate(&self) -> Result<(), IrValidationError> {
        use IrValidationError as E;

        if self.schema_version != SOLVED_SCHEMA_VERSION {
            return Err(E::UnsupportedSchemaVersion {
                version: self.schema_version,
                expected: SOLVED_SCHEMA_VERSION,
            });
        }
        let hash = self.specification_sha256.as_bytes();
        if hash.len() != 64 || !hash.iter().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return Err(E::MalformedSpecificationHash);
        }
        for (net, a) in &self.assignments {
            plain(net, || format!("assignments.{net}"))?;
            plain(&a.pin, || format!("assignments.{net}.pin"))?;
            plain(&a.peripheral, || format!("assignments.{net}.peripheral"))?;
            plain(&a.signal, || format!("assignments.{net}.signal"))?;
        }
        for (net, g) in &self.gpio {
            plain(net, || format!("gpio.{net}"))?;
            plain(&g.pin, || format!("gpio.{net}.pin"))?;
        }
        for (net, a) in &self.adc_inputs {
            plain(net, || format!("adc_inputs.{net}"))?;
            plain(&a.pin, || format!("adc_inputs.{net}.pin"))?;
            plain(&a.peripheral, || format!("adc_inputs.{net}.peripheral"))?;
            plain(&a.channel, || format!("adc_inputs.{net}.channel"))?;
        }
        for (reference, d) in &self.pin_devices {
            identifier(IdentifierKind::Reference, reference, || {
                format!("pin_devices.{reference}")
            })?;
            plain(&d.part, || format!("pin_devices.{reference}.part"))?;
        }
        for bus in self.i2c_ceilings.keys() {
            plain(bus, || format!("i2c_ceilings.{bus}"))?;
        }
        for bus in self.i2c_speeds.keys() {
            plain(bus, || format!("i2c_speeds.{bus}"))?;
        }
        for (net, line) in &self.analog_lines {
            plain(net, || format!("analog_lines.{net}"))?;
            plain(&line.bus, || format!("analog_lines.{net}.bus"))?;
        }
        for (reference, d) in &self.i2c_devices {
            identifier(IdentifierKind::Reference, reference, || {
                format!("i2c_devices.{reference}")
            })?;
            plain(&d.part, || format!("i2c_devices.{reference}.part"))?;
            plain(&d.bus, || format!("i2c_devices.{reference}.bus"))?;
            plain(&d.peripheral, || {
                format!("i2c_devices.{reference}.peripheral")
            })?;
        }
        for (reference, d) in &self.spi_devices {
            identifier(IdentifierKind::Reference, reference, || {
                format!("spi_devices.{reference}")
            })?;
            plain(&d.part, || format!("spi_devices.{reference}.part"))?;
            plain(&d.bus, || format!("spi_devices.{reference}.bus"))?;
            plain(&d.peripheral, || {
                format!("spi_devices.{reference}.peripheral")
            })?;
            plain(&d.cs, || format!("spi_devices.{reference}.cs"))?;
        }
        references_distinct(
            self.i2c_devices
                .keys()
                .chain(self.spi_devices.keys())
                .map(String::as_str),
        )?;
        for net in self.motor_outputs.keys() {
            plain(net, || format!("motor_outputs.{net}"))?;
        }
        for (label, p) in &self.serial_ports {
            plain(label, || format!("serial_ports.{label}"))?;
            plain(&p.peripheral, || format!("serial_ports.{label}.peripheral"))?;
            plain(&p.tx, || format!("serial_ports.{label}.tx"))?;
            plain(&p.rx, || format!("serial_ports.{label}.rx"))?;
        }
        for p in self.spi_controllers.keys() {
            plain(p, || format!("spi_controllers.{p}"))?;
        }
        for (p, t) in &self.i2c_timings {
            plain(p, || format!("i2c_timings.{p}"))?;
            for mode in t.timingr.keys() {
                plain(mode, || format!("i2c_timings.{p}.timingr.{mode}"))?;
            }
        }
        plain(&self.clock.source, || "clock.source".into())?;
        plain(&self.mcu, || "mcu".into())?;
        for net in self.pin_speeds.keys() {
            plain(net, || format!("pin_speeds.{net}"))?;
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
            if !I2C_SPEEDS_HZ.contains(hz) {
                return Err(E::InvalidCeiling { id: bus.clone() });
            }
        }
        if let Some(net) = self.gpio.keys().find(|n| self.assignments.contains_key(*n)) {
            return Err(E::NetAssignedTwice { id: net.clone() });
        }
        pin_devices::validate_solved(self)?;
        let mut pins = BTreeSet::new();
        let all = self
            .assignments
            .values()
            .map(|a| &a.pin)
            .chain(self.gpio.values().map(|g| &g.pin))
            .chain(self.adc_inputs.values().map(|a| &a.pin));
        for pin in all {
            if !pins.insert(pin.as_str()) {
                return Err(E::DuplicatePin { id: pin.clone() });
            }
        }
        self.validate_analog_lines()?;
        self.validate_i2c_devices()?;
        self.validate_spi_devices()?;
        self.validate_spi_lines()?;
        self.validate_motor_outputs()?;
        serial::validate_solved(self)?;
        self.validate_controllers()?;
        // The part limits again, from what the topology carries, so a bus
        // sped up or a device moved after `validate` is refused here too
        // (`AET-WO-4300c` §4.2, `RUL-16`).
        let table = parts::table()?;
        for (reference, d) in &self.i2c_devices {
            if let Some(p) = part_entry(table, reference, &d.part)? {
                check_i2c_address(reference, p, d.address)?;
            }
        }
        for (reference, d) in &self.spi_devices {
            if let Some(p) = part_entry(table, reference, &d.part)? {
                // `validate_spi_lines` gave every device's bus one rate.
                let rate_hz = self
                    .spi_lines
                    .values()
                    .find(|l| l.bus == d.bus)
                    .map_or(0, |l| l.rate_hz);
                check_spi_device(reference, p, rate_hz)?;
            }
        }
        // Last (`AET-WO-4400b2` F4): each stored speed re-derived, after the
        // part limits, so a bus past its part is named by the part first.
        self.validate_pin_speeds()
    }

    /// Sets `spi_controllers` and `i2c_timings` to what `solve` derives for
    /// the SPI and I2C peripherals of `assignments`, from `clock` and, for
    /// SPI, the lowest `rate_hz` of each controller's lines in `spi_lines`;
    /// a peripheral it derives nothing for is left out, which `validate`
    /// then refuses. It sets `pin_speeds` by `solve`'s own rule
    /// (`AET-WO-4400b` D2), leaving out a pin no selectable speed serves,
    /// which `validate` refuses too. For a topology edited by hand, as a test
    /// builds one (`AET-WO-4230`); `solve` fills all three itself.
    pub fn derive_controllers(&mut self) {
        (self.spi_controllers, self.i2c_timings) = self.derived_controllers();
        let speeds: BTreeMap<String, u8> = solve::solved_pins(self)
            .filter_map(|(net, pin)| Some((net.clone(), solve::pin_speed(self, net, pin).ok()?)))
            .collect();
        self.pin_speeds = speeds;
    }

    /// What [`SolvedTopology::derive_controllers`] sets.
    fn derived_controllers(
        &self,
    ) -> (
        BTreeMap<String, SolvedSpiController>,
        BTreeMap<String, SolvedI2cTiming>,
    ) {
        let mut spi = BTreeMap::new();
        let mut i2c = BTreeMap::new();
        for a in self.assignments.values() {
            let p = a.peripheral.as_str();
            if p.starts_with("SPI") {
                let declared = self
                    .spi_lines
                    .iter()
                    .filter(|(net, _)| {
                        self.assignments
                            .get(*net)
                            .is_some_and(|a| a.peripheral == p)
                    })
                    .map(|(_, l)| l.rate_hz)
                    .min();
                if let Some(c) =
                    declared.and_then(|d| solve::spi_controller(p, p, d, &self.clock).ok())
                {
                    spi.insert(p.to_owned(), c);
                }
            } else if p.starts_with("I2C")
                && let Some(t) = solve::i2c_controller("", &self.clock, p).ok()
            {
                i2c.insert(p.to_owned(), t);
            }
        }
        (spi, i2c)
    }

    /// `bor_level` is 0 to 3; `spi_controllers` and `i2c_timings` hold
    /// exactly the SPI and I2C peripherals of `assignments`, each with what
    /// `solve` derives from `clock` and, for SPI, the lowest `rate_hz` of the
    /// buses on it in `spi_lines` (`AET-WO-4230`). It does not judge those
    /// values for silicon: that is [`silicon`]'s rules (`RUL-20`).
    /// `pin_speeds` is keyed by exactly the nets of `assignments` and `gpio`,
    /// each `0` or `1` (`AET-WO-4400b` D2). The first stray or missing net,
    /// in sorted order, is named.
    fn validate_pin_speeds(&self) -> Result<(), IrValidationError> {
        let pins: BTreeSet<&String> = self.assignments.keys().chain(self.gpio.keys()).collect();
        let speeds: BTreeSet<&String> = self.pin_speeds.keys().collect();
        if let Some(net) = pins.symmetric_difference(&speeds).next() {
            return Err(IrValidationError::PinSpeedMismatch { id: (*net).clone() });
        }
        if let Some((net, _)) = self.pin_speeds.iter().find(|(_, v)| **v > 1) {
            return Err(IrValidationError::PinSpeedMismatch { id: net.clone() });
        }
        // F4 (`AET-WO-4400b2`): each stored speed is `solve`'s, re-derived as
        // ceilings and controllers are, so no reader takes a hand-edited one.
        for (net, pin) in solve::solved_pins(self) {
            let solved = match solve::pin_speed(self, net, pin) {
                Ok(v) => v,
                Err(solve::SolveError::PinSpeed { refusal, .. }) => {
                    return Err(IrValidationError::PinSpeedUnserved {
                        id: net.clone(),
                        pin: pin.clone(),
                        refusal,
                    });
                }
                Err(solve::SolveError::InvalidSpecification(e)) => return Err(e),
                Err(other) => unreachable!("pin_speed refuses only by speed or table: {other}"),
            };
            let stored = self.pin_speeds[net];
            if stored != solved {
                return Err(IrValidationError::PinSpeedNotSolved {
                    id: net.clone(),
                    stored,
                    solved,
                });
            }
        }
        Ok(())
    }

    fn validate_controllers(&self) -> Result<(), IrValidationError> {
        use IrValidationError as E;

        if self.bor_level > MAX_BOR_LEVEL {
            return Err(E::BorLevelOutOfRange {
                value: self.bor_level,
            });
        }
        if let Some(c) = &self.foreign_clock {
            foreign_clock_values(c)?;
        }
        let on = |prefix: &str| -> BTreeSet<&str> {
            self.assignments
                .values()
                .map(|a| a.peripheral.as_str())
                .filter(|p| p.starts_with(prefix))
                .collect()
        };
        let (want_spi, want_i2c) = self.derived_controllers();
        // Every SPI peripheral of `assignments` has an entry, no other key
        // has one, and each is what `solve` derives.
        let spi_keys: BTreeSet<&str> = self.spi_controllers.keys().map(String::as_str).collect();
        let spi = on("SPI");
        if let Some(p) = spi
            .symmetric_difference(&spi_keys)
            .next()
            .copied()
            .or_else(|| {
                let (have, want) = (&self.spi_controllers, &want_spi);
                have.keys()
                    .map(String::as_str)
                    .find(|p| have.get(*p) != want.get(*p))
            })
        {
            return Err(E::SpiControllerMismatch { id: p.to_owned() });
        }
        let i2c_keys: BTreeSet<&str> = self.i2c_timings.keys().map(String::as_str).collect();
        let i2c = on("I2C");
        if let Some(p) = i2c
            .symmetric_difference(&i2c_keys)
            .next()
            .copied()
            .or_else(|| {
                let (have, want) = (&self.i2c_timings, &want_i2c);
                have.keys()
                    .map(String::as_str)
                    .find(|p| have.get(*p) != want.get(*p))
            })
        {
            return Err(E::I2cTimingMismatch { id: p.to_owned() });
        }
        Ok(())
    }

    /// The clock is the reset clock; `motor_outputs` holds exactly the nets
    /// solved to a timer channel, each `TIMn_CHk` of a timer with channels,
    /// each channel once, with indices exactly `0..n`, `pwm` present exactly
    /// for PWM, `PSC` and `ARR` within 16 bits and `CCR` from 1 to `ARR - 1`; and
    /// the outputs of one timer share its protocol, PWM rate, `PSC` and `ARR`.
    /// It does not judge the values against the declared timing: that is the
    /// static check, [`motor::check_silicon`] (`4300` D3).
    fn validate_motor_outputs(&self) -> Result<(), IrValidationError> {
        use IrValidationError as E;

        // The voltage-scaling rule first (`AET-WO-4240v` §1.3), so a tree
        // over a limit is refused by the clock's name; then the tree must be
        // the one this engine generates, the reset RCC tree at Scale 2.
        let rule = vos::judge_generated(self);
        if let Some(c) = rule.clocks.iter().find(|c| c.holds == Some(false)) {
            return Err(E::VosLimitExceeded {
                clock: c.clock.clone(),
                fastest_hz: c.fastest_hz.unwrap_or(0),
                limit_hz: c.limit_hz.unwrap_or(0),
                vos: rule.vos,
                cite: c.cite.unwrap_or("").to_owned(),
            });
        }
        if !rule.flash.holds {
            return Err(E::FlashWaitStatesBelowTable {
                vos: rule.vos,
                hclk_fastest_hz: rule.flash.hclk_fastest_hz,
            });
        }
        // A kernel the rule cannot read is refused by name by the
        // controller checks that follow (`SpiControllerMismatch`, the
        // silicon rule), never passed: `check_silicon` holds the verdict.
        // The tree is the reset one, or an HSE tree the clock rule holds
        // (`AET-WO-4240a` §1.5).
        if self.clock.hse.is_none() && self.clock != SolvedClock::reset() {
            return Err(E::UnsupportedClock);
        }
        clock::judge_tree(&self.clock)?;
        // The copy (`AET-WO-4240a` F3, `4240b` §1.3): `generate` re-solves from
        // the specification, and every path that reads only a topology
        // (`generate hwdef --topology`, `check pinout`) comes through here. It
        // re-solves the clock from the declaration the topology carries and
        // requires the topology's every derived field to equal it. The judge
        // above shares no word-choice code with the solver (`4240a` A6, ruled
        // (a)); this is the solver's own check of its copy. What it cannot see:
        // a declaration edited together with every field derived from it, which
        // is another specification, bound only by `specification_sha256`.
        if let Some(h) = &self.clock.hse {
            let again = solve::hse_clock(&h.declared)?;
            if again != self.clock {
                let ours = again.hse.as_ref();
                let field = if ours.map(clock::SolvedHse::budget) != Some(h.budget()) {
                    "the budget"
                } else if ours.map(|o| [o.pllckselr, o.pllcfgr, o.pll1divr])
                    != Some([h.pllckselr, h.pllcfgr, h.pll1divr])
                {
                    "the PLL words"
                } else {
                    "a clock field"
                };
                return Err(E::Hse(clock::HseRefusal::HseCopyNotSolved { field }));
            }
        }
        let mismatch = |id: &str| E::MotorOutputMismatch { id: id.to_owned() };
        for (net, a) in &self.assignments {
            let timer = a.peripheral.starts_with("TIM");
            if timer != self.motor_outputs.contains_key(net) {
                return Err(mismatch(net));
            }
        }
        let mut seen = BTreeSet::new();
        let mut channels = BTreeSet::new();
        let mut timers: BTreeMap<u32, (motor::TimerKey, u32, u32)> = BTreeMap::new();
        for (net, m) in &self.motor_outputs {
            let a = self.assignments.get(net).ok_or_else(|| mismatch(net))?;
            let Some((timer, channel)) = motor::timer_channel(&a.signal) else {
                return Err(mismatch(net));
            };
            let shaped = matches!(
                (m.protocol, &m.pwm),
                (MotorProtocol::Pwm, Some(_))
                    | (MotorProtocol::Dshot300 | MotorProtocol::Dshot600, None)
            );
            let in_range = usize::try_from(m.index).is_ok_and(|i| i < self.motor_outputs.len());
            if a.peripheral != format!("TIM{timer}")
                || !shaped
                || !in_range
                || !seen.insert(m.index)
                || !channels.insert((timer, channel))
                || m.psc > 0xFFFF
                || m.arr == 0
                || m.arr > 0xFFFF
                || !ccr_shaped(m)
            {
                return Err(mismatch(net));
            }
            let key = (
                (
                    m.protocol,
                    m.pwm.as_ref().map(|p| (p.rate_hz, p.resolution_ps)),
                ),
                m.psc,
                m.arr,
            );
            if *timers.entry(timer).or_insert(key) != key {
                return Err(E::TimerShared {
                    id: format!("TIM{timer}"),
                });
            }
        }
        Ok(())
    }

    /// The motor outputs in the specification's order, with each one's
    /// timer and channel (`TIMn_CHk`) and its pin's assignment.
    pub fn motor_outputs_in_order(&self) -> Vec<(&str, &SolvedMotorOutput, &PinAssignment)> {
        let mut out: Vec<_> = self
            .motor_outputs
            .iter()
            .filter_map(|(net, m)| self.assignments.get(net).map(|a| (net.as_str(), m, a)))
            .collect();
        out.sort_by_key(|(_, m, _)| m.index);
        out
    }

    /// The serial ports in the specification's order, with the assignments
    /// of their TX and RX nets.
    pub fn serial_ports_in_order(
        &self,
    ) -> Vec<(&str, &SolvedSerialPort, &PinAssignment, &PinAssignment)> {
        let mut out: Vec<_> = self
            .serial_ports
            .iter()
            .filter_map(|(label, p)| {
                let tx = self.assignments.get(&p.tx)?;
                let rx = self.assignments.get(&p.rx)?;
                Some((label.as_str(), p, tx, rx))
            })
            .collect();
        out.sort_by_key(|(_, p, _, _)| p.index);
        out
    }

    /// `spi_lines` holds exactly the solved SCK and MOSI signals, each on its
    /// own net's assignment, with no zero value; the lines of one bus share
    /// one peripheral and one rate; and every SPI device's bus has lines.
    fn validate_spi_lines(&self) -> Result<(), IrValidationError> {
        let mismatch = |id: &str| IrValidationError::SpiLineMismatch { id: id.to_owned() };
        for (net, a) in &self.assignments {
            let driven = a.peripheral.starts_with("SPI")
                && DRIVEN_SPI_SIGNALS
                    .iter()
                    .any(|s| a.signal == format!("{}_{s}", a.peripheral));
            if driven != self.spi_lines.contains_key(net) {
                return Err(mismatch(net));
            }
        }
        let mut buses: BTreeMap<&str, (&str, u32)> = BTreeMap::new();
        // ... and the other way: one label and one rate per peripheral, so a
        // relabelled line cannot give a peripheral two rates (adversarial 1).
        let mut peripherals: BTreeMap<&str, (&str, u32)> = BTreeMap::new();
        for (net, line) in &self.spi_lines {
            plain(&line.bus, || format!("spi_lines.{net}.bus"))?;
            let Some(a) = self.assignments.get(net) else {
                return Err(mismatch(net));
            };
            if a.signal != format!("{}_{}", a.peripheral, line.signal)
                || line.rate_hz == 0
                || line.capacitance_femtofarads == 0
                || line.rise_time_ns == 0
                || line.vdd_min_millivolts == 0
            {
                return Err(mismatch(net));
            }
            let first = *buses
                .entry(line.bus.as_str())
                .or_insert((a.peripheral.as_str(), line.rate_hz));
            let by_peripheral = *peripherals
                .entry(a.peripheral.as_str())
                .or_insert((line.bus.as_str(), line.rate_hz));
            if first != (a.peripheral.as_str(), line.rate_hz)
                || by_peripheral != (line.bus.as_str(), line.rate_hz)
            {
                return Err(mismatch(net));
            }
        }
        for (reference, d) in &self.spi_devices {
            if buses.get(d.bus.as_str()).map(|b| b.0) != Some(d.peripheral.as_str()) {
                return Err(mismatch(reference));
            }
        }
        Ok(())
    }

    /// Each SPI device is on a peripheral some solved signal is on, an `SPIn`;
    /// its chip select's net is a solved GPIO; and the indices are exactly
    /// `0..n`.
    fn validate_spi_devices(&self) -> Result<(), IrValidationError> {
        use IrValidationError as E;

        let mut seen = BTreeSet::new();
        for (reference, d) in &self.spi_devices {
            let solved = d.peripheral.starts_with("SPI")
                && self
                    .assignments
                    .values()
                    .any(|a| a.peripheral == d.peripheral);
            if !solved {
                return Err(E::SpiDevicePeripheral {
                    id: reference.clone(),
                });
            }
            if !self.gpio.contains_key(&d.cs) {
                return Err(E::SpiDeviceSelect {
                    id: reference.clone(),
                });
            }
            let in_range = usize::try_from(d.index).is_ok_and(|i| i < self.spi_devices.len());
            if !in_range || !seen.insert(d.index) {
                return Err(E::SpiDeviceIndex {
                    id: reference.clone(),
                });
            }
            // `AET-WO-4400d`: a copied orientation holds as the
            // specification's does: at right angles, and only on an IMU.
            if let Some(o) = d.orientation {
                if !o.perpendicular() {
                    return Err(E::ImuAxesNotPerpendicular {
                        reference: reference.clone(),
                        x: o.x.name().to_owned(),
                        z: o.z.name().to_owned(),
                    });
                }
                let imu = parts::table()?
                    .get(&d.part)
                    .and_then(|p| p.ardupilot.as_ref())
                    .is_some_and(|a| a.kind == parts::ArduPilotKind::Imu);
                if !imu {
                    return Err(E::OrientationOnNonImu {
                        reference: reference.clone(),
                        part: d.part.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Every device's (reference, part), I2C then SPI, each in the
    /// specification's order.
    pub fn parts_in_order(&self) -> impl Iterator<Item = (&str, &str)> {
        self.i2c_devices_in_order()
            .into_iter()
            .map(|(r, d)| (r, d.part.as_str()))
            .chain(
                self.spi_devices_in_order()
                    .into_iter()
                    .map(|(r, d)| (r, d.part.as_str())),
            )
    }

    /// [`lifecycle_banner`] for the solved devices.
    pub fn lifecycle_banner(&self) -> Result<Vec<String>, IrValidationError> {
        Ok(lifecycle_banner(parts::table()?, self.parts_in_order()))
    }

    /// The SPI devices in the specification's order, as
    /// [`SolvedTopology::i2c_devices_in_order`] orders the I2C ones.
    pub fn spi_devices_in_order(&self) -> Vec<(&str, &SolvedSpiDevice)> {
        let mut out: Vec<_> = self
            .spi_devices
            .iter()
            .map(|(r, d)| (r.as_str(), d))
            .collect();
        out.sort_by_key(|(_, d)| d.index);
        out
    }

    /// Each device's address is one a specification may declare (0x08..=0x77,
    /// inside 7 bits), its bus has a ceiling, its peripheral is the one the
    /// bus's analog lines were solved to, and the indices are exactly `0..n`.
    fn validate_i2c_devices(&self) -> Result<(), IrValidationError> {
        use IrValidationError as E;

        let mut seen = BTreeSet::new();
        for (reference, d) in &self.i2c_devices {
            if !(0x08..=0x77).contains(&d.address) {
                return Err(E::I2cAddressOutOfRange {
                    id: reference.clone(),
                    value: d.address,
                });
            }
            if !self.i2c_ceilings.contains_key(&d.bus) {
                return Err(E::UnknownBus { id: d.bus.clone() });
            }
            let solved_to = self
                .analog_lines
                .iter()
                .find(|(_, line)| line.bus == d.bus)
                .and_then(|(net, _)| self.assignments.get(net));
            if solved_to.map(|a| a.peripheral.as_str()) != Some(d.peripheral.as_str()) {
                return Err(E::I2cDevicePeripheral {
                    id: reference.clone(),
                });
            }
            let in_range = usize::try_from(d.index).is_ok_and(|i| i < self.i2c_devices.len());
            if !in_range || !seen.insert(d.index) {
                return Err(E::I2cDeviceIndex {
                    id: reference.clone(),
                });
            }
        }
        Ok(())
    }

    /// The devices in the specification's order: `(reference, device)` by
    /// `index`. Every consumer reads this, never the map, whose order is the
    /// references' (`AET-WO-2203`). On a topology that passed
    /// [`SolvedTopology::validate`] the indices are exactly `0..n`.
    pub fn i2c_devices_in_order(&self) -> Vec<(&str, &SolvedI2cDevice)> {
        let mut out: Vec<_> = self
            .i2c_devices
            .iter()
            .map(|(r, d)| (r.as_str(), d))
            .collect();
        out.sort_by_key(|(_, d)| d.index);
        out
    }

    fn validate_analog_lines(&self) -> Result<(), IrValidationError> {
        use IrValidationError as E;

        // One bus per peripheral, and one peripheral per bus: the fault
        // decision is keyed by bus, and the platform omits a peripheral's
        // devices (adversarial A2).
        let mut bus_of: BTreeMap<&str, &str> = BTreeMap::new();
        let mut peripheral_of: BTreeMap<&str, &str> = BTreeMap::new();
        for (net, line) in &self.analog_lines {
            let id = || net.clone();
            let assignment = self.assignments.get(net).filter(|a| is_i2c_line(a));
            let Some(assignment) = assignment else {
                return Err(E::AnalogLineNotI2c { id: id() });
            };
            if !self.i2c_ceilings.contains_key(&line.bus) {
                return Err(E::UnknownBus {
                    id: line.bus.clone(),
                });
            }
            if line.pull_up_ohms == 0 {
                return Err(E::ZeroPullUp { id: id() });
            }
            if line.capacitance_femtofarads == 0 {
                return Err(E::ZeroCapacitance { id: id() });
            }
            if line.ceiling_ns == 0 {
                return Err(E::ZeroRiseTime { id: id() });
            }
            if line.domain_millivolts == 0 {
                return Err(E::ZeroNominalVoltage { id: id() });
            }
            let bus = *bus_of
                .entry(assignment.peripheral.as_str())
                .or_insert(line.bus.as_str());
            let peripheral = *peripheral_of
                .entry(line.bus.as_str())
                .or_insert(assignment.peripheral.as_str());
            if bus != line.bus || peripheral != assignment.peripheral {
                return Err(E::AnalogBusMismatch { id: id() });
            }
        }
        let missing = self
            .assignments
            .iter()
            .find(|(net, a)| is_i2c_line(a) && !self.analog_lines.contains_key(*net));
        if let Some((net, _)) = missing {
            return Err(E::MissingAnalogLine { id: net.clone() });
        }
        // A hand-written topology holds only the ceiling `solve` derives
        // (`AET-WO-1204`): the mapping of the smallest rise-time bound among
        // the bus's own lines, and no ceiling for a bus with no line.
        for (bus, &ceiling_hz) in &self.i2c_ceilings {
            // `solve` takes the smaller bound of both lines, so a bus holds
            // exactly its SDA and its SCL: a half-bus would derive from one.
            let mut sda_scl = [0usize; 2];
            for (net, _) in self.analog_lines.iter().filter(|(_, l)| &l.bus == bus) {
                match self
                    .assignments
                    .get(net)
                    .map(|a| a.signal.rsplit('_').next())
                {
                    Some(Some("SDA")) => sda_scl[0] += 1,
                    Some(Some("SCL")) => sda_scl[1] += 1,
                    _ => {}
                }
            }
            if sda_scl != [1, 1] {
                return Err(E::CeilingNotDerived {
                    bus: bus.clone(),
                    ceiling_hz,
                    derived_hz: None,
                });
            }
            // The worse line's rise time bounds the bus (`AET-WO-1206` §1.7).
            let slowest = self
                .analog_lines
                .values()
                .filter(|line| &line.bus == bus)
                .map(|line| line.ceiling_ns)
                .max();
            let derived_hz = slowest.and_then(electrical::ceiling_hz);
            if derived_hz != Some(ceiling_hz) {
                return Err(E::CeilingNotDerived {
                    bus: bus.clone(),
                    ceiling_hz,
                    derived_hz,
                });
            }
        }
        self.validate_i2c_speeds()
    }

    /// The declared speeds (`AET-WO-1205`): the same buses as the ceilings,
    /// each a speed a specification may declare and at most its ceiling, then
    /// each line of the bus held to the pull-up floor and capacitance limit
    /// `solve` applies at that speed (`AET-WO-1203`).
    fn validate_i2c_speeds(&self) -> Result<(), IrValidationError> {
        use IrValidationError as E;

        let stray = self
            .i2c_speeds
            .keys()
            .find(|b| !self.i2c_ceilings.contains_key(*b))
            .or_else(|| {
                self.i2c_ceilings
                    .keys()
                    .find(|b| !self.i2c_speeds.contains_key(*b))
            });
        if let Some(bus) = stray {
            return Err(E::SpeedBusMismatch { id: bus.clone() });
        }
        for (bus, &speed_hz) in &self.i2c_speeds {
            if !I2C_SPEEDS_HZ.contains(&speed_hz) {
                return Err(E::UnsupportedI2cSpeed {
                    id: bus.clone(),
                    value: speed_hz,
                });
            }
            let ceiling_hz = self.i2c_ceilings[bus];
            if speed_hz > ceiling_hz {
                return Err(E::BusTooFast {
                    bus: bus.clone(),
                    speed_hz,
                    ceiling_hz,
                });
            }
            let limit = electrical::i2c_bus_capacitance_limit_femtofarads(speed_hz);
            for (net, line) in self.analog_lines.iter().filter(|(_, l)| &l.bus == bus) {
                let floor = electrical::i2c_pull_up_floor_ohms(speed_hz, line.domain_millivolts);
                if line.pull_up_ohms < floor {
                    return Err(E::PullUpBelowSinkFloor {
                        bus: bus.clone(),
                        pull_up_ohms: line.pull_up_ohms,
                        floor_ohms: floor,
                    });
                }
                if line.capacitance_femtofarads > limit {
                    return Err(E::BusCapacitanceOverLimit {
                        net: net.clone(),
                        capacitance_femtofarads: line.capacitance_femtofarads,
                        limit_femtofarads: limit,
                    });
                }
            }
        }
        Ok(())
    }
}

/// An assignment to an I2C instance's SDA or SCL, as `solve` names them:
/// peripheral `I2C<n>`, signal `I2C<n>_SDA` or `I2C<n>_SCL`.
fn is_i2c_line(a: &PinAssignment) -> bool {
    let numbered = a
        .peripheral
        .strip_prefix("I2C")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    let line = a
        .signal
        .strip_prefix(a.peripheral.as_str())
        .is_some_and(|s| s == "_SDA" || s == "_SCL");
    numbered && line
}
