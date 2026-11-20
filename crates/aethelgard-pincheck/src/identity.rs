//! The identity judge (`AET-WO-4400b2`): the facts ArduPilot keeps in its
//! device tables rather than in `gpio_default_config`, each read from
//! ArduPilot's own computation and held against the solved topology:
//! - each SPI device's chip select, bus, mode and speeds (`HAL_SPI_DEVICE_LIST`);
//! - each motor's timer channel and `PWM(n)` index (`HAL_PWM_GROUPS`);
//! - the I2C clock (`HAL_I2C_MAX_CLOCK`, on ArduPilot's own timings);
//! - the serial order and pins (the ELF's `_serial_tab`), and each port's
//!   protocol, options and baud (the ELF's `AP_SerialManager::var_info`,
//!   `AET-WO-4405`), with the GPS ports ArduPilot can bind;
//! - the barometer probes (`HAL_BARO_PROBE_LIST`);
//! - the IMU probes (`HAL_INS_PROBE_LIST`, `AET-WO-4400d`): each driver, its
//!   SPI device, and its rotation, judged as a matrix against the declared
//!   orientation; and the driver's probe linked into the ELF.
//!
//! Each table is read through the list macro the compiler expands
//! (`SPIDevice.cpp:75`, `RCOutput.cpp:75`, `AP_Baro.cpp:666`,
//! `AP_InertialSensor.cpp:1220-1222`), never through
//! numbered macros the list may not name (adversarial A4). The expected values
//! come from the topology and `aethelgard-ir` alone; nothing here reads the
//! generated `hwdef.dat`, and the crate shares no code with its generator
//! (`INV-07`, `RUL-20`). FEEDBACK §0 gives each input and why.

use std::collections::BTreeMap;

use aethelgard_ir::orientation::{Orientation, Toward};
use aethelgard_ir::parts::{ArduPilotKind, PartsTable};
use aethelgard_ir::serial::SerialRole;
use aethelgard_ir::{PinAssignment, SolvedSerialPort, SolvedTopology};

use crate::elf::{self, SerialEntry, SerialParam, map_baudrate};
use crate::header::Preprocessed;
use crate::{Finding, Refusal};

/// RM0433 Rev 9 §2.3.2 (Table 8): GPIOA at `0x5802_0000`, each port
/// `0x400` above the last, A to K.
const GPIO_BASE: u32 = 0x5802_0000;
const GPIO_STEP: u32 = 0x400;
const PORTS: &str = "ABCDEFGHIJK";

/// The I2C clock when `HAL_I2C_MAX_CLOCK` is not defined (`I2CDevice.cpp:53-54`).
const I2C_DEFAULT_CLOCK: u64 = 100_000;

/// The H7 timings `I2CDevice.cpp:65-69` defines unless `hwdef.h` does. A
/// definition changes the clock a bus runs at whatever `HAL_I2C_MAX_CLOCK`
/// says (adversarial A5), so any is a finding.
const I2C_TIMINGS: [&str; 2] = ["HAL_I2C_H7_100_TIMINGR", "HAL_I2C_H7_400_TIMINGR"];

/// How deep a list may name a macro that names a macro.
const LIST_DEPTH: u8 = 4;

fn malformed<T>(name: &str, message: impl Into<String>) -> Result<T, Refusal> {
    Err(Refusal::Header {
        name: name.to_owned(),
        message: message.into(),
    })
}

