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

use aethelgard_ir::parts::BusKind;
use aethelgard_ir::{MEMORY_SUFFIX, SolvedI2cDevice, SolvedTopology};
use aethelgard_svd_hal::cmsis::{self, RegisterFacts};

use crate::model::{self, Model};
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

/// Every solved peripheral and port, with the name Renode lists it by, then
/// each modelled I2C device (`AET-WO-2202`) of the topology that is not on a
/// bus in `faulted`, then each SPI controller's multiplexer and each modelled
/// SPI device not in `absent` (`AET-WO-4200`), added by the emitted description.
pub fn instances(
    topology: &SolvedTopology,
    faulted: &BTreeSet<String>,
    absent: &BTreeSet<String>,
) -> Result<Vec<Instance>, TwinError> {
    let mut out = stock_instances(topology)?;
    let (modelled, _) = split_devices(topology, &present(topology, faulted)?)?;
    out.extend(modelled.into_iter().map(|d| Instance {
        peripheral: d.reference,
        name: d.name,
        model: short(d.model).to_owned(),
        added: true,
    }));
    for (controller, devices) in spi_buses(topology, absent)? {
        out.push(Instance {
            peripheral: format!("{}MUX", controller.to_ascii_uppercase()),
            name: mux_name(&controller),
            model: short(MUX_MODEL).to_owned(),
            added: true,
        });
        for d in devices {
            if let (Some(m), false) = (d.model, d.absent) {
                out.push(Instance {
                    peripheral: d.reference,
                    name: d.name,
                    model: short(m.renode()).to_owned(),
                    added: true,
                });
            }
        }
    }
    Ok(out)
}

/// The model of each SPI controller's chip-select multiplexer (`AET-WO-4200` D1).
const MUX_MODEL: &str = "SPI.SPIMultiplexer";

/// The name the emitter declares the multiplexer of the SPI controller
/// Renode names `controller` (`spi1`) under: `spi1mux`, which
/// `aethelgard_ir::RESERVED_REFERENCES` holds for every SPI controller of the
/// pinned header (`AET-WO-4200` D7, `tests/reserved_names.rs`).
pub fn mux_name(controller: &str) -> String {
    format!("{controller}mux")
}

/// One SPI device the emitted description wires behind its controller's
/// multiplexer.
struct SpiWired {
    reference: String,
    /// The reference, lower-cased: the name Renode declares.
    name: String,
    part: String,
    /// `None`: unmodelled; its chip select is wired, and nothing answers it.
    model: Option<Model>,
    /// The mux input its chip select drives, and the address it sits at.
    address: u8,
    /// Its chip select's pin: the port's Renode name and the pin's index.
    port: &'static str,
    pin: u32,
    pin_name: String,
    /// Left out behind its chip select by the fault decision (`AET-WO-4200` D4).
    absent: bool,
}

