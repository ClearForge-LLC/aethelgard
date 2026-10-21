//! Running an external tool: finding it, sandboxing it, and capturing what it
//! did (`docs/architecture.md` §6, ruled on PR #12).
//!
//! The CLI never runs a tool directly. It starts a copy of itself as a
//! *reaper*, in a new process group (so a signal to the CLI's group does not
//! reach it), with an empty environment, stdin from `/dev/null` and its
//! working directory inside the call's private temporary directory. The reaper
//! starts a *trampoline*, a third copy, in a process group of its own; the
//! trampoline confines itself with Landlock, installs the seccomp filter that
//! refuses every socket Landlock cannot see (AET-WO-0300a), records the
//! sandbox it achieved, and becomes the tool. The reaper waits. If the CLI
//! dies, by any signal, the reaper sees its parent change and kills the tool's
//! group and every descendant; a guardian removes the temporary directory.
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
use std::time::Duration;

use landlock::{
    ABI, Access, AccessFs, AccessNet, CompatLevel, Compatible, LandlockStatus, Ruleset,
    RulesetAttr, RulesetCreatedAttr, RulesetStatus, Scope, path_beneath_rules,
};
use rustix::process::{
    Pid, Signal, WaitOptions, getpid, kill_process, kill_process_group, set_child_subreaper,
    waitpid,
};
use seccompiler::{BpfProgram, apply_filter, sock_filter};

