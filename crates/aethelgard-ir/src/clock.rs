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
//!
//! **The HSE** (`AET-WO-4240a`): [`Hse`] is `mcu.hse`, a crystal (a row of the
//! power parts table, cited per figure) or an external clock (a rational
//! frequency and a cited deviation). [`declared_budget`] refuses a
//! declaration that would undercount and sums the budget; [`judge_tree`]
//! judges a solved tree's PLL words. The judge shares no code with the
//! solver's choice of words (`solve.rs`, `RUL-20` (1)): it decodes them with
//! the shared register decoder, [`SolvedClock::from_rcc`], and reads
//! `RCC_PLLCFGR` itself (`4240` D8).

use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::motor::{RccRegisters, SolvedClock};
use crate::power_parts::{self, Kind};
use crate::{IrValidationError, vos};

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

// ---------------------------------------------------------------- the declaration

/// `mcu.hse` (`AET-WO-4240a` §1.2, `4240` D4 as ruled on #240): the board's
/// high-speed external clock. `null` in the specification is none, and the
/// generated firmware runs on the HSI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hse {
    /// A quartz crystal across `OSC_IN`/`OSC_OUT` (`RCC_CR.HSEBYP` 0).
    Crystal(HseCrystal),
    /// A clock driven into `OSC_IN` (`HSEBYP` 1; RM0433 Rev 9 §8.5.2,
    /// p. 340: `OSC_OUT` is free as a GPIO).
    External(HseExternal),
}

/// A crystal: its part, and what its budget needs that no held datasheet
/// states.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct HseCrystal {
    /// A `crystal` row of the power parts table.
    pub part: String,
    /// `CL1` and `CL2`, fF, declared by value (`4240a` §1.3 (a)): the
    /// capacitors' part is `4240c`'s, once `extract` bounds the stray
    /// capacitance. Judged equal and, in series, at most the row's CL
    /// ([`crystal_budget`]).
    pub load_caps_femtofarads: [u64; 2],
    /// The pulling from the load's departure from the crystal's `CL`, ppm
    /// per side: declared, because no held datasheet states pullability
    /// (`4240` §3.2). At least 1.
    pub pull_ppm: u32,
    /// The board's operating temperature, °C: it must lie inside the row's,
    /// over which the row's stability holds.
    pub temperature_celsius: CelsiusRange,
    /// The service life, hours: at most the span the row's aging is stated
    /// for (`4300d` D-B4's 8,000 h is the precedent).
    pub life_hours: u32,
    /// The bring-up's `HSERDY` bound, µs ([`Hse::startup_max_us`]).
    pub startup_max_us: u32,
    /// Where `startup_max_us` comes from ([`Hse::startup_max_us`]).
    pub startup_source: String,
}

/// An external clock into `OSC_IN`: its frequency as an exact fraction
/// (`hz_num` / `hz_den`, `4240` F-HZ), and its deviation, used alone, with
/// where it was read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct HseExternal {
    pub hz_num: u64,
    pub hz_den: u64,
    /// ppm per side, at least 1.
    pub ppm: u32,
    /// Where `ppm` was read: 1–200 bytes of printable ASCII, cited with a
    /// document number and its revision (F-CITE strict). `validate` checks the
    /// text's form only, as it does `core_supply.source`; whether it names a
    /// document is the reviewer's to judge (adversarial A8).
    pub source: String,
    /// The bring-up's `HSERDY` bound, µs ([`Hse::startup_max_us`]).
    pub startup_max_us: u32,
    /// Where `startup_max_us` comes from ([`Hse::startup_max_us`]).
    pub startup_source: String,
}

/// The HSE's start-up bound (`AET-WO-4240b` §1.1, Q1 ruled 2026-11-20): how
/// long the generated bring-up waits for `RCC_CR.HSERDY` before it halts by
/// name (`HSE_NOT_READY`). Declared on both modes, never defaulted: DS12117
/// Rev 10 Table 33 gives `tSU(HSE)` as 2 ms **typical** with no maximum, and
/// "it can vary significantly with the crystal manufacturer" (Note 4), and no
/// held crystal or oscillator sheet states one. So the figure is declared
/// with its source (F-CITE strict in form, as `external.source`; whether it
/// names a document is the reviewer's to judge). A 0 is refused
/// (`HseDeclarationZero`). Too short a bound halts a good board; too long
/// only delays the halt of a dead one.
impl Hse {
    pub fn startup_max_us(&self) -> u32 {
        match self {
            Self::Crystal(c) => c.startup_max_us,
            Self::External(e) => e.startup_max_us,
        }
    }
    pub fn startup_source(&self) -> &str {
        match self {
            Self::Crystal(c) => &c.startup_source,
            Self::External(e) => &e.startup_source,
        }
    }
}