/// Each SPI controller with a device, by its Renode name (`spi1`), with its
/// devices in the specification's order, each at its position on that
/// controller as its mux address. A device in `absent` is kept, marked, so
/// its chip select is still wired; a reference in `absent` that is not an SPI
/// device is refused.
fn spi_buses(
    topology: &SolvedTopology,
    absent: &BTreeSet<String>,
) -> Result<std::collections::BTreeMap<String, Vec<SpiWired>>, TwinError> {
    if let Some(r) = absent
        .iter()
        .find(|r| !topology.spi_devices.contains_key(*r))
    {
        return Err(TwinError::UnknownAbsentDevice {
            reference: r.clone(),
        });
    }
    let used = solved(topology)?;
    let mut taken: BTreeSet<String> = used.iter().map(|p| p.to_ascii_lowercase()).collect();
    taken.extend(topology.i2c_devices.keys().map(|r| r.to_ascii_lowercase()));
    let mut out: std::collections::BTreeMap<String, Vec<SpiWired>> = Default::default();
    for (reference, d) in topology.spi_devices_in_order() {
        let name = reference.to_ascii_lowercase();
        let ident = name.starts_with(|c: char| c.is_ascii_lowercase())
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if !ident || !taken.insert(name.clone()) {
            return Err(TwinError::InvalidDeviceName {
                reference: reference.to_owned(),
            });
        }
        if kind(&d.peripheral)? != Kind::Spi || !used.contains(&d.peripheral) {
            return Err(TwinError::DeviceOffBus {
                reference: reference.to_owned(),
                peripheral: d.peripheral.clone(),
            });
        }
        let model = model::kind(&d.part)?;
        if model.is_some_and(|m| m.bus() != BusKind::Spi) {
            return Err(TwinError::ModelOffItsBus {
                reference: reference.to_owned(),
                part: d.part.clone(),
            });
        }
        let pin_name = topology
            .gpio
            .get(&d.cs)
            .ok_or_else(|| {
                TwinError::InvalidTopology(aethelgard_ir::IrValidationError::SpiDeviceSelect {
                    id: reference.to_owned(),
                })
            })?
            .clone();
        let (port, pin) = split_pin(&pin_name)?;
        let port = STOCK
            .iter()
            .find(|s| s.0 == format!("GPIO{port}"))
            .map(|s| s.1)
            .ok_or_else(|| TwinError::InvalidPin {
                pin: pin_name.clone(),
            })?;
        let devices = out.entry(d.peripheral.to_ascii_lowercase()).or_default();
        let address = u8::try_from(devices.len()).map_err(|_| TwinError::InvalidDeviceName {
            reference: reference.to_owned(),
        })?;
        devices.push(SpiWired {
            reference: reference.to_owned(),
            name,
            part: d.part.clone(),
            model,
            address,
            port,
            pin,
            pin_name,
            absent: absent.contains(reference),
        });
    }
    Ok(out)
}

/// Each SPI device in the specification's order, as (reference, peripheral,
/// part, mux address, chip-select pin): the wiring [`emit`] declares, which
/// the judge's checks name (`AET-WO-4200` D1).
pub fn spi_wiring(
    topology: &SolvedTopology,
) -> Result<Vec<(String, String, String, u8, String)>, TwinError> {
    let buses = spi_buses(topology, &BTreeSet::new())?;
    let mut out = Vec::new();
    for (reference, d) in topology.spi_devices_in_order() {
        let w = buses
            .values()
            .flatten()
            .find(|w| w.reference == reference)
            .ok_or_else(|| TwinError::InvalidDeviceName {
                reference: reference.to_owned(),
            })?;
        out.push((
            reference.to_owned(),
            d.peripheral.clone(),
            d.part.clone(),
            w.address,
            w.pin_name.clone(),
        ));
    }
    Ok(out)
}

/// The path Renode gives a modelled SPI device on `peripheral`,
/// `spi1.spi1mux.u4`: the controller, its multiplexer, then the emitted name.
pub fn spi_device_path(peripheral: &str, reference: &str) -> Result<String, TwinError> {
    if kind(peripheral)? != Kind::Spi {
        return Err(TwinError::DeviceOffBus {
            reference: reference.to_owned(),
            peripheral: peripheral.to_owned(),
        });
    }
    let c = peripheral.to_ascii_lowercase();
    Ok(format!(
        "{c}.{}.{}",
        mux_name(&c),
        reference.to_ascii_lowercase()
    ))
}

