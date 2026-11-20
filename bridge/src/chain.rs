//! `aethelgard-bridge --chain <request.json>` (`AET-WO-2304b`): a battery
//! feeding a board's battery-fed regulators, solved once by a transient. The
//! request is a strict JSON object of integers; the bridge writes the deck
//! itself from them, so no text of the request reaches ngspice (`INV-06`; the
//! sink names are checked and never written: the deck uses indices).
//!
//! ```json
//! {"v_open_microvolts":9000000,"healthy_milliohms":1,"fault_milliohms":500,
//!  "start_ns":10000000,"span_ns":100000000,
//!  "bulk":[{"picofarads":47000000000,"esr_milliohms":250}],
//!  "ceramics_picofarads":20440000,
//!  "sinks":[{"name":"U10","milliwatts":11645,"floor_microvolts":7000000},
//!           {"name":"U11","milliwatts":3107,"floor_microvolts":3800000}]}
//! ```
//!
//! The deck: an ideal V_open, then a series R(t) to the node `vbat`; the bulk
//! electrolytics (a C in series with its ESR to ground), the regulators' input
//! ceramics as one ideal C, and one constant-power sink per entry,
//! `I = P / max(V(vbat), floor)`. R(t) is the healthy R until `start_ns`, then
//! ramps linearly to the fault R over [`RAMP_NS`] (a deterministic step, with
//! a breakpoint at each end of the ramp), through a PWL control voltage.
//! Below its floor a sink is not solved through: the guard keeps its current
//! bounded and continuous, and the crossing is reported.

use serde_json::{Map, Value};

use crate::decimal;
use crate::request::{integer, object, string};

const KEYS: [&str; 8] = [
    "v_open_microvolts",
    "healthy_milliohms",
    "fault_milliohms",
    "start_ns",
    "span_ns",
    "bulk",
    "ceramics_picofarads",
    "sinks",
];
const MAX_BULK: usize = 8;
const MAX_SINKS: usize = 8;
/// The ramp that makes the step, in ns.
pub const RAMP_NS: u64 = 1;

