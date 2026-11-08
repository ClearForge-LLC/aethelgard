//! AET-WO-2100 spike S1: lower a `SolvedTopology` to a Renode platform
//! description, and say where every name, address and interrupt line came from.
//!
//! `emit <solved.json> <renode-root> <st-header>` writes the `.repl` to stdout
//! and one `PROVENANCE` line per fact to stderr. Nothing is from memory:
//! - names and models of stock instances: Renode's own `platforms/cpus` files;
//! - base addresses: `aethelgard-svd-hal`'s hash-checked ST header;
//! - interrupt numbers: the `<P>_IRQn` line of the same header, hash-checked here.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::process::ExitCode;

use aethelgard_ir::SolvedTopology;
use sha2::{Digest, Sha256};

/// The pin `cmsis.rs` pins (`HEADER_SHA256`, which is `pub(crate)`).
const HEADER_SHA256: &str = "4f6def439a9fa94705303fe227d34308048c44bd36263d311aa54cc94f8398f5";
/// The include chain of `stm32h753.repl` in Renode 1.17.0.
const CHAIN: [&str; 3] = [
    "platforms/cpus/stm32h753.repl",
    "platforms/cpus/stm32h743.repl",
    "platforms/cpus/stm32h7.repl",
];

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// The Renode model each solved peripheral kind is lowered to.
fn model(peripheral: &str) -> Option<&'static str> {
    if peripheral.starts_with("I2C") {
        Some("I2C.STM32F7_I2C")
    } else if peripheral.starts_with("SPI") {
        Some("SPI.STM32H7_SPI")
    } else if peripheral.starts_with("GPIO") {
        Some("GPIOPort.STM32_GPIOPort")
    } else {
        None
    }
}

/// Renode's name for an ST instance: `I2C1` is `i2c1`, `GPIOA` is `gpioPortA`.
fn renode_name(peripheral: &str) -> String {
    match peripheral.strip_prefix("GPIO") {
        Some(port) => format!("gpioPort{port}"),
        None => peripheral.to_ascii_lowercase(),
    }
}

fn run() -> Result<String, String> {
    let args: Vec<String> = std::env::args().collect();
    let [_, solved, root, header] = args.as_slice() else {
        return Err("usage: emit <solved.json> <renode-root> <st-header>".into());
    };
    let text = std::fs::read_to_string(solved).map_err(|e| format!("{solved}: {e}"))?;
    let topology: SolvedTopology =
        serde_json::from_str(&text).map_err(|e| format!("{solved}: {e}"))?;

    // Every stock instance in the include chain: `name: Model.Type @ ...`.
    let mut stock = BTreeMap::new();
    for file in CHAIN {
        let path = format!("{root}/{file}");
        let bytes = std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
        eprintln!("SOURCE {file} sha256 {}", hex(&bytes));
        for (n, line) in String::from_utf8_lossy(&bytes).lines().enumerate() {
            let Some((name, rest)) = line.split_once(": ") else {
                continue;
            };
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
                continue;
            }
            let Some((kind, _)) = rest.split_once(" @") else {
                continue;
            };
            // The first `sysbus 0x…` within the entry's registration block.
            let addr = String::from_utf8_lossy(&bytes)
                .lines()
                .skip(n)
                .take(4)
                .find_map(|l| {
                    let at = l.find("sysbus ")? + "sysbus ".len();
                    let v = l[at..].trim_start_matches('<');
                    let v = v.strip_prefix("0x")?;
                    let end = v.find(|c: char| !c.is_ascii_hexdigit()).unwrap_or(v.len());
                    u32::from_str_radix(&v[..end], 16).ok()
                });
            stock.insert(
                name.to_owned(),
                (kind.to_owned(), format!("{file}:{}", n + 1), addr),
            );
        }
    }

    let bytes = std::fs::read(header).map_err(|e| format!("{header}: {e}"))?;
    if hex(&bytes) != HEADER_SHA256 {
        return Err(format!(
            "{header}: sha256 is not the pinned {HEADER_SHA256}"
        ));
    }
    let header_text = String::from_utf8_lossy(&bytes).into_owned();
    let facts = aethelgard_svd_hal::cmsis::register_facts().map_err(|e| e.to_string())?;

    // The solved peripherals, and the GPIO port of every solved pin.
    let mut solved_set: BTreeSet<String> = BTreeSet::new();
    for a in topology.assignments.values() {
        solved_set.insert(a.peripheral.clone());
        solved_set.insert(format!("GPIO{}", &a.pin[1..2]));
    }
    for pin in topology.gpio.values() {
        solved_set.insert(format!("GPIO{}", &pin[1..2]));
    }

    let mut repl = String::from("using \"platforms/cpus/stm32h753.repl\"\n");
    for p in &solved_set {
        let want = model(p).ok_or_else(|| format!("{p}: no Renode model for this kind"))?;
        let name = renode_name(p);
        let base = facts.base(p).map_err(|e| e.to_string())?;
        eprintln!("PROVENANCE {p} base 0x{base:08X} from stm32h753xx.h {p}_BASE");
        match stock.get(&name) {
            Some((kind, at, addr)) if kind == want => {
                if *addr != Some(base) {
                    return Err(format!(
                        "{p}: Renode's `{name}` at {at} is at {addr:x?}, the header says 0x{base:08X}"
                    ));
                }
                eprintln!(
                    "PROVENANCE {p} name `{name}` model {kind} stock at {at}, Renode address equals the header's"
                );
            }
            Some((kind, at, _)) => {
                return Err(format!(
                    "{p}: Renode's `{name}` at {at} is {kind}, not {want}"
                ));
            }
            None => {
                let irq = header_text
                    .lines()
                    .find_map(|l| {
                        let (lhs, rhs) = l.trim().split_once('=')?;
                        let value = rhs.split(',').next()?.trim();
                        (lhs.trim() == format!("{p}_IRQn")
                            && value.chars().all(|c| c.is_ascii_digit()))
                        .then(|| value.to_owned())
                    })
                    .ok_or_else(|| format!("{p}: header has no {p}_IRQn"))?;
                eprintln!(
                    "PROVENANCE {p} name `{name}` model {want} added (not in Renode's chain)"
                );
                eprintln!("PROVENANCE {p} interrupt nvic@{irq} from stm32h753xx.h {p}_IRQn");
                eprintln!(
                    "PROVENANCE {p} rcc alias RegistrationPeripheralAlias.{p} (ST's name; Renode refuses an unknown alias, E21)"
                );
                let _ = write!(
                    repl,
                    "\n{name}: {want} @ {{\n        sysbus 0x{base:08X};\n        rcc RegistrationPeripheralAlias.{p}\n    }}\n    IRQ -> nvic@{irq}\n"
                );
            }
        }
    }
    Ok(repl)
}

fn main() -> ExitCode {
    match run() {
        Ok(repl) => {
            print!("{repl}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
