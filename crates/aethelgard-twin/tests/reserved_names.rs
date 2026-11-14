//! `AET-WO-2204` D5, tier 1: `aethelgard_ir::RESERVED_REFERENCES` is exactly
//! the names a lowercased device reference can collide with in the pinned
//! Renode include chain. The chain is read from the vendored copies at run
//! time (nothing is compiled in), checked against the hashes in
//! `RENODE_FILES`, and parsed; a line match would take the `else:` lines of
//! the embedded Python for entries.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use aethelgard_ir::RESERVED_REFERENCES;
use aethelgard_twin::platform::{RENODE_FILES, STOCK, mux_name};
use sha2::{Digest, Sha256};

/// The `.repl` keywords (`Grammar.cs` of Renode 1.17.0, lines 106-129), lowercased.
const KEYWORDS: [&str; 11] = [
    "as", "empty", "false", "init", "local", "new", "none", "preinit", "reset", "true", "using",
];
/// The two names `CreationDriver.cs` declares before any file loads.
const BUILTINS: [&str; 2] = ["machine", "sysbus"];

fn vendored(path: &str) -> String {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/renode-1.17.0")
        .join(path);
    let bytes = fs::read(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let (_, want) = RENODE_FILES.iter().find(|f| f.0 == path).unwrap();
    let got: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(got, *want, "{path}");
    String::from_utf8(bytes).unwrap()
}

/// Whether `c` can be in a `.repl` identifier.
fn word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The names a `.repl` file declares: each column-0 `name: Type ...` entry.
/// An untyped `name:` is an update of an existing instance, not a declaration.
/// `'''` opens and closes a string, inside which nothing is an entry; an
/// entry's own line is read before its quotes are counted, and a `//` comment
/// outside a string is dropped first.
fn declared(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_string = false;
    for raw in text.lines() {
        let line = if in_string {
            raw
        } else {
            let at = raw.find("//").unwrap_or(raw.len());
            // A `//` after an opening `'''` is string text, not a comment.
            match raw.find("'''") {
                Some(q) if q < at => raw,
                _ => &raw[..at],
            }
        };
        if !in_string && line.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            let end = line.find(|c: char| !word(c)).unwrap_or(line.len());
            let (name, rest) = line.split_at(end);
            if let Some(after) = rest.strip_prefix(':') {
                let typed = after
                    .trim_start()
                    .starts_with(|c: char| c.is_ascii_alphabetic());
                if typed && !KEYWORDS.contains(&name) {
                    names.push(name.to_owned());
                }
            }
        }
        if line.matches("'''").count() % 2 == 1 {
            in_string = !in_string;
        }
    }
    assert!(!in_string, "a string block never closes");
    names
}

fn chain_names() -> BTreeSet<String> {
    let mut all = Vec::new();
    for (path, _) in RENODE_FILES {
        all.extend(declared(&vendored(path)));
    }
    all.into_iter().collect()
}

#[test]
fn the_parser_skips_string_blocks_comments_and_updates() {
    let text = "using \"x.repl\"\n// c: Not.AnEntry\na: T.One @ sysbus\nb:\n    attr: 1\n\
                c: T.Two @ {\n    x: 1\n}\nd: P.Py @ sysbus 0x0\n    script: '''\nelse:\n  y = 1\n'''\n\
                e: T.Three @ none\nf: P.Py @ sysbus 1\n    s: '''\nif x:\nelse: z\n  '''\n\
                g: T.Four @ none // ''' tail\n";
    assert_eq!(declared(text), ["a", "c", "d", "e", "f", "g"]);
}

#[test]
fn the_reserved_list_is_the_pinned_chain_s_lowercase_names_builtins_keywords_and_muxes() {
    let mut want: BTreeSet<String> = chain_names()
        .into_iter()
        .filter(|n| n.chars().all(|c| !c.is_ascii_uppercase()))
        .collect();
    want.extend(BUILTINS.iter().chain(&KEYWORDS).map(|s| (*s).to_owned()));
    // The emitter's own names: one multiplexer per SPI controller the pinned
    // header has (`AET-WO-4200` D7).
    let facts = aethelgard_svd_hal::cmsis::register_facts().unwrap();
    let spi: Vec<String> = (1..=9)
        .map(|n| format!("SPI{n}"))
        .filter(|p| facts.base(p).is_ok())
        .collect();
    assert_eq!(spi.len(), 6, "the H753 has SPI1 to SPI6");
    want.extend(spi.iter().map(|p| mux_name(&p.to_ascii_lowercase())));
    let have: BTreeSet<String> = RESERVED_REFERENCES
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    assert_eq!(
        have,
        want,
        "left is the IR list, right the chain's:\n{:?}",
        want.iter().collect::<Vec<_>>()
    );
}

#[test]
fn every_lowercase_stock_name_is_reserved() {
    let chain = chain_names();
    for (_, name, _, _) in STOCK {
        assert!(chain.contains(name), "{name} is not in the chain");
        if name.chars().all(|c| !c.is_ascii_uppercase()) {
            assert!(RESERVED_REFERENCES.contains(&name), "{name}");
        }
    }
}

#[test]
fn a_name_with_a_capital_is_not_reserved_because_the_emitter_lowercases() {
    // `gpioPortA` lowercases to `gpioporta`, which Renode loads beside `gpioPortA`.
    assert!(chain_names().contains("gpioPortA"));
    assert!(!RESERVED_REFERENCES.contains(&"gpioporta"));
}
