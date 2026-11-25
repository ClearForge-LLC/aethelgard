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

use crate::verdict::{InputReport, Ran, Scope};
use crate::{EXIT_FAIL, EXIT_INVALID_SPEC, EXIT_UNREADABLE, Refusal};

/// `RUL-19`, in the verdict itself.
const CAPACITANCE: &str = "upper bound estimated from routed geometry";

/// Why a board with no line and no supply domain judged nothing
/// (`AET-WO-2306`).
pub const NOTHING_IN_SCOPE: &str = "no bounded line: the specification declares no I2C bus, no SPI bus and no supply domain with a drop limit";

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
    /// The lines and supply domains judged, each with its own verdict
    /// (`AET-WO-2306`); a pair's conductors are lines, so not counted twice.
    #[serde(flatten)]
    scope: Scope,
    lines: Vec<Value>,
    /// Each domain with a drop limit, judged (`AET-WO-1303b`); absent when
    /// the specification limits none, so no other verdict moves.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    supplies: Vec<Value>,
    /// Each declared coupled pair's conductors (`AET-WO-1303b`, D2): absent
    /// when the specification declares none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pairs: Vec<Value>,
    /// Each oscillator net beside a declared crystal (`AET-WO-4240c`):
    /// reported, not judged against a value; absent when no crystal is
    /// declared, so no other report moves.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    oscillator: Vec<Value>,
    /// Each buck's thermal pad, with the copper it joins on each layer,
    /// measured from the board as read (`AET-WO-4300k` (a)); absent with no
    /// buck, so no other report moves.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    thermal: Vec<Value>,
    /// Each part of the gate, judged or off by the key that turns it on
    /// (`AET-WO-1609`): [`parts`].
    ran: Vec<Ran>,
    findings: Vec<Finding>,
}

