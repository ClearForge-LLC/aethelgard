//! The names the generators take for themselves, held in one place both
//! stages read (`AET-WO-1206`): `validate` refuses a specification that
//! spells one, and the schematic emitter names no net this module does not
//! list (`aethelgard_eda::schematic`'s `own_net`).
//!
//! Step 20 of [`Specification::validate`] refuses, by the name and with the
//! JSON `generate` printed for each before it existed:
//!
//! - a reference the emitter would also number, or two of the specification's
//!   that KiCad takes for one (`DuplicateReference`, A4);
//! - a reference starting `#`, which KiCad drops (`ReservedName`);
//! - a net or domain named for one of the emitter's nets, or starting with
//!   KiCad's `unconnected-` (`ReservedName`, A5);
//! - a device reference named for a peripheral instance of the MCU
//!   (`InvalidDeviceName`, A2).

use std::collections::BTreeSet;

use crate::power::{self, BATTERY_NET};
use crate::power_parts::{self, Kind};
use crate::{BOOT0, GND, IrValidationError as E, NRST, SWCLK, SWDIO, Specification};

/// KiCad's prefix for a pad on no net. Its parity check matches it by
/// prefix, case-sensitively, so the emitter refuses a net or rail that
/// starts with it (`aethelgard_eda::pcb`), and so does `validate`.
pub const UNCONNECTED: &str = "unconnected-";

/// The nets the emitter names `<reference>_<suffix>` for a device of each
/// part: a strap or a filter pin it draws a passive on (`BME280`'s and
/// `BMP581`'s `SDO` strap, the flash's `WP` and `HOLD` straps, the
/// ICM-20948's `REGOUT` capacitor). A part not listed has none.
pub fn device_nets(part: &str) -> &'static [&'static str] {
    match part {
        "BME280" | "BMP581" => &["SDO"],
        "W25Q64JVxIQ" => &["WP", "HOLD"],
        "ICM20948" => &["REGOUT"],
        _ => &[],
    }
}

/// The nets the emitter names `<reference>_<suffix>` for a buck regulator.
pub const BUCK_NETS: [&str; 4] = ["FB", "VCC", "BOOT", "SW"];

/// The peripheral instances of the MCU a device reference may not be named
/// for, in any case (A2). The twin refuses a device named for a peripheral
/// the solve used (`aethelgard_twin::platform`, `InvalidDeviceName`); this is
/// every instance a solve can use, so `validate` refuses without solving:
/// GPIO ports A to K (the stock platform's ports, which bound any pin's),
/// `I2C1`–`I2C4` and `SPI1`–`SPI6` (the instances the H753's pin table
/// offers), each of [`crate::serial::PERIPHERALS`], and `TIM<n>` for each of
/// [`crate::motor::TIMERS`].
pub fn peripheral_instances() -> Vec<String> {
    let mut out: Vec<String> = ('A'..='K').map(|p| format!("GPIO{p}")).collect();
    out.extend((1..=4).map(|n| format!("I2C{n}")));
    out.extend((1..=6).map(|n| format!("SPI{n}")));
    out.extend(crate::serial::PERIPHERALS.iter().map(|p| (*p).to_owned()));
    out.extend(crate::motor::TIMERS.iter().map(|n| format!("TIM{n}")));
    out
}

/// A reference as KiCad compares references: ASCII case folded, and the
/// trailing number without its leading zeros (`U01` is `U1`). Two parts with
/// one key are a duplicate to KiCad (`AET-WO-1301` adversarial F4); the
/// board writer refuses them (`aethelgard_eda::pcb`).
pub fn reference_key(reference: &str) -> String {
    let upper = reference.to_ascii_uppercase();
    let stem = upper.trim_end_matches(|c: char| c.is_ascii_digit());
    let digits = upper[stem.len()..].trim_start_matches('0');
    match (upper.len() > stem.len(), digits.is_empty()) {
        (false, _) => upper,
        (true, true) => format!("{stem}0"),
        (true, false) => format!("{stem}{digits}"),
    }
}

