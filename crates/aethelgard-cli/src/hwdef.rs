//! `generate hwdef` (`AET-WO-4400b` D8): ArduPilot's `hwdef.dat` from a
//! `SolvedTopology` and the parts tables alone (`RUL-16`), by
//! `aethelgard-hwdef`. It starts no process and never overwrites a file.

use std::fs;
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::Path;

use serde::Serialize;
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
    /// The board's `defaults.parm`, written beside the file (`AET-WO-4412`
    /// (3)): where, and its sha256; absent for a board that needs none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults_sha256: Option<String>,
    pub not_emitted: Vec<String>,
}

/// Where `out`'s `defaults.parm` goes: beside it, `<name>.defaults.parm` for
/// `<name>.hwdef.dat`, else `<out>.defaults.parm` (`AET-WO-4412` (3)), as
/// `container/ardupilot-build.sh` installs it.
pub fn defaults_path(out: &Path) -> std::path::PathBuf {
    let name = out
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = name.strip_suffix(".hwdef.dat").unwrap_or(&name);
    out.with_file_name(format!("{stem}.defaults.parm"))
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
    let t = crate::pinout::parse_topology(&bytes)?;
    if fs::symlink_metadata(out).is_ok() {
        return Err(unwritable(out, "it exists"));
    }
    let hwdef = aethelgard_hwdef::generate(&t).map_err(|err| Stop {
        code: EXIT_INVALID_SPEC,
        kind: err.kind(),
        id: err.id().map(str::to_owned),
        message: err.to_string(),
    })?;
    let defaults = hwdef.defaults.as_ref().map(|d| (defaults_path(out), d));
    if let Some((path, _)) = &defaults
        && fs::symlink_metadata(path).is_ok()
    {
        return Err(unwritable(path, "it exists"));
    }
    // Never a file that exists, never through a symlink.
    let write = |path: &Path, text: &str| {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o644)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(path)
            .and_then(|mut f| f.write_all(text.as_bytes()))
            .map_err(|e| unwritable(path, &e.to_string()))
    };
    write(out, &hwdef.text)?;
    if let Some((path, text)) = &defaults {
        write(path, text)?;
    }
    Ok(Report {
        ok: true,
        generator: "hwdef",
        topology_sha256: hex(Sha256::digest(&bytes)),
        hwdef_sha256: hex(Sha256::digest(hwdef.text.as_bytes())),
        defaults: defaults.as_ref().map(|(p, _)| p.display().to_string()),
        defaults_sha256: defaults
            .as_ref()
            .map(|(_, t)| hex(Sha256::digest(t.as_bytes()))),
        not_emitted: hwdef.not_emitted,
    })
}

fn unwritable(out: &Path, why: &str) -> Stop {
    Stop::refused(
        "UnwritableOutput",
        format!("cannot write {}: {why}", out.display()),
    )
}
