//! `extract`: each I2C line's capacitance bounded from above from the routed
//! board's copper, judged against the line's declared bound (`AET-WO-1305`,
//! `RUL-19` (1)). The bound and its sources are `aethelgard_eda::extract`'s.
//!
//! The board is read once, into memory; its sha256 and the bound are both
//! taken from those bytes, so the verdict names exactly what it judged. No
//! tool runs.

use std::path::Path;
use std::process::ExitCode;

use aethelgard_eda::extract::{self as bound, CONDITIONS, LineBound};
use serde::Serialize;
use serde_json::{Value, json};

use crate::verdict::InputReport;
use crate::{EXIT_FAIL, EXIT_INVALID_SPEC, EXIT_UNREADABLE, Refusal};

/// `RUL-19`, in the verdict itself.
const CAPACITANCE: &str = "upper bound estimated from routed geometry";

#[derive(Serialize)]
struct Report {
    ok: bool,
    verdict: &'static str,
    check: &'static str,
    engine_version: &'static str,
    /// The board, as read.
    input: InputReport,
    specification_sha256: String,
    capacitance: &'static str,
    conditions: &'static [&'static str],
    lines: Vec<Value>,
    findings: Vec<Finding>,
}

#[derive(Serialize)]
struct Finding {
    kind: &'static str,
    severity: &'static str,
    message: String,
}

/// One line of the verdict: the bound with its terms, and the judgement.
fn line(b: LineBound, declared: u64) -> Value {
    let t = &b.terms;
    json!({
        "net": b.net, "board_net": b.board_net, "bus": b.bus,
        "verdict": if b.c_bound_femtofarads <= declared { "pass" } else { "fail" },
        "c_bound_femtofarads": b.c_bound_femtofarads, "declared_femtofarads": declared,
        "terms": {"trace": t.trace, "vias": t.vias, "pads": t.pads, "pins": t.pins},
        "length_nm": b.length_nm, "segments": b.segments, "via_count": b.via_count,
        "pad_count": b.pad_count, "pins": b.pins, "pull_ups": b.pull_ups,
    })
}

/// `aethelgard extract --board <pcb> --spec <spec>`.
pub fn extract(board_path: &Path, spec_path: &Path) -> ExitCode {
    let (spec, spec_sha256) = match crate::read_spec(Some(spec_path)) {
        Ok(s) => s,
        Err((r, code)) => return crate::emit(&r, code),
    };
    let (topology, table) = match crate::solve_spec(&spec, &spec_sha256) {
        Ok(t) => t,
        Err(code) => return code,
    };
    let bytes = match crate::read_input(Some(board_path)) {
        Ok(b) => b,
        Err(r) => return crate::emit(&r, EXIT_UNREADABLE),
    };
    let board_sha256 = crate::tree::sha256(&bytes);
    let refuse = |kind: &'static str, message: String, code: u8| {
        crate::emit(&Refusal::new(kind, None, message), code)
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return refuse(
            "BoardUnreadable",
            format!("{} is not UTF-8", board_path.display()),
            EXIT_UNREADABLE,
        );
    };
    // Each line and the parts the specification's design puts on it.
    let wanted = match bound::lines(&spec, &topology, &table) {
        Ok(l) => l,
        Err(r) => return refuse(r.kind, r.message, EXIT_INVALID_SPEC),
    };
    let lines = match bound::read(text).and_then(|read| bound::bound(&read, &wanted)) {
        Ok(l) => l,
        Err(r) => {
            return crate::emit(
                &Refusal::new(r.kind, r.id.as_deref(), r.message),
                EXIT_UNREADABLE,
            );
        }
    };
    let mut out = Vec::new();
    for b in lines {
        // `solve` refused a line with no constraint (`MissingBusConstraint`).
        let Some(declared) = topology.analog_lines.get(&b.net) else {
            return refuse(
                "MissingBusConstraint",
                format!("`{}` has no declared capacitance bound", b.net),
                EXIT_INVALID_SPEC,
            );
        };
        out.push(line(b, declared.capacitance_femtofarads));
    }
    let findings: Vec<Finding> = out
        .iter()
        .filter(|l| l["verdict"] == "fail")
        .map(|l| Finding {
            kind: "capacitance_over_declared_bound",
            severity: "error",
            message: format!(
                "`{}`: bound {} fF (trace {}, vias {}, pads {}, pins {}) exceeds the declared {} fF",
                l["net"].as_str().unwrap_or_default(),
                l["c_bound_femtofarads"],
                l["terms"]["trace"],
                l["terms"]["vias"],
                l["terms"]["pads"],
                l["terms"]["pins"],
                l["declared_femtofarads"]
            ),
        })
        .collect();
    let ok = findings.is_empty();
    let report = Report {
        ok,
        verdict: if ok { "pass" } else { "fail" },
        check: "extract",
        engine_version: env!("CARGO_PKG_VERSION"),
        input: InputReport {
            path: board_path.display().to_string(),
            sha256: board_sha256,
        },
        specification_sha256: spec_sha256,
        capacitance: CAPACITANCE,
        conditions: CONDITIONS,
        lines: out,
        findings,
    };
    crate::emit(&report, if ok { 0 } else { EXIT_FAIL })
}
