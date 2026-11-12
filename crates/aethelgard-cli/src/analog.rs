//! `simulate --firmware`'s analog pre-pass (`AET-WO-2200` §1.4, `RUL-17`).
//!
//! Before the boot, each I2C line of the solved topology is measured:
//! 1. `aethelgard-twin::analog` writes one netlist per line;
//! 2. the bridge, a sandboxed tool beside this executable, measures each
//!    line's 30 %–70 % rise time with libngspice;
//! 3. each measurement is recorded as `analog.rise_time_measured`;
//! 4. a line past its ceiling, still rising when the window ends, or never
//!    low when released faults its bus: `bridge.fault_state` is
//!    recorded, and the bus's devices are left out of the platform the boot
//!    loads, so the fault applies from the first instruction (ruling L5).
//!
//! The measurement and the firmware's effect are separate records, and the
//! effect is labelled as the model's rendering of a NACK, never as a measured
//! waveform (`AET-WO-2101` S6). A bridge that is missing, fails, is killed or
//! answers anything but one answer per line stops the call: nothing is read
//! as "no fault" (`INV-06`). What the design does to a line is never a stop.

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

use aethelgard_chaos::campaign;
use aethelgard_ir::SolvedTopology;
use aethelgard_twin::analog::{self, Deck};
use serde_json::{Value, json};

use crate::run;
use crate::verdict::{AnalogRecord, FaultRecord, Finding};

/// The bridge binary, found in this executable's own directory.
pub const BRIDGE: &str = "aethelgard-bridge";

/// The ngspice release the bridge is pinned to; an answer naming another is refused.
pub const NGSPICE: &str = "44.2";

/// What the firmware sees of a faulted bus, and why it is not a measurement.
pub const RENDERING: &str = "the bus's devices are absent from the platform. Renode 1.17.0 models no \
NACK, so the firmware sees the absence as a TXIS timeout: the model's rendering of the NACK an \
unreadable address draws, not a measured waveform (AET-WO-2101 S6)";

/// The pass's records and the findings that fail the verdict.
pub struct Pass {
    pub analog: Vec<AnalogRecord>,
    pub faults: Vec<FaultRecord>,
    pub findings: Vec<Finding>,
}

impl Pass {
    /// The buses the boot's platform leaves devices out of, read from the
    /// recorded fault decision itself.
    pub fn faulted(&self) -> BTreeSet<String> {
        self.faults.iter().map(|f| f.bus.clone()).collect()
    }
}

/// The bridge beside this executable, or `BridgeUnavailable` (exit 4).
pub fn bridge() -> Result<run::Tool, run::Stop> {
    let unavailable = |message: String| run::Stop {
        code: crate::EXIT_TOOL_UNAVAILABLE,
        kind: "BridgeUnavailable",
        id: Some(BRIDGE.to_owned()),
        message,
    };
    let me = std::env::current_exe()
        .map_err(|e| unavailable(format!("cannot find this executable: {e}")))?;
    let path = me
        .parent()
        .map(|d| d.join(BRIDGE))
        .ok_or_else(|| unavailable("this executable has no directory".to_owned()))?;
    match fs::metadata(&path) {
        Ok(m) if m.is_file() => Ok(run::Tool { name: BRIDGE, path }),
        _ => Err(unavailable(format!(
            "{} is not beside this executable, at {}",
            BRIDGE,
            path.display()
        ))),
    }
}

/// Lines a campaign scenario holds low, and the scenario's name
/// (`AET-WO-2300`). `simulate --firmware` passes none.
pub struct Held<'a> {
    pub scenario: &'a str,
    pub lines: &'a [crate::campaign::HeldLow],
}

