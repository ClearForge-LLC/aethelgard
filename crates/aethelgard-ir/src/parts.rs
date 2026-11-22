//! The parts table (`AET-WO-4200` D3, review B7): what each part is, as data.
//!
//! The table is `data/parts.json`, embedded in this crate and pinned by its
//! sha256 in a tier-1 test. One parser, [`parse_parts`], reads it and every
//! test fixture; it never panics, and refuses a malformed entry by naming the
//! part and the field (`INV-06`). The embedded table reaches a user only
//! through [`table`], whose error `validate` returns like any other refusal.
//!
//! Each entry holds the part's values: its bus (I2C addresses, or SPI modes
//! and clock), its identity read, its supply range, the largest capacitance of
//! any of its signal pins where the datasheet prints a maximum
//! (`AET-WO-4300c` D3), and the Renode model that stands in for it, or none. Each value group carries its datasheet citation:
//! the document, revision and section, and the document's sha256 when it is
//! held, or the URL it must be fetched from (`pending`), never a value from
//! memory. What a Renode model logs is not a part value: that stays code in
//! `aethelgard-twin`, matched on the model (`AET-WO-2204` D2). The generators
//! and the judge read the same entry (`INV-01`); neither takes an expected
//! value from the other (`INV-07`).
//!
//! Schema 3 (`AET-WO-4210`, ruled on PR #131) adds what a device's bring-up
//! needs beyond its identity: a part's [`Lifecycle`], cited to the vendor page
//! held (D1); whether it is a twin stand-in, never meant for a purchase; its
//! I/O range where the datasheet separates it from the supply (D7); and its
//! transaction [`Script`]: one configuration write and one data read after
//! the identity, each cited. A data read's expected value is the datasheet's
//! conversion of the stimulus the twin sets (D3), computed here from the
//! table alone, so the generator and the judge share it and neither takes it
//! from the other.
//!
//! Schema 4 (`AET-WO-4220`, ruled on PR #140) lets a script configure more
//! than one register, as D7 makes the ICM-42688-P's set its full scale, its
//! byte order and its enable, and wait a cited time after: a model of our own
//! samples on its own timer (D8), so its data is not ready the instant the
//! enable lands. A data word is two or three bytes, in either byte order (the
//! BMP581's are 24-bit, least significant byte first).

use std::collections::BTreeSet;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::IrValidationError;

/// The table as embedded: `data/parts.json`.
pub const PARTS_JSON: &str = include_str!("../data/parts.json");

/// The `schema_version` the parser reads.
/// Version 2 added `pin_capacitance` and named the flash for its ordering
/// code (`AET-WO-4300c` D3, F11). Version 3 made the identity's write a byte
/// list with a mask, and added `lifecycle`, `stand_in`, `io_millivolts` and
/// `script` (`AET-WO-4210`). Version 4 made `script.configure` a list of
/// writes with a `settle_us`, and gave a channel its `bytes` and `order`
/// (`AET-WO-4220`). Version 5 added `spi_output_delay_max_ns`, an SPI part's
/// output valid time, with its citation (`AET-WO-4300d2` K6). Version 6 gave
/// every script a structured `configure.start_up` (a cited maximum and/or
/// typical start-up time with the condition it is the start-up of, and its
/// `citations.start_up`), against which `settle_us` is checked at load
/// (`AET-WO-4230` §1.9, `SettleBelowStartUp`). Version 7 gave
/// `pin_capacitance` a required `typical_femtofarads`: the typical the
/// datasheet prints, under the row's `citations.pin_capacitance`, or `null`
/// (`AET-WO-4220b`, the architect's ruling on Q2, #192). It floors a
/// specification's declaration; it is never a maximum.
/// Version 8 added `ardupilot`, what ArduPilot `Copter-4.7.2` calls the
/// part, with the chip ID its driver accepts, which must be the part's own
/// identity (`AET-WO-4400b` D4; renumbered from 7 behind `4220b`'s floor).
/// Version 9 added `axes`, an IMU's sensing axes by its pins, and
/// `ardupilot.frame`, the frame an IMU driver passes its samples in
/// (`AET-WO-4400d`, ruled on PR #207), each required as a key, `null` where
/// it does not apply. Version 10 added a part's `reads`
/// (`AET-WO-4400e`). Version 11 adds `pin_parts`, the busless rows a
/// specification's `pin_devices` bind (`AET-WO-1211a`, `1211` Q6):
/// `bus: "none"` and `identity: null`, each pin's kind, pull and levels,
/// and each strapped mode's truth table ([`crate::pin_parts`]). Version 12
/// adds a pin part's `trip` and `bulk_unstated` (`AET-WO-1211e`), each
/// absent where the part has none. Version 13 requires a current output's
/// `output_max_basis`, `maximum` or `recommended_operating`: what kind of
/// figure `output_max_ua` is (`AET-WO-1216b`, `1216` D5).
pub const PARTS_SCHEMA_VERSION: u32 = 13;

/// A bus kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusKind {
    I2c,
    Spi,
}

/// The Renode 1.17.0 models a part may name, each with the buses it attaches
/// to. A name outside this set is refused; `aethelgard-twin` maps each to its
/// own model, exhaustively.
/// `Sensors.ICM42688P` and `Sensors.BMP581` are this project's own
/// (`models/renode/`, `AET-WO-4220`), each written for one bus.
pub const RENODE_MODELS: [(&str, &[BusKind]); 6] = [
    ("I2C.BME280", &[BusKind::I2c]),
    ("SPI.GenericSpiFlash", &[BusKind::Spi]),
    ("Sensors.BMP581", &[BusKind::I2c]),
    ("Sensors.ICM20948", &[BusKind::I2c, BusKind::Spi]),
    ("Sensors.ICM42688P", &[BusKind::Spi]),
    ("Sensors.ICP_101xx", &[BusKind::I2c]),
];

/// The most bytes a step writes: the ICP-10111's 16-bit commands are 2.
pub const MAX_STEP_WRITE: usize = 4;

/// The most bytes a data read takes, so one record's value fits 64 bits
/// (`AET-WO-4210` S2, F18's `n <= 9`); a channel is at least 2, so at most 4
/// channels.
pub const MAX_DATA_BYTES: u32 = 8;

/// The most configuration writes a script makes (`AET-WO-4220`).
pub const MAX_CONFIGURE: usize = 4;

/// The longest wait a script may state after its configuration, in
/// microseconds. The firmware's wait is sized for silicon (64 MHz, one cycle
/// an iteration), so Renode, at 100 MIPS and a few instructions an
/// iteration, takes longer; the boot's 100 ms must hold it with the rest of
/// the script (`AET-WO-4220`).
pub const MAX_SETTLE_US: u32 = 40_000;

/// The highest SPI clock an entry may state: no part in the table comes near
/// it, and anything above is a typo.
pub const MAX_SPI_CLOCK_HZ: u32 = 1_000_000_000;

/// The highest supply an entry may state, in millivolts.
pub const MAX_SUPPLY_MILLIVOLTS: u32 = 60_000;

/// The highest pin capacitance an entry may state, in femtofarads: no
/// part's signal pin comes near 100 pF, and anything above is a typo.
pub const MAX_PIN_FEMTOFARADS: u64 = 100_000;

/// The largest SPI output delay the table accepts, in nanoseconds: a larger
/// value is a typing error, not a datasheet.
pub const MAX_SPI_OUTPUT_DELAY_NS: u32 = 1_000;

/// The SPI command that reads a JEDEC identity (`9Fh`), the identity's width,
/// and the capacity codes (`2^code` bytes) a `SPI.GenericSpiFlash` is built
/// from: the shape the twin's emitter requires (`aethelgard-twin`
/// `FlashIdentity`), checked here so a malformed one never reaches
/// `generate` (`AET-WO-4300c` F20).
pub const JEDEC_READ: u8 = 0x9F;
pub const JEDEC_WIDTH: u8 = 3;
pub const FLASH_CAPACITY_CODES: std::ops::RangeInclusive<u8> = 0x10..=0x19;

