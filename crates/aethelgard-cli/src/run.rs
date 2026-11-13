//! Running an external tool: finding it, sandboxing it, and capturing what it
//! did (`docs/architecture.md` §6, ruled on PR #12).
//!
//! The CLI never runs a tool directly. It starts a copy of itself as a
//! *reaper*, in a new process group (so a signal to the CLI's group does not
//! reach it), with an empty environment, stdin from `/dev/null` and its
//! working directory inside the call's private temporary directory. The reaper
//! starts a *trampoline*, a third copy, in a process group of its own; the
//! trampoline confines itself with Landlock, installs the seccomp filter that
//! refuses every socket Landlock cannot see (AET-WO-0300a) and the non-socket
//! channels to other processes (AET-WO-1600), drops every capability it can
//! (AET-WO-1600), records the sandbox it achieved, and becomes the tool. The reaper waits. If the CLI
//! dies, by any signal, the reaper sees its parent change and kills the tool's
//! group and every descendant; a guardian removes the temporary directory.
//! Both are told the CLI's pid (`--cli-pid`) rather than sampling their parent,
//! so a CLI that dies before they look is seen at once (AET-WO-0300c).
//!
//! The status file the CLI reads (the sandbox line, then how the tool ended)
//! lives outside the tool's read-write directory, so the tool cannot forge it.

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::process::{CommandExt, ExitStatusExt, parent_id};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, exit};
use std::thread;
use std::time::{Duration, Instant};

use landlock::{
    ABI, Access, AccessFs, AccessNet, CompatLevel, Compatible, LandlockStatus, Ruleset,
    RulesetAttr, RulesetCreatedAttr, RulesetStatus, Scope, path_beneath_rules,
};
use std::os::fd::{AsFd, BorrowedFd};

use rustix::fs::{
    AtFlags, CWD, Dir, FileType, Mode, OFlags, fchmod, fchown, fstat, openat, statat,
};
use rustix::io::Errno;
use rustix::process::{
    Gid, Pid, Resource, Rlimit, Signal, Uid, WaitOptions, geteuid, getpid, kill_process,
    kill_process_group, set_child_subreaper, setrlimit, waitpid,
};
use rustix::thread::{
    CapabilitySet, CapabilitySets, capabilities, capability_is_in_ambient_set,
    capability_is_in_bounding_set, clear_ambient_capability_set, no_new_privs,
    remove_capability_from_bounding_set, set_capabilities, set_thread_groups, set_thread_res_gid,
    set_thread_res_uid,
};
use seccompiler::{BpfProgram, apply_filter, sock_filter};

use crate::{EXIT_TOOL_ERROR, EXIT_TOOL_UNAVAILABLE, EXIT_UNREADABLE};

/// The argument that tells the reaper and the guardian the CLI's pid.
pub const CLI_PID_ARG: &str = "--cli-pid";

/// The variable through which `mcp` tells each CLI it starts its own pid, so a
/// CLI whose parent died before it armed its parent-death signal sees that at
/// once. Internal, like the first arguments: not part of the contract.
pub const PARENT_PID_VAR: &str = "AETHELGARD_INTERNAL_PARENT_PID";

/// The first argument that turns this binary into a reaper. It is not a
/// subcommand, so it never appears in the command tree.
pub const REAPER_ARG: &str = "--aethelgard-internal-reaper";

/// The first argument that turns this binary into the trampoline.
pub const CONFINE_ARG: &str = "--aethelgard-internal-confine";

/// Captured stdout, stderr and reports are bounded; more is a `ToolError`.
pub const MAX_CAPTURE: usize = 16 * 1024 * 1024;

/// Read-only for every tool: the system's executables, libraries and config.
const SYSTEM_DIRS: [&str; 6] = ["/usr", "/lib", "/lib64", "/bin", "/sbin", "/etc"];

/// The uid and gid every tool runs under when the CLI's effective uid is 0
/// (`AET-WO-1601` §1.2): `nobody` and `nogroup`. Measured, not chosen: the
/// analog bridge's libngspice reads `getpwuid(getuid())->pw_dir` unchecked, so
/// it dies under any uid without a passwd entry (65533 and 65520 measured), and
/// the pinned image lists only 0, 1000 (`kicad`, a login user) and 65534.
pub const TOOL_UID: u32 = 65534;
/// The gid every tool runs under when the CLI runs as root, and the group of
/// the directories only it and root may enter. Not 65534: `sync` and `_apt`
/// have that as their primary group in the image (adversarial F2). 65533 is
/// no passwd entry's group and has no group entry, so no other process holds
/// it, and no tool looks the group up (tier 2 runs every tool under it).
pub const TOOL_GID: u32 = 65533;

/// `RLIMIT_FSIZE`, soft and hard, for every tool (`AET-WO-1601` §1.6, review
/// A12.1). A file a tool leaves at this size is the witness that it was cut
/// off, whatever the tool's own exit status says.
pub const MAX_TOOL_FILE: u64 = 64 * 1024 * 1024;

/// A refusal: the call stops here and prints `{"ok":false,"error":…}`.
#[derive(Debug)]
pub struct Stop {
    pub code: u8,
    pub kind: &'static str,
    pub id: Option<String>,
    pub message: String,
}

impl Stop {
    pub fn unreadable(message: String) -> Self {
        Self {
            code: EXIT_UNREADABLE,
            kind: "UnreadableInput",
            id: None,
            message,
        }
    }
    /// A tool input over a cap (`RUL-13`); `message` names the input and the cap.
    pub fn too_large(message: String) -> Self {
        Self {
            code: EXIT_UNREADABLE,
            kind: "InputTooLarge",
            id: None,
            message,
        }
    }
    /// An input read but refused by name before any tool runs (exit 3), such
    /// as `simulate`'s `MalformedFirmware`.
    pub fn refused(kind: &'static str, message: String) -> Self {
        Self {
            code: EXIT_UNREADABLE,
            kind,
            id: None,
            message,
        }
    }
    pub fn missing(tool: &str) -> Self {
        Self {
            code: EXIT_TOOL_UNAVAILABLE,
            kind: "ToolMissing",
            id: Some(tool.to_owned()),
            message: format!("`{tool}` is not on PATH"),
        }
    }
    pub fn unusable(id: &str, message: String) -> Self {
        Self {
            code: EXIT_TOOL_UNAVAILABLE,
            kind: "ToolUnusable",
            id: Some(id.to_owned()),
            message,
        }
    }
    /// A required sandbox layer that cannot be enforced: exit 4,
    /// `SandboxUnavailable`, never a run without it (`INV-06`).
    pub fn sandbox(layer: &str, message: String) -> Self {
        Self {
            code: EXIT_TOOL_UNAVAILABLE,
            kind: "SandboxUnavailable",
            id: Some(layer.to_owned()),
            message,
        }
    }
    pub fn tool_error(tool: &str, message: String) -> Self {
        Self {
            code: EXIT_TOOL_ERROR,
            kind: "ToolError",
            id: Some(tool.to_owned()),
            message,
        }
    }
}

/// A tool found on the caller's `PATH`.
pub struct Tool {
    pub name: &'static str,
    /// The path as found on `PATH`; this is what is executed.
    pub path: PathBuf,
}

impl Tool {
    /// Searches the caller's `PATH`, in order, for an executable `name`.
    pub fn find(name: &'static str) -> Result<Self, Stop> {
        let path_var = env::var_os("PATH").unwrap_or_default();
        if path_var.is_empty() {
            return Err(Stop::missing(name));
        }
        for dir in env::split_paths(&path_var) {
            // A relative entry (an empty one is the working directory) would run
            // whatever the caller happens to stand beside (review A12.2).
            if !dir.is_absolute() {
                return Err(Stop::unusable(
                    name,
                    format!("PATH entry {:?} is not an absolute path", dir),
                ));
            }
            let candidate = dir.join(name);
            let Ok(meta) = fs::metadata(&candidate) else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            if meta.permissions().mode() & 0o111 == 0 {
                return Err(Stop::unusable(
                    name,
                    format!("{} is not executable", candidate.display()),
                ));
            }
            return Ok(Self {
                name,
                path: candidate,
            });
        }
        Err(Stop::missing(name))
    }

    /// The directories Landlock must let the tool read: where it was found and,
    /// through any symlink, where it really lives.
    pub fn dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(d) = self.path.parent() {
            dirs.push(d.to_path_buf());
        }
        if let Ok(real) = fs::canonicalize(&self.path)
            && let Some(d) = real.parent()
        {
            dirs.push(d.to_path_buf());
        }
        dirs
    }
}

/// What the sandbox achieved, as the verdict reports it.
#[derive(Clone, Copy, Debug, serde::Serialize)]
pub struct SandboxReport {
    pub landlock_abi: i32,
    /// TCP bind and connect denied (Landlock ABI 4+). UDP is not covered.
    pub tcp_confined: bool,
    /// Signals and abstract Unix sockets scoped to the tool's own processes
    /// (Landlock ABI 6+).
    pub scoped: bool,
    /// `socket()` refused for every family, and `io_uring` refused, by the
    /// seccomp filter. Always true: the filter is required, so no verdict
    /// exists without it.
    pub sockets_denied: bool,
    /// True only when all three hold (`AET-WO-1601` §1.2, ruled on PR #96,
    /// `RUL-19`): the tool ran under [`TOOL_UID`] and [`TOOL_GID`], switched
    /// to and read back by the trampoline; the CLI's root is the initial user
    /// namespace's; and every path its read grant adds beyond the system set
    /// is root's and writable by no group or other. False otherwise, and
    /// `uid_boundary_unmet` names the first that failed.
    pub uid_boundary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid_boundary_unmet: Option<Unmet>,
}

/// The first condition of `uid_boundary` that did not hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Unmet {
    /// The tool ran as the caller, because the CLI did not run as root.
    ToolUid,
    /// The CLI's root is not the initial namespace's: under rootless podman
    /// the namespace's owner can reach every process in it (`AET-WO-1601` R4).
    UserNamespace,
    /// A path the tool may read, beyond the system directories, is not root's
    /// alone (`AET-WO-1601` A1: Kani's kit in the pinned image).
    Toolchain,
}

impl SandboxReport {
    /// Filesystem rights are required and everything else is requested
    /// best-effort, so what was enforced follows from the effective ABI.
    fn from_abi(landlock_abi: i32, uid_boundary: bool) -> Self {
        Self {
            landlock_abi,
            tcp_confined: landlock_abi >= 4,
            scoped: landlock_abi >= 6,
            sockets_denied: true,
            uid_boundary,
            uid_boundary_unmet: (!uid_boundary).then_some(Unmet::ToolUid),
        }
    }

    /// `uid_boundary` false for `unmet`, unless an earlier condition already
    /// failed. A verdict with several tools takes each tool's report.
    pub fn unmet(&mut self, unmet: Option<Unmet>) {
        if self.uid_boundary && unmet.is_some() {
            self.uid_boundary = false;
            self.uid_boundary_unmet = unmet;
        }
    }
}

/// The CLI's user namespace is the initial one: its uid map is the identity
/// over every uid (`0 0 4294967295`), which only the initial namespace, or one
/// the real root made, can have. Read once per process.
fn initial_user_namespace() -> bool {
    static INITIAL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *INITIAL.get_or_init(|| {
        fs::read_to_string("/proc/self/uid_map").is_ok_and(|map| identity_map(&map))
    })
}