#[derive(Debug, Clone, PartialEq)]
pub struct Bulk {
    pub picofarads: u64,
    pub esr_mohm: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sink {
    pub name: String,
    pub milliwatts: u64,
    pub floor_uv: u64,
}

/// A request that passed every bound.
#[derive(Debug, Clone, PartialEq)]
pub struct Chain {
    pub v_open_uv: u64,
    pub healthy_mohm: u64,
    pub fault_mohm: u64,
    pub start_ns: u64,
    pub span_ns: u64,
    pub bulk: Vec<Bulk>,
    pub ceramics_pf: u64,
    pub sinks: Vec<Sink>,
}

pub fn parse(bytes: &[u8]) -> Result<Chain, String> {
    let v: Value = serde_json::from_slice(bytes).map_err(|e| format!("not JSON: {e}"))?;
    let o = object(&v, "the request", &KEYS)?;
    let v_open_uv = bounded(o, "v_open_microvolts", 1, 100_000_000)?;
    let healthy_mohm = bounded(o, "healthy_milliohms", 1, 10_000_000)?;
    let fault_mohm = bounded(o, "fault_milliohms", 1, 10_000_000)?;
    if fault_mohm <= healthy_mohm {
        return Err(format!(
            "`fault_milliohms` {fault_mohm} is not above `healthy_milliohms` {healthy_mohm}"
        ));
    }
    let span_ns = bounded(o, "span_ns", 1, 100_000_000)?;
    let start_ns = bounded(o, "start_ns", 1, u64::MAX)?;
    if start_ns >= span_ns {
        return Err(format!(
            "`start_ns` {start_ns} is not before `span_ns` {span_ns}"
        ));
    }
    let ceramics_pf = bounded(o, "ceramics_picofarads", 0, 10_000_000_000)?;
    let list = o["bulk"].as_array().ok_or("`bulk` is not an array")?;
    if list.len() > MAX_BULK {
        return Err(format!("`bulk` holds {}, not 0 to {MAX_BULK}", list.len()));
    }
    let mut bulk = Vec::with_capacity(list.len());
    for (i, b) in list.iter().enumerate() {
        let bo = object(b, &format!("bulk[{i}]"), &["picofarads", "esr_milliohms"])?;
        let at = |e: String| format!("bulk[{i}]: {e}");
        bulk.push(Bulk {
            picofarads: bounded(bo, "picofarads", 1, 10_000_000_000).map_err(at)?,
            esr_mohm: bounded(bo, "esr_milliohms", 1, 1_000_000).map_err(at)?,
        });
    }
    if bulk.is_empty() && ceramics_pf == 0 {
        return Err("neither `bulk` nor `ceramics_picofarads` gives any capacitance".to_owned());
    }
    let list = o["sinks"].as_array().ok_or("`sinks` is not an array")?;
    if list.is_empty() || list.len() > MAX_SINKS {
        return Err(format!(
            "`sinks` holds {}, not 1 to {MAX_SINKS}",
            list.len()
        ));
    }
    let mut sinks: Vec<Sink> = Vec::with_capacity(list.len());
    for (i, s) in list.iter().enumerate() {
        let so = object(
            s,
            &format!("sinks[{i}]"),
            &["name", "milliwatts", "floor_microvolts"],
        )?;
        let at = |e: String| format!("sinks[{i}]: {e}");
        let name = string(so, "name").map_err(at)?;
        printable(&name, &format!("sinks[{i}].name"))?;
        if sinks.iter().any(|x| x.name == name) {
            return Err(format!("sinks[{i}]: the name `{name}` appears twice"));
        }
        let milliwatts = bounded(so, "milliwatts", 1, 1_000_000).map_err(at)?;
        let floor_uv = bounded(so, "floor_microvolts", 1, u64::MAX).map_err(at)?;
        if floor_uv >= v_open_uv {
            return Err(format!(
                "sinks[{i}].floor_microvolts {floor_uv} is not below `v_open_microvolts` {v_open_uv}"
            ));
        }
        sinks.push(Sink {
            name,
            milliwatts,
            floor_uv,
        });
    }
    let chain = Chain {
        v_open_uv,
        healthy_mohm,
        fault_mohm,
        start_ns,
        span_ns,
        bulk,
        ceramics_pf,
        sinks,
    };
    // The transient starts at the HIGH operating point of the healthy path,
    // which must exist and lie above every floor, or the start is not the
    // state the sinks' power law describes.
    let Some(v0) = chain.steady_volts(healthy_mohm) else {
        return Err(format!(
            "no operating point at `healthy_milliohms` {healthy_mohm}"
        ));
    };
    if let Some(s) = chain.sinks.iter().find(|s| v0 * 1e6 <= s.floor_uv as f64) {
        return Err(format!(
            "the healthy operating point {v0} V is not above the floor of `{}`",
            s.name
        ));
    }
    Ok(chain)
}

fn bounded(o: &Map<String, Value>, key: &str, min: u64, max: u64) -> Result<u64, String> {
    let n = integer(o, key)?;
    if n < min || n > max {
        return Err(format!("`{key}` is {n}, not {min} to {max}"));
    }
    Ok(n)
}

fn printable(s: &str, what: &str) -> Result<(), String> {
    if (1..=64).contains(&s.len()) && s.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
        Ok(())
    } else {
        Err(format!("{what} is not 1 to 64 printable ASCII bytes"))
    }
}

impl Chain {
    /// The HIGH steady state at a series `mohm`, in volts: `(V_open +
    /// sqrt(V_open² − 4·R·P)) / 2`; `None` when `V_open² < 4·R·P`, the
    /// comparison made in exact integers. In µV², 4·R·P is
    /// `4 · mΩ · mW · 10^6` (R = mΩ/10^3 Ω, P = mW/10^3 W, 1 V² = 10^12 µV²).
    pub fn steady_volts(&self, mohm: u64) -> Option<f64> {
        let p_mw: u128 = self.sinks.iter().map(|s| s.milliwatts as u128).sum();
        let four_rp = 4 * mohm as u128 * p_mw * 1_000_000;
        let vo2 = self.v_open_uv as u128 * self.v_open_uv as u128;
        if vo2 < four_rp {
            return None;
        }
        let vo = self.v_open_uv as f64 / 1e6;
        let disc = (vo2 - four_rp) as f64 / 1e12;
        Some((vo + disc.sqrt()) / 2.0)
    }

    /// The maximum timestep, in ns: a ten-thousandth of the span, at least 1.
    pub fn step_ns(&self) -> u64 {
        (self.span_ns / 10_000).max(1)
    }

    /// The breakpoints, in seconds: both ends of the resistance ramp.
    pub fn breakpoints(&self) -> [f64; 2] {
        [
            self.start_ns as f64 * 1e-9,
            (self.start_ns + RAMP_NS) as f64 * 1e-9,
        ]
    }

