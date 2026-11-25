//! Simulator platform descriptions, the analog model, and the lock-step bridge; the specification is read-only here (`docs/architecture.md` §3).
//!
//! `AET-WO-2100` (ruled on PR #56, R6): [`platform`] lowers a solved topology
//! to a Renode platform description, and [`boot`] is the judge's half of a
//! boot: the memory a firmware may load into, the boot word's contract, and
//! the register image computed from the topology alone (`INV-07`). Every
//! address and interrupt number comes from ST's header through
//! `aethelgard-svd-hal`, and every stock Renode name from Renode's own files,
//! pinned by hash in [`platform`].
//!
//! `AET-WO-2200` (`RUL-17`): [`analog`] writes each I2C line's netlist for the
//! bridge to measure, and [`platform`] leaves a faulted bus's devices out.
//!
//! `AET-WO-4200`: [`platform`] puts each SPI controller's devices behind a
//! `SPI.SPIMultiplexer`, each chip select a GPIO wired active-low (D1), and can
//! leave one device absent behind its chip select (D4); [`model`] holds each
//! model's witness and skipped lines (D2).
//!
//! `AET-WO-4300b`: [`serial`] holds what the judge expects of each serial
//! port (its pattern, frame, injection instant, bring-up and tags) and the
//! parsers for its capture and Renode's access log; [`boot`] adds each
//! USART's stored
//! registers to the expected image.
//!
//! `AET-WO-4240v`: [`platform`] replaces the stock PWR stub's script on a
//! generated platform ([`platform::pwr_override`], from RM0433), [`serial`]
//! holds the generated bring-up's PWR writes to the declared supply and VOS
//! Scale 2 ([`serial::judge_pwr`]) and states what the twin cannot show of
//! them ([`serial::PWR_RESIDUAL`]), and [`boot`] names the three PWR boot
//! words.
//!
//! `AET-WO-4240b`: [`hse`] holds a generated HSE board's RCC and flash
//! controller accesses to the H1 bring-up ([`hse::judge_clock`]) and states
//! what the twin cannot show of it ([`hse::HSE_RESIDUAL`]); [`platform`] sets
//! the stock RCC's `hseFrequency`; [`boot`] names the five HSE boot words.

#![forbid(unsafe_code)]

pub mod analog;
pub mod boot;
pub mod hse;
pub mod mavlink;
pub mod model;
pub mod pins;
pub mod platform;
pub mod rest;
pub mod serial;
pub mod timer;

use aethelgard_ir::IrValidationError;
use aethelgard_svd_hal::cmsis::RegisterError;