fn identity_map(map: &str) -> bool {
    map.split_whitespace().collect::<Vec<_>>() == ["0", "0", "4294967295"]
}

/// Whether an entry may stand in a toolchain, by the rule the image check
/// shares (`AET-WO-1602`, ruled on PR #96): owned by uid 0, not writable by
/// others, and writable by its group only when that group is gid 0, which
/// grants nothing beyond root. An ancestor of a granted path may also be a
/// sticky directory, such as `/tmp`, where no one but root can rename what
/// root put.
fn entry_ok(uid: u32, gid: u32, mode: u32, ancestor: bool) -> bool {
    let writable = mode & 0o002 != 0 || (mode & 0o020 != 0 && gid != 0);
    uid == 0 && (!writable || (ancestor && mode & 0o1000 != 0))
}

/// Whether `dir`, a path a tool's read grant adds beyond [`SYSTEM_DIRS`],
/// `/proc` and `/dev`, is root's alone: every ancestor of its real path, and
/// everything beneath it, links not followed (ruled on PR #96, condition 3).
/// Checked once per path per process.
fn toolchain_clean(dir: &Path) -> bool {
    static SEEN: std::sync::Mutex<std::collections::BTreeMap<PathBuf, bool>> =
        std::sync::Mutex::new(std::collections::BTreeMap::new());
    let Ok(real) = fs::canonicalize(dir) else {
        return false;
    };
    let exempt = SYSTEM_DIRS
        .iter()
        .chain(&["/proc", "/dev"])
        .any(|s| real.starts_with(s));
    if exempt {
        return true;
    }
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(&clean) = seen.get(&real) {
        return clean;
    }
    let ok = |p: &Path, ancestor: bool| {
        fs::symlink_metadata(p).is_ok_and(|m| {
            m.file_type().is_symlink() || entry_ok(m.uid(), m.gid(), m.mode(), ancestor)
        })
    };
    let mut clean = real.ancestors().skip(1).all(|a| ok(a, true));
    let mut stack = vec![real.clone()];
    while clean && let Some(d) = stack.pop() {
        clean = ok(&d, false);
        let Ok(entries) = fs::read_dir(&d) else {
            clean = false;
            break;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                clean = false;
                break;
            };
            match entry.metadata() {
                Ok(m) if m.is_dir() => stack.push(entry.path()),
                Ok(m) if m.file_type().is_symlink() => {}
                Ok(m) => clean &= entry_ok(m.uid(), m.gid(), m.mode(), false),
                Err(_) => clean = false,
            }
        }
    }
    seen.insert(real, clean);
    clean
}

/// How a tool run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    Code(i32),
    Signal(i32),
    /// The reaper killed the tool at a bound (`run_bounded` only).
    Bounded(Bound),
}

/// Which bound a bounded run exceeded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    Wall,
    Memory,
}

/// A wall-clock bound and a resident-memory cap on a tool's process tree,
/// which the reaper enforces (`RUL-13` as amended for `simulate --firmware`,
/// ruled on PR #56, B1). Only `Invocation::run_bounded` sets them.
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub wall: Duration,
    pub rss_kib: u64,
}

pub struct Output {
    pub ended: Ended,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub sandbox: SandboxReport,
    /// Log lines a [`LogFilter`] dropped from stdout; 0 without one.
    pub dropped_log_lines: u64,
}

/// A bounded reading of a tool's stdout for a tool whose log can flood it
/// (`AET-WO-2100` F23, `AET-WO-2301` P5): every byte up to `budget` is kept
/// as written, so a run that fits is read exactly as without a filter. Past
/// the budget, a log line (one that starts with `[`, Renode's timestamp) is
/// counted and dropped, except the first after the budget that contains each
/// of `keep`; every other line is kept. What is kept is still capped at
/// [`MAX_CAPTURE`].
#[derive(Clone, Copy)]
pub struct LogFilter {
    pub budget: usize,
    pub keep: &'static [&'static str],
}

impl Output {
    /// stdout then stderr, as text, for tools whose answer is in their output.
    pub fn text(&self) -> String {
        let mut s = String::from_utf8_lossy(&self.stdout).into_owned();
        s.push('\n');
        s.push_str(&String::from_utf8_lossy(&self.stderr));
        s
    }

    /// The exit code; a tool killed by a signal gave no verdict (`ToolError`).
    pub fn code(&self, tool: &str) -> Result<i32, Stop> {
        match self.ended {
            Ended::Code(code) => Ok(code),
            Ended::Signal(sig) => Err(Stop::tool_error(
                tool,
                format!("{tool} was killed by signal {sig}"),
            )),
            Ended::Bounded(bound) => Err(Stop::tool_error(
                tool,
                format!("{tool} was killed at its {bound:?} bound"),
            )),
        }
    }

    /// The first non-empty line of stdout, trimmed: a tool's version string.
    pub fn first_line(&self) -> Option<String> {
        String::from_utf8_lossy(&self.stdout)
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .map(str::to_owned)
    }
}

/// One tool invocation.
pub struct Invocation<'a> {
    pub tool: &'a Tool,
    pub args: Vec<OsString>,
    /// Variables beyond `HOME` and `TMPDIR`, which always point at `workdir`.
    pub env: Vec<(&'static str, OsString)>,
    /// `HOME` and `TMPDIR`, and read-write for the tool.
    pub workdir: &'a Path,
    /// The tool's working directory, inside `workdir`.
    pub cwd: PathBuf,
    /// Where the reaper writes its status; must be outside `workdir`.
    pub status: PathBuf,
    pub read_only: Vec<PathBuf>,
    pub read_write: Vec<PathBuf>,
}

impl Invocation<'_> {
    /// Runs the tool through the reaper and waits for it.
    pub fn run(&self) -> Result<Output, Stop> {
        self.start(None, None)
    }

    /// As `run`, with the reaper killing the tool's tree past `bounds`, which
    /// it then reports as `Ended::Bounded`.
    pub fn run_bounded(&self, bounds: Bounds) -> Result<Output, Stop> {
        self.start(Some(bounds), None)
    }

    /// As [`run_bounded`](Self::run_bounded), reading stdout through `filter`.
    pub fn run_bounded_filtered(&self, bounds: Bounds, filter: LogFilter) -> Result<Output, Stop> {
        self.start(Some(bounds), Some(filter))
    }

    fn start(&self, bounds: Option<Bounds>, filter: Option<LogFilter>) -> Result<Output, Stop> {
        let me = env::current_exe().map_err(|e| {
            Stop::tool_error(self.tool.name, format!("cannot find this executable: {e}"))
        })?;
        if geteuid().is_root() {
            hand_over(self.workdir).map_err(|e| {
                Stop::sandbox(
                    "uid",
                    format!(
                        "cannot hand {} to the tool uid: {e}",
                        self.workdir.display()
                    ),
                )
            })?;
        }
        let mut cmd = Command::new(me);
        cmd.arg(REAPER_ARG)
            .arg(CLI_PID_ARG)
            .arg(std::process::id().to_string())
            .arg("--status")
            .arg(&self.status);
        if let Some(b) = bounds {
            cmd.arg("--wall-ms")
                .arg(b.wall.as_millis().to_string())
                .arg("--rss-kib")
                .arg(b.rss_kib.to_string());
        }
        let system = SYSTEM_DIRS.iter().map(PathBuf::from);
        for p in system.chain(self.read_only.iter().cloned()) {
            cmd.arg("--ro").arg(p);
        }
        for p in std::iter::once(self.workdir.to_path_buf()).chain(self.read_write.iter().cloned())
        {
            cmd.arg("--rw").arg(p);
        }
        cmd.arg("--").arg(&self.tool.path).args(&self.args);
        cmd.env_clear()
            .env("HOME", self.workdir)
            .env("TMPDIR", self.workdir)
            .envs(self.env.iter().map(|(k, v)| (k, v)))
            .current_dir(&self.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        let mut child = cmd.spawn().map_err(|e| {
            Stop::tool_error(self.tool.name, format!("cannot start the reaper: {e}"))
        })?;
        let stdout = capture(child.stdout.take().expect("piped stdout"), filter);
        let stderr = capture(child.stderr.take().expect("piped stderr"), None);
        let reaper = child.wait();
        let stdout = stdout.join().expect("stdout reader");
        let stderr = stderr.join().expect("stderr reader");
        let reaper = reaper.map_err(|e| {
            Stop::tool_error(self.tool.name, format!("waiting for the reaper: {e}"))
        })?;
        let (Some((stdout, dropped_log_lines)), Some((stderr, _))) = (stdout, stderr) else {
            return Err(Stop::tool_error(
                self.tool.name,
                format!("output exceeded {MAX_CAPTURE} bytes"),
            ));
        };
        let status = fs::read_to_string(&self.status).unwrap_or_default();
        let (sandbox, ended) =
            parse_status(&status, self.tool.name).map_err(|stop| match reaper.signal() {
                Some(sig) if stop.code == EXIT_TOOL_ERROR => Stop::tool_error(
                    self.tool.name,
                    format!("{} (the reaper died by signal {sig})", stop.message),
                ),
                _ => stop,
            })?;
        // A tool cut off at the cap may have exited on its own terms (a child
        // killed by SIGXFSZ under cargo, or EFBIG handled): its output then
        // cannot be read as pass or fail (AET-WO-1601 §1.6).
        if let Some(file) = at_the_cap(self.workdir) {
            return Err(Stop::tool_error(
                self.tool.name,
                format!(
                    "{} reached the {MAX_TOOL_FILE}-byte file-size cap (RLIMIT_FSIZE)",
                    file.display()
                ),
            ));
        }
        let mut sandbox = sandbox;
        if sandbox.uid_boundary {
            if !initial_user_namespace() {
                sandbox.unmet(Some(Unmet::UserNamespace));
            } else if !self.read_only.iter().all(|p| toolchain_clean(p)) {
                sandbox.unmet(Some(Unmet::Toolchain));
            }
        }
        Ok(Output {
            ended,
            stdout,
            stderr,
            sandbox,
            dropped_log_lines,
        })
    }
}

/// Reads a pipe to its end, keeping at most `MAX_CAPTURE` bytes. Past that it
/// keeps draining, so the tool never blocks on a full pipe, and yields `None`.
/// A tool that prints forever is ended by the caller's timeout (`RUL-13`).
/// With a filter, what is kept is [`Filtered`]'s, with the dropped count.
fn capture(
    mut pipe: impl Read + Send + 'static,
    filter: Option<LogFilter>,
) -> thread::JoinHandle<Option<(Vec<u8>, u64)>> {
    thread::spawn(move || {
        let mut kept = Filtered::new(filter);
        let mut chunk = [0u8; 64 * 1024];
        loop {
            match pipe.read(&mut chunk) {
                Ok(0) => return kept.finish(),
                Ok(n) => kept.push(&chunk[..n]),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => return kept.finish(),
            }
        }
    })
}

/// The bytes a capture keeps: all of them without a filter; with one, every
/// whole line until the budget, then [`LogFilter`]'s rule line by line.
struct Filtered {
    filter: Option<LogFilter>,
    buf: Vec<u8>,
    /// A line not yet ended by a newline, once past the budget.
    partial: Vec<u8>,
    kept_once: Vec<bool>,
    dropped: u64,
    overflow: bool,
}

