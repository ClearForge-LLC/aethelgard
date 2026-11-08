//! What a tool said, for the `ToolError` that cannot recognise its outcome
//! (`AET-WO-1502`). The message ends with the tool's first error line, so a
//! caller sees the cause without re-running the tool outside the sandbox.
//! The line is tool output: it is cut (`RUL-13`) and made printable ASCII
//! before it reaches the JSON envelope or a terminal.

/// The longest quoted line, in bytes, before the `…`.
const MAX: usize = 240;

/// Which line is "the error" in a tool's output.
#[derive(Clone, Copy)]
pub enum Style {
    /// Cargo and Kani: a line that starts with `error:`.
    Cargo,
    /// Renode: a line that contains `[ERROR]`.
    Renode,
}

/// The first error line of `text`, trimmed, made printable ASCII (every other
/// byte becomes `?`) and cut to 240 bytes with `…` appended if it was cut.
pub fn first_error(text: &str, style: Style) -> Option<String> {
    let line = text.lines().map(str::trim).find(|l| match style {
        Style::Cargo => l.starts_with("error:"),
        Style::Renode => l.contains("[ERROR]"),
    })?;
    let mut out: String = line
        .bytes()
        .take(MAX)
        .map(|b| {
            if (0x20..=0x7E).contains(&b) {
                b as char
            } else {
                '?'
            }
        })
        .collect();
    if line.len() > MAX {
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

    #[test]
    fn no_error_line_gives_nothing() {
        assert_eq!(first_error("ok\nwarning: x\n", Style::Cargo), None);
        assert_eq!(first_error("[WARNING] x\n", Style::Renode), None);
        assert_eq!(suffix("", Style::Cargo), "");
    }

    #[test]
    fn the_first_of_several_wins_and_is_trimmed() {
        let t = "noise\n   error: first  \nerror: second\n";
        assert_eq!(
            first_error(t, Style::Cargo).as_deref(),
            Some("error: first")
        );
        let r = "a\n12:00 [ERROR] one\n[ERROR] two\n";
        assert_eq!(
            first_error(r, Style::Renode).as_deref(),
            Some("12:00 [ERROR] one")
        );
        assert_eq!(suffix(t, Style::Cargo), "; the tool said: error: first");
    }

    #[test]
    fn cargo_style_needs_the_prefix_and_renode_style_the_substring() {
        assert_eq!(first_error("x error: y\n", Style::Cargo), None);
        assert_eq!(first_error("error[E0277]: y\n", Style::Cargo), None);
        assert!(first_error("x [ERROR] y\n", Style::Renode).is_some());
    }

    #[test]
    fn exactly_240_bytes_is_whole_and_241_is_cut() {
        let at = format!("error: {}", "a".repeat(MAX - 7));
        assert_eq!(at.len(), 240);
        assert_eq!(first_error(&at, Style::Cargo).as_deref(), Some(at.as_str()));
        let over = format!("{at}b");
        let cut = first_error(&over, Style::Cargo).unwrap();
        assert_eq!(cut, format!("{at}…"));
    }

    #[test]
    fn non_ascii_and_control_bytes_become_question_marks() {
        let t = "error: caf\u{e9} \u{1b}[31mred\u{7}\tx\u{7f}\n";
        // é is two bytes, ESC, BEL, TAB, DEL one each.
        assert_eq!(
            first_error(t, Style::Cargo).as_deref(),
            Some("error: caf?? ?[31mred??x?")
        );
    }

    #[test]
    fn a_multibyte_char_straddling_the_cut_is_safe() {
        let t = format!("error: {}\u{e9}\u{e9}", "a".repeat(MAX - 7 - 1));
        let cut = first_error(&t, Style::Cargo).unwrap();
        assert_eq!(cut, format!("error: {}?…", "a".repeat(MAX - 8)));
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