/// One variant per refusal (`INV-06`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TwinError {
    #[error("the solved topology is malformed: {0}")]
    InvalidTopology(IrValidationError),
    #[error("pin `{pin}` is not a GPIO pin name (`P[A-K]` and 0 to 15)")]
    InvalidPin { pin: String },
    #[error(
        "peripheral `{peripheral}` has no Renode model here (I2Cn, SPIn, TIMn with channels, USARTn or UARTn of a serial port, ADC1 or ADC3 of a pin device)"
    )]
    UnsupportedPeripheral { peripheral: String },
    #[error(
        "the DMAMUX1 table (`data/dmamux1-spi-rx.tsv`) has a row that is not ST's peripheral, the resource and a decimal input, tab-separated: `{row}`"
    )]
    MalformedDmaTable { row: String },
    #[error(
        "the MAVLink table (`data/mavlink-messages.tsv`) has a row that is not a name, a decimal id, CRC_EXTRA (at most 255), base and full payload lengths (1 to 255, base at most full) and `in` or `out`, tab-separated: `{row}`"
    )]
    MalformedMessageTable { row: String },
    #[error(
        "the rest table (`data/rest.tsv`) has a row that is not a part, a non-zero amplitude in mPa, a period that is a whole number of the 100 us quantum, and a citation, tab-separated: `{row}`"
    )]
    MalformedRestTable { row: String },
    #[error(
        "`{reference}` is an IMU with no declared orientation, so the board has no rest to start from (AET-WO-4400i D4): declare its `orientation` and `board.forward`"
    )]
    RestOrientationUndeclared { reference: String },
    #[error(
        "`{part}` is a pressure part with no row in the rest table (`data/rest.tsv`): its intrinsic noise is not declared, so the board has no rest to start from"
    )]
    RestNoiseUndeclared { part: String },
    #[error("`{reference}`'s model has no {property} stimulus, which its rest needs")]
    RestUnmodelled {
        reference: String,
        property: &'static str,
    },
    #[error("`{peripheral}` is not in Renode's {file}, and only an SPI instance is added")]
    NotInRenode {
        peripheral: String,
        file: &'static str,
    },
    #[error(
        "Renode's `{name}` is at {renode:#010x} and the header puts `{peripheral}` at {header:#010x}"
    )]
    AddressMismatch {
        peripheral: String,
        name: &'static str,
        renode: u32,
        header: u32,
    },
    #[error("the fault decision names bus `{bus}`, which has no solved analog line")]
    UnknownFaultedBus { bus: String },
    #[error(
        "device `{reference}` cannot be a Renode name (a lower-case letter, then letters, digits or `_`) that no stock instance already has"
    )]
    InvalidDeviceName { reference: String },
    #[error("device `{reference}` is on `{peripheral}`, which the solved topology does not use")]
    DeviceOffBus {
        reference: String,
        peripheral: String,
    },
    #[error("part `{part}` names Renode model `{model}`, which the twin does not model")]
    UnknownModel { part: String, model: String },
    #[error(
        "device `{reference}` is a `{part}`, whose Renode model the twin places only on another kind of bus"
    )]
    ModelOffItsBus { reference: String, part: String },
    #[error(
        "device `{reference}`'s identity is not the 3 bytes (manufacturer, type, capacity 0x10 to 0x19) a `SPI.GenericSpiFlash` is built from"
    )]
    FlashIdentity { reference: String },
    #[error(
        "the fault decision names SPI device `{reference}`, which the solved topology does not have"
    )]
    UnknownAbsentDevice { reference: String },
    #[error(
        "motor output `{label}`: `{signal}` on pin `{pin}` at AF{alternate_function} has no route in Renode's stm32h7.repl, so no edge of the timer would reach the pin"
    )]
    NoTimerRoute {
        label: String,
        signal: String,
        pin: String,
        alternate_function: u8,
    },
    #[error(
        "motor output `{label}`: pin `{pin}` at AF{alternate_function} is one Renode's stm32h7.repl inverts (`invertedAFPins`, a complementary output), so its edges would reach the pin inverted"
    )]
    InvertedRoute {
        label: String,
        pin: String,
        alternate_function: u8,
    },
    #[error("a model of our own is not pinned (AET-WO-4220 D6): {why}")]
    ModelUnpinned { why: String },
    /// `AET-WO-1211c`: a pin device the twin cannot run, and what of it
    /// (`pins`); every other pin device runs, witnessed on the MCU's side.
    #[error("`{signal}`, a pin device's signal, is one the twin cannot run: {why}")]
    PinDeviceNotSimulated { signal: String, why: String },
    /// A scenario holds a net the twin must never drive: a declared output,
    /// or any pin a peripheral drives (`AET-WO-1211` §4 G4: Renode's drive
    /// overwrites `ODR`, with no contention).
    #[error(
        "`{net}` is not a pin device's declared input: holding it would overwrite what the MCU drives on it, which Renode models as no contention (AET-WO-1211 §4 G4)"
    )]
    PinHeldIsOutput { net: String },
    #[error("`{net}` is no signal the specification declares, so no pin is held")]
    PinHeldUnknown { net: String },
    #[error("`{net}` is not a pin device's ADC input, so no channel is set")]
    AnalogValueUnknown { net: String },
    #[error(transparent)]
    Register(#[from] RegisterError),
}

