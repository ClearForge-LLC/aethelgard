//! An executable stub, written by a child process (`AET-WO-1603` §1.6, `AET-WO-4220` F15).
//!
//! A test that writes a stub with `fs::write` holds a descriptor open for writing on it, and a
//! sibling test thread that forks in that window gives its child a copy, which lives until the
//! child execs. An `execve` of the stub meanwhile fails with `ETXTBSY`. The kernel counts the
//! inode's writers, so writing elsewhere and renaming does not help (measured: 78 in 2,000).
//! Here a `/bin/sh` child writes and closes the file, and exits before this returns: this test
//! process never holds the descriptor, so no fork of it can (measured: 0 in 6,000). Nothing
//! retries on `ETXTBSY`.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Writes `body` to `path`, mode 0755, from a child process.
pub fn write_executable(path: &Path, body: impl AsRef<[u8]>) {
    let mut child = Command::new("/bin/sh")
        .args(["-c", "cat > \"$1\" && chmod 755 \"$1\"", "sh"])
        .arg(path)
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(body.as_ref())
        .unwrap();
    assert!(
        child.wait().unwrap().success(),
        "writing {}",
        path.display()
    );
}

/// Copies the executable `from` to `to`, mode 0755, from a child process, for
/// the same reason.
#[allow(dead_code)]
pub fn copy_executable(from: &Path, to: &Path) {
    let status = Command::new("/bin/sh")
        .args(["-c", "cat < \"$1\" > \"$2\" && chmod 755 \"$2\"", "sh"])
        .arg(from)
        .arg(to)
        .stdin(Stdio::null())
        .status()
        .unwrap();
    assert!(
        status.success(),
        "copying {} to {}",
        from.display(),
        to.display()
    );
}