/// A temperature range, °C, both ends included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct CelsiusRange {
    pub min: i32,
    pub max: i32,
}

crate::object_only!(
    HseCrystal,
    HseExternal,
    CelsiusRange,
    SolvedHse,
    SolvedKernelSwitch,
    SolvedAdcClock
);

/// PLL1 started from the HSI for the kernel-clock switch alone
/// (`AET-WO-4240e` D1 (a), ruled): a kernel mux moves to a new input only
/// while its current input and the new one both run, and gives no clock
/// otherwise (RM0433 Rev 9 §8.5.10, pp. 371–372). `SPI123SEL` resets to
/// `pll1_q_ck` (p. 414), so on a board with no HSE tree the bring-up runs
/// PLL1 from the HSI across the select writes: its three words written with
/// PLL1 off, `PLL1ON`, `PLL1RDY` read 1, the selects, then `PLL1ON` cleared.
/// No other clock is taken from PLL1, so it stops when the selects are done.
/// Absent when no select leaves a reset input that is a PLL1 output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedKernelSwitch {
    /// `RCC_PLLCKSELR` (§8.7.11, offset 0x028): `PLLSRC` `00` (`hsi_ck`).
    pub pllckselr: u32,
    /// `RCC_PLLCFGR` (§8.7.12, offset 0x02C): `DIVQ1EN` set.
    pub pllcfgr: u32,
    /// `RCC_PLL1DIVR` (§8.7.13, offset 0x030).
    pub pll1divr: u32,
}

/// The clock the ADCs convert on (`AET-WO-4240e`, the architect's ruling on
/// the ADC): `ADCx_CCR.CKMODE`, written before the ADC is enabled. `00`
/// takes `adc_ker_ck_input` through `RCC_D3CCIPR.ADCSEL`; any other code
/// takes `adc_sclk` divided by 1, 2 or 4, and not the kernel clock (RM0433
/// Rev 9 §25.4.3, p. 920; `CKMODE` p. 1039). Absent when no ADC is enabled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedAdcClock {
    pub ckmode: u32,
}

/// The HSE tree as solved (`AET-WO-4240a`): the declaration, copied by
/// `solve` (`RUL-16`), the budget it gives, and the three PLL words the
/// solver chose. [`judge_tree`] re-derives the budget and judges the words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedHse {
    pub declared: Hse,
    pub fast_ppm: u32,
    pub slow_ppm: u32,
    /// `RCC_PLLCKSELR` (§8.7.11, offset 0x028).
    pub pllckselr: u32,
    /// `RCC_PLLCFGR` (§8.7.12, offset 0x02C).
    pub pllcfgr: u32,
    /// `RCC_PLL1DIVR` (§8.7.13, offset 0x030).
    pub pll1divr: u32,
    /// `FLASH_ACR` as the bring-up writes it before the switch (`AET-WO-4240b`
    /// Q6, ruled): `LATENCY` and `WRHIGHFREQ` of RM0433 Rev 9 Table 17's least
    /// VOS Scale 2 row that holds at `hclk`'s fastest, from the solver's own
    /// copy of the rows; `aethelgard_ir::vos::flash_at_reset` judges it.
    pub flash_acr: u32,
}

impl SolvedHse {
    pub fn budget(&self) -> Budget {
        Budget {
            fast_ppm: self.fast_ppm,
            slow_ppm: self.slow_ppm,
        }
    }
}

// ---------------------------------------------------------------- refusals

