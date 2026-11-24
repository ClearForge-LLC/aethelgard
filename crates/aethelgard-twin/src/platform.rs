//! The Renode platform description of a solved topology (`AET-WO-2100` §1.1).
//!
//! The description includes Renode 1.17.0's `stm32h753.repl` by a root-relative
//! path (spike F1: a file-relative include breaks once `simulate` copies the
//! file) and adds only what the topology solved and Renode lacks: an SPI
//! instance other than `spi4`, at the header's base, with its interrupt wired
//! (spike F2). It never redeclares a stock instance, since a duplicate
//! registration breaks the load (`architecture.md` §2.9). The output is a
//! function of the topology, the pinned header and the cited DMAMUX1 table
//! ([`DMAMUX1_SPI_RX`], `AET-WO-4400c2`) alone (`INV-01`).

use std::collections::BTreeSet;
use std::fmt::Write as _;

use aethelgard_ir::parts::BusKind;
use aethelgard_ir::{MEMORY_SUFFIX, SolvedI2cDevice, SolvedTopology};
use aethelgard_svd_hal::cmsis::{self, RegisterFacts};

use aethelgard_ir::motor::TIMERS;

use crate::model::{self, Model};
use crate::{Kind, TwinError, kind, split_pin, timer};

/// The Renode release the stock table below was read from.
pub const RENODE_VERSION: &str = "1.17.0";

/// The Renode build the tools image carries (`AET-WO-1605`, ruled on #223):
/// 1.17.0 with upstream's NVIC fixes rebuilt in, as `renode --version`'s
/// `build:` line names it. `simulate` refuses a boot on any other build
/// (`AET-WO-4400g` §1.5). The image pins it in `container/smoke.sh`; the
/// tier-1 test `the_renode_build_is_the_images_pin` holds this constant equal to that line, so
/// the build is typed once here and checked against the image's pin.
pub const RENODE_BUILD: &str = "1.17.0+20260907gitf1dd1b4af.aet1605";

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
pub const STOCK: [(&str, &str, &str, u32); 26] = [
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
    // Each USART and UART of `aethelgard_ir::serial::PERIPHERALS`
    // (`AET-WO-4300b`): listed, never added. `lpuart1` is stock too, and no
    // port may use it.
    ("USART1", "usart1", "UART.STM32F7_USART", 0x4001_1000),
    ("USART2", "usart2", "UART.STM32F7_USART", 0x4000_4400),
    ("USART3", "usart3", "UART.STM32F7_USART", 0x4000_4800),
    ("UART4", "uart4", "UART.STM32F7_USART", 0x4000_4C00),
    ("UART5", "uart5", "UART.STM32F7_USART", 0x4000_5000),
    ("USART6", "usart6", "UART.STM32F7_USART", 0x4001_1400),
    ("UART7", "uart7", "UART.STM32F7_USART", 0x4000_7800),
    ("UART8", "uart8", "UART.STM32F7_USART", 0x4000_7C00),
    // ADC1 (with ADC2 as its unmodelled slave, L231-239) and ADC3 (L84-91),
    // a pin device's inputs' (`AET-WO-1211c`, `crate::pins::ADCS`): listed,
    // never added.
    ("ADC1", "adcM1S2", "Analog.STM32H7_ADC", 0x4002_2000),
    ("ADC3", "adc3", "Analog.STM32H7_ADC", 0x5802_6000),
];

/// The DMAMUX1 request input of each SPI controller's receive DMA, transcribed
/// with its citation from RM0433 Rev 9, Table 124 (`AET-WO-4400c2`, ruled on
/// PR #204, comment 6945751393: the numbers from the cited table as data).
/// Renode 1.17.0's `STM32H7_SPI` raises `DMARecieve` for each word received
/// with `RXDMAEN` set, and its `STM32_DMAMUX` serves the channel whose
/// `DMAREQ_ID` equals the line's number, so a firmware that receives SPI by
/// DMA (ArduPilot) reads nothing without the line (`AET-WO-4400c` stall 1).
pub const DMAMUX1_SPI_RX: &str = include_str!("../data/dmamux1-spi-rx.tsv");

