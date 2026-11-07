//! Vendor pin data for the STM32H753ZI: ST's `STM32_open_pin_data` files,
//! vendored byte for byte, hash-checked before anything reads them, and parsed
//! into a [`PinTable`] (`docs/architecture.md` §2.8). Never board files
//! (`docs/architecture.md` §3). The rules are in `README.md`.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use aethelgard_ir::{Pin, PinSource, PinTable};
use roxmltree::{Document, Node};
use sha2::{Digest, Sha256};

#[cfg(test)]
mod tests;

const PART: &str = "STM32H753ZITx";
const REPOSITORY: &str = "https://github.com/STMicroelectronics/STM32_open_pin_data";
const COMMIT: &str = "7d1f1514ed5583ec5007ad91236b4e1d377295b1";

const MCU_FILE: &str = "STM32H753ZITx.xml";
const MCU_SHA256: &str = "d167c0884383dd22adc1f25b712c4b12ed1cdf35d898af7edba34ce0e7f98a05";
const GPIO_FILE: &str = "GPIO-STM32H747_gpio_v1_0_Modes.xml";
const GPIO_SHA256: &str = "49f51ae2508a11df6b385b86ed21172ea43478c4d314314db465e0f8d278c246";

const MCU_BYTES: &[u8] = include_bytes!("../data/st-open-pin-data/STM32H753ZITx.xml");
const GPIO_BYTES: &[u8] =
    include_bytes!("../data/st-open-pin-data/GPIO-STM32H747_gpio_v1_0_Modes.xml");

/// One variant per refusal (`INV-06`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PinTableError {
    #[error("no pin table for part `{part}`; only `STM32H753ZITx` is known")]
    UnknownPart { part: String },
    #[error("vendored file `{file}` has sha256 {actual}, not the pinned {expected}")]
    SourceHashMismatch {
        file: &'static str,
        expected: &'static str,
        actual: String,
    },
    #[error("malformed pin source: {detail}")]
    MalformedSource { detail: String },
}

impl PinTableError {
    /// The variant name.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::UnknownPart { .. } => "UnknownPart",
            Self::SourceHashMismatch { .. } => "SourceHashMismatch",
            Self::MalformedSource { .. } => "MalformedSource",
        }
    }
}

fn malformed<T>(detail: impl Into<String>) -> Result<T, PinTableError> {
    Err(PinTableError::MalformedSource {
        detail: detail.into(),
    })
}

/// The pin table of `part`. Only `STM32H753ZITx` is known.
pub fn pin_table(part: &str) -> Result<PinTable, PinTableError> {
    if part != PART {
        return Err(PinTableError::UnknownPart { part: part.into() });
    }
    build(MCU_BYTES, GPIO_BYTES)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

fn check_hash(
    file: &'static str,
    expected: &'static str,
    bytes: &[u8],
) -> Result<String, PinTableError> {
    let actual = sha256_hex(bytes);
    if actual == expected {
        Ok(actual)
    } else {
        Err(PinTableError::SourceHashMismatch {
            file,
            expected,
            actual,
        })
    }
}

/// Checks both hashes against the pinned constants, then parses.
fn build(mcu: &[u8], gpio: &[u8]) -> Result<PinTable, PinTableError> {
    let mcu_hash = check_hash(MCU_FILE, MCU_SHA256, mcu)?;
    let gpio_hash = check_hash(GPIO_FILE, GPIO_SHA256, gpio)?;
    let source = PinSource {
        repository: REPOSITORY.into(),
        commit: COMMIT.into(),
        files: BTreeMap::from([(MCU_FILE.into(), mcu_hash), (GPIO_FILE.into(), gpio_hash)]),
    };
    parse(mcu, gpio, source)
}

fn text(bytes: &[u8], file: &str) -> Result<String, PinTableError> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text.into()),
        Err(_) => malformed(format!("{file} is not UTF-8")),
    }
}

fn document<'a>(text: &'a str, file: &str) -> Result<Document<'a>, PinTableError> {
    // roxmltree's defaults refuse a DTD.
    match Document::parse(text) {
        Ok(doc) => Ok(doc),
        Err(e) => malformed(format!("{file}: {e}")),
    }
}

fn children<'a, 'i: 'a>(node: Node<'a, 'i>, name: &'a str) -> impl Iterator<Item = Node<'a, 'i>> {
    node.children()
        .filter(move |n| n.is_element() && n.tag_name().name() == name)
}

