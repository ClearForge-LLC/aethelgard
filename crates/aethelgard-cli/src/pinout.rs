//! `check pinout` (`AET-WO-4400a`): the pinout check of `aethelgard-pincheck`
//! over a linked firmware ELF, optionally its `hwdef.h`, a `SolvedTopology`
//! and a declared allow-list. It starts no process: the check reads bytes.
//!
//! The package is the solver's own pin table for `STM32H753ZITx`, the one
//! part the check's register layout knows. The topology names no part yet
//! (`AET-WO-4300a` adds it), so none is read from it.

use std::path::Path;

use aethelgard_pincheck::{AllowList, Judgement, Refusal as CheckRefusal};
use serde::Serialize;
use serde_json::error::Category;
use sha2::{Digest, Sha256};

use crate::input::hex;
use crate::run::Stop;
use crate::{EXIT_INTERNAL, EXIT_INVALID_SPEC, EXIT_TOOL_ERROR, EXIT_UNREADABLE, read_input};

/// The verdict: the check's judgement (`ok` first among its fields) and the
/// sha256 of every input it read.
#[derive(Serialize)]
pub struct Verdict {
    pub check: &'static str,
    pub topology_sha256: String,
    pub elf_sha256: String,
    pub hwdef_h_sha256: Option<String>,
    pub allow_sha256: Option<String>,
    #[serde(flatten)]
    pub judgement: Judgement,
}

fn sha256(bytes: &[u8]) -> String {
    hex(Sha256::digest(bytes))
}

fn read(path: &Path) -> Result<Vec<u8>, Stop> {
    read_input(Some(path)).map_err(|r| Stop {
        code: EXIT_UNREADABLE,
        kind: r.error.kind,
        id: Some(path.display().to_string()),
        message: r.error.message,
    })
}

fn refused(err: CheckRefusal) -> Stop {
    let code = match err {
        // Two witnesses of one image that disagree: never a verdict (RUL-10).
        CheckRefusal::DecodesDisagree { .. } => EXIT_TOOL_ERROR,
        CheckRefusal::InvalidPin { .. } | CheckRefusal::NoPinSpeed { .. } => EXIT_INVALID_SPEC,
        CheckRefusal::UnsupportedPart { .. } => EXIT_INTERNAL,
        CheckRefusal::Elf { .. }
        | CheckRefusal::Header { .. }
        | CheckRefusal::Conditional { .. }
        | CheckRefusal::Allow { .. } => EXIT_UNREADABLE,
    };
    Stop {
        code,
        kind: err.kind(),
        id: err.id(),
        message: err.to_string(),
    }
}

/// Reads every input, then judges.
pub fn check(
    topology: &Path,
    elf: &Path,
    hwdef_h: Option<&Path>,
    allow: Option<&Path>,
) -> Result<Verdict, Stop> {
    let topology_bytes = read(topology)?;
    let elf_bytes = read(elf)?;
    let header_bytes = hwdef_h.map(read).transpose()?;
    let allow_bytes = allow.map(read).transpose()?;

    let topology: aethelgard_ir::SolvedTopology =
        serde_json::from_slice(&topology_bytes).map_err(|err| {
            let (code, kind) = if err.classify() == Category::Data {
                (EXIT_INVALID_SPEC, "SchemaViolation")
            } else {
                (EXIT_UNREADABLE, "InvalidJson")
            };
            Stop {
                code,
                kind,
                id: None,
                message: format!("the topology: {err}"),
            }
        })?;
    topology.validate().map_err(|err| Stop {
        code: EXIT_INVALID_SPEC,
        kind: err.kind(),
        id: err.id().map(str::to_owned),
        message: format!("the topology: {err}"),
    })?;
    let config = aethelgard_pincheck::elf::read(&elf_bytes).map_err(refused)?;
    let header = match &header_bytes {
        Some(bytes) => {
            let text = std::str::from_utf8(bytes).map_err(|_| Stop {
                code: EXIT_UNREADABLE,
                kind: "MalformedHeader",
                id: None,
                message: "hwdef.h is not UTF-8".into(),
            })?;
            Some(aethelgard_pincheck::header::read(text).map_err(refused)?)
        }
        None => None,
    };
    let allow_list = match &allow_bytes {
        Some(bytes) => AllowList::parse(bytes).map_err(refused)?,
        None => AllowList::default(),
    };
    let table = aethelgard_svd_hal::pin_table(aethelgard_pincheck::PART).map_err(|err| Stop {
        code: EXIT_INTERNAL,
        kind: err.kind(),
        id: None,
        message: format!("the pin table is unusable: {err}"),
    })?;
    let judgement =
        aethelgard_pincheck::judge(&topology, &table, &config, header.as_ref(), &allow_list)
            .map_err(refused)?;
    Ok(Verdict {
        check: "pinout",
        topology_sha256: sha256(&topology_bytes),
        elf_sha256: sha256(&elf_bytes),
        hwdef_h_sha256: header_bytes.as_deref().map(sha256),
        allow_sha256: allow_bytes.as_deref().map(sha256),
        judgement,
    })
}