/// The rows of [`DMAMUX1_SPI_RX`]: (ST's peripheral, the table's resource,
/// its DMA request MUX input). A row that is not three tab-separated fields
/// with a decimal input is refused by name.
pub fn dmamux1_spi_rx() -> Result<Vec<(&'static str, &'static str, u32)>, TwinError> {
    dma_rows(DMAMUX1_SPI_RX)
}

/// The rows of a table in [`DMAMUX1_SPI_RX`]'s format.
pub(crate) fn dma_rows(text: &str) -> Result<Vec<(&str, &str, u32)>, TwinError> {
    text.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let bad = || TwinError::MalformedDmaTable { row: l.to_owned() };
            let mut f = l.split('\t');
            let (Some(p), Some(r), Some(n), None) = (f.next(), f.next(), f.next(), f.next()) else {
                return Err(bad());
            };
            if !n.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad());
            }
            Ok((p, r, n.parse().map_err(|_| bad())?))
        })
        .collect()
}

/// Each USART's receive DMA request on DMAMUX1, from RM0433 Rev 9 Table 124
/// (`AET-WO-4400i` D2). Renode 1.17.0's `STM32F7_USART` raises
/// `ReceiveDmaRequest` while `CR3.DMAR` is set and its receive queue is not
/// empty (`STM32F7_USART.cs` L100-111), so a firmware that receives by DMA
/// (ArduPilot) reads no byte without the line (`spike/4400i` `noreq`).
pub const DMAMUX1_USART_RX: &str = include_str!("../data/dmamux1-usart-rx.tsv");

/// The DMAMUX1 input of `peripheral`'s (a USART's) receive DMA; any USART
/// with no row is refused (`INV-06`).
pub fn usart_rx_request(peripheral: &str) -> Result<u32, TwinError> {
    dma_rows(DMAMUX1_USART_RX)?
        .into_iter()
        .find(|(p, _, _)| *p == peripheral)
        .map(|(_, _, n)| n)
        .ok_or_else(|| TwinError::UnsupportedPeripheral {
            peripheral: peripheral.to_owned(),
        })
}

/// The DMAMUX1 input of `peripheral`'s receive DMA. `None` for SPI6, whose
/// request is on DMAMUX2 (RM0433 Table 128), which Renode 1.17.0's platform
/// does not declare; the emitted entry says so. Any other SPI with no row is
/// refused (`INV-06`).
pub fn spi_rx_request(peripheral: &str) -> Result<Option<u32>, TwinError> {
    if peripheral == "SPI6" {
        return Ok(None);
    }
    dmamux1_spi_rx()?
        .into_iter()
        .find(|(p, _, _)| *p == peripheral)
        .map(|(_, _, n)| Some(n))
        .ok_or_else(|| TwinError::UnsupportedPeripheral {
            peripheral: peripheral.to_owned(),
        })
}

/// How the header names a model of our own and its pin, before
/// `` `models/renode/<file>` sha256 `<64 hex>`. `` (`AET-WO-4220` D6).
pub const OWN_LINE: &str = "Model of our own (AET-WO-4220): ";

/// Who wrote a model, for its comment: Renode's, or this project's own with
/// its file.
fn whose(m: Model) -> String {
    match m.own() {
        Some((file, _)) => format!(
            "Aethelgard's own `{}` (`{}/{file}`)",
            m.renode(),
            model::OWN_DIR
        ),
        None => format!("Renode {RENODE_VERSION}'s `{}`", m.renode()),
    }
}

