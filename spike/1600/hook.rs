//! AET-WO-1600 spike seam, compiled into the CLI only with `--features
//! with-tools` (the `tools` CI job). The build deletes it.
//!
//! If `AETHELGARD_SPIKE_1600` names a variant, the trampoline installs that
//! variant's program as a SECOND seccomp filter, after the pinned socket
//! filter, and removes the variable before it becomes the tool. Seccomp
//! evaluates every installed filter and keeps the most restrictive action, so a
//! variant can only add refusals; it never weakens the sandbox. An unknown
//! variant refuses the call (`unavailable seccomp`), so a typo can never pass
//! as a measurement of the baseline.

use std::env;
use std::process::Command;

use seccompiler::{BpfProgram, sock_filter};

pub const VAR: &str = "AETHELGARD_SPIKE_1600";

/// The CLI clears the reaper's environment; this carries the variant across.
pub fn forward(cmd: &mut Command) {
    if let Some(v) = env::var_os(VAR) {
        cmd.env(VAR, v);
    }
}

/// Installs the named variant on this thread, if one is named.
pub fn install() -> Result<(), String> {
    let Some(raw) = env::var_os(VAR) else {
        return Ok(());
    };
    let name = raw.to_str().ok_or("variant is not UTF-8")?;
    let program = program(name).ok_or_else(|| format!("unknown spike variant {name:?}"))?;
    seccompiler::apply_filter(&program).map_err(|e| e.to_string())
}

pub const EPERM: u32 = 1;
pub const ENOSYS: u32 = 38;

/// System V IPC (x86_64 numbers).
pub const SYSV: &[(&str, u32)] = &[
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
];
/// Namespaces; `clone` is refused only with a `CLONE_NEW*` bit (see below).
pub const NS: &[(&str, u32)] = &[("unshare", 272), ("setns", 308)];
/// pidfds and cross-process access.
pub const PIDFD: &[(&str, u32)] = &[
    ("pidfd_open", 434),
    ("pidfd_getfd", 438),
    ("pidfd_send_signal", 424),
    ("ptrace", 101),
    ("process_vm_readv", 310),
    ("process_vm_writev", 311),
    ("kcmp", 312),
];
/// Other kernel surfaces.
pub const KERNEL: &[(&str, u32)] = &[
    ("keyctl", 250),
    ("add_key", 248),
    ("request_key", 249),
    ("bpf", 321),
    ("perf_event_open", 298),
    ("userfaultfd", 323),
];
pub const NR_CLONE: u32 = 56;
pub const NR_CLONE3: u32 = 435;
/// Every `CLONE_NEW*` bit `clone(2)` can carry in its low 32-bit flags word:
/// NEWNS 0x20000, NEWCGROUP 0x2000000, NEWUTS 0x4000000, NEWIPC 0x8000000,
/// NEWUSER 0x10000000, NEWPID 0x20000000, NEWNET 0x40000000. CLONE_NEWTIME
/// (0x80) lies inside CSIGNAL for `clone`, which the kernel strips, so only
/// `clone3` and `unshare` can carry it.
pub const CLONE_NEW_MASK: u32 = 0x7e02_0000;

/// What a variant refuses: the numbers refused with EPERM, whether `clone`
/// with a namespace bit is refused, and what `clone3` returns, if refused.
pub struct Spec {
    pub eperm: Vec<u32>,
    pub clone_new: bool,
    pub clone3: Option<u32>,
}

