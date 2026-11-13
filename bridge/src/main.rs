//! Every invocation prints the identity line first, `{"bridge":"aethelgard-bridge",
//! "source_sha256":…,"version":…}`, and flushes it: `source_sha256` is the digest
//! of the bridge's own source that `build.rs` computed (its header gives the
//! recipe), so the CLI can refuse a bridge that is not the one it was built with.
//!
//! `aethelgard-bridge --rail <request.json>` solves one declared supply droop
//! (see `rail`); `aethelgard-bridge --renode <executable> [args…]` runs
//! Renode as the bridge's child, with the bridge's own stdin, stdout, stderr and
//! environment, and exits as it exits (128 + the signal, if it was killed).
//!
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
mod rail;
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

/// How far ngspice's `meas … WHEN` may lie from the interpolation, in ns, at
/// a crossing `seconds` into the run: ngspice prints it to 7 significant
/// digits (`AET-WO-2301` adversarial F3: 1,014,422.000 ns where the accepted
/// points give 1,014,422.222, and 20,014,420 where they give 20,014,422.2), so
/// half a unit of the 7th digit, plus the I2C path's 0.01 ns.
fn rail_agreement_ns(seconds: f64) -> f64 {
    let unit = 10f64.powi(seconds.abs().log10().floor() as i32 - 6);
    unit / 2.0 * 1e9 + AGREEMENT_NS
}

/// The first line of stdout in every mode.
fn identity() -> String {
    json!({
        "bridge": "aethelgard-bridge",
        "source_sha256": env!("AETHELGARD_BRIDGE_SOURCE_SHA256"),
        "version": env!("CARGO_PKG_VERSION"),
    })
    .to_string()
}

const USAGE: &str = "usage: aethelgard-bridge <request.json> | --rail <request.json> | --renode <absolute executable> [args...]";