/// The models of our own a platform description may load: each its header
/// names, with its pin. Refused (`ModelUnpinned`, `AET-WO-4220` D6):
/// - a header line naming a file the twin does not pin, or a sha256 other
///   than its pin, or one model twice;
/// - a device line declaring a model of our own its header does not name,
///   however the type is spelled: Renode resolves `Sensors.X` and
///   `Antmicro.Renode.Peripherals.Sensors.X` alike, and needs no space after
///   the colon (adversarial F1, F7).
///
/// Only the header's leading comment lines are read as the header. A
/// platform this emitter wrote passes; the CLI also reads a caller's.
pub fn own_models(text: &str) -> Result<Vec<Model>, TwinError> {
    let unpinned = |why: String| Err(TwinError::ModelUnpinned { why });
    let mut named: Vec<Model> = Vec::new();
    for line in text.lines().take_while(|l| l.starts_with("//")) {
        let Some(rest) = line
            .strip_prefix("// - ")
            .and_then(|l| l.strip_prefix(OWN_LINE))
        else {
            continue;
        };
        let parsed = rest
            .strip_prefix('`')
            .and_then(|r| r.split_once("` sha256 `"))
            .and_then(|(path, r)| Some((path, r.strip_suffix("`.")?)));
        let Some((path, sha)) = parsed else {
            return unpinned(format!(
                "the header line `{line}` is not `{OWN_LINE}`<path>` sha256 `<hex>`.`"
            ));
        };
        let Some(&(m, _, pin)) = model::OWN
            .iter()
            .find(|o| path == format!("{}/{}", model::OWN_DIR, o.1))
        else {
            return unpinned(format!(
                "the header names `{path}`, which is no model of our own the twin pins"
            ));
        };
        if sha != pin {
            return unpinned(format!(
                "the header pins `{path}` at sha256 `{sha}`, but the twin pins it at `{pin}`"
            ));
        }
        if named.contains(&m) {
            return unpinned(format!("the header names `{path}` twice"));
        }
        named.push(m);
    }
    for line in text.lines() {
        let class = line
            .split_once(':')
            .and_then(|(_, r)| r.split_whitespace().next())
            .map(|t| t.rsplit('.').next().unwrap_or(t));
        let own = model::OWN
            .iter()
            .map(|o| o.0)
            .find(|m| class.is_some_and(|c| m.renode().rsplit('.').next() == Some(c)));
        if let Some(m) = own.filter(|m| !named.contains(m)) {
            return unpinned(format!(
                "a device is declared as `{}`, but the header names no pinned copy of it",
                m.renode()
            ));
        }
    }
    Ok(named)
}

/// The model an added SPI instance takes: `spi4`'s, from [`STOCK_FILE`].
const SPI_MODEL: &str = "SPI.STM32H7_SPI";

/// The model of every stock timer of [`STOCK_FILE`] (`AET-WO-4300a`).
const TIMER_MODEL: &str = "Timers.STM32_Timer";

/// The Renode name of each timer of `aethelgard_ir::motor::TIMERS`, in its
/// order: `timer1` for TIM1.
const TIMER_NAMES: [&str; 12] = [
    "timer1", "timer2", "timer3", "timer4", "timer5", "timer8", "timer12", "timer13", "timer14",
    "timer15", "timer16", "timer17",
];

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
            .pin
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

/// One SPI device as the emitted description wires it (`AET-WO-4200` D1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpiSelect {
    pub reference: String,
    /// ST's name of the controller, `SPI1`.
    pub peripheral: String,
    pub part: String,
    /// The multiplexer address, which its chip select drives.
    pub address: u8,
    /// The chip select's pin, `PA0`.
    pub select: String,
}

