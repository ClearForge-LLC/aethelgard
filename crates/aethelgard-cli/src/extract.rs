//! `extract`: each I2C line's capacitance, and each SPI bus's SCK and MOSI
//! (`AET-WO-4300c` D2), bounded from above from the routed board's copper and
//! judged against the line's declared bound (`AET-WO-1305`, `RUL-19` (1)). The bound and its sources are `aethelgard_eda::extract`'s.
//!
//! The board is read once, into memory; its sha256 and the bound are both
//! taken from those bytes, so the verdict names exactly what it judged. No
//! tool runs.

use std::path::Path;
use std::process::ExitCode;

use aethelgard_eda::extract::{self as bound, LineBound};
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
    /// The lifecycle banner (`AET-WO-4210` D1); absent when it is empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lifecycle: Vec<String>,
    capacitance: &'static str,
    conditions: Vec<String>,
    lines: Vec<Value>,
    /// Each domain with a drop limit, judged (`AET-WO-1303b`); absent when
    /// the specification limits none, so no other verdict moves.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    supplies: Vec<Value>,
    /// Each declared coupled pair's conductors (`AET-WO-1303b`, D2): absent
    /// when the specification declares none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pairs: Vec<Value>,
    findings: Vec<Finding>,
}

/// D2's stated condition: the bound holds each conductor's capacitance, not
/// the difference between them.
const PAIR_MISMATCH: &str = "unverified: the conductors' capacitance mismatch is a stated condition of the router's symmetric pair routing (AET-WO-1303 D2), not bounded";

/// One supply domain's verdict object.
fn supply(g: &aethelgard_eda::supply::Gate) -> Value {
    use aethelgard_eda::supply::PinResult;
    let pin = |r: &PinResult| match r {
        PinResult::Bounded {
            pin,
            r_bound_microohms,
        } => json!({"pin": pin, "r_bound_microohms": r_bound_microohms}),
        PinResult::Refused { pin, kind, message } => {
            json!({"pin": pin, "refused": {"kind": kind, "message": message}})
        }
    };
    let pass = g
        .drop_bound_microvolts
        .is_some_and(|d| d <= u64::from(g.limit_millivolts) * 1000);
    let mut v = json!({
        "domain": g.domain,
        "verdict": if pass { "pass" } else { "fail" },
        "resistance": "upper bound certified from routed geometry",
        "source": {"reference": g.source.0, "pad": g.source.1},
        "return": {"reference": g.ret.0, "pad": g.ret.1},
        "load": g.mcu,
        "load_milliamps": g.load_milliamps,
        "limit_millivolts": g.limit_millivolts,
        "limit_source": g.limit_source,
        "drop_bound_microvolts": g.drop_bound_microvolts,
        "pins": g.pins.iter().map(pin).collect::<Vec<_>>(),
        "returns": g.returns.iter().map(pin).collect::<Vec<_>>(),
        "conditions": g.conditions,
    });
    if let Some(n) = &g.note {
        v["note"] = n.clone().into();
    }
    if !g.notes.is_empty() {
        v["notes"] = g.notes.clone().into();
    }
    v
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
    let mut v = json!({
        "net": b.net, "board_net": b.board_net, "bus": b.bus,
        "verdict": if b.c_bound_femtofarads <= declared { "pass" } else { "fail" },
        "c_bound_femtofarads": b.c_bound_femtofarads, "declared_femtofarads": declared,
        "terms": {"trace": t.trace, "vias": t.vias, "pads": t.pads, "pins": t.pins},
        "length_nm": b.length_nm, "segments": b.segments, "via_count": b.via_count,
        "pad_count": b.pad_count, "pins": b.pins, "pull_ups": b.pull_ups,
    });
    // An I2C line's object is `AET-WO-1305`'s, unchanged.
    if let Some(signal) = b.spi_signal {
        v["signal"] = signal.into();
        v["drivers"] = b.drivers.into();
    }
    // A line with no connector is unchanged too (`AET-WO-4300f` D8).
    if !b.offboard.is_empty() {
        v["terms"]["offboard"] = t.offboard.into();
        v["offboard"] = b.offboard.into();
    }
    // A pin counted at a declared value (`AET-WO-4220b`) is listed apart from
    // `pins`, which are maxima; a line with none is unchanged.
    if !b.declared_pins.is_empty() {
        v["terms"]["declared_pins"] = t.declared_pins.into();
        v["declared_pins"] = b.declared_pins.into();
    }
    v
}