fn main() -> ExitCode {
    if emit_text(&identity()).is_err() {
        return ExitCode::from(3);
    }
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.first().and_then(|a| a.to_str()) {
        Some("--rail") => {
            let [_, path] = args.as_slice() else {
                return fatal(2, "usage", USAGE);
            };
            return run_rail(path);
        }
        Some("--renode") => return run_renode(&args[1..]),
        _ => {}
    }
    let [path] = args.as_slice() else {
        return fatal(2, "usage", USAGE);
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

/// A refusal of the request is exit 2, an unusable library 3, a failure of
/// ngspice or of the agreement between the two methods 1.
fn run_rail(path: &std::ffi::OsStr) -> ExitCode {
    let rail = match read(path).and_then(|b| rail::parse(&b)) {
        Ok(rail) => rail,
        Err(e) => return fatal(2, "request_refused", &e),
    };
    let mut spice = match Ngspice::start() {
        Ok(s) => s,
        Err(e) => return fatal(3, "ngspice_unusable", &e),
    };
    let lines = match solve_rail(&mut spice, &rail) {
        Ok(lines) => lines,
        Err((kind, message)) => return fatal(1, kind, &message),
    };
    if host::exit().is_some() {
        return ExitCode::from(3);
    }
    for line in &lines {
        if emit(line).is_err() {
            return ExitCode::from(3);
        }
    }
    ExitCode::SUCCESS
}

/// Every output line of a solved rail, none of them written until ngspice's
/// `meas` has agreed with the interpolation on every crossing.
fn solve_rail(
    spice: &mut Ngspice,
    rail: &rail::Rail,
) -> Result<Vec<Value>, (&'static str, String)> {
    let solver = |e: String| ("solver_failed", e);
    let span = rail.span_ns as f64 * 1e-9;
    spice.instantiate(&rail.deck()).map_err(solver)?;
    let step = spice.do_step(0.0, span).map_err(solver)?;
    if step.early_return {
        return Err((
            "early_return",
            format!(
                "the transient stopped at {} s, before {span} s",
                step.last_successful_time
            ),
        ));
    }
    let time = spice.get_output("time").map_err(solver)?;
    let v = spice.get_output("rail").map_err(solver)?;
    if time.len() != v.len() || v.is_empty() || v.iter().any(|x| !x.is_finite()) {
        return Err((
            "solver_failed",
            format!("{} times and {} values", time.len(), v.len()),
        ));
    }
    let mut found = Vec::new();
    let mut counter = 0;
    for (i, level) in rail.levels.iter().enumerate() {
        let volts = level.microvolts as f64 / 1e6;
        let text = decimal(level.microvolts, 6);
        let all = rail::crossings(&time, &v, volts, i);
        for falling in [true, false] {
            let mine: Vec<_> = all.iter().filter(|c| c.falling == falling).collect();
            // The k-th crossing of each direction, then one more than there are.
            for k in 1..=mine.len() + 1 {
                counter += 1;
                let name = format!("aethelgard_w{counter}");
                let meas = spice
                    .meas_when(&name, "rail", &text, falling, k)
                    .map_err(solver)?;
                let dir = if falling { "falling" } else { "rising" };
                let Some(own) = mine.get(k - 1) else {
                    if let Some(m) = meas {
                        return Err((
                            "methods_disagree",
                            format!(
                                "`{}`: the interpolation found {} {dir} crossings and ngspice's meas a {k}th at {} ns",
                                level.name,
                                mine.len(),
                                m * 1e9
                            ),
                        ));
                    }
                    continue;
                };
                // Written so that a NaN on either side disagrees.
                let agree =
                    meas.is_some_and(|m| (own.time - m).abs() * 1e9 <= rail_agreement_ns(own.time));
                if !agree {
                    return Err((
                        "methods_disagree",
                        format!(
                            "`{}`, {dir} crossing {k}: the interpolation gives {} ns and ngspice's meas {meas:?} s",
                            level.name,
                            own.time * 1e9
                        ),
                    ));
                }
            }
        }
        found.extend(all);
    }
    found.sort_by(|a, b| a.time.total_cmp(&b.time).then(a.level.cmp(&b.level)));
    let mut lines: Vec<Value> = found
        .iter()
        .map(|c| {
            let level = &rail.levels[c.level];
            json!({
                "at_ps": rail::picoseconds(c.time),
                "falling": c.falling,
                "level": level.name,
                "method": "interpolated",
                "microvolts": level.microvolts,
                "ngspice": NGSPICE,
            })
        })
        .collect();
    let min = v.iter().copied().fold(f64::INFINITY, f64::min);
    lines.push(json!({
        "accepted_points": v.len(),
        "minimum_microvolts": (min * 1e6).round() as i64,
        "rail": "solved",
    }));
    Ok(lines)
}

/// Renode is the bridge's child: the same stdin, stdout, stderr and
/// environment, with no `/dev/null` anywhere, which the CLI's sandbox does not
/// grant. The bridge neither reads nor alters what the child prints.
fn run_renode(args: &[std::ffi::OsString]) -> ExitCode {
    use std::os::unix::process::ExitStatusExt;

    let Some((exe, rest)) = args.split_first() else {
        return fatal(2, "usage", USAGE);
    };
    if !std::path::Path::new(exe).is_absolute() {
        return fatal(
            2,
            "request_refused",
            "the executable is not an absolute path",
        );
    }
    match std::process::Command::new(exe).args(rest).status() {
        Ok(status) => match (status.code(), status.signal()) {
            (Some(code), _) => ExitCode::from(code as u8),
            (None, Some(signal)) => ExitCode::from((128 + signal) as u8),
            (None, None) => ExitCode::from(1),
        },
        // As a shell's "cannot execute": distinct from a refusal or a library fault.
        Err(e) => fatal(
            126,
            "spawn_failed",
            &format!("cannot run the executable: {e}"),
        ),
    }
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
    emit_text(&record.to_string())
}

fn emit_text(line: &str) -> std::io::Result<()> {
    let mut out = std::io::stdout().lock();
    writeln!(out, "{line}")?;
    out.flush()
}

fn fatal(code: u8, kind: &str, message: &str) -> ExitCode {
    let _ = emit(&json!({ "error": kind, "message": message }));
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    /// `AET-WO-2301` adversarial F3: `meas` prints 7 significant digits, so
    /// the tolerance is half a unit of the 7th digit at the crossing's time.
    #[test]
    fn the_rail_agreement_follows_meas_precision() {
        let ns = super::rail_agreement_ns;
        assert!((ns(1.014e-3) - 0.51).abs() < 1e-9);
        assert!((ns(20.01e-3) - 5.01).abs() < 1e-9);
        assert!((ns(99.9e-3) - 5.01).abs() < 1e-9);
    }

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
    fn the_identity_line_is_the_documented_one() {
        let line = super::identity();
        let (head, rest) =
            line.split_at(r#"{"bridge":"aethelgard-bridge","source_sha256":""#.len());
        assert_eq!(head, r#"{"bridge":"aethelgard-bridge","source_sha256":""#);
        let (hex, tail) = rest.split_at(64);
        assert!(
            hex.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
        assert_eq!(
            tail,
            format!(r#"","version":"{}"}}"#, env!("CARGO_PKG_VERSION"))
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
