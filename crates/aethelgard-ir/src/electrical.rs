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

/// Output low level, in millivolts, for a rail above 2 V (UM10204 table 10,
/// VOL1).
const VOL_MILLIVOLTS: u64 = 400;
/// A rail at or below this reads VOL as 0.2 × VDD (table 10, VOL3).
const LOW_RAIL_MILLIVOLTS: u64 = 2000;

/// The sink current a driver must carry at VOL, in microamperes (table 10:
/// IOL 3 mA for Standard-mode and Fast-mode, 20 mA for Fast-mode Plus).
fn sink_microamps(mode: I2cMode) -> u64 {
    match mode {
        I2cMode::Standard | I2cMode::Fast => 3_000,
        I2cMode::FastPlus => 20_000,
    }
}

/// The least pull-up, in ohms, whose sink current stays within the mode's
/// IOL: `ceil((VDD − VOL) / IOL)` (UM10204 §7.1, equation 1). A rail of 0
/// gives 0; `validate` refuses such a rail before this is asked.
pub fn i2c_pull_up_floor_ohms(speed_hz: u32, domain_millivolts: u32) -> u32 {
    let vdd = u64::from(domain_millivolts);
    let iol = sink_microamps(i2c_mode(speed_hz));
    // mV / µA × 1000 = Ω. Scaled by 5 so 0.2 × VDD stays exact.
    let drop_x5 = if vdd <= LOW_RAIL_MILLIVOLTS {
        4 * vdd
    } else {
        5 * (vdd - VOL_MILLIVOLTS)
    };
    let den = 5 * iol;
    let num = drop_x5 * 1000;
    u32::try_from(num.div_ceil(den)).expect("a u32 rail over at least 3 mA fits a u32")
}

/// The bus capacitance limit, in femtofarads (UM10204 table 10: Cb 400 pF
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
    fn modes_at_each_boundary() {
        assert_eq!(i2c_mode(100_000), I2cMode::Standard);
        assert_eq!(i2c_mode(100_001), I2cMode::Fast);
        assert_eq!(i2c_mode(400_000), I2cMode::Fast);
        assert_eq!(i2c_mode(400_001), I2cMode::FastPlus);
        assert_eq!(i2c_mode(1_000_000), I2cMode::FastPlus);
    }

    #[test]
    fn the_floor_for_each_mode_at_1v8_and_3v3() {
        // (speed, rail mV, floor Ω)
        let table = [
            (100_000, 1800, 480), // 0.2 × 1.8 V = 360 mV; 1440 mV / 3 mA
            (100_000, 3300, 967), // 2900 mV / 3 mA = 966.67
            (400_000, 1800, 480),
            (400_000, 3300, 967),
            (1_000_000, 1800, 72),  // 1440 mV / 20 mA
            (1_000_000, 3300, 145), // 2900 mV / 20 mA
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
    fn the_floor_at_the_2v_boundary_and_the_extremes() {
        // 2.0 V uses 0.2 × VDD: 1600 mV / 3 mA = 533.33; 2.001 V uses 400 mV.
        assert_eq!(i2c_pull_up_floor_ohms(400_000, 2000), 534);
        assert_eq!(i2c_pull_up_floor_ohms(400_000, 2001), 534);
        assert_eq!(i2c_pull_up_floor_ohms(400_000, 0), 0);
        assert_eq!(i2c_pull_up_floor_ohms(400_000, 400), 107);
        assert_eq!(i2c_pull_up_floor_ohms(400_000, u32::MAX), 1_431_655_632);
    }

    #[test]
    fn the_capacitance_limit_per_mode() {
        assert_eq!(i2c_bus_capacitance_limit_femtofarads(100_000), 400_000);
        assert_eq!(i2c_bus_capacitance_limit_femtofarads(400_000), 400_000);
        assert_eq!(i2c_bus_capacitance_limit_femtofarads(1_000_000), 550_000);
    }
}