/// Each SPI device in the specification's order, as [`emit`] wires it: the
/// judge's checks name the same address and pin.
pub fn spi_wiring(topology: &SolvedTopology) -> Result<Vec<SpiSelect>, TwinError> {
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
        out.push(SpiSelect {
            reference: reference.to_owned(),
            peripheral: d.peripheral.clone(),
            part: d.part.clone(),
            address: w.address,
            select: w.pin_name.clone(),
        });
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
                        "\n// {r}: {whose}, at {c}mux@{k}, chip select {pin}.\n\
                         {n}: {name} @ {c}mux {k}\n",
                        whose = whose(m),
                        name = m.renode(),
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
        // Every caller validated the topology (`stock_instances`), which
        // refuses a listed part off its row's bus (`PartOnWrongBus`); every
        // modelled part has a row, on its model's bus (`tests/spi_platform.rs`,
        // `every_modelled_part_s_row_is_on_its_model_s_bus`). So an I2C
        // device's model is an I2C model here (`AET-WO-1206` §1.6: the
        // `ModelOffItsBus` check that stood here was unreachable, and went).
        let Some(model) = model::kind(&d.part)? else {
            unmodelled.push((reference, d));
            continue;
        };
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
    // Each model of our own the platform declares, with its pin (`AET-WO-4220`
    // D6): the CLI includes exactly these, from its own copies, before the
    // load. None for a board of Renode's models, whose platform is unchanged.
    let mut own: BTreeSet<(&str, &str)> = BTreeSet::new();
    let declared = modelled
        .iter()
        .filter_map(|d| Model::from_renode(d.model))
        .chain(
            spi.values()
                .flatten()
                .filter(|d| !d.absent)
                .filter_map(|d| d.model),
        );
    own.extend(declared.filter_map(Model::own));
    for (file, sha) in own {
        let _ = writeln!(
            out,
            "// - {OWN_LINE}`{}/{file}` sha256 `{sha}`.",
            model::OWN_DIR
        );
    }
    // The lifecycle banner (`AET-WO-4210` D1): none for a board of `active`
    // parts, whose platform is then unchanged.
    for line in topology
        .lifecycle_banner()
        .map_err(TwinError::InvalidTopology)?
    {
        let _ = writeln!(out, "// - {line}.");
    }
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
        let (source, dma) = match spi_rx_request(&i.peripheral)? {
            Some(n) => (
                "its receive DMA request RM0433 Table 124's;",
                format!("    DMARecieve -> dmamux1@{n}\n"),
            ),
            None => (
                "no receive DMA line (its request is on DMAMUX2, which Renode does not declare);",
                String::new(),
            ),
        };
        let _ = write!(
            out,
            "\n// {p}: {p}_BASE and {p}_IRQn from the header, {source} the model is `spi4`'s.\n\
             {name}: {SPI_MODEL} @ {{\n        sysbus 0x{base:08X};\n        rcc RegistrationPeripheralAlias.{p}\n    }}\n    IRQ -> nvic@{irq}\n{dma}",
            p = i.peripheral,
            name = i.name,
        );
    }
    for d in &modelled {
        let _ = write!(
            out,
            "\n// {r}: {whose}, at 0x{a:02X} on {bus}.\n\
             {name}: {m} @ {bus} 0x{a:02X}\n",
            r = d.reference,
            whose = Model::from_renode(d.model)
                .map_or_else(|| format!("Renode {RENODE_VERSION}'s `{}`", d.model), whose),
            m = d.model,
            a = d.address,
            bus = d.bus,
            name = d.name,
        );
    }
    emit_spi(&mut out, &spi)?;
    emit_observers(&mut out, topology)?;
    out.push_str(&crate::boot::brown_out(topology)?.presentation());
    emit_clock(&mut out, topology)?;
    if topology.foreign_clock.is_none() {
        out.push_str(&pwr_override()?);
    }
    Ok(out)
}

