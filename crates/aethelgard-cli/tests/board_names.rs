//! No board's name in engine code (`AET-WO-1103`). Roadmap §6: capability
//! enters through the specification, so engine code never knows a board by
//! name. This test holds that rule over every `.rs` file of the engine.
//!
//! - **The names** come from the repository (`boards`).
//! - **The selection** is every `.rs` file under `crates/<member>/src/**`,
//!   `bridge/src/**` and `models/**` (`select`). Its `*` is one path component,
//!   so `crates/aethelgard-cli/tests/fixtures/forge-log/src` is not selected.
//! - **Allowed:** comments and doc comments (they cite where a value was
//!   measured), and `#[cfg(test)]` items, whether inline or out of line.
//! - **A hit:** a name in anything else, such as a string literal, an
//!   identifier or an attribute value.
//!
//! Two tests keep the selection from going vacuous: a planted tree must yield
//! exactly its planted hits, and each arm of the real selection holds a floor.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// ------------------------------------------------------------ the names

/// The board families, and the names they were derived from.
///
/// A name is a directory under `boards/` or a file stem under `examples/`. The
/// name is split on `-`. A name with an all-digit segment names a board: the
/// board is its segments up to that number, and the family is the segments
/// before it.
/// - `sentinel-01-bench` and `sentinel-01-minimal` are variants of
///   `sentinel-01`.
/// - `sentinel-02-bench` names `sentinel-02`.
///
/// A name with no number names no board and is returned as excluded:
/// `invalid-zero-domain` is `validate`'s refusal fixture.
///
/// The test matches a family followed by a number (`family_hits`), not each
/// derived board. A board with no directory or example of its own is still
/// caught when it shares a family, such as Sentinel-03, whose specification
/// is a test fixture. A board of a new family is caught only once it has a
/// directory or an example.
struct Names {
    families: BTreeSet<String>,
    boards: BTreeSet<String>,
    excluded: BTreeSet<String>,
}

fn names(root: &Path) -> Names {
    let mut stems = Vec::new();
    for (dir, want_dir) in [("boards", true), ("examples", false)] {
        for entry in fs::read_dir(root.join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() != want_dir {
                continue;
            }
            let stem = path.file_stem().unwrap().to_string_lossy().to_lowercase();
            stems.push(stem.replace('_', "-"));
        }
    }
    let mut names = Names {
        families: BTreeSet::new(),
        boards: BTreeSet::new(),
        excluded: BTreeSet::new(),
    };
    for stem in stems {
        let segments: Vec<&str> = stem.split('-').collect();
        match segments
            .iter()
            .position(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        {
            Some(n) if n > 0 => {
                names.families.insert(segments[..n].join("-"));
                names.boards.insert(segments[..=n].join("-"));
            }
            _ => {
                names.excluded.insert(stem);
            }
        }
    }
    names
}

/// Every place `text` names a board: a family, then `-`, `_` or nothing, then
/// digits, matched case-insensitively. Returns each match's byte offset and its
/// name, normalised to lower case and `-`.
fn family_hits(text: &str, families: &BTreeSet<String>) -> Vec<(usize, String)> {
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut hits = Vec::new();
    for family in families {
        // A family's own `-` matches `-` or `_` too.
        let pattern: Vec<u8> = family.bytes().collect();
        for start in 0..bytes.len() {
            let mut k = start;
            let mut matched = true;
            for &p in &pattern {
                let ok = match p {
                    b'-' => matches!(bytes.get(k), Some(b'-' | b'_')),
                    _ => bytes.get(k) == Some(&p),
                };
                if !ok {
                    matched = false;
                    break;
                }
                k += 1;
            }
            if !matched {
                continue;
            }
            if matches!(bytes.get(k), Some(b'-' | b'_')) {
                k += 1;
            }
            let digits = bytes[k.min(bytes.len())..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count();
            if digits > 0 {
                let name = lower[start..k + digits].replace('_', "-");
                hits.push((start, name));
            }
        }
    }
    hits.sort();
    hits
}

// ------------------------------------------------------------ the selection

/// The three arms of the selection, as `select` reads them.
const ARMS: [&str; 3] = ["crates/*/src", "bridge/src", "models"];

/// Every `.rs` file of each arm, relative to `root` and sorted. A missing
/// directory selects nothing, and the floor test reports it.
fn select(root: &Path) -> BTreeMap<&'static str, Vec<PathBuf>> {
    let mut selected = BTreeMap::new();
    for arm in ARMS {
        let mut dirs = Vec::new();
        match arm.split_once("/*/") {
            Some((parent, child)) => {
                if let Ok(entries) = fs::read_dir(root.join(parent)) {
                    for entry in entries {
                        let path = entry.unwrap().path();
                        if path.is_dir() {
                            dirs.push(path.join(child));
                        }
                    }
                }
            }
            None => dirs.push(root.join(arm)),
        }
        let mut files = Vec::new();
        while let Some(dir) = dirs.pop() {
            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    files.push(path.strip_prefix(root).unwrap().to_path_buf());
                }
            }
        }
        files.sort();
        selected.insert(arm, files);
    }
    selected
}

