//! `AET-WO-1608` D3: an `InvalidIdentifier` message ends with the fix, the
//! field's key and the whole shape of its identifier family. Each family's
//! text is pinned here, and each shape is held to `check_identifier` over a
//! sample, so a value written to the quoted shape is never refused by the
//! next identifier rule (WO §5: "a quoted fix that, pasted as written, is
//! refused by the next rule").

use aethelgard_ir::{
    IdentifierKind, RESERVED_REFERENCES, Specification, check_identifier, identifier_fix,
};
use serde_json::Value;

const BENCH: &str = include_str!("../../../examples/sentinel-02-bench.json");

const BASE: &str = "1 to 64 bytes of printable ASCII 0x21-0x7E, none of them `/`, `{`, `}`, `$`, `[`, `]`, `\"`, `%`, `*`, `,` or `\\`, and no date shape (4 digits, `-`, 2 digits, `-`, 2 digits) or time shape (2 digits, `:`, 2 digits, `:`, 2 digits)";
const PLAIN_SHAPE: &str = "1 to 64 bytes of printable ASCII 0x21-0x7E, none of / { } $ [ ] \" % * , or \\, with no date shape dddd-dd-dd or time shape dd:dd:dd";
const PROJECT_SHAPE: &str = "1 to 58 bytes of a-z, 0-9 and -, a letter first, not ending with -, with no -- and no date shape dddd-dd-dd, and not a Windows device name (con, prn, aux, nul, com0 to com9, lpt0 to lpt9)";
const REFERENCE_SHAPE: &str = "1 to 64 bytes, a letter then letters, digits and _, in any case neither a name the Renode platform holds (RESERVED_REFERENCES) nor p, a port a to k, a pin 0 to 15 and obs (pa8obs)";

/// The bench example with `set` applied, validated: its one refusal.
fn refused(set: impl FnOnce(&mut Value)) -> aethelgard_ir::IrValidationError {
    let mut v: Value = serde_json::from_str(BENCH).unwrap();
    set(&mut v);
    let spec: Specification = serde_json::from_value(v).unwrap();
    spec.validate().unwrap_err()
}

#[test]
fn an_empty_project_name_quotes_the_crate_name_shape() {
    // Measured before this WO: the base rule alone, which `My_Board` obeys
    // and the crate-name rule then refuses.
    let e = refused(|v| v["project_name"] = "".into());
    assert_eq!(e.kind(), "InvalidIdentifier");
    assert_eq!(e.path(), Some("project_name"));
    assert_eq!(
        e.to_string(),
        format!(
            "`project_name` is not an identifier: {BASE}; the fix: \"project_name\": \"<{PROJECT_SHAPE}>\""
        )
    );
}

#[test]
fn a_project_name_past_the_base_rule_quotes_the_same_shape() {
    let e = refused(|v| v["project_name"] = "My_Board".into());
    assert_eq!(
        e.to_string(),
        format!(
            "`project_name` is not an identifier: a project name is a crate name: 1 to 58 bytes of `a-z`, `0-9` and `-`, starting with a letter, not ending with `-`, with no `--` (`generate` names the board crate `<project>-board`, at most 64 bytes); the fix: \"project_name\": \"<{PROJECT_SHAPE}>\""
        )
    );
}

#[test]
fn a_reference_quotes_the_renode_shape() {
    let e = refused(|v| v["spi_devices"][0]["reference"] = "".into());
    assert_eq!(e.path(), Some("spi_devices[0].reference"));
    assert_eq!(
        e.to_string(),
        format!(
            "`spi_devices[0].reference` is not an identifier: {BASE}; the fix: \"reference\": \"<{REFERENCE_SHAPE}>\""
        )
    );
}

#[test]
fn a_plain_name_quotes_the_base_shape() {
    let e = refused(|v| v["i2c_buses"][0]["sda"]["net"] = "a/b".into());
    assert_eq!(e.path(), Some("i2c_buses[0].sda.net"));
    assert_eq!(
        e.to_string(),
        format!(
            "`i2c_buses[0].sda.net` is not an identifier: {BASE}; the fix: \"net\": \"<{PLAIN_SHAPE}>\""
        )
    );
}

