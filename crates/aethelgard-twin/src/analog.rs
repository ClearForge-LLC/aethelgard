//! The analog model of each I2C line (`AET-WO-2200` §1.2): one netlist per
//! entry of the solved topology's `analog_lines`, for the bridge to measure.
//!
//! The deck is `AET-WO-2101` S2's: the pull-up from the domain's rail to the
//! line, the line's capacitance to ground, and the open-drain driver as a
//! voltage-controlled switch. Its control is an external source the bridge
//! drives with one I2C byte window, 0x7F with NACK, so the line is held low
//! for the first bit and released once, 2.5 µs into a 22.5 µs window. The
//! `.options` and `.tran` cards are S1's, verbatim.
//!
//! Every value is an integer written as text (ohms, femtofarads, millivolts,
//! picoseconds), so two emissions are byte-identical and ngspice reads what
//! the topology says (`INV-01`).

use aethelgard_ir::{AnalogLine, SolvedTopology};

use crate::TwinError;

/// The S2 window: nine bits at 400 kHz.
pub const BIT_PS: u64 = 2_500_000;
pub const BITS: u64 = 9;
pub const WINDOW_PS: u64 = BIT_PS * BITS;
/// Each control edge ramps over 1 ns (S1).
pub const EDGE_PS: u64 = 1_000;
/// The byte S2 measured with: one release edge, after the first bit.
pub const BYTE: u8 = 0x7F;
/// S2's ninth bit is a NACK: the driver stays off.
pub const NACK: bool = true;

/// The external source the bridge drives, and the node it measures.
pub const SOURCE: &str = "vctl";
pub const PROBE: &str = "line";

/// One line's deck and what the bridge measures on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deck {
    pub net: String,
    pub bus: String,
    pub ceiling_ns: u32,
    /// One card per line; the first is the title, the last `.end`.
    pub netlist: Vec<String>,
    /// The control source as (picoseconds, millivolts) points from 0.
    pub stimulus: Vec<(u64, u64)>,
    /// Where the driver starts to release the line.
    pub release_ps: u64,
    /// 30 % and 70 % of the rail.
    pub low_uv: u64,
    pub high_uv: u64,
    pub stop_ps: u64,
}

/// One deck per analog line, in net order.
pub fn decks(topology: &SolvedTopology) -> Result<Vec<Deck>, TwinError> {
    topology.validate().map_err(TwinError::InvalidTopology)?;
    Ok(topology
        .analog_lines
        .iter()
        .map(|(net, line)| deck(net, line))
        .collect())
}

/// The driver's level in each bit: `true` pulls the line low. A 1 bit, and
/// a NACK, release it.
fn driven() -> [bool; BITS as usize] {
    let mut bits = [false; BITS as usize];
    for (k, bit) in bits.iter_mut().take(8).enumerate() {
        *bit = (BYTE >> (7 - k)) & 1 == 0;
    }
    bits[8] = !NACK;
    bits
}

fn deck(net: &str, line: &AnalogLine) -> Deck {
    let mv = u64::from(line.domain_millivolts);
    let level = |on: bool| if on { mv } else { 0 };
    let bits = driven();
    let mut stimulus = vec![(0, level(bits[0]))];
    for k in 1..BITS {
        let (before, after) = (bits[k as usize - 1], bits[k as usize]);
        stimulus.push((k * BIT_PS, level(before)));
        stimulus.push((k * BIT_PS + EDGE_PS, level(after)));
    }
    let release = (1..BITS)
        .find(|&k| bits[k as usize - 1] && !bits[k as usize])
        .map(|k| k * BIT_PS)
        .expect("0x7F releases after its first bit");
    let mut netlist = Vec::new();
    let mut card = |s: String| netlist.push(s);
    card(format!(
        "* aethelgard-twin analog (AET-WO-2200): net {net}, bus {}",
        line.bus
    ));
    card(format!("Vdd vdd 0 dc {}", decimal(mv, 3)));
    card(format!("R1 vdd line {}", line.pull_up_ohms));
    card(format!("C1 line 0 {}f", line.capacitance_femtofarads));
    card("S1 line 0 ctl 0 swm".to_owned());
    // The switch closes above half the control swing, which is the rail.
    card(format!(
        ".model swm sw vt={} vh=0 ron=10 roff=1e12",
        decimal(mv * 500, 6)
    ));
    card(format!("V{} ctl 0 dc 0 external", &SOURCE[1..]));
    card(".options reltol=1e-3 abstol=1e-12 vntol=1e-6 method=trap".to_owned());
    card(format!(".tran 1n {}u 0 5n", decimal(WINDOW_PS, 6)));
    card(".end".to_owned());
    Deck {
        net: net.to_owned(),
        bus: line.bus.clone(),
        ceiling_ns: line.ceiling_ns,
        netlist,
        stimulus,
        release_ps: release,
        low_uv: mv * 300,
        high_uv: mv * 700,
        stop_ps: WINDOW_PS,
    }
}

