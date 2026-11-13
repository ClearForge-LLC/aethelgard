//! `AET-WO-1306` §1.3: the compiled fence that keeps first-match reads off
//! caller input.
//!
//! **The mechanism** is a type: every gate reads a [`strict::Tree`], which
//! only `aethelgard_eda::strict` produces, through [`strict::Node`], which
//! has no first-match lookup (`one` refuses a second match). A `.child(…)`
//! added to a gate's reading of the tree does not compile.
//!
//! **This guard** pins what the type cannot: which modules may hold a raw
//! `Sx` tree and look into it (the readers of vendored data, which hashes
//! pin), which may parse KiCad text, and that every other module of
//! `aethelgard-eda` and `aethelgard-cli` does neither. A module not listed
//! here fails, so a new reader cannot slip past; the methods are named by
//! path below, so renaming one fails to compile; and `Sx`'s and `Node`'s
//! methods are pinned, so a new lookup cannot be added beside them.

use std::fs;
use std::path::Path;

use aethelgard_eda::sexpr::Sx;
use aethelgard_eda::strict;

/// What may look into a raw tree: the first-match lookups and the raw item
/// list. Anything holding one can resolve a repeated child first-or-last.
const LOOKUPS: &[&str] = &[
    ".child(",
    ".children(",
    ".property(",
    ".items(",
    ".items_mut(",
];
/// The code as one line of single-spaced tokens, so a statement split over
/// lines reads as one.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The module's statements: its flattened code cut at `;`, `{` and `}`.
fn statements(text: &str) -> Vec<String> {
    let flat = flat(text);
    flat.split([';', '{', '}']).map(str::to_owned).collect()
}

/// What could turn KiCad text into a raw tree: a path to `sexpr::parse`, an
/// import of `sexpr` under another name, a glob, or `parse` or `as` inside a
/// grouped import from `sexpr` (`AET-WO-1306` adversarial F2: `use
/// crate::sexpr as raw;` passed a line-by-line scan). Read on the
/// flattened code, so a multi-line `use` is one.
fn parses(text: &str) -> Vec<String> {
    let flat = flat(text).replace(":: ", "::").replace(" ::", "::");
    let mut found = Vec::new();
    for (at, _) in flat.match_indices("sexpr") {
        let rest = &flat[at + "sexpr".len()..];
        let group = rest
            .strip_prefix("::{")
            .and_then(|g| g.split_once('}'))
            .map(|(g, _)| g);
        let bad = rest.starts_with("::parse")
            || rest.starts_with(" as ")
            || rest.starts_with("::*")
            || group.is_some_and(|g| {
                g.split([',', ' '])
                    .any(|w| matches!(w, "parse" | "as" | "*"))
            });
        if bad {
            found.push(flat[at..(at + 40).min(flat.len())].to_owned());
        }
    }
    found
}

/// A statement that takes one item from a `Node`'s children by position or
/// by search: one of several, chosen first or last, which is what `one`
/// refuses (`AET-WO-1306` adversarial F1: `pad.all("net").next()` passed
/// the type and the name scan).
const PICKS: &[&str] = &[
    ".next()",
    ".last()",
    ".nth(",
    ".find(",
    ".find_map(",
    ".position(",
    ".rposition(",
    ".max_by",
    ".min_by",
    ".skip(",
    ".take(",
    ".step_by(",
    ".rev()",
];

fn picks(text: &str) -> Vec<String> {
    statements(text)
        .into_iter()
        .filter(|st| {
            let from = [".all(", ".lists("].iter().filter_map(|t| st.find(t)).min();
            from.is_some_and(|i| PICKS.iter().any(|p| st[i..].contains(p)))
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Role {
    /// Defines `Sx` and its lookups.
    Sexpr,
    /// The strict reader: parses caller input, looks into the raw tree to
    /// judge it, and hands out only `Node`s.
    Strict,
    /// Reads vendored data (hashes pinned by `aethelgard-kicad-lib`) or the
    /// generator's own trees: may look and parse.
    Vendored,
    /// Reads caller input: only through `strict`.
    Gate,
    /// Reads no KiCad file.
    Other,
}

/// Every module of the two crates, by role.
const MODULES: &[(&str, Role)] = &[
    ("aethelgard-eda/src/sexpr.rs", Role::Sexpr),
    ("aethelgard-eda/src/strict.rs", Role::Strict),
    ("aethelgard-eda/src/library.rs", Role::Vendored),
    ("aethelgard-eda/src/schematic.rs", Role::Vendored),
    ("aethelgard-eda/src/pcb.rs", Role::Vendored),
    ("aethelgard-eda/src/board.rs", Role::Gate),
    ("aethelgard-eda/src/parity.rs", Role::Gate),
    ("aethelgard-eda/src/route.rs", Role::Gate),
    ("aethelgard-eda/src/extract.rs", Role::Gate),
    ("aethelgard-eda/src/bom.rs", Role::Other),
    ("aethelgard-eda/src/lib.rs", Role::Other),
    ("aethelgard-eda/src/rules.rs", Role::Other),
    ("aethelgard-cli/src/kicad.rs", Role::Gate),
    ("aethelgard-cli/src/extract.rs", Role::Gate),
    ("aethelgard-cli/src/verify.rs", Role::Gate),
    ("aethelgard-cli/src/main.rs", Role::Gate),
    ("aethelgard-cli/src/analog.rs", Role::Other),
    ("aethelgard-cli/src/campaign.rs", Role::Other),
    ("aethelgard-cli/src/elf.rs", Role::Other),
    ("aethelgard-cli/src/firmware.rs", Role::Other),
    ("aethelgard-cli/src/generate.rs", Role::Other),
    ("aethelgard-cli/src/input.rs", Role::Other),
    ("aethelgard-cli/src/kani.rs", Role::Other),
    ("aethelgard-cli/src/lib.rs", Role::Other),
    ("aethelgard-cli/src/mcp.rs", Role::Other),
    ("aethelgard-cli/src/propose.rs", Role::Other),
    ("aethelgard-cli/src/renode.rs", Role::Other),
    ("aethelgard-cli/src/run.rs", Role::Other),
    ("aethelgard-cli/src/said.rs", Role::Other),
    ("aethelgard-cli/src/tree.rs", Role::Other),
    ("aethelgard-cli/src/verdict.rs", Role::Other),
];

/// Each gate module's call into the strict reader, or into a gate module
/// that makes it.
const ENTRY: &[(&str, &str)] = &[
    ("aethelgard-eda/src/board.rs", "strict::board("),
    ("aethelgard-eda/src/parity.rs", "strict::schematic("),
    ("aethelgard-eda/src/extract.rs", "strict::board("),
    ("aethelgard-eda/src/route.rs", "strict::footprint("),
    (
        "aethelgard-cli/src/kicad.rs",
        "aethelgard_eda::board::read(",
    ),
    (
        "aethelgard-cli/src/kicad.rs",
        "aethelgard_eda::parity::flat(",
    ),
    (
        "aethelgard-cli/src/kicad.rs",
        "aethelgard_eda::parity::schematic(",
    ),
];

fn crates() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
}

/// A module's code, without its `#[cfg(test)]` tests.
fn code(module: &str) -> String {
    let text = fs::read_to_string(crates().join(module)).unwrap();
    match text.find("#[cfg(test)]") {
        Some(at) => text[..at].to_owned(),
        None => text,
    }
}

/// The names of the `pub fn`s in the `impl` block that starts with `head`.
fn methods(module: &str, head: &str) -> Vec<String> {
    let text = code(module);
    let start = text
        .find(head)
        .unwrap_or_else(|| panic!("{module}: no `{head}`"));
    let body = &text[start..];
    let end = body.find("\n}\n").unwrap();
    body[..end]
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            l.strip_prefix("pub fn ")
                .or_else(|| l.strip_prefix("pub(crate) fn "))
        })
        .map(|l| l.split(['(', '<']).next().unwrap().to_owned())
        .collect()
}

