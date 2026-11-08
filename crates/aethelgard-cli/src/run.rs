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
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::{CommandExt, ExitStatusExt, parent_id};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, exit};
use std::thread;
use std::time::{Duration, Instant};

use landlock::{
    ABI, Access, AccessFs, AccessNet, CompatLevel, Compatible, LandlockStatus, Ruleset,
    RulesetAttr, RulesetCreatedAttr, RulesetStatus, Scope, path_beneath_rules,
};
use rustix::io::Errno;
use rustix::process::{
    Pid, Signal, WaitOptions, getpid, kill_process, kill_process_group, set_child_subreaper,
    waitpid,
};
use rustix::thread::{
    CapabilitySet, CapabilitySets, capabilities, capability_is_in_ambient_set,
    capability_is_in_bounding_set, clear_ambient_capability_set, no_new_privs,
    remove_capability_from_bounding_set, set_capabilities,
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
        for dir in env::split_paths(&path_var) {
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
            let path = if candidate.is_absolute() {
                candidate
            } else {
                env::current_dir()
                    .map_err(|e| Stop::unusable(name, e.to_string()))?
                    .join(candidate)
            };
            return Ok(Self { name, path });
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
}

impl SandboxReport {
    /// Filesystem rights are required and everything else is requested
    /// best-effort, so what was enforced follows from the effective ABI.
    fn from_abi(landlock_abi: i32) -> Self {
        Self {
            landlock_abi,
            tcp_confined: landlock_abi >= 4,
            scoped: landlock_abi >= 6,
            sockets_denied: true,
        }
    }
}

/// How a tool run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    Code(i32),
    Signal(i32),
}

pub struct Output {
    pub ended: Ended,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub sandbox: SandboxReport,
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
        let me = env::current_exe().map_err(|e| {
            Stop::tool_error(self.tool.name, format!("cannot find this executable: {e}"))
        })?;
        let mut cmd = Command::new(me);
        cmd.arg(REAPER_ARG)
            .arg(CLI_PID_ARG)
            .arg(std::process::id().to_string())
            .arg("--status")
            .arg(&self.status);
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
        let stdout = capture(child.stdout.take().expect("piped stdout"));
        let stderr = capture(child.stderr.take().expect("piped stderr"));
        let reaper = child.wait();
        let stdout = stdout.join().expect("stdout reader");
        let stderr = stderr.join().expect("stderr reader");
        let reaper = reaper.map_err(|e| {
            Stop::tool_error(self.tool.name, format!("waiting for the reaper: {e}"))
        })?;
        let (Some(stdout), Some(stderr)) = (stdout, stderr) else {
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
        Ok(Output {
            ended,
            stdout,
            stderr,
            sandbox,
        })
    }
}

/// Reads a pipe to its end, keeping at most `MAX_CAPTURE` bytes. Past that it
/// keeps draining, so the tool never blocks on a full pipe, and yields `None`.
/// A tool that prints forever is ended by the caller's timeout (`RUL-13`).
fn capture(mut pipe: impl Read + Send + 'static) -> thread::JoinHandle<Option<Vec<u8>>> {
    thread::spawn(move || {
        let mut buf = Vec::new();
        let mut overflow = false;
        let mut chunk = [0u8; 64 * 1024];
        loop {
            match pipe.read(&mut chunk) {
                Ok(0) => return (!overflow).then_some(buf),
                Ok(n) if overflow || buf.len() + n > MAX_CAPTURE => {
                    overflow = true;
                    buf = Vec::new();
                }
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => return (!overflow).then_some(buf),
            }
        }
    })
}

/// The status file holds one line about the sandbox, then one about how the
/// tool ended: `sandbox abi=8` then `ended code=0` or `ended signal=9`;
/// or a single `unavailable …` or `spawn-error …` line.
fn parse_status(status: &str, tool: &str) -> Result<(SandboxReport, Ended), Stop> {
    let mut lines = status.lines();
    let first = lines.next().unwrap_or_default();
    if let Some(rest) = first.strip_prefix("unavailable ") {
        // Only the trampoline writes this line: `landlock`, `seccomp` or
        // `capabilities`.
        let (layer, reason) = rest.split_once(' ').unwrap_or((rest, ""));
        return Err(Stop {
            code: EXIT_TOOL_UNAVAILABLE,
            kind: "SandboxUnavailable",
            id: Some(layer.to_owned()),
            message: format!("the required {layer} sandbox layer cannot be enforced: {reason}"),
        });
    }
    let sandbox = first
        .strip_prefix("sandbox abi=")
        .and_then(|abi| abi.parse().ok())
        .map(SandboxReport::from_abi)
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
            Some("--ro") => read_only.extend(it.next().map(PathBuf::from)),
            Some("--rw") => read_write.extend(it.next().map(PathBuf::from)),
            Some("--") => {
                tool = it.cloned().collect();
                break;
            }
            _ => exit(64),
        }
    }
    match (cli_pid, status) {
        (Some(cli_pid), Some(status)) if !tool.is_empty() => Internal {
            cli_pid,
            status,
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
        thread::sleep(Duration::from_millis(20));
    }
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
    let sandbox = match confine(&internal.read_only, &internal.read_write) {
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
    let _ = writeln!(status, "sandbox abi={}", sandbox.landlock_abi);
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
///    (Landlock and the filter both set it), and that the bounding set is empty
///    if step 1 ran.
///
/// Any failure is an error, and the call refuses (`INV-06`).
fn drop_capabilities() -> Result<(), String> {
    let errno = |what: &str, e: Errno| format!("{what}: errno {}", e.raw_os_error());
    // Capability numbers run from 0 to the kernel's last; past it, EINVAL.
    let each = |test: fn(CapabilitySet) -> rustix::io::Result<bool>| {
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
    };
    let before = capabilities(None).map_err(|e| errno("capget", e))?;
    let setpcap = before.effective.contains(CapabilitySet::SETPCAP);
    if setpcap {
        for cap in
            each(capability_is_in_bounding_set).map_err(|e| errno("reading the bounding set", e))?
        {
            remove_capability_from_bounding_set(cap)
                .map_err(|e| errno(&format!("dropping {cap:?} from the bounding set"), e))?;
        }
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
    let left =
        each(capability_is_in_ambient_set).map_err(|e| errno("reading the ambient set", e))?;
    if !left.is_empty() {
        return Err(format!("ambient capabilities still held: {left:?}"));
    }
    if setpcap {
        let left = each(capability_is_in_bounding_set)
            .map_err(|e| errno("reading the bounding set", e))?;
        if !left.is_empty() {
            return Err(format!("bounding capabilities still held: {left:?}"));
        }
    }
    match no_new_privs() {
        Ok(true) => Ok(()),
        Ok(false) => Err("no_new_privs is not set".to_owned()),
        Err(e) => Err(errno("reading no_new_privs", e)),
    }
}

/// Applies Landlock to this process: filesystem rights through ABI 3 are
/// required; where the kernel can, TCP bind and connect are denied (ABI 4+)
/// and signals and abstract Unix sockets are scoped to the tool (ABI 6+).
fn confine(read_only: &[PathBuf], read_write: &[PathBuf]) -> Result<SandboxReport, String> {
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
        RulesetStatus::PartiallyEnforced | RulesetStatus::FullyEnforced => {
            Ok(SandboxReport::from_abi(abi))
        }
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