/// The text that declares each SPI controller's multiplexer, the chip-select
/// wiring (one block per port), then each modelled device present.
fn emit_spi(
    out: &mut String,
    buses: &std::collections::BTreeMap<String, Vec<SpiWired>>,
) -> Result<(), TwinError> {
    let mut ports: std::collections::BTreeMap<&str, Vec<(u32, String)>> = Default::default();
    for (c, devices) in buses {
        let _ = write!(
            out,
            "\n// {C}'s chip selects (AET-WO-4200 D1): Renode {RENODE_VERSION}'s `{MUX_MODEL}`, input k\n\
             // active-low and the device at address k selected by it.\n\
             {m}: {MUX_MODEL} @ {c}\n    init:\n",
            m = mux_name(c),
            C = c.to_ascii_uppercase(),
        );
        for d in devices {
            let _ = writeln!(out, "        SetActiveLow {}", d.address);
            ports
                .entry(d.port)
                .or_default()
                .push((d.pin, format!("{c}mux@{}", d.address)));
        }
    }
    for (port, mut lines) in ports {
        lines.sort();
        let _ = writeln!(out, "\n{port}:");
        for (pin, to) in lines {
            let _ = writeln!(out, "    {pin} -> {to}");
        }
    }
    for (c, devices) in buses {
        for d in devices.iter().filter(|d| !d.absent) {
            let (r, n, k) = (&d.reference, &d.name, d.address);
            match d.model {
                None => {
                    let _ = write!(
                        out,
                        "\n// {r} ({}): no Renode {RENODE_VERSION} model; its chip select ({}) is wired to\n\
                         // {c}mux@{k}, and nothing answers it.\n",
                        d.part, d.pin_name
                    );
                }
                Some(Model::GenericSpiFlash) => {
                    let id = aethelgard_ir::parts::identity(&d.part)
                        .map_err(TwinError::InvalidTopology)?
                        .filter(|id| id.width == 3 && (0x10..=0x19).contains(&(id.value & 0xFF)))
                        .ok_or_else(|| TwinError::FlashIdentity {
                            reference: r.clone(),
                        })?;
                    let [_, maker, kind, capacity] = id.value.to_be_bytes();
                    let _ = write!(
                        out,
                        "\n// {r}: Renode {RENODE_VERSION}'s `{m}`, its JEDEC ID the parts table's, at\n\
                         // {c}mux@{k}, chip select {pin}; its array is 2^capacity bytes.\n\
                         {n}{MEMORY_SUFFIX}: Memory.MappedMemory\n    size: 0x{size:X}\n\
                         \n{n}: {m} @ {c}mux {k}\n    underlyingMemory: {n}{MEMORY_SUFFIX}\n    \
                         manufacturerId: 0x{maker:02X}\n    memoryType: 0x{kind:02X}\n    capacityCode: 0x{capacity:02X}\n",
                        m = Model::GenericSpiFlash.renode(),
                        pin = d.pin_name,
                        size = 1u64 << capacity,
                    );
                }
                Some(m) => {
                    let _ = write!(
                        out,
                        "\n// {r}: Renode {RENODE_VERSION}'s `{m}`, at {c}mux@{k}, chip select {pin}.\n\
                         {n}: {m} @ {c}mux {k}\n",
                        m = m.renode(),
                        pin = d.pin_name,
                    );
                }
            }
        }
    }
    Ok(())
}

/// Every solved peripheral and port, with the name Renode lists it by.
fn stock_instances(topology: &SolvedTopology) -> Result<Vec<Instance>, TwinError> {
    topology.validate().map_err(TwinError::InvalidTopology)?;
    let facts = cmsis::register_facts()?;
    solved(topology)?
        .iter()
        .map(|p| instance(p, &facts))
        .collect()
}

/// One I2C device the emitted description declares.
struct Modelled {
    reference: String,
    name: String,
    model: &'static str,
    /// Renode's name of the controller, such as `i2c1`.
    bus: &'static str,
    address: u8,
}

/// The devices whose part has a model, in name order, and the rest, in the
/// order given. A name that cannot be a Renode identifier, or that a solved
/// peripheral or another device already has, is refused: a duplicate
/// registration breaks the load (`architecture.md` §2.9).
fn split_devices<'d>(
    topology: &SolvedTopology,
    devices: &[Device<'d>],
) -> Result<(Vec<Modelled>, Vec<Device<'d>>), TwinError> {
    let used = solved(topology)?;
    // A stock name is `validate`'s refusal (`RESERVED_REFERENCES`); what the
    // emitter itself adds, the solved peripherals, is refused here.
    let mut taken: BTreeSet<String> = used.iter().map(|p| p.to_ascii_lowercase()).collect();
    let mut modelled = Vec::new();
    let mut unmodelled = Vec::new();
    for &(reference, d) in devices {
        let name = reference.to_ascii_lowercase();
        let ident = name.starts_with(|c: char| c.is_ascii_lowercase())
            && name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if !ident || !taken.insert(name.clone()) {
            return Err(TwinError::InvalidDeviceName {
                reference: reference.to_owned(),
            });
        }
        let Some(model) = model::kind(&d.part)? else {
            unmodelled.push((reference, d));
            continue;
        };
        if model.bus() != BusKind::I2c {
            return Err(TwinError::ModelOffItsBus {
                reference: reference.to_owned(),
                part: d.part.clone(),
            });
        }
        let model = model.renode();
        let bus = STOCK
            .iter()
            .find(|s| s.0 == d.peripheral && used.contains(&d.peripheral))
            .filter(|s| s.2 == "I2C.STM32F7_I2C")
            .ok_or_else(|| TwinError::DeviceOffBus {
                reference: reference.to_owned(),
                peripheral: d.peripheral.clone(),
            })?;
        modelled.push(Modelled {
            reference: reference.to_owned(),
            name,
            model,
            bus: bus.1,
            address: d.address,
        });
    }
    modelled.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((modelled, unmodelled))
}