/// The PWR the generated firmware's bring-up meets (`AET-WO-4240v` §1.6,
/// D-3 (a)): an update entry that replaces the script of the stock `pwr`
/// stub ([`STOCK_FILE`] L728–745, a `Python.PythonPeripheral` that drops
/// every write and reads `PWR_CR3`'s supply bits 0), written from RM0433 Rev
/// 9, never from the generator. Emitted only on a generated platform: a
/// platform with `mcu.foreign_clock` keeps the stock stub byte for byte
/// (`AET-WO-4400g`'s ArduPilot witnesses). Measured in Renode 1.17.0
/// (`AET-WO-4240v` FEEDBACK): the entry replaces the stock script, and
/// `LogPeripheralAccess` logs the script's accesses as any model's.
///
/// What it holds, in the script's own globals:
/// - `PWR_CR3` reads [`crate::serial::PWR_CR3_POR`], revision V's power-on
///   value (§6.8.4, p. 307), until the first write; that write is stored
///   whole but for `USB33RDY` (bit 26, read-only and 0 at POR: no USB is
///   modelled, so the stock stub's 1 there is not kept), and every later write
///   is ignored, as the write-once lower byte is on silicon (p. 308);
/// - `PWR_D3CR` reads `VOS` as stored, `01` (Scale 3) from reset (p. 310:
///   reset `0x0000 4000`), with `VOSRDY` 1 ("Following reset VOSRDY will be
///   read 1"); a write stores `VOS` alone, every other bit reserved or
///   read-only;
/// - `PWR_CSR1` reads `ACTVOS` equal to the stored `VOS` and `ACTVOSRDY` 1
///   (§6.8.2), as the stock stub's `0x6000` does at reset;
/// - every other offset answers as the stock stub does (`CR1` `0xF000C100`,
///   any other a warning and 0).
///
/// The residue ([`crate::serial::PWR_RESIDUAL`]): `ACTVOSRDY` and `VOSRDY`
/// never read 0, so the twin never sees a wait time out, and an invalid
/// `LDOEN`/`BYPASS` pair is stored rather than locking the register with the
/// default kept (p. 308; [`crate::serial::judge_pwr`] names the write); no
/// Run* mode is modelled; no twin clock depends on `VOS`, so a firmware left
/// at VOS3 at 64 MHz runs the same, and only `aethelgard_ir::vos` refuses
/// it; and `PWR_CR3`'s lock across a warm reset is modelled only as far as
/// the stored first write: Renode re-runs the script's initialization at
/// `machine Reset` (measured), so the twin's warm reset is a power-on reset
/// for `PWR_CR3`.
pub fn pwr_override() -> Result<String, TwinError> {
    let f = cmsis::register_facts()?;
    let pwr = f.layout("PWR")?;
    let (cr1, csr1, cr3, d3cr) = (
        pwr.offset("CR1", 0)?,
        pwr.offset("CSR1", 0)?,
        pwr.offset("CR3", 0)?,
        pwr.offset("D3CR", 0)?,
    );
    let (vos_pos, vos_width) = f.field("PWR_D3CR_VOS")?;
    let vos_mask = ((1u32 << vos_width) - 1) << vos_pos;
    let vos_reset = 1u32 << vos_pos;
    let vosrdy = 1u32 << f.position("PWR_D3CR_VOSRDY_Pos")?;
    let actvosrdy = 1u32 << f.position("PWR_CSR1_ACTVOSRDY_Pos")?;
    // `ACTVOS` is read as the stored `VOS` in place: both fields are bits
    // 15:14 (header L14460, L14552), which a tier-1 test holds.
    let usb33rdy = 1u32 << f.position("PWR_CR3_USB33RDY_Pos")?;
    Ok(format!(
        "\n// The PWR (AET-WO-4240v D-3 (a)): the stock stub's script replaced, from RM0433 Rev 9\n\
         // §6.8: PWR_CR3 reads revision V's power-on 0x{por:08X} until its first write, stores that\n\
         // write (USB33RDY 0) and ignores later ones; PWR_D3CR stores VOS (Scale 3 from reset) with\n\
         // VOSRDY 1; PWR_CSR1 reads ACTVOS as the stored VOS with ACTVOSRDY 1; any other offset as stock.\n\
         pwr:\n\
         \x20   initable: true\n\
         \x20   script: '''\n\
         if request.IsInit:\n\
         \x20   cr3 = 0x{por:08X}\n\
         \x20   cr3_written = False\n\
         \x20   vos = 0x{vos_reset:08X}\n\
         elif request.Offset == 0x{cr1:02X}:\n\
         \x20   request.Value = 0xF000C100\n\
         elif request.Offset == 0x{csr1:02X}:\n\
         \x20   request.Value = vos | 0x{actvosrdy:08X}\n\
         elif request.Offset == 0x{cr3:02X}:\n\
         \x20   if request.IsWrite:\n\
         \x20       if not cr3_written:\n\
         \x20           cr3 = request.Value & 0x{keep:08X}\n\
         \x20           cr3_written = True\n\
         \x20   else:\n\
         \x20       request.Value = cr3\n\
         elif request.Offset == 0x{d3cr:02X}:\n\
         \x20   if request.IsWrite:\n\
         \x20       vos = request.Value & 0x{vos_mask:08X}\n\
         \x20   else:\n\
         \x20       request.Value = vos | 0x{vosrdy:08X}\n\
         else:\n\
         \x20   self.WarningLog(\"Unhandled access to offset 0x{{0:X}}\", request.Offset)\n\
         '''\n",
        por = crate::serial::PWR_CR3_POR,
        keep = !usb33rdy,
    ))
}

