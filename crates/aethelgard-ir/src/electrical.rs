//! Static I2C electrical rules (`AET-WO-1203`), from NXP UM10204 rev. 7
//! ("I2C-bus specification and user manual", 1 October 2021).
//!
//! `solve` refuses a specification that breaks either rule, and the proposal
//! loop reads its floor from here, so the floor and the gate are one fact
//! (`RUL-18`, `INV-07`). Integer arithmetic only.

/// An I2C speed class (UM10204 table 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I2cMode {
    /// Standard-mode, up to 100 kHz.
    Standard,
    /// Fast-mode, up to 400 kHz.
    Fast,
    /// Fast-mode Plus, up to 1 MHz.
    FastPlus,
}

/// The mode whose clock limit is at least `speed_hz`. A speed above 1 MHz is
/// refused earlier (`UnsupportedI2cSpeed`); here it reads as Fast-mode Plus.
pub fn i2c_mode(speed_hz: u32) -> I2cMode {
    match speed_hz {
        0..=100_000 => I2cMode::Standard,
        100_001..=400_000 => I2cMode::Fast,
        _ => I2cMode::FastPlus,
    }
}

/// The fastest I2C mode whose maximum rise time (NXP UM10204) is at least
/// `rise_time_ns`, as its speed in Hz; `None` when it is slower than every mode.
/// `solve` derives a bus's ceiling with it and `SolvedTopology::validate`
/// re-derives the same (`AET-WO-1204`), so the two share this one function.
pub fn ceiling_hz(rise_time_ns: u32) -> Option<u32> {
    match rise_time_ns {
        0..=120 => Some(1_000_000),
        121..=300 => Some(400_000),
        301..=1000 => Some(100_000),
        _ => None,
    }
}

/// Output low level, in millivolts, for a rail above 2 V (UM10204 rev. 7
/// table 10, VOL1: at most 0.4 V at 3 mA sink current, VDD > 2 V).
const VOL_MILLIVOLTS: u64 = 400;
/// A rail at or below this reads VOL as 0.2 × VDD (table 10, VOL2: at most
/// 0.2 × VDD at 2 mA sink current, VDD ≤ 2 V).
const LOW_RAIL_MILLIVOLTS: u64 = 2000;
/// The sink current at VOL2, in microamperes. Table 10 gives it for Fast-mode
/// and Fast-mode Plus; the Standard-mode column is unspecified ("-"), so
/// Standard-mode takes the Fast-mode row (ruling on `AET-WO-1203` F1).
const LOW_RAIL_SINK_MICROAMPS: u64 = 2_000;

/// A rail at or above this has DS12117's 8 mA `VOL` row (2.7 V ≤ VDD ≤ 3.6 V).
const FM_PLUS_8MA_RAIL_MILLIVOLTS: u64 = 2700;

/// The sink current a driver must carry at VOL1, in microamperes, for a rail
/// above 2 V.
///
/// Standard-mode and Fast-mode take UM10204 rev. 7 table 10's 3 mA. Fast-mode
/// Plus does not take table 10's 20 mA: this MCU's I/O does not give it
/// (`AET-WO-4230` D8, review A8). DS12117 Rev 10 §6.3.32 (p. 188, rev Y)
/// states "The 20 mA output drive requirement in Fast-mode Plus is not
/// supported", and ST's open pin data carries no pad type, so the `FTf` row of
/// Table 50 (`VOLFM+`, p. 139) is not claimed. Table 50 (rev Y, p. 139, read
/// from the rendered page) holds `VOL` at most 0.4 V at 8 mA for 2.7 V ≤ VDD
/// ≤ 3.6 V and at 4 mA for 1.62 V ≤ VDD ≤ 3.6 V, so Fast-mode Plus takes 8 mA
/// at a rail of 2.7 V or more and 4 mA below it. The same text is in the
/// rev V section of DS12117 (pp. 243 and 298 by text search, not rendered).
fn sink_microamps(mode: I2cMode, rail_millivolts: u64) -> u64 {
    match mode {
        I2cMode::Standard | I2cMode::Fast => 3_000,
        I2cMode::FastPlus if rail_millivolts >= FM_PLUS_8MA_RAIL_MILLIVOLTS => 8_000,
        I2cMode::FastPlus => 4_000,
    }
}

/// The least pull-up, in ohms, whose sink current stays within the driver's
/// rating: `ceil((VDD − VOL) / IOL)` (UM10204 §7.1, equation 2).
///
/// `domain_millivolts` is the domain's **maximum** voltage
/// ([`crate::maximum_millivolts`]): the sink current is greatest at the
/// highest rail, so the floor is taken there, never at the nominal
/// (`AET-WO-4230` §1.5, review A8).
///
/// - VDD above 2 V: VOL is 0.4 V and IOL is 3 mA (Standard-mode, Fast-mode)
///   or 8 mA (Fast-mode Plus at 2.7 V or more; 4 mA below it).
/// - VDD at most 2 V: VOL is 0.2 × VDD and IOL is 2 mA, in every mode.
///
/// A rail of 0 gives 0; `validate` refuses such a rail before this is asked.
pub fn i2c_pull_up_floor_ohms(speed_hz: u32, domain_millivolts: u32) -> u32 {
    let vdd = u64::from(domain_millivolts);
    // mV / µA × 1000 = Ω. Scaled by 5 so 0.2 × VDD stays exact.
    let (drop_x5, iol) = if vdd <= LOW_RAIL_MILLIVOLTS {
        (4 * vdd, LOW_RAIL_SINK_MICROAMPS)
    } else {
        (
            5 * (vdd - VOL_MILLIVOLTS),
            sink_microamps(i2c_mode(speed_hz), vdd),
        )
    };
    let num = drop_x5 * 1000;
    let den = 5 * iol;
    u32::try_from(num.div_ceil(den)).expect("a u32 rail over at least 2 mA fits a u32")
}

