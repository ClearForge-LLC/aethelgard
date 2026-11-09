//! `aethelgard-bridge <request.json>`: measures each I2C line's 30 %–70 % rise
//! time with libngspice in this process (`RUL-17`, `AET-WO-2200` §1.3). The
//! CLI runs it as a sandboxed tool, beside its own executable.
//!
//! For each line of the request, in order, stdout gets one JSON line:
//! - measured: `{"method":"interpolated","net":…,"ngspice":"44.2","rise_30_70_ns":…}`;
//! - not measured: `{"error":<kind>,"message":…,"net":…}`.
//!
//! A refused request or an unusable library gets one line, `{"error":…,"message":…}`.
//! Exit: 0 every line measured; 1 a line not measured; 2 the request refused;
//! 3 the library unusable.
//!
//! The rise is the bridge's own interpolation of the points ngspice accepted,
//! and it must agree with ngspice's `meas` to 0.01 ns; disagreement is an error.

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
            Ok(ns) => json!({
                "net": line.net,
                "rise_30_70_ns": ns,
                "method": "interpolated",
                "ngspice": NGSPICE,
            }),
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

fn measure_line(spice: &mut Ngspice, line: &request::Line) -> Result<f64, (&'static str, String)> {
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
    let own_ns = own.seconds * 1e9;
    let meas = spice
        .meas(
            "aethelgard_rise",
            &line.probe,
            &decimal(line.low_uv, 6),
            &decimal(line.high_uv, 6),
            &format!("{}p", line.release_ps),
        )
        .map_err(solver)?;
    agreed(own_ns, meas)
}

/// The interpolated rise, when ngspice's `meas` (in seconds) found one
/// within [`AGREEMENT_NS`] of it.
fn agreed(own_ns: f64, meas: Option<f64>) -> Result<f64, (&'static str, String)> {
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
    Ok(own_ns)
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
    fn the_two_methods_must_agree_to_0_01_ns() {
        use super::agreed;
        assert_eq!(agreed(186.4014, Some(186.4014e-9)), Ok(186.4014));
        assert!(agreed(186.40, Some(186.409e-9)).is_ok());
        for meas in [None, Some(186.42e-9), Some(f64::NAN), Some(186.39e-9)] {
            let err = agreed(186.40, meas).unwrap_err();
            assert_eq!(err.0, "methods_disagree", "{meas:?}");
        }
        assert!(agreed(f64::NAN, Some(186.40e-9)).is_err());
    }

    #[test]
    fn decimals_carry_no_trailing_zero() {
        assert_eq!(super::decimal(990_000, 6), "0.99");
        assert_eq!(super::decimal(2_310_000, 6), "2.31");
        assert_eq!(super::decimal(3_000_000, 6), "3");
        assert_eq!(super::decimal(999_900, 6), "0.9999");
    }
}