/// The footprint `emit` draws a test point with (`AET-WO-1312`): a bare pad.
const TEST_PAD: &str = "TestPoint:TestPoint_Pad_D1.0mm";

/// One condition per test pad on a bounded line (`AET-WO-1604` §1.5, `1312`
/// Q2), in the lines' order, each line's pads by reference. The pads are
/// read from the board judged, never from the specification (`RUL-16`):
/// every footprint drawn as a test pad with a pad on the line's net. Its
/// copper is in the bound, as every pad's is; a probe clipped to it is not.
/// A line with no test pad, and a test pad on a net no line bounds, state
/// nothing.
fn probe_conditions(read: &bound::ReadBoard, lines: &[LineBound]) -> Vec<String> {
    let mut out = Vec::new();
    for b in lines {
        for part in &read.board.parts {
            let test_pad = read
                .footprints
                .get(&part.reference)
                .is_some_and(|(library_id, _)| library_id == TEST_PAD);
            let on_line = part
                .pads
                .iter()
                .any(|p| p.net.as_deref() == Some(b.board_net.as_str()));
            if test_pad && on_line {
                out.push(format!(
                    "`{}` is a test pad on `{}`: a probe clipped to it is an off-board load outside this bound, so a measured edge is compared with the bound plus the probe's own capacitance",
                    part.reference, b.net
                ));
            }
        }
    }
    out
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
    let stack = aethelgard_eda::stack::Stack::of(&spec);
    let bounded = bound::read(text, &stack)
        .and_then(|read| bound::bound(&read, &wanted).map(|lines| (read, lines)));
    let (read, lines) = match bounded {
        Ok(l) => l,
        Err(r) => {
            return crate::emit(
                &Refusal::new(r.kind, r.id.as_deref(), r.message),
                EXIT_UNREADABLE,
            );
        }
    };
    let gates = match aethelgard_eda::supply::supplies(&spec, &topology, &table, &read) {
        Ok(g) => g,
        Err(r) => return refuse(r.kind, r.message, EXIT_INVALID_SPEC),
    };
    let mut out = Vec::new();
    let mut conditions = bound::conditions(&stack);
    if lines.iter().any(|b| b.spi_signal.is_some()) {
        // A declared SPI pin is not a cited maximum (`AET-WO-4220b`, H1).
        let declared = lines
            .iter()
            .any(|b| b.spi_signal.is_some() && !b.declared_pins.is_empty());
        conditions.push(if declared {
            bound::SPI_CONDITION_BESIDE_DECLARED.to_owned()
        } else {
            bound::SPI_CONDITION.to_owned()
        });
    }
    if aethelgard_eda::schematic::declares_offboard(&spec) {
        conditions.push(bound::OFFBOARD.to_owned());
    }
    // A declared I2C pin is not UM10204's `Ci`, so the standing condition
    // names the pins it covers (`AET-WO-4220b`, adversarial A2).
    if lines
        .iter()
        .any(|b| b.spi_signal.is_none() && !b.declared_pins.is_empty())
    {
        for c in &mut conditions {
            if c == bound::I2C_PIN_CONDITION {
                bound::I2C_PIN_CONDITION_BESIDE_DECLARED.clone_into(c);
            }
        }
    }
    conditions.extend(bound::declared_conditions(&spec));
    conditions.extend(probe_conditions(&read, &lines));
    // each declared pair's conductors, from the lines' own bounds, against
    // the net's declared capacitance: a conductor the extraction does not
    // bound as a line is refused by name
    let mut pairs = Vec::new();
    for p in &spec.diff_pairs {
        let mut conductors = Vec::new();
        for net in &p.nets {
            let Some(b) = lines.iter().find(|b| &b.net == net) else {
                return refuse(
                    "PairConductorUnbounded",
                    format!(
                        "`{net}` of pair `{}` is not a line the extraction bounds",
                        p.name
                    ),
                    EXIT_INVALID_SPEC,
                );
            };
            let Some(c) = spec.net_constraints.iter().find(|c| &c.net_name == net) else {
                return refuse(
                    "MissingBusConstraint",
                    format!(
                        "`{net}` of pair `{}` has no declared capacitance bound",
                        p.name
                    ),
                    EXIT_INVALID_SPEC,
                );
            };
            conductors.push(json!({
                "net": net,
                "verdict": if b.c_bound_femtofarads <= c.max_capacitance_femtofarads { "pass" } else { "fail" },
                "c_bound_femtofarads": b.c_bound_femtofarads,
                "declared_femtofarads": c.max_capacitance_femtofarads,
            }));
        }
        pairs.push(json!({"name": p.name, "conductors": conductors, "mismatch": PAIR_MISMATCH}));
    }
    for b in lines {
        // `solve` refused a line with no constraint (`MissingBusConstraint`,
        // `MissingSpiConstraint`).
        let declared = match b.spi_signal {
            None => topology
                .analog_lines
                .get(&b.net)
                .map(|l| l.capacitance_femtofarads),
            Some(_) => topology
                .spi_lines
                .get(&b.net)
                .map(|l| l.capacitance_femtofarads),
        };
        let Some(declared) = declared else {
            return refuse(
                "MissingBusConstraint",
                format!("`{}` has no declared capacitance bound", b.net),
                EXIT_INVALID_SPEC,
            );
        };
        out.push(line(b, declared));
    }
    let mut findings: Vec<Finding> = out
        .iter()
        .filter(|l| l["verdict"] == "fail")
        .map(|l| Finding {
            kind: "capacitance_over_declared_bound",
            severity: "error",
            message: format!(
                "`{}`: bound {} fF (trace {}, vias {}, pads {}, pins {}{}{}) exceeds the declared {} fF",
                l["net"].as_str().unwrap_or_default(),
                l["c_bound_femtofarads"],
                l["terms"]["trace"],
                l["terms"]["vias"],
                l["terms"]["pads"],
                l["terms"]["pins"],
                l["terms"]
                    .get("offboard")
                    .map_or_else(String::new, |o| format!(", offboard {o}")),
                l["terms"]
                    .get("declared_pins")
                    .map_or_else(String::new, |d| format!(", declared pins {d}")),
                l["declared_femtofarads"]
            ),
        })
        .collect();
    for g in &gates {
        for (side, pins) in [("supply", &g.pins), ("return", &g.returns)] {
            for r in pins.iter() {
                if let aethelgard_eda::supply::PinResult::Refused { pin, kind, message } = r {
                    findings.push(Finding {
                        kind: "supply_pin_unbounded",
                        severity: "error",
                        message: format!(
                            "`{}` {side} pin {}.{pin}: {kind}: {message}",
                            g.domain, g.mcu
                        ),
                    });
                }
            }
        }
        let refused = g
            .pins
            .iter()
            .chain(&g.returns)
            .any(|r| matches!(r, aethelgard_eda::supply::PinResult::Refused { .. }));
        if g.drop_bound_microvolts.is_none() && !refused {
            // belt-and-suspenders: today a drop is absent only beside a
            // refused pin, which has its finding; any other absence fails too
            findings.push(Finding {
                kind: "supply_drop_unbounded",
                severity: "error",
                message: format!("`{}`: no drop bound", g.domain),
            });
        }
        if let Some(d) = g.drop_bound_microvolts
            && d > u64::from(g.limit_millivolts) * 1000
        {
            findings.push(Finding {
                kind: "supply_drop_over_limit",
                severity: "error",
                message: format!(
                    "`{}`: VDD + VSS drop bound {} uV at {} mA exceeds the declared {} mV",
                    g.domain, d, g.load_milliamps, g.limit_millivolts
                ),
            });
        }
    }
    let supplies: Vec<Value> = gates.iter().map(supply).collect();
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
        lifecycle: topology.lifecycle_banner().unwrap_or_default(),
        capacitance: CAPACITANCE,
        conditions,
        lines: out,
        supplies,
        pairs,
        findings,
    };
    crate::emit(&report, if ok { 0 } else { EXIT_FAIL })
}
