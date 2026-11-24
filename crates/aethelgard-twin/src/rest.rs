//! A board at rest (`AET-WO-4400i` D4, ruled on PR #396): the stimuli a
//! foreign run with a declared input starts from, derived from the board's
//! declared orientation and the parts' datasheets, never from the firmware
//! and never a constant the judge invents (`INV-07`). A run without an input
//! keeps the parts table's witness stimuli, so no committed verdict moves.
//!
//! - **An accelerometer** reads +1 g on a chip axis declared `up`, −1 g on
//!   one declared `down`, and 0 on the others: TDK InvenSense DS-000347 rev
//!   1.9 §4.7 (sha256 `9c6b57b0…`, the parts table's cited copy), "When the
//!   device is placed on a flat surface, it will measure 0g on the X- and
//!   Y-axes and +1g on the Z-axis". The board lies F.Cu up, so `up` is the
//!   board frame's −z (`aethelgard_ir::orientation`). An IMU with no declared
//!   orientation has no rest: refused (`RestOrientationUndeclared`, `INV-06`).
//! - **A barometer** reads the parts table's pressure, moved by its
//!   intrinsic noise ([`REST`]): a part with no row is refused
//!   (`RestNoiseUndeclared`).

use crate::TwinError;
use crate::boot::{QUANTUM_NS, StepCheck};
use crate::model::Model;
use aethelgard_ir::SolvedTopology;
use aethelgard_ir::orientation::{Orientation, Toward};
use aethelgard_ir::parts::{self, Quantity, StepKind};

/// Each pressure part's noise, with its citation (`data/rest.tsv`).
pub const REST: &str = include_str!("../data/rest.tsv");

/// The accelerometer's citation, as the verdict names it.
pub const AT_REST: &str = "TDK InvenSense DS-000347 rev 1.9 §4.7: on a flat surface 0g on X and Y and +1g on Z; each chip axis by the device's declared orientation";

/// One stimulus: at `at_ns`, `sysbus.<device> <property> <value>`, the value
/// exactly as Renode is given it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stimulus {
    pub at_ns: u64,
    pub reference: String,
    pub device: String,
    pub property: &'static str,
    pub value: String,
    pub citation: String,
}

impl Stimulus {
    /// The monitor command.
    pub fn command(&self) -> String {
        format!("sysbus.{} {} {}", self.device, self.property, self.value)
    }
}

/// A row of [`REST`]: (part, amplitude in mPa, period in ns, citation).
fn rows() -> Result<Vec<(&'static str, u32, u64, &'static str)>, TwinError> {
    REST.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let bad = || TwinError::MalformedRestTable { row: l.to_owned() };
            let f: Vec<&'static str> = l.split('\t').collect();
            let [part, amp, period, citation] = f.as_slice() else {
                return Err(bad());
            };
            let amp: u32 = amp.parse().map_err(|_| bad())?;
            let period: u64 = period.parse().map_err(|_| bad())?;
            if amp == 0 || period == 0 || !period.is_multiple_of(QUANTUM_NS) || citation.is_empty()
            {
                return Err(bad());
            }
            Ok((*part, amp, period, *citation))
        })
        .collect()
}

/// A board-frame direction's component along "up" (−z): +1, −1 or 0.
fn up(t: Toward) -> i8 {
    match t {
        Toward::Up => 1,
        Toward::Down => -1,
        _ => 0,
    }
}

/// The chip's Y, as Z × X, from the declared X and Z: `None` only for axes
/// not at right angles, which `validate` refuses.
fn chip_y(o: &Orientation) -> Option<Toward> {
    let v = |t: Toward| -> [i8; 3] {
        match t {
            Toward::Forward => [1, 0, 0],
            Toward::Aft => [-1, 0, 0],
            Toward::Right => [0, 1, 0],
            Toward::Left => [0, -1, 0],
            Toward::Down => [0, 0, 1],
            Toward::Up => [0, 0, -1],
        }
    };
    let (z, x) = (v(o.z), v(o.x));
    let y = [
        z[1] * x[2] - z[2] * x[1],
        z[2] * x[0] - z[0] * x[2],
        z[0] * x[1] - z[1] * x[0],
    ];
    [
        Toward::Forward,
        Toward::Aft,
        Toward::Right,
        Toward::Left,
        Toward::Down,
        Toward::Up,
    ]
    .into_iter()
    .find(|t| v(*t) == y)
}

/// Each chip axis's reading at rest, in g, as (X, Y, Z).
pub fn accel_at_rest(o: &Orientation) -> Option<(i8, i8, i8)> {
    Some((up(o.x), up(chip_y(o)?), up(o.z)))
}

