//! The Renode platform description of a solved topology (`AET-WO-2100` §1.1).
//!
//! The description includes Renode 1.17.0's `stm32h753.repl` by a root-relative
//! path (spike F1: a file-relative include breaks once `simulate` copies the
//! file) and adds only what the topology solved and Renode lacks: an SPI
//! instance other than `spi4`, at the header's base, with its interrupt wired
//! (spike F2). It never redeclares a stock instance, since a duplicate
//! registration breaks the load (`architecture.md` §2.9). The output is a
//! function of the topology and the pinned header alone (`INV-01`).

use std::collections::BTreeSet;
use std::fmt::Write as _;

use aethelgard_ir::SolvedTopology;
use aethelgard_svd_hal::cmsis::{self, RegisterFacts};

use crate::{Kind, TwinError, kind, split_pin};

/// The Renode release the stock table below was read from.
pub const RENODE_VERSION: &str = "1.17.0";

/// The include chain of `stm32h753.repl` in Renode 1.17.0, each with its
/// sha256. The tier-2 test checks the installed files against these.
pub const RENODE_FILES: [(&str, &str); 3] = [
    (
        "platforms/cpus/stm32h753.repl",
        "13c5571ea3c2a66d23d7dc5e49d09c6013d31c946abdbfc450aebb08929d10ce",
    ),
    (
        "platforms/cpus/stm32h743.repl",
        "86b2c2c5f524a3f2b65da8ce20101c7a83690b1fb8c98a8cff96d399c15f231c",
    ),
    (
        STOCK_FILE,
        "b85cb4af7a7e98f784ed822941ddc2ec4ddc0c3928c3efe7ef7bb4702764251e",
    ),
];

/// The file that declares every stock instance this emitter relies on.
pub const STOCK_FILE: &str = "platforms/cpus/stm32h7.repl";

/// The stock instances of [`STOCK_FILE`] a solved peripheral can be, as
/// (ST name, Renode name, Renode model, Renode's `sysbus` address), read from
/// the file at the hash in [`RENODE_FILES`].
pub const STOCK: [(&str, &str, &str, u32); 16] = [
    ("GPIOA", "gpioPortA", "GPIOPort.STM32_GPIOPort", 0x5802_0000),
    ("GPIOB", "gpioPortB", "GPIOPort.STM32_GPIOPort", 0x5802_0400),
    ("GPIOC", "gpioPortC", "GPIOPort.STM32_GPIOPort", 0x5802_0800),
    ("GPIOD", "gpioPortD", "GPIOPort.STM32_GPIOPort", 0x5802_0C00),
    ("GPIOE", "gpioPortE", "GPIOPort.STM32_GPIOPort", 0x5802_1000),
    ("GPIOF", "gpioPortF", "GPIOPort.STM32_GPIOPort", 0x5802_1400),
    ("GPIOG", "gpioPortG", "GPIOPort.STM32_GPIOPort", 0x5802_1800),
    ("GPIOH", "gpioPortH", "GPIOPort.STM32_GPIOPort", 0x5802_1C00),
    ("GPIOI", "gpioPortI", "GPIOPort.STM32_GPIOPort", 0x5802_2000),
    ("GPIOJ", "gpioPortJ", "GPIOPort.STM32_GPIOPort", 0x5802_2400),
    ("GPIOK", "gpioPortK", "GPIOPort.STM32_GPIOPort", 0x5802_2800),
    ("I2C1", "i2c1", "I2C.STM32F7_I2C", 0x4000_5400),
    ("I2C2", "i2c2", "I2C.STM32F7_I2C", 0x4000_5800),
    ("I2C3", "i2c3", "I2C.STM32F7_I2C", 0x4000_5C00),
    ("I2C4", "i2c4", "I2C.STM32F7_I2C", 0x5800_1C00),
    ("SPI4", "spi4", "SPI.STM32H7_SPI", 0x4001_3400),
];

/// The model an added SPI instance takes: `spi4`'s, from [`STOCK_FILE`].
const SPI_MODEL: &str = "SPI.STM32H7_SPI";

/// One solved peripheral as the platform has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// ST's name, such as `SPI1` or `GPIOA`.
    pub peripheral: String,
    /// Renode's name, such as `spi1` or `gpioPortA`.
    pub name: String,
    /// Renode's model, without its namespace, as `peripherals` lists it.
    pub model: String,
    /// True when the emitted description adds it; false for a stock instance.
    pub added: bool,
}

/// Every solved peripheral and port, with the name Renode lists it by.
pub fn instances(topology: &SolvedTopology) -> Result<Vec<Instance>, TwinError> {
    topology.validate().map_err(TwinError::InvalidTopology)?;
    let facts = cmsis::register_facts()?;
    solved(topology)?
        .iter()
        .map(|p| instance(p, &facts))
        .collect()
}

