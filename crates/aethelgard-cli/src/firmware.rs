//! `build`: the firmware ELF of a board crate (`AET-WO-3301` §1.2).
//!
//! The crate directory is the board crate `generate` writes, which holds its
//! firmware crate in `firmware/`: the firmware's one dependency is the board
//! crate at `..`, so the copy must hold both. Cargo runs as `prove` runs it,
//! on a copy, in the sandbox, with an empty environment but for `RUSTUP_HOME`,
//! `PATH`, `CARGO_TARGET_DIR` and, when the caller set it, `RUSTUP_TOOLCHAIN`.
//!
//! **Path independence (`AET-WO-3200` F11), for the generated crate.** Cargo
//! hashes a path dependency's absolute path into every symbol name, so the
//! same crate built at two paths gives two files that differ only in
//! `.strtab`. `--remap-path-prefix` does not reach those hashes (measured).
//! The build therefore strips symbols, with
//! `--config profile.release.strip="symbols"` on cargo's own command line,
//! which the crate's manifest profile does not override (measured). A
//! supplied crate can still make its ELF path-dependent: rustflags in its
//! `.cargo/config.toml` come after the profile's strip, and its code can
//! embed `env!("CARGO_MANIFEST_DIR")` (adversarial B6). The loaded image,
//! which is what `simulate` judges (`image_sha256`), is unchanged by the
//! strip (measured).

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

/// What cargo's JSON messages say: whether the build finished and succeeded
/// (`build-finished`), every manifest a unit was built from, and every
/// executable.
struct Messages {
    finished: Option<bool>,
    manifests: Vec<PathBuf>,
    executables: Vec<PathBuf>,
    /// The package of each build script cargo ran.
    build_scripts: Vec<String>,
}

fn messages(stdout: &[u8]) -> Messages {
    let mut m = Messages {
        finished: None,
        manifests: Vec::new(),
        executables: Vec::new(),
        build_scripts: Vec::new(),
    };
    for line in String::from_utf8_lossy(stdout).lines() {
        let Ok(message) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        match message["reason"].as_str() {
            Some("build-finished") => m.finished = message["success"].as_bool(),
            Some("build-script-executed") => m
                .build_scripts
                .push(message["package_id"].as_str().unwrap_or("?").to_owned()),
            Some("compiler-artifact") => {
                if let Some(manifest) = message["manifest_path"].as_str() {
                    m.manifests.push(PathBuf::from(manifest));
                }
                if let Some(exe) = message["executable"].as_str() {
                    m.executables.push(PathBuf::from(exe));
                }
            }
            _ => {}
        }
    }
    m
}

/// The sources cargo's dep-info for `exe` lists (`<exe>.d`, written beside
/// it): every file any unit read, `#[path]` modules and `include_bytes!`
/// targets included, the toolchain's own excluded. A space inside a path is
/// written `\ `.
fn dep_info(exe: &Path) -> Result<Vec<PathBuf>, Stop> {
    let mut d = exe.as_os_str().to_owned();
    d.push(".d");
    let text = fs::read_to_string(&d).map_err(|e| {
        Stop::tool_error(
            "cargo",
            format!("cargo wrote no dep-info beside the ELF ({e})"),
        )
    })?;
    let Some((_, deps)) = text.lines().next().and_then(|l| l.split_once(": ")) else {
        return Err(Stop::tool_error(
            "cargo",
            "cargo's dep-info names no source".to_owned(),
        ));
    };
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = deps.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&' ') => current.push(chars.next().unwrap_or(' ')),
            ' ' => {
                if !current.is_empty() {
                    out.push(PathBuf::from(std::mem::take(&mut current)));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        out.push(PathBuf::from(current));
    }
    Ok(out)
}