/// The rest stimuli of every modelled IMU and barometer on the board, from
/// 0 to `window_ns`, in time order. `checks` give each device's Renode path.
pub fn stimuli(
    topology: &SolvedTopology,
    checks: &[StepCheck],
    window_ns: u64,
) -> Result<Vec<Stimulus>, TwinError> {
    let table = parts::table().map_err(TwinError::InvalidTopology)?;
    let rows = rows()?;
    let mut out = Vec::new();
    for check in checks.iter().filter(|c| c.step == StepKind::Data) {
        let part = topology
            .spi_devices
            .get(&check.reference)
            .map(|d| (d.part.as_str(), d.orientation))
            .or_else(|| {
                topology
                    .i2c_devices
                    .get(&check.reference)
                    .map(|d| (d.part.as_str(), None))
            });
        let Some((part, orientation)) = part else {
            continue;
        };
        let Some(p) = table.get(part) else { continue };
        let (Some(renode), Some(script)) = (&p.renode, &p.script) else {
            continue;
        };
        let Some(model) = Model::from_renode(renode) else {
            continue;
        };
        let has = |q: Quantity| script.channels.iter().find(|c| c.quantity == q);
        let at = |at_ns, property, value: String, citation: &str| Stimulus {
            at_ns,
            reference: check.reference.clone(),
            device: check.device.clone(),
            property,
            value,
            citation: citation.to_owned(),
        };
        if has(Quantity::AccelerationX).is_some() || has(Quantity::AccelerationY).is_some() {
            let o = orientation.ok_or_else(|| TwinError::RestOrientationUndeclared {
                reference: check.reference.clone(),
            })?;
            let (x, y, z) =
                accel_at_rest(&o).ok_or_else(|| TwinError::RestOrientationUndeclared {
                    reference: check.reference.clone(),
                })?;
            for (q, g) in [
                (Quantity::AccelerationX, x),
                (Quantity::AccelerationY, y),
                (Quantity::AccelerationZ, z),
            ] {
                let property = model.stimulus(q).ok_or_else(|| TwinError::RestUnmodelled {
                    reference: check.reference.clone(),
                    property: "an acceleration",
                })?;
                out.push(at(0, property, g.to_string(), AT_REST));
            }
        }
        if let Some(c) = has(Quantity::Pressure) {
            let (_, amp, period, citation) =
                rows.iter().find(|r| r.0 == part).ok_or_else(|| {
                    TwinError::RestNoiseUndeclared {
                        part: part.to_owned(),
                    }
                })?;
            let property =
                model
                    .stimulus(Quantity::Pressure)
                    .ok_or_else(|| TwinError::RestUnmodelled {
                        reference: check.reference.clone(),
                        property: "pressure",
                    })?;
            let base = i64::from(c.stimulus) * 1000;
            let mut t = 0u64;
            let mut k = 0u64;
            while t < window_ns {
                let mpa = if k.is_multiple_of(2) {
                    base + i64::from(*amp)
                } else {
                    base - i64::from(*amp)
                };
                let value = format!("{}.{:03}", mpa / 1000, mpa % 1000);
                out.push(at(t, property, value, citation));
                t += period;
                k += 1;
            }
        }
    }
    out.sort_by_key(|s| s.at_ns);
    Ok(out)
}

/// A stimulus value in thousandths: `101325.080` is 101325080, `-1` is
/// -1000. `None` for any other text.
fn milli(v: &str) -> Option<i64> {
    let (neg, v) = v.strip_prefix('-').map_or((false, v), |r| (true, r));
    let (whole, frac) = v.split_once('.').unwrap_or((v, ""));
    if whole.is_empty()
        || frac.len() > 3
        || !(whole.bytes().chain(frac.bytes())).all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let m = whole.parse::<i64>().ok()? * 1000 + format!("{frac:0<3}").parse::<i64>().ok()?;
    Some(if neg { -m } else { m })
}

/// A channel's bytes on the wire for a value in thousandths of its unit:
/// `Channel::raw`'s rule (`round((value - zero) · lsb.0 / lsb.1)`, half away
/// from zero, two's complement in `bytes` bytes), at a thousandth's
/// resolution, which the models of our own round alike (`Raw24(Pressure ·
/// 64)`, `BMP581.cs`).
fn wire_at(c: &parts::Channel, value_milli: i64) -> Vec<u8> {
    let num = (i128::from(value_milli) - i128::from(c.zero) * 1000) * i128::from(c.lsb.0);
    let den = i128::from(c.lsb.1) * 1000;
    let rounded = (2 * num + num.signum() * den) / (2 * den);
    let mask = (1u128 << (8 * u32::from(c.bytes))) - 1;
    let raw = ((rounded as u128) & mask) as u32;
    let mut bytes = raw.to_le_bytes()[..usize::from(c.bytes)].to_vec();
    if c.big_endian {
        bytes.reverse();
    }
    bytes
}

