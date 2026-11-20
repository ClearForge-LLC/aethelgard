//! The clock source's deviation, as one ppm budget per side (`AET-WO-4240a`
//! §1.4, `4240` D3, ruled on #240).
//!
//! **One budget.** Every timing check that judges a clock at its extremes
//! takes the fastest and the slowest a source runs from here: the serial
//! receiver's tolerance, DShot's and PWM's bands, the I2C `TIMINGR` rule and
//! the voltage-scaling rule. Before `4240a` the IR held three copies of the
//! HSI's figures (`motor`, `silicon`, `solve`); D3 replaced them with this one.
//! The generated firmware keeps its own copy in `aethelgard-svd-hal`
//! (`RUL-20`): the generator never reads this module.
//!
//! **Rounding stays at the call site.** A budget is ppm per side; each check
//! keeps its own rounding of `nominal × (10⁶ ± ppm) / 10⁶`, and
//! [`Budget::fastest_hz`] and [`Budget::slowest_hz`] are the ceiling and the
//! floor the silicon rule and the solver used before.

/// A source's deviation from its nominal frequency at its fastest and at its
/// slowest, in ppm, each the sum of its cited terms rounded away from zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    pub fast_ppm: u32,
    pub slow_ppm: u32,
}

impl Budget {
    /// The HSI (`AET-WO-4230` contract, amendment 1), from DS12117 Rev 10
    /// Table 36 (rev Y, p. 128) = Table 133 (rev V, p. 232): `fHSI` 63.7 to
    /// 64.3 MHz at 3.3 V and 30 °C, `ΔVDD` −0.12 % to +0.03 %, and `ΔTEMP`
    /// −2 % to +1 % over TJ −40 °C to TJmax (no declaration bounds the
    /// junction temperature, so the wider row is taken). The two figures are
    /// the ones every HSI verdict has used since `4230`; summing the terms
    /// read today gives +14,987.5 and −25,887.5, not these (`4240v` F5, a
    /// backlog row for the next silicon-facts work order). `4240a` moves no
    /// verdict, so it keeps them.
    pub const HSI: Budget = Budget {
        fast_ppm: 14_984,
        slow_ppm: 25_894,
    };

    /// `nominal_hz` at the fastest, rounded up.
    pub fn fastest_hz(self, nominal_hz: u64) -> u64 {
        let n = u128::from(nominal_hz) * u128::from(1_000_000 + u64::from(self.fast_ppm));
        u64::try_from(n.div_ceil(1_000_000)).unwrap_or(u64::MAX)
    }

    /// `nominal_hz` at the slowest, rounded down; 0 at a deviation of 100 %
    /// or more.
    pub fn slowest_hz(self, nominal_hz: u64) -> u64 {
        let n = u128::from(nominal_hz)
            * u128::from(1_000_000u64.saturating_sub(u64::from(self.slow_ppm)));
        u64::try_from(n / 1_000_000).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The HSI's figures, written by hand: a budget that moves either side
    /// fails here (`4240v` F5 says why they are not the plain sum).
    #[test]
    fn the_hsi_budget_is_the_figures_every_verdict_used() {
        assert_eq!(Budget::HSI.fast_ppm, 14_984);
        assert_eq!(Budget::HSI.slow_ppm, 25_894);
        assert_eq!(Budget::HSI.fastest_hz(64_000_000), 64_958_976);
        assert_eq!(Budget::HSI.slowest_hz(64_000_000), 62_342_784);
        assert_eq!(Budget::HSI.fastest_hz(1), 2);
        assert_eq!(Budget::HSI.slowest_hz(1), 0);
    }
}
