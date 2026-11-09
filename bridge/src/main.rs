//! `aethelgard-bridge <request.json>`: measures each I2C line's 30 %–70 % rise
//! time with libngspice in this process (`RUL-17`, `AET-WO-2200` §1.3). The
//! CLI runs it as a sandboxed tool, beside its own executable.
//!
//! For each line of the request, in order, stdout gets one JSON line, one of
//! three outcomes of the design (ruled F7 (b) on PR #70):
//! - risen: `{"method":"interpolated","net":…,"ngspice":"44.2","rise_30_70_ns":…}`;
//! - still rising when the window ended:
//!   `{"complete":false,"method":"interpolated","net":…,"ngspice":"44.2","rise_30_70_ns_lower_bound":…}`;
//! - never low when released: `{"method":"interpolated","net":…,"ngspice":"44.2","volts_at_release":…}`;
//!
//! or, when the bridge itself fails on it, `{"error":<kind>,"message":…,"net":…}`.
//! A refused request or an unusable library gets one line, `{"error":…,"message":…}`.
//! Exit: 0 every line has an outcome; 1 the bridge failed on a line; 2 the
//! request refused; 3 the library unusable.
//!
//! The outcome is the bridge's own interpolation of the points ngspice
//! accepted, and ngspice's `meas` must agree: the same rise to 0.01 ns, or no
//! rise at all. Disagreement is an error.

#![deny(unsafe_code)]

#[allow(unsafe_code)]
mod ffi;
#[cfg(test)]
mod header;
mod host;
mod measure;
mod request;
mod solver;

use std::fs::File;
use std::io::{Read, Write};
use std::process::ExitCode;

use serde_json::{Value, json};

use measure::Outcome;
use solver::{CoSimulation, NGSPICE, Ngspice};

/// The largest disagreement allowed between the two methods, in ns.
const AGREEMENT_NS: f64 = 0.01;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let [path] = args.as_slice() else {
        return fatal(2, "usage", "usage: aethelgard-bridge <request.json>");
    };
    let lines = match read(path).and_then(|b| request::parse(&b)) {
        Ok(lines) => lines,
        Err(e) => return fatal(2, "request_refused", &e),
    };
    let mut spice = match Ngspice::start() {
        Ok(s) => s,
        Err(e) => return fatal(3, "ngspice_unusable", &e),
    };
    let mut all = true;
    for line in &lines {
        let record = match measure_line(&mut spice, line) {
            Ok(outcome) => record(&line.net, outcome),
            Err((kind, message)) => {
                all = false;
                json!({ "net": line.net, "error": kind, "message": message })
            }
        };
        if emit(&record).is_err() {
            return ExitCode::from(3);
        }
        // ngspice cannot be trusted after it asked to exit.
        if host::exit().is_some() {
            return ExitCode::from(3);
        }
    }
    ExitCode::from(if all { 0 } else { 1 })
}

fn read(path: &std::ffi::OsStr) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|e| format!("cannot open the request: {e}"))?;
    let mut bytes = Vec::new();
    file.take(request::MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("cannot read the request: {e}"))?;
    if bytes.len() as u64 > request::MAX_BYTES {
        return Err(format!("the request is over {} bytes", request::MAX_BYTES));
    }
    Ok(bytes)
}

/// The line the CLI reads for one outcome; every time is in ns.
fn record(net: &str, outcome: Outcome) -> Value {
    let mut v = json!({ "net": net, "method": "interpolated", "ngspice": NGSPICE });
    let fields = v.as_object_mut().expect("an object");
    match outcome {
        Outcome::Complete { seconds, .. } => {
            fields.insert("rise_30_70_ns".into(), json!(seconds * 1e9));
        }
        Outcome::Incomplete { lower_bound, .. } => {
            fields.insert("complete".into(), json!(false));
            fields.insert("rise_30_70_ns_lower_bound".into(), json!(lower_bound * 1e9));
        }
        Outcome::StuckHigh { volts } => {
            fields.insert("volts_at_release".into(), json!(volts));
        }
    }
    v
}

fn measure_line(
    spice: &mut Ngspice,
    line: &request::Line,
) -> Result<Outcome, (&'static str, String)> {
    let solver = |e: String| ("solver_failed", e);
    spice.instantiate(&line.netlist).map_err(solver)?;
    spice
        .set_input(&line.source, line.stimulus.clone())
        .map_err(solver)?;
    let step = spice.do_step(0.0, line.stop).map_err(solver)?;
    if step.early_return {
        return Err((
            "early_return",
            format!(
                "the transient stopped at {} s, before {} s",
                step.last_successful_time, line.stop
            ),
        ));
    }
    let time = spice.get_output("time").map_err(solver)?;
    let v = spice.get_output(&line.probe).map_err(solver)?;
    let own = measure::rise(&time, &v, line.release, line.low, line.high)
        .map_err(|e| (e.kind(), e.message()))?;
    let meas = spice
        .meas(
            "aethelgard_rise",
            &line.probe,
            &decimal(line.low_uv, 6),
            &decimal(line.high_uv, 6),
            &format!("{}p", line.release_ps),
        )
        .map_err(solver)?;
    agreed(own, meas)
}