/// UM10204 rev. 7.0 (1 October 2021), Table 10 "Characteristics of the SDA
/// and SCL I/O stages", page 43: `Ci`, "capacitance for each I/O pin",
/// maximum 10 pF in Standard-mode, Fast-mode and Fast-mode Plus. Its note
/// [10]: "Special purpose devices such as multiplexers and switches may
/// exceed this capacitance because they connect multiple paths together."
/// `extract` counts every undeclared I2C pin at it, and `validate` refuses a
/// declaration below it on an I2C device (`AET-WO-4220b`).
pub const I2C_PIN_CI_FEMTOFARADS: u64 = 10_000;

/// The bus capacitance limit, in femtofarads (UM10204 table 11: Cb 400 pF
/// for Standard-mode and Fast-mode, 550 pF for Fast-mode Plus).
pub fn i2c_bus_capacitance_limit_femtofarads(speed_hz: u32) -> u64 {
    match i2c_mode(speed_hz) {
        I2cMode::Standard | I2cMode::Fast => 400_000,
        I2cMode::FastPlus => 550_000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ceiling_mapping_at_each_boundary() {
        assert_eq!(ceiling_hz(1), Some(1_000_000));
        assert_eq!(ceiling_hz(120), Some(1_000_000));
        assert_eq!(ceiling_hz(121), Some(400_000));
        assert_eq!(ceiling_hz(300), Some(400_000));
        assert_eq!(ceiling_hz(301), Some(100_000));
        assert_eq!(ceiling_hz(1000), Some(100_000));
        assert_eq!(ceiling_hz(1001), None);
        assert_eq!(ceiling_hz(u32::MAX), None);
    }

    #[test]
    fn modes_at_each_boundary() {
        assert_eq!(i2c_mode(100_000), I2cMode::Standard);
        assert_eq!(i2c_mode(100_001), I2cMode::Fast);
        assert_eq!(i2c_mode(400_000), I2cMode::Fast);
        assert_eq!(i2c_mode(400_001), I2cMode::FastPlus);
        assert_eq!(i2c_mode(1_000_000), I2cMode::FastPlus);
    }

    #[test]
    fn the_floor_for_each_mode_at_1v8_2v_and_3v3() {
        // (speed, rail mV, floor Ω)
        let table = [
            // At or below 2 V: 0.2 × VDD at 2 mA, in every mode.
            (100_000, 1800, 720), // 1440 mV / 2 mA
            (400_000, 1800, 720),
            (1_000_000, 1800, 720),
            (100_000, 2000, 800), // 1600 mV / 2 mA
            (400_000, 2000, 800),
            (1_000_000, 2000, 800),
            // Above 2 V: 0.4 V at 3 / 3 mA, 8 mA (4 mA below 2.7 V) in Fm+.
            (100_000, 2001, 534), // 1601 mV / 3 mA = 533.67
            (400_000, 2001, 534),
            (1_000_000, 2001, 401), // 1601 mV / 4 mA = 400.25 (below 2.7 V)
            (1_000_000, 2699, 575), // 2299 mV / 4 mA = 574.75
            (1_000_000, 2700, 288), // 2300 mV / 8 mA = 287.5
            (100_000, 3300, 967),   // 2900 mV / 3 mA = 966.67
            (400_000, 3300, 967),
            (1_000_000, 3300, 363), // 2900 mV / 8 mA = 362.5
            // The domain's maximum, 3.3 V + 5 % = 3465 mV (review A8).
            (100_000, 3465, 1022), // 3065 mV / 3 mA = 1021.67, not 967
            (400_000, 3465, 1022),
            (1_000_000, 3465, 384), // 3065 mV / 8 mA = 383.1
        ];
        for (speed, mv, want) in table {
            assert_eq!(
                i2c_pull_up_floor_ohms(speed, mv),
                want,
                "{speed} Hz {mv} mV"
            );
        }
    }

    #[test]
    fn the_floor_at_the_extremes() {
        assert_eq!(i2c_pull_up_floor_ohms(400_000, 0), 0);
        assert_eq!(i2c_pull_up_floor_ohms(400_000, 400), 160); // 320 mV / 2 mA
        assert_eq!(i2c_pull_up_floor_ohms(400_000, u32::MAX), 1_431_655_632);
    }

    #[test]
    fn the_capacitance_limit_per_mode() {
        assert_eq!(i2c_bus_capacitance_limit_femtofarads(100_000), 400_000);
        assert_eq!(i2c_bus_capacitance_limit_femtofarads(400_000), 400_000);
        assert_eq!(i2c_bus_capacitance_limit_femtofarads(1_000_000), 550_000);
    }
}