/// The pieces of `s` split at `sep` outside `()`, `{}` and quotes, each
/// trimmed; empty pieces are dropped.
fn split(s: &str, sep: char) -> Vec<String> {
    let (mut out, mut cur, mut depth, mut quoted) = (Vec::new(), String::new(), 0i32, false);
    for c in s.chars() {
        match c {
            '"' => quoted = !quoted,
            '(' | '{' if !quoted => depth += 1,
            ')' | '}' if !quoted => depth -= 1,
            c if c == sep && !quoted && depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    out.push(cur);
    out.into_iter()
        .map(|p| p.trim().to_owned())
        .filter(|p| !p.is_empty())
        .collect()
}

/// The top-level comma-separated arguments of `s`.
fn args(s: &str) -> Vec<String> {
    split(s, ',')
}

/// The inside of `open … close` wrapping all of `s`, or `None`.
fn inside<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    s.trim().strip_prefix(open)?.strip_suffix(close)
}

/// A C integer constant as GCC reads it (adversarial A3): `0x` hex, a
/// leading `0` octal, else decimal, with `U` and `L` suffixes. `None` for
/// anything else, or a value past `u64`. Every character must be a digit of
/// the radix: Rust's parsers take a leading `+`, which in C is an operator
/// (`0+303240` is 303240, not octal 100000).
fn number(s: &str) -> Option<u64> {
    let s = s.trim().trim_end_matches(['U', 'u', 'L', 'l']);
    let (digits, radix) = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        (hex, 16)
    } else if let Some(octal) = s.strip_prefix('0').filter(|o| !o.is_empty()) {
        (octal, 8)
    } else {
        (s, 10)
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    u64::from_str_radix(digits, radix).ok()
}

fn is_name(s: &str) -> bool {
    s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && s.bytes().next().is_some_and(|b| !b.is_ascii_digit())
}

/// The elements the compiler expands `list` to: its value split at `sep`,
/// each element that names a defined macro replaced by that macro's own
/// elements, as the preprocessor would. Each comes with the macro whose value
/// held it. An undefined list is empty; an undefined name is an element
/// (`RETURN_IF_NO_SPACE`), which a caller's form check refuses unless it is
/// one it knows.
fn expand(h: &Preprocessed, list: &str, sep: char) -> Result<Vec<(String, String)>, Refusal> {
    fn walk(
        h: &Preprocessed,
        name: &str,
        value: &str,
        sep: char,
        depth: u8,
        out: &mut Vec<(String, String)>,
    ) -> Result<(), Refusal> {
        for e in split(value, sep) {
            match (is_name(&e) && depth < LIST_DEPTH)
                .then(|| h.value(&e))
                .transpose()?
                .flatten()
            {
                Some(inner) => walk(h, &e, &inner, sep, depth + 1, out)?,
                None => out.push((name.to_owned(), e)),
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    if let Some(value) = h.value(list)? {
        walk(h, list, &value, sep, 0, &mut out)?;
    }
    Ok(out)
}

/// `PAL_LINE(GPIOx,nU)` as `Pxn`.
fn pal_line(s: &str) -> Option<String> {
    let a = args(inside(s, "PAL_LINE(", ")")?);
    let [port, pin] = a.as_slice() else {
        return None;
    };
    let port = port.strip_prefix("GPIO")?;
    let pin = number(pin)?;
    (port.len() == 1 && PORTS.contains(port) && pin < 16).then(|| format!("P{port}{pin}"))
}

/// An ELF `ioline_t`, a port's base address plus the pin, as `Pxn`.
fn line_pin(v: u32) -> Option<String> {
    let base = v & !(GPIO_STEP - 1);
    let pin = v & (GPIO_STEP - 1);
    let port = base.checked_sub(GPIO_BASE)? / GPIO_STEP;
    let letter = PORTS.chars().nth(usize::try_from(port).ok()?)?;
    (base == GPIO_BASE + port * GPIO_STEP && pin < 16).then(|| format!("P{letter}{pin}"))
}

/// A peripheral's number: `USART1` 1, `UART4` 4, `SPI2` 2.
fn unit(peripheral: &str) -> Option<u8> {
    peripheral
        .trim_start_matches(|c: char| c.is_ascii_alphabetic())
        .parse()
        .ok()
}

/// Collects findings, each named `identity`.
struct Out(Vec<Finding>);

impl Out {
    fn push(
        &mut self,
        kind: &'static str,
        at: &str,
        net: Option<&str>,
        expected: String,
        decoded: String,
    ) {
        self.0.push(Finding {
            check: "identity",
            kind,
            pin: at.to_owned(),
            net: net.map(str::to_owned),
            expected,
            decoded,
        });
    }
}

/// A numeric define, or `None` when it is not defined; refused when it is
/// not a C integer constant.
fn defined_number(h: &Preprocessed, name: &str) -> Result<Option<u64>, Refusal> {
    match h.value(name)? {
        None => Ok(None),
        Some(v) => match number(&v) {
            Some(n) => Ok(Some(n)),
            None => malformed(name, format!("is not a C integer constant: `{v}`")),
        },
    }
}

/// The ordered list `name`, `A,B,…`, each `{ &<driver>n, n, … }`: the
/// number of each entry in order.
fn bus_list(h: &Preprocessed, name: &str, driver: &str) -> Result<Vec<u8>, Refusal> {
    let Some(list) = h.value(name)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for entry in args(&list) {
        // Each candidate definition (one per DMA branch, which the reader
        // does not evaluate) must name the same driver; that is all read.
        let numbers: Vec<Option<u8>> = h
            .candidates(&entry)
            .iter()
            .map(|body| {
                args(inside(body, "{", "}").unwrap_or(""))
                    .first()
                    .and_then(|d| d.strip_prefix(&format!("&{driver}")))
                    .and_then(|n| n.parse::<u8>().ok())
            })
            .collect();
        match numbers.as_slice() {
            [] => return malformed(name, format!("names `{entry}`, which is not defined")),
            [Some(n), rest @ ..] if rest.iter().all(|m| *m == Some(*n)) => out.push(*n),
            _ => {
                return malformed(
                    &entry,
                    format!("is not `{{ &{driver}n, … }}` alike in every branch"),
                );
            }
        }
    }
    Ok(out)
}

/// Judges ArduPilot's device tables (`header`, and the ELF's serial table
/// and symbols) against `t`, with `parts` naming each part's ArduPilot
/// driver and SPI name. Every mismatch is a finding; a table in a form this
/// reader does not know is refused by name.
pub fn judge(
    t: &SolvedTopology,
    parts: &PartsTable,
    header: &Preprocessed,
    elf_bytes: &[u8],
) -> Result<Vec<Finding>, Refusal> {
    let serial = elf::serial_table(elf_bytes)?;
    let params = elf::serial_params(elf_bytes)?;
    let receivers = elf::gps_receivers(elf_bytes)?;
    let mut out = Out(Vec::new());
    spi(t, parts, header, &mut out)?;
    motors(t, header, &mut out)?;
    i2c_clock(t, header, &mut out)?;
    serial_ports(t, &serial, &params, &mut out);
    serial_settings(t, &params, receivers, &mut out);
    baros(t, parts, header, &mut out)?;
    imus(t, parts, header, elf_bytes, &mut out)?;
    Ok(out.0)
}

/// One `SPIDesc` as read: its macro, bus, chip select, mode, low and high speed.
type SpiDesc = (String, String, String, u8, u64, u64);

/// Each SPI device the parts table names for ArduPilot: one `SPIDesc` of
/// that name, on its solved controller, its chip select on its solved pin,
/// in the driven mode, at speeds within the kHz at or below the controller's
/// achieved rate (never above it: the pin's speed was judged at that rate).
fn spi(
    t: &SolvedTopology,
    parts: &PartsTable,
    h: &Preprocessed,
    out: &mut Out,
) -> Result<(), Refusal> {
    let mhz = h.value("MHZ")?;
    let khz = h.value("KHZ")?;
    let buses = bus_list(h, "HAL_SPI_BUS_LIST", "SPID")?;
    let speed = |s: &str| -> Option<u64> {
        let (n, unit) = s.split_once('*').unwrap_or((s, ""));
        let n = number(n)?;
        match unit.trim() {
            "" => Some(n),
            "MHZ" if mhz.as_deref() == Some("(1000U*1000U)") => n.checked_mul(1_000_000),
            "KHZ" if khz.as_deref() == Some("(1000U)") => n.checked_mul(1000),
            _ => None,
        }
    };
    let mut table: BTreeMap<String, Vec<SpiDesc>> = BTreeMap::new();
    for (name, v) in expand(h, "HAL_SPI_DEVICE_LIST", ',')? {
        let a = args(inside(&v, "SPIDesc(", ")").unwrap_or(""));
        let parsed = (|| {
            let [dev, bus, _devid, cs, mode, low, high] = a.as_slice() else {
                return None;
            };
            let dev = inside(dev, "\"", "\"")?.to_owned();
            let bus = buses.get(usize::try_from(number(bus)?).ok()?)?;
            let mode = mode.strip_prefix("SPIDEV_MODE")?.parse::<u8>().ok()?;
            Some((
                dev,
                format!("SPI{bus}"),
                pal_line(cs)?,
                mode,
                speed(low)?,
                speed(high)?,
            ))
        })();
        let Some((dev, bus, cs, mode, low, high)) = parsed else {
            return malformed(
                &name,
                format!(
                    "is not SPIDesc(\"name\", bus, devid, PAL_LINE(..), SPIDEV_MODEn, speed, speed) on a listed bus: `{v}`"
                ),
            );
        };
        table
            .entry(dev)
            .or_default()
            .push((name, bus, cs, mode, low, high));
    }
    let mut claimed = Vec::new();
    for (reference, d) in t.spi_devices_in_order() {
        let Some(row) = parts.get(&d.part).and_then(|p| p.ardupilot.as_ref()) else {
            continue;
        };
        let Some(dev) = row.spidev.as_deref() else {
            continue;
        };
        claimed.push(dev.to_owned());
        let at = format!("SPIDEV {dev}");
        let Some(entries) = table.get(dev) else {
            out.push(
                "MissingSpiDevice",
                &at,
                Some(reference),
                format!("one SPIDesc \"{dev}\" ({})", d.part),
                "none".into(),
            );
            continue;
        };
        if entries.len() > 1 {
            out.push(
                "SpiDeviceTwice",
                &at,
                Some(reference),
                "one SPIDesc".into(),
                format!("{} SPIDescs", entries.len()),
            );
        }
        let (_, bus, cs, mode, low, high) = &entries[0];
        if *bus != d.peripheral {
            out.push(
                "WrongSpiBus",
                &at,
                Some(reference),
                d.peripheral.clone(),
                bus.clone(),
            );
        }
        let want_cs = &t.gpio[&d.cs];
        if cs != want_cs {
            out.push(
                "WrongChipSelect",
                &at,
                Some(reference),
                format!("{want_cs} ({})", d.cs),
                cs.clone(),
            );
        }
        let driven = aethelgard_ir::parts::DRIVEN_SPI_MODE;
        if *mode != driven {
            out.push(
                "WrongSpiMode",
                &at,
                Some(reference),
                format!("MODE{driven}"),
                format!("MODE{mode}"),
            );
        }
        if let Some(c) = t.spi_controllers.get(&d.peripheral) {
            let top = c.achieved_hz;
            for (which, hz) in [("low", low), ("high", high)] {
                if *hz > top || top - *hz >= 1000 {
                    out.push(
                        "WrongSpiSpeed",
                        &format!("{at} {which}"),
                        Some(reference),
                        format!(
                            "within 1 kHz at or below {top} Hz ({}'s achieved_hz)",
                            d.peripheral
                        ),
                        format!("{hz} Hz"),
                    );
                }
            }
        }
    }
    for (dev, entries) in &table {
        if !claimed.contains(dev) {
            let (name, bus, cs, ..) = &entries[0];
            out.push(
                "UnknownSpiDevice",
                name,
                None,
                "a device of the topology".into(),
                format!("\"{dev}\" on {bus}, CS {cs}"),
            );
        }
    }
    Ok(())
}

/// Each motor output: the PWM group of its timer gives its channel its
/// `PWM(n)` index (zero-based), and every other channel of every group is
/// disabled (255).
fn motors(t: &SolvedTopology, h: &Preprocessed, out: &mut Out) -> Result<(), Refusal> {
    let mut groups: BTreeMap<u8, (String, [u8; 4])> = BTreeMap::new();
    for (name, v) in expand(h, "HAL_PWM_GROUPS", ',')? {
        let parsed = (|| {
            let a = args(inside(&v, "{", "}")?);
            let chan = args(inside(a.get(1)?, "{", "}")?);
            let chan: Vec<u8> = chan
                .iter()
                .map(|c| number(c).and_then(|n| u8::try_from(n).ok()))
                .collect::<Option<_>>()?;
            let timer = a
                .iter()
                .find_map(|x| x.strip_prefix("&PWMD")?.parse::<u8>().ok())?;
            Some((timer, <[u8; 4]>::try_from(chan).ok()?))
        })();
        let Some((timer, chan)) = parsed else {
            return malformed(
                &name,
                format!("is not {{ adv, {{c1, c2, c3, c4}}, …, &PWMDn, … }}: `{v}`"),
            );
        };
        if groups.insert(timer, (name.clone(), chan)).is_some() {
            return malformed(&name, format!("is a second group for TIM{timer}"));
        }
    }
    let mut want: BTreeMap<(u8, usize), (&str, u32)> = BTreeMap::new();
    for (net, m) in &t.motor_outputs {
        let a = &t.assignments[net];
        let slot = (|| {
            let (timer, ch) = a.signal.split_once("_CH")?;
            Some((
                unit(timer)?,
                ch.parse::<usize>()
                    .ok()?
                    .checked_sub(1)
                    .filter(|c| *c < 4)?,
            ))
        })();
        let Some(slot) = slot else {
            out.push(
                "WrongMotorChannel",
                &a.signal,
                Some(net),
                "a TIMn_CHk channel".into(),
                a.signal.clone(),
            );
            continue;
        };
        want.insert(slot, (net.as_str(), m.index));
    }
    for (&(timer, ch), &(net, index)) in &want {
        let at = format!("TIM{timer}_CH{}", ch + 1);
        match groups.get(&timer) {
            None => out.push(
                "MissingPwmGroup",
                &at,
                Some(net),
                format!("PWM({})", index + 1),
                "no group for the timer".into(),
            ),
            Some((_, chan)) if u32::from(chan[ch]) != index => out.push(
                "WrongMotorChannel",
                &at,
                Some(net),
                format!("PWM({})", index + 1),
                if chan[ch] == 255 {
                    "disabled".into()
                } else {
                    format!("PWM({})", u32::from(chan[ch]) + 1)
                },
            ),
            Some(_) => {}
        }
    }
    for (timer, (name, chan)) in &groups {
        for (ch, c) in chan.iter().enumerate() {
            if *c != 255 && !want.contains_key(&(*timer, ch)) {
                out.push(
                    "UnexpectedPwmOutput",
                    &format!("TIM{timer}_CH{}", ch + 1),
                    None,
                    format!("disabled ({name})"),
                    format!("PWM({})", u32::from(*c) + 1),
                );
            }
        }
    }
    Ok(())
}

/// ArduPilot's one I2C clock, never above the slowest declared bus, and on
/// ArduPilot's own H7 timings.
fn i2c_clock(t: &SolvedTopology, h: &Preprocessed, out: &mut Out) -> Result<(), Refusal> {
    let Some(slowest) = t.i2c_speeds.values().min() else {
        return Ok(());
    };
    let clock = defined_number(h, "HAL_I2C_MAX_CLOCK")?.unwrap_or(I2C_DEFAULT_CLOCK);
    if clock > u64::from(*slowest) {
        out.push(
            "I2cClockAboveDeclared",
            "HAL_I2C_MAX_CLOCK",
            None,
            format!("at most {slowest} Hz (the slowest declared bus)"),
            format!("{clock} Hz"),
        );
    }
    for name in I2C_TIMINGS {
        if let Some(v) = h.value(name)? {
            out.push(
                "I2cTimingOverridden",
                name,
                None,
                "ArduPilot's own (I2CDevice.cpp:65-69)".into(),
                v,
            );
        }
    }
    Ok(())
}

/// The topology's ports in ArduPilot's order: the console first, as SERIAL0,
/// then the others in the specification's order.
fn serial_order(
    t: &SolvedTopology,
) -> Vec<(&str, &SolvedSerialPort, &PinAssignment, &PinAssignment)> {
    let (mut order, rest): (Vec<_>, Vec<_>) = t
        .serial_ports_in_order()
        .into_iter()
        .partition(|(_, p, ..)| p.role == SerialRole::Console);
    order.extend(rest);
    order
}

/// The serial order, from the ELF: SERIAL0 the console, then the other ports
/// in the specification's order, each on its solved peripheral (its driver
/// `SDn` and its number) and pins, every later entry empty; and each port's
/// baud, from its `n_BAUD` in var_info, except RC input's, which ArduPilot
/// sets itself (`AET-WO-4400b` F3).
fn serial_ports(t: &SolvedTopology, serial: &[SerialEntry], params: &[SerialParam], out: &mut Out) {
    let order = serial_order(t);
    for n in 0..order.len().max(serial.len()) {
        let at = format!("SERIAL{n}");
        let got = serial.get(n).filter(|e| !e.empty);
        let got_text = got.map_or_else(
            || "none".to_owned(),
            |e| {
                format!(
                    "{}{} TX {} RX {}{}",
                    if e.is_usb { "USB " } else { "U(S)ART" },
                    e.instance,
                    line_pin(e.tx_line).unwrap_or_else(|| format!("{:#010x}", e.tx_line)),
                    line_pin(e.rx_line).unwrap_or_else(|| format!("{:#010x}", e.rx_line)),
                    if e.driver_is_instance {
                        ""
                    } else {
                        ", driver not SD<instance>"
                    }
                )
            },
        );
        let Some((label, p, tx, rx)) = order.get(n) else {
            if got.is_some() {
                out.push("UnexpectedSerialPort", &at, None, "none".into(), got_text);
            }
            continue;
        };
        let want = format!(
            "U(S)ART{} TX {} RX {}",
            unit(&p.peripheral).unwrap_or(0),
            tx.pin,
            rx.pin
        );
        match got {
            None => out.push("MissingSerialPort", &at, Some(label), want, got_text),
            Some(e) => {
                let ok = !e.is_usb
                    && e.driver_is_instance
                    && Some(e.instance) == unit(&p.peripheral)
                    && line_pin(e.tx_line).as_deref() == Some(tx.pin.as_str())
                    && line_pin(e.rx_line).as_deref() == Some(rx.pin.as_str());
                if !ok {
                    out.push("WrongSerialPort", &at, Some(label), want, got_text);
                }
            }
        }
        if matches!(p.role, SerialRole::RcCrsf | SerialRole::RcSbus) {
            continue;
        }
        // A port var_info does not compile is named by `serial_settings`.
        let Some(param) = params.iter().find(|q| usize::from(q.port) == n) else {
            continue;
        };
        // Each port runs at its parameter, mapped (`AP_SerialManager.h:151-153`;
        // SERIAL0 reopened by the GCS, `GCS_Common.cpp:221`). SERIAL0's
        // first open, at `DEFAULT_SERIAL0_BAUD` itself (`:419`), is not
        // judged: var_info holds it only as the define / 1000 (`:172`), and
        // it is no longer read from `hwdef.h` (FEEDBACK D1).
        let declared = u64::from(p.framing.baud);
        let runs = map_baudrate(param.baud);
        if runs != declared {
            out.push(
                "WrongSerialBaud",
                &at,
                Some(label),
                format!("{declared} baud"),
                format!("{runs} baud"),
            );
        }
    }
}

/// The `SerialProtocol` names a finding can carry (`AP_SerialManager.h:37-91`):
/// the protocols the judge expects or counts, and None, each port's default
/// on SERIAL5–9. Any other value is shown as its number.
const SERIAL_PROTOCOLS: [(&str, i64); 6] = [
    ("SerialProtocol_None", -1),
    ("SerialProtocol_MAVLink", 1),
    ("SerialProtocol_MAVLink2", MAVLINK2),
    ("SerialProtocol_GPS", GPS),
    ("SerialProtocol_GPS2", GPS2),
    ("SerialProtocol_RCIN", RCIN),
];

const MAVLINK2: i64 = 2;
const GPS: i64 = 5;
/// `SerialProtocol_GPS2`, which ArduPilot finds as a GPS port
/// (`protocol_match`, `AP_SerialManager.cpp:786-790`).
const GPS2: i64 = 6;
const RCIN: i64 = 23;

/// `SERIALn_OPTIONS` bits (`AP_HAL/UARTDriver.h:90-102`).
const RXINV: i64 = 1 << 0;
const TXINV: i64 = 1 << 1;

fn protocol_name(v: i64) -> String {
    SERIAL_PROTOCOLS
        .iter()
        .find(|(_, p)| *p == v)
        .map_or_else(|| v.to_string(), |(n, _)| format!("{n} ({v})"))
}

/// Each port's protocol and options as `var_info` compiles them, against the
/// topology's role and declared inversion (the defaults, the `#ifndef`s, the
/// SERIAL7/8 literal options and `HAL_HAVE_SERIALn_PARAMS` are all folded
/// in by the compiler):
/// - **A port var_info does not compile** has no parameters at all
///   (`MissingSerialParameters`): its `HAL_HAVE_SERIALn_PARAMS` is not 1,
///   or it is past SERIAL9, where the table stops (`:379-397`).
/// - **Protocol:** `n_PROTOCOL`, expected by role, exactly: MAVLink 1 and 2
///   are different wires, so `protocol_match`'s equivalences (`:773-793`)
///   do not apply. On SERIAL0 it is a literal MAVLink 2 (`:180`), so the
///   port there must be the console or a MAVLink port (`AET-WO-4404` D4).
/// - **Options:** `n_OPTIONS`. A port that is not RC input sets RXINV and
///   TXINV exactly when it declares `inverted` (both lines,
///   `ir/src/serial.rs:82`), and no other bit. On RC input, RXINV is
///   AP_RCProtocol's (`AP_RCProtocol.cpp:326-336`) and is not judged; every
///   other bit must be clear. SERIAL0 has no options row (`:165-181`), so a
///   port there declared inverted runs not inverted.
/// - **One RC input at most:** a second compiled RCIN port is dropped
///   (`AP_SerialManager.cpp:536-541`), even where the topology asks for it.
/// - **One GPS port per receiver at most:** `AP_GPS::init` gives the first
///   GPS port to receiver 1, the next to receiver 2, and so on
///   (`AP_GPS.cpp:345-349`), so a compiled GPS (or GPS2) port past
///   `receivers`, the count `AP_GPS::var_info` compiles
///   ([`elf::gps_receivers`], `GPS_MAX_RECEIVERS`), can never be bound
///   (`GpsPortUnbound`). At compiled defaults only receiver 1 takes one:
///   `GPS2_TYPE` is 0 (`AP_GPS_Params.cpp:31`, `AP_GPS.cpp:328`),
///   `AET-WO-4405` D2.
/// - **No AP_GPS at all** (`receivers` `None`): each port the topology
///   declares a GPS has no consumer (`GpsNotCompiled`), and the count is
///   not made.
///
/// The last two counts take every compiled port, declared or not, in n
/// order, as ArduPilot's own search does (`find_protocol_instance`,
/// `:600-612`).
fn serial_settings(
    t: &SolvedTopology,
    params: &[SerialParam],
    receivers: Option<u8>,
    out: &mut Out,
) {
    let order = serial_order(t);
    let net_of = |n: usize| order.get(n).map(|(l, ..)| *l);
    for (n, (label, p, ..)) in order.iter().enumerate() {
        let at = format!("SERIAL{n}");
        let Some(param) = params.iter().find(|q| usize::from(q.port) == n) else {
            out.push(
                "MissingSerialParameters",
                &at,
                Some(label),
                format!("SERIAL{n}'s parameters in var_info"),
                format!("none: var_info compiles no SERIAL{n} row (AP_SerialManager.cpp:164-400)"),
            );
            continue;
        };
        // belt-and-suspenders: the generator's `protocol()` maps a role to
        // the define it writes; this match is the judge's own (`RUL-20`).
        let want = match p.role {
            SerialRole::Console | SerialRole::Mavlink => MAVLINK2,
            SerialRole::Gps => GPS,
            SerialRole::RcCrsf | SerialRole::RcSbus => RCIN,
        };
        let runs = i64::from(param.protocol);
        if runs != want {
            out.push(
                "WrongSerialProtocol",
                &at,
                Some(label),
                format!("{}, for a {} port", protocol_name(want), p.role.name()),
                protocol_name(runs),
            );
        }
        let declared = |bits: i64| {
            if bits == 0 {
                "none".to_owned()
            } else {
                format!("{bits:#x}")
            }
        };
        let Some(options) = param.options.map(i64::from) else {
            if p.framing.inverted {
                out.push(
                    "WrongSerialOptions",
                    &at,
                    Some(label),
                    "RXINV and TXINV (declared inverted)".into(),
                    format!(
                        "none: SERIAL{n} has no options parameter (AP_SerialManager.cpp:165-181)"
                    ),
                );
            }
            continue;
        };
        let (wrong, expected) = if want == RCIN {
            (
                options & !RXINV != 0,
                "no bit but RXINV, which RC input sets (AP_RCProtocol.cpp:326-336)".to_owned(),
            )
        } else {
            let want = if p.framing.inverted { RXINV | TXINV } else { 0 };
            (options != want, declared(want))
        };
        if wrong {
            out.push(
                "WrongSerialOptions",
                &at,
                Some(label),
                expected,
                declared(options),
            );
        }
    }
    let (mut rcin, mut gps) = (Vec::new(), Vec::new());
    for q in params {
        let (n, runs) = (usize::from(q.port), i64::from(q.protocol));
        let at = format!("SERIAL{n}");
        if runs == RCIN {
            if let Some(first) = rcin.first() {
                out.push(
                    "RcInputTwice",
                    &at,
                    net_of(n),
                    format!("one RCIN port, {first} (AP_SerialManager.cpp:536-541)"),
                    protocol_name(RCIN),
                );
            }
            rcin.push(at);
        } else if let Some(cap) = receivers.filter(|_| runs == GPS || runs == GPS2) {
            if gps.len() >= usize::from(cap) {
                let s = if cap == 1 { "" } else { "s" };
                out.push(
                    "GpsPortUnbound",
                    &at,
                    net_of(n),
                    format!(
                        "at most {cap} GPS port{s}, {} (AP_GPS::var_info compiles {cap} receiver{s}; AP_GPS.cpp:345-349)",
                        if gps.is_empty() {
                            "none".to_owned()
                        } else {
                            gps.join(" and ")
                        }
                    ),
                    protocol_name(runs),
                );
            } else {
                gps.push(at);
            }
        }
    }
    if receivers.is_none() {
        for (n, (label, p, ..)) in order.iter().enumerate() {
            if p.role == SerialRole::Gps {
                out.push(
                    "GpsNotCompiled",
                    &format!("SERIAL{n}"),
                    Some(label),
                    "AP_GPS, to bind this GPS port".into(),
                    "none: the ELF has no AP_GPS::var_info".into(),
                );
            }
        }
    }
}

/// The barometer probes the list compiles, in order, against the topology's
/// barometers (the I2C devices whose part's ArduPilot row is a barometer), in
/// index order: each the row's driver, on its solved controller, at its
/// address.
fn baros(
    t: &SolvedTopology,
    parts: &PartsTable,
    h: &Preprocessed,
    out: &mut Out,
) -> Result<(), Refusal> {
    let buses = bus_list(h, "HAL_I2C_DEVICE_LIST", "I2CD")?;
    let mut probes = Vec::new();
    for (name, stmt) in expand(h, "HAL_BARO_PROBE_LIST", ';')? {
        if stmt == "RETURN_IF_NO_SPACE" {
            continue;
        }
        let parsed = (|| {
            let a = args(inside(&stmt, "probe_i2c_dev(", ")")?);
            let [driver, bus, addr] = a.as_slice() else {
                return None;
            };
            let driver = driver
                .strip_prefix("AP_Baro_")?
                .strip_suffix("::probe")?
                .to_owned();
            let bus = buses.get(usize::try_from(number(bus)?).ok()?)?;
            Some((
                driver,
                format!("I2C{bus}"),
                u8::try_from(number(addr)?).ok()?,
            ))
        })();
        match parsed {
            Some(p) => probes.push((name, p)),
            None => {
                return malformed(
                    &name,
                    format!(
                        "is not probe_i2c_dev(AP_Baro_<driver>::probe, bus, address) on a listed bus: `{stmt}`"
                    ),
                );
            }
        }
    }
    let want: Vec<(&str, (String, String, u8))> = t
        .i2c_devices_in_order()
        .into_iter()
        .filter_map(|(r, d)| {
            let a = parts.get(&d.part)?.ardupilot.as_ref()?;
            (a.kind == ArduPilotKind::Baro).then(|| {
                (
                    r,
                    (
                        a.driver.clone().unwrap_or_default(),
                        d.peripheral.clone(),
                        d.address,
                    ),
                )
            })
        })
        .collect();
    let show = |(d, b, a): &(String, String, u8)| format!("{d} on {b} at {a:#04x}");
    for i in 0..want.len().max(probes.len()) {
        let at = probes
            .get(i)
            .map_or_else(|| "HAL_BARO_PROBE_LIST".to_owned(), |(n, _)| n.clone());
        match (want.get(i), probes.get(i)) {
            (Some((r, w)), Some((_, p))) if w != p => {
                out.push("WrongBaroProbe", &at, Some(r), show(w), show(p));
            }
            (Some(_), Some(_)) | (None, None) => {}
            (Some((r, w)), None) => {
                out.push("MissingBaroProbe", &at, Some(r), show(w), "none".into());
            }
            (None, Some((_, p))) => {
                out.push("UnexpectedBaroProbe", &at, None, "none".into(), show(p));
            }
        }
    }
    Ok(())
}

/// A rotation's matrix, row `i` board axis `i` in the driver's x, y, z.
type Matrix = [[i8; 3]; 3];

/// ArduPilot's axis-aligned `Rotation`s at the tag, each as the matrix
/// `Vector3<T>::rotate` applies (`libraries/AP_Math/vector3.cpp`, each `case`
/// evaluated on the basis vectors; `rotations.h` L27-83; `AET-WO-4400d`
/// FEEDBACK §1.1): row `i` is board axis `i` (x forward, y right, z down) in
/// terms of the driver's x, y, z. Every name is here, aliases too (26 and
/// 27), because a rotation is judged by what it does, not how it is spelled
/// (ruled D4). This table is the judge's own (`RUL-20`).
const ROTATIONS: [(&str, Matrix); 26] = [
    ("ROTATION_NONE", [[1, 0, 0], [0, 1, 0], [0, 0, 1]]),
    ("ROTATION_YAW_90", [[0, -1, 0], [1, 0, 0], [0, 0, 1]]),
    ("ROTATION_YAW_180", [[-1, 0, 0], [0, -1, 0], [0, 0, 1]]),
    ("ROTATION_YAW_270", [[0, 1, 0], [-1, 0, 0], [0, 0, 1]]),
    ("ROTATION_ROLL_180", [[1, 0, 0], [0, -1, 0], [0, 0, -1]]),
    (
        "ROTATION_ROLL_180_YAW_90",
        [[0, 1, 0], [1, 0, 0], [0, 0, -1]],
    ),
    ("ROTATION_PITCH_180", [[-1, 0, 0], [0, 1, 0], [0, 0, -1]]),
    (
        "ROTATION_ROLL_180_YAW_270",
        [[0, -1, 0], [-1, 0, 0], [0, 0, -1]],
    ),
    ("ROTATION_ROLL_90", [[1, 0, 0], [0, 0, -1], [0, 1, 0]]),
    ("ROTATION_ROLL_90_YAW_90", [[0, 0, 1], [1, 0, 0], [0, 1, 0]]),
    ("ROTATION_ROLL_270", [[1, 0, 0], [0, 0, 1], [0, -1, 0]]),
    (
        "ROTATION_ROLL_270_YAW_90",
        [[0, 0, -1], [1, 0, 0], [0, -1, 0]],
    ),
    ("ROTATION_PITCH_90", [[0, 0, 1], [0, 1, 0], [-1, 0, 0]]),
    ("ROTATION_PITCH_270", [[0, 0, -1], [0, 1, 0], [1, 0, 0]]),
    (
        "ROTATION_PITCH_180_YAW_90",
        [[0, -1, 0], [-1, 0, 0], [0, 0, -1]],
    ),
    (
        "ROTATION_PITCH_180_YAW_270",
        [[0, 1, 0], [1, 0, 0], [0, 0, -1]],
    ),
    (
        "ROTATION_ROLL_90_PITCH_90",
        [[0, 1, 0], [0, 0, -1], [-1, 0, 0]],
    ),
    (
        "ROTATION_ROLL_180_PITCH_90",
        [[0, 0, -1], [0, -1, 0], [-1, 0, 0]],
    ),
    (
        "ROTATION_ROLL_270_PITCH_90",
        [[0, -1, 0], [0, 0, 1], [-1, 0, 0]],
    ),
    (
        "ROTATION_ROLL_90_PITCH_180",
        [[-1, 0, 0], [0, 0, -1], [0, -1, 0]],
    ),
    (
        "ROTATION_ROLL_270_PITCH_180",
        [[-1, 0, 0], [0, 0, 1], [0, 1, 0]],
    ),
    (
        "ROTATION_ROLL_90_PITCH_270",
        [[0, -1, 0], [0, 0, -1], [1, 0, 0]],
    ),
    (
        "ROTATION_ROLL_180_PITCH_270",
        [[0, 0, 1], [0, -1, 0], [1, 0, 0]],
    ),
    (
        "ROTATION_ROLL_270_PITCH_270",
        [[0, 1, 0], [0, 0, 1], [1, 0, 0]],
    ),
    (
        "ROTATION_ROLL_90_PITCH_180_YAW_90",
        [[0, 0, 1], [-1, 0, 0], [0, -1, 0]],
    ),
    (
        "ROTATION_ROLL_90_YAW_270",
        [[0, 0, -1], [-1, 0, 0], [0, 1, 0]],
    ),
];

/// The matrix taking the chip's axes to the board's for a declared
/// orientation, the driver passing the chip's axes unchanged (the only
/// `ardupilot.frame` the parts table accepts): column `j` is chip axis `j`
/// in the board frame, Y being Z × X. The board frame is the engine's
/// constant: x forward, y right, z down through F.Cu (ruled D3).
fn expected_matrix(o: &Orientation) -> Matrix {
    // Each direction as (axis, sign) in the board frame.
    let axis = |d: Toward| match d {
        Toward::Forward => (0, 1),
        Toward::Aft => (0, -1),
        Toward::Right => (1, 1),
        Toward::Left => (1, -1),
        Toward::Down => (2, 1),
        Toward::Up => (2, -1),
    };
    let mut m = [[0i8; 3]; 3];
    let (xa, xs) = axis(o.x);
    let (za, zs) = axis(o.z);
    m[xa][0] = xs;
    m[za][2] = zs;
    // Y = Z × X: on the remaining axis, with the sign the cross product
    // gives (e_a × e_b = +e_c for (a, b, c) a cyclic order of 0, 1, 2).
    let ya = 3 - xa - za;
    let cyclic = (za + 1) % 3 == xa;
    m[ya][1] = zs * xs * if cyclic { 1 } else { -1 };
    m
}

/// The IMU probes the list compiles, in order, against the topology's IMUs
/// (the SPI devices whose part's ArduPilot row is an IMU), in index order:
/// each the row's driver, on its SPI device name, with a rotation whose
/// matrix is the declared orientation's; and each driver's
/// `probe(AP_InertialSensor&, OwnPtr<Device>, Rotation)` defined in the ELF
/// under its mangled name (measured in the CI-built ELF, FEEDBACK §5).
fn imus(
    t: &SolvedTopology,
    parts: &PartsTable,
    h: &Preprocessed,
    elf_bytes: &[u8],
    out: &mut Out,
) -> Result<(), Refusal> {
    let mut probes = Vec::new();
    for (name, stmt) in expand(h, "HAL_INS_PROBE_LIST", ';')? {
        let parsed = (|| {
            let call = inside(&stmt, "ADD_BACKEND(", ")")?;
            let (driver, rest) = call
                .strip_prefix("AP_InertialSensor_")?
                .split_once("::probe(")?;
            let a = args(rest.strip_suffix(')')?);
            let [this, dev, rotation] = a.as_slice() else {
                return None;
            };
            let dev = inside(dev, "hal.spi->get_device(\"", "\")")?;
            (this == "*this" && is_name(driver) && is_name(dev) && is_name(rotation))
                .then(|| (driver.to_owned(), dev.to_owned(), rotation.clone()))
        })();
        match parsed {
            Some((driver, dev, rotation)) => {
                // A rotation is an enumerator; a macro of that name would
                // stand in for it (adversarial).
                if h.value(&rotation)?.is_some() {
                    return malformed(
                        &rotation,
                        "is defined as a macro, where ArduPilot's Rotation enumerator stands",
                    );
                }
                let Some((_, m)) = ROTATIONS.iter().find(|(n, _)| *n == rotation) else {
                    return malformed(
                        &name,
                        format!("names `{rotation}`, not one of ArduPilot's right-angle rotations"),
                    );
                };
                probes.push((name, driver, dev, rotation, *m));
            }
            None => {
                return malformed(
                    &name,
                    format!(
                        "is not ADD_BACKEND(AP_InertialSensor_<driver>::probe(*this,hal.spi->get_device(\"<name>\"),ROTATION_<x>)): `{stmt}`"
                    ),
                );
            }
        }
    }
    // `HAL_SITL_INVENSENSEV3` compiles one more probe after the list, on
    // I2C bus 1 at address 1, `ROTATION_NONE` (`AP_InertialSensor.cpp`
    // L1223-1225): no topology declares it.
    if h.value("HAL_SITL_INVENSENSEV3")?.is_some() {
        out.push(
            "UnexpectedImuProbe",
            "HAL_SITL_INVENSENSEV3",
            None,
            "none".into(),
            "Invensensev3 on I2C bus 1 at 0x01, ROTATION_NONE".into(),
        );
    }
    // The topology's IMUs: (reference, driver, device, matrix), the matrix
    // `None` where no orientation is declared or the driver's frame is
    // uncited, which no rotation can match.
    let want: Vec<(&str, String, String, Option<Matrix>)> = t
        .spi_devices_in_order()
        .into_iter()
        .filter_map(|(r, d)| {
            let a = parts.get(&d.part)?.ardupilot.as_ref()?;
            (a.kind == ArduPilotKind::Imu).then(|| {
                // belt-and-suspenders: `validate` refuses an orientation
                // whose part cites no driver frame; a hand-written topology
                // can still pair them, and no rotation is then right.
                let m = d
                    .orientation
                    .filter(|_| a.frame.is_some())
                    .map(|o| expected_matrix(&o));
                (
                    r,
                    a.driver.clone().unwrap_or_default(),
                    a.spidev.clone().unwrap_or_default(),
                    m,
                )
            })
        })
        .collect();
    let show_m = |m: &Option<Matrix>| match m {
        Some(m) => ROTATIONS
            .iter()
            .filter(|(_, r)| r == m)
            .map(|(n, _)| *n)
            .collect::<Vec<_>>()
            .join(" or "),
        None => "an undeclared orientation".to_owned(),
    };
    for i in 0..want.len().max(probes.len()) {
        let at = probes
            .get(i)
            .map_or_else(|| "HAL_INS_PROBE_LIST".to_owned(), |p| p.0.clone());
        match (want.get(i), probes.get(i)) {
            (Some((r, wd, wdev, wm)), Some((_, pd, pdev, pr, pm))) => {
                if (wd, wdev) != (pd, pdev) {
                    out.push(
                        "WrongImuProbe",
                        &at,
                        Some(r),
                        format!("{wd} on SPI device {wdev}"),
                        format!("{pd} on SPI device {pdev}"),
                    );
                } else if wm.as_ref() != Some(pm) {
                    out.push("WrongImuRotation", &at, Some(r), show_m(wm), pr.clone());
                }
                let probe = format!(
                    "_ZN{}AP_InertialSensor_{pd}5probeER17AP_InertialSensorN6AP_HAL6OwnPtrINS2_6DeviceEEE8Rotation",
                    "AP_InertialSensor_".len() + pd.len()
                );
                if !elf::defines_function(elf_bytes, &probe)? {
                    out.push(
                        "ImuDriverNotLinked",
                        &at,
                        Some(r),
                        format!("{probe} defined"),
                        "not in the ELF's symbol table".to_owned(),
                    );
                }
            }
            (None, None) => {}
            (Some((r, wd, wdev, wm)), None) => {
                out.push(
                    "MissingImuProbe",
                    &at,
                    Some(r),
                    format!("{wd} on SPI device {wdev}, {}", show_m(wm)),
                    "none".into(),
                );
            }
            (None, Some((_, pd, pdev, pr, _))) => {
                out.push(
                    "UnexpectedImuProbe",
                    &at,
                    None,
                    "none".into(),
                    format!("{pd} on SPI device {pdev}, {pr}"),
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The judge's own `{x, z}` matrices: every right-angle pair names one
    /// of the table's rotations, the 24 pairs 24 different ones; the
    /// sentinel-02-own declaration (D5) is `ROTATION_ROLL_180_YAW_90` or its
    /// alias 27; and each matrix in the table is a rotation (determinant 1).
    #[test]
    fn each_declared_orientation_is_one_of_ardupilots_rotations() {
        let all = [
            Toward::Forward,
            Toward::Aft,
            Toward::Right,
            Toward::Left,
            Toward::Up,
            Toward::Down,
        ];
        let mut seen = Vec::new();
        for x in all {
            for z in all.into_iter().filter(|z| *z != x && *z != x.opposite()) {
                let m = expected_matrix(&Orientation { x, z });
                assert!(ROTATIONS.iter().any(|(_, r)| *r == m), "{x:?} {z:?}");
                assert!(!seen.contains(&m), "{x:?} {z:?}");
                seen.push(m);
            }
        }
        assert_eq!(seen.len(), 24);
        let d5 = expected_matrix(&Orientation {
            x: Toward::Right,
            z: Toward::Up,
        });
        let names: Vec<&str> = ROTATIONS
            .iter()
            .filter(|(_, r)| *r == d5)
            .map(|(n, _)| *n)
            .collect();
        assert_eq!(
            names,
            ["ROTATION_ROLL_180_YAW_90", "ROTATION_PITCH_180_YAW_270"]
        );
        for (n, m) in ROTATIONS {
            let det = i32::from(m[0][0])
                * (i32::from(m[1][1]) * i32::from(m[2][2])
                    - i32::from(m[1][2]) * i32::from(m[2][1]))
                - i32::from(m[0][1])
                    * (i32::from(m[1][0]) * i32::from(m[2][2])
                        - i32::from(m[1][2]) * i32::from(m[2][0]))
                + i32::from(m[0][2])
                    * (i32::from(m[1][0]) * i32::from(m[2][1])
                        - i32::from(m[1][1]) * i32::from(m[2][0]));
            assert_eq!(det, 1, "{n}");
        }
    }

    #[test]
    fn lines_numbers_and_arguments_read_as_c_and_ardupilot_write_them() {
        assert_eq!(line_pin(0x5802_040e).as_deref(), Some("PB14"));
        assert_eq!(line_pin(0x5802_000f).as_deref(), Some("PA15"));
        assert_eq!(line_pin(0x5802_0010), None);
        assert_eq!(line_pin(0x4002_0000), None);
        assert_eq!(pal_line("PAL_LINE(GPIOA,0U)").as_deref(), Some("PA0"));
        assert_eq!(pal_line("PAL_LINE(GPIOZ,0U)"), None);
        assert_eq!(
            args(r#""dataflash" , 0, 1, PAL_LINE(GPIOA,0U) , SPIDEV_MODE0, 16*MHZ, 16*MHZ"#),
            [
                "\"dataflash\"",
                "0",
                "1",
                "PAL_LINE(GPIOA,0U)",
                "SPIDEV_MODE0",
                "16*MHZ",
                "16*MHZ"
            ]
        );
        // C integer constants as GCC reads them (adversarial A3).
        assert_eq!(number("0115200"), Some(39_552));
        assert_eq!(number("0x61A80U"), Some(400_000));
        assert_eq!(number("400000UL"), Some(400_000));
        assert_eq!(number("0"), Some(0));
        assert_eq!(number("09"), None);
        for signed in ["0+303240", "+400000", "0x+61A80", "-1", "0U"] {
            assert_eq!(number(signed), (signed == "0U").then_some(0), "{signed}");
        }
        assert_eq!(number("18446744073709551616"), None);
        assert_eq!(map_baudrate(57), 57_600);
        assert_eq!(map_baudrate(14), 14_000);
        assert_eq!(map_baudrate(420_000), 420_000);
    }
}
