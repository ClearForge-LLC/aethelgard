//! A minimal MCP client over a child's stdio, for the `aethelgard mcp` tests (`AET-WO-1500`).
//! Each test file uses a different part of it.
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

pub const WAIT: Duration = Duration::from_secs(60);

pub fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

pub struct Client {
    pub child: Child,
    pub stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    next_id: u64,
}

impl Client {
    pub fn spawn(configure: impl FnOnce(&mut Command)) -> Client {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_aethelgard"));
        cmd.arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure(&mut cmd);
        let mut child = cmd.spawn().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let stdin = child.stdin.take();
        Client {
            child,
            stdin,
            lines,
            next_id: 1,
        }
    }

    pub fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        writeln!(stdin, "{message}").unwrap();
        stdin.flush().unwrap();
    }

    /// The next message from the server, or `None` on timeout or end of output.
    pub fn recv(&self, timeout: Duration) -> Option<Value> {
        match self.lines.recv_timeout(timeout) {
            Ok(line) => Some(serde_json::from_str(&line).unwrap_or_else(|e| {
                panic!("stdout carried a line that is not JSON ({e}): {line:?}")
            })),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => None,
        }
    }

    /// Sends a request and returns the response with its id; notifications from the server in
    /// between are skipped.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.start(method, params);
        self.response(id)
    }

    pub fn start(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        id
    }

    pub fn response(&self, id: u64) -> Value {
        let deadline = Instant::now() + WAIT;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match self.recv(left) {
                Some(m) if m["id"] == id => return m,
                Some(_) => {}
                None => break,
            }
        }
        panic!("no response to request {id}");
    }

    pub fn initialize(&mut self) -> Value {
        let r = self.request(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {},
                   "clientInfo": {"name": "test", "version": "0"}}),
        );
        self.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        r
    }

    pub fn call(&mut self, tool: &str, arguments: Value) -> Value {
        self.request("tools/call", json!({"name": tool, "arguments": arguments}))
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Every process that descends from `root` right now, read from `/proc`.
pub fn descendants(root: u32) -> Vec<u32> {
    let mut parent_of = Vec::new();
    for entry in std::fs::read_dir("/proc").unwrap().flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else {
            continue;
        };
        // "pid (comm) state ppid ...": comm may hold spaces and parentheses.
        let Some(rest) = stat.rsplit_once(')').map(|(_, r)| r) else {
            continue;
        };
        if let Some(ppid) = rest
            .split_whitespace()
            .nth(1)
            .and_then(|p| p.parse::<u32>().ok())
        {
            parent_of.push((pid, ppid));
        }
    }
    let mut found = vec![root];
    let mut i = 0;
    while i < found.len() {
        let p = found[i];
        for &(pid, ppid) in &parent_of {
            if ppid == p && !found.contains(&pid) {
                found.push(pid);
            }
        }
        i += 1;
    }
    found.remove(0);
    found
}

/// The fields of `/proc/<pid>/stat` from field 3 (the state) on, so field `n` is at `n - 3`;
/// `None` once the process is gone. The command name may hold spaces and parentheses.
pub fn stat_fields(pid: u32) -> Option<Vec<String>> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let rest = stat.rsplit_once(')')?.1;
    Some(rest.split_whitespace().map(str::to_owned).collect())
}

pub fn alive(pid: u32) -> bool {
    // A zombie is dead for this purpose: its state is Z.
    stat_fields(pid).is_some_and(|f| f[0] != "Z")
}