/// The kernel clock the twin builds timer `n` at (`AET-WO-4400g` §1.1–§1.2):
/// the declared one when the topology carries `foreign_clock`, else
/// [`SolvedClock::timer_kernel_hz`]; never the stock file's 250 MHz, which no
/// firmware chose. The
/// emitter writes it, and the observers predict at it, from this one place.
///
/// [`SolvedClock::timer_kernel_hz`]: aethelgard_ir::SolvedClock::timer_kernel_hz
pub fn timer_hz(topology: &SolvedTopology, n: u32) -> u64 {
    let apb = aethelgard_ir::motor::apb(n);
    match &topology.foreign_clock {
        Some(c) => c.timer_hz(apb),
        None => topology.clock.timer_kernel_hz(apb),
    }
}

/// Each instance the clock sets, as (the description's name, frequency in
/// Hz): every timer and USART [`STOCK_FILE`] carries, and under the solved
/// clock LPUART1, LPTIM1–5 and the DWT as well, each at its reset kernel
/// clock. A foreign firmware declares none of those, so under a declared
/// clock they keep the stock file's figures, which
/// [`unclocked`] names.
///
/// Two clocked models are set by neither: the NVIC's SysTick follows the
/// RCC (its constructor pushes HSI's 64 MHz before any firmware runs, and a
/// firmware's own RCC writes push its clock: `AET-WO-4400f` F4, measured),
/// and the independent watchdog counts LSI, outside the solved clock tree,
/// at the stock file's 56 kHz where silicon's LSI is 32 kHz (RM0433 Rev 9
/// §8.5.2); no judge reads it.
pub fn clocked(topology: &SolvedTopology) -> Vec<(String, u64)> {
    let mut out: Vec<(String, u64)> = timer::STOCK_TIMERS
        .iter()
        .map(|&(n, _)| (format!("timer{n}"), timer_hz(topology, n)))
        .collect();
    for p in aethelgard_ir::serial::PERIPHERALS {
        let hz = match &topology.foreign_clock {
            Some(c) => c.usarts[p],
            None => aethelgard_ir::serial::kernel_hz(&topology.clock, p)
                .expect("every listed peripheral has a reset kernel clock"),
        };
        out.push((p.to_ascii_lowercase(), hz));
    }
    if topology.foreign_clock.is_none() {
        let c = &topology.clock;
        out.push(("lpuart1".into(), c.pclk4_hz()));
        out.push(("lptimer1".into(), c.pclk1_hz()));
        for n in 2..=5 {
            out.push((format!("lptimer{n}"), c.pclk4_hz()));
        }
        out.push(("dwt".into(), c.core_hz()));
    }
    out
}

/// The instances of [`STOCK_FILE`] whose frequency the solved clock sets and
/// a declared one does not: none under the solved clock; the DWT, LPTIM1–5
/// and LPUART1 under a declared one. A foreign verdict names them.
pub fn unclocked(topology: &SolvedTopology) -> Vec<&'static str> {
    if topology.foreign_clock.is_none() {
        return Vec::new();
    }
    vec![
        "dwt", "lptimer1", "lptimer2", "lptimer3", "lptimer4", "lptimer5", "lpuart1",
    ]
}