impl Filtered {
    fn new(filter: Option<LogFilter>) -> Self {
        let keep = filter.map_or(0, |f| f.keep.len());
        Self {
            filter,
            buf: Vec::new(),
            partial: Vec::new(),
            kept_once: vec![false; keep],
            dropped: 0,
            overflow: false,
        }
    }

    fn keep(&mut self, bytes: &[u8]) {
        if self.overflow || self.buf.len() + bytes.len() > MAX_CAPTURE {
            self.overflow = true;
            self.buf = Vec::new();
        } else {
            self.buf.extend_from_slice(bytes);
        }
    }

    fn push(&mut self, mut bytes: &[u8]) {
        let Some(filter) = self.filter else {
            return self.keep(bytes);
        };
        while !bytes.is_empty() {
            if self.partial.is_empty() && self.buf.len() < filter.budget {
                // Under the budget, whole chunks up to the next line end.
                let take = match bytes.iter().position(|&b| b == b'\n') {
                    Some(i) => i + 1,
                    None => bytes.len(),
                };
                let (now, rest) = bytes.split_at(take);
                self.keep(now);
                bytes = rest;
                continue;
            }
            match bytes.iter().position(|&b| b == b'\n') {
                Some(i) => {
                    self.partial.extend_from_slice(&bytes[..=i]);
                    bytes = &bytes[i + 1..];
                    self.line();
                }
                None => {
                    // A line longer than the cap is an overflow, not a hang.
                    if self.partial.len() + bytes.len() > MAX_CAPTURE {
                        self.overflow = true;
                        self.partial.clear();
                    } else {
                        self.partial.extend_from_slice(bytes);
                    }
                    bytes = &[];
                }
            }
        }
    }

    /// One whole line past the budget.
    fn line(&mut self) {
        let line = std::mem::take(&mut self.partial);
        let Some(filter) = self.filter else {
            return self.keep(&line);
        };
        if !line.starts_with(b"[") {
            return self.keep(&line);
        }
        let text = String::from_utf8_lossy(&line);
        if let Some(i) = filter
            .keep
            .iter()
            .enumerate()
            .position(|(i, k)| !self.kept_once[i] && text.contains(k))
        {
            self.kept_once[i] = true;
            return self.keep(&line);
        }
        self.dropped += 1;
    }

    fn finish(mut self) -> Option<(Vec<u8>, u64)> {
        if !self.partial.is_empty() {
            self.line();
        }
        (!self.overflow).then_some((self.buf, self.dropped))
    }
}

/// The mode a root CLI's `workdir` has once the tool uid owns it: root's,
/// writable by [`TOOL_GID`] alone, and sticky, so the tool uid can create
/// entries but cannot rename or remove one root made (`AET-WO-1601` §1.2).
const HANDED_OVER: u32 = 0o1770;

/// As root, hands `workdir` to the tool uid the first time a tool runs in it
/// (`AET-WO-1601` §1.2). Until then it is root's alone (0700), and everything
/// in it is the CLI's: the hashed copies and the files the CLI wrote beside
/// them. Those are sealed (files 0444, directories 0555), all by descriptor
/// and never following a link; then the directory becomes root's, group
/// [`TOOL_GID`], mode [`HANDED_OVER`]. A later call on the same directory
/// finds it handed over and touches nothing in it: the tool uid has had it
/// since, so root acts there by no path.
fn hand_over(workdir: &Path) -> Result<(), String> {
    let dir = openat(
        CWD,
        workdir,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|e| errno("open", e))?;
    let st = fstat(&dir).map_err(|e| errno("stat", e))?;
    let perm = st.st_mode & 0o7777;
    match (st.st_uid, st.st_gid, perm) {
        (0, TOOL_GID, HANDED_OVER) => Ok(()),
        (0, 0, 0o700) => {
            seal(dir.as_fd(), 0)?;
            fchown(&dir, None, Some(Gid::from_raw(TOOL_GID))).map_err(|e| errno("chown", e))?;
            fchmod(&dir, Mode::from_raw_mode(HANDED_OVER)).map_err(|e| errno("chmod", e))
        }
        (uid, gid, mode) => Err(format!(
            "it is {uid}:{gid} mode {mode:o}, neither the CLI's own nor handed over"
        )),
    }
}

/// As root, makes `dir`, which the CLI made and no tool has had, a directory
/// the tool uid may add to but not change: root's, group [`TOOL_GID`], mode
/// [`HANDED_OVER`]. [`seal`] keeps that mode. `prove` marks the crate's root
/// so, where cargo writes the `Cargo.lock` a crate without one needs
/// (adversarial F4a: sealed 0555, every such `prove` became exit 5).
pub fn add_only(dir: &Path) -> io::Result<()> {
    let fd = openat(
        CWD,
        dir,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    fchown(&fd, None, Some(Gid::from_raw(TOOL_GID)))?;
    fchmod(&fd, Mode::from_raw_mode(HANDED_OVER))?;
    Ok(())
}

/// Seals everything below `dir` read-only, by descriptor. Every entry must be
/// root's, and a file or a directory: nothing else has had the directory yet,
/// and the CLI copies no link. A directory [`add_only`] marked keeps its mode.
fn seal(dir: BorrowedFd<'_>, depth: usize) -> Result<(), String> {
    if depth > 64 {
        return Err("the copy is more than 64 directories deep".to_owned());
    }
    let mut names = Vec::new();
    for entry in Dir::read_from(dir).map_err(|e| errno("read a directory", e))? {
        let entry = entry.map_err(|e| errno("read a directory", e))?;
        let name = entry.file_name().to_owned();
        if name.as_bytes() != b"." && name.as_bytes() != b".." {
            names.push(name);
        }
    }
    for name in names {
        let shown = String::from_utf8_lossy(name.as_bytes()).into_owned();
        let st = statat(dir, name.as_c_str(), AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|e| errno(&format!("stat {shown}"), e))?;
        if st.st_uid != 0 {
            return Err(format!("{shown} is not root's before any tool ran"));
        }
        let (flags, mode) = match FileType::from_raw_mode(st.st_mode) {
            FileType::Directory => (OFlags::DIRECTORY, 0o555),
            FileType::RegularFile => (OFlags::NONBLOCK | OFlags::NOCTTY, 0o444),
            _ => return Err(format!("{shown} is not a file or a directory")),
        };
        let fd = openat(
            dir,
            name.as_c_str(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | flags,
            Mode::empty(),
        )
        .map_err(|e| errno(&format!("open {shown}"), e))?;
        let opened = fstat(&fd).map_err(|e| errno(&format!("stat {shown}"), e))?;
        if mode == 0o555 {
            seal(fd.as_fd(), depth + 1)?;
            if (opened.st_gid, opened.st_mode & 0o7777) == (TOOL_GID, HANDED_OVER) {
                continue;
            }
        }
        fchmod(&fd, Mode::from_raw_mode(mode)).map_err(|e| errno(&format!("chmod {shown}"), e))?;
    }
    Ok(())
}

/// A regular file under `dir` at [`MAX_TOOL_FILE`] bytes or more: what a write
/// cut off by `RLIMIT_FSIZE` leaves. Links are not followed.
fn at_the_cap(dir: &Path) -> Option<PathBuf> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).into_iter().flatten().flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                stack.push(entry.path());
            } else if meta.is_file() && meta.len() >= MAX_TOOL_FILE {
                return Some(entry.path());
            }
        }
    }
    None
}

/// The status file holds one line about the sandbox, then one about how the
/// tool ended: `sandbox abi=8 uid_boundary=1` then `ended code=0` or `ended signal=9`;
/// or a single `unavailable …` or `spawn-error …` line.
fn parse_status(status: &str, tool: &str) -> Result<(SandboxReport, Ended), Stop> {
    let mut lines = status.lines();
    let first = lines.next().unwrap_or_default();
    if let Some(rest) = first.strip_prefix("unavailable ") {
        // Only the trampoline writes this line: `descriptors`, `uid`,
        // `landlock`, `seccomp`, `capabilities` or `fsize`.
        let (layer, reason) = rest.split_once(' ').unwrap_or((rest, ""));
        return Err(Stop::sandbox(
            layer,
            format!("the required {layer} sandbox layer cannot be enforced: {reason}"),
        ));
    }
    let sandbox = first
        .strip_prefix("sandbox abi=")
        .and_then(|rest| rest.split_once(" uid_boundary="))
        .and_then(|(abi, uid)| {
            let uid_boundary = match uid {
                "1" => true,
                "0" => false,
                _ => return None,
            };
            Some(SandboxReport::from_abi(abi.parse().ok()?, uid_boundary))
        })
        .ok_or_else(|| Stop::tool_error(tool, "the reaper reported no sandbox".to_owned()))?;
    let second = lines.next().unwrap_or_default();
    if let Some(reason) = second.strip_prefix("spawn-error ") {
        return Err(Stop::unusable(
            tool,
            format!("cannot start {tool}: {reason}"),
        ));
    }
    let ended = second
        .strip_prefix("ended ")
        .and_then(|rest| match rest.split_once('=') {
            Some(("code", v)) => v.parse().ok().map(Ended::Code),
            Some(("signal", v)) => v.parse().ok().map(Ended::Signal),
            Some(("bound", "wall")) => Some(Ended::Bounded(Bound::Wall)),
            Some(("bound", "memory")) => Some(Ended::Bounded(Bound::Memory)),
            _ => None,
        })
        .ok_or_else(|| {
            Stop::tool_error(
                tool,
                "the reaper did not report how the tool ended".to_owned(),
            )
        })?;
    Ok((sandbox, ended))
}

/// The internal arguments shared by the reaper and the trampoline:
/// `--cli-pid N --status F [--ro P]… [--rw P]… -- TOOL ARGS…`.
struct Internal {
    cli_pid: u32,
    status: PathBuf,
    bounds: Option<Bounds>,
    read_only: Vec<PathBuf>,
    read_write: Vec<PathBuf>,
    tool: Vec<OsString>,
}

/// A pid exactly as this binary writes one: decimal digits with no sign and no
/// leading zero, at most `i32::MAX`. Anything else is `None`, a refusal.
pub(crate) fn pid_arg(arg: &OsStr) -> Option<u32> {
    let s = arg.to_str()?;
    if s.starts_with('0') || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse::<i32>().ok().map(i32::unsigned_abs)
}

fn parse_internal(args: &[OsString]) -> Internal {
    let (mut cli_pid, mut status) = (None, None);
    let (mut wall_ms, mut rss_kib) = (None, None);
    let number = |a: Option<&OsString>| -> u64 {
        a.and_then(|a| a.to_str())
            .filter(|s| !s.starts_with('0') && s.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| exit(64))
    };
    let (mut read_only, mut read_write, mut tool) = (Vec::new(), Vec::new(), Vec::new());
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.to_str() {
            // Once only: a second pid could only contradict the first.
            Some(CLI_PID_ARG) if cli_pid.is_none() => {
                cli_pid = Some(
                    it.next()
                        .and_then(|a| pid_arg(a))
                        .unwrap_or_else(|| exit(64)),
                );
            }
            Some("--status") => status = it.next().map(PathBuf::from),
            Some("--wall-ms") if wall_ms.is_none() => wall_ms = Some(number(it.next())),
            Some("--rss-kib") if rss_kib.is_none() => rss_kib = Some(number(it.next())),
            Some("--ro") => read_only.extend(it.next().map(PathBuf::from)),
            Some("--rw") => read_write.extend(it.next().map(PathBuf::from)),
            Some("--") => {
                tool = it.cloned().collect();
                break;
            }
            _ => exit(64),
        }
    }
    let bounds = match (wall_ms, rss_kib) {
        (Some(ms), Some(kib)) => Some(Bounds {
            wall: Duration::from_millis(ms),
            rss_kib: kib,
        }),
        (None, None) => None,
        _ => exit(64),
    };
    match (cli_pid, status) {
        (Some(cli_pid), Some(status)) if !tool.is_empty() => Internal {
            cli_pid,
            status,
            bounds,
            read_only,
            read_write,
            tool,
        },
        _ => exit(64),
    }
}