/// Why an HSE declaration or its solved tree is refused (`AET-WO-4240a`).
/// Each is its own kind ([`HseRefusal::kind`]).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HseRefusal {
    #[error("mcu.hse.part `{part}` is not a crystal row of the power parts table")]
    HsePartNotCrystal { part: String },
    #[error("mcu.hse.{field} is 0: a declared zero is no declaration (`INV-06`)")]
    HseDeclarationZero { field: &'static str },
    #[error("the HSE is {hz_num}/{hz_den} Hz, outside {min_hz} to {max_hz} Hz ({cite})")]
    HseFrequencyOutOfRange {
        hz_num: u64,
        hz_den: u64,
        min_hz: u64,
        max_hz: u64,
        cite: &'static str,
    },
    #[error(
        "mcu.hse.temperature_celsius {min} to {max} °C does not lie inside `{part}`'s {part_min} to {part_max} °C, over which its stability is stated"
    )]
    HseTemperatureOutsidePart {
        part: String,
        min: i32,
        max: i32,
        part_min: i64,
        part_max: i64,
    },
    #[error(
        "mcu.hse.life_hours is {life_hours}, beyond the {aging_hours} h `{part}`'s aging is stated for: no cited term covers the rest"
    )]
    HseLifeBeyondAging {
        part: String,
        life_hours: u32,
        aging_hours: i64,
    },
    #[error(
        "`{part}`'s critical gain is {gm_crit_nano_amps_per_volt} nA/V at its ESR and C0 maxima, at or over DS12117 Rev 10's Gm_crit_max of {max_nano_amps_per_volt} nA/V (Table 33 rev Y p. 125 = Table 130 rev V p. 229), which AN2867 Rev 25 §3.4 p. 13 requires it be under (the formula is p. 13's): the oscillator may not start"
    )]
    HseGmCritExceeded {
        part: String,
        gm_crit_nano_amps_per_volt: u64,
        max_nano_amps_per_volt: u64,
    },
    #[error(
        "the HSE's budget is +{fast_ppm}/-{slow_ppm} ppm: a side of 1,000,000 ppm or more is no clock"
    )]
    HseBudgetUnbounded { fast_ppm: u64, slow_ppm: u64 },
    #[error(
        "mcu.hse.load_caps_femtofarads {caps:?}: {reason} (AN2867 Rev 25 §3.3 p. 12 gives CL = CL1·CL2/(CL1 + CL2) + Cs, and §3.4 p. 13 assumes CL1 = CL2 and the crystal seeing its CL)"
    )]
    HseLoadCapsUnfit {
        caps: [u64; 2],
        reason: &'static str,
    },
    #[error(
        "no PLL1 setting runs sys_ck at exactly 64 MHz from {hz_num}/{hz_den} Hz with a whole-Hz reference strictly inside a PLL1RGE range (`4240` H1, F-RGE)"
    )]
    HseTreeUnreachable { hz_num: u64, hz_den: u64 },
    #[error(
        "the PLL1 reference, {num}/{den} Hz, is not strictly inside PLL1RGE's {min_hz} to {max_hz} Hz (RM0433 Rev 9 §8.7.12, p. 402: the ranges share their ends; `4240` F-RGE)"
    )]
    PllReferenceNotInsideRange {
        num: u64,
        den: u64,
        min_hz: u64,
        max_hz: u64,
    },
    #[error(
        "PLL1VCOSEL {vcosel} takes no reference in PLL1RGE {rge:02b}: the medium VCO (1) is for a 1 to 2 MHz reference, the wide one (0) for 2 to 16 MHz (RM0433 Rev 9 p. 347, p. 402)"
    )]
    PllVcoSelectMismatch { rge: u32, vcosel: u32 },
    #[error(
        "the VCO, {num}/{den} Hz, is outside PLL1VCOSEL {vcosel}'s {min_hz} to {max_hz} Hz ({cite})"
    )]
    PllVcoOutOfRange {
        num: u64,
        den: u64,
        vcosel: u32,
        min_hz: u64,
        max_hz: u64,
        cite: &'static str,
    },
    #[error(
        "RCC_PLLCFGR.{field} is 0: sys_ck is pll1_p_ck, which it disables (RM0433 Rev 9 p. 401)"
    )]
    PllOutputDisabled { field: &'static str },
    #[error("the solved PLL words do not decode: {reason}")]
    PllWordsUndecodable { reason: String },
    #[error("the solved HSE clock disagrees with its own declaration or words: {reason}")]
    HseClockMismatch { reason: String },
    /// `AET-WO-4240a` F3, built in `4240b`: a topology's `clock` is not what
    /// the solver gives for the HSE declaration it carries.
    #[error(
        "the topology's clock is not the one its own HSE declaration solves to ({field} differs): a path that reads only a topology re-solves the copy and refuses an edited field"
    )]
    HseCopyNotSolved { field: &'static str },
}

impl HseRefusal {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::HsePartNotCrystal { .. } => "HsePartNotCrystal",
            Self::HseDeclarationZero { .. } => "HseDeclarationZero",
            Self::HseFrequencyOutOfRange { .. } => "HseFrequencyOutOfRange",
            Self::HseTemperatureOutsidePart { .. } => "HseTemperatureOutsidePart",
            Self::HseLifeBeyondAging { .. } => "HseLifeBeyondAging",
            Self::HseGmCritExceeded { .. } => "HseGmCritExceeded",
            Self::HseTreeUnreachable { .. } => "HseTreeUnreachable",
            Self::HseBudgetUnbounded { .. } => "HseBudgetUnbounded",
            Self::HseLoadCapsUnfit { .. } => "HseLoadCapsUnfit",
            Self::PllReferenceNotInsideRange { .. } => "PllReferenceNotInsideRange",
            Self::PllVcoSelectMismatch { .. } => "PllVcoSelectMismatch",
            Self::PllVcoOutOfRange { .. } => "PllVcoOutOfRange",
            Self::PllOutputDisabled { .. } => "PllOutputDisabled",
            Self::PllWordsUndecodable { .. } => "PllWordsUndecodable",
            Self::HseClockMismatch { .. } => "HseClockMismatch",
            Self::HseCopyNotSolved { .. } => "HseCopyNotSolved",
        }
    }
}

