//! `aethelgard init --template <name> --out <file>` (`AET-WO-1608`): one
//! specification a person starts from, which cannot pass until they fill it.
//!
//! Each template is an `examples/` specification with every value a person
//! must own left empty (`""`), which `validate` refuses by its own kind and
//! path until it is written (`INV-06`): the project name and every citation.
//! Nothing else differs from the example; `tests/init_tier1.rs` holds each
//! template to its example field for field. The bytes are held in the binary
//! and written verbatim: nothing is parsed, filled or defaulted.

use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::ExitCode;

use serde::Serialize;

use crate::{EXIT_UNREADABLE, Refusal, emit};

/// One template: its name on the command line, its bytes, and the JSON path
/// of each placeholder, in document order.
pub struct Template {
    pub name: &'static str,
    pub text: &'static str,
    pub placeholders: &'static [&'static str],
}

/// Every template, by name. `i2c-spi-bench` is `examples/sentinel-02-bench.json`
/// with its ten placeholders (`AET-WO-1608` D1, ruled 2026-11-23), named for
/// what it holds: engine code names no board (`board_names.rs`, roadmap §6).
pub const TEMPLATES: [Template; 1] = [Template {
    name: "i2c-spi-bench",
    text: include_str!("../templates/i2c-spi-bench.json"),
    placeholders: &[
        "project_name",
        "mcu.core_supply.source",
        "i2c_devices[0].pin_capacitance.declared.condition",
        "i2c_devices[0].pin_capacitance.declared.source",
        "spi_devices[0].pin_capacitance.declared.condition",
        "spi_devices[0].pin_capacitance.declared.source",
        "board.stackup.finished_thickness_um.source",
        "board.stackup.dielectrics[0].source",
        "board.stackup.copper_max_um.source",
        "board.stackup.relative_permittivity_max_permille.source",
    ],
}];

#[derive(Serialize)]
struct Initialized<'a> {
    ok: bool,
    template: &'a str,
    out: String,
    placeholders: &'a [&'a str],
}

/// Writes template `name` to `out`, a path that must not exist. A file,
/// a directory, a FIFO or a symlink (dangling or not) already there is
/// refused, exit 3, `UnwritableOutput`, and left as it was: the file is
/// created with `O_CREAT | O_EXCL | O_NOFOLLOW`, so nothing that exists is
/// opened, followed or waited on. No directory is created.
pub fn init(name: &str, out: &Path) -> ExitCode {
    let Some(t) = TEMPLATES.iter().find(|t| t.name == name) else {
        unreachable!("clap accepts only the names in TEMPLATES")
    };
    let written = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o644)
        .custom_flags((rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::CLOEXEC).bits() as i32)
        .open(out)
        .map_err(|e| {
            format!(
                "cannot create {}: {e}; init never overwrites",
                out.display()
            )
        })
        .and_then(|mut f| {
            f.write_all(t.text.as_bytes()).map_err(|e| {
                format!(
                    "cannot write {}: {e}; the file it created may be partial",
                    out.display()
                )
            })
        });
    match written {
        Ok(()) => emit(
            &Initialized {
                ok: true,
                template: t.name,
                out: out.display().to_string(),
                placeholders: t.placeholders,
            },
            0,
        ),
        Err(message) => emit(
            &Refusal::new("UnwritableOutput", None, message),
            EXIT_UNREADABLE,
        ),
    }
}