/// The reaper: `aethelgard --aethelgard-internal-reaper <internal args>`.
/// Unconfined, so it can clean up; never returns. It never sets a death signal
/// on itself: it has to outlive the CLI to clean up after it.
pub fn reaper_main(args: &[OsString]) -> ! {
    let internal = parse_internal(args);
    if parent_id() != internal.cli_pid {
        // The CLI died before this process looked: start nothing.
        exit(70);
    }
    // Orphans the tool leaves, a double-forked daemon in its own session
    // included, are reparented here, so none outlives the call.
    if set_child_subreaper(Some(getpid())).is_err() {
        exit(70);
    }
    let Ok(me) = env::current_exe() else { exit(70) };
    let Ok(mut child) = Command::new(me)
        .arg(CONFINE_ARG)
        .args(args)
        .process_group(0)
        .spawn()
    else {
        exit(70);
    };
    let tool_group = Pid::from_child(&child);
    let started = Instant::now();
    let mut tick: u32 = 0;
    loop {
        match child.try_wait() {
            Ok(Some(ended)) => {
                // Whatever the tool left running dies with it.
                let _ = kill_process_group(tool_group, Signal::KILL);
                kill_descendants();
                let line = match (ended.code(), ended.signal()) {
                    (Some(code), _) => format!("ended code={code}"),
                    (None, Some(sig)) => format!("ended signal={sig}"),
                    (None, None) => "ended unknown".to_owned(),
                };
                let appended = fs::OpenOptions::new()
                    .append(true)
                    .open(&internal.status)
                    .and_then(|mut f| writeln!(f, "{line}"));
                exit(if appended.is_ok() { 0 } else { 70 });
            }
            Ok(None) => {}
            Err(_) => exit(70),
        }
        if parent_id() != internal.cli_pid {
            // The CLI is gone: take everything the tool started. The guardian
            // removes the temporary directory.
            let _ = kill_process_group(tool_group, Signal::KILL);
            let _ = child.wait();
            kill_descendants();
            exit(70);
        }
        if let Some(b) = internal.bounds {
            // The memory is read every fifth tick (100 ms): a full /proc scan.
            let over = if started.elapsed() > b.wall {
                Some("wall")
            } else if tick.is_multiple_of(5) && tree_rss_kib() > b.rss_kib {
                Some("memory")
            } else {
                None
            };
            if let Some(which) = over {
                let _ = kill_process_group(tool_group, Signal::KILL);
                let _ = child.wait();
                kill_descendants();
                let appended = fs::OpenOptions::new()
                    .append(true)
                    .open(&internal.status)
                    .and_then(|mut f| writeln!(f, "ended bound={which}"));
                exit(if appended.is_ok() { 0 } else { 70 });
            }
            tick = tick.wrapping_add(1);
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// The resident memory, in KiB, of every process descended from this one: the
/// tool's whole tree, since this process is its subreaper.
fn tree_rss_kib() -> u64 {
    let mut parent = std::collections::BTreeMap::new();
    for entry in fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(stat) = fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        // `pid (comm) state ppid …`; comm may hold spaces or parens.
        if let Some(ppid) = stat
            .rsplit_once(')')
            .and_then(|(_, rest)| rest.split_whitespace().nth(1))
            .and_then(|p| p.parse::<u32>().ok())
        {
            parent.insert(pid, ppid);
        }
    }
    let me = std::process::id();
    let descends = |mut pid: u32| {
        for _ in 0..64 {
            match parent.get(&pid) {
                Some(&p) if p == me => return true,
                Some(&p) if p > 1 => pid = p,
                _ => return false,
            }
        }
        false
    };
    parent
        .keys()
        .filter(|&&pid| descends(pid))
        .filter_map(|pid| fs::read_to_string(format!("/proc/{pid}/status")).ok())
        .filter_map(|status| {
            status
                .lines()
                .find_map(|l| l.strip_prefix("VmRSS:"))
                .and_then(|v| v.split_whitespace().next()?.parse::<u64>().ok())
        })
        .sum()
}

/// Kills every process whose parent is this one, until none is left. As a
/// child subreaper, this process inherits every orphan the tool leaves.
fn kill_descendants() {
    let me = std::process::id().to_string();
    for _ in 0..100 {
        let children: Vec<Pid> = fs::read_dir("/proc")
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let stat = fs::read_to_string(e.path().join("stat")).ok()?;
                // `pid (comm) state ppid …`; comm may hold spaces or parens.
                let ppid = stat.rsplit_once(')')?.1.split_whitespace().nth(1)?;
                (ppid == me)
                    .then(|| e.file_name().to_str()?.parse().ok().and_then(Pid::from_raw))
                    .flatten()
            })
            .collect();
        if children.is_empty() {
            return;
        }
        for pid in children {
            let _ = kill_process(pid, Signal::KILL);
        }
        while let Ok(Some(_)) = waitpid(None, WaitOptions::NOHANG) {}
        thread::sleep(Duration::from_millis(10));
    }
}

/// How long the guardian keeps trying to remove its directory once the CLI is gone.
const GUARD_REMOVAL_BOUND: Duration = Duration::from_secs(10);