/// The clock as update entries (`AET-WO-4400g`): each instance's
/// `frequency`, which Renode 1.17.0 builds a timer's counter and its
/// capture/compare channels at (`STM32_Timer.cs` L104), a USART's `BaudRate`
/// and receiver timeout from, and the DWT's cycle counter at. Set at
/// construction, so no RCC push undoes it at a timer's reset
/// (`LimitTimer.cs` L324, `AET-WO-4400f`). The USART frequency drives only
/// `BaudRate` and the receiver timeout in this build; upstream `88afd12bb`
/// paces received characters at `BaudRate`, so from the next Renode a wrong
/// frequency changes what the firmware receives.
///
/// On a foreign board, each declared serial port's USART also carries its
/// receive DMA request ([`usart_rx_request`], `AET-WO-4400i` D2: foreign
/// boards only, so no generated board's platform moves).
fn emit_clock(out: &mut String, topology: &SolvedTopology) -> Result<(), TwinError> {
    let whose = if topology.foreign_clock.is_some() {
        format!(
            "the specification's mcu.foreign_clock; {} keep the stock file's",
            unclocked(topology).join(", ")
        )
    } else if topology.clock.hse.is_some() {
        format!(
            "the solved clock's (PLL1 from the HSE at {} Hz, sys_ck {} Hz, every prescaler 1, AET-WO-4240b)",
            topology.clock.hse_hz, topology.clock.sysclk_hz
        )
    } else {
        "the solved clock's (the reset clock, HSI at 64 MHz, RM0433 Rev 9 §8.7.2)".to_owned()
    };
    let _ = write!(
        out,
        "\n// The clock (AET-WO-4400g): each timer, USART and the DWT at {whose}.\n"
    );
    for (name, hz) in clocked(topology) {
        let _ = write!(out, "{name}:\n    frequency: {hz}\n");
        if topology.foreign_clock.is_none() {
            continue;
        }
        if let Some(port) = topology
            .serial_ports
            .values()
            .find(|p| p.peripheral.eq_ignore_ascii_case(&name))
        {
            let n = usart_rx_request(&port.peripheral)?;
            let _ = write!(out, "    ReceiveDmaRequest -> dmamux1@{n}\n");
        }
    }
    emit_hse(out, topology);
    Ok(())
}

/// The stock RCC's HSE frequency on a board with an HSE (`AET-WO-4240b`
/// §1.6): an update entry after the stock include, `rcc:` then
/// `hseFrequency: <clock.hse_hz>`. Measured in Renode 1.17.0
/// (`spike/4240b/renode/results.md`): the entry is taken (`sys_ck` after `SW`
/// = PLL1 reads 64,000,000 Hz on both fixtures, 32,000,000 without it), and a
/// second full declaration of `rcc` is Renode's error E02. The model divides
/// the reference in integers, so it is exact only for a whole-Hz HSE over
/// `DIVM1`, which `aethelgard_ir::clock::judge_tree` requires. Nothing is
/// emitted for a board with no HSE, whose platform is unchanged, nor under
/// `mcu.foreign_clock` (`simulate --foreign` on an HSE board is `4240a` F4's,
/// not this work order's).
fn emit_hse(out: &mut String, topology: &SolvedTopology) {
    if topology.clock.hse.is_some() && topology.foreign_clock.is_none() {
        let _ = write!(
            out,
            "\n// The HSE (AET-WO-4240b): the stock rcc's hseFrequency (8 MHz by default) is the solved\n\
             // clock.hse_hz.\n\
             rcc:\n    hseFrequency: {}\n",
            topology.clock.hse_hz
        );
    }
}

