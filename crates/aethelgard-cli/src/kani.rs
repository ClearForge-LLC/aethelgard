//! `prove`: `cargo kani --harness <name>` on a copy of the crate. Two
//! witnesses: the exit code and the output lines Kani prints for each outcome
//! (measured in the AET-WO-0300 spike, S5). Exit 1 alone cannot tell a failed
//! proof from a compile error or a missing harness, so the lines decide which,
//! and any combination not in the table is a `ToolError`.

use crate::run::{Output, Stop};
use crate::verdict::{Finding, code_of};

const TOOL: &str = "cargo-kani";

/// `(successes, failures)` from `Complete - N successfully verified harnesses, M failures, T total.`
fn complete(line: &str) -> Option<(u32, u32)> {
    let rest = line.strip_prefix("Complete - ")?;
    let (ok, rest) = rest.split_once(" successfully verified harnesses, ")?;
    let (failed, _) = rest.split_once(" failures")?;
    Some((ok.parse().ok()?, failed.parse().ok()?))
}

pub fn judge(out: &Output, harness: &str) -> Result<Vec<Finding>, Stop> {
    let code = code_of(out.ended)
        .ok_or_else(|| Stop::tool_error(TOOL, format!("cargo kani was killed: {:?}", out.ended)))?;
    let text = out.text();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let has = |l: &str| lines.contains(&l);
    let succeeded = has("VERIFICATION:- SUCCESSFUL");
    let failed = has("VERIFICATION:- FAILED");
    let totals: Vec<(u32, u32)> = lines.iter().filter_map(|l| complete(l)).collect();
    let compile_errors = lines.iter().any(|l| {
        l.contains("compilation errors") && l.starts_with("error: Failed to execute cargo")
    });
    let no_harness = lines
        .iter()
        .any(|l| l.starts_with("error: Failed to match the following harness(es)"));
    let disagree =
        |why: &str| Stop::tool_error(TOOL, format!("cargo kani exited {code} but {why}"));

    match code {
        0 => {
            let complete_ok = matches!(totals.as_slice(), [(n, 0)] if *n >= 1);
            if succeeded && !failed && complete_ok && !compile_errors && !no_harness {
                Ok(Vec::new())
            } else {
                Err(disagree(
                    "its output does not show one successful verification and no failure",
                ))
            }
        }
        1 => match (failed, compile_errors, no_harness) {
            (true, false, false)
                if !succeeded && matches!(totals.as_slice(), [(_, f)] if *f >= 1) =>
            {
                let checks: Vec<Finding> = lines
                    .iter()
                    .filter_map(|l| l.strip_prefix("Failed Checks: "))
                    .map(|m| Finding::plain("proof_failed", m.to_owned()))
                    .collect();
                Ok(if checks.is_empty() {
                    vec![Finding::plain(
                        "proof_failed",
                        format!("harness `{harness}`: VERIFICATION:- FAILED"),
                    )]
                } else {
                    checks
                })
            }
            (false, true, false) if !succeeded => {
                let first = lines
                    .iter()
                    .find(|l| l.starts_with("error[") || l.starts_with("error: could not compile"));
                let summary = lines
                    .iter()
                    .find(|l| l.contains("compilation errors"))
                    .copied()
                    .unwrap_or_default();
                Ok(vec![Finding::plain(
                    "compile_error",
                    first.copied().unwrap_or(summary).to_owned(),
                )])
            }
            (false, false, true) if !succeeded => Ok(vec![Finding::plain(
                "harness_not_found",
                format!("no harness matches `{harness}`"),
            )]),
            _ => Err(disagree(
                "its output matches none, or more than one, of: failed proof, compile error, missing harness",
            )),
        },
        other => Err(Stop::tool_error(
            TOOL,
            format!("cargo kani exited {other}, which is neither 0 nor 1"),
        )),
    }
}