/// The `.repl` text for `topology`.
pub fn emit(topology: &SolvedTopology) -> Result<String, TwinError> {
    emit_faulted(topology, &BTreeSet::new())
}

/// The `.repl` text for `topology` with the analog pass's fault decision:
/// each bus in `faulted` has its devices left out, which is how a bus fault
/// reaches the firmware (`RUL-17`, `AET-WO-2101` route iii). The decision is
/// written into the provenance comment, so a platform emitted without it
/// differs from one emitted with it. A faulted bus that the topology did not
/// solve is refused.
pub fn emit_faulted(
    topology: &SolvedTopology,
    faulted: &BTreeSet<String>,
) -> Result<String, TwinError> {
    let instances = instances(topology)?;
    let mut omitted = Vec::new();
    for bus in faulted {
        let peripheral = topology
            .analog_lines
            .iter()
            .find(|(_, line)| &line.bus == bus)
            .and_then(|(net, _)| topology.assignments.get(net))
            .map(|a| a.peripheral.as_str())
            .ok_or_else(|| TwinError::UnknownFaultedBus { bus: bus.clone() })?;
        omitted.push(format!(
            "\n// - Analog fault (AET-WO-2200): bus `{bus}` ({peripheral}) measured past its rise-time\n\
             //   ceiling, so its devices are left out of this platform."
        ));
    }
    let facts = cmsis::register_facts()?;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "// @generated by aethelgard-twin (AET-WO-2100). Do not edit: the regeneration\n\
         // test compares every byte of this file with the emitter's output.\n\
         //\n\
         // - Solved topology of specification sha256 `{spec}`.\n\
         // - Renode {RENODE_VERSION}: `{STOCK_FILE}` sha256 `{stock}`.\n\
         // - Register facts: `{repo}` at `{commit}`, `{file}` sha256 `{sha}`.",
        spec = topology.specification_sha256,
        stock = RENODE_FILES[2].1,
        repo = cmsis::REPOSITORY,
        commit = cmsis::COMMIT,
        file = cmsis::HEADER_FILE,
        sha = cmsis::HEADER_SHA256,
    );
    for line in &omitted {
        out.pop();
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("using \"platforms/cpus/stm32h753.repl\"\n");
    for i in instances.iter().filter(|i| i.added) {
        let base = facts.base(&i.peripheral)?;
        let irq = facts.irq(&i.peripheral)?;
        let _ = write!(
            out,
            "\n// {p}: {p}_BASE and {p}_IRQn from the header; the model is `spi4`'s.\n\
             {name}: {SPI_MODEL} @ {{\n        sysbus 0x{base:08X};\n        rcc RegistrationPeripheralAlias.{p}\n    }}\n    IRQ -> nvic@{irq}\n",
            p = i.peripheral,
            name = i.name,
        );
    }
    Ok(out)
}

/// The solved peripherals and the ports of every solved pin, in name order.
fn solved(topology: &SolvedTopology) -> Result<BTreeSet<String>, TwinError> {
    let mut set = BTreeSet::new();
    for a in topology.assignments.values() {
        set.insert(a.peripheral.clone());
        set.insert(format!("GPIO{}", split_pin(&a.pin)?.0));
    }
    for pin in topology.gpio.values() {
        set.insert(format!("GPIO{}", split_pin(pin)?.0));
    }
    Ok(set)
}

fn instance(peripheral: &str, facts: &RegisterFacts) -> Result<Instance, TwinError> {
    let kind = kind(peripheral)?;
    let header = facts.base(peripheral)?;
    if let Some(&(_, name, model, renode)) = STOCK.iter().find(|s| s.0 == peripheral) {
        if renode != header {
            return Err(TwinError::AddressMismatch {
                peripheral: peripheral.to_owned(),
                name,
                renode,
                header,
            });
        }
        return Ok(Instance {
            peripheral: peripheral.to_owned(),
            name: name.to_owned(),
            model: short(model).to_owned(),
            added: false,
        });
    }
    // belt-and-suspenders: every I2C and GPIO instance of the pinned H753
    // header is stock, so this cannot fire today (mutant M15 survives); it
    // stops a header with another one being emitted under the SPI model.
    if kind != Kind::Spi {
        return Err(TwinError::NotInRenode {
            peripheral: peripheral.to_owned(),
            file: STOCK_FILE,
        });
    }
    Ok(Instance {
        peripheral: peripheral.to_owned(),
        name: peripheral.to_ascii_lowercase(),
        model: short(SPI_MODEL).to_owned(),
        added: true,
    })
}

/// `SPI.STM32H7_SPI` is listed as `STM32H7_SPI`.
fn short(model: &str) -> &str {
    model.rsplit('.').next().unwrap_or(model)
}