fn refuse(r: HseRefusal) -> IrValidationError {
    IrValidationError::Hse(r)
}

// ---------------------------------------------------------------- the budget

/// DS12117 Rev 10 Table 33 (rev Y, p. 125) = Table 130 (rev V, p. 229): the
/// HSE oscillator's frequency, 4 to 48 MHz.
const CRYSTAL_HZ: (u64, u64, &str) = (
    4_000_000,
    48_000_000,
    "DS12117 Rev 10 Table 33 rev Y p. 125 = Table 130 rev V p. 229",
);
/// DS12117 Rev 10 Table 31 (rev Y, p. 123) = Table 128 (rev V, p. 227): the
/// external user clock, 4 to 50 MHz.
const EXTERNAL_HZ: (u64, u64, &str) = (
    4_000_000,
    50_000_000,
    "DS12117 Rev 10 Table 31 rev Y p. 123 = Table 128 rev V p. 227",
);
/// `Gm_crit_max`, the oscillator's maximum critical crystal gm at startup:
/// 1.5 mA/V (DS12117 Rev 10 Table 33 rev Y p. 125 = Table 130 rev V p. 229).
pub const GM_CRIT_MAX_NANO_AMPS_PER_VOLT: u64 = 1_500_000;

/// The crystal row `part` names.
pub fn crystal_row(part: &str) -> Result<&'static power_parts::Crystal, IrValidationError> {
    let table = power_parts::table()?;
    match table.get(part).map(|r| &r.kind) {
        Some(Kind::Crystal(c)) => Ok(c),
        _ => Err(refuse(HseRefusal::HsePartNotCrystal {
            part: part.to_owned(),
        })),
    }
}

/// The source's frequency as `(numerator, denominator)` in Hz: a crystal's
/// row, or an external clock's declaration.
pub fn source_hz(hse: &Hse) -> Result<(u64, u64), IrValidationError> {
    match hse {
        Hse::Crystal(c) => {
            let row = crystal_row(&c.part)?;
            Ok((u64::try_from(row.frequency_hz.value).unwrap_or(0), 1))
        }
        Hse::External(e) => Ok((e.hz_num, e.hz_den)),
    }
}

/// `Gm_crit` (`4240` §3): what the oscillator must supply for the crystal to
/// start, against DS12117's `Gm_crit_max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GmCrit {
    /// `4 · ESR · (2πF)² · (C0 + CL)²` at the row's ESR and `C0` maxima, with
    /// π² rounded up ((355/113)² > π²), in nA/V, rounded up: an upper bound.
    pub nano_amps_per_volt: u64,
    /// Under [`GM_CRIT_MAX_NANO_AMPS_PER_VOLT`], strictly: AN2867 Rev 25 §3.4
    /// p. 13 asks Gm_crit_max "greater than" gmcrit, and §4.2 p. 26 gmcrit
    /// "smaller than" it (`AET-WO-4300k`).
    pub holds: bool,
    /// `Gm_crit_max` / `Gm_crit`, in hundredths, rounded down, judging
    /// nothing. It is **not** AN2867's gain margin: Rev 25 §3.4 p. 14 puts
    /// the factor 5 inside `Gm_crit_max` (= gm / 5), so its gain margin,
    /// gm / gmcrit, is five times this, and with `Gm_crit_max` stated its
    /// criterion is `holds` alone (`AET-WO-4300k` M3).
    pub an2867_margin_hundredths_uncited: u64,
}

