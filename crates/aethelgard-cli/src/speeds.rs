//! `check speeds`: the output speed check's verdict for one specification
//! (`AET-WO-4300c` §1.2, §1.4, ruled D1-D4).
//!
//! Every pin the board crate configures is listed with the `OSPEEDR` value
//! the generator emits. A line whose edges the MCU drives at its
//! peripheral's rate (each SPI bus's SCK and MOSI) is judged against DS12117
//! Table 149, with the figures that decided it. Every other line is named
//! with the reason it is out of scope. The verdict is a function of the
//! solved topology alone (`RUL-16`), so it reaches a specification no board
//! can be drawn for; its capacitance label says so.
//!
//! The capacitance judged is the declared one (`net_constraints`). Where the
//! specification's design can be drawn, `extract` (and `verify`'s extract
//! gate) bounds it on the routed board. Where it cannot, the verdict labels
//! it unverified and names the emitter's refusal.

use std::path::Path;
use std::process::ExitCode;

use aethelgard_svd_hal::board::{self as board, SpeedCheck};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{EXIT_INVALID_SPEC, Refusal};

#[derive(Serialize)]
struct Report {
    ok: bool,
    verdict: &'static str,
    check: &'static str,
    engine_version: &'static str,
    specification_sha256: String,
    /// The table judged against.
    table: Value,
    /// Where the judged capacitance comes from, and whether a board bounds it.
    capacitance: String,
    lines: Vec<Value>,
}

fn ospeedr(v: u8) -> &'static str {
    ["00", "01", "10", "11"][usize::from(v & 3)]
}

/// `aethelgard check speeds --spec <spec>`.
pub fn speeds(spec_path: &Path) -> ExitCode {
    let (spec, topology, table) = match crate::solved(spec_path) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let pins = match board::pin_speeds(&topology, &table) {
        Ok(p) => p,
        Err(e) => {
            return crate::emit(
                &Refusal::new(e.kind(), None, e.to_string()),
                EXIT_INVALID_SPEC,
            );
        }
    };
    let speeds_table = match aethelgard_ir::speeds::table() {
        Ok(t) => t,
        Err(e) => return crate::emit(&Refusal::new(e.kind(), None, e.to_string()), 70),
    };
    let capacitance = match aethelgard_eda::schematic::emit(&spec, &topology, &table) {
        Ok(_) => "declared (net_constraints); extract bounds it on the routed board".to_owned(),
        Err(e) => format!(
            "declared (net_constraints), UNVERIFIED: no board can be drawn ({}: {e})",
            e.kind()
        ),
    };
    let lines = pins
        .into_iter()
        .map(|p| {
            let mut v = json!({
                "pin": p.pin, "net": p.net, "signal": p.signal,
                "ospeedr": ospeedr(p.ospeedr),
            });
            match p.check {
                SpeedCheck::Judged {
                    rate_hz,
                    vdd_min_millivolts,
                    capacitance_femtofarads,
                    rise_time_ns,
                    row,
                    fmax_hz,
                } => {
                    v["judged"] = json!({
                        "verdict": "pass",
                        "rate_hz": rate_hz,
                        "vdd_min_millivolts": vdd_min_millivolts,
                        "band_millivolts": [row.vdd_min_millivolts, row.vdd_max_millivolts],
                        "declared_femtofarads": capacitance_femtofarads,
                        "row_picofarads": row.load_picofarads,
                        "fmax_hz": fmax_hz,
                        "declared_rise_time_ns": rise_time_ns,
                        "rise_fall_picoseconds": row.rise_fall_picoseconds,
                    });
                }
                SpeedCheck::OutOfScope { reason } => v["out_of_scope"] = reason.into(),
            }
            v
        })
        .collect();
    let report = Report {
        ok: true,
        verdict: "pass",
        check: "speeds",
        engine_version: env!("CARGO_PKG_VERSION"),
        specification_sha256: topology.specification_sha256.clone(),
        table: json!({
            "mcu": speeds_table.mcu,
            "rev_v": format!("{}, rev. {}, {}", speeds_table.rev_v.document, speeds_table.rev_v.revision, speeds_table.rev_v.section),
            "rev_y": format!("{}, rev. {}, {}", speeds_table.rev_y.document, speeds_table.rev_y.revision, speeds_table.rev_y.section),
            "definition": speeds_table.definition,
        }),
        capacitance,
        lines,
    };
    crate::emit(&report, 0)
}