    /// The deck, written from the integers alone.
    pub fn deck(&self) -> Vec<String> {
        let v0 = self
            .steady_volts(self.healthy_mohm)
            .expect("parse checked the healthy operating point");
        let (rh, rf) = (decimal(self.healthy_mohm, 3), decimal(self.fault_mohm, 3));
        let ramp_end = self.start_ns + RAMP_NS;
        let mut d = vec![
            "* aethelgard chain".to_owned(),
            format!("vopen src 0 dc {}", decimal(self.v_open_uv, 6)),
            format!("vctl ctl 0 pwl(0 0 {}n 0 {ramp_end}n 1)", self.start_ns),
            format!("rs src vbat r = '{rh} + ({rf} - {rh}) * v(ctl)'"),
        ];
        let mut nodeset = format!(".nodeset v(vbat)={v0:.9}");
        for (i, b) in self.bulk.iter().enumerate() {
            d.push(format!("resr{i} vbat nb{i} {}", decimal(b.esr_mohm, 3)));
            d.push(format!("cb{i} nb{i} 0 {}", decimal(b.picofarads, 12)));
            nodeset.push_str(&format!(" v(nb{i})={v0:.9}"));
        }
        if self.ceramics_pf > 0 {
            d.push(
                "* the regulators' input ceramics, summed, as one ideal C (their ESR is not held)"
                    .to_owned(),
            );
            d.push(format!("cceramic vbat 0 {}", decimal(self.ceramics_pf, 12)));
        }
        for (i, s) in self.sinks.iter().enumerate() {
            d.push(format!(
                "bsink{i} vbat 0 i = '{} / max(v(vbat), {})'",
                decimal(s.milliwatts, 3),
                decimal(s.floor_uv, 6)
            ));
        }
        d.push(nodeset);
        d.push(".options reltol=1e-6 abstol=1e-12 vntol=1e-9 method=trap".to_owned());
        let step = self.step_ns();
        d.push(format!(".tran {step}n {}n 0 {step}n", self.span_ns));
        d.push(".end".to_owned());
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{"v_open_microvolts":9000000,"healthy_milliohms":1,
        "fault_milliohms":500,"start_ns":10000000,"span_ns":100000000,
        "bulk":[{"picofarads":47000000,"esr_milliohms":250}],
        "ceramics_picofarads":20440000,
        "sinks":[{"name":"U10","milliwatts":11645,"floor_microvolts":7000000},
                 {"name":"U11","milliwatts":3107,"floor_microvolts":3800000}]}"#;

    #[test]
    fn the_reference_request_parses_and_its_deck_is_exact() {
        let chain = parse(GOOD.as_bytes()).unwrap();
        assert_eq!(chain.step_ns(), 10_000);
        assert_eq!(
            chain.breakpoints(),
            [10_000_000f64 * 1e-9, 10_000_001f64 * 1e-9]
        );
        assert_eq!(
            chain.deck().join("\n"),
            "* aethelgard chain\n\
             vopen src 0 dc 9\n\
             vctl ctl 0 pwl(0 0 10000000n 0 10000001n 1)\n\
             rs src vbat r = '0.001 + (0.5 - 0.001) * v(ctl)'\n\
             resr0 vbat nb0 0.25\n\
             cb0 nb0 0 0.000047\n\
             * the regulators' input ceramics, summed, as one ideal C (their ESR is not held)\n\
             cceramic vbat 0 0.00002044\n\
             bsink0 vbat 0 i = '11.645 / max(v(vbat), 7)'\n\
             bsink1 vbat 0 i = '3.107 / max(v(vbat), 3.8)'\n\
             .nodeset v(vbat)=8.998360590 v(nb0)=8.998360590\n\
             .options reltol=1e-6 abstol=1e-12 vntol=1e-9 method=trap\n\
             .tran 10000n 100000000n 0 10000n\n\
             .end"
        );
    }

    #[test]
    fn the_operating_point_exists_exactly_when_the_discriminant_is_not_negative() {
        let chain = parse(GOOD.as_bytes()).unwrap();
        assert!(chain.steady_volts(960).is_some());
        // 4 * R * P = 81 V^2 at R = 81 / (4 * 14.752) = 1.3726 ohm.
        assert!(chain.steady_volts(1_372).is_some());
        assert!(chain.steady_volts(1_373).is_none());
        // Near the knee the two roots merge at V_open / 2.
        let v = chain.steady_volts(1_372).unwrap();
        assert!((4.5..4.7).contains(&v), "{v}");
    }
}
