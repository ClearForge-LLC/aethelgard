//! `check drc` and `check erc`: `kicad-cli` with `--exit-code-violations`
//! (`RUL-03`) and a JSON report. Two witnesses: the report's findings and the
//! exit code must agree (0 with none, 5 with some), and the report must name
//! the version that `kicad-cli version` printed and the default severities.

use std::fs::OpenOptions;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use rustix::fs::OFlags;
use serde_json::Value;

use crate::run::{MAX_CAPTURE, Stop};
use crate::verdict::{Finding, Where};

#[derive(Clone, Copy)]
pub enum Check {
    Drc,
    Erc,
}

impl Check {
    pub fn name(self) -> &'static str {
        self.verb()[1]
    }
    /// The `kicad-cli` subcommand.
    pub fn verb(self) -> [&'static str; 2] {
        match self {
            Check::Drc => ["pcb", "drc"],
            Check::Erc => ["sch", "erc"],
        }
    }
    /// The file name the copy is given; KiCad reads the extension.
    pub fn copy_name(self) -> String {
        format!("input.kicad_{}", self.verb()[0])
    }
}

const TOOL: &str = "kicad-cli";

/// The severities KiCad must report it included, and the only ones a finding may have.
pub const SEVERITIES: [&str; 2] = ["error", "warning"];

/// Judges a run from its exit code and its report file.
pub fn judge(check: Check, code: i32, report: &Path, version: &str) -> Result<Vec<Finding>, Stop> {
    let doc = read_report(report)?;
    let field = |key: &str| {
        doc.get(key)
            .ok_or_else(|| Stop::tool_error(TOOL, format!("the report has no `{key}`")))
    };
    let schema = format!("https://schemas.kicad.org/{}.v1.json", check.name());
    if field("$schema")?.as_str() != Some(&schema) {
        return Err(Stop::tool_error(
            TOOL,
            format!("the report's $schema is not {schema}"),
        ));
    }
    if field("kicad_version")?.as_str() != Some(version) {
        return Err(Stop::tool_error(
            TOOL,
            format!(
                "the report names kicad_version {}, but `{TOOL} version` printed {version}",
                field("kicad_version")?
            ),
        ));
    }
    let severities = field("included_severities")?;
    if *severities != serde_json::json!(SEVERITIES) {
        return Err(Stop::tool_error(
            TOOL,
            format!("the report's included_severities is {severities}, not {SEVERITIES:?}"),
        ));
    }
    let mut findings = Vec::new();
    match check {
        Check::Drc => {
            for key in ["violations", "unconnected_items", "schematic_parity"] {
                findings.extend(parse_findings(field(key)?)?);
            }
        }
        Check::Erc => {
            let sheets = field("sheets")?
                .as_array()
                .ok_or_else(|| Stop::tool_error(TOOL, "`sheets` is not an array".to_owned()))?;
            for sheet in sheets {
                let violations = sheet.get("violations").ok_or_else(|| {
                    Stop::tool_error(TOOL, "a sheet has no `violations`".to_owned())
                })?;
                findings.extend(parse_findings(violations)?);
            }
        }
    }
    match (code, findings.is_empty()) {
        (0, true) | (5, false) => Ok(findings),
        _ => Err(Stop::tool_error(
            TOOL,
            format!(
                "{TOOL} exited {code} with {} finding(s) in the report; the codes measured for it are 0 with none and 5 with some",
                findings.len()
            ),
        )),
    }
}

/// The tool wrote the report where it could also have put a symlink or a
/// FIFO; neither is followed or waited on, so only a regular file is read.
fn read_report(path: &Path) -> Result<Value, Stop> {
    let no_report = |e: std::io::Error| Stop::tool_error(TOOL, format!("no report: {e}"));
    let file = OpenOptions::new()
        .read(true)
        .custom_flags((OFlags::NOFOLLOW | OFlags::NONBLOCK).bits() as i32)
        .open(path)
        .map_err(no_report)?;
    if !file.metadata().map_err(no_report)?.is_file() {
        return Err(Stop::tool_error(
            TOOL,
            "the report is not a regular file".to_owned(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_CAPTURE as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(no_report)?;
    if bytes.len() > MAX_CAPTURE {
        return Err(Stop::tool_error(
            TOOL,
            format!("the report exceeds {MAX_CAPTURE} bytes"),
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|e| Stop::tool_error(TOOL, format!("the report is not JSON: {e}")))
}

/// Each finding must have exactly the shape KiCad 10 writes; anything else is
/// a report this CLI does not understand, so not a verdict.
fn parse_findings(list: &Value) -> Result<Vec<Finding>, Stop> {
    let bad = |what: &str| Stop::tool_error(TOOL, format!("a finding has no valid {what}"));
    let items = list.as_array().ok_or_else(|| bad("list"))?;
    items
        .iter()
        .map(|f| {
            let text = |key: &str| {
                f.get(key)
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .ok_or_else(|| bad(key))
            };
            let severity = text("severity")?;
            if !SEVERITIES.contains(&severity.as_str()) {
                return Err(bad("severity"));
            }
            let location = f
                .get("items")
                .and_then(Value::as_array)
                .ok_or_else(|| bad("items"))?
                .iter()
                .map(|i| {
                    let pos = |axis: &str| {
                        i.get("pos")
                            .and_then(|p| p.get(axis))
                            .and_then(Value::as_f64)
                            .ok_or_else(|| bad("pos"))
                    };
                    Ok(Where {
                        description: i
                            .get("description")
                            .and_then(Value::as_str)
                            .ok_or_else(|| bad("item description"))?
                            .to_owned(),
                        x: pos("x")?,
                        y: pos("y")?,
                        uuid: i
                            .get("uuid")
                            .and_then(Value::as_str)
                            .ok_or_else(|| bad("uuid"))?
                            .to_owned(),
                    })
                })
                .collect::<Result<Vec<_>, Stop>>()?;
            Ok(Finding {
                kind: text("type")?,
                severity,
                message: text("description")?,
                location,
            })
        })
        .collect()
}