#[test]
fn every_module_is_known_and_reads_kicad_files_only_as_its_role_allows() {
    let mut found = Vec::new();
    for krate in ["aethelgard-eda", "aethelgard-cli"] {
        for entry in fs::read_dir(crates().join(krate).join("src")).unwrap() {
            let name = entry.unwrap().file_name().into_string().unwrap();
            found.push(format!("{krate}/src/{name}"));
        }
    }
    found.sort();
    let mut known: Vec<String> = MODULES.iter().map(|(m, _)| (*m).to_owned()).collect();
    known.sort();
    assert_eq!(
        found, known,
        "a module was added or removed: give it a role here"
    );
    for (module, role) in MODULES {
        let text = code(module);
        let looks: Vec<&str> = LOOKUPS
            .iter()
            .copied()
            .filter(|t| text.contains(t))
            .collect();
        let parses = parses(&text);
        if *role == Role::Gate {
            let picks = picks(&text);
            assert!(
                picks.is_empty(),
                "{module} takes one of a node's children by position or search: {picks:?}"
            );
        }
        match role {
            Role::Sexpr | Role::Vendored | Role::Strict => {}
            Role::Gate | Role::Other => {
                assert!(
                    looks.is_empty(),
                    "{module} looks into a raw tree: {looks:?}"
                );
                assert!(
                    parses.is_empty(),
                    "{module} parses KiCad text itself: {parses:?}"
                );
            }
        }
        // A gate names no raw tree, but `route::part_of`'s, which it hands
        // straight to the strict reader (`pcb` calls it, outside the fence).
        let raw = text.matches("Sx").count();
        let allowed = match *module {
            "aethelgard-eda/src/route.rs" => 2,
            _ if *role == Role::Gate || *role == Role::Other => 0,
            _ => raw,
        };
        assert_eq!(raw, allowed, "{module} names `Sx` {raw} times");
    }
}

#[test]
fn every_gate_module_calls_the_strict_reader() {
    for (module, call) in ENTRY {
        assert!(code(module).contains(call), "{module} does not call {call}");
    }
    // The entry points by path: renaming one fails to compile here.
    let _: fn(&str) -> Result<strict::Tree, strict::Refusal> = strict::board;
    let _: fn(&str) -> Result<strict::Tree, strict::Refusal> = strict::schematic;
    let _: fn(&Sx) -> Result<strict::Tree, strict::Refusal> = strict::footprint;
    let _ = aethelgard_eda::board::read;
    let _ = aethelgard_eda::parity::flat;
}

#[test]
fn the_lookups_are_pinned_by_path_and_by_name() {
    // The first-match lookups by path: renaming one fails to compile here,
    // and the scan above then searches for the new name.
    let _ = Sx::child;
    let _ = Sx::children;
    let _ = Sx::property;
    let _ = Sx::items;
    let _ = Sx::items_mut;
    assert_eq!(
        methods("aethelgard-eda/src/sexpr.rs", "impl Sx {"),
        [
            "atom",
            "str",
            "node",
            "head",
            "items",
            "items_mut",
            "text",
            "arg",
            "children",
            "child",
            "property"
        ],
        "a lookup was added to `Sx`: name it in LOOKUPS"
    );
    assert_eq!(
        methods("aethelgard-eda/src/strict.rs", "impl<'a> Node<'a> {"),
        [
            "head", "arg", "last", "len", "texts", "lists", "all", "one", "named", "field"
        ],
        "a method was added to `Node`: it must not resolve a repeat first-or-last"
    );
}