/// Whether a reference with KiCad key `key` can meet one the emitter numbers
/// itself (A4). The emitter draws one `U`, the MCU, as `mcu` (exact: the
/// first `U<n>` the specification does not spell). It numbers its resistors
/// and capacitors `R<n>` and `C<n>` from 1, skipping each spelled exactly,
/// and how far it counts depends on what it draws; so every `R` or `C` and a
/// number from 1 with no leading zero is held, a pattern no example, fixture
/// or scenario uses (`AET-WO-1206` §1.1, measured).
fn emitter_numbered(key: &str, mcu: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some('U') => key == mcu,
        Some('R' | 'C') => {
            chars
                .as_str()
                .starts_with(|c: char| ('1'..='9').contains(&c))
                && chars.all(|c| c.is_ascii_digit())
        }
        _ => false,
    }
}

/// Every reference the specification declares, in the emitter's claim order:
/// I2C devices, SPI devices, connectors, the power tree's, then the test
/// points (`AET-WO-1312`).
pub(crate) fn references(spec: &Specification) -> Vec<&str> {
    spec.declared_i2c_devices()
        .iter()
        .map(|d| d.reference.as_str())
        .chain(
            spec.declared_spi_devices()
                .iter()
                .map(|d| d.reference.as_str()),
        )
        .chain(
            spec.declared_connectors()
                .iter()
                .map(|c| c.reference.as_str()),
        )
        .chain(
            spec.power
                .iter()
                .flat_map(power::references)
                .map(String::as_str),
        )
        .chain(
            spec.declared_test_points()
                .iter()
                .map(|t| t.reference.as_str()),
        )
        .collect()
}

/// The MCU's reference: the emitter numbers it `U<n>`, the first `n` from 1
/// whose `U<n>` the specification does not spell exactly.
pub fn mcu_reference(spec: &Specification) -> String {
    let spelled: BTreeSet<&str> = references(spec).into_iter().collect();
    (1..)
        .map(|n| format!("U{n}"))
        .find(|r| !spelled.contains(r.as_str()))
        .expect("a finite specification leaves some U<n> free")
}

/// Every net the schematic emitter names for itself, for `spec`: `GND`, the
/// battery net when a power tree is declared, the MCU's support nets as its
/// power-parts row draws them (`NRST`, `SWDIO`, `SWCLK` and `BOOT0` for each
/// group the row has, `BOOT0` drawn only when a connector or test point
/// carries it (`AET-WO-1312`), and `<mcu>_VCAP<n>` for each `VCAP` pin, `AET-WO-4300g`), each
/// device's [`device_nets`], each buck's [`BUCK_NETS`], and each crossing's
/// four translated lines. The emitter refuses to name a net this list lacks.
pub fn own_nets(spec: &Specification) -> Result<Vec<String>, E> {
    let mut out = vec![GND.to_owned()];
    if spec.power.is_some() {
        out.push(BATTERY_NET.to_owned());
    }
    let table = power_parts::table()?;
    if let Some(Kind::Mcu(m)) = table.get(&spec.mcu.part).map(|row| &row.kind) {
        for net in [NRST, SWDIO, SWCLK] {
            if !m.support.group(net).is_empty() {
                out.push(net.to_owned());
            }
        }
        // `BOOT0` is drawn on its own net only when a connector pin or test
        // point carries it (`AET-WO-1312`); otherwise the strap wires it.
        if crate::bench::carries_boot0(spec) && !m.support.group(BOOT0).is_empty() {
            out.push(BOOT0.to_owned());
        }
        let mcu = mcu_reference(spec);
        out.extend((1..=m.support.group("VCAP").len()).map(|n| format!("{mcu}_VCAP{n}")));
    }
    let devices = spec
        .declared_i2c_devices()
        .iter()
        .map(|d| (&d.reference, &d.part))
        .chain(
            spec.declared_spi_devices()
                .iter()
                .map(|d| (&d.reference, &d.part)),
        );
    for (reference, part) in devices {
        out.extend(device_nets(part).iter().map(|s| format!("{reference}_{s}")));
    }
    if let Some(p) = &spec.power {
        for r in &p.regulators {
            if let Some(Kind::Buck(_)) = table.get(&r.part).map(|row| &row.kind) {
                out.extend(BUCK_NETS.iter().map(|s| format!("{}_{s}", r.reference)));
            }
        }
        for c in &p.crossings {
            if let Some(Kind::Translator(t)) = table.get(&c.part).map(|row| &row.kind)
                && let Some(lines) = t.crossing_lines()
            {
                out.extend(lines.iter().map(|s| format!("{}_{s}", c.reference)));
            }
        }
    }
    Ok(out)
}

