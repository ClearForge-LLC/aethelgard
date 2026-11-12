//! `build`: the firmware ELF of a board crate (`AET-WO-3301` §1.2).
//!
//! The crate directory is the board crate `generate` writes, which holds its
//! firmware crate in `firmware/`: the firmware's one dependency is the board
//! crate at `..`, so the copy must hold both. Cargo runs as `prove` runs it,
//! on a copy, in the sandbox, with an empty environment but for `RUSTUP_HOME`,
//! `PATH`, `CARGO_TARGET_DIR` and, when the caller set it, `RUSTUP_TOOLCHAIN`.
//!
//! **Path independence (`AET-WO-3200` F11).** Cargo hashes a path
//! dependency's absolute path into every symbol name, so the same crate built
//! at two paths gives two files that differ only in `.strtab`.
//! `--remap-path-prefix` does not reach those hashes (measured). The build
//! therefore strips symbols, with `--config profile.release.strip="symbols"`
//! on cargo's own command line, which a crate's manifest or its
//! `.cargo/config.toml` cannot override. The loaded image, which is what
//! `simulate` judges (`image_sha256`), is unchanged by the strip (measured).

use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::input::{self, MAX_INPUT_BYTES};
use crate::run::{self, Stop};
use crate::said::{self, Style};
use crate::verdict::{self, Finding};

/// The target every generated firmware is built for.
pub const TARGET: &str = "thumbv7em-none-eabihf";

/// The cargo configuration `build` adds: no symbols, so no build path.
pub const STRIP: &str = "profile.release.strip=\"symbols\"";

/// The arguments cargo is given, in order, after `build`.
pub fn cargo_args() -> Vec<OsString> {
    run::args([
        "build",
        "--release",
        "--offline",
        "--locked",
        "--target",
        TARGET,
        "--config",
        STRIP,
        "--message-format=json-render-diagnostics",
    ])
}

/// The lockfiles of a board crate and its firmware may name only path
/// packages: a `source` is a registry or git dependency (`RUL-04`).
fn dependency_free(copy: &Path) -> Result<(), Stop> {
    if !copy.join("firmware/Cargo.toml").is_file() {
        return Err(Stop::refused(
            "NotABoardCrate",
            "the crate holds no firmware/Cargo.toml: build takes the board crate directory `generate` writes"
                .to_owned(),
        ));
    }
    for lock in ["Cargo.lock", "firmware/Cargo.lock"] {
        let text = fs::read_to_string(copy.join(lock)).map_err(|e| {
            Stop::refused(
                "CrateHasDependency",
                format!("the crate's {lock} cannot be read ({e}), so its dependencies are unknown"),
            )
        })?;
        if let Some(line) = text.lines().find(|l| l.trim_start().starts_with("source")) {
            return Err(Stop::refused(
                "CrateHasDependency",
                format!(
                    "{lock} names a package from outside the crate (`{}`); a board crate has no dependencies (RUL-04)",
                    line.trim()
                ),
            ));
        }
    }
    Ok(())
}

/// The one executable cargo reports, which must be a regular file inside
/// `target`.
fn artifact(stdout: &[u8], target: &Path) -> Result<PathBuf, Stop> {
    let mut found = Vec::new();
    for line in String::from_utf8_lossy(stdout).lines() {
        let Ok(message) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if message["reason"] == "compiler-artifact"
            && let Some(exe) = message["executable"].as_str()
        {
            found.push(PathBuf::from(exe));
        }
    }
    match found.as_slice() {
        [one] if one.starts_with(target) && one.is_file() => Ok(one.clone()),
        [one] => Err(Stop::tool_error(
            "cargo",
            format!(
                "cargo reported the executable {}, which is not a file in its target directory",
                one.display()
            ),
        )),
        many => Err(Stop::tool_error(
            "cargo",
            format!(
                "cargo exited 0 but reported {} executables, not one",
                many.len()
            ),
        )),
    }
}

