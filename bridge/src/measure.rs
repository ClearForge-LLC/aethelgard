//! The 30 %–70 % rise time, by the bridge's own linear interpolation of the
//! points ngspice accepted (`AET-WO-2101` S2). The edge is the first rise
//! after the driver releases the line, so a crossing on any other edge is
//! never reported. What the line does after the release is one of three
//! outcomes, each a property of the design (ruled F7 (b) on PR #70): it
//! rises, it is still rising when the window ends, or it was never low.

/// Why the vectors could not be read at all: a failure of the bridge or the
/// solver, never of the design.
#[derive(Debug, Clone, PartialEq)]
pub enum Unreadable {
    /// The vectors are empty, of different lengths, not finite, or not in time order.
    Malformed(String),
    /// No accepted point at or after the release.
    NoRelease,
}

impl Unreadable {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Malformed(_) => "vectors_malformed",
            Self::NoRelease => "no_point_after_release",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Malformed(m) => m.clone(),
            Self::NoRelease => "no accepted point at or after the release".to_owned(),
        }
    }
}

/// What the line did after the release.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// It rose from 30 % to 70 % inside the window, in `seconds`.
    Complete { seconds: f64, t_low: f64 },
    /// It had not reached 70 % when the window ended. `lower_bound` is the
    /// window remaining after its 30 % crossing, or after the release when it
    /// never reached 30 %: a charging line slows as it rises, so its 30–70 %
    /// rise takes longer than its 0–30 % rise did, which was the whole window.
    Incomplete { lower_bound: f64, peak_volts: f64 },
    /// It was at or above 30 % when the driver released it, so no rise
    /// exists to measure: the driver cannot pull it low.
    StuckHigh { volts: f64 },
}

pub fn rise(
    time: &[f64],
    v: &[f64],
    release: f64,
    low: f64,
    high: f64,
) -> Result<Outcome, Unreadable> {
    if time.len() != v.len() || time.len() < 2 {
        return Err(Unreadable::Malformed(format!(
            "time has {} points and the line {}",
            time.len(),
            v.len()
        )));
    }
    if !time.iter().chain(v).all(|x| x.is_finite()) {
        return Err(Unreadable::Malformed("a point is not finite".to_owned()));
    }
    if time.windows(2).any(|w| w[1] < w[0]) {
        return Err(Unreadable::Malformed("time goes backwards".to_owned()));
    }
    let start = time
        .iter()
        .position(|&t| t >= release)
        .ok_or(Unreadable::NoRelease)?;
    if v[start] >= low {
        return Ok(Outcome::StuckHigh { volts: v[start] });
    }
    let end = time[time.len() - 1];
    let peak_volts = v[start..].iter().copied().fold(f64::MIN, f64::max);
    let Some((k, t_low)) = crossing(time, v, low, start) else {
        return Ok(Outcome::Incomplete {
            lower_bound: end - release,
            peak_volts,
        });
    };
    let Some((_, t_high)) = crossing(time, v, high, k) else {
        return Ok(Outcome::Incomplete {
            lower_bound: end - t_low,
            peak_volts,
        });
    };
    Ok(Outcome::Complete {
        seconds: t_high - t_low,
        t_low,
    })
}

/// The first upward crossing of `level` at or after point `from`, linearly
/// interpolated between the two accepted points that bracket it.
fn crossing(time: &[f64], v: &[f64], level: f64, from: usize) -> Option<(usize, f64)> {
    (from..v.len() - 1)
        .find(|&k| v[k] < level && v[k + 1] >= level)
        .map(|k| {
            let t = time[k] + (level - v[k]) * (time[k + 1] - time[k]) / (v[k + 1] - v[k]);
            (k, t)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rc(tau: f64, release: f64, n: usize, end: f64) -> (Vec<f64>, Vec<f64>) {
        let t: Vec<f64> = (0..n).map(|i| end * i as f64 / (n - 1) as f64).collect();
        let v = t
            .iter()
            .map(|&t| {
                if t < release {
                    0.0
                } else {
                    3.3 * (1.0 - (-(t - release) / tau).exp())
                }
            })
            .collect();
        (t, v)
    }

    #[test]
    fn an_rc_rise_is_0_8473_tau() {
        let (t, v) = rc(220e-9, 2.5e-6, 200_001, 22.5e-6);
        let Outcome::Complete { seconds, .. } = rise(&t, &v, 2.5e-6, 0.99, 2.31).unwrap() else {
            panic!("not complete");
        };
        assert!((seconds * 1e9 - 186.40).abs() < 0.05, "{seconds}");
    }

    #[test]
    fn an_edge_before_the_release_is_not_measured() {
        // A full rise and fall before the release: it is ignored, and the
        // line must be low at the release.
        let (t, mut v) = rc(220e-9, 2.5e-6, 20_001, 22.5e-6);
        for (ti, vi) in t.iter().zip(v.iter_mut()) {
            if (0.5e-6..1.5e-6).contains(ti) {
                *vi = 3.3;
            }
        }
        let Outcome::Complete { t_low, .. } = rise(&t, &v, 2.5e-6, 0.99, 2.31).unwrap() else {
            panic!("not complete");
        };
        assert!(t_low > 2.5e-6);
    }

    #[test]
    fn a_line_never_low_or_still_rising_is_an_outcome_with_its_bound() {
        let (t, v) = rc(220e-9, 2.5e-6, 2001, 22.5e-6);
        let high: Vec<f64> = v.iter().map(|_| 1.65).collect();
        assert_eq!(
            rise(&t, &high, 2.5e-6, 0.99, 2.31),
            Ok(Outcome::StuckHigh { volts: 1.65 })
        );
        // Reaches 30 % but not 70 %: the bound is the window after t30.
        let (t, v) = rc(18e-6, 2.5e-6, 20_001, 22.5e-6);
        let Ok(Outcome::Incomplete { lower_bound, .. }) = rise(&t, &v, 2.5e-6, 0.99, 2.31) else {
            panic!("not incomplete");
        };
        let t30 = 2.5e-6 + 18e-6 * (1.0f64 / 0.7).ln();
        assert!(
            (lower_bound - (22.5e-6 - t30)).abs() < 1e-9,
            "{lower_bound}"
        );
        // Never reaches 30 %: the bound is the window after the release.
        let (t, v) = rc(1e-3, 2.5e-6, 2001, 22.5e-6);
        let Ok(Outcome::Incomplete { lower_bound, .. }) = rise(&t, &v, 2.5e-6, 0.99, 2.31) else {
            panic!("not incomplete");
        };
        assert!((lower_bound - 20e-6).abs() < 1e-12, "{lower_bound}");
    }

    #[test]
    fn unreadable_vectors_are_refused() {
        let (t, v) = rc(220e-9, 2.5e-6, 2001, 22.5e-6);
        let mut nan = v.clone();
        nan[5] = f64::NAN;
        let kind = |r: Result<Outcome, Unreadable>| r.unwrap_err().kind();
        assert_eq!(
            kind(rise(&t, &nan, 2.5e-6, 0.99, 2.31)),
            "vectors_malformed"
        );
        assert_eq!(
            kind(rise(&t, &v[1..], 2.5e-6, 0.99, 2.31)),
            "vectors_malformed"
        );
        assert_eq!(
            kind(rise(&t, &v, 30e-6, 0.99, 2.31)),
            "no_point_after_release"
        );
    }
}