/// The `.repl` text for `topology`, with its I2C devices declared where the
/// part has a model and listed as unmodelled where it has none.
pub fn emit(topology: &SolvedTopology) -> Result<String, TwinError> {
    emit_faulted(topology, &BTreeSet::new(), &BTreeSet::new())
}

/// Each bus in `faulted` with the peripheral the topology solved it on. A
/// faulted bus with no solved analog line is refused.
fn faulted_peripherals(
    topology: &SolvedTopology,
    faulted: &BTreeSet<String>,
) -> Result<Vec<(String, String)>, TwinError> {
    faulted
        .iter()
        .map(|bus| {
            topology
                .analog_lines
                .iter()
                .find(|(_, line)| &line.bus == bus)
                .and_then(|(net, _)| topology.assignments.get(net))
                .map(|a| (bus.clone(), a.peripheral.clone()))
                .ok_or_else(|| TwinError::UnknownFaultedBus { bus: bus.clone() })
        })
        .collect()
}

/// A device of the topology, by reference, as the topology orders them.
type Device<'t> = (&'t str, &'t SolvedI2cDevice);

/// The topology's devices that are not on a bus in `faulted`, in the
/// specification's order: what the platform declares, and so what Renode
/// lists, after the analog pass.
pub fn present<'t>(
    topology: &'t SolvedTopology,
    faulted: &BTreeSet<String>,
) -> Result<Vec<Device<'t>>, TwinError> {
    let off = faulted_peripherals(topology, faulted)?;
    Ok(topology
        .i2c_devices_in_order()
        .into_iter()
        .filter(|(_, d)| !off.iter().any(|(_, p)| *p == d.peripheral))
        .collect())
}