/// The name the W25Q64JV had before its ordering code was part of it
/// (`AET-WO-4300c` F11). Its JEDEC identity depends on that code, so a
/// specification naming it is refused, naming both.
pub const BARE_W25Q64JV: &str = "W25Q64JV";

/// The one SPI mode (CPOL × 2 + CPHA) the generated firmware drives on every
/// controller: `aethelgard-svd-hal`'s `spi_code` (`board.rs`) writes `CFG2`
/// with `MASTER`, `SSM` and `AFCNTR` only, so `CPOL` and `CPHA` are 0
/// (RM0433 Rev 9 §50.11.4), whatever the solve or the part. `validate`
/// refuses a device whose row's `modes` lack it (`SpiModeNotDriven`,
/// `AET-WO-1207` §1.5); a test in `aethelgard-svd-hal` pins it to the word
/// the generator writes.
pub const DRIVEN_SPI_MODE: u8 = 0;

/// How a part sits on its bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bus {
    /// The 7-bit addresses its straps allow, each from 0x08 to 0x77.
    I2c { addresses: Vec<u8> },
    /// The SPI modes it accepts (CPOL × 2 + CPHA), and its highest clock.
    Spi { modes: Vec<u8>, max_clock_hz: u32 },
}

impl Bus {
    pub fn kind(&self) -> BusKind {
        match self {
            Bus::I2c { .. } => BusKind::I2c,
            Bus::Spi { .. } => BusKind::Spi,
        }
    }
}

/// One identity read: the bytes written first (an I2C register or a 16-bit
/// command, or an SPI command with the read bit as the part needs it), then
/// `width` bytes (1 to 4) read back, which a present part answers with
/// `value` under `mask`, the first byte on the wire as the most significant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub write: Vec<u8>,
    pub width: u8,
    pub value: u32,
    /// The bits the identity fixes; all of them unless the table says less
    /// (the ICP-10111's bits 15:6 are unspecified, `AET-WO-4210` F6).
    pub mask: u32,
    /// `false` when no held datasheet states the value: allowed only for a
    /// twin stand-in, whose identity is then cited to the model's source and
    /// a driver (`AET-WO-4210` D2).
    pub datasheet_value: bool,
}

/// A part's production status on its vendor's page (`AET-WO-4210` D1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    Active,
    /// Not recommended for new designs.
    Nrnd,
    /// End of life: refused on the fabrication path, by name.
    Eol,
}

impl Lifecycle {
    pub fn name(self) -> &'static str {
        match self {
            Lifecycle::Active => "active",
            Lifecycle::Nrnd => "nrnd",
            Lifecycle::Eol => "eol",
        }
    }
}

/// A physical quantity the twin can set on a model before a run (the
/// stimulus, `AET-WO-4210` D3), each in one unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    /// In g.
    AccelerationX,
    AccelerationY,
    AccelerationZ,
    /// In degrees Celsius.
    Temperature,
    /// In pascals (`AET-WO-4220`).
    Pressure,
}

/// Every quantity, its table name and its unit.
pub const QUANTITIES: [(Quantity, &str, &str); 5] = [
    (Quantity::AccelerationX, "acceleration_x", "g"),
    (Quantity::AccelerationY, "acceleration_y", "g"),
    (Quantity::AccelerationZ, "acceleration_z", "g"),
    (Quantity::Temperature, "temperature", "celsius"),
    (Quantity::Pressure, "pressure", "pascal"),
];

impl Quantity {
    pub fn name(self) -> &'static str {
        QUANTITIES.iter().find(|q| q.0 == self).map_or("", |q| q.1)
    }
}

/// One word of a data read, `bytes` long (2 or 3), most significant byte
/// first on the wire when `big_endian`, else least significant first: the
/// quantity the twin sets to `stimulus`, and the datasheet's conversion,
/// `raw = round((stimulus - zero) * lsb.0 / lsb.1)`, in two's complement when
/// `signed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Channel {
    pub quantity: Quantity,
    pub stimulus: i32,
    pub zero: i32,
    pub lsb: (u32, u32),
    pub signed: bool,
    pub bytes: u8,
    pub big_endian: bool,
}

impl Channel {
    /// The word the part answers for the stimulus, rounded half away from
    /// zero, in `bytes` bytes. The parser refused a channel whose word does
    /// not fit them.
    pub fn raw(&self) -> u32 {
        // Two's complement in `bytes` bytes; `channel` checked the range.
        let mask = (1u64 << (8 * u32::from(self.bytes))) - 1;
        (self.rounded() as u64 & mask) as u32
    }

    /// The word's bytes in the order they cross the wire.
    pub fn wire(&self) -> Vec<u8> {
        let raw = self.raw().to_le_bytes();
        let mut bytes = raw[..usize::from(self.bytes)].to_vec();
        if self.big_endian {
            bytes.reverse();
        }
        bytes
    }

    /// `round((stimulus - zero) * lsb.0 / lsb.1)`, half away from zero, in
    /// 128 bits, where no `i32` and `u32` inputs overflow (adversarial A5).
    fn rounded(&self) -> i128 {
        let num = (i128::from(self.stimulus) - i128::from(self.zero)) * i128::from(self.lsb.0);
        let den = i128::from(self.lsb.1);
        (2 * num + num.signum() * den) / (2 * den)
    }
}

/// What a device's bring-up does after its identity (`AET-WO-4210` S2): its
/// configuration writes, a wait, then one data read, each write and the read
/// in its own transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    /// The bytes each configuration write sends, in order; nothing is read
    /// back (`AET-WO-4220`: 1 to [`MAX_CONFIGURE`]).
    pub configure: Vec<Vec<u8>>,
    /// How long the firmware waits after the last write before the data read,
    /// in microseconds, cited to the datasheet's start-up and conversion
    /// times; `None` only for a stand-in whose model answers at once.
    pub settle_us: Option<u32>,
    /// The start-up of the configured state that `settle_us` must cover
    /// (`AET-WO-4230` §1.9).
    pub start_up: StartUp,
    /// The bytes the data read writes first (a register or a command).
    pub data: Vec<u8>,
    /// The words it reads back, in order.
    pub channels: Vec<Channel>,
}

/// The datasheet's start-up of a script's configured state: the time from the
/// last configuration write until the first data read is valid, as printed.
/// At least one of `max_us` and `typ_us` is set unless the part is a stand-in
/// whose datasheet prints none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartUp {
    pub max_us: Option<u32>,
    pub typ_us: Option<u32>,
    /// What the figure is the start-up of.
    pub condition: String,
}

/// The condition a typical-only start-up is judged under (the verify ledger
/// shows it with the row).
pub const SETTLE_TWICE_TYPICAL: &str = "settle covers twice the typical-only start-up";
/// The condition a cited start-up maximum is judged under.
pub const SETTLE_COVERS_MAXIMUM: &str = "settle covers the cited start-up maximum";

impl StartUp {
    /// The least `settle_us` that passes and the named condition it passes
    /// under: the maximum when one is cited, else twice the typical figure;
    /// `None` when the datasheet prints neither.
    pub fn required_settle(&self) -> Option<(u32, &'static str)> {
        match (self.max_us, self.typ_us) {
            (Some(max), _) => Some((max, SETTLE_COVERS_MAXIMUM)),
            (None, Some(typ)) => Some((typ.saturating_mul(2), SETTLE_TWICE_TYPICAL)),
            (None, None) => None,
        }
    }
}

/// Which step of a device's script.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StepKind {
    Identity,
    Configure,
    Data,
}

impl StepKind {
    pub fn name(self) -> &'static str {
        match self {
            StepKind::Identity => "identity",
            StepKind::Configure => "configure",
            StepKind::Data => "data",
        }
    }
}

/// The name of each configuration step of a script that makes more than one
/// (`AET-WO-4220`): every step of a device has its own name, so a record or a
/// witness of it is joined by (reference, name) to that step alone. A script
/// with one configuration names it `configure`, as before.
pub const CONFIGURE_STEPS: [&str; MAX_CONFIGURE] =
    ["configure 1", "configure 2", "configure 3", "configure 4"];