/// Measures every analog line of `topology` in `dir`, a directory of its own
/// that only the bridge writes; `status` is the reaper's, outside it. A held
/// line is not measured: the bridge's decision is the fault, from the
/// scenario, without a measurement.
pub fn pass(
    topology: &SolvedTopology,
    held: &Held<'_>,
    dir: &Path,
    status: &Path,
) -> Result<Pass, run::Stop> {
    let decks: Vec<Deck> = analog::decks(topology)
        .map_err(crate::twin_stop)?
        .into_iter()
        .filter(|d| !held.lines.iter().any(|h| h.net == d.net))
        .collect();
    let mut pass = measure(&decks, dir, status)?;
    for line in held.lines {
        pass.findings.push(Finding::plain(
            campaign::HELD_LOW_FINDING,
            format!(
                "bus `{}`: net `{}` is held low by scenario `{}`; the bus is faulted without a measurement",
                line.bus, line.net, held.scenario
            ),
        ));
        pass.faults.push(FaultRecord {
            kind: "bridge.fault_state",
            source: format!("scenario {}", held.scenario),
            bus: line.bus.clone(),
            fault: "absent",
            from: vec![campaign::HELD_LOW],
            nets: vec![line.net.clone()],
            firmware_sees: RENDERING,
        });
    }
    Ok(pass)
}

/// Runs the bridge on `decks`; with none, there is nothing to measure.
fn measure(decks: &[Deck], dir: &Path, status: &Path) -> Result<Pass, run::Stop> {
    if decks.is_empty() {
        return Ok(judge(decks, &[]));
    }
    let tool = bridge()?;
    fs::DirBuilder::new()
        .mode(0o700)
        .create(dir)
        .map_err(|e| run::Stop::tool_error(BRIDGE, format!("cannot create its directory: {e}")))?;
    let request = dir.join("request.json");
    fs::write(&request, request_json(decks).to_string())
        .map_err(|e| run::Stop::tool_error(BRIDGE, format!("cannot write its request: {e}")))?;
    let out = run::Invocation {
        tool: &tool,
        args: vec![request.into_os_string()],
        env: Vec::new(),
        workdir: dir,
        cwd: dir.to_path_buf(),
        status: status.to_path_buf(),
        read_only: tool.dirs(),
        read_write: Vec::new(),
    }
    .run_bounded(crate::renode::BOOT_BOUNDS)?;
    let code = out.code(BRIDGE)?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    if code != 0 {
        return Err(run::Stop::tool_error(
            BRIDGE,
            format!(
                "exited {code}: {} {}",
                stdout.trim(),
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        ));
    }
    let measured = answers(decks, &stdout).map_err(|e| run::Stop::tool_error(BRIDGE, e))?;
    Ok(judge(decks, &measured))
}

fn request_json(decks: &[Deck]) -> Value {
    let lines: Vec<Value> = decks
        .iter()
        .map(|d| {
            json!({
                "net": d.net,
                "netlist": d.netlist,
                "source": analog::SOURCE,
                "stimulus": d.stimulus,
                "probe": analog::PROBE,
                "release_ps": d.release_ps,
                "low_uv": d.low_uv,
                "high_uv": d.high_uv,
                "stop_ps": d.stop_ps,
            })
        })
        .collect();
    json!({ "lines": lines })
}

/// What the bridge found on one line (ruled F7 (b) on PR #70).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Answer {
    /// Its 30 %–70 % rise, in ns.
    Rise(f64),
    /// Still rising when the window ended; at least this many ns.
    Incomplete(f64),
    /// At or above 30 % when the driver released it, in volts.
    StuckHigh(f64),
}

/// One answer per deck, in the request's order, each exactly one of the
/// bridge's three documented lines; anything else is refused.
fn answers(decks: &[Deck], stdout: &str) -> Result<Vec<Answer>, String> {
    let lines: Vec<&str> = stdout.lines().collect();
    if lines.len() != decks.len() {
        return Err(format!(
            "answered {} lines for {} nets: {stdout}",
            lines.len(),
            decks.len()
        ));
    }
    decks
        .iter()
        .zip(lines)
        .map(|(deck, line)| {
            let v: Value =
                serde_json::from_str(line).map_err(|e| format!("`{line}` is not JSON: {e}"))?;
            answer(deck, &v).ok_or_else(|| format!("`{line}` is not an answer for `{}`", deck.net))
        })
        .collect()
}