pub fn spec(variant: &str) -> Option<Spec> {
    let nrs = |sets: &[&[(&str, u32)]]| -> Vec<u32> {
        sets.iter()
            .flat_map(|s| s.iter().map(|&(_, n)| n))
            .collect()
    };
    let s1 = || nrs(&[SYSV, NS, PIDFD, KERNEL]);
    let (eperm, clone_new, clone3) = match variant {
        "allow" => (Vec::new(), false, None),
        "sysv" => (nrs(&[SYSV]), false, None),
        "ns" => (nrs(&[NS]), true, None),
        "pidfd" => (nrs(&[PIDFD]), false, None),
        "kernel" => (nrs(&[KERNEL]), false, None),
        "s1" => (s1(), true, None),
        "s2-enosys" => (s1(), true, Some(ENOSYS)),
        "s2-eperm" => (s1(), true, Some(EPERM)),
        "clone3-enosys" => (Vec::new(), false, Some(ENOSYS)),
        "clone3-eperm" => (Vec::new(), false, Some(EPERM)),
        _ => return None,
    };
    Some(Spec {
        eperm,
        clone_new,
        clone3,
    })
}

/// The variant's program. Same opcodes and layout rules as `socket_filter`:
/// arch first (anything but x86_64 is killed), then the x32 bit (killed),
/// then one `jeq` per refused number, then `clone`'s flags.
pub fn program(variant: &str) -> Option<BpfProgram> {
    spec(variant).map(|s| assemble(&s))
}

pub fn assemble(s: &Spec) -> BpfProgram {
    const LD_W_ABS: u16 = 0x20;
    const JEQ_K: u16 = 0x15;
    const JSET_K: u16 = 0x45;
    const RET_K: u16 = 0x06;
    const AUDIT_ARCH_X86_64: u32 = 0xc000_003e;
    const X32_SYSCALL_BIT: u32 = 0x4000_0000;
    const RET_ALLOW: u32 = 0x7fff_0000;
    const RET_ERRNO: u32 = 0x0005_0000;
    const RET_KILL_PROCESS: u32 = 0x8000_0000;
    // Instructions before the return block, so every jump is computable:
    // 4 header, one per number, 1 for clone3, 3 for clone's flags.
    let body =
        4 + s.eperm.len() + usize::from(s.clone3.is_some()) + if s.clone_new { 3 } else { 0 };
    // ALLOW first: the last rule falls through to it. (The first draft put
    // EPERM first, so a variant with no rules refused everything; the
    // interpreter test caught it.)
    let (allow, eperm, enosys, kill) = (body, body + 1, body + 2, body + 3);
    let mut p: Vec<sock_filter> = Vec::new();
    let mut emit = |code: u16, k: u32, jt: usize, jf: usize, at: usize| {
        // `jt`/`jf` are absolute targets (0 means fall through).
        let off = |t: usize| {
            if t == 0 {
                0
            } else {
                u8::try_from(t - at - 1).expect("jump fits")
            }
        };
        p.push(sock_filter {
            code,
            jt: off(jt),
            jf: off(jf),
            k,
        });
    };
    let mut at = 0;
    emit(LD_W_ABS, 4, 0, 0, at);
    at += 1;
    emit(JEQ_K, AUDIT_ARCH_X86_64, 0, kill, at);
    at += 1;
    emit(LD_W_ABS, 0, 0, 0, at);
    at += 1;
    emit(JSET_K, X32_SYSCALL_BIT, kill, 0, at);
    at += 1;
    for &nr in &s.eperm {
        emit(JEQ_K, nr, eperm, 0, at);
        at += 1;
    }
    if let Some(errno) = s.clone3 {
        emit(
            JEQ_K,
            NR_CLONE3,
            if errno == ENOSYS { enosys } else { eperm },
            0,
            at,
        );
        at += 1;
    }
    if s.clone_new {
        emit(JEQ_K, NR_CLONE, 0, allow, at);
        at += 1;
        emit(LD_W_ABS, 16, 0, 0, at); // clone's flags, low 32 bits
        at += 1;
        emit(JSET_K, CLONE_NEW_MASK, eperm, allow, at);
        at += 1;
    }
    assert_eq!(at, body);
    emit(RET_K, RET_ALLOW, 0, 0, at);
    emit(RET_K, RET_ERRNO | EPERM, 0, 0, at + 1);
    emit(RET_K, RET_ERRNO | ENOSYS, 0, 0, at + 2);
    emit(RET_K, RET_KILL_PROCESS, 0, 0, at + 3);
    p
}