/// Copies the built ELF to `out`, which must not exist, and returns its sha256.
fn deliver(elf: &Path, out: &Path) -> Result<String, Stop> {
    let mut bytes = Vec::new();
    fs::File::open(elf)
        .and_then(|f| f.take(MAX_INPUT_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|e| Stop::tool_error("cargo", format!("cannot read the built ELF: {e}")))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(Stop::tool_error(
            "cargo",
            format!("the built ELF exceeds {MAX_INPUT_BYTES} bytes"),
        ));
    }
    // Never a file that exists, never through a symlink.
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o644)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(out)
        .and_then(|mut f| f.write_all(&bytes))
        .map_err(|e| unwritable(out, &e.to_string()))?;
    Ok(input::hex(<sha2::Sha256 as sha2::Digest>::digest(&bytes)))
}

fn unwritable(out: &Path, why: &str) -> Stop {
    Stop::refused(
        "UnwritableOutput",
        format!("cannot write the ELF to {}: {why}", out.display()),
    )
}

/// `aethelgard build --crate <dir> --out <file>`.
pub fn build(crate_dir: &Path, out: &Path) -> Result<verdict::Verdict, Stop> {
    input::dir_check(crate_dir)?;
    if fs::symlink_metadata(out).is_ok() {
        return Err(unwritable(out, "it exists"));
    }
    // The copy is checked before any tool is looked for, so a crate with a
    // dependency is refused whatever is installed.
    let scratch = crate::Scratch::new()?;
    let input = input::crate_dir(crate_dir, &scratch.tool.join("crate"))?;
    dependency_free(&input.copy)?;
    let cargo = run::Tool::find("cargo")?;
    let rustup_home = run::required_var("RUSTUP_HOME", true)?;
    let path_var = run::required_var("PATH", false)?;
    let target = scratch.tool.join("target");
    let mut env = vec![
        ("RUSTUP_HOME", rustup_home.clone()),
        ("PATH", path_var),
        ("CARGO_TARGET_DIR", target.clone().into_os_string()),
    ];
    if let Some(toolchain) = std::env::var_os("RUSTUP_TOOLCHAIN").filter(|v| !v.is_empty()) {
        env.push(("RUSTUP_TOOLCHAIN", toolchain));
    }
    let mut read_only = cargo.dirs();
    read_only.push(PathBuf::from(&rustup_home));
    let firmware = input.copy.join("firmware");
    let invocation = |args: Vec<OsString>, status: &str| run::Invocation {
        tool: &cargo,
        args,
        env: env.clone(),
        workdir: &scratch.tool,
        cwd: firmware.clone(),
        status: scratch.root.join(status),
        read_only: read_only.clone(),
        // Cargo starts its own children with stdin from /dev/null.
        read_write: vec![PathBuf::from("/dev/null")],
    };
    let version = crate::version(invocation(run::args(["--version"]), "status-version"))?;
    let ran = invocation(cargo_args(), "status-run").run()?;
    let code = ran.code("cargo")?;
    let (findings, elf) = match code {
        0 => {
            let built = artifact(&ran.stdout, &target)?;
            (Vec::new(), Some(built))
        }
        // Cargo's one failure code. Its `error:` lines are the second witness.
        101 => match said::first_error(&String::from_utf8_lossy(&ran.stderr), Style::Cargo) {
            Some(line) => (vec![Finding::plain("build_failed", line)], None),
            None => {
                return Err(Stop::tool_error(
                    "cargo",
                    "cargo exited 101 but printed no error line".to_owned(),
                ));
            }
        },
        other => {
            return Err(Stop::tool_error(
                "cargo",
                format!(
                    "cargo exited {other}, which is neither 0 nor 101{}",
                    said::suffix(&ran.text(), Style::Cargo)
                ),
            ));
        }
    };
    let mut v = verdict::Verdict::new(
        "build",
        &input,
        &cargo,
        version,
        code,
        ran.sandbox,
        findings,
    );
    if let Some(built) = elf {
        let sha256 = deliver(&built, out)?;
        let path = fs::canonicalize(out).unwrap_or_else(|_| out.to_path_buf());
        v.elf = Some(verdict::InputReport {
            path: path.display().to_string(),
            sha256,
        });
    }
    Ok(v)
}