/// A test's own directory under the system temporary directory, removed on drop.
pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new(label: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("aet-test-{}-{label}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Scratch(std::fs::canonicalize(p).unwrap())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A directory the CLI accepts as `TMPDIR` (`AET-WO-1601` §1.4), outside
/// `$HOME` (`AET-WO-1607` (2), `1603` F2): cargo reads `~/.cargo/config.toml`
/// in every ancestor of a crate it builds, so `build` and `prove` refuse a
/// scratch directory under a home that holds one (`CargoConfigRefused`).
/// As root, `/var/lib/aethelgard`, searchable by the tool uid; otherwise the
/// user's runtime directory `/run/user/<euid>`, derived from the uid and
/// never from an environment variable. Each has only root's and this user's
/// directories above it, so no other user can write any of its ancestors.
/// Panics, naming why, if the base is missing or resolves under `$HOME`.
/// Removed on drop.
pub fn private_tmp(label: &str) -> Scratch {
    use std::os::unix::fs::DirBuilderExt;
    let euid = rustix::process::geteuid();
    let (base, mode) = if euid.is_root() {
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o711)
            .create("/var/lib/aethelgard")
            .unwrap();
        (PathBuf::from("/var/lib/aethelgard"), 0o711)
    } else {
        let run = PathBuf::from(format!("/run/user/{}", euid.as_raw()));
        let base = run.join("aethelgard-tests");
        if let Err(e) = std::fs::DirBuilder::new().mode(0o700).create(&base)
            && e.kind() != std::io::ErrorKind::AlreadyExists
        {
            panic!(
                "no private test scratch outside $HOME: cannot create {} ({e}); \
                 the tier-1 tests need the user's runtime directory {}",
                base.display(),
                run.display()
            );
        }
        (base, 0o700)
    };
    let base = std::fs::canonicalize(&base).unwrap();
    // A home of `/` holds every path, so nothing could be outside it.
    if let Some(home) = std::env::var_os("HOME")
        .and_then(|h| std::fs::canonicalize(h).ok())
        .filter(|h| h.parent().is_some())
    {
        assert!(
            !base.starts_with(&home),
            "test scratch {} resolves under $HOME {}",
            base.display(),
            home.display()
        );
    }
    let p = base.join(format!("tests-{}-{label}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::DirBuilder::new().mode(mode).create(&p).unwrap();
    Scratch(p)
}

/// The CLI's euid is 0 and its uid map is the initial namespace's identity.
pub fn initial_root() -> bool {
    rustix::process::geteuid().is_root()
        && std::fs::read_to_string("/proc/self/uid_map")
            .is_ok_and(|m| m.split_whitespace().collect::<Vec<_>>() == ["0", "0", "4294967295"])
}

/// The directories a tool found on `PATH` adds to its read grant: where it
/// was found and, through any link, where it lives.
pub fn tool_dirs(name: &str) -> Vec<PathBuf> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let found = std::env::split_paths(&path)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
        .unwrap_or_else(|| panic!("{name} is not on PATH"));
    let real = std::fs::canonicalize(&found).unwrap();
    vec![
        found.parent().unwrap().to_path_buf(),
        real.parent().unwrap().to_path_buf(),
    ]
}

/// What `sandbox.uid_boundary` and `uid_boundary_unmet` must say for a tool
/// whose read grant adds `paths` (ruled on PR #96), worked out apart from the
/// CLI: `find` beneath each path, and each ancestor's owner and mode.
pub fn expected_boundary(paths: &[PathBuf]) -> (bool, Option<&'static str>) {
    use std::os::unix::fs::MetadataExt;
    if !rustix::process::geteuid().is_root() {
        return (false, Some("tool_uid"));
    }
    if !initial_root() {
        return (false, Some("user_namespace"));
    }
    let clean = |p: &PathBuf| {
        let real = std::fs::canonicalize(p).unwrap();
        let system = [
            "/usr", "/lib", "/lib64", "/bin", "/sbin", "/etc", "/proc", "/dev",
        ];
        if system.iter().any(|s| real.starts_with(s)) {
            return true;
        }
        let ancestors = real.ancestors().skip(1).all(|a| {
            let m = std::fs::metadata(a).unwrap();
            let writable = m.mode() & 0o002 != 0 || (m.mode() & 0o020 != 0 && m.gid() != 0);
            m.uid() == 0 && (!writable || m.mode() & 0o1000 != 0)
        });
        let out = Command::new("find")
            .arg(&real)
            .args([
                "!", "-type", "l", "(", "!", "-uid", "0", "-o", "-perm", "-002", "-o", "(",
                "-perm", "-020", "!", "-gid", "0", ")", ")",
            ])
            .args(["-print", "-quit"])
            .output()
            .unwrap();
        ancestors && out.status.success() && out.stdout.is_empty()
    };
    if paths.iter().all(clean) {
        (true, None)
    } else {
        (false, Some("toolchain"))
    }
}

/// Polls until the condition holds or a deadline passes.
pub fn eventually(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if cond() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("timed out waiting for: {what}");
}

pub mod corpus;
pub mod edges;
pub mod foreign;
pub mod power;
pub mod rename;
pub mod stub;
pub mod th;
pub mod verify;

/// The identity line the bridge built from this tree prints first in a run
/// that started the pinned libngspice (`AET-WO-2200` F15, `bridge/build.rs`,
/// `AET-WO-1606`), so a stub bridge can be the one the CLI expects.
pub fn bridge_identity() -> String {
    format!(
        r#"{{"bridge":"aethelgard-bridge","ngspice":"44.2","ngspice_banner":"** ngspice-44.2 shared library","source_sha256":"{}","version":"0.1.0"}}"#,
        bridge_digest()
    )
}

/// The identity line of a run that started no library: a refusal, or
/// `--renode`.
pub fn bare_bridge_identity() -> String {
    format!(
        r#"{{"bridge":"aethelgard-bridge","source_sha256":"{}","version":"0.1.0"}}"#,
        bridge_digest()
    )
}

/// The digest of the bridge's sources: the recipe of `bridge/build.rs`,
/// recomputed here from the files on disk.
fn bridge_digest() -> String {
    use sha2::{Digest, Sha256};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bridge");
    let mut files = vec![
        "Cargo.toml".to_owned(),
        "Cargo.lock".to_owned(),
        "build.rs".to_owned(),
    ];
    let mut src: Vec<String> = std::fs::read_dir(root.join("src"))
        .unwrap()
        .map(|e| format!("src/{}", e.unwrap().file_name().to_str().unwrap()))
        .collect();
    src.sort();
    files.extend(src);
    let mut h = Sha256::new();
    for f in &files {
        let bytes = std::fs::read(root.join(f)).unwrap();
        h.update(f.as_bytes());
        h.update([0u8]);
        h.update((bytes.len() as u64).to_le_bytes());
        h.update(&bytes);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// A stub bridge's body with the identity line printed first, after its
/// `#!` line, so every stub is the bridge this CLI was built against.
pub fn identified_stub(body: &str) -> String {
    let (shebang, rest) = body.split_once('\n').unwrap_or((body, ""));
    format!("{shebang}\necho '{}'\n{rest}", bridge_identity())
}