/// The data step's witness for each distinct set of values the rest gives
/// the device over the run (`AET-WO-4400i` D4): the step's exact lines as
/// its model writes them, from the same conversion the witness stimulus's
/// lines come from. Empty for a device the rest does not move.
pub fn data_witnesses(
    topology: &SolvedTopology,
    check: &StepCheck,
    stimuli: &[Stimulus],
) -> Result<Vec<Vec<String>>, TwinError> {
    let mine: Vec<&Stimulus> = stimuli
        .iter()
        .filter(|s| s.device == check.device)
        .collect();
    if check.step != StepKind::Data || mine.is_empty() {
        return Ok(Vec::new());
    }
    let table = parts::table().map_err(TwinError::InvalidTopology)?;
    let part = topology
        .spi_devices
        .get(&check.reference)
        .map(|d| d.part.as_str())
        .or_else(|| {
            topology
                .i2c_devices
                .get(&check.reference)
                .map(|d| d.part.as_str())
        });
    let Some(p) = part.and_then(|part| table.get(part)) else {
        return Ok(Vec::new());
    };
    let (Some(renode), Some(script)) = (&p.renode, &p.script) else {
        return Ok(Vec::new());
    };
    let Some(model) = Model::from_renode(renode) else {
        return Ok(Vec::new());
    };
    let Some(data) = p.steps().into_iter().find(|s| s.kind == StepKind::Data) else {
        return Ok(Vec::new());
    };
    // Each channel's value in thousandths, from its witness stimulus, then
    // as each rest stimulus sets it, in time order.
    let mut now: Vec<i64> = script
        .channels
        .iter()
        .map(|c| i64::from(c.stimulus) * 1000)
        .collect();
    let mut sets: Vec<Vec<i64>> = Vec::new();
    let mut k = 0;
    while k < mine.len() {
        let at = mine[k].at_ns;
        while k < mine.len() && mine[k].at_ns == at {
            let s = mine[k];
            let value = milli(&s.value).ok_or_else(|| TwinError::RestUnmodelled {
                reference: check.reference.clone(),
                property: s.property,
            })?;
            for (i, c) in script.channels.iter().enumerate() {
                if model.stimulus(c.quantity) == Some(s.property) {
                    now[i] = value;
                }
            }
            k += 1;
        }
        if !sets.contains(&now) {
            sets.push(now.clone());
        }
    }
    let mut out = Vec::new();
    for set in sets {
        let wire: Vec<u8> = script
            .channels
            .iter()
            .zip(&set)
            .flat_map(|(c, v)| wire_at(c, *v))
            .collect();
        let step = parts::Step {
            expected: wire.iter().fold(0u64, |v, &b| (v << 8) | u64::from(b)),
            ..data.clone()
        };
        if let Some(lines) = model.witness(&check.device, &step) {
            out.push(lines);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aethelgard_ir::orientation::Toward::*;

    fn o(x: Toward, z: Toward) -> Orientation {
        Orientation { x, z }
    }

    #[test]
    fn an_axis_reads_one_g_where_it_points_up_by_the_declared_orientation() {
        // Sentinel-03's and sentinel-02-own's U5: x right, z up.
        assert_eq!(accel_at_rest(&o(Right, Up)), Some((0, 0, 1)));
        // Mounted upside down: z down reads −1 g.
        assert_eq!(accel_at_rest(&o(Forward, Down)), Some((0, 0, -1)));
        // On its edge, x up: +1 g on x. Y = Z × X.
        assert_eq!(accel_at_rest(&o(Up, Forward)), Some((1, 0, 0)));
        // z forward, x right: Y = forward × right = down, so −1 g on y.
        assert_eq!(accel_at_rest(&o(Right, Forward)), Some((0, -1, 0)));
        // Not at right angles: no rest.
        assert_eq!(accel_at_rest(&o(Up, Down)), None);
    }

    #[test]
    fn a_value_in_thousandths_converts_as_the_parts_table_does() {
        assert_eq!(milli("101325.080"), Some(101_325_080));
        assert_eq!(milli("-1"), Some(-1000));
        assert_eq!(milli("0"), Some(0));
        assert_eq!(milli("1.5"), Some(1500));
        assert_eq!(milli("1.0005"), None);
        assert_eq!(milli("x"), None);
        // The BMP581's pressure channel: 64 counts a pascal, 3 bytes, little
        // endian. 101325 Pa is the parts table's own word; ±0.08 Pa is ±5.12
        // counts, rounded half away from zero to ±5.
        let table = parts::table().unwrap();
        let c = table
            .get("BMP581")
            .unwrap()
            .script
            .as_ref()
            .unwrap()
            .channels[1];
        assert_eq!(wire_at(&c, 101_325_000), c.wire());
        let n = |b: Vec<u8>| u32::from(b[0]) | u32::from(b[1]) << 8 | u32::from(b[2]) << 16;
        assert_eq!(n(wire_at(&c, 101_325_080)), 101_325 * 64 + 5);
        assert_eq!(n(wire_at(&c, 101_324_920)), 101_325 * 64 - 5);
    }

    #[test]
    fn the_noise_row_parses_on_the_quantum() {
        let r = rows().unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!((r[0].0, r[0].1, r[0].2), ("BMP581", 80, 83_300_000));
    }
}