/// The interpolated outcome, when ngspice's `meas` (in seconds) agrees: the
/// same rise within [`AGREEMENT_NS`], or, for a line that did not rise, none.
fn agreed(own: Outcome, meas: Option<f64>) -> Result<Outcome, (&'static str, String)> {
    let Outcome::Complete { seconds, .. } = own else {
        return match meas {
            None => Ok(own),
            Some(m) => Err((
                "methods_disagree",
                format!(
                    "the interpolation found no rise ({own:?}) and ngspice's meas {} ns",
                    m * 1e9
                ),
            )),
        };
    };
    let own_ns = seconds * 1e9;
    let Some(meas_s) = meas else {
        return Err((
            "methods_disagree",
            format!("ngspice's meas found no rise; the interpolation found {own_ns} ns"),
        ));
    };
    let meas_ns = meas_s * 1e9;
    // Written so that a NaN on either side disagrees.
    let agree = (own_ns - meas_ns).abs() <= AGREEMENT_NS;
    if !agree {
        return Err((
            "methods_disagree",
            format!("the interpolation gives {own_ns} ns and ngspice's meas {meas_ns} ns"),
        ));
    }
    Ok(own)
}

/// `value / 10^scale` as a decimal with no trailing zeros: 990000 µV is `0.99`.
fn decimal(value: u64, scale: u32) -> String {
    let unit = 10u64.pow(scale);
    let (whole, frac) = (value / unit, value % unit);
    if frac == 0 {
        return whole.to_string();
    }
    let digits = format!("{frac:0width$}", width = scale as usize);
    format!("{whole}.{}", digits.trim_end_matches('0'))
}

fn emit(record: &Value) -> std::io::Result<()> {
    let mut out = std::io::stdout().lock();
    writeln!(out, "{record}")?;
    out.flush()
}

fn fatal(code: u8, kind: &str, message: &str) -> ExitCode {
    let _ = emit(&json!({ "error": kind, "message": message }));
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_two_methods_must_agree_to_0_01_ns_or_both_find_no_rise() {
        use super::{Outcome, agreed};
        let rise = |ns: f64| Outcome::Complete {
            seconds: ns * 1e-9,
            t_low: 0.0,
        };
        assert_eq!(
            agreed(rise(186.4014), Some(186.4014e-9)),
            Ok(rise(186.4014))
        );
        assert!(agreed(rise(186.40), Some(186.409e-9)).is_ok());
        for meas in [None, Some(186.42e-9), Some(f64::NAN), Some(186.39e-9)] {
            let err = agreed(rise(186.40), meas).unwrap_err();
            assert_eq!(err.0, "methods_disagree", "{meas:?}");
        }
        assert!(agreed(rise(f64::NAN), Some(186.40e-9)).is_err());
        for none in [
            Outcome::StuckHigh { volts: 1.65 },
            Outcome::Incomplete {
                lower_bound: 1e-6,
                peak_volts: 2.0,
            },
        ] {
            assert_eq!(agreed(none, None), Ok(none));
            assert_eq!(
                agreed(none, Some(186.4e-9)).unwrap_err().0,
                "methods_disagree"
            );
        }
    }

    #[test]
    fn each_outcome_is_its_documented_line() {
        use super::{Outcome, record};
        let text = |o| record("N", o).to_string();
        assert_eq!(
            text(Outcome::Complete {
                seconds: 1.5e-9,
                t_low: 0.0
            }),
            r#"{"method":"interpolated","net":"N","ngspice":"44.2","rise_30_70_ns":1.5}"#
        );
        assert_eq!(
            text(Outcome::Incomplete {
                lower_bound: 2e-6,
                peak_volts: 2.0
            }),
            r#"{"complete":false,"method":"interpolated","net":"N","ngspice":"44.2","rise_30_70_ns_lower_bound":2000.0}"#
        );
        assert_eq!(
            text(Outcome::StuckHigh { volts: 1.65 }),
            r#"{"method":"interpolated","net":"N","ngspice":"44.2","volts_at_release":1.65}"#
        );
    }

    #[test]
    fn decimals_carry_no_trailing_zero() {
        assert_eq!(super::decimal(990_000, 6), "0.99");
        assert_eq!(super::decimal(2_310_000, 6), "2.31");
        assert_eq!(super::decimal(3_000_000, 6), "3");
        assert_eq!(super::decimal(999_900, 6), "0.9999");
    }
}
