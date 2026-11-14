//! The part's supply supervisor as the campaign renders it (`AET-WO-2301`,
//! ruled D3 and D4 on PR #107): one table of thresholds and timings, with its
//! source, and the rule that turns a rail's crossings into resets. The
//! campaign composes Renode's schedule with [`schedule`], and replay judges
//! the archived resets with the same function, so the rule lives here alone.
//!
//! Every value is DS12117 Rev 10's, read from st.com (`AET-WO-2301` D10,
//! closed 2026-11-14; `AET-WO-2205` §1.7). Rev 7's values, first read from a
//! mirror (`2301` F1), are equal, and a test holds them so.

/// Where the table's values come from.
pub struct Source {
    pub document: &'static str,
    pub revision: u32,
    pub date: &'static str,
    pub table: &'static str,
    /// The copy actually read, and its sha256.
    pub read_from: &'static str,
    pub sha256: &'static str,
}

pub const SOURCE: Source = Source {
    document: "DS12117 (STM32H753xI datasheet)",
    revision: 10,
    date: "January 2026",
    table: "Table 16, Reset and power control block characteristics (section 6.3.4, p. 107)",
    read_from: "st.com (fetched by Scotty, 2026-11-14)",
    sha256: "607b9eadaef1c54f93e02d7cea58c862a1552bda47c848d96f2de5c0ee84c032",
};

/// One brown-out threshold: (min, typ, max) for each edge, in µV.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Threshold {
    pub name: &'static str,
    pub falling_uv: [u64; 3],
    pub rising_uv: [u64; 3],
}

impl Threshold {
    pub const fn falling(&self) -> u64 {
        self.falling_uv[1]
    }
    pub const fn rising(&self) -> u64 {
        self.rising_uv[1]
    }
}

/// `VBOR0` to `VBOR3`, Table 16. BOR0 is the part's lowest reset: Rev 10's
/// row is `VBOR0/POR/PDR`, the POR and PDR thresholds (Rev 7 had no POR/PDR
/// row, and said BOR0 "is enabled in all modes").
pub const BOR: [Threshold; 4] = [
    Threshold {
        name: "VBOR0",
        falling_uv: [1_580_000, 1_620_000, 1_680_000],
        rising_uv: [1_620_000, 1_670_000, 1_710_000],
    },
    Threshold {
        name: "VBOR1",
        falling_uv: [1_950_000, 2_000_000, 2_060_000],
        rising_uv: [2_040_000, 2_100_000, 2_150_000],
    },
    Threshold {
        name: "VBOR2",
        falling_uv: [2_250_000, 2_310_000, 2_370_000],
        rising_uv: [2_340_000, 2_410_000, 2_470_000],
    },
    Threshold {
        name: "VBOR3",
        falling_uv: [2_540_000, 2_610_000, 2_680_000],
        rising_uv: [2_630_000, 2_700_000, 2_780_000],
    },
];

/// "Reset temporization after BOR0 released", typical, Table 16.
pub const T_RSTTEMPO_NS: u64 = 377_000;

/// The schedule's resolution: a reset or a release lands on the first 10 ns
/// boundary at or after the crossing (one instruction at the twin's MIPS).
pub const RESOLUTION_NS: u64 = 10;

/// A release takes effect only at the end of the next `RunFor` (`AET-WO-2301`
/// F5, measured), so every release is followed by a `RunFor` this long.
pub const NUDGE_NS: u64 = 10;

/// The firmware must have rewritten its boot word and completed its I2C read
/// this long after a release (`AET-WO-2301` §2.3). Measured: 20 µs.
pub const RECOVERY_BOUND_NS: u64 = 100_000;

/// How a reset is rendered, as kind 18 records it: Renode 1.17.0 models no
/// BOR, PVD or reset cause (`AET-WO-2301` P1, F6), so these monitor commands
/// stand in for the supervisor. The campaign composes exactly these.
pub const ASSERT_RENDERING: &str = "no BOR in Renode 1.17.0: machine Reset; the cold writes and cpu VectorTableOffset re-applied; cpu IsHalted true; the boot word and the record re-armed";
pub const RELEASE_RENDERING: &str = "cpu IsHalted false; emulation RunFor 10 ns";

/// The highest BOR level the option bytes can select.
pub const MAX_BOR_LEVEL: u8 = 3;

/// The thresholds active at a BOR level: BOR0 always, and the selected one.
pub fn active(bor_level: u8) -> Vec<&'static Threshold> {
    let mut v = vec![&BOR[0]];
    if (1..=MAX_BOR_LEVEL).contains(&bor_level) {
        v.push(&BOR[usize::from(bor_level)]);
    }
    v
}