#[test]
fn the_key_is_the_last_segment_and_an_element_quotes_its_array() {
    let fix = |p| identifier_fix(p, IdentifierKind::Plain);
    assert_eq!(fix("mcu.part"), format!("\"part\": \"<{PLAIN_SHAPE}>\""));
    assert_eq!(
        fix("pin_devices[0].straps.IMODE.tie"),
        format!("\"tie\": \"<{PLAIN_SHAPE}>\"")
    );
    assert_eq!(
        fix("connectors[1].pins[3].pin"),
        format!("\"pin\": \"<{PLAIN_SHAPE}>\"")
    );
    assert_eq!(
        fix("net_classes[0].nets[2]"),
        format!("\"nets\": [..., \"<{PLAIN_SHAPE}>\"]")
    );
    assert_eq!(
        fix("project_name"),
        format!("\"project_name\": \"<{PLAIN_SHAPE}>\"")
    );
}

/// Each shape, restated here from its text alone.
fn date_shape(b: &[u8]) -> bool {
    let shaped = |pat: &[u8]| {
        b.windows(pat.len()).any(|w| {
            w.iter().zip(pat).all(|(c, p)| {
                if *p == b'd' {
                    c.is_ascii_digit()
                } else {
                    c == p
                }
            })
        })
    };
    shaped(b"dddd-dd-dd") || shaped(b"dd:dd:dd")
}

fn plain_shape(s: &str) -> bool {
    let b = s.as_bytes();
    (1..=64).contains(&b.len())
        && b.iter()
            .all(|c| (0x21..=0x7E).contains(c) && !br#"/{}$[]"%*,\"#.contains(c))
        && !date_shape(b)
}

fn project_shape(s: &str) -> bool {
    let b = s.as_bytes();
    let windows = ["con", "prn", "aux", "nul"]
        .iter()
        .map(|w| w.to_string())
        .chain((0..10).flat_map(|n| [format!("com{n}"), format!("lpt{n}")]))
        .any(|w| w == s);
    (1..=58).contains(&b.len())
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
        && b[0].is_ascii_lowercase()
        && !s.ends_with('-')
        && !s.contains("--")
        && !date_shape(b)
        && !windows
}

fn reference_shape(s: &str) -> bool {
    let b = s.as_bytes();
    let lower = s.to_ascii_lowercase();
    let observer = lower
        .strip_prefix('p')
        .and_then(|r| {
            let mut chars = r.chars();
            let port = chars.next()?;
            let pin = chars.as_str().strip_suffix("obs")?;
            Some(('a'..='k').contains(&port) && (0..=15).any(|n| n.to_string() == pin))
        })
        .unwrap_or(false);
    (1..=64).contains(&b.len())
        && b[0].is_ascii_alphabetic()
        && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
        && !RESERVED_REFERENCES.contains(&lower.as_str())
        && !observer
}

/// Every string of up to three bytes over an alphabet that reaches each
/// rule, and the edge cases no short string can.
fn sample() -> Vec<String> {
    let alphabet = [
        "a", "z", "C", "o", "p", "0", "9", "-", "_", ":", "/", " ", "\"", "é",
    ];
    let mut out = vec![String::new()];
    let mut layer = vec![String::new()];
    for _ in 0..3 {
        layer = layer
            .iter()
            .flat_map(|s| alphabet.iter().map(move |c| format!("{s}{c}")))
            .collect();
        out.extend(layer.iter().cloned());
    }
    for s in [
        "my-board",
        "a--b",
        "board-",
        "con",
        "Con",
        "com0",
        "lpt9",
        "com10",
        "a2026-11-23",
        "b-2026-11-2",
        "x12:34:56",
        "U5",
        "spi1",
        "SPI1MUX",
        "sysbus",
        "Machine",
        "pa8obs",
        "PK15obs",
        "pa16obs",
        "pa08obs",
        "pl1obs",
        "pobs",
        "pa8ob",
        "I2C_SDA",
        "1abc",
        "_a",
    ] {
        out.push(s.to_owned());
    }
    for n in [57, 58, 59, 63, 64, 65] {
        out.push("a".repeat(n));
        out.push(format!("b{}", "-1".repeat(n / 2)));
    }
    out
}

#[test]
fn a_value_of_the_quoted_shape_passes_and_no_other_does() {
    type Shape = fn(&str) -> bool;
    let families: [(IdentifierKind, Shape); 3] = [
        (IdentifierKind::Plain, plain_shape),
        (IdentifierKind::Project, project_shape),
        (IdentifierKind::Reference, reference_shape),
    ];
    let mut both = [0usize; 3];
    for (i, (kind, shape)) in families.into_iter().enumerate() {
        for s in sample() {
            let ok = check_identifier(kind, &s).is_ok();
            assert_eq!(ok, shape(&s), "{kind:?} {s:?}: check_identifier ok={ok}");
            both[i] += usize::from(ok);
        }
    }
    // The sample reaches both sides of every family.
    assert!(both.iter().all(|&n| n > 10), "{both:?}");
}