/// The leading run matching `P[A-K][0-9]{1,2}`. A third digit (`PA123`) makes
/// no such run, so it refuses rather than reading `PA12`.
fn canonical(name: &str) -> Result<&str, PinTableError> {
    let b = name.as_bytes();
    let digits = b.iter().skip(2).take_while(|d| d.is_ascii_digit()).count();
    let port = b.get(1).is_some_and(|p| (b'A'..=b'K').contains(p));
    if b.first() == Some(&b'P') && port && (1..=2).contains(&digits) {
        Ok(&name[..2 + digits])
    } else {
        malformed(format!("I/O pin `{name}` has no canonical name"))
    }
}

/// `GPIO_AF<n>_<anything>` with `n` in 0..=15, no leading zero.
fn af_number(value: &str, pin: &str, signal: &str) -> Result<u8, PinTableError> {
    let parsed = value.strip_prefix("GPIO_AF").and_then(|rest| {
        let (n, tail) = rest.split_once('_')?;
        let plain = n.bytes().all(|d| d.is_ascii_digit()) && (n == "0" || !n.starts_with('0'));
        let n: u8 = n.parse().ok().filter(|n| plain && *n <= 15)?;
        (!tail.is_empty()).then_some(n)
    });
    match parsed {
        Some(n) => Ok(n),
        None => malformed(format!(
            "{pin} {signal}: `{value}` is not GPIO_AF<n>_<name>"
        )),
    }
}

fn parse(mcu: &[u8], gpio: &[u8], source: PinSource) -> Result<PinTable, PinTableError> {
    let (mcu_text, gpio_text) = (text(mcu, MCU_FILE)?, text(gpio, GPIO_FILE)?);
    let (mcu_doc, gpio_doc) = (
        document(&mcu_text, MCU_FILE)?,
        document(&gpio_text, GPIO_FILE)?,
    );

    let root = mcu_doc.root_element();
    if root.tag_name().name() != "Mcu" || root.attribute("RefName") != Some(PART) {
        return malformed(format!("{MCU_FILE} is not the <Mcu> of {PART}"));
    }
    let Some(package) = root.attribute("Package") else {
        return malformed(format!("{MCU_FILE} has no Package"));
    };

    let mut gpio_pins = BTreeMap::new();
    for pin in children(gpio_doc.root_element(), "GPIO_Pin") {
        let Some(name) = pin.attribute("Name") else {
            return malformed(format!("{GPIO_FILE} has a <GPIO_Pin> with no Name"));
        };
        if gpio_pins.insert(name, pin).is_some() {
            return malformed(format!("{GPIO_FILE} has <GPIO_Pin> `{name}` twice"));
        }
    }

    let mut pins = BTreeMap::new();
    let mut positions = BTreeSet::new();
    for pin in children(root, "Pin").filter(|p| p.attribute("Type") == Some("I/O")) {
        let Some(full) = pin.attribute("Name") else {
            return malformed(format!("{MCU_FILE} has an I/O <Pin> with no Name"));
        };
        let name = canonical(full)?;
        let Some(position) = pin
            .attribute("Position")
            .and_then(|p| p.parse::<u16>().ok())
        else {
            return malformed(format!("pin `{full}` has no numeric Position"));
        };
        if !positions.insert(position) {
            return malformed(format!("pin `{full}` repeats position {position}"));
        }
        let signals: BTreeSet<&str> = children(pin, "Signal")
            .filter_map(|s| s.attribute("Name"))
            .collect();
        let Some(gpio_pin) = gpio_pins.get(full) else {
            return malformed(format!("pin `{full}` is not in {GPIO_FILE}"));
        };

        let mut alternate_functions: BTreeMap<String, u8> = BTreeMap::new();
        for signal in children(*gpio_pin, "PinSignal") {
            let Some(signal_name) = signal.attribute("Name") else {
                return malformed(format!("pin `{full}` has a <PinSignal> with no Name"));
            };
            if !signals.contains(signal_name) {
                continue;
            }
            let values = children(signal, "SpecificParameter")
                .filter(|p| p.attribute("Name") == Some("GPIO_AF"))
                .flat_map(|p| children(p, "PossibleValue"));
            for value in values {
                let af = af_number(value.text().unwrap_or(""), name, signal_name)?;
                if alternate_functions
                    .insert(signal_name.into(), af)
                    .is_some_and(|before| before != af)
                {
                    return malformed(format!("{name} {signal_name} has two AF numbers"));
                }
            }
        }
        let pin = Pin {
            position,
            alternate_functions,
        };
        if pins.insert(name.to_owned(), pin).is_some() {
            return malformed(format!("two I/O pins reduce to `{name}`"));
        }
    }

    let table = PinTable {
        part: PART.into(),
        package: package.into(),
        source,
        pins,
    };
    if let Err(e) = table.validate() {
        return malformed(format!("{e}"));
    }
    Ok(table)
}
