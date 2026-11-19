//! The battery sag's chain, solved statically (`AET-WO-2304b` §1.5, §1.6):
//! the operating point of constant-power sinks behind a series R from an
//! ideal source, and the clauses that refuse a sag wherever the declared
//! chain (D1 (a)) would be wrong (F5, D-B9). In this crate, the format's,
//! so replay re-judges kind 31 from its own fields without `aethelgard-ir`.
//!
//! Integer units throughout: µV, mΩ, mW, pF, ns.

use crate::campaign::ScenarioError;
use crate::record::{BatterySag, SagSink};

/// The most battery-fed sinks kind 31 carries (the bridge's bound).
pub const MAX_SINKS: usize = 8;

/// The bounds of a sag's integers, each checked before any arithmetic: a
/// path of at most 10 kΩ (the bridge's bound), a start inside the 100 ms boot.
pub const SERIES_MILLIOHMS_MAX: u64 = 10_000_000;
pub const START_NS_MAX: u64 = 100_000_000;

/// How far the deck's solved minimum may sit from the static operating
/// point, µV. The deck solves to `reltol` 1e-6 (9 µV at 9 V) and measured
/// at most 0.5 µV off the closed form (`bridge/tests/chain.rs`); the static
/// point is rounded down to whole µV. 1 mV is the bound the predictions
/// stated (Q5); a larger difference is an internal error, never a verdict.
pub const SOLVE_TOLERANCE_UV: u64 = 1_000;

/// The refusal kind of every clause but the three the work order names on
/// their own (§1.6).
pub const REFUSED: &str = "BatterySagRefused";

/// The open voltage the bridge's deck admits, µV (100 V).
pub const V_OPEN_MAX_UV: u64 = 100_000_000;

/// The chain the specification declares, as the scenario's checks and kind
/// 31 read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SagChain {
    pub v_open_microvolts: u64,
    pub healthy_milliohms: u32,
    pub sinks: Vec<SagSink>,
    /// Every capacitance on `VBAT` the deck holds, pF.
    pub capacitance_picofarads: u64,
}

impl SagChain {
    /// Every sink's input power, mW.
    pub fn input_milliwatts(&self) -> u64 {
        self.sinks
            .iter()
            .map(|k| k.input_milliwatts)
            .fold(0, u64::saturating_add)
    }
}

/// The operating point of `p_mw` of constant power behind `r_mohm` from
/// `v_open_uv`, µV: V = (V_open + √(V_open² − 4·R·P)) / 2, the high root,
/// rounded down (toward refusal). `None` when V_open² < 4·R·P: past the
/// collapse there is none. In µV², R·P [mΩ·mW] is R·P·10⁶.
pub fn operating_point_uv(v_open_uv: u64, r_mohm: u64, p_mw: u64) -> Option<u64> {
    let v = u128::from(v_open_uv);
    let load = 4u128
        .checked_mul(u128::from(r_mohm))?
        .checked_mul(u128::from(p_mw))?
        .checked_mul(1_000_000)?;
    // At a zero discriminant the two roots meet: the collapse itself, no
    // operating point to hold (adversarial 9b).
    let disc = (v * v).checked_sub(load).filter(|d| *d > 0)?;
    u64::try_from((v + disc.isqrt()) / 2).ok()
}

/// The sink whose floor binds: the highest, the first in the
/// specification's order on a tie.
pub fn binding(sinks: &[SagSink]) -> Option<&SagSink> {
    sinks.iter().rev().max_by_key(|k| k.floor_microvolts())
}

/// The time after the step by which the chain has settled, ns: 10 τ, where
/// τ is the incremental time constant at the operating point `v_uv`,
/// R·C / (1 − R·P/V²), rounded up. Under constant power the incremental
/// conductance is 1/R − P/V², so τ is longer than R·C; at the operating
/// point R·P·10⁶ = V·(V_open − V), so the denominator is V·(2V − V_open),
/// positive above the collapse. `None` at or below V_open / 2.
pub fn settle_ns(chain: &SagChain, r_mohm: u64, v_uv: u64) -> Option<u64> {
    let (v, vo) = (u128::from(v_uv), u128::from(chain.v_open_microvolts));
    let den = v.checked_mul((2 * v).checked_sub(vo).filter(|d| *d > 0)?)?;
    // mΩ · pF = 10⁻¹⁵ s = 10⁻⁶ ns.
    let rc = u128::from(r_mohm).checked_mul(u128::from(chain.capacitance_picofarads))?;
    let tau = rc.checked_mul(v * v)?.div_ceil(den.checked_mul(1_000_000)?);
    u64::try_from(tau.checked_mul(10)?).ok()
}