/// Each motor output's observer (`AET-WO-4300a` §1.4): a
/// `Miscellaneous.LED` on its pin, the cheapest pin observer the `4300`
/// spike named, logging each change at `Noisy`. The pin's stock input route
/// is restated with the observer added, one block per port, so the stock
/// wiring is kept whole (a later route of a pin replaces an earlier one).
fn emit_observers(out: &mut String, topology: &SolvedTopology) -> Result<(), TwinError> {
    let observers = timer::observers(topology)?;
    let mut ports: std::collections::BTreeMap<char, Vec<&timer::Observer>> =
        std::collections::BTreeMap::new();
    for o in &observers {
        let _ = write!(
            out,
            "\n// {label}: {timer}_CH{ch} on {pin} (AF{af}), observed by Renode {RENODE_VERSION}'s\n\
             // `Miscellaneous.LED` (AET-WO-4300a).\n\
             {name}: Miscellaneous.LED @ gpioPort{port} {index}\n",
            label = o.label,
            timer = format_args!("TIM{}", o.timer),
            ch = o.channel,
            pin = o.pin,
            af = o.alternate_function,
            name = o.name,
            port = o.port,
            index = o.index,
        );
        ports.entry(o.port).or_default().push(o);
    }
    for (port, list) in ports {
        let _ = write!(out, "\ngpioPort{port}:\n");
        for o in list {
            // `observers` refused a pin with no stock route, and every pin
            // of `TIMER_ROUTES` has an input route (`tests/timer_routes.rs`).
            let route = timer::stock_input(o.port, o.index).expect("a routed pin");
            let _ = writeln!(out, "    {} -> {route} | {}@0", o.index, o.name);
        }
    }
    Ok(())
}

/// The solved peripherals and the ports of every solved pin, in name order.
fn solved(topology: &SolvedTopology) -> Result<BTreeSet<String>, TwinError> {
    let mut set = BTreeSet::new();
    for a in topology.assignments.values() {
        set.insert(a.peripheral.clone());
        set.insert(format!("GPIO{}", split_pin(&a.pin)?.0));
    }
    for g in topology.gpio.values() {
        set.insert(format!("GPIO{}", split_pin(&g.pin)?.0));
    }
    // A pin device's ADC inputs (`AET-WO-1211c`): a solved resolution marks
    // one (`AET-WO-1211b` S1); a `power.sense` input carries none, is
    // generated by no firmware, and stays out of the listing as before.
    for a in topology.adc_inputs.values().filter(|a| a.bits.is_some()) {
        set.insert(a.peripheral.clone());
        set.insert(format!("GPIO{}", split_pin(&a.pin)?.0));
    }
    Ok(set)
}

fn instance(peripheral: &str, facts: &RegisterFacts) -> Result<Instance, TwinError> {
    let kind = kind(peripheral)?;
    let header = facts.base(peripheral)?;
    if kind == Kind::Timer {
        // `kind` admitted only a number of `motor::TIMERS`, each stock.
        let n: u32 = peripheral[3..].parse().expect("a timer's number");
        let name: &'static str =
            TIMER_NAMES[TIMERS.iter().position(|&t| t == n).expect("a stock timer")];
        let (_, renode) = *timer::STOCK_TIMERS
            .iter()
            .find(|(t, _)| *t == n)
            .expect("every timer with channels is stock");
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
            model: short(TIMER_MODEL).to_owned(),
            added: false,
        });
    }
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
        // `GPIOA` is the emitter's own port block. `SPI1`, which it also
        // declares, is a reserved reference since `AET-WO-4300c` F21, so the
        // topology refuses it first (below).
        for name in ["GPIOA", "gpioa"] {
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
        for name in ["nvic", "NVIC", "i2c1", "True", "sysbus", "SPI1", "spi1"] {
            assert_eq!(kind(&[(name, "BME280", 0x76)]), "InvalidTopology", "{name}");
        }
        // Only the lowercased form collides: `gpioPortA` loads.
        let text = super::emit(&with_devices(&[("gpioPortA", "BME280", 0x76)])).unwrap();
        assert!(text.contains("gpioporta: I2C.BME280"));
    }
}