/// Every net and domain name the specification gives, in document order:
/// domains, the I2C and SPI lines, chip selects, serial lines, motor
/// outputs, then the power tree's sense nets. The emitter holds its own nets
/// against this list (`AET-WO-1207` §1.4), as step 20 does.
pub fn spelled_nets(spec: &Specification) -> Vec<&str> {
    let mut out: Vec<&str> = spec.voltage_domains.iter().map(|d| d.id.as_str()).collect();
    for b in &spec.i2c_buses {
        out.extend([b.sda.net.as_str(), b.scl.net.as_str()]);
    }
    for b in &spec.spi_buses {
        out.extend([b.sck.net.as_str(), b.mosi.net.as_str(), b.miso.net.as_str()]);
    }
    out.extend(
        spec.declared_spi_devices()
            .iter()
            .map(|d| d.cs.net.as_str()),
    );
    for p in &spec.serial_ports {
        out.extend([p.tx.net.as_str(), p.rx.net.as_str()]);
    }
    out.extend(spec.motor_outputs.iter().map(|m| m.label.as_str()));
    if let Some(p) = &spec.power {
        out.extend(p.sense.iter().map(|s| s.net.as_str()));
    }
    out
}

/// `name` against a name the emitter holds: the name alone when spelled the
/// same, else both, the emitter's first, as the emitter's own refusal names
/// them.
fn pair(own: &str, spelled: &str) -> String {
    if own == spelled {
        own.to_owned()
    } else {
        format!("{own} and {spelled}")
    }
}

/// Step 20 of `validate`: the names (`AET-WO-1206` §1.3). In `generate`'s
/// order: the emitter's references (A4) and nets (A5), then the twin's
/// device names (A2).
pub(crate) fn check(spec: &Specification) -> Result<(), E> {
    let mcu = mcu_reference(spec);
    for r in references(spec) {
        // KiCad drops a reference starting `#` from the netlist and the bill
        // of materials (its power symbols' prefix); the emitter refuses one,
        // and only the power tree's references can spell it (adversarial F2).
        if r.starts_with('#') {
            return Err(E::ReservedName(r.to_owned()));
        }
        let key = reference_key(r);
        if emitter_numbered(&key, &mcu) && r != key {
            let mut both = [key.as_str(), r];
            both.sort_unstable();
            return Err(E::EmitterReference(both.join(" and ")));
        }
    }
    // The reset switch is numbered as the MCU is (`AET-WO-1312`): a reference
    // KiCad takes for it but spelled otherwise (`sw1`, `SW01`) collides.
    if let Some(sw) = crate::bench::switch_reference(spec)
        && let Some(r) = references(spec)
            .into_iter()
            .find(|r| reference_key(r) == sw && *r != sw)
    {
        let mut both = [sw.as_str(), r];
        both.sort_unstable();
        return Err(E::EmitterReference(both.join(" and ")));
    }
    let own = own_nets(spec)?;
    // A power reference starting `unconnected-` names the emitter's own nets
    // so (`unconnected-x_FB`), which the board writer refuses (adversarial F3).
    if let Some(o) = own.iter().find(|o| o.starts_with(UNCONNECTED)) {
        return Err(E::ReservedName(o.clone()));
    }
    for name in spelled_nets(spec) {
        if name.starts_with(UNCONNECTED) {
            return Err(E::ReservedName(name.to_owned()));
        }
        if let Some(o) = own.iter().find(|o| o.eq_ignore_ascii_case(name)) {
            return Err(E::ReservedName(pair(o, name)));
        }
    }
    // Two references of the specification with one KiCad key that differ in
    // more than case (`J1` and `J01`): step 8 and the power tree's check fold
    // case only, and the board writer refuses the pair (adversarial F1).
    let mut keys = std::collections::BTreeMap::new();
    for r in references(spec) {
        if let Some(other) = keys.insert(reference_key(r), r) {
            let mut both = [other, r];
            both.sort_unstable();
            return Err(E::EmitterReference(both.join(" and ")));
        }
    }
    let instances = peripheral_instances();
    let devices = spec
        .declared_i2c_devices()
        .iter()
        .map(|d| &d.reference)
        .chain(spec.declared_spi_devices().iter().map(|d| &d.reference));
    for reference in devices {
        if instances.iter().any(|p| p.eq_ignore_ascii_case(reference)) {
            return Err(E::InvalidDeviceName {
                reference: reference.clone(),
            });
        }
    }
    Ok(())
}