// ------------------------------------------------------------ the lexer

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tok {
    Comment,
    Doc,
    Str,
    Char,
    Lifetime,
    Ident,
    Num,
    Punct(u8),
}

fn ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

/// The end of a `"…"` string whose body starts at `i`.
fn string_end(b: &[u8], mut i: usize, file: &Path) -> usize {
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    panic!("{}: unterminated string", file.display());
}

/// Rust's tokens, each with its byte range, comments included. It knows
/// line, block (nested) and doc comments; strings, byte and C strings, and raw
/// strings of any `#` count; char literals as against lifetimes; raw
/// identifiers. It panics on an unterminated string or comment, so a source it
/// misreads fails loudly rather than skipping text.
fn lex(src: &str, file: &Path) -> Vec<(Tok, usize, usize)> {
    let b = src.as_bytes();
    let at = |i: usize, s: &str| b[i..].starts_with(s.as_bytes());
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let start = i;
        let tok = if c.is_ascii_whitespace() {
            i += 1;
            continue;
        } else if at(i, "//") {
            i = b[i..]
                .iter()
                .position(|&x| x == b'\n')
                .map_or(b.len(), |n| i + n);
            let doc = (at(start, "///") && !at(start, "////")) || at(start, "//!");
            if doc { Tok::Doc } else { Tok::Comment }
        } else if at(i, "/*") {
            let mut depth = 0;
            loop {
                if i >= b.len() {
                    panic!("{}: unterminated block comment", file.display());
                } else if at(i, "/*") {
                    depth += 1;
                    i += 2;
                } else if at(i, "*/") {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            let doc =
                (at(start, "/**") && !at(start, "/***") && !at(start, "/**/")) || at(start, "/*!");
            if doc { Tok::Doc } else { Tok::Comment }
        } else if c == b'"' {
            i = string_end(b, i + 1, file);
            Tok::Str
        } else if c == b'\'' {
            if b.get(i + 1) == Some(&b'\\') {
                let mut k = i + 3;
                while k < b.len() && b[k] != b'\'' {
                    k += 1;
                }
                i = k + 1;
                Tok::Char
            } else {
                // One character, then `'`, is a char literal; else a lifetime.
                let width = src[i + 1..].chars().next().map_or(1, char::len_utf8);
                if b.get(i + 1 + width) == Some(&b'\'') {
                    i += width + 2;
                    Tok::Char
                } else {
                    i += 1;
                    while i < b.len() && ident_byte(b[i]) {
                        i += 1;
                    }
                    Tok::Lifetime
                }
            }
        } else if c.is_ascii_digit() {
            while i < b.len()
                && (ident_byte(b[i])
                    || (b[i] == b'.' && b.get(i + 1).is_some_and(u8::is_ascii_digit)))
            {
                i += 1;
            }
            Tok::Num
        } else if ident_byte(c) {
            while i < b.len() && ident_byte(b[i]) {
                i += 1;
            }
            let word = &src[start..i];
            let hashes = b[i..].iter().take_while(|&&x| x == b'#').count();
            if matches!(word, "r" | "br" | "cr") && b.get(i + hashes) == Some(&b'"') {
                // A raw string: ends at `"` and the same number of `#`.
                let close = format!("\"{}", "#".repeat(hashes));
                let body = i + hashes + 1;
                let n = src[body..]
                    .find(&close)
                    .unwrap_or_else(|| panic!("{}: unterminated raw string", file.display()));
                i = body + n + close.len();
                Tok::Str
            } else if word == "r" && hashes == 1 && b.get(i + 1).is_some_and(|&x| ident_byte(x)) {
                // A raw identifier, `r#name`.
                i += 1;
                while i < b.len() && ident_byte(b[i]) {
                    i += 1;
                }
                Tok::Ident
            } else if matches!(word, "b" | "c") && b.get(i) == Some(&b'"') {
                i = string_end(b, i + 1, file);
                Tok::Str
            } else if word == "b" && b.get(i) == Some(&b'\'') {
                let mut k = i + 1;
                if b.get(k) == Some(&b'\\') {
                    k += 2;
                }
                while k < b.len() && b[k] != b'\'' {
                    k += 1;
                }
                i = k + 1;
                Tok::Char
            } else {
                Tok::Ident
            }
        } else {
            i += 1;
            Tok::Punct(c)
        };
        out.push((tok, start, i));
    }
    out
}

// ------------------------------------------------------------ the test-only regions

/// What `#[cfg(test)]` removes from one file.
#[derive(Default)]
struct TestOnly {
    /// The whole file (`#![cfg(test)]` before its first item).
    whole: bool,
    /// Byte ranges of inline `#[cfg(test)]` items.
    ranges: Vec<(usize, usize)>,
    /// Files and directories of out-of-line `#[cfg(test)] mod name;`
    /// declarations, relative to the root.
    files: Vec<PathBuf>,
    dirs: Vec<PathBuf>,
}

/// The test-only regions of `src`, from its tokens without comments.
///
/// - **Which attribute:** exactly `cfg(test)`. Anything else is scanned:
///   `cfg(not(test))`, `cfg(any(test, …))`, `cfg(all(test, …))`, `cfg_attr`,
///   and a bare `#[test]`. So the skip can only ever be narrower than what is
///   really test-only.
/// - **Where an item ends:** at the first `;` or `,` at its own depth, after a
///   `{…}` group at its own depth closes, or before the close of the group
///   around it. `()`, `[]` and `{}` all count toward depth. A `,` inside
///   generics or a closure's arguments ends the region early, which again
///   only scans more. A region never runs past its item.
/// - **Out-of-line modules:** `#[cfg(test)] mod name;` at a file's top level
///   removes `name.rs`, `name/mod.rs` and everything under `name/`, by Rust's
///   module path rule, or the file its `#[path = "…"]` names. A declaration
///   inside an inline module is not resolved, so its file is scanned.
fn test_only(src: &str, tokens: &[(Tok, usize, usize)], file: &Path) -> TestOnly {
    let code: Vec<(Tok, usize, usize)> = tokens
        .iter()
        .copied()
        .filter(|(t, _, _)| !matches!(t, Tok::Comment | Tok::Doc))
        .collect();
    let text = |n: usize| &src[code[n].1..code[n].2];
    let punct = |n: usize, p: u8| code.get(n).is_some_and(|t| t.0 == Tok::Punct(p));
    // The token after the `]` closing the attribute whose `[` is at `n`.
    let attr_end = |n: usize| {
        let mut depth = 0i32;
        for (k, t) in code.iter().enumerate().skip(n) {
            match t.0 {
                Tok::Punct(b'(' | b'[' | b'{') => depth += 1,
                Tok::Punct(b')' | b']' | b'}') => {
                    depth -= 1;
                    if depth == 0 {
                        return k + 1;
                    }
                }
                _ => {}
            }
        }
        panic!("{}: unterminated attribute", file.display());
    };
    let is_cfg_test = |open: usize, end: usize| {
        end == open + 6
            && text(open + 1) == "cfg"
            && punct(open + 2, b'(')
            && text(open + 3) == "test"
            && punct(open + 4, b')')
    };
    let mut out = TestOnly::default();
    let mut depth = 0i32;
    let mut seen_item = false;
    let mut n = 0;
    while n < code.len() {
        let inner = punct(n, b'#') && punct(n + 1, b'!') && punct(n + 2, b'[');
        let outer = punct(n, b'#') && punct(n + 1, b'[');
        if inner {
            let end = attr_end(n + 2);
            if depth == 0 && !seen_item && is_cfg_test(n + 2, end) {
                out.whole = true;
                return out;
            }
            n = end;
            continue;
        }
        if outer {
            let end = attr_end(n + 1);
            if !is_cfg_test(n + 1, end) {
                n = end;
                continue;
            }
            // Its further attributes, then the item itself.
            let start = code[n].1;
            let mut k = end;
            let mut path = None;
            while punct(k, b'#') && punct(k + 1, b'[') {
                let e = attr_end(k + 1);
                if text(k + 2) == "path" && punct(k + 3, b'=') && code[k + 4].0 == Tok::Str {
                    path = Some(text(k + 4).trim_matches('"').to_owned());
                }
                k = e;
            }
            let item = k;
            let mut d = 0i32;
            let stop = loop {
                let Some(t) = code.get(k) else {
                    panic!("{}: a #[cfg(test)] item runs to the end", file.display());
                };
                match t.0 {
                    Tok::Punct(b'(' | b'[' | b'{') => d += 1,
                    Tok::Punct(b')' | b']' | b'}') => {
                        d -= 1;
                        if d < 0 {
                            break k;
                        }
                        if d == 0 && t.0 == Tok::Punct(b'}') {
                            break k + 1;
                        }
                    }
                    Tok::Punct(b';' | b',') if d == 0 => break k + 1,
                    _ => {}
                }
                k += 1;
            };
            out.ranges.push((start, code[stop - 1].2));
            // `[pub[(…)]] mod name;` at the top level.
            let mut m = item;
            if text(m) == "pub" {
                m += 1;
                if punct(m, b'(') {
                    m = attr_end(m);
                }
            }
            if depth == 0 && text(m) == "mod" && code[m + 1].0 == Tok::Ident && punct(m + 2, b';') {
                let dir = file.parent().unwrap();
                let name = text(m + 1);
                if let Some(p) = path {
                    out.files.push(dir.join(p));
                } else {
                    let stem = file.file_stem().unwrap().to_string_lossy();
                    let base = if matches!(&*stem, "lib" | "main" | "mod") {
                        dir.to_path_buf()
                    } else {
                        dir.join(&*stem)
                    };
                    out.files.push(base.join(format!("{name}.rs")));
                    out.dirs.push(base.join(name));
                }
            }
            seen_item = true;
            n = stop;
            continue;
        }
        match code[n].0 {
            Tok::Punct(b'(' | b'[' | b'{') => depth += 1,
            Tok::Punct(b')' | b']' | b'}') => depth -= 1,
            _ => {}
        }
        seen_item = true;
        n += 1;
    }
    if depth != 0 {
        panic!("{}: unbalanced delimiters", file.display());
    }
    out
}

// ------------------------------------------------------------ the scan

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Class {
    /// In code that is not test-only: the hits this test forbids.
    Code,
    Comment,
    Doc,
    /// Inside an inline `#[cfg(test)]` item.
    TestItem,
    /// In a file that is a `#[cfg(test)]` module.
    TestFile,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Hit {
    class: Class,
    file: String,
    line: usize,
    name: String,
}

struct Scan {
    names: Names,
    selected: BTreeMap<&'static str, usize>,
    files: usize,
    test_files: usize,
    lines: usize,
    hits: Vec<Hit>,
}

fn scan(root: &Path) -> Scan {
    let names = names(root);
    let selected = select(root);
    let all: Vec<PathBuf> = selected.values().flatten().cloned().collect();
    let mut sources = Vec::new();
    let mut test_files = BTreeSet::new();
    let mut test_dirs = Vec::new();
    for file in &all {
        let src = fs::read_to_string(root.join(file)).unwrap();
        let tokens = lex(&src, file);
        let only = test_only(&src, &tokens, file);
        if only.whole {
            test_files.insert(file.clone());
        }
        test_files.extend(only.files.iter().cloned());
        test_dirs.extend(only.dirs.iter().cloned());
        sources.push((file, src, tokens, only.ranges));
    }
    let mut hits = Vec::new();
    let mut lines = 0;
    let mut tests = 0;
    for (file, src, tokens, ranges) in sources {
        let in_test_file =
            test_files.contains(file) || test_dirs.iter().any(|d| file.starts_with(d));
        tests += usize::from(in_test_file);
        lines += src.lines().count();
        for (offset, name) in family_hits(&src, &names.families) {
            let class = if in_test_file {
                Class::TestFile
            } else if ranges.iter().any(|&(s, e)| (s..e).contains(&offset)) {
                Class::TestItem
            } else {
                match tokens.iter().find(|t| (t.1..t.2).contains(&offset)) {
                    Some((Tok::Comment, ..)) => Class::Comment,
                    Some((Tok::Doc, ..)) => Class::Doc,
                    _ => Class::Code,
                }
            };
            let line = src[..offset].matches('\n').count() + 1;
            let file = file.to_string_lossy().into_owned();
            hits.push(Hit {
                class,
                file,
                line,
                name,
            });
        }
    }
    // One hit per name per line: `boards/x-01/x-01.repl` is one citation.
    hits.sort();
    hits.dedup();
    Scan {
        names,
        selected: selected.iter().map(|(arm, f)| (*arm, f.len())).collect(),
        files: all.len(),
        test_files: tests,
        lines,
        hits,
    }
}

fn code_hits(scan: &Scan) -> Vec<String> {
    scan.hits
        .iter()
        .filter(|h| h.class == Class::Code)
        .map(|h| format!("{}:{}: `{}`", h.file, h.line, h.name))
        .collect()
}

// ------------------------------------------------------------ the tests

#[test]
fn no_board_name_in_engine_code() {
    let scan = scan(&root());
    // The figures FEEDBACK reports (`cargo test --test board_names -- --nocapture`).
    println!(
        "families {:?}, boards {:?}, excluded {:?}",
        scan.names.families, scan.names.boards, scan.names.excluded
    );
    println!(
        "selected {:?}: {} files ({} test-only modules), {} lines",
        scan.selected, scan.files, scan.test_files, scan.lines
    );
    for h in &scan.hits {
        println!("{:?} {}:{} {}", h.class, h.file, h.line, h.name);
    }
    let hits = code_hits(&scan);
    assert!(
        hits.is_empty(),
        "a board's name in engine code (roadmap §6: capability enters through the specification; \
         move it into a comment or a test):\n{}",
        hits.join("\n")
    );
}

/// The selection's floor, per arm, so that an arm which stops matching fails
/// here even when the others still match. Measured on `main` at `52a2d62e`:
/// 98 files under `crates/*/src`, 8 under `bridge/src` and 1 under `models`.
/// Each floor sits a little below its count, so ordinary deletions pass.
const FLOORS: [(&str, usize); 3] = [("crates/*/src", 90), ("bridge/src", 7), ("models", 1)];

#[test]
fn each_arm_of_the_selection_holds_its_floor() {
    let selected = select(&root());
    for (arm, floor) in FLOORS {
        let found = selected[arm].len();
        assert!(
            found >= floor,
            "{arm} selects {found} files, below its floor {floor}"
        );
    }
}

#[test]
fn the_names_come_from_boards_and_examples() {
    let names = names(&root());
    assert!(names.families.contains("sentinel"), "{:?}", names.families);
    for board in ["sentinel-01", "sentinel-02"] {
        assert!(names.boards.contains(board), "{:?}", names.boards);
    }
    assert!(
        names.excluded.contains("invalid-zero-domain"),
        "{:?}",
        names.excluded
    );
    // A plain-sense "sentinel" is no board's name.
    assert!(family_hits("a sentinel value, sentinels 0", &names.families).is_empty());
}

/// A tree laid out like the repository, with a planted name in every arm at
/// depth and in every place the scan must judge. Exactly the planted code
/// hits are found, and every other planted name lands in its allowed class.
#[test]
fn a_planted_tree_yields_exactly_its_planted_hits() {
    let tree =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("board-names-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tree);
    let write = |path: &str, text: &str| {
        let path = tree.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    };
    fs::create_dir_all(tree.join("boards/acme-07")).unwrap();
    write("examples/acme-09-bench.json", "{}");
    write("examples/a-fixture.json", "{}");
    write(
        "crates/widget/src/lib.rs",
        "//! The acme-07 board was measured here.\n\
         #[cfg(test)]\n\
         mod tests;\n\
         #[cfg(test)]\n\
         #[path = \"elsewhere.rs\"]\n\
         mod other;\n\
         mod deep;\n",
    );
    write(
        "crates/widget/src/tests.rs",
        "const A: &str = \"acme-07\";\n",
    );
    write(
        "crates/widget/src/tests/inner.rs",
        "const A: &str = \"acme-07\";\n",
    );
    write(
        "crates/widget/src/elsewhere.rs",
        "const A: &str = \"acme-07\";\n",
    );
    write(
        "crates/widget/src/deep/er/x.rs",
        "// a comment: acme-07, and \"acme-07\" in quotes\n\
         /* nested /* acme-07 */ still a comment acme-07 */\n\
         /// a doc comment: ACME_07\n\
         const S: &str = \"see the Acme_07 board\";\n\
         const R: &str = r#\"raw \"acme-07\" string\"#;\n\
         const Q: char = '\"';\n\
         fn f<'a>(x: &'a str) -> &'a str { x }\n\
         const N: &str = \"#[cfg(test)] mod acme_07\";\n\
         #[cfg(test)]\n\
         mod tests {\n    const T: &str = \"acme-09\";\n    fn g() { let s = \"}\"; }\n}\n\
         #[cfg(any(test, feature = \"x\"))]\n\
         const U: &str = \"ACME07\";\n\
         fn h() {\n    #[cfg(test)]\n    let v = \"acme-07\";\n    let w = \"acme-11\";\n}\n\
         #[doc = \"acme-07\"]\n\
         fn i() {}\n",
    );
    write("crates/widget/tests/t.rs", "const A: &str = \"acme-07\";\n");
    write(
        "crates/widget/fixtures/x/src/y.rs",
        "const A: &str = \"acme-07\";\n",
    );
    write("bridge/src/a/b.rs", "fn acme_07() {}\n");
    write("bridge/vendor/v.rs", "const A: &str = \"acme-07\";\n");
    write("models/renode/src/c.rs", "static S: &str = \"acme-07\";\n");

    let scan = scan(&tree);
    let _ = fs::remove_dir_all(&tree);
    assert_eq!(
        scan.names.families,
        BTreeSet::from(["acme".to_owned()]),
        "{:?}",
        scan.names.excluded
    );
    assert_eq!(
        code_hits(&scan),
        [
            "bridge/src/a/b.rs:1: `acme-07`",
            "crates/widget/src/deep/er/x.rs:4: `acme-07`",
            "crates/widget/src/deep/er/x.rs:5: `acme-07`",
            "crates/widget/src/deep/er/x.rs:8: `acme-07`",
            "crates/widget/src/deep/er/x.rs:15: `acme07`",
            "crates/widget/src/deep/er/x.rs:19: `acme-11`",
            "crates/widget/src/deep/er/x.rs:21: `acme-07`",
            "models/renode/src/c.rs:1: `acme-07`",
        ]
    );
    let allowed: Vec<(Class, &str, usize)> = scan
        .hits
        .iter()
        .filter(|h| h.class != Class::Code)
        .map(|h| (h.class, h.file.as_str(), h.line))
        .collect();
    let x = "crates/widget/src/deep/er/x.rs";
    assert_eq!(
        allowed,
        [
            (Class::Comment, x, 1),
            (Class::Comment, x, 2),
            (Class::Doc, x, 3),
            (Class::Doc, "crates/widget/src/lib.rs", 1),
            (Class::TestItem, x, 11),
            (Class::TestItem, x, 18),
            (Class::TestFile, "crates/widget/src/elsewhere.rs", 1),
            (Class::TestFile, "crates/widget/src/tests.rs", 1),
            (Class::TestFile, "crates/widget/src/tests/inner.rs", 1),
        ]
    );
}