fn answer(deck: &Deck, v: &Value) -> Option<Answer> {
    let o = v.as_object()?;
    let ours = o.get("net").and_then(Value::as_str) == Some(deck.net.as_str())
        && o.get("method").and_then(Value::as_str) == Some("interpolated")
        && o.get("ngspice").and_then(Value::as_str) == Some(NGSPICE);
    let value = |key: &str| o.get(key).and_then(Value::as_f64).filter(|x| x.is_finite());
    let positive = |key: &str| value(key).filter(|x| *x > 0.0);
    let answer = match o.len() {
        4 if o.contains_key("rise_30_70_ns") => Answer::Rise(positive("rise_30_70_ns")?),
        4 if o.contains_key("volts_at_release") => Answer::StuckHigh(value("volts_at_release")?),
        5 if o.get("complete") == Some(&Value::Bool(false)) => {
            Answer::Incomplete(positive("rise_30_70_ns_lower_bound")?)
        }
        _ => return None,
    };
    ours.then_some(answer)
}

/// Why a bus is faulted, as its fault record's `from` names it.
const PAST_CEILING: &str = "analog.rise_time_measured > ceiling_ns";
const STUCK_HIGH: &str = "analog.line_stuck_high";

fn judge(decks: &[Deck], answers: &[Answer]) -> Pass {
    let mut analog_records = Vec::new();
    let mut findings = Vec::new();
    let mut faulted: Vec<(String, Vec<String>, Vec<&'static str>)> = Vec::new();
    for (deck, &answer) in decks.iter().zip(answers) {
        let (bus, net, ceiling) = (&deck.bus, &deck.net, deck.ceiling_ns);
        let mut record = AnalogRecord {
            kind: "analog.rise_time_measured",
            source: "ngspice",
            bus: bus.clone(),
            net: net.clone(),
            complete: true,
            rise_30_70_ns: None,
            rise_30_70_ns_lower_bound: None,
            volts_at_release: None,
            ceiling_ns: ceiling,
            exceeds: false,
            method: "interpolated",
            ngspice: NGSPICE,
        };
        let cause = match answer {
            Answer::Rise(rise) => {
                record.rise_30_70_ns = Some(rise);
                record.exceeds = rise > f64::from(ceiling);
                record.exceeds.then(|| {
                    Finding::plain(
                        campaign::PAST_CEILING_FINDING,
                        format!(
                            "bus `{bus}`: net `{net}` rose from 30 % to 70 % in {rise} ns, past its {ceiling} ns ceiling"
                        ),
                    )
                })
                .map(|f| (f, PAST_CEILING))
            }
            // Always past the ceiling: a charging line slows as it rises, so a
            // rise unfinished 20 µs after the release is at least 4/7 of that,
            // 11.4 µs, against at most 1,000 ns for any I2C mode.
            Answer::Incomplete(bound) => {
                record.complete = false;
                record.rise_30_70_ns_lower_bound = Some(bound);
                record.exceeds = true;
                let f = Finding::plain(
                    campaign::PAST_CEILING_FINDING,
                    format!(
                        "bus `{bus}`: net `{net}` had not reached 70 % when the window ended: its 30 % to 70 % rise is more than {bound} ns (a lower bound), past its {ceiling} ns ceiling"
                    ),
                );
                Some((f, PAST_CEILING))
            }
            Answer::StuckHigh(volts) => {
                record.kind = STUCK_HIGH;
                record.complete = false;
                record.volts_at_release = Some(volts);
                let f = Finding::plain(
                    campaign::STUCK_HIGH_FINDING,
                    format!(
                        "bus `{bus}`: net `{net}` was at {volts} V when its driver released it, at or above 30 % of the rail: the driver cannot pull it low"
                    ),
                );
                Some((f, STUCK_HIGH))
            }
        };
        analog_records.push(record);
        if let Some((finding, from)) = cause {
            findings.push(finding);
            match faulted.iter_mut().find(|(b, _, _)| b == bus) {
                Some((_, nets, causes)) => {
                    nets.push(net.clone());
                    if !causes.contains(&from) {
                        causes.push(from);
                    }
                }
                None => faulted.push((bus.clone(), vec![net.clone()], vec![from])),
            }
        }
    }
    faulted.sort();
    let faults = faulted
        .into_iter()
        .map(|(bus, nets, from)| FaultRecord {
            kind: "bridge.fault_state",
            source: "aethelgard".to_owned(),
            bus,
            fault: "absent",
            from,
            nets,
            firmware_sees: RENDERING,
        })
        .collect();
    Pass {
        analog: analog_records,
        faults,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decks() -> Vec<Deck> {
        let topology: SolvedTopology = serde_json::from_str(
            r#"{"schema_version":2,"specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","assignments":{"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},"I2C_SDA":{"pin":"PB7","peripheral":"I2C1","signal":"I2C1_SDA","alternate_function":4}},"gpio":{},"i2c_ceilings":{"SENSORS":400000},"i2c_speeds":{"SENSORS":400000},"analog_lines":{"I2C_SCL":{"bus":"SENSORS","pull_up_ohms":47000,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300},"I2C_SDA":{"bus":"SENSORS","pull_up_ohms":47000,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300}},"i2c_devices":{"U2":{"index":0,"part":"BME280","bus":"SENSORS","peripheral":"I2C1","address":118},"U3":{"index":1,"part":"TMP117","bus":"SENSORS","peripheral":"I2C1","address":72}}}"#,
        )
        .unwrap();
        analog::decks(&topology).unwrap()
    }

    fn line(net: &str, rise: &str) -> String {
        format!(
            r#"{{"method":"interpolated","net":"{net}","ngspice":"44.2","rise_30_70_ns":{rise}}}"#
        )
    }

    #[test]
    fn every_answer_but_one_documented_line_per_net_in_order_is_refused() {
        let d = decks();
        let good = format!("{}\n{}\n", line("I2C_SCL", "1.5"), line("I2C_SDA", "2.5"));
        assert_eq!(
            answers(&d, &good).unwrap(),
            [Answer::Rise(1.5), Answer::Rise(2.5)]
        );
        let other = format!(
            "{}\n{}\n",
            r#"{"complete":false,"method":"interpolated","net":"I2C_SCL","ngspice":"44.2","rise_30_70_ns_lower_bound":13298.0}"#,
            r#"{"method":"interpolated","net":"I2C_SDA","ngspice":"44.2","volts_at_release":1.65}"#
        );
        assert_eq!(
            answers(&d, &other).unwrap(),
            [Answer::Incomplete(13298.0), Answer::StuckHigh(1.65)]
        );
        let incomplete = |extra: &str| {
            format!(
                r#"{{"complete":false,"method":"interpolated","net":"I2C_SCL","ngspice":"44.2"{extra}}}{}{}"#,
                "\n",
                line("I2C_SDA", "2.5")
            )
        };
        let refused = [
            // An incomplete line that says it completed, gives no bound, or a rise too.
            incomplete(r#","rise_30_70_ns_lower_bound":1.0"#).replace("false", "true"),
            incomplete(""),
            incomplete(r#","rise_30_70_ns_lower_bound":0"#),
            incomplete(r#","rise_30_70_ns":1.0"#),
            incomplete(r#","rise_30_70_ns_lower_bound":1.0,"rise_30_70_ns":1.0"#),
            // A stuck line with no voltage, or with a rise beside it.
            line("I2C_SCL", "1.5").replace("rise_30_70_ns\":1.5", "volts_at_release\":null")
                + "\n"
                + &line("I2C_SDA", "2.5"),
            line("I2C_SCL", "1.5").replace('}', r#","volts_at_release":1.0}"#)
                + "\n"
                + &line("I2C_SDA", "2.5"),
            String::new(),
            line("I2C_SCL", "1.5"),
            format!("{}\n{}\n", line("I2C_SDA", "2.5"), line("I2C_SCL", "1.5")),
            format!("{}\n{}\n", line("I2C_SCL", "1.5"), line("I2C_SCL", "1.5")),
            format!(
                "{}\n{}\n{}\n",
                line("I2C_SCL", "1.5"),
                line("I2C_SDA", "2.5"),
                line("I2C_SDA", "2.5")
            ),
            format!("{}\n{}\n", line("I2C_SCL", "0"), line("I2C_SDA", "2.5")),
            format!("{}\n{}\n", line("I2C_SCL", "-1"), line("I2C_SDA", "2.5")),
            format!("{}\n{}\n", line("I2C_SCL", "null"), line("I2C_SDA", "2.5")),
            format!("{}\n{}\n", line("I2C_SCL", "\"1\""), line("I2C_SDA", "2.5")),
            format!("{}\n{}\n", line("I2C_SCL", "1e400"), line("I2C_SDA", "2.5")),
            format!(
                "{}\n{}\n",
                line("I2C_SCL", "1.5").replace("44.2", "45"),
                line("I2C_SDA", "2.5")
            ),
            format!(
                "{}\n{}\n",
                line("I2C_SCL", "1.5").replace("interpolated", "meas"),
                line("I2C_SDA", "2.5")
            ),
            format!(
                "{}\n{}\n",
                line("I2C_SCL", "1.5").replace('}', r#","exceeds":false}"#),
                line("I2C_SDA", "2.5")
            ),
            format!(
                r#"{{"net":"I2C_SCL","error":"methods_disagree","message":"m"}}{}{}"#,
                "\n",
                line("I2C_SDA", "2.5")
            ),
        ];
        for (i, out) in refused.iter().enumerate() {
            assert!(answers(&d, out).is_err(), "case {i} was accepted: {out}");
        }
    }

    #[test]
    fn a_line_past_its_ceiling_faults_its_bus_once() {
        let d = decks();
        let rises = |a: f64, b: f64| [Answer::Rise(a), Answer::Rise(b)];
        let pass = judge(&d, &rises(3982.3, 186.4));
        assert_eq!(pass.analog.len(), 2);
        assert!(pass.analog[0].exceeds && !pass.analog[1].exceeds);
        assert_eq!(pass.faulted(), BTreeSet::from(["SENSORS".to_owned()]));
        assert_eq!(pass.faults[0].nets, ["I2C_SCL"]);
        assert_eq!(pass.findings.len(), 1);
        let pass = judge(&d, &rises(3982.3, 3982.3));
        assert_eq!((pass.faults.len(), pass.findings.len()), (1, 2));
        assert_eq!(pass.faults[0].nets, ["I2C_SCL", "I2C_SDA"]);
        // At the ceiling exactly is not past it.
        let pass = judge(&d, &rises(300.0, 186.4));
        assert!(pass.faults.is_empty() && pass.findings.is_empty());
        assert_eq!(pass.analog[0].rise_30_70_ns, Some(300.0));
    }

    /// Ruled F7 (b) on PR #70: a rise unfinished in the window and a line
    /// never low are faults of the design, never a stop.
    #[test]
    fn an_unfinished_rise_and_a_line_stuck_high_fault_the_bus() {
        let d = decks();
        let pass = judge(&d, &[Answer::Incomplete(13298.0), Answer::Rise(186.4)]);
        let r = &pass.analog[0];
        assert_eq!(
            (r.kind, r.complete, r.exceeds),
            ("analog.rise_time_measured", false, true)
        );
        assert_eq!(
            (r.rise_30_70_ns, r.rise_30_70_ns_lower_bound),
            (None, Some(13298.0))
        );
        assert_eq!(pass.faults[0].from, [PAST_CEILING]);
        assert_eq!(pass.findings[0].kind, "analog_rise_exceeds_ceiling");
        assert!(
            pass.findings[0]
                .message
                .contains("more than 13298 ns (a lower bound)")
        );

        let pass = judge(&d, &[Answer::StuckHigh(1.65), Answer::Rise(3982.3)]);
        let r = &pass.analog[0];
        assert_eq!(
            (r.kind, r.complete, r.exceeds),
            ("analog.line_stuck_high", false, false)
        );
        assert_eq!(r.volts_at_release, Some(1.65));
        assert_eq!(pass.faults.len(), 1);
        assert_eq!(pass.faults[0].nets, ["I2C_SCL", "I2C_SDA"]);
        assert_eq!(pass.faults[0].from, [STUCK_HIGH, PAST_CEILING]);
        let kinds: Vec<&str> = pass.findings.iter().map(|f| f.kind.as_str()).collect();
        assert_eq!(
            kinds,
            ["analog_line_stuck_high", "analog_rise_exceeds_ceiling"]
        );
    }
}
