//! The 30 %–70 % rise time, by the bridge's own linear interpolation of the
//! points ngspice accepted (`AET-WO-2101` S2). The edge is the first rise
//! after the driver releases the line, and the line must be below 30 % when it
//! does: a crossing taken from any other edge, or from a line that never fell,
//! is refused rather than reported.

/// Why no rise time was measured.
#[derive(Debug, Clone, PartialEq)]
pub enum NoRise {
    /// The vectors are empty, of different lengths, not finite, or not in time order.
    Malformed(String),
    /// No accepted point at or after the release.
    NoRelease,
    /// The line was at or above 30 % when the driver released it.
    NotLowAtRelease { volts: f64 },
    /// The line did not reach the threshold before the transient ended.
    Incomplete {
        threshold: &'static str,
        peak_volts: f64,
    },
}

impl NoRise {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Malformed(_) => "vectors_malformed",
            Self::NoRelease => "no_point_after_release",
            Self::NotLowAtRelease { .. } => "line_not_low_at_release",
            Self::Incomplete { .. } => "rise_incomplete",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Malformed(m) => m.clone(),
            Self::NoRelease => "no accepted point at or after the release".to_owned(),
            Self::NotLowAtRelease { volts } => {
                format!("the line is at {volts} V when the driver releases it, not below 30 %")
            }
            Self::Incomplete {
                threshold,
                peak_volts,
            } => format!(
                "the line never reaches {threshold} after the release; its peak is {peak_volts} V"
            ),
        }
    }
}

/// The rise in seconds, with the two crossing times.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rise {
    pub seconds: f64,
    pub t_low: f64,
    pub t_high: f64,
}

pub fn rise(time: &[f64], v: &[f64], release: f64, low: f64, high: f64) -> Result<Rise, NoRise> {
    if time.len() != v.len() || time.len() < 2 {
        return Err(NoRise::Malformed(format!(
            "time has {} points and the line {}",
            time.len(),
            v.len()
        )));
    }
    if !time.iter().chain(v).all(|x| x.is_finite()) {
        return Err(NoRise::Malformed("a point is not finite".to_owned()));
    }
    if time.windows(2).any(|w| w[1] < w[0]) {
        return Err(NoRise::Malformed("time goes backwards".to_owned()));
    }
    let start = time
        .iter()
        .position(|&t| t >= release)
        .ok_or(NoRise::NoRelease)?;
    if v[start] >= low {
        return Err(NoRise::NotLowAtRelease { volts: v[start] });
    }
    let peak = v[start..].iter().copied().fold(f64::MIN, f64::max);
    let (k, t_low) = crossing(time, v, low, start).ok_or(NoRise::Incomplete {
        threshold: "30 %",
        peak_volts: peak,
    })?;
    let (_, t_high) = crossing(time, v, high, k).ok_or(NoRise::Incomplete {
        threshold: "70 %",
        peak_volts: peak,
    })?;
    Ok(Rise {
        seconds: t_high - t_low,
        t_low,
        t_high,
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
        let r = rise(&t, &v, 2.5e-6, 0.99, 2.31).unwrap();
        assert!((r.seconds * 1e9 - 186.40).abs() < 0.05, "{r:?}");
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
        let r = rise(&t, &v, 2.5e-6, 0.99, 2.31).unwrap();
        assert!(r.t_low > 2.5e-6);
    }

    #[test]
    fn refusals() {
        let (t, v) = rc(220e-9, 2.5e-6, 2001, 22.5e-6);
        let high: Vec<f64> = v.iter().map(|_| 3.3).collect();
        assert_eq!(
            rise(&t, &high, 2.5e-6, 0.99, 2.31).unwrap_err().kind(),
            "line_not_low_at_release"
        );
        let slow = rc(1e-3, 2.5e-6, 2001, 22.5e-6);
        assert_eq!(
            rise(&slow.0, &slow.1, 2.5e-6, 0.99, 2.31)
                .unwrap_err()
                .kind(),
            "rise_incomplete"
        );
        let mut nan = v.clone();
        nan[5] = f64::NAN;
        assert_eq!(
            rise(&t, &nan, 2.5e-6, 0.99, 2.31).unwrap_err().kind(),
            "vectors_malformed"
        );
        assert_eq!(
            rise(&t, &v[1..], 2.5e-6, 0.99, 2.31).unwrap_err().kind(),
            "vectors_malformed"
        );
        assert_eq!(
            rise(&t, &v, 30e-6, 0.99, 2.31).unwrap_err().kind(),
            "no_point_after_release"
        );
    }
}