/// Every unit cargo built came from the copy, and read only the copy's
/// files, each path resolved (`..` and symlinks) before it is compared: a
/// path dependency outside the crate, a `[patch]` or `paths` override, a
/// `#[path]` module, an `include_bytes!` or a `[lib] path` outside it is code
/// the crate hash does not cover (`RUL-04`, adversarial B5). No build script
/// may run: a board crate has none, and one reads what dep-info never lists.
/// Linker inputs named in rustflags are not in dep-info; they can come only
/// from what the sandbox lets cargo read (FEEDBACK F12).
fn built_from_the_copy(m: &Messages, sources: &[PathBuf], copy: &Path) -> Result<(), Stop> {
    let refuse = |message: String| Err(Stop::refused("CrateHasDependency", message));
    if let Some(package) = m.build_scripts.first() {
        return refuse(format!(
            "cargo ran a build script of {package}; a board crate has none (RUL-04)"
        ));
    }
    let root = fs::canonicalize(copy)
        .map_err(|e| Stop::tool_error("cargo", format!("cannot resolve the copy: {e}")))?;
    for path in m.manifests.iter().chain(sources) {
        match fs::canonicalize(path) {
            Ok(resolved) if resolved.starts_with(&root) => {}
            Ok(resolved) => {
                return refuse(format!(
                    "cargo built from {}, which is outside the crate; a board crate has no dependencies (RUL-04)",
                    resolved.display()
                ));
            }
            Err(e) => {
                return refuse(format!(
                    "cargo built from {}, which cannot be resolved ({e})",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

/// The build's outcome from its two witnesses: `Some(true)` built,
/// `Some(false)` the crate failed to build, `None` neither (`ToolError`).
fn classify(code: i32, finished: Option<bool>) -> Option<bool> {
    match (code, finished) {
        (0, Some(true)) => Some(true),
        (101, Some(false)) => Some(false),
        _ => None,
    }
}

/// The one executable cargo reports, which must be a regular file inside
/// `target`.
fn artifact(m: &Messages, target: &Path) -> Result<PathBuf, Stop> {
    match m.executables.as_slice() {
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
    // Two witnesses: cargo's exit code and its `build-finished` message. Exit
    // 101 is every cargo failure, a configuration it cannot load included, so
    // only 101 with a build that finished unsuccessfully is the crate's fault.
    let m = messages(&ran.stdout);
    let (findings, elf) = match classify(code, m.finished) {
        Some(true) => {
            let elf = artifact(&m, &target)?;
            built_from_the_copy(&m, &dep_info(&elf)?, &input.copy)?;
            (Vec::new(), Some(elf))
        }
        Some(false) => {
            let line = said::first_error(&String::from_utf8_lossy(&ran.stderr), Style::Cargo)
                .unwrap_or_else(|| "the crate did not build".to_owned());
            (vec![Finding::plain("build_failed", line)], None)
        }
        None => {
            return Err(Stop::tool_error(
                "cargo",
                format!(
                    "cargo exited {code} with build-finished {:?}, which is neither a build nor a failed one{}",
                    m.finished,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Adversarial B5: a unit built from a manifest outside the copy, a
    /// source read from outside it (spelled absolute or through `..`), and a
    /// build script are each refused, whatever the lockfile says.
    #[test]
    fn a_unit_or_source_from_outside_the_copy_and_a_build_script_are_refused() {
        let base = std::env::temp_dir().join(format!("aet-3301-outside-{}", std::process::id()));
        let copy = base.join("crate");
        fs::create_dir_all(copy.join("src")).unwrap();
        fs::create_dir_all(base.join("evil")).unwrap();
        for f in ["Cargo.toml", "src/lib.rs"] {
            fs::write(copy.join(f), "").unwrap();
        }
        fs::write(base.join("evil/inject.rs"), "").unwrap();
        let built = format!(
            "{{\"reason\":\"compiler-artifact\",\"manifest_path\":\"{}\",\"executable\":null}}\n{{\"reason\":\"build-finished\",\"success\":true}}\n",
            copy.join("Cargo.toml").display()
        );
        let m = messages(built.as_bytes());
        assert_eq!(m.finished, Some(true));
        let inside = [copy.join("src/lib.rs")];
        assert!(built_from_the_copy(&m, &inside, &copy).is_ok());
        for outside in [
            base.join("evil/inject.rs"),
            copy.join("src/../../evil/inject.rs"),
        ] {
            let read = [inside[0].clone(), outside.clone()];
            let e = built_from_the_copy(&m, &read, &copy).unwrap_err();
            assert_eq!(e.kind, "CrateHasDependency", "{}", outside.display());
            assert!(e.message.contains("evil/inject.rs"), "{}", e.message);
        }
        let other = built.replace(
            &copy.join("Cargo.toml").display().to_string(),
            &base.join("evil/inject.rs").display().to_string(),
        );
        let e = built_from_the_copy(&messages(other.as_bytes()), &inside, &copy).unwrap_err();
        assert_eq!(e.kind, "CrateHasDependency");
        let script = format!(
            "{built}{{\"reason\":\"build-script-executed\",\"package_id\":\"path+file:///x#evil@0.0.0\"}}\n"
        );
        let e = built_from_the_copy(&messages(script.as_bytes()), &inside, &copy).unwrap_err();
        assert!(e.message.contains("build script"), "{}", e.message);
        fs::remove_dir_all(&base).unwrap();
    }

    /// Adversarial B7 and mutants M21, M22: only both witnesses agreeing is a
    /// build or a failed one.
    #[test]
    fn only_both_witnesses_agreeing_is_a_build_or_a_failure() {
        assert_eq!(classify(0, Some(true)), Some(true));
        assert_eq!(classify(101, Some(false)), Some(false));
        for (code, finished) in [
            (0, None),
            (0, Some(false)),
            (101, None),
            (101, Some(true)),
            (1, Some(false)),
        ] {
            assert_eq!(classify(code, finished), None, "{code} {finished:?}");
        }
    }

    /// Adversarial B7: no `build-finished` (cargo could not load its
    /// configuration) is not a finished build of either kind.
    #[test]
    fn build_finished_is_read_and_its_absence_is_none() {
        assert_eq!(
            messages(b"{\"reason\":\"build-finished\",\"success\":false}\n").finished,
            Some(false)
        );
        assert_eq!(messages(b"").finished, None);
        assert_eq!(
            messages(b"error: could not load Cargo configuration\n").finished,
            None
        );
    }

    /// Cargo's dep-info, as measured: the output, then every source, with a
    /// space in a path escaped.
    #[test]
    fn dep_info_lists_every_source() {
        let dir = std::env::temp_dir().join(format!("aet-3301-depinfo-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("fw");
        fs::write(
            dir.join("fw.d"),
            "/t/fw: /x/inject.rs /s/crate/firmware/src/main.rs /s/my\\ dir/lib.rs\n\n/x/inject.rs:\n",
        )
        .unwrap();
        assert_eq!(
            dep_info(&exe).unwrap(),
            [
                PathBuf::from("/x/inject.rs"),
                PathBuf::from("/s/crate/firmware/src/main.rs"),
                PathBuf::from("/s/my dir/lib.rs"),
            ]
        );
        fs::remove_file(dir.join("fw.d")).unwrap();
        assert_eq!(dep_info(&exe).unwrap_err().kind, "ToolError");
        fs::remove_dir_all(&dir).unwrap();
    }
}