/// One transaction of a device's script, as the firmware makes it and the
/// judge expects it: write `write`, then read `width` bytes (none for a
/// configuration), which a present part answers with `expected` under
/// `mask`, the first byte on the wire as the most significant. After it the
/// firmware waits `settle_us` microseconds (the script's last configuration
/// write only; 0 elsewhere), which the judge does not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub kind: StepKind,
    /// Its name among the device's steps: the kind's name, or, for one of
    /// several configuration writes, its place among them ([`CONFIGURE_STEPS`]).
    pub label: &'static str,
    pub write: Vec<u8>,
    pub width: u8,
    pub expected: u64,
    pub mask: u64,
    pub settle_us: u32,
}

/// The bits a read of `width` bytes can set: a value with any bit above them
/// was not read, whatever the mask fixes (a configuration reads none, so its
/// value is 0).
pub fn width_mask(width: u8) -> u64 {
    match width {
        0 => 0,
        8.. => u64::MAX,
        w => (1u64 << (8 * u32::from(w))) - 1,
    }
}

/// Where a value group was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    pub document: String,
    pub revision: String,
    pub section: String,
    pub held: Held,
}

/// Whether the cited document is held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Held {
    /// The document's sha256, 64 lowercase hex digits.
    Sha256(String),
    /// Not held: the URL it must be fetched from (`AET-WO-4200` D8). The
    /// value stands, marked, until it is.
    Pending(String),
}

/// One citation per value group; `io`, `spi_output_delay`, `configure` and
/// `data` exactly when the part has an I/O range, an output delay and a
/// script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citations {
    pub bus: Citation,
    pub identity: Citation,
    pub supply: Citation,
    pub io: Option<Citation>,
    pub pin_capacitance: Citation,
    pub spi_output_delay: Option<Citation>,
    pub lifecycle: Citation,
    pub configure: Option<Citation>,
    pub start_up: Option<Citation>,
    pub data: Option<Citation>,
    /// `Some` exactly when the part lists [`Part::reads`] (`AET-WO-4400e`).
    pub reads: Option<Citation>,
}

impl Citations {
    /// Every citation the entry holds, with its value group's name.
    pub fn all(&self) -> Vec<(&'static str, &Citation)> {
        let mut v = vec![
            ("bus", &self.bus),
            ("identity", &self.identity),
            ("supply", &self.supply),
        ];
        v.extend(self.io.as_ref().map(|c| ("io", c)));
        v.push(("pin_capacitance", &self.pin_capacitance));
        v.extend(
            self.spi_output_delay
                .as_ref()
                .map(|c| ("spi_output_delay", c)),
        );
        v.push(("lifecycle", &self.lifecycle));
        v.extend(self.configure.as_ref().map(|c| ("configure", c)));
        v.extend(self.start_up.as_ref().map(|c| ("start_up", c)));
        v.extend(self.data.as_ref().map(|c| ("data", c)));
        v.extend(self.reads.as_ref().map(|c| ("reads", c)));
        v
    }
}

/// A data read a driver may make (`AET-WO-4400e`, D6 (i)): `write` sent
/// first (the register to read from), then at least `read` bytes clocked in,
/// as the datasheet states the burst. The bytes' values are the model's to
/// answer and are not part of the read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataRead {
    pub write: Vec<u8>,
    pub read: u8,
}

/// The most data reads a part lists, and the most bytes one reads.
pub const MAX_READS: usize = 4;
pub const MAX_READ_BYTES: u8 = 64;

/// One part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub part: String,
    pub bus: Bus,
    pub identity: Identity,
    /// The vendor's production status (`AET-WO-4210` D1).
    pub lifecycle: Lifecycle,
    /// In the table to exercise the twin, never for a purchase
    /// (`AET-WO-4210` D1). Only a stand-in may carry an identity no held
    /// datasheet states.
    pub stand_in: bool,
    /// The supply range, (min, max) in millivolts.
    pub supply_millivolts: (u32, u32),
    /// The range its signal pins accept, (min, max) in millivolts, where the
    /// datasheet separates it from the supply (`VDDIO`); `None` when the
    /// supply range is the I/O range (`AET-WO-4210` D7).
    pub io_millivolts: Option<(u32, u32)>,
    /// The largest maximum capacitance the datasheet prints for any of the
    /// part's signal pins, in femtofarads, or `None` when it prints none: a
    /// typical value is never promoted to a maximum (`AET-WO-4300c` D3).
    pub pin_capacitance_femtofarads: Option<u64>,
    /// The typical pin capacitance the datasheet prints, in femtofarads, or
    /// `None`: a floor for a specification's declaration (`AET-WO-4220b`),
    /// never a bound. A "< 10" pF in the TYP column is held as 10 pF.
    pub pin_capacitance_typical_femtofarads: Option<u64>,
    /// An SPI part's output valid time after the clock edge, at most, in
    /// nanoseconds, or `None`: the datasheet states none, or the table holds
    /// none (`AET-WO-4300d2` K6). Only an SPI part may carry one.
    pub spi_output_delay_max_ns: Option<u32>,
    /// The Renode model, as `platforms` names it, or `None`: unmodelled.
    pub renode: Option<String>,
    /// The bring-up after the identity, or `None`: the identity alone.
    pub script: Option<Script>,
    /// The data reads a driver of the part may make, as its datasheet states
    /// them (`AET-WO-4400e`, ruled on PR #212, D6 (i)): what a foreign
    /// firmware's read of the part is witnessed by. Only the foreign judge
    /// reads them; [`Part::steps`] never does, so no generated firmware
    /// performs one and no generated verdict moves. Empty: none listed.
    pub reads: Vec<DataRead>,
    pub citations: Citations,
    /// What ArduPilot calls the part, or `None`: no `hwdef.dat` line names
    /// it (`AET-WO-4400b` D4).
    pub ardupilot: Option<ArduPilot>,
    /// An IMU's cited axes, or `None`: not an IMU, or not cited
    /// (`AET-WO-4400d`). Only a part whose `ardupilot.kind` is `imu` may
    /// carry them.
    pub axes: Option<Axes>,
}

/// The `hwdef.dat` line a part takes in ArduPilot `Copter-4.7.2`
/// (`AET-WO-4400b` D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArduPilotKind {
    /// A `BARO <driver> …` line.
    Baro,
    /// An `IMU <driver> SPI:<spidev> …` line.
    Imu,
    /// The SPI device the flash logger opens by name.
    Dataflash,
}

/// What ArduPilot calls a part: its line, its driver, the `SPIDEV` name an
/// SPI part takes, and the chip ID the driver accepts, cited at the tag. The
/// parser holds the ID equal to the part's own identity, so a row that
/// writes down the driver's true ID (TMP119's 0x2117 for a TMP117) is
/// refused at load. It cannot check that the cited driver really accepts
/// that ID: that rests on the citation (`AET-WO-4400b` adversarial A8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArduPilot {
    pub kind: ArduPilotKind,
    /// The backend's name; `None` for a dataflash.
    pub driver: Option<String>,
    /// The `SPIDEV` name; `Some` exactly for an SPI part.
    pub spidev: Option<String>,
    pub id: u32,
    pub citation: Citation,
    /// An IMU driver's sample frame: `Some` when the row cites that the
    /// driver passes the chip's own X, Y and Z unchanged, the one frame this
    /// engine accepts (`AET-WO-4400d`); `None` when uncited, which
    /// `validate` refuses for a device that declares an orientation
    /// (`ImuAxesNotCited`).
    pub frame: Option<Citation>,
}

/// An IMU's sensing axes, as the datasheet's orientation figure draws them
/// against the package (`AET-WO-4400d`): +X points from the package's
/// centre toward the pins of `x_toward_pins`, +Y toward `y_toward_pins`, and
/// +Z out of the marked top face. Stated by pin numbers, so that it holds
/// for any footprint of the part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Axes {
    pub x_toward_pins: Vec<String>,
    pub y_toward_pins: Vec<String>,
    pub citation: Citation,
}

