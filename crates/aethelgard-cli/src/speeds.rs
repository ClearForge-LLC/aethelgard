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
//! The capacitance judged is the declared one (`net_constraints`), never a
//! bound: this check draws no board. Where the specification's design can be
//! drawn, `extract` (and `verify`'s extract gate) bounds it on the routed
//! board; where it cannot, the verdict labels it unverified and names the
//! emitter's refusal. `judged` counts the lines judged, 0 for a
//! specification with no SPI bus.
//!
//! `AET-WO-4230` §1.2 (Amendment 1): a line's rate is its controller's
//! achieved rate at the fastest HSI (`spi_controllers[..].achieved_max_hz`),
//! not the declared speed; the output names the declared, the nominal
//! achieved and the fastest achieved rate, and the line holds when the
//! chosen speed's `fmax` is at least the fastest.

use std::path::Path;
use std::process::ExitCode;

use aethelgard_svd_hal::board::{self as board, SpeedCheck};
use serde::Serialize;
use serde_json::{Value, json};

use crate::{EXIT_FAIL, EXIT_INVALID_SPEC, Refusal};

#[derive(Serialize)]
struct Report {
    ok: bool,
    verdict: &'static str,
    check: &'static str,
    engine_version: &'static str,
    specification_sha256: String,
    /// The lifecycle banner (`AET-WO-4210` D1); absent when it is empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lifecycle: Vec<String>,
    /// The table judged against.
    table: Value,
    /// Where the judged capacitance comes from, and whether a board can
    /// bound it.
    capacitance: String,
    /// How many lines were judged; the others are out of scope.
    judged: usize,
    lines: Vec<Value>,
}

fn ospeedr(v: u8) -> &'static str {
    ["00", "01", "10", "11"][usize::from(v & 3)]
}

/// One judged line's rates and verdict (`AET-WO-4230` §1.2): its SPI
/// controller's declared, nominal achieved and fastest achieved rate, from
/// the solved topology, against the chosen speed's `fmax_hz`. `None` when
/// the line's net is not on a solved SPI controller.
fn judge_line(
    topology: &aethelgard_ir::SolvedTopology,
    net: &str,
    fmax_hz: u32,
) -> Option<(bool, Value)> {
    let peripheral = &topology.assignments.get(net)?.peripheral;
    let c = topology.spi_controllers.get(peripheral)?;
    let holds = c.achieved_max_hz <= u64::from(fmax_hz);
    Some((
        holds,
        json!({
            "verdict": if holds { "pass" } else { "fail" },
            "controller": peripheral,
            "declared_hz": c.declared_hz,
            "achieved_hz": c.achieved_hz,
            "achieved_max_hz": c.achieved_max_hz,
        }),
    ))
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
    // Nothing here bounds the copper: say so, and who does (adversarial 4).
    let capacitance = match aethelgard_eda::schematic::emit(&spec, &topology, &table) {
        Ok(_) => "declared (net_constraints), not bounded here: run extract, or verify's extract gate, on the routed board".to_owned(),
        Err(e) => format!(
            "declared (net_constraints), UNVERIFIED: no board can be drawn ({}: {e})",
            e.kind()
        ),
    };
    let judged = pins
        .iter()
        .filter(|p| matches!(p.check, SpeedCheck::Judged { .. }))
        .count();
    let mut ok = true;
    let lines = pins
        .into_iter()
        .map(|p| {
            let mut v = json!({
                "pin": p.pin, "net": p.net, "signal": p.signal,
                "ospeedr": ospeedr(p.ospeedr),
            });
            match p.check {
                SpeedCheck::Judged {
                    vdd_min_millivolts,
                    capacitance_femtofarads,
                    rise_time_ns,
                    row,
                    fmax_hz,
                    ..
                } => {
                    // `SolvedTopology::validate` puts every solved SCK and
                    // MOSI on a solved controller; a line that is not fails.
                    let (holds, rates) = judge_line(&topology, &p.net, fmax_hz)
                        .unwrap_or_else(|| (false, json!({"verdict": "fail", "controller": null})));
                    ok &= holds;
                    v["judged"] = json!({
                        "vdd_min_millivolts": vdd_min_millivolts,
                        "band_millivolts": [row.vdd_min_millivolts, row.vdd_max_millivolts],
                        "declared_femtofarads": capacitance_femtofarads,
                        "row_picofarads": row.load_picofarads,
                        "fmax_hz": fmax_hz,
                        "declared_rise_time_ns": rise_time_ns,
                        "rise_fall_picoseconds": row.rise_fall_picoseconds,
                    });
                    if let (Value::Object(j), Value::Object(r)) = (&mut v["judged"], rates) {
                        j.extend(r);
                    }
                }
                SpeedCheck::OutOfScope { reason } => v["out_of_scope"] = reason.into(),
            }
            v
        })
        .collect();
    let report = Report {
        ok,
        verdict: if ok { "pass" } else { "fail" },
        check: "speeds",
        engine_version: env!("CARGO_PKG_VERSION"),
        specification_sha256: topology.specification_sha256.clone(),
        lifecycle: topology.lifecycle_banner().unwrap_or_default(),
        table: json!({
            "mcu": speeds_table.mcu,
            "rev_v": format!("{}, rev. {}, {}", speeds_table.rev_v.document, speeds_table.rev_v.revision, speeds_table.rev_v.section),
            "rev_y": format!("{}, rev. {}, {}", speeds_table.rev_y.document, speeds_table.rev_y.revision, speeds_table.rev_y.section),
            "definition": speeds_table.definition,
        }),
        capacitance,
        judged,
        lines,
    };
    crate::emit(&report, if ok { 0 } else { EXIT_FAIL })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `AET-WO-4230` §1.2: the reference's SPI1 is judged at its fastest
    /// achieved rate (16,239,744 Hz at MBR 1, Amendment 1), not its declared
    /// 20 MHz; a speed whose `fmax` is under that fails, one at it holds.
    #[test]
    fn a_line_is_judged_at_its_controller_s_fastest_achieved_rate() {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/sentinel-01-minimal.json"),
        )
        .unwrap();
        let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
        let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
        let t = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
        let (holds, v) = judge_line(&t, "SPI_SCK", 16_239_744).unwrap();
        assert!(holds);
        assert_eq!(
            v,
            json!({"verdict": "pass", "controller": "SPI1", "declared_hz": 20_000_000,
                   "achieved_hz": 16_000_000, "achieved_max_hz": 16_239_744})
        );
        let (holds, v) = judge_line(&t, "SPI_MOSI", 16_239_743).unwrap();
        assert!(!holds);
        assert_eq!(v["verdict"], "fail");
        // A net on no SPI controller has no rate to judge.
        assert!(judge_line(&t, "I2C_SCL", u32::MAX).is_none());
    }
}