/// `Gm_crit` for `row`. The formula is AN2867 Rev 25 §3.4 (p. 13); DS12117
/// states the threshold only.
pub fn gm_crit(row: &power_parts::Crystal) -> GmCrit {
    let pos = |v: i64| u128::try_from(v).unwrap_or(0);
    let (esr_mohm, f) = (
        pos(row.esr_max_milliohms.value),
        pos(row.frequency_hz.value),
    );
    let c_ff = pos(row.c0_max_femtofarads.value) + pos(row.load_femtofarads.value);
    // gm [A/V] = 16 π² · ESR · F² · C² = 16 π² · ESR_mΩ · F² · C_fF² / 10³³,
    // so in nA/V it is 16 π² · ESR_mΩ · F² · C_fF² / 10²⁴, with π² ≤ (355/113)².
    let num = [16u128, 355 * 355, esr_mohm, f, f, c_ff, c_ff]
        .iter()
        .try_fold(1u128, |a, b| a.checked_mul(*b));
    let den = 113u128 * 113 * 10u128.pow(24);
    let nano = num.map_or(u64::MAX, |n| {
        u64::try_from(n.div_ceil(den)).unwrap_or(u64::MAX)
    });
    GmCrit {
        nano_amps_per_volt: nano,
        holds: nano < GM_CRIT_MAX_NANO_AMPS_PER_VOLT,
        an2867_margin_hundredths_uncited: (GM_CRIT_MAX_NANO_AMPS_PER_VOLT * 100)
            .checked_div(nano)
            .unwrap_or(0),
    }
}

/// The budget `hse` declares, each refusal by name (`AET-WO-4240a` §1.4):
/// - a crystal: the row's tolerance at +25 °C, its stability over its own
///   temperature range (the board's must lie inside it), its aging over the
///   span the row states (the service life must not exceed it), and the
///   declared pull, per side; its frequency inside DS12117's crystal range
///   and its `Gm_crit` under `Gm_crit_max`;
/// - an external clock: the declared ppm alone, its frequency inside
///   DS12117's external-clock range.
pub fn declared_budget(hse: &Hse) -> Result<Budget, IrValidationError> {
    if hse.startup_max_us() == 0 {
        return Err(refuse(HseRefusal::HseDeclarationZero {
            field: "startup_max_us",
        }));
    }
    crate::board::source(hse.startup_source(), "mcu.hse.startup_source")?;
    match hse {
        Hse::Crystal(c) => {
            let table = power_parts::table()?;
            let row = table.get(&c.part).ok_or_else(|| {
                refuse(HseRefusal::HsePartNotCrystal {
                    part: c.part.clone(),
                })
            })?;
            crystal_budget(c, row)
        }
        Hse::External(e) => {
            for (field, v) in [
                ("hz_num", e.hz_num),
                ("hz_den", e.hz_den),
                ("ppm", e.ppm.into()),
            ] {
                if v == 0 {
                    return Err(refuse(HseRefusal::HseDeclarationZero { field }));
                }
            }
            crate::board::source(&e.source, "mcu.hse.source")?;
            in_range(e.hz_num, e.hz_den, EXTERNAL_HZ)?;
            bounded(e.ppm.into(), e.ppm.into())
        }
    }
}