impl Part {
    /// The range a device's bus domain must lie in: the I/O range where the
    /// datasheet gives one, else the supply's (`AET-WO-4210` D7).
    pub fn io_range(&self) -> ((u32, u32), &Citation) {
        match (self.io_millivolts, &self.citations.io) {
            (Some(r), Some(c)) => (r, c),
            _ => (self.supply_millivolts, &self.citations.supply),
        }
    }

    /// The device's transactions, in the order the firmware makes them and
    /// the judge expects them: the identity, then the script's
    /// configuration and data read, if it has one.
    pub fn steps(&self) -> Vec<Step> {
        let id = &self.identity;
        let mut steps = vec![Step {
            kind: StepKind::Identity,
            label: StepKind::Identity.name(),
            write: id.write.clone(),
            width: id.width,
            expected: u64::from(id.value),
            mask: u64::from(id.mask),
            settle_us: 0,
        }];
        if let Some(s) = &self.script {
            let last = s.configure.len().saturating_sub(1);
            for (i, write) in s.configure.iter().enumerate() {
                steps.push(Step {
                    kind: StepKind::Configure,
                    label: if s.configure.len() == 1 {
                        StepKind::Configure.name()
                    } else {
                        CONFIGURE_STEPS[i]
                    },
                    write: write.clone(),
                    width: 0,
                    expected: 0,
                    mask: 0,
                    settle_us: if i == last {
                        s.settle_us.unwrap_or(0)
                    } else {
                        0
                    },
                });
            }
            let wire: Vec<u8> = s.channels.iter().flat_map(Channel::wire).collect();
            let width = u8::try_from(wire.len()).unwrap_or(u8::MAX);
            let expected = wire.iter().fold(0u64, |v, &b| (v << 8) | u64::from(b));
            steps.push(Step {
                kind: StepKind::Data,
                label: StepKind::Data.name(),
                write: s.data.clone(),
                width,
                expected,
                mask: width_mask(width),
                settle_us: 0,
            });
        }
        steps
    }
}

/// The parsed table, sorted by part name, each name once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartsTable {
    parts: Vec<Part>,
    /// The busless rows, sorted by part name (`AET-WO-1211a`). No bus
    /// device finds one through [`PartsTable::get`].
    pin_parts: Vec<crate::pin_parts::PinPart>,
}

impl PartsTable {
    pub fn parts(&self) -> &[Part] {
        &self.parts
    }

    pub fn get(&self, part: &str) -> Option<&Part> {
        self.parts
            .binary_search_by(|p| p.part.as_str().cmp(part))
            .ok()
            .map(|i| &self.parts[i])
    }

    /// The busless row of `part` (`AET-WO-1211a`).
    pub fn pin_part(&self, part: &str) -> Option<&crate::pin_parts::PinPart> {
        self.pin_parts.iter().find(|p| p.part == part)
    }

    pub fn pin_parts(&self) -> &[crate::pin_parts::PinPart] {
        &self.pin_parts
    }

