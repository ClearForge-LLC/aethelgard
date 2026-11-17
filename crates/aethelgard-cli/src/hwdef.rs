//! `generate hwdef` (`AET-WO-4400b` D8): ArduPilot's `hwdef.dat` from a
//! `SolvedTopology` and the parts tables alone (`RUL-16`), by
//! `aethelgard-hwdef`. It starts no process and never overwrites a file.

use std::fs;
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::Path;

use serde::Serialize;
use serde_json::error::Category;
use sha2::{Digest, Sha256};

use crate::input::hex;
use crate::run::Stop;
use crate::{EXIT_INVALID_SPEC, EXIT_UNREADABLE, read_input};

/// What was written: the input's and the file's sha256, and every fact the
/// topology holds that no line states, each with why.
#[derive(Serialize)]
pub struct Report {
    pub ok: bool,
    pub generator: &'static str,
    pub topology_sha256: String,
    pub hwdef_sha256: String,
    pub not_emitted: Vec<String>,
}

/// Reads and validates the topology, generates, and writes `out`, which must
/// not exist. A topology the generator cannot state is refused, exit 2.
pub fn generate(topology: &Path, out: &Path) -> Result<Report, Stop> {
    let bytes = read_input(Some(topology)).map_err(|r| Stop {
        code: EXIT_UNREADABLE,
        kind: r.error.kind,
        id: Some(topology.display().to_string()),
        message: r.error.message,
    })?;
    let t: aethelgard_ir::SolvedTopology = serde_json::from_slice(&bytes).map_err(|err| {
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
    if fs::symlink_metadata(out).is_ok() {
        return Err(unwritable(out, "it exists"));
    }
    let hwdef = aethelgard_hwdef::generate(&t).map_err(|err| Stop {
        code: EXIT_INVALID_SPEC,
        kind: err.kind(),
        id: err.id().map(str::to_owned),
        message: err.to_string(),
    })?;
    // Never a file that exists, never through a symlink.
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o644)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(out)
        .and_then(|mut f| f.write_all(hwdef.text.as_bytes()))
        .map_err(|e| unwritable(out, &e.to_string()))?;
    Ok(Report {
        ok: true,
        generator: "hwdef",
        topology_sha256: hex(Sha256::digest(&bytes)),
        hwdef_sha256: hex(Sha256::digest(hwdef.text.as_bytes())),
        not_emitted: hwdef.not_emitted,
    })
}

fn unwritable(out: &Path, why: &str) -> Stop {
    Stop::refused(
        "UnwritableOutput",
        format!("cannot write hwdef.dat to {}: {why}", out.display()),
    )
}