/// A frequency of `num`/`den` Hz inside `(min, max, cite)`, both ends
/// included.
fn in_range(
    num: u64,
    den: u64,
    (min_hz, max_hz, cite): (u64, u64, &'static str),
) -> Result<(), IrValidationError> {
    let (n, d) = (u128::from(num), u128::from(den));
    if n < u128::from(min_hz) * d || n > u128::from(max_hz) * d {
        return Err(refuse(HseRefusal::HseFrequencyOutOfRange {
            hz_num: num,
            hz_den: den,
            min_hz,
            max_hz,
            cite,
        }));
    }
    Ok(())
}

/// A crystal's budget on `row`, the power parts row its `part` names
/// ([`declared_budget`] passes the embedded table's), each refusal by name.
pub fn crystal_budget(c: &HseCrystal, row: &power_parts::Row) -> Result<Budget, IrValidationError> {
    let zero = |field| Err(refuse(HseRefusal::HseDeclarationZero { field }));
    if c.pull_ppm == 0 {
        return zero("pull_ppm");
    }
    if c.life_hours == 0 {
        return zero("life_hours");
    }
    if c.load_caps_femtofarads.contains(&0) {
        return zero("load_caps_femtofarads");
    }
    let Kind::Crystal(crystal) = &row.kind else {
        return Err(refuse(HseRefusal::HsePartNotCrystal {
            part: c.part.clone(),
        }));
    };
    // The capacitors in series, at most the row's CL: the stray capacitance
    // (`4240c`'s, from `extract`) only adds to them, so a series value over CL
    // already loads the crystal past the CL its frequency and `Gm_crit` are
    // stated at (adversarial A3).
    let [c1, c2] = c.load_caps_femtofarads;
    let unfit = |reason| {
        Err(refuse(HseRefusal::HseLoadCapsUnfit {
            caps: c.load_caps_femtofarads,
            reason,
        }))
    };
    if c1 != c2 {
        return unfit("CL1 and CL2 differ");
    }
    if u128::from(c1) * u128::from(c2)
        > u128::try_from(crystal.load_femtofarads.value).unwrap_or(0)
            * (u128::from(c1) + u128::from(c2))
    {
        return unfit("in series they exceed the crystal's CL, before any stray capacitance");
    }
    let t = row.temperature_celsius.range;
    let (lo, hi) = (c.temperature_celsius.min, c.temperature_celsius.max);
    if lo > hi || i64::from(lo) < t.min || i64::from(hi) > t.max {
        return Err(refuse(HseRefusal::HseTemperatureOutsidePart {
            part: c.part.clone(),
            min: lo,
            max: hi,
            part_min: t.min,
            part_max: t.max,
        }));
    }
    if i64::from(c.life_hours) > crystal.aging_hours.value {
        return Err(refuse(HseRefusal::HseLifeBeyondAging {
            part: c.part.clone(),
            life_hours: c.life_hours,
            aging_hours: crystal.aging_hours.value,
        }));
    }
    in_range(
        u64::try_from(crystal.frequency_hz.value).unwrap_or(0),
        1,
        CRYSTAL_HZ,
    )?;
    let g = gm_crit(crystal);
    if !g.holds {
        return Err(refuse(HseRefusal::HseGmCritExceeded {
            part: c.part.clone(),
            gm_crit_nano_amps_per_volt: g.nano_amps_per_volt,
            max_nano_amps_per_volt: GM_CRIT_MAX_NANO_AMPS_PER_VOLT,
        }));
    }
    let side = |pick: fn(&power_parts::Range) -> i64| {
        [
            &crystal.tolerance_ppm,
            &crystal.stability_ppm,
            &crystal.aging_ppm,
        ]
        .iter()
        .map(|r| pick(&r.range).unsigned_abs())
        .sum::<u64>()
            + u64::from(c.pull_ppm)
    };
    bounded(side(|r| r.max), side(|r| r.min))
}

/// A budget whose sides are each under 1,000,000 ppm (adversarial A1: a
/// deviation of 100 % or more would wrap the checks' `10⁶ − ppm`).
fn bounded(fast: u64, slow: u64) -> Result<Budget, IrValidationError> {
    match (u32::try_from(fast), u32::try_from(slow)) {
        (Ok(fast_ppm), Ok(slow_ppm)) if fast < 1_000_000 && slow < 1_000_000 => {
            Ok(Budget { fast_ppm, slow_ppm })
        }
        _ => Err(refuse(HseRefusal::HseBudgetUnbounded {
            fast_ppm: fast,
            slow_ppm: slow,
        })),
    }
}

// ---------------------------------------------------------------- the judge

/// What [`judge_tree`] found for a tree that holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreeVerdict {
    /// The PLL1 reference, `(num, den)` Hz, strictly inside `range_hz`.
    pub reference_hz: (u64, u64),
    pub range_hz: (u64, u64),
    /// The VCO, `(num, den)` Hz, inside its `PLL1VCOSEL` range.
    pub vco_hz: (u64, u64),
    pub pll1_p_hz: u64,
    /// `pll1_q_ck`, when `DIVQ1EN` is 1 (nothing selects it until `4240b`).
    pub pll1_q_hz: Option<u64>,
    /// A crystal's `Gm_crit`; `None` for an external clock.
    pub gm_crit: Option<GmCrit>,
}

/// `width` bits of `v` from bit `at`.
fn field(v: u32, at: u32, width: u32) -> u32 {
    (v >> at) & ((1 << width) - 1)
}

