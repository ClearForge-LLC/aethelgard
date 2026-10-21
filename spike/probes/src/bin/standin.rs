//! S3, S4, S5 probe: the stand-in "tool". Every attempt prints one line
//! `ANSWER <mode> <name>=<result>`, where the result is `ok` or the exact errno.
//!
//!   standin attack BUS TMUX ABSTRACT_NAME TCP_PORT   the five S4 attempts + io_uring
//!   standin x32                                      socket() through the x32 ABI
//!   standin i386                                     socket() through int 0x80 (i386)
//!   standin listen ABSTRACT_NAME TCP_PORT SECONDS    targets for the attempts, run outside
//!
//! Raw system calls need `unsafe`; this crate is outside the workspace.

use std::io::{self, Read};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixListener, UnixStream};
use std::time::Duration;

fn mode() -> String {
    std::env::var("PROBE_MODE").unwrap_or_else(|_| "bare".into())
}

fn errno_name(n: i32) -> &'static str {
    match n {
        1 => "EPERM",
        2 => "ENOENT",
        13 => "EACCES",
        38 => "ENOSYS",
        93 => "EPROTONOSUPPORT",
        97 => "EAFNOSUPPORT",
        111 => "ECONNREFUSED",
        _ => "other",
    }
}

fn report<T>(name: &str, r: io::Result<T>) {
    let result = match r {
        Ok(_) => "ok".to_owned(),
        Err(e) => match e.raw_os_error() {
            Some(n) => format!("errno={n}({})", errno_name(n)),
            None => format!("error={e}"),
        },
    };
    println!("ANSWER {} {name}={result}", mode());
}

fn raw(ret: libc::c_long) -> io::Result<libc::c_long> {
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ret)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("attack") => {
            let (bus, tmux, name, port) = (&args[2], &args[3], &args[4], &args[5]);
            report("unix_path_session_bus", UnixStream::connect(bus));
            report("unix_path_tmux", UnixStream::connect(tmux));
            report(
                "unix_abstract",
                SocketAddr::from_abstract_name(name.as_bytes())
                    .and_then(|a| UnixStream::connect_addr(&a)),
            );
            report(
                "udp_sendto_127_0_0_1",
                UdpSocket::bind("127.0.0.1:0").and_then(|s| s.send_to(b"x", "127.0.0.1:9")),
            );
            report(
                "tcp_connect_127_0_0_1",
                TcpStream::connect(("127.0.0.1", port.parse::<u16>().unwrap())),
            );
            report("socketpair", UnixStream::pair());
            let mut params = [0u8; 120]; // struct io_uring_params
            // SAFETY: io_uring_setup(entries, params) with a writable 120-byte buffer.
            let r = raw(unsafe { libc::syscall(libc::SYS_io_uring_setup, 1, params.as_mut_ptr()) });
            report("io_uring_setup", r);
        }
        Some("x32") => {
            // SAFETY: socket(AF_UNIX, SOCK_STREAM, 0) through the x32 syscall number.
            let r = raw(unsafe { libc::syscall(0x4000_0000 | libc::SYS_socket, 1, 1, 0) });
            report("x32_socket", r);
        }
        Some("i386") => {
            let ret: i32;
            // SAFETY: i386 socket(AF_UNIX=1, SOCK_STREAM=1, 0) is system call 359 via int 0x80.
            unsafe {
                // rbx is reserved by LLVM, so the first argument goes in through
                // another register and is swapped into ebx around the call.
                std::arch::asm!(
                    "xchg {arg1:e}, ebx",
                    "int 0x80",
                    "xchg {arg1:e}, ebx",
                    arg1 = inout(reg) 1i32 => _,
                    inlateout("eax") 359i32 => ret,
                    in("ecx") 1,
                    in("edx") 0,
                );
            }
            let r = if ret < 0 {
                Err(io::Error::from_raw_os_error(-ret))
            } else {
                Ok(ret)
            };
            report("i386_socket", r);
        }
        Some("listen") => {
            let name = &args[2];
            let port: u16 = args[3].parse().unwrap();
            let secs: u64 = args[4].parse().unwrap();
            let addr = SocketAddr::from_abstract_name(name.as_bytes()).unwrap();
            let unix = UnixListener::bind_addr(&addr).unwrap();
            let tcp = TcpListener::bind(("127.0.0.1", port)).unwrap();
            unix.set_nonblocking(true).unwrap();
            tcp.set_nonblocking(true).unwrap();
            eprintln!("listening on abstract {name} and 127.0.0.1:{port} for {secs} s");
            let until = std::time::Instant::now() + Duration::from_secs(secs);
            while std::time::Instant::now() < until {
                if let Ok((mut s, _)) = unix.accept() {
                    let _ = s.read(&mut [0u8; 1]);
                }
                let _ = tcp.accept();
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        _ => {
            eprintln!("usage: see the source");
            std::process::exit(64);
        }
    }
}
