//! A logic line's levels at its receiver's pin (`AET-WO-4411` (c1), ruled
//! 2026-11-26T04:03Z): the driver's VOH and VOL at its own pin against the
//! receiver's VIH and VIL at its own, each linear in the supply at that pin.
//!
//! A pin's supply is its domain's source band less, at most, the domain's
//! drop limit. Two pins on one domain share one source, so only the drops
//! separate them: they are judged at one source voltage, at its minimum and
//! at its maximum. Pins on two domains are judged at each source's extremes
//! independently. Every per-mille term is at least zero, so a level is
//! lowest at the lowest supply its pin can see and highest at the highest.

/// A level linear in its pin's supply: `permille` of it plus `offset_uv`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Linear {
    pub permille: i64,
    pub offset_uv: i64,
}

impl Linear {
    /// At `supply_uv`, rounded down (a minimum the driver guarantees, or a
    /// maximum the receiver reads low below).
    fn floor(self, supply_uv: i64) -> i64 {
        (supply_uv * self.permille).div_euclid(1000) + self.offset_uv
    }

    /// At `supply_uv`, rounded up (a threshold the driver must clear).
    fn ceil(self, supply_uv: i64) -> i64 {
        -(-(supply_uv * self.permille)).div_euclid(1000) + self.offset_uv
    }
}

/// One pin's supply: its domain, the domain's source band, µV, and the drop
/// to the pin at most, µV.
#[derive(Debug, Clone, Copy)]
pub struct Supply<'a> {
    pub domain: &'a str,
    pub band_uv: (i64, i64),
    pub drop_uv: i64,
}

/// A line's driver: VOH at least `high_min`, VOL at most `low_max`, each
/// stated at `load_ua` (`None`: at a high-impedance load). Its high is at
/// most its own supply.
#[derive(Debug, Clone)]
pub struct Driver<'a> {
    pub who: String,
    pub supply: Supply<'a>,
    pub high_min: Linear,
    pub low_max: Linear,
    pub load_ua: Option<i64>,
}

/// A line's receiver: it reads high above `high_min`, low below
/// `low_max`, takes at most `high_max` when stated, and draws `current_na`.
#[derive(Debug, Clone)]
pub struct Receiver<'a> {
    pub who: String,
    pub supply: Supply<'a>,
    pub high_min: Linear,
    pub low_max: Linear,
    pub high_max: Option<Linear>,
    pub current_na: i64,
}

/// What one line holds at its worst corner, µV each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Margins {
    pub high_uv: i64,
    pub low_uv: i64,
    pub over_uv: Option<i64>,
}

/// The first level the line does not hold, worded for a refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unmet(pub String);

/// The source voltages a line is judged at: one shared source, or each
/// pair of extremes.
fn corners(d: &Supply<'_>, r: &Supply<'_>) -> Vec<(i64, i64)> {
    if d.domain == r.domain {
        vec![(d.band_uv.0, d.band_uv.0), (d.band_uv.1, d.band_uv.1)]
    } else {
        let mut v = Vec::with_capacity(4);
        for vd in [d.band_uv.0, d.band_uv.1] {
            for vr in [r.band_uv.0, r.band_uv.1] {
                v.push((vd, vr));
            }
        }
        v
    }
}

/// Judges one line (`net`) from `d` to `r`.
///
/// # Errors
/// The first unmet level: the driver's current rows stated under the
/// receiver's draw, its high under the receiver's high threshold, its low
/// over the receiver's low threshold, or its supply over the receiver's
/// highest input.
pub fn judge(net: &str, d: &Driver<'_>, r: &Receiver<'_>) -> Result<Margins, Unmet> {
    if let Some(ua) = d.load_ua
        && r.current_na > ua * 1000
    {
        return Err(Unmet(format!(
            "`{net}`: {} draws up to {} nA, over the {ua} uA {}'s output levels are stated at",
            r.who, r.current_na, d.who
        )));
    }
    let mut m = Margins {
        high_uv: i64::MAX,
        low_uv: i64::MAX,
        over_uv: r.high_max.map(|_| i64::MAX),
    };
    let (dd, rd) = (d.supply.drop_uv, r.supply.drop_uv);
    for (vd, vr) in corners(&d.supply, &r.supply) {
        let (high, threshold) = (d.high_min.floor(vd - dd), r.high_min.ceil(vr));
        if high < threshold {
            return Err(Unmet(format!(
                "`{net}`: {}'s high is at least {high} uV at its pin (`{}` at {vd} uV less {dd} uV), under the {threshold} uV {} reads high above (`{}` at {vr} uV)",
                d.who, d.supply.domain, r.who, r.supply.domain
            )));
        }
        m.high_uv = m.high_uv.min(high - threshold);
        let (low, threshold) = (d.low_max.ceil(vd), r.low_max.floor(vr - rd));
        if low > threshold {
            return Err(Unmet(format!(
                "`{net}`: {}'s low is up to {low} uV (`{}` at {vd} uV), over the {threshold} uV {} reads low below (`{}` at {vr} uV less {rd} uV)",
                d.who, d.supply.domain, r.who, r.supply.domain
            )));
        }
        m.low_uv = m.low_uv.min(threshold - low);
        if let (Some(max), Some(o)) = (r.high_max, m.over_uv.as_mut()) {
            let limit = max.floor(vr - rd);
            if vd > limit {
                return Err(Unmet(format!(
                    "`{net}`: {}'s high reaches its supply, {vd} uV (`{}`), over the {limit} uV {} takes (`{}` at {vr} uV less {rd} uV)",
                    d.who, d.supply.domain, r.who, r.supply.domain
                )));
            }
            *o = (*o).min(limit - vd);
        }
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::{Driver, Linear, Receiver, Supply, judge};

    fn lin(permille: i64, offset_uv: i64) -> Linear {
        Linear {
            permille,
            offset_uv,
        }
    }

    /// One source, two drops: F2's band counted once. The same pins on two
    /// rails with the same bands are judged at the bands' opposite ends.
    #[test]
    fn one_rail_is_judged_at_one_source_two_at_each_extreme() {
        let s = |domain| Supply {
            domain,
            band_uv: (3_144_727, 3_417_204),
            drop_uv: 144_000,
        };
        let d = |domain| Driver {
            who: "D".into(),
            supply: s(domain),
            high_min: lin(1000, -100_000),
            low_max: lin(0, 100_000),
            load_ua: Some(100),
        };
        let r = |domain| Receiver {
            who: "R".into(),
            supply: s(domain),
            high_min: lin(750, 0),
            low_max: lin(250, 0),
            high_max: Some(lin(1000, 300_000)),
            current_na: 50,
        };
        let m = judge("N", &d("A"), &r("A")).unwrap();
        assert_eq!(m.over_uv, Some(156_000));
        let e = judge("N", &d("A"), &r("B")).unwrap_err();
        assert!(
            e.0.contains("3417204 uV") && e.0.contains("over the 3300727 uV"),
            "{e:?}"
        );
    }
}