/// The level names the bridge is asked to find crossings of, with their
/// typical values: each active threshold's falling and rising level.
pub fn levels(bor_level: u8) -> Vec<(String, u64)> {
    active(bor_level)
        .into_iter()
        .flat_map(|t| {
            [
                (format!("{}_falling", t.name), t.falling()),
                (format!("{}_rising", t.name), t.rising()),
            ]
        })
        .collect()
}

/// One crossing of one level, as ngspice computed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Crossing {
    pub level: String,
    pub microvolts: u64,
    pub at_ps: u64,
    pub falling: bool,
}

/// One reset the supervisor holds: from `asserted_ns` until `released_ns`
/// (absent when the rail never recovers in the run), with the thresholds
/// that held it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reset {
    pub asserted_ns: u64,
    pub released_ns: Option<u64>,
    pub cause: Vec<String>,
}

/// `ps` rounded up to the schedule's resolution, in ns.
pub fn on_grid(ps: u64) -> u64 {
    ps.div_ceil(RESOLUTION_NS * 1000) * RESOLUTION_NS
}

/// The resets the active thresholds hold for these crossings (ruled D4): a
/// threshold goes low at a falling crossing of its falling level and clears
/// at a rising crossing of its rising level, BOR0 `tRSTTEMPO` later; the part
/// is held in reset while any is low, so a reset is asserted when the first
/// goes low and released when the last clears. Crossings in the other
/// direction of a level are hysteresis and change nothing. Times are on the
/// 10 ns grid. A crossing of an unknown level is an error.
pub fn schedule(bor_level: u8, crossings: &[Crossing]) -> Result<Vec<Reset>, &'static str> {
    let active = active(bor_level);
    // Each threshold's edges in time order: (time ns, goes low).
    let mut edges: Vec<Vec<(u64, bool)>> = vec![Vec::new(); active.len()];
    for c in crossings {
        let (i, t) = active
            .iter()
            .enumerate()
            .find(|(_, t)| {
                c.level == format!("{}_falling", t.name) || c.level == format!("{}_rising", t.name)
            })
            .ok_or("a crossing names a level no active threshold has")?;
        let falling_level = c.level.ends_with("_falling");
        let want = if falling_level {
            t.falling()
        } else {
            t.rising()
        };
        if c.microvolts != want {
            return Err("a crossing's level is not the table's typical value");
        }
        if falling_level == c.falling {
            edges[i].push((on_grid(c.at_ps), c.falling));
        }
    }
    // Each threshold's low intervals: low from a fall; a rise clears it at the
    // rise (BOR0: `tRSTTEMPO` later) unless it falls again first, when the
    // interval goes on (`AET-WO-2301` adversarial: a stale release).
    let mut intervals: Vec<(u64, Option<u64>, usize)> = Vec::new();
    for (i, e) in edges.iter_mut().enumerate() {
        e.sort();
        let delay = if active[i].name == BOR[0].name {
            T_RSTTEMPO_NS
        } else {
            0
        };
        let mut open: Option<u64> = None;
        let mut clear: Option<u64> = None;
        for &(at, falls) in e.iter() {
            match (falls, open, clear) {
                (true, None, _) => open = Some(at),
                (true, Some(_), Some(c)) if at > c => {
                    intervals.push((open.take().expect("open"), Some(c), i));
                    open = Some(at);
                    clear = None;
                }
                (true, Some(_), _) => clear = None,
                (false, Some(_), _) => {
                    clear = Some(at.checked_add(delay).ok_or("a release time overflows")?);
                }
                (false, None, _) => {}
            }
        }
        if let Some(start) = open {
            intervals.push((start, clear, i));
        }
    }
    // The part is held while any threshold is low: the union of the intervals.
    intervals.sort_by_key(|&(start, end, i)| (start, end.is_none(), end, i));
    let mut out: Vec<Reset> = Vec::new();
    for (start, end, i) in intervals {
        let name = active[i].name.to_owned();
        match out.last_mut() {
            Some(r) if r.released_ns.is_none_or(|rel| start <= rel) => {
                r.released_ns = match (r.released_ns, end) {
                    (Some(a), Some(b)) => Some(a.max(b)),
                    _ => None,
                };
                if !r.cause.contains(&name) {
                    r.cause.push(name);
                }
            }
            _ => out.push(Reset {
                asserted_ns: start,
                released_ns: end,
                cause: vec![name],
            }),
        }
    }
    for r in &mut out {
        r.cause.sort();
    }
    Ok(out)
}

/// Where a level lies against a threshold edge's band: below the minimum,
/// inside [min, max], or above the maximum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Below,
    Inside,
    Above,
}