/// A battery sag the chain admits (`AET-WO-2304b` §1.6), judged from the
/// declared chain alone: refused when it is not a fault (its R at or below
/// the healthy R), when it has no operating point (F5), when the operating
/// point lies below any sink's shutdown (F5) or regulation floor (D-B9,
/// F11), naming the sink and both figures, and when the chain has not
/// settled by `span_ns`. `Ok` is the static operating point, µV.
pub fn check_battery_sag(
    sag: &BatterySag,
    chain: &SagChain,
    span_ns: u64,
) -> Result<u64, ScenarioError> {
    let refuse = |m: String| ScenarioError::new(REFUSED, None, m);
    let r = u64::from(sag.series_milliohms);
    // The shape the parser and the deck hold, judged here too, so replay
    // refuses a record no scenario could have made (adversarial 4).
    let shaped = (1..=SERIES_MILLIOHMS_MAX).contains(&r)
        && (1..=START_NS_MAX).contains(&sag.start_ns)
        && (1..=V_OPEN_MAX_UV).contains(&chain.v_open_microvolts)
        && chain.capacitance_picofarads > 0
        && (1..=MAX_SINKS).contains(&chain.sinks.len())
        && chain.sinks.iter().all(|k| {
            k.input_milliwatts > 0 && k.shutdown_millivolts > 0 && k.regulation_millivolts > 0
        });
    if !shaped {
        return Err(refuse(format!(
            "out of shape: {r} mΩ from {} ns, {} µV open, {} pF on the battery, {} sinks, each with power and both floors above 0, outside the bounds the scenario and the deck hold",
            sag.start_ns,
            chain.v_open_microvolts,
            chain.capacitance_picofarads,
            chain.sinks.len()
        )));
    }
    if sag.series_milliohms <= chain.healthy_milliohms {
        return Err(refuse(format!(
            "not a fault: its {r} mΩ is not above the healthy path's {} mΩ",
            chain.healthy_milliohms
        )));
    }
    let p = chain.input_milliwatts();
    let Some(v) = operating_point_uv(chain.v_open_microvolts, r, p) else {
        return Err(refuse(format!(
            "no operating point: {p} mW of constant power behind {r} mΩ from {} µV collapses (F5)",
            chain.v_open_microvolts
        )));
    };
    for k in &chain.sinks {
        let floor = k.shutdown_millivolts.saturating_mul(1000);
        if v < floor {
            return Err(refuse(format!(
                "below shutdown: the battery sits at {v} µV, under {}'s stated minimum operating input of {floor} µV, where its behaviour is not stated (F5); its regulation floor is {} µV",
                k.name,
                k.regulation_millivolts.saturating_mul(1000)
            )));
        }
    }
    for k in &chain.sinks {
        let floor = k.regulation_millivolts.saturating_mul(1000);
        if v < floor {
            return Err(refuse(format!(
                "below regulation: the battery sits at {v} µV, under the {floor} µV from which {}'s output regulation band is stated (D-B9); its shutdown floor is {} µV",
                k.name,
                k.shutdown_millivolts.saturating_mul(1000)
            )));
        }
    }
    let end = settle_ns(chain, r, v).and_then(|s| s.checked_add(sag.start_ns));
    match end {
        Some(end) if end <= span_ns => Ok(v),
        _ => Err(refuse(format!(
            "past the span: stepped at {} ns, the chain settles (10 τ) at {} ns, past the boot's {span_ns} ns",
            sag.start_ns,
            end.map_or_else(|| "no time".to_owned(), |e| e.to_string())
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sink(name: &str, mw: u64, shut: u64, reg: u64) -> SagSink {
        SagSink {
            name: name.into(),
            input_milliwatts: mw,
            shutdown_millivolts: shut,
            regulation_millivolts: reg,
        }
    }

    #[test]
    fn the_operating_point_is_the_high_root_rounded_down() {
        // Sentinel-03 (predictions Q1-Q4): 14,752 mW from 9 V.
        assert_eq!(operating_point_uv(9_000_000, 1, 14_752), Some(8_998_360));
        assert_eq!(operating_point_uv(9_000_000, 500, 14_752), Some(8_088_035));
        assert_eq!(
            operating_point_uv(9_000_000, 1_372, 14_752),
            Some(4_601_271)
        );
        assert_eq!(operating_point_uv(9_000_000, 1_373, 14_752), None);
        assert_eq!(operating_point_uv(9_000_000, 0, 14_752), Some(9_000_000));
        assert_eq!(operating_point_uv(u64::MAX, u64::MAX, u64::MAX), None);
    }

    #[test]
    fn the_binding_floor_is_the_highest_and_the_first_on_a_tie() {
        let s = [sink("U10", 1, 3_800, 7_000), sink("U11", 1, 3_800, 3_800)];
        assert_eq!(binding(&s).unwrap().name, "U10");
        let t = [sink("A", 1, 3_800, 3_800), sink("B", 1, 3_800, 3_800)];
        assert_eq!(binding(&t).unwrap().name, "A");
        assert!(binding(&[]).is_none());
    }
}