/// Step 20a of `validate` (`AET-WO-1209` §1.2, D3 as ruled 2026-11-19): each
/// `power.loads` reference names a part the board has, in document order:
///
/// 1. one the specification declares ([`references`]);
/// 2. the MCU, numbered as the emitter numbers it ([`mcu_reference`]);
/// 3. the reset switch ([`crate::bench::switch_reference`], which the
///    emitter itself calls);
/// 4. a resistor or capacitor the emitter numbers (`R<n>`, `C<n>`), by
///    [`emitter_numbered`]'s pattern.
///
/// Which `R<n>` and `C<n>` the emitter draws is its draw order
/// (`aethelgard_eda::schematic::design`); the CLI's `validate` holds a load
/// to that exact set. Neither catches an `R<n>` that names a different part
/// once an earlier one leaves and the numbers move.
///
/// A reference that names a part, judged entry by entry after the above,
/// must not name one whose current its own row already counts
/// (`AET-WO-1209c`, [`crate::power::Load::reference`]): the MCU, counted by
/// its table row, or a declared regulator, counted as its input's load.
pub(crate) fn loads(spec: &Specification) -> Result<(), E> {
    let Some(power) = &spec.power else {
        return Ok(());
    };
    let declared = references(spec);
    let mcu = mcu_reference(spec);
    let switch = crate::bench::switch_reference(spec);
    for (i, l) in power.loads.iter().enumerate() {
        let Some(r) = &l.reference else { continue };
        // `emitter_numbered` with no MCU matches `R<n>` and `C<n>` only.
        let has = declared.contains(&r.as_str())
            || *r == mcu
            || switch.as_deref() == Some(r.as_str())
            || emitter_numbered(r, "");
        if !has {
            return Err(E::LoadNamesNoPart {
                path: format!("power.loads[{i}].reference"),
                domain: l.domain.clone(),
                reference: r.clone(),
            });
        }
        let counted_by = if *r == mcu {
            Some("the MCU's table row".to_owned())
        } else {
            power
                .regulators
                .iter()
                .find(|g| g.reference == *r)
                .map(|g| format!("regulator `{r}`'s input on `{}`", g.input))
        };
        if let Some(counted_by) = counted_by {
            return Err(E::LoadNamesCountedPart {
                path: format!("power.loads[{i}].reference"),
                domain: l.domain.clone(),
                reference: r.clone(),
                counted_by,
            });
        }
    }
    Ok(())
}

/// Windows' reserved device names (`AET-WO-1102` A7): a file whose stem is
/// one of these, in any case, cannot be opened on Windows. `com0` and `lpt0`
/// are on Microsoft's list too (`AET-WO-1207` §1.3, `1206` F7); its
/// superscript forms are not ASCII, which every identifier already is.
pub const WINDOWS_RESERVED: [&str; 24] = [
    "aux", "com0", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9", "con",
    "lpt0", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9", "nul", "prn",
];

/// Whether a name's stem, before its first `.` and without trailing spaces,
/// is a Windows reserved device name in any case.
pub fn windows_reserved(name: &str) -> bool {
    // Windows drops trailing spaces from a name before it matches a device.
    let stem = name.split('.').next().unwrap_or(name).trim_end_matches(' ');
    WINDOWS_RESERVED
        .iter()
        .any(|w| w.eq_ignore_ascii_case(stem))
}