/// The gate's parts (`AET-WO-1609`, ruled D1 (a), D3): each one's row from
/// the specification's own predicate, the one eda tests before it bounds
/// anything, with the count of what it judged. The bus lines always run; a
/// supply domain only under a non-empty `power.drop_limits`
/// (`aethelgard_eda::supply::supplies`: `[]` validates and judges nothing);
/// an oscillator net only beside a declared crystal
/// (`aethelgard_eda::extract::oscillator`: an external clock has none), each
/// judged by its no-via rule; a coupled pair's conductors only under
/// `diff_pairs`. A test drives each both ways against eda's own result.
fn parts(
    spec: &aethelgard_ir::Specification,
    lines: usize,
    supplies: usize,
    oscillator: usize,
    conductors: usize,
    thermal: usize,
    openings: usize,
) -> Vec<Ran> {
    let drop_limits = spec
        .power
        .as_ref()
        .is_some_and(|p| !p.drop_limits.is_empty());
    let crystal = matches!(spec.mcu.hse, Some(aethelgard_ir::clock::Hse::Crystal(_)));
    vec![
        Ran::judged("extract.lines", lines as u64),
        Ran::part(
            "extract.supply",
            drop_limits,
            supplies as u64,
            "power.drop_limits",
        ),
        Ran::part(
            "extract.oscillator",
            crystal,
            oscillator as u64,
            "mcu.hse.crystal",
        ),
        Ran::part(
            "extract.pairs",
            !spec.diff_pairs.is_empty(),
            conductors as u64,
            "diff_pairs",
        ),
        Ran::judged("extract.openings", openings as u64),
        Ran::part(
            "extract.thermal",
            spec.power
                .as_ref()
                .is_some_and(|p| p.regulators.iter().any(|r| r.junction.is_some())),
            thermal as u64,
            "power.regulators[].junction",
        ),
    ]
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
    /// Why the line was routed with no regard to its bound
    /// (`AET-WO-1314b`): on a `capacitance_over_declared_bound` finding
    /// only, and only where the specification routes with no capacitance
    /// ladder; absent otherwise, so no other finding moves.
    #[serde(skip_serializing_if = "Option::is_none")]
    cause: Option<&'static str>,
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

/// One oscillator net's object (`AET-WO-4240c` §1.4 (b), ruled 16:34Z): its
/// copper bound from above, its length, vias and pads, the crystal's gap to
/// the MCU, and DS12117's `CIO`, labelled typical and not added.
fn oscillator_net(o: &aethelgard_eda::extract::Oscillator) -> Value {
    let b = &o.line;
    json!({
        "net": b.net, "board_net": b.board_net,
        "c_bound_femtofarads": b.c_bound_femtofarads,
        "terms": {"trace": b.terms.trace, "vias": b.terms.vias, "pads": b.terms.pads},
        "length_nm": b.length_nm, "segments": b.segments, "via_count": b.via_count,
        "pad_count": b.pad_count,
        "crystal": o.crystal, "crystal_gap_um": o.crystal_gap_um,
        "mcu_pin_cio_typical_femtofarads": bound::OSC_PIN_CIO_TYPICAL_FEMTOFARADS,
    })
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
    if let Err(r) = crate::draws_a_board(&spec, "extract") {
        return crate::emit(&r, EXIT_INVALID_SPEC);
    }
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
    // the board as declared, and whether `emit` routed this specification by
    // the capacitance ladder: from the specification alone, as `emit`'s
    // placement decides it (`aethelgard_eda::schematic::ladder_runs`); a
    // partial tolerance block is refused by either, by the same name
    let declared = aethelgard_eda::stack::Stack::of(&spec)
        .and_then(|stack| Ok((stack, aethelgard_eda::schematic::ladder_runs(&spec)?)));
    let (stack, laddered) = match declared {
        Ok(d) => d,
        Err(path) => {
            let s = crate::undeclared(path);
            return crate::emit(&Refusal::new(s.kind, s.id.as_deref(), s.message), s.code);
        }
    };
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
    // The oscillator nets beside a declared crystal (`AET-WO-4240c` §1.4 (b),
    // §1.6, ruled A7): reported, and a via on one is a finding; none for a
    // specification that declares no crystal, whose report is unchanged.
    let oscillator = match bound::oscillator(&spec, &topology, &table, &read) {
        Ok(o) => o,
        Err(r) => {
            return crate::emit(
                &Refusal::new(r.kind, r.id.as_deref(), r.message),
                EXIT_UNREADABLE,
            );
        }
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
            cause: (!laddered).then_some(bound::LADDER_NOT_RUN),
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
                        cause: None,
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
                cause: None,
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
                cause: None,
            });
        }
    }
    for o in &oscillator {
        if let Some(r) = o.refusal() {
            findings.push(Finding {
                kind: r.kind,
                severity: "error",
                message: r.message,
                cause: None,
            });
        }
    }
    if !oscillator.is_empty() {
        conditions.push(bound::OSC_PIN_CONDITION.to_owned());
    }
    // `AET-WO-4300k` (a): each buck's thermal pad, the copper it joins
    // measured on the board as read, and its junction screened at the least
    // joined layer's area. A radio module's ground pad has its own thermal
    // vias (`AET-WO-4411`) and no buck's junction to screen.
    let bucks: Vec<&str> = spec
        .power
        .iter()
        .flat_map(|p| p.regulators.iter().map(|r| r.reference.as_str()))
        .collect();
    let pads: Vec<(String, String)> = read
        .board
        .stack
        .thermal
        .iter()
        .filter(|t| bucks.contains(&t.reference.as_str()))
        .map(|t| (t.reference.clone(), t.pad.clone()))
        .collect();
    let mut thermal = Vec::new();
    for j in aethelgard_eda::pour::joined(&read.board, &read.routed, &pads) {
        let layers: serde_json::Map<String, Value> = j
            .layers
            .iter()
            .map(|(l, a)| (l.clone(), json!(a)))
            .collect();
        let verdict = match aethelgard_ir::power::junction_screen(&spec, &j.reference, j.min_mm2) {
            Ok(text) => {
                conditions.push(text);
                "pass"
            }
            Err(e) => {
                findings.push(Finding {
                    kind: e.kind(),
                    severity: "error",
                    message: e.to_string(),
                    cause: None,
                });
                "fail"
            }
        };
        thermal.push(json!({
            "reference": j.reference,
            "joined_square_millimetres": layers,
            "least_square_millimetres": j.min_mm2,
            "bound": "lower, from the board as read",
            "verdict": verdict,
        }));
    }
    // `AET-WO-4300k` (4): every connector opens off the board, by its row.
    let (openings, inward) = aethelgard_eda::opening::judge(&read.board, |r| {
        read.footprints.get(r).map(|(_, value)| value.clone())
    });
    if let Some(i) = inward {
        findings.push(Finding {
            kind: "ConnectorOpensInward",
            severity: "error",
            message: i.message,
            cause: None,
        });
    }
    let oscillator: Vec<Value> = oscillator.iter().map(oscillator_net).collect();
    let supplies: Vec<Value> = gates.iter().map(supply).collect();
    let scope = Scope::of(
        (out.len() + supplies.len() + oscillator.len()) as u64,
        NOTHING_IN_SCOPE,
    );
    let conductors = pairs
        .iter()
        .map(|p| p["conductors"].as_array().map_or(0, Vec::len))
        .sum();
    let ran = parts(
        &spec,
        out.len(),
        supplies.len(),
        oscillator.len(),
        conductors,
        thermal.len(),
        openings,
    );
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
        scope,
        lines: out,
        supplies,
        pairs,
        oscillator,
        thermal,
        ran,
        findings,
    };
    crate::emit(&report, if ok { 0 } else { EXIT_FAIL })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::fixture as spec;
    use serde_json::json;

    fn row(rows: &[Ran], gate: &str) -> Ran {
        rows.iter().find(|r| r.gate == gate).unwrap().clone()
    }

    /// Every part has its row, every time, in this order: none is omitted,
    /// whatever the specification declares (`AET-WO-1609` §2).
    #[test]
    fn every_part_has_a_row_in_order() {
        let s = spec("../../../../examples/sentinel-03.json", |_| ());
        let gates: Vec<String> = parts(&s, 0, 0, 0, 0, 0, 0)
            .into_iter()
            .map(|r| r.gate)
            .collect();
        assert_eq!(
            gates,
            [
                "extract.lines",
                "extract.supply",
                "extract.oscillator",
                "extract.pairs",
                "extract.openings",
                "extract.thermal"
            ]
        );
        assert_eq!(
            row(&parts(&s, 0, 0, 0, 0, 0, 0), "extract.lines"),
            Ran::judged("extract.lines", 0),
            "no line is `judged 0`, never omitted"
        );
    }

    /// The supply part, both ways: off with no power tree and with
    /// `power.drop_limits` empty (which `validate` refuses since
    /// `AET-WO-4300k`, but the part reads the key alone); judged with a limit,
    /// its count eda's.
    #[test]
    fn the_supply_part_is_off_until_a_drop_limit_is_declared() {
        let off = Ran::off("extract.supply", "power.drop_limits");
        let empty = spec("../../../../examples/sentinel-03.json", |v| {
            v["power"]["drop_limits"] = json!([]);
        });
        assert_eq!(row(&parts(&empty, 3, 0, 0, 0, 0, 0), "extract.supply"), off);
        let no_power = spec("sentinel-01-bench.json", |_| ());
        assert!(no_power.power.is_none());
        assert_eq!(
            row(&parts(&no_power, 3, 0, 0, 0, 0, 0), "extract.supply"),
            off
        );
        let limited = spec("../../../../examples/sentinel-03.json", |_| ());
        assert_eq!(
            row(&parts(&limited, 3, 1, 0, 0, 0, 0), "extract.supply"),
            Ran::judged("extract.supply", 1)
        );
    }

    /// The oscillator part, both ways: judged beside a crystal (each net by
    /// its no-via rule), off with no HSE and off with an external clock,
    /// whose `mcu.hse` is present.
    #[test]
    fn the_oscillator_part_is_on_only_beside_a_crystal() {
        let off = Ran::off("extract.oscillator", "mcu.hse.crystal");
        let crystal = spec("hse-crystal-16.json", |_| ());
        assert_eq!(
            row(&parts(&crystal, 3, 0, 2, 0, 0, 0), "extract.oscillator"),
            Ran::judged("extract.oscillator", 2)
        );
        let external = spec("hse-external-8.json", |_| ());
        assert!(external.mcu.hse.is_some());
        assert_eq!(
            row(&parts(&external, 3, 0, 0, 0, 0, 0), "extract.oscillator"),
            off
        );
        let none = spec("sentinel-01-bench.json", |_| ());
        assert!(none.mcu.hse.is_none());
        assert_eq!(
            row(&parts(&none, 3, 0, 0, 0, 0, 0), "extract.oscillator"),
            off
        );
    }

    /// The pair part, both ways: judged by its conductors under
    /// `diff_pairs`, off without it.
    #[test]
    fn the_pair_part_is_off_without_diff_pairs() {
        let paired = spec("pair-standin.json", |_| ());
        assert!(!paired.diff_pairs.is_empty());
        assert_eq!(
            row(&parts(&paired, 4, 0, 0, 2, 0, 0), "extract.pairs"),
            Ran::judged("extract.pairs", 2)
        );
        let unpaired = spec("pair-standin.json", |v| {
            v["diff_pairs"] = json!([]);
        });
        assert_eq!(
            row(&parts(&unpaired, 4, 0, 0, 0, 0, 0), "extract.pairs"),
            Ran::off("extract.pairs", "diff_pairs")
        );
    }
}
