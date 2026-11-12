//! `prove`: `cargo kani --harness <name>` on a copy of the crate. Two
//! witnesses: the exit code and the output lines Kani prints for each outcome
//! (measured in the AET-WO-0300 spike, S5). Exit 1 alone cannot tell a failed
//! proof from a compile error or a missing harness, so the lines decide which,
//! and any combination not in the table is a `ToolError`.
//!
//! A harness's `kani::cover!` statements are its non-vacuity witnesses
//! (`AET-WO-1400`, adversarial E1): an `assume` that cuts every path makes
//! every assertion hold, and Kani still exits 0. So on exit 0, every cover
//! property Kani reports must be satisfied, or the verdict is a fail.

use crate::run::{Output, Stop};
use crate::said::{self, Style};
use crate::verdict::Finding;

const TOOL: &str = "cargo-kani";

/// `(satisfied, total)` from `** S of T cover properties satisfied`, with or
/// without a parenthesised remainder such as `(1 unreachable)`.
fn covers(line: &str) -> Option<(u32, u32)> {
    let rest = line.strip_prefix("** ")?;
    let (satisfied, rest) = rest.split_once(" of ")?;
    let (total, rest) = rest.split_once(" cover properties satisfied")?;
    if !(rest.is_empty() || rest.starts_with(" (")) {
        return None;
    }
    Some((satisfied.parse().ok()?, total.parse().ok()?))
}

/// `(successes, failures)` from `Complete - N successfully verified harnesses, M failures, T total.`
fn complete(line: &str) -> Option<(u32, u32)> {
    let rest = line.strip_prefix("Complete - ")?;
    let (ok, rest) = rest.split_once(" successfully verified harnesses, ")?;
    let (failed, _) = rest.split_once(" failures")?;
    Some((ok.parse().ok()?, failed.parse().ok()?))
}

pub fn judge(code: i32, out: &Output, harness: &str) -> Result<Vec<Finding>, Stop> {
    let text = out.text();
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let has = |l: &str| lines.contains(&l);
    let succeeded = has("VERIFICATION:- SUCCESSFUL");
    let failed = has("VERIFICATION:- FAILED");
    let totals: Vec<(u32, u32)> = lines.iter().filter_map(|l| complete(l)).collect();
    let cover_lines: Vec<(&str, (u32, u32))> = lines
        .iter()
        .filter_map(|l| covers(l).map(|c| (*l, c)))
        .collect();
    let compile_errors = lines.iter().any(|l| {
        l.contains("compilation errors") && l.starts_with("error: Failed to execute cargo")
    });
    let no_harness = lines
        .iter()
        .any(|l| l.starts_with("error: Failed to match the following harness(es)"));
    let disagree = |why: &str| {
        let said = said::suffix(&text, Style::Cargo);
        Stop::tool_error(TOOL, format!("cargo kani exited {code} but {why}{said}"))
    };

    // A third witness: `--harness` is a substring filter, so the harnesses
    // Kani says it checked must be exactly the one asked for.
    let checked: Vec<&str> = lines
        .iter()
        .filter_map(|l| l.strip_prefix("Checking harness ")?.strip_suffix("..."))
        .collect();
    let suffix = format!("::{harness}");
    let exact = |name: &&str| *name == harness || name.ends_with(&suffix);
    if (code == 0 || code == 1) && !checked.is_empty() {
        if !checked.iter().any(exact) {
            return Ok(vec![Finding::plain(
                "harness_not_found",
                format!(
                    "no harness is named `{harness}`; it only matched {}",
                    checked.join(", ")
                ),
            )]);
        }
        if checked.len() > 1 {
            return Ok(vec![Finding::plain(
                "harness_ambiguous",
                format!(
                    "`{harness}` matches {}; name one by its full path",
                    checked.join(", ")
                ),
            )]);
        }
    }

    match code {
        0 => {
            let complete_ok = totals == [(1, 0)];
            if succeeded
                && !failed
                && complete_ok
                && checked.len() == 1
                && !compile_errors
                && !no_harness
            {
                match cover_lines.as_slice() {
                    [] => Ok(Vec::new()),
                    [(_, (satisfied, total))] if satisfied == total => Ok(Vec::new()),
                    [(line, _)] => Ok(vec![Finding::plain(
                        "cover_unsatisfied",
                        format!(
                            "{line}: a path the harness covers is never reached, so its proof may be vacuous"
                        ),
                    )]),
                    _ => Err(disagree(
                        "it printed more than one cover summary for one harness",
                    )),
                }
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
            format!(
                "cargo kani exited {other}, which is neither 0 nor 1{}",
                said::suffix(&text, Style::Cargo)
            ),
        )),
    }
}

/// The cover properties Kani reported for the harness, from its cover
/// summary; 0 when it printed none (`AET-WO-3301` §1.4). [`judge`] has
/// already refused more than one summary.
pub fn cover_count(out: &Output) -> u32 {
    out.text()
        .lines()
        .map(str::trim)
        .find_map(covers)
        .map_or(0, |(_, total)| total)
}