/// Removes `dir` with `remove`, retrying every 20 ms while a writer can still be racing it (the
/// trampoline's status file, a tool stub the reaper has not killed yet), so the last `rmdir` may
/// lose. Returns the guardian's exit code: 0 once the directory is gone (`remove` succeeded, or
/// failed `NotFound` with the directory absent), 70 once `bound` has passed.
fn remove_with_retry(
    dir: &Path,
    bound: Duration,
    mut remove: impl FnMut(&Path) -> io::Result<()>,
) -> i32 {
    let give_up = Instant::now() + bound;
    loop {
        match remove(dir) {
            Ok(()) => return 0,
            Err(e) if e.kind() == io::ErrorKind::NotFound && !dir.exists() => return 0,
            Err(_) => {}
        }
        if Instant::now() >= give_up {
            return 70;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// The guardian: `aethelgard --aethelgard-internal-guard --cli-pid N <dir>`.
/// Started, in its own process group, before the call's temporary directory
/// exists, it removes the directory if the CLI dies before removing it itself,
/// whatever the CLI was doing then, retrying while a writer races it, and exits 70
/// (directory left) after the bound. It never sets a death signal on itself.
/// Never returns.
pub fn guard_main(args: &[OsString]) -> ! {
    let (cli_pid, dir) = match args {
        [flag, pid, dir] if flag == CLI_PID_ARG => (pid_arg(pid), PathBuf::from(dir)),
        _ => exit(64),
    };
    let Some(cli_pid) = cli_pid else { exit(64) };
    // The path must end in its own `aethelgard-*` name: `file_name` reads past a
    // trailing `/` or `/.`, and with either `remove_dir_all` follows a symlink.
    let raw = dir.as_os_str().as_encoded_bytes();
    if raw.ends_with(b"/")
        || raw.ends_with(b"/.")
        || !dir
            .file_name()
            .and_then(OsStr::to_str)
            .is_some_and(|n| n.starts_with("aethelgard-"))
    {
        exit(64);
    }
    let mut seen = false;
    loop {
        // The parent first: once it is gone, nothing can create the directory.
        if parent_id() != cli_pid {
            exit(remove_with_retry(&dir, GUARD_REMOVAL_BOUND, |d| {
                fs::remove_dir_all(d)
            }));
        }
        if dir.exists() {
            seen = true;
        } else if seen {
            // The CLI removed it.
            exit(0);
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// The first argument that turns this binary into the guardian.
pub const GUARD_ARG: &str = "--aethelgard-internal-guard";

/// The trampoline: `aethelgard --aethelgard-internal-confine <internal args>`.
/// Confines itself with Landlock, records the sandbox, and becomes the tool.
pub fn confine_main(args: &[OsString]) -> ! {
    let internal = parse_internal(args);
    // Opened before Landlock applies; close-on-exec, so the tool never has it.
    let Ok(mut status) = File::create(&internal.status) else {
        exit(70)
    };
    // Checked before Landlock, which denies /proc to most tools. Nothing
    // opened from here to exec is inheritable: Rust opens close-on-exec.
    match inherited_descriptors() {
        Ok(leaked) if leaked.is_empty() => {}
        Ok(leaked) => {
            let _ = writeln!(
                status,
                "unavailable descriptors inherited without close-on-exec: {leaked:?}"
            );
            exit(0);
        }
        Err(reason) => {
            let _ = writeln!(status, "unavailable descriptors {reason}");
            exit(0);
        }
    }
    // As root, the tool runs under its own uid (AET-WO-1601 §1.2), set here,
    // before Landlock and the filter, while this process is single-threaded.
    // There is no fallback to root: a switch that fails refuses the call.
    let separate = geteuid().is_root();
    if separate && let Err(reason) = separate_uid() {
        let _ = writeln!(status, "unavailable uid {reason}");
        exit(0);
    }
    let abi = match confine(&internal.read_only, &internal.read_write) {
        Ok(s) => s,
        Err(reason) => {
            let _ = writeln!(status, "unavailable landlock {reason}");
            exit(0);
        }
    };
    // Required like Landlock (INV-06): no filter, no tool.
    let installed = seccomp_filter(std::process::id())
        .ok_or_else(|| "no filter for this architecture".to_owned())
        .and_then(|program| apply_filter(&program).map_err(|e| e.to_string()));
    if let Err(e) = installed {
        let _ = writeln!(status, "unavailable seccomp {e}");
        exit(0);
    }
    // Required too (INV-06): a tool never starts holding a capability.
    if let Err(e) = drop_capabilities() {
        let _ = writeln!(status, "unavailable capabilities {e}");
        exit(0);
    }
    // Soft and hard alike, so the tool cannot raise it (AET-WO-1601 §1.6).
    let cap = Rlimit {
        current: Some(MAX_TOOL_FILE),
        maximum: Some(MAX_TOOL_FILE),
    };
    if let Err(e) = setrlimit(Resource::Fsize, cap) {
        let _ = writeln!(
            status,
            "unavailable fsize setrlimit: errno {}",
            e.raw_os_error()
        );
        exit(0);
    }
    let _ = writeln!(
        status,
        "sandbox abi={abi} uid_boundary={}",
        u8::from(separate)
    );
    let (program, tool_args) = internal
        .tool
        .split_first()
        .expect("parse_internal checked the tool");
    let err = Command::new(program).args(tool_args).exec();
    let _ = writeln!(status, "spawn-error {err}");
    exit(127);
}

/// Every descriptor above 2 this process holds without close-on-exec: each
/// would reach the tool, a channel no sandbox layer sees (ruled on PR #15,
/// B2). Read from /proc/self/fdinfo; any doubt is an error, never a pass.
pub(crate) fn inherited_descriptors() -> Result<Vec<u32>, String> {
    const O_CLOEXEC: u32 = 0o2_000_000;
    let unreadable = |what: &str, e: io::Error| format!("cannot read {what}: {e}");
    let mut leaked = Vec::new();
    for entry in fs::read_dir("/proc/self/fd").map_err(|e| unreadable("/proc/self/fd", e))? {
        let entry = entry.map_err(|e| unreadable("/proc/self/fd", e))?;
        let Some(fd) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            return Err(format!(
                "unexpected entry {:?} in /proc/self/fd",
                entry.file_name()
            ));
        };
        if fd <= 2 {
            continue;
        }
        let info = match fs::read_to_string(format!("/proc/self/fdinfo/{fd}")) {
            Ok(info) => info,
            // The directory listing's own descriptor, closed once read.
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(unreadable(&format!("/proc/self/fdinfo/{fd}"), e)),
        };
        let flags = info
            .lines()
            .find_map(|l| l.strip_prefix("flags:"))
            .and_then(|v| u32::from_str_radix(v.trim(), 8).ok())
            .ok_or_else(|| format!("no flags for descriptor {fd}"))?;
        if flags & O_CLOEXEC == 0 {
            leaked.push(fd);
        }
    }
    leaked.sort_unstable();
    Ok(leaked)
}

/// The system calls the filter refuses with `EPERM`: `socket` and `io_uring`
/// (AET-WO-0300a), then (AET-WO-1600, ruled on PR #51) every non-socket channel to
/// another process of the same uid that the spike found open on a host, and
/// the kernel surfaces beside them. No tool needs any of them (the spike ran
/// the whole tier-2 suite with each refused); Kani's `cargo` calls
/// `pidfd_open` and carries on without it. The POSIX message queues were added
/// by the adversarial pass under the ruling's F10 clause: Landlock refuses to
/// open one, but `O_CREAT` still creates it in the host's namespace, and
/// `mq_unlink` deletes another process's.
const DENIED: [(&str, u32); 37] = [
    ("socket", 41),
    ("io_uring_setup", 425),
    ("io_uring_enter", 426),
    ("io_uring_register", 427),
    // System V IPC: on a host, the host's IPC namespace.
    ("shmget", 29),
    ("shmat", 30),
    ("shmctl", 31),
    ("shmdt", 67),
    ("semget", 64),
    ("semop", 65),
    ("semctl", 66),
    ("msgget", 68),
    ("msgsnd", 69),
    ("msgrcv", 70),
    ("msgctl", 71),
    ("semtimedop", 220),
    // Namespaces; `clone` is refused only with a namespace bit, below.
    ("unshare", 272),
    ("setns", 308),
    // pidfds and cross-process access.
    ("pidfd_open", 434),
    ("pidfd_getfd", 438),
    ("pidfd_send_signal", 424),
    ("ptrace", 101),
    ("process_vm_readv", 310),
    ("process_vm_writev", 311),
    ("kcmp", 312),
    // Other kernel surfaces.
    ("keyctl", 250),
    ("add_key", 248),
    ("request_key", 249),
    ("bpf", 321),
    ("perf_event_open", 298),
    ("userfaultfd", 323),
    // POSIX message queues (F10).
    ("mq_open", 240),
    ("mq_unlink", 241),
    ("mq_timedsend", 242),
    ("mq_timedreceive", 243),
    ("mq_notify", 244),
    ("mq_getsetattr", 245),
];

/// Calls that reach another process by pid and that the tools make only on
/// themselves, with pid 0 (the spike's census: `prlimit64` 1925 of 1925 calls,
/// `sched_setaffinity` 54 of 54). Allowed for pid 0 and for the tool's own pid,
/// baked in when the trampoline installs the filter; refused for any other.
/// `prlimit64` on another pid can kill it (`RLIMIT_CPU`) past Landlock's
/// signal scoping, which the adversarial pass measured (F10).
const PID_TESTED: [(&str, u32); 2] = [("prlimit64", 302), ("sched_setaffinity", 203)];

/// Every `CLONE_NEW*` bit `clone(2)`'s 32-bit flags word can carry: NEWNS,
/// NEWCGROUP, NEWUTS, NEWIPC, NEWUSER, NEWPID and NEWNET. `CLONE_NEWTIME`
/// (0x80) lies inside `CSIGNAL`, which the kernel strips from `clone`'s
/// flags, so only `unshare` and `clone3` could carry it, and both are refused.
const CLONE_NEW_MASK: u32 = 0x7e02_0000;

/// The seccomp filter (AET-WO-0300a, ruled on PR #15; grown by AET-WO-1600,
/// ruled on PR #51): a fixed, default-allow program, assembled here rather than
/// by `seccompiler::SeccompFilter`, whose compiled rules match system-call
/// numbers under `AUDIT_ARCH_X86_64` and so would let an x32 call
/// (number | 0x4000_0000) through a default-allow filter.
///
/// - Any other architecture, and any x32 call, kills the process.
/// - `socket` is refused for every family because no tool needs one (measured
///   in the AET-WO-0300a spike). `socketpair` is allowed only as an `AF_UNIX`
///   stream or seqpacket pair: such a pair is born connected to itself and
///   cannot be redirected. A datagram pair (`SOCK_DGRAM`, or `SOCK_RAW`, which
///   `AF_UNIX` treats as datagram) can `connect` or `sendto` any pathname
///   datagram socket outside, which the adversarial pass showed by writing to
///   the systemd journal, so it is refused like `socket`.
/// - `io_uring` and every call in [`DENIED`] are refused with `EPERM`, and so
///   is `clone` with any bit of [`CLONE_NEW_MASK`].
/// - [`PID_TESTED`]'s `self_pid` is the trampoline's, which the exec keeps;
///   `setpriority`
///   and `ioprio_set` are refused only in their every-process-of-the-user
///   forms (`PRIO_USER`, `IOPRIO_WHO_USER`). Their per-pid forms stay open:
///   the tools call them on their own threads' ids, which no filter can name.
/// - `clone3` returns `ENOSYS`, never `EPERM`. Its flags sit in user memory,
///   where seccomp cannot read them; `ENOSYS` makes glibc fall back to `clone`,
///   whose flags the filter tests, and is what Docker's default profile already
///   returns. `EPERM` would stop every tool from starting a thread or a child.
#[cfg(target_arch = "x86_64")]
pub fn seccomp_filter(self_pid: u32) -> Option<BpfProgram> {
    const LD_W_ABS: u16 = 0x20; // BPF_LD | BPF_W | BPF_ABS
    const AND_K: u16 = 0x54; // BPF_ALU | BPF_AND | BPF_K
    const JEQ_K: u16 = 0x15; // BPF_JMP | BPF_JEQ | BPF_K
    const JSET_K: u16 = 0x45; // BPF_JMP | BPF_JSET | BPF_K
    const RET_K: u16 = 0x06; // BPF_RET | BPF_K
    const AUDIT_ARCH_X86_64: u32 = 0xc000_003e;
    const X32_SYSCALL_BIT: u32 = 0x4000_0000;
    const OFFSET_NR: u32 = 0;
    const OFFSET_ARCH: u32 = 4;
    const OFFSET_ARG0: u32 = 16; // low 32 bits, little-endian
    const OFFSET_ARG1: u32 = 24;
    const NR_SOCKETPAIR: u32 = 53;
    const NR_CLONE: u32 = 56;
    const NR_SETPRIORITY: u32 = 141;
    const NR_IOPRIO_SET: u32 = 251;
    const NR_CLONE3: u32 = 435;
    const AF_UNIX: u32 = 1;
    const SOCK_TYPE_MASK: u32 = 0xf;
    const SOCK_STREAM: u32 = 1;
    const SOCK_SEQPACKET: u32 = 5;
    const PRIO_USER: u32 = 2; // linux/resource.h
    const IOPRIO_WHO_USER: u32 = 3; // linux/ioprio.h
    const RET_ALLOW: u32 = 0x7fff_0000;
    const RET_ERRNO: u32 = 0x0005_0000;
    const EPERM: u32 = 1;
    const ENOSYS: u32 = 38;
    const RET_KILL_PROCESS: u32 = 0x8000_0000;
    use Label::{Allow, Clone, Enosys, Eperm, Ioprio, Kill, Next, Pid, Priority, Socketpair};
    // (label of this instruction, code, k, jump if true, jump if false)
    let mut prog: Vec<(Label, u16, u32, Label, Label)> = vec![
        (Next, LD_W_ABS, OFFSET_ARCH, Next, Next),
        (Next, JEQ_K, AUDIT_ARCH_X86_64, Next, Kill),
        (Next, LD_W_ABS, OFFSET_NR, Next, Next),
        (Next, JSET_K, X32_SYSCALL_BIT, Kill, Next),
    ];
    for &(_, nr) in &DENIED {
        prog.push((Next, JEQ_K, nr, Eperm, Next));
    }
    prog.push((Next, JEQ_K, NR_CLONE3, Enosys, Next));
    for &(_, nr) in &PID_TESTED {
        prog.push((Next, JEQ_K, nr, Pid, Next));
    }
    prog.extend([
        (Next, JEQ_K, NR_SETPRIORITY, Priority, Next),
        (Next, JEQ_K, NR_IOPRIO_SET, Ioprio, Next),
        (Next, JEQ_K, NR_CLONE, Clone, Socketpair),
        // clone: refused with any namespace bit in its flags.
        (Clone, LD_W_ABS, OFFSET_ARG0, Next, Next),
        (Next, JSET_K, CLONE_NEW_MASK, Eperm, Allow),
        // socketpair: only an AF_UNIX stream or seqpacket pair.
        (Socketpair, JEQ_K, NR_SOCKETPAIR, Next, Allow),
        (Next, LD_W_ABS, OFFSET_ARG0, Next, Next),
        (Next, JEQ_K, AF_UNIX, Next, Eperm),
        (Next, LD_W_ABS, OFFSET_ARG1, Next, Next),
        (Next, AND_K, SOCK_TYPE_MASK, Next, Next),
        (Next, JEQ_K, SOCK_STREAM, Allow, Next),
        (Next, JEQ_K, SOCK_SEQPACKET, Allow, Eperm),
        // PID_TESTED: pid 0 or the tool's own.
        (Pid, LD_W_ABS, OFFSET_ARG0, Next, Next),
        (Next, JEQ_K, 0, Allow, Next),
        (Next, JEQ_K, self_pid, Allow, Eperm),
        // setpriority and ioprio_set: never every process of the user.
        (Priority, LD_W_ABS, OFFSET_ARG0, Next, Next),
        (Next, JEQ_K, PRIO_USER, Eperm, Allow),
        (Ioprio, LD_W_ABS, OFFSET_ARG0, Next, Next),
        (Next, JEQ_K, IOPRIO_WHO_USER, Eperm, Allow),
        (Eperm, RET_K, RET_ERRNO | EPERM, Next, Next),
        (Enosys, RET_K, RET_ERRNO | ENOSYS, Next, Next),
        (Kill, RET_K, RET_KILL_PROCESS, Next, Next),
        (Allow, RET_K, RET_ALLOW, Next, Next),
    ]);
    // Jumps name labels; each becomes the forward offset BPF counts from the
    // next instruction.
    let at = |want: Label| {
        prog.iter()
            .position(|i| i.0 == want)
            .expect("every label is placed")
    };
    let mut p = Vec::with_capacity(prog.len());
    for (i, &(_, code, k, jt, jf)) in prog.iter().enumerate() {
        let off = |t: Label| match t {
            Next => 0,
            t => u8::try_from(at(t) - i - 1).expect("a forward jump that fits in 8 bits"),
        };
        p.push(sock_filter {
            code,
            jt: off(jt),
            jf: off(jf),
            k,
        });
    }
    Some(p)
}

/// Jump targets in `seccomp_filter`'s program; `Next` falls through.
#[cfg(target_arch = "x86_64")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Label {
    Next,
    Eperm,
    Enosys,
    Kill,
    Allow,
    Clone,
    Socketpair,
    Pid,
    Priority,
    Ioprio,
}

/// No filter exists for other architectures, so the call refuses with
/// `SandboxUnavailable`, `id` `seccomp`, rather than running unfiltered.
#[cfg(not(target_arch = "x86_64"))]
pub fn seccomp_filter(_self_pid: u32) -> Option<BpfProgram> {
    None
}

/// Drops every capability it can, then checks (AET-WO-1600, ruled on PR #51):
///
/// 1. With `CAP_SETPCAP` effective (root in the image), empties the bounding
///    set. This must come before step 3, which takes `CAP_SETPCAP` away; the
///    reverse order leaves the bounding set full (measured in the spike). A
///    user on a host cannot empty it and need not: under `no_new_privs` an
///    exec gains nothing from it.
/// 2. Clears the ambient set.
/// 3. Clears the effective, permitted and inheritable sets.
/// 4. Verifies that those four sets are empty and `no_new_privs` is set
///    (Landlock and the filter both set it). Step 1 verifies itself.
///
/// Any failure is an error, and the call refuses (`INV-06`). A trampoline
/// that started as root emptied the bounding set in [`separate_uid`], before
/// the uid change took `CAP_SETPCAP` away.
fn drop_capabilities() -> Result<(), String> {
    let before = capabilities(None).map_err(|e| errno("capget", e))?;
    let setpcap = before.effective.contains(CapabilitySet::SETPCAP);
    if setpcap {
        empty_bounding_set()?;
    }
    clear_ambient_capability_set().map_err(|e| errno("clearing the ambient set", e))?;
    let none = CapabilitySets {
        effective: CapabilitySet::empty(),
        permitted: CapabilitySet::empty(),
        inheritable: CapabilitySet::empty(),
    };
    set_capabilities(None, none).map_err(|e| errno("capset", e))?;
    let after = capabilities(None).map_err(|e| errno("capget", e))?;
    if !(after.effective.is_empty() && after.permitted.is_empty() && after.inheritable.is_empty()) {
        return Err(format!("capabilities still held: {after:?}"));
    }
    let left = each_capability(capability_is_in_ambient_set)
        .map_err(|e| errno("reading the ambient set", e))?;
    if !left.is_empty() {
        return Err(format!("ambient capabilities still held: {left:?}"));
    }
    match no_new_privs() {
        Ok(true) => Ok(()),
        Ok(false) => Err("no_new_privs is not set".to_owned()),
        Err(e) => Err(errno("reading no_new_privs", e)),
    }
}

fn errno(what: &str, e: Errno) -> String {
    format!("{what}: errno {}", e.raw_os_error())
}

/// Every capability `test` says is held. Capability numbers run from 0 to the
/// kernel's last; past it, `EINVAL`.
fn each_capability(
    test: fn(CapabilitySet) -> rustix::io::Result<bool>,
) -> rustix::io::Result<Vec<CapabilitySet>> {
    let mut held = Vec::new();
    for n in 0..64 {
        let cap = CapabilitySet::from_bits_retain(1 << n);
        match test(cap) {
            Ok(true) => held.push(cap),
            Ok(false) => {}
            Err(Errno::INVAL) if n > 0 => break,
            Err(e) => return Err(e),
        }
    }
    Ok(held)
}

/// Empties the bounding set, then reads it back; needs `CAP_SETPCAP` effective.
fn empty_bounding_set() -> Result<(), String> {
    let read = || {
        each_capability(capability_is_in_bounding_set)
            .map_err(|e| errno("reading the bounding set", e))
    };
    for cap in read()? {
        remove_capability_from_bounding_set(cap)
            .map_err(|e| errno(&format!("dropping {cap:?} from the bounding set"), e))?;
    }
    match read()? {
        left if left.is_empty() => Ok(()),
        left => Err(format!("bounding capabilities still held: {left:?}")),
    }
}

/// The trampoline, started as root, becomes [`TOOL_UID`] and [`TOOL_GID`]
/// with no supplementary group (`AET-WO-1601` §1.2), then reads every id back
/// from the kernel. The bounding set is emptied first: the uid change clears
/// the effective set, and `CAP_SETPCAP` with it. Each call is the thread's own
/// system call (`rustix::thread`), and this process has one thread here.
fn separate_uid() -> Result<(), String> {
    empty_bounding_set()?;
    let (uid, gid) = (Uid::from_raw(TOOL_UID), Gid::from_raw(TOOL_GID));
    set_thread_groups(&[]).map_err(|e| errno("setgroups", e))?;
    set_thread_res_gid(gid, gid, gid).map_err(|e| errno("setresgid", e))?;
    set_thread_res_uid(uid, uid, uid).map_err(|e| errno("setresuid", e))?;
    let status = fs::read_to_string("/proc/thread-self/status")
        .map_err(|e| format!("cannot read back the ids: {e}"))?;
    ids_are_the_tool_s(&status)
}

/// `/proc/<pid>/status` says the real, effective, saved and filesystem uid
/// and gid are all the tool's, and there is no supplementary group.
fn ids_are_the_tool_s(status: &str) -> Result<(), String> {
    let field = |name: &str| {
        status
            .lines()
            .find_map(|l| l.strip_prefix(name))
            .map(|v| v.split_whitespace().collect::<Vec<_>>())
            .ok_or_else(|| format!("no {name} line in the read-back"))
    };
    let uid = TOOL_UID.to_string();
    let gid = TOOL_GID.to_string();
    let (uids, gids, groups) = (field("Uid:")?, field("Gid:")?, field("Groups:")?);
    if uids.len() != 4 || uids.iter().any(|u| *u != uid) {
        return Err(format!("uids read back as {uids:?}"));
    }
    if gids.len() != 4 || gids.iter().any(|g| *g != gid) {
        return Err(format!("gids read back as {gids:?}"));
    }
    if !groups.is_empty() {
        return Err(format!("supplementary groups read back as {groups:?}"));
    }
    Ok(())
}

/// Applies Landlock to this process: filesystem rights through ABI 3 are
/// required; where the kernel can, TCP bind and connect are denied (ABI 4+)
/// and signals and abstract Unix sockets are scoped to the tool (ABI 6+).
fn confine(read_only: &[PathBuf], read_write: &[PathBuf]) -> Result<i32, String> {
    let fs_rights = AccessFs::from_all(ABI::V3);
    let status = Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(fs_rights)
        .and_then(|r| {
            r.set_compatibility(CompatLevel::BestEffort)
                .handle_access(AccessNet::from_all(ABI::V4))
        })
        .and_then(|r| r.scope(Scope::from_all(ABI::V6)))
        .and_then(|r| r.create())
        .and_then(|r| r.add_rules(path_beneath_rules(read_only, AccessFs::from_read(ABI::V3))))
        .and_then(|r| r.add_rules(path_beneath_rules(read_write, fs_rights)))
        .and_then(|r| r.restrict_self())
        .map_err(|e| e.to_string())?;
    let abi = match status.landlock {
        LandlockStatus::Available { effective_abi, .. } => effective_abi as i32,
        other => return Err(format!("{other:?}")),
    };
    match status.ruleset {
        RulesetStatus::NotEnforced => Err("the ruleset was not enforced".to_owned()),
        RulesetStatus::PartiallyEnforced | RulesetStatus::FullyEnforced => Ok(abi),
    }
}

/// The value of a variable the caller must provide, as an existing directory
/// when `dir` is set.
pub fn required_var(name: &'static str, dir: bool) -> Result<OsString, Stop> {
    let value = env::var_os(name).filter(|v| !v.is_empty());
    match value {
        Some(v) if !dir || Path::new(&v).is_dir() => Ok(v),
        Some(v) => Err(Stop::unusable(
            name,
            format!("{name}={} is not a directory", Path::new(&v).display()),
        )),
        None => Err(Stop::unusable(name, format!("{name} is not set"))),
    }
}

/// Builds the OS-string argument list for a tool.
pub fn args<I, S>(items: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    items.into_iter().map(|s| s.as_ref().to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    const TEST_BOUND: Duration = Duration::from_millis(200);

    #[test]
    fn a_removal_that_always_fails_gives_up_with_70_after_the_bound() {
        let mut calls = 0;
        let started = Instant::now();
        let code = remove_with_retry(Path::new("/nonexistent/aethelgard-x"), TEST_BOUND, |_| {
            calls += 1;
            Err(io::Error::from_raw_os_error(39)) // ENOTEMPTY
        });
        let took = started.elapsed();
        assert_eq!(code, 70);
        assert!(calls > 1, "never retried");
        assert!(took >= TEST_BOUND, "gave up early: {took:?}");
        assert!(took < TEST_BOUND * 2, "gave up late: {took:?}");
    }

    #[test]
    fn a_removal_that_fails_3_times_then_succeeds_returns_0_after_4_calls() {
        let mut calls = 0;
        let code = remove_with_retry(Path::new("/nonexistent/aethelgard-x"), TEST_BOUND, |_| {
            calls += 1;
            if calls <= 3 {
                Err(io::Error::from_raw_os_error(39))
            } else {
                Ok(())
            }
        });
        assert_eq!((code, calls), (0, 4));
    }

    #[test]
    fn not_found_with_the_directory_absent_returns_0_on_the_first_call() {
        let mut calls = 0;
        let code = remove_with_retry(Path::new("/nonexistent/aethelgard-x"), TEST_BOUND, |_| {
            calls += 1;
            Err(io::ErrorKind::NotFound.into())
        });
        assert_eq!((code, calls), (0, 1));
    }

    #[test]
    fn not_found_while_the_directory_exists_keeps_retrying() {
        let mut calls = 0;
        // `/` exists, so a `NotFound` from the remover is a lost race, not a clean exit.
        let code = remove_with_retry(Path::new("/"), TEST_BOUND, |_| {
            calls += 1;
            Err(io::ErrorKind::NotFound.into())
        });
        assert_eq!(code, 70);
        assert!(calls > 1, "returned on the first NotFound");
    }

    /// The three conditions (ruled on PR #96): the identity map is the only
    /// initial namespace; an entry is clean only when root's and writable by
    /// no group or other (a sticky ancestor allowed); a path a user owns is
    /// never clean, a system directory is not walked; and the first failed
    /// condition is the one named.
    #[test]
    fn uid_boundary_holds_only_when_all_three_conditions_do() {
        assert!(identity_map("         0          0 4294967295\n"));
        assert!(!identity_map("0 1001 1\n1 165536 65536\n"));
        assert!(!identity_map("0 0 65536\n"));
        assert!(!identity_map("0 0 4294967295\n1 1 1\n"));
        assert!(entry_ok(0, 0, 0o40755, false));
        assert!(
            entry_ok(0, 0, 0o40775, false),
            "root's group writing grants nothing"
        );
        assert!(!entry_ok(0, 50, 0o40775, false), "another group writing");
        assert!(!entry_ok(0, 0, 0o40757, false));
        assert!(!entry_ok(1001, 0, 0o40755, false));
        assert!(
            entry_ok(0, 0, 0o41777, true),
            "a sticky ancestor such as /tmp"
        );
        assert!(
            !entry_ok(0, 0, 0o41777, false),
            "a sticky directory in a toolchain"
        );
        assert!(
            toolchain_clean(Path::new("/usr/bin")),
            "the system set is not walked"
        );
        let mine = env::temp_dir().join(format!("aet-1601-clean-{}", std::process::id()));
        fs::create_dir_all(&mine).unwrap();
        assert_eq!(toolchain_clean(&mine), geteuid().is_root());
        fs::remove_dir_all(&mine).unwrap();
        // Where the kit and the toolchain are set (the image), the walk agrees
        // with `find` (in the pinned image, Kani's kit is uid 1001's: A1).
        for var in ["KANI_HOME", "RUSTUP_HOME"] {
            let Some(home) = env::var_os(var) else {
                continue;
            };
            let find = std::process::Command::new("find")
                .arg(&home)
                .args([
                    "!", "-type", "l", "(", "!", "-uid", "0", "-o", "-perm", "-002", "-o", "(",
                    "-perm", "-020", "!", "-gid", "0", ")", ")",
                ])
                .args(["-print", "-quit"])
                .output()
                .unwrap();
            assert_eq!(
                toolchain_clean(Path::new(&home)),
                find.stdout.is_empty(),
                "{var}"
            );
        }
        let mut s = SandboxReport::from_abi(6, true);
        assert_eq!((s.uid_boundary, s.uid_boundary_unmet), (true, None));
        s.unmet(None);
        assert!(s.uid_boundary);
        s.unmet(Some(Unmet::UserNamespace));
        s.unmet(Some(Unmet::Toolchain));
        assert_eq!(
            (s.uid_boundary, s.uid_boundary_unmet),
            (false, Some(Unmet::UserNamespace))
        );
        let s = SandboxReport::from_abi(6, false);
        assert_eq!(s.uid_boundary_unmet, Some(Unmet::ToolUid));
        let json = serde_json::to_value(s).unwrap();
        assert_eq!(json["uid_boundary_unmet"], "tool_uid");
    }

    /// `uid_boundary` is read only from the trampoline's own line, and only
    /// as `0` or `1`; anything else is no sandbox report at all.
    #[test]
    fn the_status_line_says_uid_boundary_exactly() {
        let ended = "\nended code=0\n";
        let sandbox = |first: &str| parse_status(&format!("{first}{ended}"), "t").map(|r| r.0);
        assert!(
            sandbox("sandbox abi=6 uid_boundary=1")
                .unwrap()
                .uid_boundary
        );
        assert!(
            !sandbox("sandbox abi=6 uid_boundary=0")
                .unwrap()
                .uid_boundary
        );
        for bad in [
            "sandbox abi=6",
            "sandbox abi=6 uid_boundary=",
            "sandbox abi=6 uid_boundary=true",
            "sandbox abi=6 uid_boundary=2",
            "sandbox abi=6 uid_boundary=1 x",
            "sandbox abi=x uid_boundary=1",
            "sandbox uid_boundary=1 abi=6",
        ] {
            let stop = sandbox(bad).unwrap_err();
            assert_eq!(stop.kind, "ToolError", "{bad}");
        }
        let stop = parse_status("unavailable uid setresuid: errno 1\n", "t").unwrap_err();
        assert_eq!((stop.code, stop.kind), (4, "SandboxUnavailable"));
        assert_eq!(stop.id.as_deref(), Some("uid"));
    }

    /// The read-back accepts only every id the tool's and no group at all.
    #[test]
    fn the_read_back_needs_all_four_ids_and_no_group() {
        let status = |uid: &str, gid: &str, groups: &str| {
            format!("Name:\tx\nUid:\t{uid}\nGid:\t{gid}\nGroups:\t{groups}\n")
        };
        let four = |id: u32| [id; 4].map(|i| i.to_string()).join("\t");
        let (u, g) = (four(TOOL_UID), four(TOOL_GID));
        assert!(ids_are_the_tool_s(&status(&u, &g, "")).is_ok());
        let one_off = |id: u32, at: usize| {
            let mut ids = [
                id.to_string(),
                id.to_string(),
                id.to_string(),
                id.to_string(),
            ];
            ids[at] = "0".to_owned();
            ids.join("\t")
        };
        for (uid, gid, groups) in [
            (one_off(TOOL_UID, 3), g.clone(), String::new()),
            (one_off(TOOL_UID, 0), g.clone(), String::new()),
            (u.clone(), one_off(TOOL_GID, 2), String::new()),
            (u.clone(), g.clone(), "0".to_owned()),
            (u.clone(), g.clone(), TOOL_GID.to_string()),
            (u.clone(), u.clone(), String::new()),
            (
                [TOOL_UID; 3].map(|i| i.to_string()).join("\t"),
                g.clone(),
                String::new(),
            ),
        ] {
            assert!(
                ids_are_the_tool_s(&status(&uid, &gid, &groups)).is_err(),
                "{uid} / {gid} / {groups}"
            );
        }
        assert!(ids_are_the_tool_s(&format!("Uid:\t{u}\n")).is_err());
    }

    /// As root (tier 1 in the image), a directory is handed over once: its
    /// entries sealed and the directory made root's, group [`TOOL_GID`],
    /// mode [`HANDED_OVER`]; an entry not root's before then refuses the
    /// hand-over; a second call changes nothing. As a user there is nothing
    /// to hand over, and the CLI never calls it.
    #[test]
    fn a_directory_is_handed_over_once_and_only_if_root_made_everything_in_it() {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt};
        if !geteuid().is_root() {
            return;
        }
        let base = env::temp_dir().join(format!("aet-1601-handover-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let make = |name: &str| {
            let d = base.join(name);
            fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(&d)
                .unwrap();
            fs::set_permissions(&d, fs::Permissions::from_mode(0o700)).unwrap();
            fs::create_dir(d.join("crate")).unwrap();
            fs::write(d.join("crate/lib.rs"), b"x").unwrap();
            d
        };
        let mode = |p: &Path| fs::symlink_metadata(p).unwrap().mode() & 0o7777;
        let ok = make("ok");
        hand_over(&ok).unwrap();
        let meta = fs::metadata(&ok).unwrap();
        assert_eq!(
            (meta.uid(), meta.gid(), mode(&ok)),
            (0, TOOL_GID, HANDED_OVER)
        );
        assert_eq!(mode(&ok.join("crate")), 0o555);
        assert_eq!(mode(&ok.join("crate/lib.rs")), 0o444);
        // A second call finds it handed over and seals nothing new.
        fs::write(ok.join("later"), b"y").unwrap();
        hand_over(&ok).unwrap();
        assert_eq!(mode(&ok.join("later")), 0o644);
        // A directory the CLI made add-only keeps its mode; its files are sealed.
        let marked = make("marked");
        add_only(&marked.join("crate")).unwrap();
        hand_over(&marked).unwrap();
        let crate_dir = fs::metadata(marked.join("crate")).unwrap();
        assert_eq!(
            (
                crate_dir.uid(),
                crate_dir.gid(),
                mode(&marked.join("crate"))
            ),
            (0, TOOL_GID, HANDED_OVER)
        );
        assert_eq!(mode(&marked.join("crate/lib.rs")), 0o444);
        // The CLI copies no link, so a link in the directory refuses the hand-over.
        let linked = make("linked");
        std::os::unix::fs::symlink("/etc/passwd", linked.join("crate/link")).unwrap();
        assert!(
            hand_over(&linked)
                .unwrap_err()
                .contains("not a file or a directory")
        );
        let theirs = make("theirs");
        std::os::unix::fs::chown(theirs.join("crate/lib.rs"), Some(4242), None).unwrap();
        assert!(hand_over(&theirs).unwrap_err().contains("not root's"));
        assert_eq!(mode(&theirs), 0o700, "a refused directory was handed over");
        fs::remove_dir_all(&base).unwrap();
    }

    /// The filter as an exact instruction sequence: any edit to it must show
    /// up here (ruled on PR #15, extended by AET-WO-1600). (code, jt, jf, k),
    /// with 4242 standing for the tool's pid.
    #[test]
    fn seccomp_filter_is_exactly_this_program() {
        let got: Vec<(u16, u8, u8, u32)> = seccomp_filter(4242)
            .unwrap()
            .iter()
            .map(|i| (i.code, i.jt, i.jf, i.k))
            .collect();
        let want = vec![
            (0x20, 0, 0, 4),             // 0: load arch
            (0x15, 0, 63, 0xc000_003e),  // 1: x86_64, else kill
            (0x20, 0, 0, 0),             // 2: load nr
            (0x45, 61, 0, 0x4000_0000),  // 3: x32 bit: kill
            (0x15, 58, 0, 41),           // 4: socket: EPERM
            (0x15, 57, 0, 425),          // 5: io_uring_setup: EPERM
            (0x15, 56, 0, 426),          // 6: io_uring_enter: EPERM
            (0x15, 55, 0, 427),          // 7: io_uring_register: EPERM
            (0x15, 54, 0, 29),           // 8: shmget: EPERM
            (0x15, 53, 0, 30),           // 9: shmat: EPERM
            (0x15, 52, 0, 31),           // 10: shmctl: EPERM
            (0x15, 51, 0, 67),           // 11: shmdt: EPERM
            (0x15, 50, 0, 64),           // 12: semget: EPERM
            (0x15, 49, 0, 65),           // 13: semop: EPERM
            (0x15, 48, 0, 66),           // 14: semctl: EPERM
            (0x15, 47, 0, 68),           // 15: msgget: EPERM
            (0x15, 46, 0, 69),           // 16: msgsnd: EPERM
            (0x15, 45, 0, 70),           // 17: msgrcv: EPERM
            (0x15, 44, 0, 71),           // 18: msgctl: EPERM
            (0x15, 43, 0, 220),          // 19: semtimedop: EPERM
            (0x15, 42, 0, 272),          // 20: unshare: EPERM
            (0x15, 41, 0, 308),          // 21: setns: EPERM
            (0x15, 40, 0, 434),          // 22: pidfd_open: EPERM
            (0x15, 39, 0, 438),          // 23: pidfd_getfd: EPERM
            (0x15, 38, 0, 424),          // 24: pidfd_send_signal: EPERM
            (0x15, 37, 0, 101),          // 25: ptrace: EPERM
            (0x15, 36, 0, 310),          // 26: process_vm_readv: EPERM
            (0x15, 35, 0, 311),          // 27: process_vm_writev: EPERM
            (0x15, 34, 0, 312),          // 28: kcmp: EPERM
            (0x15, 33, 0, 250),          // 29: keyctl: EPERM
            (0x15, 32, 0, 248),          // 30: add_key: EPERM
            (0x15, 31, 0, 249),          // 31: request_key: EPERM
            (0x15, 30, 0, 321),          // 32: bpf: EPERM
            (0x15, 29, 0, 298),          // 33: perf_event_open: EPERM
            (0x15, 28, 0, 323),          // 34: userfaultfd: EPERM
            (0x15, 27, 0, 240),          // 35: mq_open: EPERM
            (0x15, 26, 0, 241),          // 36: mq_unlink: EPERM
            (0x15, 25, 0, 242),          // 37: mq_timedsend: EPERM
            (0x15, 24, 0, 243),          // 38: mq_timedreceive: EPERM
            (0x15, 23, 0, 244),          // 39: mq_notify: EPERM
            (0x15, 22, 0, 245),          // 40: mq_getsetattr: EPERM
            (0x15, 22, 0, 435),          // 41: clone3: ENOSYS
            (0x15, 13, 0, 302),          // 42: prlimit64: the pid test
            (0x15, 12, 0, 203),          // 43: sched_setaffinity: the pid test
            (0x15, 14, 0, 141),          // 44: setpriority: the which test
            (0x15, 15, 0, 251),          // 45: ioprio_set: the which test
            (0x15, 0, 2, 56),            // 46: clone? else to socketpair
            (0x20, 0, 0, 16),            // 47: load clone's flags
            (0x45, 14, 17, 0x7e02_0000), // 48: a CLONE_NEW* bit: EPERM, else allow
            (0x15, 0, 16, 53),           // 49: socketpair? else allow
            (0x20, 0, 0, 16),            // 50: load family
            (0x15, 0, 11, 1),            // 51: AF_UNIX, else EPERM
            (0x20, 0, 0, 24),            // 52: load type
            (0x54, 0, 0, 0xf),           // 53: type & SOCK_TYPE_MASK
            (0x15, 11, 0, 1),            // 54: SOCK_STREAM: allow
            (0x15, 10, 7, 5),            // 55: SOCK_SEQPACKET: allow, else EPERM
            (0x20, 0, 0, 16),            // 56: load the pid
            (0x15, 8, 0, 0),             // 57: pid 0: allow
            (0x15, 7, 4, 4242),          // 58: the tool's own pid: allow, else EPERM
            (0x20, 0, 0, 16),            // 59: load setpriority's which
            (0x15, 2, 5, 2),             // 60: PRIO_USER: EPERM, else allow
            (0x20, 0, 0, 16),            // 61: load ioprio_set's which
            (0x15, 0, 3, 3),             // 62: IOPRIO_WHO_USER: EPERM, else allow
            (0x06, 0, 0, 0x0005_0001),   // 63: errno EPERM
            (0x06, 0, 0, 0x0005_0026),   // 64: errno ENOSYS
            (0x06, 0, 0, 0x8000_0000),   // 65: kill process
            (0x06, 0, 0, 0x7fff_0000),   // 66: allow
        ];
        assert_eq!(got, want);
    }

    /// A classic-BPF interpreter for exactly the opcodes the filter uses, run
    /// over `seccomp_data` built here: the oracle for the exhaustive test
    /// (ruled on PR #51, F8). Anything else it meets is a failure.
    fn run_bpf(p: &[sock_filter], nr: u32, arch: u32, args: [u64; 6]) -> u32 {
        let mut data = [0u8; 64];
        data[0..4].copy_from_slice(&nr.to_le_bytes());
        data[4..8].copy_from_slice(&arch.to_le_bytes());
        for (i, a) in args.iter().enumerate() {
            data[16 + 8 * i..24 + 8 * i].copy_from_slice(&a.to_le_bytes());
        }
        let (mut pc, mut acc) = (0usize, 0u32);
        for _ in 0..p.len() {
            let op = &p[pc];
            let jump = |taken: bool| pc + 1 + usize::from(if taken { op.jt } else { op.jf });
            pc = match op.code {
                0x20 => {
                    let k = op.k as usize;
                    acc = u32::from_le_bytes(data[k..k + 4].try_into().unwrap());
                    pc + 1
                }
                0x54 => {
                    acc &= op.k;
                    pc + 1
                }
                0x15 => jump(acc == op.k),
                0x45 => jump(acc & op.k != 0),
                0x06 => return op.k,
                other => panic!("opcode {other:#x} at {pc}"),
            };
            assert!(pc < p.len(), "fell off the end of the program");
        }
        panic!("no return within {} steps", p.len());
    }

    /// `#define NAME VALUE` lines from a kernel header, as name → value.
    fn header(path: &str, prefix: &str) -> BTreeMap<String, u64> {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{path}: {e} (the kernel headers are the oracle)"));
        text.lines()
            .filter_map(|l| {
                let mut w = l.split_whitespace();
                (w.next()? == "#define").then_some(())?;
                let name = w.next()?.strip_prefix(prefix)?;
                let v = w.next()?;
                let v = match v.strip_prefix("0x") {
                    Some(hex) => u64::from_str_radix(hex, 16).ok()?,
                    None => v.parse().ok()?,
                };
                Some((name.to_owned(), v))
            })
            .collect()
    }

    /// Every number, both ABIs, every `clone` flag bit and every
    /// `socketpair` family and type, against what the rulings say, with the
    /// numbers taken from the kernel's own headers rather than from the filter.
    #[test]
    fn the_filter_refuses_exactly_what_was_ruled_for_every_number() {
        const X86_64: u32 = 0xc000_003e;
        const ALLOW: u32 = 0x7fff_0000;
        const EPERM: u32 = 0x0005_0001;
        const ENOSYS: u32 = 0x0005_0026;
        const KILL: u32 = 0x8000_0000;
        let refused = [
            "socket",
            "io_uring_setup",
            "io_uring_enter",
            "io_uring_register",
            "shmget",
            "shmat",
            "shmdt",
            "shmctl",
            "msgget",
            "msgsnd",
            "msgrcv",
            "msgctl",
            "semget",
            "semop",
            "semtimedop",
            "semctl",
            "unshare",
            "setns",
            "pidfd_open",
            "pidfd_getfd",
            "pidfd_send_signal",
            "ptrace",
            "process_vm_readv",
            "process_vm_writev",
            "kcmp",
            "keyctl",
            "add_key",
            "request_key",
            "bpf",
            "perf_event_open",
            "userfaultfd",
            "mq_open",
            "mq_unlink",
            "mq_timedsend",
            "mq_timedreceive",
            "mq_notify",
            "mq_getsetattr",
        ];
        let nrs = header("/usr/include/x86_64-linux-gnu/asm/unistd_64.h", "__NR_");
        let nr = |name: &str| u32::try_from(nrs[name]).unwrap();
        let refused: BTreeMap<u32, &str> = refused.iter().map(|&n| (nr(n), n)).collect();
        assert_eq!(refused.len(), 37, "37 distinct numbers");
        let (clone, clone3, socketpair) = (nr("clone"), nr("clone3"), nr("socketpair"));
        let pid_tested = [nr("prlimit64"), nr("sched_setaffinity")];
        let (setpriority, ioprio_set) = (nr("setpriority"), nr("ioprio_set"));
        let me = 4242u64;
        let p = seccomp_filter(4242).unwrap();
        for n in 0..0x1_0000u32 {
            for arch in [0x4000_0003, 0xc000_00b7, 0x4000_003e, 0] {
                assert_eq!(run_bpf(&p, n, arch, [0; 6]), KILL, "arch {arch:#x} nr {n}");
            }
            assert_eq!(
                run_bpf(&p, n | 0x4000_0000, X86_64, [0; 6]),
                KILL,
                "x32 {n}"
            );
            if [clone, socketpair, setpriority, ioprio_set].contains(&n) || pid_tested.contains(&n)
            {
                continue;
            }
            let want = if refused.contains_key(&n) {
                EPERM
            } else if n == clone3 {
                ENOSYS
            } else {
                ALLOW
            };
            assert_eq!(run_bpf(&p, n, X86_64, [!0; 6]), want, "nr {n}");
        }
        for n in [
            0x8000_0000 | 41,
            0x8000_0000 | clone3,
            0x3fff_ffff,
            0xffff_ffff,
        ] {
            let want = if n & 0x4000_0000 != 0 { KILL } else { ALLOW };
            assert_eq!(run_bpf(&p, n, X86_64, [0; 6]), want, "nr {n:#x}");
        }
        // clone: refused exactly when a CLONE_NEW* bit that fits its 32-bit
        // flags word is set; the high word is never read.
        let sched = header("/usr/include/linux/sched.h", "CLONE_NEW");
        let mask = sched
            .values()
            .filter(|&&v| v & 0xff == 0 && v <= u64::from(u32::MAX))
            .fold(0u64, |m, v| m | v);
        assert_eq!(sched.len(), 8, "CLONE_NEW* in linux/sched.h: {sched:?}");
        for flags in [0x11u64, 0x003d_0f00, 0x0120_0011, 0x4111] {
            assert_eq!(
                run_bpf(&p, clone, X86_64, [flags, 0, 0, 0, 0, 0]),
                ALLOW,
                "{flags:#x}"
            );
        }
        for bit in 0..64 {
            let flags = 0x11u64 | (1 << bit);
            let want = if bit < 32 && mask & (1 << bit) != 0 {
                EPERM
            } else {
                ALLOW
            };
            assert_eq!(
                run_bpf(&p, clone, X86_64, [flags, 0, 0, 0, 0, 0]),
                want,
                "clone bit {bit}"
            );
        }
        // prlimit64 and sched_setaffinity: pid 0 and the tool's own; the pid
        // is an int, so only the low word counts, for the kernel too.
        for n in pid_tested {
            for pid in [
                0,
                me,
                me - 1,
                me + 1,
                1,
                0xffff_ffff,
                me | (1 << 32),
                1 << 32,
            ] {
                let ok = pid & 0xffff_ffff == 0 || pid & 0xffff_ffff == me;
                let got = run_bpf(&p, n, X86_64, [pid, 0, 0, 0, 0, 0]);
                assert_eq!(got, if ok { ALLOW } else { EPERM }, "nr {n} pid {pid:#x}");
            }
        }
        // setpriority's PRIO_USER (linux/resource.h) and ioprio_set's
        // IOPRIO_WHO_USER (an enum in linux/ioprio.h, so a literal here):
        // refused for any who; every other which allowed.
        let prio_user = header("/usr/include/linux/resource.h", "PRIO_")["USER"];
        for (n, user) in [(setpriority, prio_user), (ioprio_set, 3)] {
            for which in (0..8u64).chain([user | (1 << 32)]) {
                for who in [0, me, 1, u64::MAX] {
                    let refused = which & 0xffff_ffff == user;
                    let got = run_bpf(&p, n, X86_64, [which, who, 19, 0, 0, 0]);
                    assert_eq!(
                        got,
                        if refused { EPERM } else { ALLOW },
                        "nr {n} {which:#x} {who}"
                    );
                }
            }
        }
        // socketpair: an AF_UNIX stream or seqpacket pair, with any flags.
        for family in 0..64u64 {
            for kind in 0..16u64 {
                for flags in [0u64, 0x800, 0x8_0000, 0x8_0800] {
                    let ok = family == 1 && (kind == 1 || kind == 5);
                    let got = run_bpf(&p, socketpair, X86_64, [family, kind | flags, 0, 0, 0, 0]);
                    assert_eq!(
                        got,
                        if ok { ALLOW } else { EPERM },
                        "{family} {kind} {flags:#x}"
                    );
                }
            }
        }
    }

    #[test]
    fn an_unavailable_line_is_sandbox_unavailable_naming_its_layer() {
        for layer in ["seccomp", "landlock", "capabilities"] {
            let stop = parse_status(&format!("unavailable {layer} some reason\n"), "kicad-cli")
                .unwrap_err();
            assert_eq!(
                (stop.code, stop.kind, stop.id.as_deref()),
                (4, "SandboxUnavailable", Some(layer))
            );
        }
    }
}
