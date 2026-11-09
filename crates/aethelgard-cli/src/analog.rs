//! `simulate --firmware`'s analog pre-pass (`AET-WO-2200` §1.4, `RUL-17`).
//!
//! Before the boot, each I2C line of the solved topology is measured:
//! 1. `aethelgard-twin::analog` writes one netlist per line;
//! 2. the bridge, a sandboxed tool beside this executable, measures each
//!    line's 30 %–70 % rise time with libngspice;
//! 3. each measurement is recorded as `analog.rise_time_measured`;
//! 4. a line past its ceiling faults its bus: `bridge.fault_state` is
//!    recorded, and the bus's devices are left out of the platform the boot
//!    loads, so the fault applies from the first instruction (ruling L5).
//!
//! The measurement and the firmware's effect are separate records, and the
//! effect is labelled as the model's rendering of a NACK, never as a measured
//! waveform (`AET-WO-2101` S6). A bridge that is missing, fails, is killed or
//! answers anything but one measurement per line stops the call: nothing is
//! read as "no fault" (`INV-06`).

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

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

/// The bridge beside this executable, or `bridge_unavailable` (exit 4).
pub fn bridge() -> Result<run::Tool, run::Stop> {
    let unavailable = |message: String| run::Stop {
        code: crate::EXIT_TOOL_UNAVAILABLE,
        kind: "bridge_unavailable",
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

/// Measures every analog line of `topology` in `dir`, a directory of its own
/// that only the bridge writes; `status` is the reaper's, outside it.
pub fn pass(topology: &SolvedTopology, dir: &Path, status: &Path) -> Result<Pass, run::Stop> {
    let decks = analog::decks(topology).map_err(crate::twin_stop)?;
    let tool = bridge()?;
    fs::DirBuilder::new()
        .mode(0o700)
        .create(dir)
        .map_err(|e| run::Stop::tool_error(BRIDGE, format!("cannot create its directory: {e}")))?;
    let request = dir.join("request.json");
    fs::write(&request, request_json(&decks).to_string())
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
    let measured = answers(&decks, &stdout).map_err(|e| run::Stop::tool_error(BRIDGE, e))?;
    Ok(judge(&decks, &measured))
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

/// One measurement per deck, in the request's order, each exactly the
/// bridge's documented line; anything else is refused.
fn answers(decks: &[Deck], stdout: &str) -> Result<Vec<f64>, String> {
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
            let fields = v.as_object().filter(|o| {
                o.len() == 4
                    && o.get("net").and_then(Value::as_str) == Some(deck.net.as_str())
                    && o.get("method").and_then(Value::as_str) == Some("interpolated")
                    && o.get("ngspice").and_then(Value::as_str) == Some(NGSPICE)
            });
            let rise = fields
                .and_then(|o| o.get("rise_30_70_ns"))
                .and_then(Value::as_f64)
                .filter(|r| r.is_finite() && *r > 0.0);
            rise.ok_or_else(|| format!("`{line}` is not the measurement of `{}`", deck.net))
        })
        .collect()
}

fn judge(decks: &[Deck], measured: &[f64]) -> Pass {
    let mut analog_records = Vec::new();
    let mut findings = Vec::new();
    let mut faulted: Vec<(String, Vec<String>)> = Vec::new();
    for (deck, &rise) in decks.iter().zip(measured) {
        let exceeds = rise > f64::from(deck.ceiling_ns);
        analog_records.push(AnalogRecord {
            kind: "analog.rise_time_measured",
            source: "ngspice",
            bus: deck.bus.clone(),
            net: deck.net.clone(),
            rise_30_70_ns: rise,
            ceiling_ns: deck.ceiling_ns,
            exceeds,
            method: "interpolated",
            ngspice: NGSPICE,
        });
        if exceeds {
            findings.push(Finding::plain(
                "analog_rise_exceeds_ceiling",
                format!(
                    "bus `{}`: net `{}` rose from 30 % to 70 % in {rise} ns, past its {} ns ceiling",
                    deck.bus, deck.net, deck.ceiling_ns
                ),
            ));
            match faulted.iter_mut().find(|(bus, _)| bus == &deck.bus) {
                Some((_, nets)) => nets.push(deck.net.clone()),
                None => faulted.push((deck.bus.clone(), vec![deck.net.clone()])),
            }
        }
    }
    faulted.sort();
    let faults = faulted
        .into_iter()
        .map(|(bus, nets)| FaultRecord {
            kind: "bridge.fault_state",
            source: "aethelgard",
            bus,
            fault: "absent",
            from: "analog.rise_time_measured > ceiling_ns",
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
            r#"{"schema_version":1,"specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","assignments":{"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},"I2C_SDA":{"pin":"PB7","peripheral":"I2C1","signal":"I2C1_SDA","alternate_function":4}},"gpio":{},"i2c_ceilings":{"SENSORS":400000},"analog_lines":{"I2C_SCL":{"bus":"SENSORS","pull_up_ohms":47000,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300},"I2C_SDA":{"bus":"SENSORS","pull_up_ohms":47000,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300}}}"#,
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
    fn every_answer_but_one_measurement_per_net_in_order_is_refused() {
        let d = decks();
        let good = format!("{}\n{}\n", line("I2C_SCL", "1.5"), line("I2C_SDA", "2.5"));
        assert_eq!(answers(&d, &good).unwrap(), [1.5, 2.5]);
        let refused = [
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
                r#"{{"net":"I2C_SCL","error":"rise_incomplete","message":"m"}}{}{}"#,
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
        let pass = judge(&d, &[3982.3, 186.4]);
        assert_eq!(pass.analog.len(), 2);
        assert!(pass.analog[0].exceeds && !pass.analog[1].exceeds);
        assert_eq!(pass.faulted(), BTreeSet::from(["SENSORS".to_owned()]));
        assert_eq!(pass.faults[0].nets, ["I2C_SCL"]);
        assert_eq!(pass.findings.len(), 1);
        let pass = judge(&d, &[3982.3, 3982.3]);
        assert_eq!((pass.faults.len(), pass.findings.len()), (1, 2));
        assert_eq!(pass.faults[0].nets, ["I2C_SCL", "I2C_SDA"]);
        // At the ceiling exactly is not past it.
        let pass = judge(&d, &[300.0, 186.4]);
        assert!(pass.faults.is_empty() && pass.findings.is_empty());
    }
}
