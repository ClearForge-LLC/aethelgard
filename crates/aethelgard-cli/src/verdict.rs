//! The one shape every tool verdict takes (`crates/aethelgard-cli/README.md`).

use serde::Serialize;

use crate::input::Input;
use crate::run::{SandboxReport, Tool};

#[derive(Serialize)]
pub struct Verdict {
    pub ok: bool,
    pub verdict: &'static str,
    pub check: &'static str,
    pub input: InputReport,
    pub tool: ToolReport,
    pub sandbox: SandboxReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub included_severities: Option<&'static [&'static str]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub machine: Option<Machine>,
    pub findings: Vec<Finding>,
}

#[derive(Serialize)]
pub struct InputReport {
    pub path: String,
    pub sha256: String,
}

#[derive(Serialize)]
pub struct ToolReport {
    pub name: &'static str,
    pub version: String,
    pub exit_code: i32,
}

#[derive(Serialize)]
pub struct Finding {
    pub kind: String,
    pub severity: String,
    pub message: String,
    #[serde(rename = "where")]
    pub location: Vec<Where>,
}

#[derive(Serialize)]
pub struct Where {
    pub description: String,
    pub x: f64,
    pub y: f64,
    pub uuid: String,
}

#[derive(Serialize)]
pub struct Machine {
    pub cpus: Vec<String>,
    pub peripherals: Vec<Peripheral>,
}

#[derive(Serialize)]
pub struct Peripheral {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}

impl Finding {
    /// A finding with no location: the Kani and Renode kinds.
    pub fn plain(kind: &str, message: String) -> Self {
        Self {
            kind: kind.to_owned(),
            severity: "error".to_owned(),
            message,
            location: Vec::new(),
        }
    }
}

impl Verdict {
    /// A verdict is pass exactly when it has no finding; the caller decides
    /// that only after both witnesses agreed.
    pub fn new(
        check: &'static str,
        input: &Input,
        tool: &Tool,
        version: String,
        code: i32,
        sandbox: SandboxReport,
        findings: Vec<Finding>,
    ) -> Self {
        let pass = findings.is_empty();
        Self {
            ok: pass,
            verdict: if pass { "pass" } else { "fail" },
            check,
            input: InputReport {
                path: input.original.display().to_string(),
                sha256: input.sha256.clone(),
            },
            tool: ToolReport {
                name: tool.name,
                version,
                exit_code: code,
            },
            sandbox,
            included_severities: None,
            harness: None,
            machine: None,
            findings,
        }
    }
}
