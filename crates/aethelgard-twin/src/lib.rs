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

#![forbid(unsafe_code)]

pub mod analog;
pub mod boot;
pub mod model;
pub mod platform;

use aethelgard_ir::IrValidationError;
use aethelgard_svd_hal::cmsis::RegisterError;

/// One variant per refusal (`INV-06`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TwinError {
    #[error("the solved topology is malformed: {0}")]
    InvalidTopology(IrValidationError),
    #[error("pin `{pin}` is not a GPIO pin name (`P[A-K]` and 0 to 15)")]
    InvalidPin { pin: String },
    #[error("peripheral `{peripheral}` has no Renode model here (I2Cn, SPIn)")]
    UnsupportedPeripheral { peripheral: String },
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
            Self::NotInRenode { .. } => "NotInRenode",
            Self::AddressMismatch { .. } => "AddressMismatch",
            Self::UnknownFaultedBus { .. } => "UnknownFaultedBus",
            Self::InvalidDeviceName { .. } => "InvalidDeviceName",
            Self::DeviceOffBus { .. } => "DeviceOffBus",
            Self::UnknownModel { .. } => "UnknownModel",
            Self::ModelOffItsBus { .. } => "ModelOffItsBus",
            Self::FlashIdentity { .. } => "FlashIdentity",
            Self::UnknownAbsentDevice { .. } => "UnknownAbsentDevice",
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