/// The rule on a solved HSE tree (`AET-WO-4240a` §1.5, `4240` §4.2, D8),
/// `None` when `clock` declares no HSE. It re-derives the budget from the
/// copied declaration, reads `RCC_PLLCFGR` itself, and decodes the three
/// words with the shared [`SolvedClock::from_rcc`], then refuses by name:
/// a `PLLSRC` other than HSE or a fractional PLL; `DIVP1EN` 0; a reference
/// not strictly inside its `PLL1RGE` range (F-RGE, ruled for the generated
/// tree); a `PLL1VCOSEL` whose input range excludes it; a VCO outside the
/// select's range; and a clock whose fields are not what the words decode
/// to, exactly, with every prescaler 1 at VOS Scale 2. Every bus and kernel
/// clock against its VOS limit is [`crate::vos::judge_generated`]'s, at
/// this budget's fastest. `Gm_crit` is judged with the declaration.
pub fn judge_tree(clock: &SolvedClock) -> Result<Option<TreeVerdict>, IrValidationError> {
    let Some(h) = &clock.hse else {
        return Ok(None);
    };
    let mismatch = |reason: String| refuse(HseRefusal::HseClockMismatch { reason });
    let undecodable = |reason: String| refuse(HseRefusal::PllWordsUndecodable { reason });
    let budget = declared_budget(&h.declared)?;
    if budget != h.budget() {
        return Err(mismatch(format!(
            "its budget is +{}/-{} ppm, and the declaration gives +{}/-{}",
            h.fast_ppm, h.slow_ppm, budget.fast_ppm, budget.slow_ppm
        )));
    }
    let (num, den) = source_hz(&h.declared)?;
    if u128::from(num) != u128::from(clock.hse_hz) * u128::from(den) {
        return Err(mismatch(format!(
            "hse_hz is {}, and the declaration gives {num}/{den} Hz",
            clock.hse_hz
        )));
    }
    // RM0433 Rev 9 §8.7.11, p. 399: PLLSRC[1:0] `10` HSE; DIVM1[5:0].
    if field(h.pllckselr, 0, 2) != 0b10 {
        return Err(undecodable("RCC_PLLCKSELR.PLLSRC is not HSE (10)".into()));
    }
    // §8.7.12, pp. 400–402: PLL1FRACEN 0, PLL1VCOSEL 1, PLL1RGE 3:2, DIVP1EN 16,
    // DIVQ1EN 17.
    if field(h.pllcfgr, 0, 1) != 0 {
        return Err(undecodable("RCC_PLLCFGR.PLL1FRACEN is 1".into()));
    }
    if field(h.pllcfgr, 16, 1) == 0 {
        return Err(refuse(HseRefusal::PllOutputDisabled { field: "DIVP1EN" }));
    }
    // Every bit the judge does not read must be clear (adversarial A4):
    // `PLLCKSELR` beyond `PLLSRC` and `DIVM1` (reserved, and PLL2/PLL3's
    // prescalers, which H1 leaves 0, disabled); `PLLCFGR` beyond `PLL1FRACEN`,
    // `PLL1VCOSEL`, `PLL1RGE`, `DIVP1EN` and `DIVQ1EN` (`DIVR1EN`, PLL2/PLL3's
    // fields, reserved); `PLL1DIVR`'s reserved bits 23 and 31 (pp. 399–404).
    for (register, word, read) in [
        ("RCC_PLLCKSELR", h.pllckselr, 0x0000_03F3u32),
        ("RCC_PLLCFGR", h.pllcfgr, 0x0003_000F),
        ("RCC_PLL1DIVR", h.pll1divr, 0x7F7F_FFFF),
    ] {
        if word & !read != 0 {
            return Err(undecodable(format!(
                "{register} sets bits {:#010x} the judge does not read",
                word & !read
            )));
        }
    }
    // §8.7.13, p. 403: DIVR1 `0000000` is "Not allowed".
    if field(h.pll1divr, 24, 7) == 0 {
        return Err(undecodable(
            "RCC_PLL1DIVR.DIVR1 is 0, \"Not allowed\"".into(),
        ));
    }
    let divm1 = u64::from(field(h.pllckselr, 4, 6));
    if divm1 == 0 {
        return Err(undecodable(
            "RCC_PLLCKSELR.DIVM1 is 0, the prescaler disabled".into(),
        ));
    }
    let rge = field(h.pllcfgr, 2, 2);
    let vcosel = field(h.pllcfgr, 1, 1);
    let (min_hz, max_hz) = [
        (1_000_000, 2_000_000),
        (2_000_000, 4_000_000),
        (4_000_000, 8_000_000),
        (8_000_000, 16_000_000),
    ][rge as usize];
    // The reference is num / (den · DIVM1), compared in exact integers.
    let (rn, rd) = (u128::from(num), u128::from(den) * u128::from(divm1));
    let ref_den = den.saturating_mul(divm1);
    if rn <= u128::from(min_hz) * rd || rn >= u128::from(max_hz) * rd {
        return Err(refuse(HseRefusal::PllReferenceNotInsideRange {
            num,
            den: ref_den,
            min_hz,
            max_hz,
        }));
    }
    if (vcosel == 1) != (rge == 0) {
        return Err(refuse(HseRefusal::PllVcoSelectMismatch { rge, vcosel }));
    }
    // VCO limits: medium 150–420 MHz (DS12117 Rev 10 Table 40 rev Y = Table
    // 137 rev V); wide 192–836 MHz, revision Y's Table 39 (p. 130), the
    // stricter of the two revisions' maxima (rev V 960, Table 136), since no
    // silicon revision is declared (`4240` F-REV).
    let (vmin, vmax, cite) = if vcosel == 1 {
        (
            150_000_000u64,
            420_000_000u64,
            "DS12117 Rev 10 Table 40 rev Y = Table 137 rev V",
        )
    } else {
        (
            192_000_000,
            836_000_000,
            "DS12117 Rev 10 Table 39 rev Y p. 130, the stricter revision (`4240` F-REV)",
        )
    };
    let divn1 = u128::from(field(h.pll1divr, 0, 9)) + 1;
    let vn = rn * divn1;
    if vn < u128::from(vmin) * rd || vn > u128::from(vmax) * rd {
        return Err(refuse(HseRefusal::PllVcoOutOfRange {
            num: u64::try_from(vn).unwrap_or(u64::MAX),
            den: ref_den,
            vcosel,
            min_hz: vmin,
            max_hz: vmax,
            cite,
        }));
    }
    // The shared decoder (`4400g`'s), on the words as the bring-up would
    // leave them: HSE on and ready (bypassed for an external clock), PLL1
    // on and ready, SW = SWS = PLL1, every prescaler at reset.
    let bypass = matches!(h.declared, Hse::External(_));
    let registers = RccRegisters {
        cr: 1 << 16 | 1 << 17 | u32::from(bypass) << 18 | 1 << 24 | 1 << 25,
        cfgr: 0b011 | 0b011 << 3,
        d1cfgr: 0,
        d2cfgr: 0,
        pllckselr: h.pllckselr,
        pll1divr: h.pll1divr,
    };
    let decoded = SolvedClock::from_rcc(&registers, clock.hse_hz, vos::Vos::Scale2)
        .map_err(|e| undecodable(e.to_string()))?;
    let p = u128::from(field(h.pll1divr, 9, 7)) + 1;
    let exact = vn == u128::from(decoded.sysclk_hz) * rd * p;
    let expected = SolvedClock {
        hse: clock.hse.clone(),
        ..decoded
    };
    if !exact || expected != *clock {
        return Err(mismatch(format!(
            "the words decode to sys_ck {} Hz{} with every prescaler 1 at VOS Scale 2, and the clock is {clock:?}",
            expected.sysclk_hz,
            if exact { "" } else { " (not exactly)" }
        )));
    }
    let pll1_q_hz = (field(h.pllcfgr, 17, 1) == 1).then(|| {
        let q = u128::from(field(h.pll1divr, 16, 7)) + 1;
        u64::try_from(vn / (rd * q)).unwrap_or(u64::MAX)
    });
    // `pll1_q_ck` is SPI1–3's kernel at `SPI123SEL`'s reset (`4240` D6, from
    // `4240b`): judged against that kernel's limit at VOS Scale 2, at the
    // HSE's fastest (adversarial A4).
    if let Some(q) = pll1_q_hz {
        let fastest_hz = budget.fastest_hz(q);
        let (limit, cite) = vos::limit_hz(vos::Family::Spi123, vos::Vos::Scale2)
            .expect("Table 60 gives SPI1-3 a limit at Scale 2");
        if fastest_hz > limit {
            return Err(IrValidationError::VosLimitExceeded {
                clock: "pll1_q_ck (SPI1-3's kernel at SPI123SEL's reset)".into(),
                fastest_hz,
                limit_hz: limit,
                vos: vos::Vos::Scale2,
                cite: cite.into(),
            });
        }
    }
    let gm = match &h.declared {
        Hse::Crystal(c) => Some(gm_crit(crystal_row(&c.part)?)),
        Hse::External(_) => None,
    };
    Ok(Some(TreeVerdict {
        reference_hz: (num, ref_den),
        range_hz: (min_hz, max_hz),
        vco_hz: (u64::try_from(vn).unwrap_or(u64::MAX), ref_den),
        pll1_p_hz: u64::from(decoded.sysclk_hz),
        pll1_q_hz,
        gm_crit: gm,
    }))
}

/// The conditions a declared crystal rests on that no held datasheet lets
/// the engine compute (`AET-WO-4300k` M3), each by its id: AN2867 Rev 25
/// §4.2 Step 3's safety factor (§3.8, a negative-resistance measurement) and
/// Step 4's drive level (§3.5, from a measured `Vpp`), both p. 27.
pub fn conditions(hse: Option<&Hse>) -> Vec<(String, String)> {
    let Some(Hse::Crystal(c)) = hse else {
        return Vec::new();
    };
    vec![
        (
            "hse_safety_factor_unmeasured:HSE".to_owned(),
            format!(
                "HSE: {}'s oscillation safety factor is not computed: AN2867 Rev 25 §4.2 Step 3, p. 27, asks it assessed by §3.8's negative-resistance measurement, a bench row",
                c.part
            ),
        ),
        (
            "hse_drive_level_unmeasured:HSE".to_owned(),
            format!(
                "HSE: {}'s drive level is not computed: AN2867 Rev 25 §4.2 Step 4, p. 27, computes it by §3.5 from a measured Vpp, a bench row",
                c.part
            ),
        ),
    ]
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
