//! What a tool said, for the `ToolError` that cannot recognise its outcome
//! (`AET-WO-1502`). The message ends with the tool's first error line, so a
//! caller sees the cause without re-running the tool outside the sandbox.
//! The line is tool output: it is cut (`RUL-13`) and made printable ASCII
//! before it reaches the JSON envelope or a terminal.

/// The longest quote, in bytes, before the `…`: 240 for Renode's one line, 400
/// for cargo's joined lines.
const MAX_ONE: usize = 240;
const MAX_JOINED: usize = 400;

/// Which lines are "the error" in a tool's output.
#[derive(Clone, Copy)]
pub enum Style {
    /// Cargo and Kani: every line that starts with `error:`, joined by ` | `.
    /// Cargo's cause is often not on the first one. Also a `panicked at` line
    /// (the tool itself crashed) and a line naming a signal (a child killed),
    /// the signal named where the line gives its number alone (`AET-WO-4300k`,
    /// ruled 21:38Z: a tool that crashes must say why, `INV-06`).
    Cargo,
    /// Renode: the first line that contains `[ERROR]`.
    Renode,
}

/// The signal number a line reports a process killed by: `signal: 9` (cargo
/// and Rust's exit status), `signal 9` (`killed by signal 9`), or a shell's
/// bare `Killed` (SIGKILL).
fn signal(line: &str) -> Option<u32> {
    if line == "Killed" {
        return Some(9);
    }
    let at = line
        .find("signal: ")
        .map(|i| i + 8)
        .or_else(|| line.find("killed by signal ").map(|i| i + 17))?;
    let digits: String = line[at..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// A signal's name, for the numbers Linux gives every architecture alike.
fn signal_name(n: u32) -> Option<&'static str> {
    Some(match n {
        1 => "SIGHUP",
        2 => "SIGINT",
        3 => "SIGQUIT",
        4 => "SIGILL",
        6 => "SIGABRT",
        8 => "SIGFPE",
        9 => "SIGKILL",
        11 => "SIGSEGV",
        13 => "SIGPIPE",
        15 => "SIGTERM",
        _ => return None,
    })
}

/// `line`, with its signal's name appended where it gives the number alone.
fn named(line: &str) -> String {
    match signal(line).and_then(|n| signal_name(n).map(|s| (n, s))) {
        Some((n, name)) if !line.contains(name) => format!("{line} (signal {n} is {name})"),
        _ => line.to_owned(),
    }
}

/// The error lines of `text`, each trimmed, the whole made printable ASCII
/// (every other byte becomes `?`) and cut to the style's cap with `…` appended
/// if it was cut.
pub fn first_error(text: &str, style: Style) -> Option<String> {
    let mut lines = text.lines().map(str::trim);
    let (joined, max) = match style {
        Style::Cargo => (
            lines
                .filter(|l| {
                    l.starts_with("error:") || l.contains("panicked at") || signal(l).is_some()
                })
                .map(named)
                .collect::<Vec<_>>()
                .join(" | "),
            MAX_JOINED,
        ),
        Style::Renode => (lines.find(|l| l.contains("[ERROR]"))?.to_owned(), MAX_ONE),
    };
    if joined.is_empty() {
        return None;
    }
    let mut out: String = joined
        .bytes()
        .take(max)
        .map(|b| {
            if (0x20..=0x7E).contains(&b) {
                b as char
            } else {
                '?'
            }
        })
        .collect();
    if joined.len() > max {
        out.push('…');
    }
    Some(out)
}

/// `"; the tool said: <line>"`, or nothing when the tool printed no error line.
pub fn suffix(text: &str, style: Style) -> String {
    first_error(text, style).map_or_else(String::new, |l| format!("; the tool said: {l}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A planted stand-in that panics, and a child killed by a signal, each
    /// quoted by name: the panic's own line, and the signal's number and name
    /// (`AET-WO-4300k`, ruled 21:38Z).
    #[test]
    fn a_panic_and_a_signal_killed_child_are_quoted_by_name() {
        use std::process::Command;
        // A real stand-in: a shell that prints a Rust panic's line and exits
        // 101, as cargo-kani does when it panics.
        let out = Command::new("sh")
            .args([
                "-c",
                "echo \"thread 'main' panicked at kani-driver/src/main.rs:12:5:\" >&2; echo 'called Result::unwrap()' >&2; exit 101",
            ])
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(101));
        let text = String::from_utf8(out.stderr).unwrap();
        assert_eq!(
            suffix(&text, Style::Cargo),
            "; the tool said: thread 'main' panicked at kani-driver/src/main.rs:12:5:"
        );
        // A real child killed by SIGKILL, reported the way cargo reports a
        // child that did not exit (the status's own `Display`).
        let child = Command::new("sh")
            .args(["-c", "kill -9 $$"])
            .status()
            .unwrap();
        assert_eq!(child.code(), None);
        let text = format!("error: process didn't exit successfully: `cbmc` ({child})\n");
        let said = suffix(&text, Style::Cargo);
        assert!(
            said.contains("signal: 9") && said.contains("SIGKILL"),
            "{said}"
        );
        // A child named only by its number gets its name; a shell's bare
        // `Killed` is SIGKILL.
        assert_eq!(
            first_error("cbmc was killed by signal 9\n", Style::Cargo).as_deref(),
            Some("cbmc was killed by signal 9 (signal 9 is SIGKILL)")
        );
        assert_eq!(
            first_error("noise\nKilled\n", Style::Cargo).as_deref(),
            Some("Killed (signal 9 is SIGKILL)")
        );
        assert_eq!(
            first_error("signal: 64\n", Style::Cargo).as_deref(),
            Some("signal: 64")
        );
    }

    #[test]
    fn no_error_line_gives_nothing() {
        assert_eq!(first_error("ok\nwarning: x\n", Style::Cargo), None);
        assert_eq!(first_error("[WARNING] x\n", Style::Renode), None);
        assert_eq!(suffix("", Style::Cargo), "");
    }

    #[test]
    fn cargo_joins_every_error_line_and_renode_takes_the_first() {
        let t = "noise\n   error: first  \nwarning: x\nerror: second\n";
        assert_eq!(
            first_error(t, Style::Cargo).as_deref(),
            Some("error: first | error: second")
        );
        assert_eq!(
            suffix(t, Style::Cargo),
            "; the tool said: error: first | error: second"
        );
        let r = "a\n12:00 [ERROR] one\n[ERROR] two\n";
        assert_eq!(
            first_error(r, Style::Renode).as_deref(),
            Some("12:00 [ERROR] one")
        );
    }

    #[test]
    fn cargo_joins_three_lines_in_order() {
        let t = "error: a\nerror: b\nnote\n  error: c\n";
        assert_eq!(
            first_error(t, Style::Cargo).as_deref(),
            Some("error: a | error: b | error: c")
        );
    }

    #[test]
    fn cargo_style_needs_the_prefix_and_renode_style_the_substring() {
        assert_eq!(first_error("x error: y\n", Style::Cargo), None);
        assert_eq!(first_error("error[E0277]: y\n", Style::Cargo), None);
        assert!(first_error("x [ERROR] y\n", Style::Renode).is_some());
    }

    #[test]
    fn renode_cap_is_240_bytes_exactly() {
        let at = format!("[ERROR] {}", "a".repeat(MAX_ONE - 8));
        assert_eq!(at.len(), 240);
        assert_eq!(
            first_error(&at, Style::Renode).as_deref(),
            Some(at.as_str())
        );
        assert_eq!(
            first_error(&format!("{at}b"), Style::Renode).unwrap(),
            format!("{at}…")
        );
    }

    #[test]
    fn cargo_cap_is_400_bytes_on_the_joined_text() {
        let one = format!("error: {}", "a".repeat(193)); // 200 bytes
        let two = format!("error: {}", "b".repeat(193 - 3)); // 197; joined 200 + 3 + 197 = 400
        let at = format!("{one}\n{two}\n");
        let got = first_error(&at, Style::Cargo).unwrap();
        assert_eq!(got.len(), 400);
        assert!(!got.ends_with('…'));
        let over = format!("{one}\n{two}b\n");
        let cut = first_error(&over, Style::Cargo).unwrap();
        assert_eq!(cut, format!("{}…", got));
    }

    #[test]
    fn non_ascii_and_control_bytes_become_question_marks() {
        let t = "error: caf\u{e9} \u{1b}[31mred\u{7}\tx\u{7f}\n";
        assert_eq!(
            first_error(t, Style::Cargo).as_deref(),
            Some("error: caf?? ?[31mred??x?")
        );
    }

    #[test]
    fn a_multibyte_char_straddling_the_cut_is_safe() {
        let t = format!("[ERROR] {}\u{e9}\u{e9}", "a".repeat(MAX_ONE - 8 - 1));
        let cut = first_error(&t, Style::Renode).unwrap();
        assert_eq!(cut, format!("[ERROR] {}?…", "a".repeat(MAX_ONE - 9)));
    }

    #[test]
    fn crlf_endings_leave_no_carriage_return() {
        let t = "a\r\nerror: boom\r\nb\r\n";
        assert_eq!(first_error(t, Style::Cargo).as_deref(), Some("error: boom"));
        // A lone CR inside a line is a control byte.
        assert_eq!(
            first_error("error: a\rb\n", Style::Cargo).as_deref(),
            Some("error: a?b")
        );
    }

    #[test]
    fn a_json_hostile_line_is_printable_ascii_only() {
        let q = first_error("error: \"\\\n", Style::Cargo).unwrap();
        assert!(q.bytes().all(|b| (0x20..=0x7E).contains(&b)));
    }
}