use crate::{EXIT_TOOL_ERROR, EXIT_TOOL_UNAVAILABLE, EXIT_UNREADABLE};

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
        cmd.arg(REAPER_ARG).arg("--status").arg(&self.status);
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
        let (layer, reason) = rest.split_once(' ').unwrap_or((rest, ""));
        let message = match layer {
            "landlock" => {
                format!("Landlock cannot enforce the required filesystem rules: {reason}")
            }
            "seccomp" => {
                format!("the seccomp filter that refuses sockets cannot be installed: {reason}")
            }
            _ => {
                return Err(Stop::tool_error(
                    tool,
                    format!("the reaper reported an unknown sandbox layer: {rest}"),
                ));
            }
        };
        return Err(Stop {
            code: EXIT_TOOL_UNAVAILABLE,
            kind: "SandboxUnavailable",
            id: Some(layer.to_owned()),
            message,
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
/// `--status F [--ro P]… [--rw P]… -- TOOL ARGS…`.
struct Internal {
    status: PathBuf,
    read_only: Vec<PathBuf>,
    read_write: Vec<PathBuf>,
    tool: Vec<OsString>,
}

fn parse_internal(args: &[OsString]) -> Internal {
    let mut status = None;
    let (mut read_only, mut read_write, mut tool) = (Vec::new(), Vec::new(), Vec::new());
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.to_str() {
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
    match status {
        Some(status) if !tool.is_empty() => Internal {
            status,
            read_only,
            read_write,
            tool,
        },
        _ => exit(64),
    }
}

/// The reaper: `aethelgard --aethelgard-internal-reaper <internal args>`.
/// Unconfined, so it can clean up; never returns.
pub fn reaper_main(args: &[OsString]) -> ! {
    let internal = parse_internal(args);
    let parent = parent_id();
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
        if parent_id() != parent {
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

/// The guardian: `aethelgard --aethelgard-internal-guard <dir>`. In its own
/// process group from the moment the call's temporary directory exists, it
/// removes the directory if the CLI dies before removing it itself, whatever
/// the CLI was doing then. Never returns.
pub fn guard_main(args: &[OsString]) -> ! {
    let Some(dir) = args.first().map(PathBuf::from) else {
        exit(64)
    };
    if !dir
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|n| n.starts_with("aethelgard-"))
    {
        exit(64);
    }
    let parent = parent_id();
    loop {
        if !dir.exists() {
            exit(0);
        }
        if parent_id() != parent {
            let _ = fs::remove_dir_all(&dir);
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
    let sandbox = match confine(&internal.read_only, &internal.read_write) {
        Ok(s) => s,
        Err(reason) => {
            let _ = writeln!(status, "unavailable landlock {reason}");
            exit(0);
        }
    };
    // Required like Landlock (INV-06): no filter, no tool.
    if let Err(e) = apply_filter(&socket_filter()) {
        let _ = writeln!(status, "unavailable seccomp {e}");
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

/// The seccomp filter (AET-WO-0300a, ruled on PR #15): a fixed program,
/// assembled here rather than by `seccompiler::SeccompFilter`, whose compiled
/// rules match system-call numbers under `AUDIT_ARCH_X86_64` and so would let
/// an x32 call (number | 0x4000_0000) through a default-allow filter.
///
/// In order: kill the process for any architecture but x86_64, and for any
/// x32 call; refuse `socket` (every family: no tool needs one, measured in the
/// spike) and the three `io_uring` calls with `EPERM`; allow everything else,
/// `socketpair` included, which reaches nothing outside the process.
pub fn socket_filter() -> BpfProgram {
    const LD_W_ABS: u16 = 0x20; // BPF_LD | BPF_W | BPF_ABS
    const JEQ_K: u16 = 0x15; // BPF_JMP | BPF_JEQ | BPF_K
    const JSET_K: u16 = 0x45; // BPF_JMP | BPF_JSET | BPF_K
    const RET_K: u16 = 0x06; // BPF_RET | BPF_K
    const AUDIT_ARCH_X86_64: u32 = 0xc000_003e;
    const X32_SYSCALL_BIT: u32 = 0x4000_0000;
    const OFFSET_NR: u32 = 0;
    const OFFSET_ARCH: u32 = 4;
    const NR_SOCKET: u32 = 41;
    const NR_IO_URING_SETUP: u32 = 425;
    const NR_IO_URING_ENTER: u32 = 426;
    const NR_IO_URING_REGISTER: u32 = 427;
    const RET_ALLOW: u32 = 0x7fff_0000;
    const RET_EPERM: u32 = 0x0005_0000 | 1;
    const RET_KILL_PROCESS: u32 = 0x8000_0000;
    let op = |code, k, jt, jf| sock_filter { code, jt, jf, k };
    // Jump offsets count instructions after the next one: 9 is EPERM, 10 kill.
    vec![
        op(LD_W_ABS, OFFSET_ARCH, 0, 0),       // 0
        op(JEQ_K, AUDIT_ARCH_X86_64, 0, 8),    // 1: else kill (10)
        op(LD_W_ABS, OFFSET_NR, 0, 0),         // 2
        op(JSET_K, X32_SYSCALL_BIT, 6, 0),     // 3: x32, kill (10)
        op(JEQ_K, NR_SOCKET, 4, 0),            // 4: EPERM (9)
        op(JEQ_K, NR_IO_URING_SETUP, 3, 0),    // 5: EPERM (9)
        op(JEQ_K, NR_IO_URING_ENTER, 2, 0),    // 6: EPERM (9)
        op(JEQ_K, NR_IO_URING_REGISTER, 1, 0), // 7: EPERM (9)
        op(RET_K, RET_ALLOW, 0, 0),            // 8
        op(RET_K, RET_EPERM, 0, 0),            // 9
        op(RET_K, RET_KILL_PROCESS, 0, 0),     // 10
    ]
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
    use super::*;

    /// The filter as an exact instruction sequence: any edit to it must show
    /// up here (ruled on PR #15). (code, jt, jf, k)
    #[test]
    fn socket_filter_is_exactly_this_program() {
        let got: Vec<(u16, u8, u8, u32)> = socket_filter()
            .iter()
            .map(|i| (i.code, i.jt, i.jf, i.k))
            .collect();
        let want = vec![
            (0x20, 0, 0, 4),           // load arch
            (0x15, 0, 8, 0xc000_003e), // x86_64, else kill
            (0x20, 0, 0, 0),           // load nr
            (0x45, 6, 0, 0x4000_0000), // x32 bit: kill
            (0x15, 4, 0, 41),          // socket: EPERM
            (0x15, 3, 0, 425),         // io_uring_setup: EPERM
            (0x15, 2, 0, 426),         // io_uring_enter: EPERM
            (0x15, 1, 0, 427),         // io_uring_register: EPERM
            (0x06, 0, 0, 0x7fff_0000), // allow
            (0x06, 0, 0, 0x0005_0001), // errno EPERM
            (0x06, 0, 0, 0x8000_0000), // kill process
        ];
        assert_eq!(got, want);
    }

    #[test]
    fn every_jump_lands_on_the_return_it_names() {
        let p = socket_filter();
        let kill = p.len() - 1;
        let eperm = p.len() - 2;
        assert_eq!(1 + 1 + usize::from(p[1].jf), kill);
        assert_eq!(3 + 1 + usize::from(p[3].jt), kill);
        for (i, op) in p.iter().enumerate().take(8).skip(4) {
            assert_eq!(i + 1 + usize::from(op.jt), eperm, "instruction {i}");
        }
    }

    #[test]
    fn an_unavailable_seccomp_line_is_sandbox_unavailable_naming_seccomp() {
        let stop = parse_status("unavailable seccomp Prctl(EINVAL)\n", "kicad-cli").unwrap_err();
        assert_eq!(
            (stop.code, stop.kind, stop.id.as_deref()),
            (4, "SandboxUnavailable", Some("seccomp"))
        );
    }

    #[test]
    fn an_unavailable_landlock_line_still_names_landlock() {
        let stop = parse_status("unavailable landlock NotImplemented\n", "kicad-cli").unwrap_err();
        assert_eq!(
            (stop.code, stop.kind, stop.id.as_deref()),
            (4, "SandboxUnavailable", Some("landlock"))
        );
    }

    #[test]
    fn a_verdict_reports_the_sockets_denied() {
        let (sandbox, _) = parse_status("sandbox abi=8\nended code=0\n", "kicad-cli").unwrap();
        assert!(sandbox.sockets_denied);
    }
}