/// The `.repl` text for `topology` with the fault decision:
/// - each bus in `faulted` has its devices left out, modelled or not, which
///   is how a bus fault reaches the firmware (`RUL-17`, `AET-WO-2101` route
///   iii);
/// - each SPI device in `absent` is left out behind its chip select, which
///   stays wired, so a read of it meets the multiplexer's `not connected`
///   (`AET-WO-4200` D4).
///
/// The decision and the devices it leaves out are written into the
/// provenance comment, so a platform emitted without it differs from one
/// emitted with it.
pub fn emit_faulted(
    topology: &SolvedTopology,
    faulted: &BTreeSet<String>,
    absent: &BTreeSet<String>,
) -> Result<String, TwinError> {
    let instances = stock_instances(topology)?;
    let kept = present(topology, faulted)?;
    let (modelled, unmodelled) = split_devices(topology, &kept)?;
    let spi = spi_buses(topology, absent)?;
    let mut omitted = Vec::new();
    for d in spi.values().flatten().filter(|d| d.absent) {
        omitted.push(format!(
            "\n// - SPI fault (AET-WO-4200 D4): {} ({}) is absent behind its chip select {}.",
            d.reference, d.part, d.pin_name
        ));
    }
    for (bus, peripheral) in faulted_peripherals(topology, faulted)? {
        let gone: Vec<String> = topology
            .i2c_devices_in_order()
            .into_iter()
            .filter(|(_, d)| d.peripheral == peripheral)
            .map(|(reference, d)| format!("{reference} ({})", d.part))
            .collect();
        omitted.push(format!(
            "\n// - Analog fault (AET-WO-2200): bus `{bus}` ({peripheral}) is faulted by the analog pass,\n\
             //   so its devices are left out of this platform: {}.",
            if gone.is_empty() { "none".to_owned() } else { gone.join(", ") }
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
         // - Register facts: `{repo}` at `{commit}`, `{file}` sha256 `{sha}`.\n\
         // - Unmodelled I2C devices (no Renode {RENODE_VERSION} model): {unmodelled}.",
        unmodelled = if unmodelled.is_empty() {
            "none".to_owned()
        } else {
            unmodelled
                .iter()
                .map(|(reference, d)| {
                    format!(
                        "{reference} ({}) at {} 0x{:02X}",
                        d.part, d.peripheral, d.address
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        },
        spec = topology.specification_sha256,
        stock = RENODE_FILES[2].1,
        repo = cmsis::REPOSITORY,
        commit = cmsis::COMMIT,
        file = cmsis::HEADER_FILE,
        sha = cmsis::HEADER_SHA256,
    );
    let spi_unmodelled: Vec<String> = spi
        .iter()
        .flat_map(|(c, ds)| {
            ds.iter()
                .filter(|d| d.model.is_none())
                .map(move |d| format!("{} ({}) at {}mux@{}", d.reference, d.part, c, d.address))
        })
        .collect();
    if !spi.is_empty() {
        out.pop();
        let _ = writeln!(
            out,
            "\n// - Unmodelled SPI devices (no Renode {RENODE_VERSION} model): {}.",
            if spi_unmodelled.is_empty() {
                "none".to_owned()
            } else {
                spi_unmodelled.join(", ")
            }
        );
    }
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
    for d in &modelled {
        let _ = write!(
            out,
            "\n// {r}: Renode {RENODE_VERSION}'s `{m}`, at 0x{a:02X} on {bus}.\n\
             {name}: {m} @ {bus} 0x{a:02X}\n",
            r = d.reference,
            m = d.model,
            a = d.address,
            bus = d.bus,
            name = d.name,
        );
    }
    emit_spi(&mut out, &spi)?;
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

/// The path Renode gives a modelled device on `peripheral`, `i2c1.u2`: the
/// controller's stock name, then the emitted name (the reference, lower-cased,
/// as [`split_devices`] declares it). The witness lines carry it
/// (`AET-WO-2204` D1).
pub fn device_path(peripheral: &str, reference: &str) -> Result<String, TwinError> {
    let bus = STOCK
        .iter()
        .find(|s| s.0 == peripheral && s.2 == "I2C.STM32F7_I2C")
        .ok_or_else(|| TwinError::DeviceOffBus {
            reference: reference.to_owned(),
            peripheral: peripheral.to_owned(),
        })?;
    Ok(format!("{}.{}", bus.1, reference.to_ascii_lowercase()))
}

/// `AET-WO-2204` D5: the stock names a reference may not take are
/// `aethelgard_ir::RESERVED_REFERENCES`, which `validate` enforces
/// (`tests/reserved_names.rs` proves the list). What `split_devices` still
/// refuses is what only the emitter knows: the peripherals it adds from the
/// solution, and a repeat of a name.
#[cfg(test)]
mod tests {
    use crate::tests::with_devices;

    fn kind(devices: &[(&str, &str, u8)]) -> &'static str {
        super::emit(&with_devices(devices)).unwrap_err().kind()
    }

    #[test]
    fn a_name_the_emitter_adds_is_refused_here() {
        // `SPI1` is solved in the reference topology, so the emitter declares `spi1`.
        for name in ["SPI1", "spi1", "GPIOA", "gpioa"] {
            assert_eq!(
                kind(&[(name, "BME280", 0x76)]),
                "InvalidDeviceName",
                "{name}"
            );
        }
    }

    #[test]
    fn two_references_that_differ_only_in_case_are_refused_by_the_topology() {
        // `AET-WO-2204` measured `SolvedTopology::validate` not folding
        // references; since `AET-WO-4200` D7 it does, so the topology refuses
        // first. `split_devices` keeps its own check.
        assert_eq!(
            kind(&[("U2", "BME280", 0x76), ("u2", "BME280", 0x77)]),
            "InvalidTopology"
        );
    }

    #[test]
    fn a_reserved_name_is_the_topology_s_refusal_not_the_emitter_s() {
        for name in ["nvic", "NVIC", "i2c1", "True", "sysbus"] {
            assert_eq!(kind(&[(name, "BME280", 0x76)]), "InvalidTopology", "{name}");
        }
        // Only the lowercased form collides: `gpioPortA` loads.
        let text = super::emit(&with_devices(&[("gpioPortA", "BME280", 0x76)])).unwrap();
        assert!(text.contains("gpioporta: I2C.BME280"));
    }
}
