//! S1, S2, S5 probe: install the AET-WO-0300a filter on this process, then
//! become CMD. Safe Rust only: the BPF program is assembled from
//! `seccompiler::sock_filter` values and installed with
//! `seccompiler::apply_filter`.
//!
//!   seccomp-wrap [--allow-family N]... -- CMD ARGS...
//!
//! The filter, in order: kill the process for any architecture but x86_64,
//! and for any x32 system call (number with bit 0x40000000); refuse `socket`
//! with EPERM unless its family is allowed; refuse `io_uring_setup`,
//! `io_uring_enter` and `io_uring_register` with EPERM; allow everything else.

#![forbid(unsafe_code)]

use std::os::unix::process::CommandExt;
use std::process::{Command, exit};
use std::time::Instant;

use seccompiler::{BpfProgram, apply_filter, sock_filter};

const LD_W_ABS: u16 = 0x20; // BPF_LD | BPF_W | BPF_ABS
const JEQ_K: u16 = 0x15; // BPF_JMP | BPF_JEQ | BPF_K
const JSET_K: u16 = 0x45; // BPF_JMP | BPF_JSET | BPF_K
const RET_K: u16 = 0x06; // BPF_RET | BPF_K

const AUDIT_ARCH_X86_64: u32 = 0xc000_003e;
const X32_SYSCALL_BIT: u32 = 0x4000_0000;
const NR_SOCKET: u32 = 41;
const NR_IO_URING: [u32; 3] = [425, 426, 427];
const OFFSET_NR: u32 = 0;
const OFFSET_ARCH: u32 = 4;
const OFFSET_ARG0_LOW: u32 = 16; // args[0], low 32 bits on little-endian

const RET_ALLOW: u32 = 0x7fff_0000;
const RET_EPERM: u32 = 0x0005_0000 | 1;
const RET_KILL_PROCESS: u32 = 0x8000_0000;

fn stmt(code: u16, k: u32) -> sock_filter {
    sock_filter {
        code,
        jt: 0,
        jf: 0,
        k,
    }
}

fn jump(code: u16, k: u32, jt: u8, jf: u8) -> sock_filter {
    sock_filter { code, jt, jf, k }
}

/// The program. Jump offsets are relative to the next instruction.
pub fn program(allowed_families: &[u32]) -> BpfProgram {
    let n = allowed_families.len();
    // Fixed layout: 0..=8 dispatch, 9 load family, 10..10+n allowed
    // families, then four returns.
    let sock = 9;
    let deny_socket = 10 + n;
    let deny = deny_socket + 1;
    let kill = deny + 1;
    let allow_family = kill + 1;
    let rel = |from: usize, to: usize| -> u8 {
        u8::try_from(to - from - 1).expect("program fits in 255")
    };
    let mut p = vec![
        stmt(LD_W_ABS, OFFSET_ARCH),
        jump(JEQ_K, AUDIT_ARCH_X86_64, 0, rel(1, kill)),
        stmt(LD_W_ABS, OFFSET_NR),
        jump(JSET_K, X32_SYSCALL_BIT, rel(3, kill), 0),
        jump(JEQ_K, NR_SOCKET, rel(4, sock), 0),
        jump(JEQ_K, NR_IO_URING[0], rel(5, deny), 0),
        jump(JEQ_K, NR_IO_URING[1], rel(6, deny), 0),
        jump(JEQ_K, NR_IO_URING[2], rel(7, deny), 0),
        stmt(RET_K, RET_ALLOW),
        stmt(LD_W_ABS, OFFSET_ARG0_LOW),
    ];
    for (i, family) in allowed_families.iter().enumerate() {
        p.push(jump(JEQ_K, *family, rel(10 + i, allow_family), 0));
    }
    p.push(stmt(RET_K, RET_EPERM)); // deny_socket
    p.push(stmt(RET_K, RET_EPERM)); // deny
    p.push(stmt(RET_K, RET_KILL_PROCESS)); // kill
    p.push(stmt(RET_K, RET_ALLOW)); // allow_family
    debug_assert_eq!(p.len(), allow_family + 1);
    p
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(sep) = args.iter().position(|a| a == "--") else {
        eprintln!("usage: seccomp-wrap [--allow-family N]... -- CMD ARGS...");
        exit(64);
    };
    let mut allowed = Vec::new();
    let mut i = 1;
    while i < sep {
        match (
            args[i].as_str(),
            args.get(i + 1).and_then(|v| v.parse::<u32>().ok()),
        ) {
            ("--allow-family", Some(f)) => allowed.push(f),
            _ => {
                eprintln!("bad argument {}", args[i]);
                exit(64);
            }
        }
        i += 2;
    }
    let prog = program(&allowed);
    let start = Instant::now();
    if let Err(e) = apply_filter(&prog) {
        eprintln!("seccomp-wrap: install failed: {e}");
        exit(70);
    }
    eprintln!(
        "seccomp-wrap: installed {} instructions in {} us, allowed families {allowed:?}",
        prog.len(),
        start.elapsed().as_micros()
    );
    let err = Command::new(&args[sep + 1]).args(&args[sep + 2..]).exec();
    eprintln!("seccomp-wrap: exec: {err}");
    exit(127);
}