pub fn band(uv: u64, edge: [u64; 3]) -> Band {
    if uv < edge[0] {
        Band::Below
    } else if uv <= edge[2] {
        Band::Inside
    } else {
        Band::Above
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(level: &str, uv: u64, at_ps: u64, falling: bool) -> Crossing {
        Crossing {
            level: level.to_owned(),
            microvolts: uv,
            at_ps,
            falling,
        }
    }

    /// `AET-WO-2205` §1.7: Rev 10 changed no value. Rev 7's Table 26 as
    /// `AET-WO-2301` read it, written out again here, not taken from [`BOR`].
    #[test]
    fn every_rev_10_value_equals_rev_7s() {
        let rev7: [(&str, [u64; 3], [u64; 3]); 4] = [
            (
                "VBOR0",
                [1_580_000, 1_620_000, 1_680_000],
                [1_620_000, 1_670_000, 1_710_000],
            ),
            (
                "VBOR1",
                [1_950_000, 2_000_000, 2_060_000],
                [2_040_000, 2_100_000, 2_150_000],
            ),
            (
                "VBOR2",
                [2_250_000, 2_310_000, 2_370_000],
                [2_340_000, 2_410_000, 2_470_000],
            ),
            (
                "VBOR3",
                [2_540_000, 2_610_000, 2_680_000],
                [2_630_000, 2_700_000, 2_780_000],
            ),
        ];
        assert_eq!(BOR.len(), rev7.len());
        for (t, (name, falling, rising)) in BOR.iter().zip(rev7) {
            assert_eq!((t.name, t.falling_uv, t.rising_uv), (name, falling, rising));
        }
        assert_eq!(T_RSTTEMPO_NS, 377_000);
        assert_eq!(
            (SOURCE.revision, SOURCE.date),
            (10, "January 2026"),
            "the table's source is Rev 10"
        );
    }

    #[test]
    fn the_table_is_ordered() {
        for t in BOR {
            assert!(t.falling_uv[0] < t.falling_uv[1] && t.falling_uv[1] < t.falling_uv[2]);
            assert!(t.rising_uv[0] < t.rising_uv[1] && t.rising_uv[1] < t.rising_uv[2]);
            assert!(t.falling() < t.rising(), "{}: hysteresis", t.name);
        }
        assert!(BOR.windows(2).all(|w| w[0].rising() < w[1].falling()));
        assert_eq!(levels(3).len(), 4);
        assert_eq!(levels(0).len(), 2);
    }

    #[test]
    fn bor3_measured_crossings_give_the_measured_reset() {
        // AET-WO-2301 P2: the bor3 case, ngspice's crossings.
        let r = schedule(
            3,
            &[
                c("VBOR3_rising", 2_700_000, 1_012_422_222, true),
                c("VBOR3_falling", 2_610_000, 1_014_422_222, true),
                c("VBOR3_falling", 2_610_000, 1_225_977_778, false),
                c("VBOR3_rising", 2_700_000, 1_227_977_778, false),
            ],
        )
        .unwrap();
        assert_eq!(
            r,
            [Reset {
                asserted_ns: 1_014_430,
                released_ns: Some(1_227_980),
                cause: vec!["VBOR3".to_owned()],
            }]
        );
    }

    #[test]
    fn bor0_releases_trsttempo_after_its_rise_and_holds_past_bor3() {
        // The pdr case at BOR level 3: both thresholds go low; BOR0 clears last.
        let r = schedule(
            3,
            &[
                c("VBOR3_falling", 2_610_000, 1_006_295_238, true),
                c("VBOR0_falling", 1_620_000, 1_015_723_810, true),
                c("VBOR0_rising", 1_670_000, 1_225_152_381, false),
                c("VBOR3_rising", 2_700_000, 1_234_961_905, false),
            ],
        )
        .unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].asserted_ns, 1_006_300);
        assert_eq!(r[0].released_ns, Some(1_225_160 + T_RSTTEMPO_NS));
        assert_eq!(r[0].cause, ["VBOR0", "VBOR3"]);
    }

    #[test]
    fn a_rail_that_never_recovers_holds_the_reset() {
        let r = schedule(0, &[c("VBOR0_falling", 1_620_000, 5_000_000, true)]).unwrap();
        assert_eq!(r[0].released_ns, None);
    }

    #[test]
    fn unknown_or_wrong_levels_are_refused() {
        assert!(schedule(3, &[c("VBOR2_falling", 2_310_000, 1, true)]).is_err());
        assert!(schedule(3, &[c("VBOR3_falling", 2_600_000, 1, true)]).is_err());
    }

    #[test]
    fn the_grid_rounds_up() {
        assert_eq!(on_grid(1_014_422_222), 1_014_430);
        assert_eq!(on_grid(1_014_420_000), 1_014_420);
        assert_eq!(on_grid(1_014_420_001), 1_014_430);
        assert_eq!(band(2_350_000, BOR[3].falling_uv), Band::Below);
        assert_eq!(band(2_600_000, BOR[3].falling_uv), Band::Inside);
        assert_eq!(band(2_680_001, BOR[3].falling_uv), Band::Above);
    }
}
