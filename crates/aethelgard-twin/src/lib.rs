//! Simulator platform descriptions, the analog model, and the lock-step bridge; the specification is read-only here (`docs/architecture.md` §3).
//!
//! `AET-WO-2100` (ruled on PR #56, R6): [`platform`] lowers a solved topology
//! to a Renode platform description, and [`boot`] is the judge's half of a
//! boot: the memory a firmware may load into, the boot word's contract, and
//! the register image computed from the topology alone (`INV-07`). Every
//! address and interrupt number comes from ST's header through
//! `aethelgard-svd-hal`, and every stock Renode name from Renode's own files,
//! pinned by hash in [`platform`].

#![forbid(unsafe_code)]

pub mod boot;
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

/// `PB6` is port `B`, pin 6.
pub(crate) fn split_pin(pin: &str) -> Result<(char, u32), TwinError> {
    let bad = || TwinError::InvalidPin {
        pin: pin.to_owned(),
    };
    let rest = pin.strip_prefix('P').ok_or_else(bad)?;
    let mut chars = rest.chars();
    let port = chars
        .next()
        .filter(|c| ('A'..='K').contains(c))
        .ok_or_else(bad)?;
    let digits = chars.as_str();
    if digits.is_empty() || digits.len() > 2 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    if digits.len() == 2 && digits.starts_with('0') {
        return Err(bad());
    }
    let index: u32 = digits.parse().map_err(|_| bad())?;
    if index > 15 {
        return Err(bad());
    }
    Ok((port, index))
}

#[cfg(test)]
mod tests;