/// `value / 10^scale` with no trailing zero: 3300 mV is `3.3`, written as
/// ngspice reads it.
fn decimal(value: u64, scale: u32) -> String {
    let unit = 10u64.pow(scale);
    let (whole, frac) = (value / unit, value % unit);
    if frac == 0 {
        return whole.to_string();
    }
    let digits = format!("{frac:0width$}", width = scale as usize);
    format!("{whole}.{}", digits.trim_end_matches('0'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topology(edit: impl FnOnce(&mut AnalogLine)) -> SolvedTopology {
        let mut t: SolvedTopology =
            serde_json::from_str(crate::tests::REFERENCE).expect("the reference topology");
        for line in t.analog_lines.values_mut() {
            line.pull_up_ohms = 2200;
        }
        edit(t.analog_lines.get_mut("I2C_SDA").unwrap());
        t
    }

    #[test]
    fn the_reference_deck_is_s2s() {
        let d = decks(&topology(|_| {})).unwrap();
        assert_eq!(d.len(), 2);
        assert_eq!(
            d[1].netlist,
            [
                "* aethelgard-twin analog (AET-WO-2200): net I2C_SDA, bus SENSORS",
                "Vdd vdd 0 dc 3.3",
                "R1 vdd line 2200",
                "C1 line 0 100000f",
                "S1 line 0 ctl 0 swm",
                ".model swm sw vt=1.65 vh=0 ron=10 roff=1e12",
                "Vctl ctl 0 dc 0 external",
                ".options reltol=1e-3 abstol=1e-12 vntol=1e-6 method=trap",
                ".tran 1n 22.5u 0 5n",
                ".end",
            ]
        );
        // Low for bit 0, released over 1 ns at 2.5 µs, and held off after.
        let mut want = vec![(0, 3300), (2_500_000, 3300), (2_501_000, 0)];
        for k in 2..9 {
            want.extend([(k * BIT_PS, 0), (k * BIT_PS + EDGE_PS, 0)]);
        }
        assert_eq!(d[1].stimulus, want);
        assert_eq!(
            (d[1].release_ps, d[1].low_uv, d[1].high_uv, d[1].stop_ps),
            (2_500_000, 990_000, 2_310_000, 22_500_000)
        );
        assert_eq!((d[1].ceiling_ns, d[1].bus.as_str()), (300, "SENSORS"));
    }

    #[test]
    fn two_emissions_are_byte_identical() {
        let a = decks(&topology(|_| {})).unwrap();
        let b = decks(&topology(|_| {})).unwrap();
        // The netlist cards, byte for byte, and every other field.
        assert_eq!(a, b);
        assert!(a[0].netlist.contains(&"R1 vdd line 2200".to_owned()));
    }

    #[test]
    fn values_reach_the_deck_as_written() {
        let d = decks(&topology(|l| {
            l.pull_up_ohms = 47_000;
            l.capacitance_femtofarads = u64::MAX;
            l.domain_millivolts = 1801;
        }))
        .unwrap();
        let sda = &d[1].netlist;
        assert_eq!(sda[1], "Vdd vdd 0 dc 1.801");
        assert_eq!(sda[2], "R1 vdd line 47000");
        assert_eq!(sda[3], "C1 line 0 18446744073709551615f");
        assert_eq!(sda[5], ".model swm sw vt=0.9005 vh=0 ron=10 roff=1e12");
        assert_eq!((d[1].low_uv, d[1].high_uv), (540_300, 1_260_700));
        // The other line is untouched.
        assert_eq!(d[0].netlist[2], "R1 vdd line 2200");
    }

    /// The fault decision reaches the platform text (`AET-WO-2200` §1.4.4):
    /// a faulted bus declares no device, modelled or not, and names those it
    /// leaves out; nothing else changes, and no decision is today's platform.
    #[test]
    fn a_faulted_bus_declares_no_device_and_names_them() {
        use std::collections::BTreeSet;

        use crate::platform::{emit, emit_faulted, present};

        let t = topology(|_| {});
        let plain = emit(&t).unwrap();
        assert!(plain.contains("\nu2: I2C.BME280 @ i2c1 0x76\n"), "{plain}");
        assert_eq!(emit_faulted(&t, &BTreeSet::new()).unwrap(), plain);
        let sensors = BTreeSet::from(["SENSORS".to_owned()]);
        let faulted = emit_faulted(&t, &sensors).unwrap();
        let note = "// - Analog fault (AET-WO-2200): bus `SENSORS` (I2C1) is faulted by the analog pass,\n\
                    //   so its devices are left out of this platform: U2 (BME280), U3 (TMP117).\n";
        assert!(faulted.contains(note), "{faulted}");
        // 2300 F13: a held-low bus is not "measured" past anything.
        assert!(!faulted.contains("measured"), "{faulted}");
        assert!(!faulted.contains("u2:"), "{faulted}");
        // The rest is the platform with no device at all.
        let mut none = t.clone();
        none.i2c_devices.clear();
        assert_eq!(faulted.replacen(note, "", 1), emit(&none).unwrap());
        assert!(present(&t, &sensors).unwrap().is_empty());
        assert_eq!(
            present(&t, &BTreeSet::new()).unwrap(),
            t.i2c_devices_in_order()
        );
        let err = emit_faulted(&t, &BTreeSet::from(["FLASH".to_owned()])).unwrap_err();
        assert_eq!(err.kind(), "UnknownFaultedBus");
    }

    #[test]
    fn a_malformed_topology_is_refused() {
        let err = decks(&topology(|l| l.pull_up_ohms = 0)).unwrap_err();
        assert_eq!(err.kind(), "InvalidTopology");
    }
}