impl TwinError {
    /// The variant name, or the register error's.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::InvalidTopology(_) => "InvalidTopology",
            Self::InvalidPin { .. } => "InvalidPin",
            Self::UnsupportedPeripheral { .. } => "UnsupportedPeripheral",
            Self::MalformedDmaTable { .. } => "MalformedDmaTable",
            Self::MalformedMessageTable { .. } => "MalformedMessageTable",
            Self::MalformedRestTable { .. } => "MalformedRestTable",
            Self::RestOrientationUndeclared { .. } => "RestOrientationUndeclared",
            Self::RestNoiseUndeclared { .. } => "RestNoiseUndeclared",
            Self::RestUnmodelled { .. } => "RestUnmodelled",
            Self::NotInRenode { .. } => "NotInRenode",
            Self::AddressMismatch { .. } => "AddressMismatch",
            Self::UnknownFaultedBus { .. } => "UnknownFaultedBus",
            Self::InvalidDeviceName { .. } => "InvalidDeviceName",
            Self::DeviceOffBus { .. } => "DeviceOffBus",
            Self::UnknownModel { .. } => "UnknownModel",
            Self::ModelOffItsBus { .. } => "ModelOffItsBus",
            Self::ModelUnpinned { .. } => "ModelUnpinned",
            Self::FlashIdentity { .. } => "FlashIdentity",
            Self::UnknownAbsentDevice { .. } => "UnknownAbsentDevice",
            Self::NoTimerRoute { .. } => "NoTimerRoute",
            Self::InvertedRoute { .. } => "InvertedRoute",
            Self::PinDeviceNotSimulated { .. } => "PinDeviceNotSimulated",
            Self::PinHeldIsOutput { .. } => "PinHeldIsOutput",
            Self::PinHeldUnknown { .. } => "PinHeldUnknown",
            Self::AnalogValueUnknown { .. } => "AnalogValueUnknown",
            Self::Register(e) => e.kind(),
        }
    }
}

/// What a solved peripheral is, by its ST name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Gpio,
    I2c,
    Spi,
    /// A timer with channels (`AET-WO-4300a`).
    Timer,
    /// A USART or UART a serial port may use (`AET-WO-4300b`,
    /// `aethelgard_ir::serial::PERIPHERALS`).
    Serial,
    /// ADC1 or ADC3, a pin device's input's (`AET-WO-1211c`, [`pins::ADCS`]).
    Adc,
}

pub(crate) fn kind(peripheral: &str) -> Result<Kind, TwinError> {
    let numbered = |prefix: &str| {
        peripheral
            .strip_prefix(prefix)
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    };
    if peripheral.len() == 5
        && peripheral.starts_with("GPIO")
        && peripheral.as_bytes()[4].is_ascii_uppercase()
    {
        Ok(Kind::Gpio)
    } else if numbered("I2C") {
        Ok(Kind::I2c)
    } else if numbered("SPI") {
        Ok(Kind::Spi)
    } else if numbered("TIM")
        && peripheral[3..]
            .parse()
            .is_ok_and(|n| aethelgard_ir::motor::TIMERS.contains(&n))
    {
        Ok(Kind::Timer)
    } else if aethelgard_ir::serial::PERIPHERALS.contains(&peripheral) {
        Ok(Kind::Serial)
    } else if pins::ADCS.iter().any(|(p, _)| *p == peripheral) {
        Ok(Kind::Adc)
    } else {
        Err(TwinError::UnsupportedPeripheral {
            peripheral: peripheral.to_owned(),
        })
    }
}

/// `PB6` is port `B`, pin 6, by `aethelgard-svd-hal`'s one pin parser.
pub(crate) fn split_pin(pin: &str) -> Result<(char, u32), TwinError> {
    aethelgard_svd_hal::board::pin_name(pin).ok_or_else(|| TwinError::InvalidPin {
        pin: pin.to_owned(),
    })
}

#[cfg(test)]
mod tests;
