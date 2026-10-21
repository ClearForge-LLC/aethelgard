//! S4 probe: do a tool's processes outlive the program that started them?
//!
//!   spawn-tree bare      -- CMD...   start CMD and wait on it
//!   spawn-tree pdeathsig -- CMD...   start a trampoline that asks for SIGKILL
//!                                     when its parent dies, then becomes CMD
//!   spawn-tree reaper    -- CMD...   start a reaper that puts CMD in its own
//!                                     process group and kills the whole group
//!                                     when the reaper's parent dies
//!
//! Every process inherits this one's stdin, so the caller holds it open (a
//! FIFO) independently of the process it kills: Renode quits at stdin EOF, and
//! a pipe owned by this process would close when it dies, faking a kill.
//! Prints `pid <role> <n>` lines; the caller kills this process with SIGTERM
//! or SIGKILL and looks for survivors in /proc.

#![forbid(unsafe_code)]

use std::os::unix::process::{CommandExt, parent_id};
use std::process::{Command, exit};
use std::thread::sleep;
use std::time::Duration;

use rustix::process::{Pid, Signal, kill_process_group, set_parent_process_death_signal};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(sep) = args.iter().position(|a| a == "--") else {
        eprintln!("usage: spawn-tree bare|pdeathsig|reaper -- CMD...");
        exit(64);
    };
    let mode = args[1].as_str();
    let cmd = &args[sep + 1..];
    let me = std::env::current_exe().expect("current_exe");
    match mode {
        // Roles a parent invokes; never typed by the caller.
        "_trampoline" => {
            let parent = parent_id();
            set_parent_process_death_signal(Some(Signal::KILL)).expect("prctl");
            if parent_id() != parent {
                exit(70); // the parent died before the request took effect
            }
            let err = tool(cmd).exec();
            eprintln!("exec: {err}");
            exit(70);
        }
        "_reaper" => {
            let parent = parent_id();
            let mut child = tool(cmd).process_group(0).spawn().expect("spawn tool");
            let pgid = Pid::from_raw(child.id() as i32).expect("pid");
            println!("pid tool {}", child.id());
            loop {
                if let Some(status) = child.try_wait().expect("try_wait") {
                    exit(status.code().unwrap_or(70));
                }
                if parent_id() != parent {
                    let _ = kill_process_group(pgid, Signal::KILL);
                    exit(70);
                }
                sleep(Duration::from_millis(50));
            }
        }
        "bare" | "pdeathsig" | "reaper" => {}
        _ => {
            eprintln!("unknown mode {mode}");
            exit(64);
        }
    }
    println!("pid probe {}", std::process::id());
    let mut child = match mode {
        "bare" => tool(cmd).spawn(),
        "pdeathsig" => Command::new(&me)
            .arg("_trampoline")
            .arg("--")
            .args(cmd)
            .spawn(),
        _ => Command::new(&me).arg("_reaper").arg("--").args(cmd).spawn(),
    }
    .expect("spawn");
    println!("pid child {}", child.id());
    let status = child.wait().expect("wait");
    println!("child exited {status}");
}

fn tool(cmd: &[String]) -> Command {
    let mut c = Command::new(&cmd[0]);
    c.args(&cmd[1..]);
    c
}