    /// Every citation whose document is not held: (part, value group, URL).
    pub fn pending(&self) -> Vec<(&str, &'static str, &str)> {
        let mut out = Vec::new();
        for p in &self.parts {
            for (group, c) in p.citations.all() {
                if let Held::Pending(url) = &c.held {
                    out.push((p.part.as_str(), group, url.as_str()));
                }
            }
        }
        out
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawTable {
    schema_version: u32,
    /// Each entry's own text, parsed strictly on its own (`RawPart`), so a
    /// key given twice in an entry is refused, never collapsed (`AET-WO-4200`
    /// adversarial A2).
    parts: Vec<Box<serde_json::value::RawValue>>,
    /// The busless rows (`AET-WO-1211a`), each parsed on its own as `parts`'s.
    pin_parts: Vec<Box<serde_json::value::RawValue>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPart {
    part: String,
    lifecycle: String,
    stand_in: bool,
    bus: RawBus,
    identity: RawIdentity,
    supply_millivolts: RawSupply,
    /// Required: `null` says the supply range is the I/O range.
    #[serde(deserialize_with = "Option::deserialize")]
    io_millivolts: Option<RawSupply>,
    pin_capacitance: RawPinCapacitance,
    /// Required: `null` says the table holds no output delay for the part;
    /// only an SPI part may give one.
    #[serde(deserialize_with = "Option::deserialize")]
    spi_output_delay_max_ns: Option<u32>,
    /// Required: `null` says unmodelled; an absent key is refused, never read
    /// as unmodelled (adversarial A2).
    #[serde(deserialize_with = "Option::deserialize")]
    renode: Option<String>,
    /// Required: `null` says the identity alone.
    #[serde(deserialize_with = "Option::deserialize")]
    script: Option<RawScript>,
    /// Required: `null` says the part lists no data read (`AET-WO-4400e`).
    #[serde(deserialize_with = "Option::deserialize")]
    reads: Option<Vec<RawRead>>,
    citations: RawCitations,
    /// Required: `null` says no `hwdef.dat` line names the part.
    #[serde(deserialize_with = "Option::deserialize")]
    ardupilot: Option<RawArduPilot>,
    /// Required: `null` says not an IMU, or its axes uncited.
    #[serde(deserialize_with = "Option::deserialize")]
    axes: Option<RawAxes>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAxes {
    x_toward_pins: Vec<String>,
    y_toward_pins: Vec<String>,
    z: String,
    at: RawCitation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFrame {
    is: String,
    at: RawCitation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawArduPilot {
    kind: String,
    #[serde(deserialize_with = "Option::deserialize")]
    driver: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    spidev: Option<String>,
    id: String,
    at: RawCitation,
    /// Required: `null` for a part that is not an IMU, or whose driver's
    /// frame is uncited.
    #[serde(deserialize_with = "Option::deserialize")]
    frame: Option<RawFrame>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRead {
    write: Vec<String>,
    read: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScript {
    configure: RawConfigure,
    data: RawData,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfigure {
    writes: Vec<Vec<String>>,
    /// Required: `null` says the model answers at once (a stand-in only).
    #[serde(deserialize_with = "Option::deserialize")]
    settle_us: Option<u32>,
    start_up: RawStartUp,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStartUp {
    /// Required: `null` says the datasheet prints no maximum.
    #[serde(deserialize_with = "Option::deserialize")]
    max_us: Option<u32>,
    /// Required: `null` says the datasheet prints no typical figure.
    #[serde(deserialize_with = "Option::deserialize")]
    typ_us: Option<u32>,
    condition: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawData {
    write: Vec<String>,
    channels: Vec<RawChannel>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawChannel {
    quantity: String,
    unit: String,
    stimulus: i32,
    zero: i32,
    lsb: [u32; 2],
    signed: bool,
    bytes: u8,
    order: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
enum RawBus {
    I2c(RawI2c),
    Spi(RawSpi),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawI2c {
    addresses: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSpi {
    modes: Vec<u8>,
    max_clock_hz: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawIdentity {
    write: Vec<String>,
    read: u8,
    value: String,
    /// Required: `null` says every bit of the value is fixed.
    #[serde(deserialize_with = "Option::deserialize")]
    mask: Option<String>,
    datasheet_value: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSupply {
    min: u32,
    max: u32,
}

/// Required: `null` says the datasheet prints no maximum; an absent key is
/// refused, never read as none.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPinCapacitance {
    #[serde(deserialize_with = "Option::deserialize")]
    max_femtofarads: Option<u64>,
    #[serde(deserialize_with = "Option::deserialize")]
    typical_femtofarads: Option<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCitations {
    bus: RawCitation,
    identity: RawCitation,
    supply: RawCitation,
    /// Required: `null` exactly when `io_millivolts` is.
    #[serde(deserialize_with = "Option::deserialize")]
    io: Option<RawCitation>,
    pin_capacitance: RawCitation,
    /// Required: `null` exactly when `spi_output_delay_max_ns` is.
    #[serde(deserialize_with = "Option::deserialize")]
    spi_output_delay: Option<RawCitation>,
    lifecycle: RawCitation,
    /// Required: `null` exactly when `script` is.
    #[serde(deserialize_with = "Option::deserialize")]
    configure: Option<RawCitation>,
    /// Required: `null` exactly when `script` is.
    #[serde(deserialize_with = "Option::deserialize")]
    start_up: Option<RawCitation>,
    #[serde(deserialize_with = "Option::deserialize")]
    data: Option<RawCitation>,
    /// Required: `null` exactly when `reads` is.
    #[serde(deserialize_with = "Option::deserialize")]
    reads: Option<RawCitation>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawCitation {
    document: String,
    revision: String,
    section: String,
    sha256: Option<String>,
    pending: Option<String>,
}

impl RawCitation {
    /// A citation built from its parts, for the power parts table's
    /// documents, which [`citation`] then checks as it checks these.
    pub(crate) fn new(
        document: String,
        revision: String,
        section: String,
        sha256: Option<String>,
        pending: Option<String>,
    ) -> Self {
        Self {
            document,
            revision,
            section,
            sha256,
            pending,
        }
    }
}

fn refused(part: Option<&str>, reason: impl Into<String>) -> IrValidationError {
    IrValidationError::PartsTable {
        part: part.map(str::to_owned),
        reason: reason.into(),
    }
}

/// `0x` then 1 to 8 upper-case hexadecimal digits, at most `max`.
fn hex(text: &str, field: &str, max: u32) -> Result<u32, String> {
    let digits = text
        .strip_prefix("0x")
        .filter(|d| (1..=8).contains(&d.len()))
        .filter(|d| {
            d.bytes()
                .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b))
        })
        .ok_or_else(|| {
            format!("`{field}` is `{text}`, not `0x` then 1 to 8 upper-case hex digits")
        })?;
    let v = u32::from_str_radix(digits, 16).map_err(|e| format!("`{field}`: {e}"))?;
    if v > max {
        return Err(format!("`{field}` is {text}, above {max:#X}"));
    }
    Ok(v)
}

/// Printable ASCII, 1 to `max` bytes.
fn text(value: &str, field: &str, max: usize) -> Result<(), String> {
    if value.is_empty() || value.len() > max || !value.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
        return Err(format!(
            "`{field}` is not 1 to {max} bytes of printable ASCII"
        ));
    }
    Ok(())
}

pub(crate) fn citation(raw: RawCitation, group: &str) -> Result<Citation, String> {
    text(&raw.document, &format!("citations.{group}.document"), 128)?;
    text(&raw.revision, &format!("citations.{group}.revision"), 64)?;
    text(&raw.section, &format!("citations.{group}.section"), 256)?;
    let held = match (raw.sha256, raw.pending) {
        (Some(h), None) => {
            if h.len() != 64 || !h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
                return Err(format!(
                    "`citations.{group}.sha256` is not 64 lower-case hex digits"
                ));
            }
            Held::Sha256(h)
        }
        (None, Some(url)) => {
            text(&url, &format!("citations.{group}.pending"), 256)?;
            if url.strip_prefix("https://").is_none_or(str::is_empty) {
                return Err(format!("`citations.{group}.pending` is not an https URL"));
            }
            Held::Pending(url)
        }
        _ => {
            return Err(format!(
                "`citations.{group}` has neither or both of `sha256` and `pending`"
            ));
        }
    };
    Ok(Citation {
        document: raw.document,
        revision: raw.revision,
        section: raw.section,
        held,
    })
}

/// 1 to [`MAX_STEP_WRITE`] bytes, each `0x` then 1 or 2 upper-case hex digits.
fn bytes(list: &[String], field: &str) -> Result<Vec<u8>, String> {
    if list.is_empty() || list.len() > MAX_STEP_WRITE {
        return Err(format!("`{field}` is not 1 to {MAX_STEP_WRITE} bytes"));
    }
    list.iter()
        .map(|b| u8::try_from(hex(b, field, 0xFF)?).map_err(|e| e.to_string()))
        .collect()
}

/// A millivolt range: 1 <= min <= max <= [`MAX_SUPPLY_MILLIVOLTS`].
fn millivolts(raw: &RawSupply, field: &str) -> Result<(u32, u32), String> {
    let (min, max) = (raw.min, raw.max);
    if min == 0 || min > max || max > MAX_SUPPLY_MILLIVOLTS {
        return Err(format!(
            "`{field}` is {min} to {max}, not 1 <= min <= max <= {MAX_SUPPLY_MILLIVOLTS}"
        ));
    }
    Ok((min, max))
}

fn channel(raw: &RawChannel, at: usize) -> Result<Channel, String> {
    let field = format!("script.data.channels[{at}]");
    let Some(&(quantity, _, unit)) = QUANTITIES.iter().find(|q| q.1 == raw.quantity) else {
        return Err(format!(
            "`{field}.quantity` is `{}`, not a quantity the twin can set",
            raw.quantity
        ));
    };
    if raw.unit != unit {
        return Err(format!(
            "`{field}.unit` is `{}`, but `{}` is in `{unit}`",
            raw.unit, raw.quantity
        ));
    }
    let [num, den] = raw.lsb;
    if num == 0 || den == 0 {
        return Err(format!("`{field}.lsb` holds a zero"));
    }
    if !(2..=3).contains(&raw.bytes) {
        return Err(format!("`{field}.bytes` is {}, not 2 or 3", raw.bytes));
    }
    let big_endian = match raw.order.as_str() {
        "big" => true,
        "little" => false,
        other => {
            return Err(format!(
                "`{field}.order` is `{other}`, not `big` or `little`"
            ));
        }
    };
    let c = Channel {
        quantity,
        stimulus: raw.stimulus,
        zero: raw.zero,
        lsb: (num, den),
        signed: raw.signed,
        bytes: raw.bytes,
        big_endian,
    };
    let rounded = c.rounded();
    let bits = 8 * u32::from(c.bytes);
    let range = if c.signed {
        -(1i128 << (bits - 1))..=(1i128 << (bits - 1)) - 1
    } else {
        0..=(1i128 << bits) - 1
    };
    if !range.contains(&rounded) {
        return Err(format!(
            "`{field}`: the stimulus converts to {rounded}, which a {} {bits}-bit word cannot hold",
            if c.signed { "signed" } else { "unsigned" }
        ));
    }
    Ok(c)
}

/// The `reads` column (`AET-WO-4400e`): 1 to [`MAX_READS`] reads, each 1 to
/// 4 bytes written and 1 to [`MAX_READ_BYTES`] read, no read listed twice,
/// and only for a part the twin has a model of: a read no model answers
/// could witness nothing.
fn reads(raw: &[RawRead], modelled: bool) -> Result<Vec<DataRead>, String> {
    if !modelled {
        return Err("`reads` is given for a part with no `renode` model".to_owned());
    }
    if raw.is_empty() || raw.len() > MAX_READS {
        return Err(format!("`reads` is not 1 to {MAX_READS} reads"));
    }
    let mut out: Vec<DataRead> = Vec::new();
    for (i, r) in raw.iter().enumerate() {
        let write = bytes(&r.write, &format!("reads[{i}].write"))?;
        if write.len() > 4 {
            return Err(format!("`reads[{i}].write` is more than 4 bytes"));
        }
        let read = u8::try_from(r.read)
            .ok()
            .filter(|n| (1..=MAX_READ_BYTES).contains(n))
            .ok_or_else(|| format!("`reads[{i}].read` is {}, not 1 to {MAX_READ_BYTES}", r.read))?;
        let d = DataRead { write, read };
        if out.contains(&d) {
            return Err(format!("`reads[{i}]` is listed twice"));
        }
        out.push(d);
    }
    Ok(out)
}

fn script(raw: &RawScript, stand_in: bool) -> Result<Script, String> {
    let writes = &raw.configure.writes;
    if writes.is_empty() || writes.len() > MAX_CONFIGURE {
        return Err(format!(
            "`script.configure.writes` is not 1 to {MAX_CONFIGURE} writes"
        ));
    }
    let configure = writes
        .iter()
        .enumerate()
        .map(|(i, w)| bytes(w, &format!("script.configure.writes[{i}]")))
        .collect::<Result<Vec<_>, _>>()?;
    let settle_us = raw.configure.settle_us;
    match settle_us {
        None if !stand_in => {
            return Err(
                "`script.configure.settle_us` is null, which only a twin stand-in may be: a part's model samples on its own timer, so the firmware waits a cited time".to_owned(),
            );
        }
        Some(us) if !(1..=MAX_SETTLE_US).contains(&us) => {
            return Err(format!(
                "`script.configure.settle_us` is {us}, not null or 1 to {MAX_SETTLE_US}"
            ));
        }
        _ => {}
    }
    let start_up = start_up(&raw.configure.start_up, settle_us, stand_in)?;
    let data = bytes(&raw.data.write, "script.data.write")?;
    if raw.data.channels.is_empty() {
        return Err("`script.data.channels` is empty".to_owned());
    }
    let channels = raw
        .data
        .channels
        .iter()
        .enumerate()
        .map(|(i, c)| channel(c, i))
        .collect::<Result<Vec<_>, _>>()?;
    let width: u32 = channels.iter().map(|c| u32::from(c.bytes)).sum();
    if width > MAX_DATA_BYTES {
        return Err(format!(
            "`script.data.channels` read {width} bytes, more than {MAX_DATA_BYTES}"
        ));
    }
    Ok(Script {
        configure,
        settle_us,
        start_up,
        data,
        channels,
    })
}

/// `SettleBelowStartUp` (`AET-WO-4230` §1.9): the cited start-up, and the
/// wait that must cover it: `settle_us` at least the maximum when one is
/// cited, else at least twice the typical figure. A null wait beside a figure
/// is refused the same way.
fn start_up(raw: &RawStartUp, settle_us: Option<u32>, stand_in: bool) -> Result<StartUp, String> {
    text(&raw.condition, "script.configure.start_up.condition", 256)?;
    for (name, v) in [("max_us", raw.max_us), ("typ_us", raw.typ_us)] {
        if v.is_some_and(|us| us == 0) {
            return Err(format!(
                "`script.configure.start_up.{name}` is 0, not null or a positive time"
            ));
        }
    }
    if let (Some(max), Some(typ)) = (raw.max_us, raw.typ_us)
        && typ > max
    {
        return Err(format!(
            "`script.configure.start_up` has typ_us {typ} above max_us {max}"
        ));
    }
    let start_up = StartUp {
        max_us: raw.max_us,
        typ_us: raw.typ_us,
        condition: raw.condition.clone(),
    };
    match (start_up.required_settle(), settle_us) {
        (None, _) if !stand_in => Err(
            "`script.configure.start_up` holds neither max_us nor typ_us, which only a twin stand-in whose datasheet prints none may".to_owned(),
        ),
        (None, _) => Ok(start_up),
        (Some((need, why)), None) => Err(format!(
            "SettleBelowStartUp: `script.configure.settle_us` is null beside a cited start-up of {need} us ({why})"
        )),
        (Some((need, why)), Some(us)) if us < need => Err(format!(
            "SettleBelowStartUp: `script.configure.settle_us` is {us}, below {need} ({why})"
        )),
        (Some(_), Some(_)) => Ok(start_up),
    }
}

/// A citation present exactly when its value group is.
fn paired(
    raw: Option<RawCitation>,
    present: bool,
    group: &str,
    value: &str,
) -> Result<Option<Citation>, String> {
    match (raw, present) {
        (Some(c), true) => citation(c, group).map(Some),
        (None, false) => Ok(None),
        (Some(_), false) => Err(format!(
            "`citations.{group}` is given but `{value}` is null"
        )),
        (None, true) => Err(format!(
            "`citations.{group}` is null but `{value}` is given"
        )),
    }
}

fn part(raw: RawPart) -> Result<Part, String> {
    let lifecycle = match raw.lifecycle.as_str() {
        "active" => Lifecycle::Active,
        "nrnd" => Lifecycle::Nrnd,
        "eol" => Lifecycle::Eol,
        other => {
            return Err(format!(
                "`lifecycle` is `{other}`, not `active`, `nrnd` or `eol`"
            ));
        }
    };
    let bus = match raw.bus {
        RawBus::I2c(i2c) => {
            if i2c.addresses.is_empty() {
                return Err("`bus.i2c.addresses` is empty".to_owned());
            }
            let mut addresses = Vec::new();
            for a in &i2c.addresses {
                let v = hex(a, "bus.i2c.addresses", 0x77)?;
                if v < 0x08 {
                    return Err(format!("`bus.i2c.addresses` holds {a}, below 0x08"));
                }
                let v = u8::try_from(v).map_err(|e| e.to_string())?;
                if addresses.contains(&v) {
                    return Err(format!("`bus.i2c.addresses` holds {a} twice"));
                }
                addresses.push(v);
            }
            Bus::I2c { addresses }
        }
        RawBus::Spi(spi) => {
            if spi.modes.is_empty() {
                return Err("`bus.spi.modes` is empty".to_owned());
            }
            let mut seen = BTreeSet::new();
            for &m in &spi.modes {
                if m > 3 {
                    return Err(format!("`bus.spi.modes` holds {m}, not 0 to 3"));
                }
                if !seen.insert(m) {
                    return Err(format!("`bus.spi.modes` holds {m} twice"));
                }
            }
            if !(1..=MAX_SPI_CLOCK_HZ).contains(&spi.max_clock_hz) {
                return Err(format!(
                    "`bus.spi.max_clock_hz` is {}, not 1 to {MAX_SPI_CLOCK_HZ}",
                    spi.max_clock_hz
                ));
            }
            Bus::Spi {
                modes: spi.modes,
                max_clock_hz: spi.max_clock_hz,
            }
        }
    };
    let width = raw.identity.read;
    if !(1..=4).contains(&width) {
        return Err(format!("`identity.read` is {width}, not 1 to 4"));
    }
    let write = bytes(&raw.identity.write, "identity.write")?;
    let max = if width == 4 {
        u32::MAX
    } else {
        (1u32 << (8 * width)) - 1
    };
    let value = hex(&raw.identity.value, "identity.value", max)?;
    let mask = match &raw.identity.mask {
        None => max,
        Some(m) => {
            let m = hex(m, "identity.mask", max)?;
            if m == 0 || value & !m != 0 {
                return Err(format!(
                    "`identity.mask` is {m:#X}: zero, or it leaves bits of the value unfixed"
                ));
            }
            m
        }
    };
    // A stand-in is in the table to exercise the twin, never for a purchase,
    // so it must carry a lifecycle the specification has to acknowledge and
    // the banner names (adversarial A6).
    if raw.stand_in && lifecycle == Lifecycle::Active {
        return Err("`stand_in` is true for an `active` part: a twin stand-in is `eol` or `nrnd`, so a board that uses it must acknowledge it".to_owned());
    }
    if !raw.identity.datasheet_value && !raw.stand_in {
        return Err(
            "`identity.datasheet_value` is false, which only a twin stand-in may be".to_owned(),
        );
    }
    let (min, max) = millivolts(&raw.supply_millivolts, "supply_millivolts")?;
    let io = raw
        .io_millivolts
        .as_ref()
        .map(|r| millivolts(r, "io_millivolts"))
        .transpose()?;
    let script = raw
        .script
        .as_ref()
        .map(|s| script(s, raw.stand_in))
        .transpose()?;
    let pin_capacitance = raw.pin_capacitance.max_femtofarads;
    if pin_capacitance.is_some_and(|c| !(1..=MAX_PIN_FEMTOFARADS).contains(&c)) {
        return Err(format!(
            "`pin_capacitance.max_femtofarads` is {pin_capacitance:?}, not null or 1 to {MAX_PIN_FEMTOFARADS}"
        ));
    }
    let typical = raw.pin_capacitance.typical_femtofarads;
    if typical.is_some_and(|c| {
        !(1..=MAX_PIN_FEMTOFARADS).contains(&c) || pin_capacitance.is_some_and(|m| c > m)
    }) {
        return Err(format!(
            "`pin_capacitance.typical_femtofarads` is {typical:?}, not null or 1 to {MAX_PIN_FEMTOFARADS}, nor above the maximum"
        ));
    }
    if let Some(ns) = raw.spi_output_delay_max_ns {
        if bus.kind() != BusKind::Spi {
            return Err(
                "`spi_output_delay_max_ns` is given on an I2C part, which has no SPI output"
                    .to_owned(),
            );
        }
        if !(1..=MAX_SPI_OUTPUT_DELAY_NS).contains(&ns) {
            return Err(format!(
                "`spi_output_delay_max_ns` is {ns}, not null or 1 to {MAX_SPI_OUTPUT_DELAY_NS}"
            ));
        }
    }
    if let Some(model) = &raw.renode {
        let Some((_, buses)) = RENODE_MODELS.iter().find(|(m, _)| m == model) else {
            return Err(format!(
                "`renode` names `{model}`, which is not a known model"
            ));
        };
        if !buses.contains(&bus.kind()) {
            return Err(format!(
                "`renode` names `{model}`, which does not attach to the part's bus"
            ));
        }
        let capacity = u8::try_from(value & 0xFF).map_err(|e| e.to_string())?;
        if model == "SPI.GenericSpiFlash"
            && (write != [JEDEC_READ]
                || width != JEDEC_WIDTH
                || !FLASH_CAPACITY_CODES.contains(&capacity))
        {
            return Err(format!(
                "`identity` is not a JEDEC identity a `SPI.GenericSpiFlash` is built from: write {JEDEC_READ:#04X}, read {JEDEC_WIDTH}, a capacity code {:#04X} to {:#04X} in the last byte",
                FLASH_CAPACITY_CODES.start(),
                FLASH_CAPACITY_CODES.end()
            ));
        }
    }
    let c = raw.citations;
    let citations = Citations {
        bus: citation(c.bus, "bus")?,
        identity: citation(c.identity, "identity")?,
        supply: citation(c.supply, "supply")?,
        io: paired(c.io, io.is_some(), "io", "io_millivolts")?,
        pin_capacitance: citation(c.pin_capacitance, "pin_capacitance")?,
        spi_output_delay: paired(
            c.spi_output_delay,
            raw.spi_output_delay_max_ns.is_some(),
            "spi_output_delay",
            "spi_output_delay_max_ns",
        )?,
        lifecycle: citation(c.lifecycle, "lifecycle")?,
        configure: paired(c.configure, script.is_some(), "configure", "script")?,
        start_up: paired(c.start_up, script.is_some(), "start_up", "script")?,
        data: paired(c.data, script.is_some(), "data", "script")?,
        reads: paired(c.reads, raw.reads.is_some(), "reads", "reads")?,
    };
    let reads = match &raw.reads {
        None => Vec::new(),
        Some(r) => reads(r, raw.renode.is_some())?,
    };
    let ardupilot = match raw.ardupilot {
        None => None,
        Some(a) => Some(ardupilot(a, &bus, value)?),
    };
    let imu = ardupilot
        .as_ref()
        .is_some_and(|a| a.kind == ArduPilotKind::Imu);
    let axes = match raw.axes {
        None => None,
        Some(_) if !imu => {
            return Err(
                "`axes` is given only for a part whose `ardupilot.kind` is `imu`".to_owned(),
            );
        }
        Some(a) => Some(axes(a)?),
    };
    Ok(Part {
        part: raw.part,
        lifecycle,
        stand_in: raw.stand_in,
        bus,
        identity: Identity {
            write,
            width,
            value,
            mask,
            datasheet_value: raw.identity.datasheet_value,
        },
        supply_millivolts: (min, max),
        io_millivolts: io,
        pin_capacitance_femtofarads: pin_capacitance,
        pin_capacitance_typical_femtofarads: typical,
        spi_output_delay_max_ns: raw.spi_output_delay_max_ns,
        renode: raw.renode,
        script,
        reads,
        citations,
        ardupilot,
        axes,
    })
}

/// The `axes` column: two non-empty groups of pin numbers (1 to 8 of
/// `[A-Za-z0-9]` each), no pin named twice, and +Z out of the `top`, the
/// marked face: a part whose +Z points into the board would need a
/// footprint on B.Cu, which this engine does not place.
fn axes(raw: RawAxes) -> Result<Axes, String> {
    let mut seen = std::collections::BTreeSet::new();
    for (group, pins) in [
        ("axes.x_toward_pins", &raw.x_toward_pins),
        ("axes.y_toward_pins", &raw.y_toward_pins),
    ] {
        if pins.is_empty() {
            return Err(format!("`{group}` is empty"));
        }
        for pin in pins {
            let ok = (1..=8).contains(&pin.len()) && pin.bytes().all(|b| b.is_ascii_alphanumeric());
            if !ok {
                return Err(format!(
                    "`{group}` holds `{pin}`, not 1 to 8 of [A-Za-z0-9]"
                ));
            }
            if !seen.insert(pin.as_str()) {
                return Err(format!(
                    "`{group}` names pin `{pin}`, which `axes` names twice"
                ));
            }
        }
    }
    if raw.z != "top" {
        return Err(format!("`axes.z` is `{}`, not `top`", raw.z));
    }
    Ok(Axes {
        x_toward_pins: raw.x_toward_pins,
        y_toward_pins: raw.y_toward_pins,
        citation: citation(raw.at, "axes")?,
    })
}

/// A name ArduPilot reads as a C token or a string: 1 to 32 of `[A-Za-z0-9_]`.
fn token(value: &str, field: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 32
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err(format!(
            "`{field}` is `{value}`, not 1 to 32 of [A-Za-z0-9_]"
        ));
    }
    Ok(())
}

/// The `ardupilot` column: a known kind; a driver exactly for a barometer or
/// an IMU; an `SPIDEV` name exactly for an SPI part (lower case, as
/// ArduPilot's own names are); an IMU only on SPI; and the chip ID equal to
/// the part's identity value.
fn ardupilot(raw: RawArduPilot, bus: &Bus, identity: u32) -> Result<ArduPilot, String> {
    let kind = match raw.kind.as_str() {
        "baro" => ArduPilotKind::Baro,
        "imu" => ArduPilotKind::Imu,
        "dataflash" => ArduPilotKind::Dataflash,
        other => {
            return Err(format!(
                "`ardupilot.kind` is `{other}`, not `baro`, `imu` or `dataflash`"
            ));
        }
    };
    match (&raw.driver, kind) {
        (Some(d), ArduPilotKind::Baro | ArduPilotKind::Imu) => token(d, "ardupilot.driver")?,
        (None, ArduPilotKind::Dataflash) => {}
        _ => {
            return Err("`ardupilot.driver` is given exactly for a `baro` or an `imu`".to_owned());
        }
    }
    let spi = matches!(bus, Bus::Spi { .. });
    match &raw.spidev {
        Some(n) if spi => {
            token(n, "ardupilot.spidev")?;
            if n.bytes().any(|b| b.is_ascii_uppercase()) {
                return Err(format!("`ardupilot.spidev` is `{n}`, not lower case"));
            }
        }
        None if !spi => {}
        _ => {
            return Err("`ardupilot.spidev` is given exactly for an SPI part".to_owned());
        }
    }
    if !spi && kind != ArduPilotKind::Baro {
        return Err("an I2C part's `ardupilot.kind` is `baro`".to_owned());
    }
    let frame = match (raw.frame, kind) {
        (None, _) => None,
        (Some(f), ArduPilotKind::Imu) if f.is == "chip" => Some(citation(f.at, "ardupilot.frame")?),
        (Some(f), ArduPilotKind::Imu) => {
            return Err(format!(
                "`ardupilot.frame.is` is `{}`, not `chip`, the one frame this engine accepts",
                f.is
            ));
        }
        (Some(_), _) => {
            return Err("`ardupilot.frame` is given only for an `imu`".to_owned());
        }
    };
    let id = hex(&raw.id, "ardupilot.id", u32::MAX)?;
    if id != identity {
        return Err(format!(
            "`ardupilot.id` is {}, but the part's identity value is {identity:#X}: ArduPilot's driver would refuse the chip",
            raw.id
        ));
    }
    Ok(ArduPilot {
        kind,
        driver: raw.driver,
        spidev: raw.spidev,
        id,
        citation: citation(raw.at, "ardupilot")?,
        frame,
    })
}

/// Parses a parts table, refusing the first malformed entry by naming its
/// part and the field: an unknown key, a field of the wrong kind or out of
/// range, a model that is not known or does not attach to the part's bus, a
/// citation that is neither held nor pending, a key given twice in an entry,
/// a missing `renode` (`null` is unmodelled), a part named twice in any case,
/// or an empty table. It never panics.
pub fn parse_parts(json: &str) -> Result<PartsTable, IrValidationError> {
    let raw: RawTable = serde_json::from_str(json).map_err(|e| refused(None, e.to_string()))?;
    if raw.schema_version != PARTS_SCHEMA_VERSION {
        return Err(refused(
            None,
            format!(
                "schema_version is {}, not {PARTS_SCHEMA_VERSION}",
                raw.schema_version
            ),
        ));
    }
    if raw.parts.is_empty() {
        return Err(refused(None, "`parts` is empty"));
    }
    let mut parts: Vec<Part> = Vec::with_capacity(raw.parts.len());
    for (i, text) in raw.parts.iter().enumerate() {
        // The name, read leniently, only to say which entry is refused.
        let entry: serde_json::Value =
            serde_json::from_str(text.get()).map_err(|e| refused(None, e.to_string()))?;
        let name = entry
            .get("part")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| refused(None, format!("entry {i} has no string `part`")))?;
        let named = |reason: String| refused(Some(&name), reason);
        crate::check_identifier(crate::IdentifierKind::Plain, &name)
            .map_err(|rule| named(format!("`part` is not an identifier: {rule}")))?;
        let one_bus = entry
            .get("bus")
            .and_then(serde_json::Value::as_object)
            .is_some_and(|b| b.len() == 1);
        if !one_bus {
            return Err(named(
                "`bus` is not an object holding exactly one of `i2c` and `spi`".to_owned(),
            ));
        }
        let raw: RawPart = serde_json::from_str(text.get()).map_err(|e| named(e.to_string()))?;
        let p = part(raw).map_err(named)?;
        if parts.iter().any(|q| q.part.eq_ignore_ascii_case(&p.part)) {
            return Err(named(
                "the part is in the table twice, in some case".to_owned(),
            ));
        }
        parts.push(p);
    }
    parts.sort_by(|a, b| a.part.cmp(&b.part));
    let mut pin_parts: Vec<crate::pin_parts::PinPart> = Vec::new();
    for (i, text) in raw.pin_parts.iter().enumerate() {
        let entry: serde_json::Value =
            serde_json::from_str(text.get()).map_err(|e| refused(None, e.to_string()))?;
        let name = entry
            .get("part")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| refused(None, format!("pin_parts entry {i} has no string `part`")))?;
        let named = |reason: String| refused(Some(&name), reason);
        crate::check_identifier(crate::IdentifierKind::Plain, &name)
            .map_err(|rule| named(format!("`part` is not an identifier: {rule}")))?;
        let raw: crate::pin_parts::RawPinPart =
            serde_json::from_str(text.get()).map_err(|e| named(e.to_string()))?;
        let p = crate::pin_parts::pin_part(raw).map_err(named)?;
        let clash = parts
            .iter()
            .map(|q| &q.part)
            .chain(pin_parts.iter().map(|q| &q.part));
        if clash.into_iter().any(|q| q.eq_ignore_ascii_case(&p.part)) {
            return Err(named(
                "the part is in the table twice, in some case".to_owned(),
            ));
        }
        pin_parts.push(p);
    }
    pin_parts.sort_by(|a, b| a.part.cmp(&b.part));
    Ok(PartsTable { parts, pin_parts })
}

static TABLE: LazyLock<Result<PartsTable, IrValidationError>> =
    LazyLock::new(|| parse_parts(PARTS_JSON));

/// The embedded table, parsed once. A malformed table is an error here, which
/// every caller returns, never a panic.
pub fn table() -> Result<&'static PartsTable, IrValidationError> {
    TABLE.as_ref().map_err(Clone::clone)
}

/// The identity of `part`, or `None`: a part with no entry has no identity read.
pub fn identity(part: &str) -> Result<Option<Identity>, IrValidationError> {
    Ok(table()?.get(part).map(|p| p.identity.clone()))
}

/// The steps of `part`'s script ([`Part::steps`]), or `None`: a part with no
/// entry makes no transaction.
pub fn steps(part: &str) -> Result<Option<Vec<Step>>, IrValidationError> {
    Ok(table()?.get(part).map(Part::steps))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_table_parses_and_holds_the_seven_parts() {
        let t = table().unwrap();
        let names: Vec<&str> = t.parts().iter().map(|p| p.part.as_str()).collect();
        assert_eq!(
            names,
            [
                "BME280",
                "BMP581",
                "ICM20948",
                "ICM42688P",
                "ICP10111",
                "TMP117",
                "W25Q64JVxIQ"
            ]
        );
        assert_eq!(identity("BME280").unwrap().unwrap().value, 0x60);
        assert_eq!(identity("TMP117").unwrap().unwrap().width, 2);
        assert_eq!(identity("bme280").unwrap(), None);
        assert_eq!(identity("W25Q128").unwrap(), None);
        assert_eq!(identity(BARE_W25Q64JV).unwrap(), None);
    }

    /// The embedded table with the W25Q64JVxIQ row's `bus.spi.modes` set to
    /// `modes`, parsed as a table is.
    fn with_flash_modes(modes: &[u8]) -> PartsTable {
        let mut doc: serde_json::Value = serde_json::from_str(PARTS_JSON).unwrap();
        let row = doc["parts"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|p| p["part"] == "W25Q64JVxIQ")
            .unwrap();
        row["bus"]["spi"]["modes"] = serde_json::json!(modes);
        parse_parts(&doc.to_string()).unwrap()
    }

    /// The crate's own power-board fixture, whose first SPI device, `U4`, is
    /// a W25Q64JVxIQ (inside the package: CI's package gate refuses an
    /// `include_str!` reaching outside the crate).
    fn reference() -> crate::Specification {
        serde_json::from_str(include_str!("../tests/fixtures/sentinel-02-power.json")).unwrap()
    }

    /// `AET-WO-1207` §1.5 (Grok 2026-11-16 row 5): step 14 refuses a device
    /// whose part's row lists no mode the controller drives, naming the row's
    /// modes and its citation. Before, a row's `modes` were parsed and never
    /// read, and each of these passed (`4c61fad`).
    #[test]
    fn aet_1207_a_device_whose_modes_exclude_the_driven_one_is_refused() {
        let spec = reference();
        for modes in [&[1u8, 2, 3][..], &[3], &[2]] {
            let table = with_flash_modes(modes);
            let err = spec.validate_parts(&table).unwrap_err();
            assert_eq!(err.kind(), "SpiModeNotDriven", "{modes:?}");
            assert_eq!(err.id(), Some("U4"));
            // The row's own citation (`AET-WO-4211` revises that row).
            let c = &table.get("W25Q64JVxIQ").unwrap().citations.bus;
            assert_eq!(
                err.to_string(),
                format!(
                    "device `U4` is a `W25Q64JVxIQ`, which accepts SPI modes {modes:?} ({} rev. {}, {}), not mode 0, the one the generated firmware drives (CFG2 CPOL 0, CPHA 0)",
                    c.document, c.revision, c.section
                )
            );
        }
        for modes in [&[0u8][..], &[3, 0], &[0, 1, 2, 3]] {
            let table = with_flash_modes(modes);
            assert_eq!(spec.validate_parts(&table), Ok(()), "{modes:?}");
        }
    }
}
