//! Checks the hand-written FFI in `ffi.rs` against the vendored C header
//! `vendor/ngspice-44.2/sharedspice.h` (`AET-WO-2200` section 1.3, ruling L3).
//!
//! `ffi.rs` declares by hand what libngspice exports, and a wrong signature
//! there is undefined behaviour that no compiler flags. So this test reads
//! both files exactly as compiled, parses the few C forms the header uses
//! (`IMPEXP` functions, callback typedefs, two structs) and the few Rust forms
//! `ffi.rs` uses, and requires that every Rust declaration has a header
//! counterpart with the same parameters, return type and fields, and that every
//! `L<n>` line citation in a doc comment points at lines naming the declaration
//! it cites. A second test feeds the comparator wrong declarations and demands
//! that it notices, so a comparator that accepts everything cannot pass.
//!
//! This is a parser for these two files, not for C or Rust in general.

use std::collections::BTreeMap;
use std::fmt::Debug;

const HEADER: &str = include_str!("../vendor/ngspice-44.2/sharedspice.h");
const FFI: &str = include_str!("ffi.rs");
/// Rust struct names and their C names.
const STRUCTS: [(&str, &str); 6] = [
    ("NgComplex", "ngcomplex"),
    ("VectorInfo", "vector_info"),
    ("VecValues", "vecvalues"),
    ("VecValuesAll", "vecvaluesall"),
    ("VecInfo", "vecinfo"),
    ("VecInfoAll", "vecinfoall"),
];

/// Return type and parameter types, in the normal form of `c_ty`.
type Sig = (String, Vec<String>);
type Sigs = BTreeMap<String, Sig>;
/// Field name and type, in order.
type Fields = Vec<(String, String)>;

/// Blanks comments out, keeping every newline so line numbers survive.
fn strip_comments(src: &str) -> String {
    let (mut out, mut block) = (String::new(), false);
    let mut it = src.chars().peekable();
    while let Some(c) = it.next() {
        match (block, c, it.peek().copied()) {
            (true, '*', Some('/')) => {
                it.next();
                block = false;
            }
            (true, '\n', _) => out.push('\n'),
            (true, ..) => {}
            (false, '/', Some('*')) => block = true,
            (false, '/', Some('/')) => while it.next_if(|&n| n != '\n').is_some() {},
            (false, ..) => out.push(c),
        }
    }
    out
}

/// The C statements outside `#ifdef XSPICE` and `#ifdef __cplusplus`, each on
/// one line of text. Preprocessor lines are skipped.
fn c_statements(clean: &str) -> Vec<String> {
    let (mut out, mut cur, mut depth, mut cont) = (Vec::new(), String::new(), 0i32, false);
    let mut gated: Vec<bool> = Vec::new();
    for t in clean.lines().map(str::trim) {
        if cont || t.starts_with('#') {
            if t.starts_with("#if") {
                gated.push(matches!(t, "#ifdef XSPICE" | "#ifdef __cplusplus"));
            } else if t.starts_with("#endif") {
                gated.pop();
            }
            cont = t.ends_with('\\');
        } else if !t.is_empty() && !gated.contains(&true) {
            cur.push_str(t);
            cur.push(' ');
            depth += t.matches('{').count() as i32 - t.matches('}').count() as i32;
            if depth == 0 && t.ends_with(';') {
                out.push(cur.trim().to_owned());
                cur.clear();
            }
        }
    }
    out
}

/// One C declarator as (type, name): `char *x` and `char* x` both give
/// `("char*", "x")`. Without `named`, the last word is a name only when a type
/// word remains before it.
fn c_ty(decl: &str, named: bool) -> (String, String) {
    let spaced = decl.replace('*', " * ");
    let mut w: Vec<&str> = spaced.split_whitespace().collect();
    let ident = |s: &&str| s.chars().all(|c| c.is_alphanumeric() || c == '_');
    let has_name = w.len() > 1
        && w.last().is_some_and(ident)
        && (named || !["const", "unsigned", "struct"].contains(&w[w.len() - 2]));
    let name = if has_name { w.pop().unwrap() } else { "" }.to_owned();
    let ty = w
        .iter()
        .map(|&s| match s {
            "NG_BOOL" => "bool",
            "pvector_info" => "vector_info *",
            "pvecvalues" => "vecvalues *",
            "pvecvaluesall" => "vecvaluesall *",
            "pvecinfo" => "vecinfo *",
            "pvecinfoall" => "vecinfoall *",
            "ngcomplex_t" => "ngcomplex",
            s => s,
        })
        .collect::<Vec<_>>()
        .join(" ");
    (ty.replace(" *", "*"), name)
}

fn c_params(list: &str) -> Vec<String> {
    match list.trim() {
        "" | "void" => Vec::new(),
        l => l.split(',').map(|p| c_ty(p, false).0).collect(),
    }
}

/// Functions, callback typedefs and structs of the header.
fn parse_header(clean: &str) -> (Sigs, Sigs, BTreeMap<String, Fields>) {
    let (mut funcs, mut cbs, mut structs) = (Sigs::new(), Sigs::new(), BTreeMap::new());
    for s in c_statements(clean) {
        if let Some(rest) = s.strip_prefix("IMPEXP") {
            let (head, args) = rest.split_once('(').unwrap();
            let head = head.trim_end();
            let name = head.rsplit([' ', '*']).next().unwrap();
            let params = c_params(args.rsplit_once(')').unwrap().0);
            funcs.insert(
                name.to_owned(),
                (c_ty(&head[..head.len() - name.len()], false).0, params),
            );
        } else if let Some(open) = s.find('{') {
            let name = s
                .split("struct ")
                .nth(1)
                .and_then(|r| r.split_whitespace().next());
            let body = &s[open + 1..s.rfind('}').unwrap()];
            let fields = body.split(';').filter(|d| !d.trim().is_empty());
            let fields = fields.map(|d| c_ty(d, true)).map(|(t, n)| (n, t)).collect();
            structs.insert(name.unwrap().to_owned(), fields);
        } else if let Some((ret, rest)) = s.strip_prefix("typedef").and_then(|r| r.split_once('('))
            && let Some((name, args)) = rest.split_once(")(")
        {
            let params = c_params(args.rsplit_once(')').unwrap().0);
            cbs.insert(name.trim().to_owned(), (c_ty(ret, false).0, params));
        }
    }
    (funcs, cbs, structs)
}

/// One Rust declaration of `ffi.rs`: kind `s` (struct), `t` (callback alias)
/// or `f` (function in the `extern` block), with its doc comment.
struct Decl {
    kind: char,
    name: String,
    body: String,
    doc: String,
}

/// The declarations of `ffi.rs` up to the end of its `extern` block.
fn rust_decls(src: &str) -> Vec<Decl> {
    let (mut out, mut cur, mut doc, mut in_block) =
        (Vec::new(), None::<Decl>, String::new(), false);
    for line in src.lines().map(str::trim) {
        if let Some(c) = cur.as_mut() {
            c.body.push_str(line);
            c.body.push('\n');
        } else {
            let start = if let Some(r) = line.strip_prefix("pub struct ") {
                Some(('s', r.trim_end_matches(" {"), ""))
            } else if let Some((n, b)) = line
                .strip_prefix("pub type ")
                .and_then(|r| r.split_once('='))
            {
                Some(('t', n.trim(), b.trim()))
            } else if let Some(r) = line.strip_prefix("fn ").filter(|_| in_block) {
                r.find('(').map(|i| ('f', &r[..i], &r[i..]))
            } else {
                None
            };
            if let Some((kind, name, body)) = start {
                let (name, body, doc) =
                    (name.to_owned(), body.to_owned(), std::mem::take(&mut doc));
                cur = Some(Decl {
                    kind,
                    name,
                    body,
                    doc,
                });
            } else if let Some(d) = line.strip_prefix("///") {
                doc.push_str(d);
                doc.push(' ');
            } else if line.starts_with("unsafe extern") {
                in_block = true;
            } else if in_block && line == "}" {
                break;
            }
        }
        if cur.as_ref().is_some_and(|c| {
            if c.kind == 's' {
                line == "}"
            } else {
                line.ends_with(';')
            }
        }) {
            out.extend(cur.take());
        }
    }
    out
}

/// A Rust type in the normal form of `c_ty`; `Option<Alias>` is the C pointer
/// to the callback typedef the alias is named after (up to case).
fn rust_ty(t: &str, cbs: &Sigs) -> String {
    let t = t.trim();
    if let Some(r) = t.strip_prefix("*mut ") {
        return format!("{}*", rust_ty(r, cbs));
    }
    if let Some(r) = t.strip_prefix("*const ") {
        return format!("const {}*", rust_ty(r, cbs));
    }
    if let Some(r) = t.strip_prefix("Option<").and_then(|r| r.strip_suffix('>')) {
        return format!("{}*", rust_ty(r, cbs));
    }
    match t {
        "c_int" => "int".into(),
        "c_double" => "double".into(),
        "c_char" => "char".into(),
        "c_void" => "void".into(),
        "i16" => "short".into(),
        "bool" => "bool".into(),
        _ => c_struct(t)
            .or_else(|| c_callback(t, cbs))
            .unwrap_or(t)
            .to_owned(),
    }
}

fn c_struct(rust: &str) -> Option<&'static str> {
    STRUCTS.iter().find(|s| s.0 == rust).map(|s| s.1)
}

fn c_callback<'a>(rust: &str, cbs: &'a Sigs) -> Option<&'a str> {
    cbs.keys()
        .map(String::as_str)
        .find(|c| c.eq_ignore_ascii_case(rust))
}

/// A Rust signature from its text starting at the parameter list. `named`
/// strips `name:` from each parameter.
fn rust_sig(text: &str, named: bool, cbs: &Sigs) -> Sig {
    let (open, close) = (text.find('(').unwrap(), text.rfind(')').unwrap());
    let params = text[open + 1..close]
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty());
    let params = params.map(|p| {
        if named {
            p.split_once(':').unwrap().1
        } else {
            p
        }
    });
    let ret = text[close + 1..].trim().trim_end_matches(';').trim();
    let ret = ret
        .strip_prefix("->")
        .map_or("void".into(), |r| rust_ty(r, cbs));
    (ret, params.map(|p| rust_ty(p, cbs)).collect())
}

/// `L<n>` and `L<a>-<b>` in a doc comment, each with the text before it
/// since the previous citation.
fn citations(doc: &str) -> Vec<(usize, usize, &str)> {
    let number = |at: usize| {
        let n = doc[at..].chars().take_while(|c| c.is_ascii_digit()).count();
        (doc[at..at + n].parse::<usize>().unwrap(), at + n)
    };
    let (b, mut out, mut seg, mut i) = (doc.as_bytes(), Vec::new(), 0, 0);
    while i < b.len() {
        if b[i] == b'L'
            && b.get(i + 1).is_some_and(u8::is_ascii_digit)
            && (i == 0 || !b[i - 1].is_ascii_alphanumeric())
        {
            let (first, j) = number(i + 1);
            let (last, end) = if doc[j..].starts_with('-') {
                number(j + 1)
            } else {
                (first, j)
            };
            out.push((first, last, &doc[seg..i]));
            (seg, i) = (end, end);
        } else {
            i += 1;
        }
    }
    out
}

fn has_word(text: &str, word: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    text.match_indices(word).any(|(i, _)| {
        !text[..i].chars().next_back().is_some_and(ident)
            && !text[i + word.len()..].chars().next().is_some_and(ident)
    })
}

fn compare<T: PartialEq + Debug>(
    what: &str,
    name: &str,
    rust: T,
    c: Option<&T>,
    errs: &mut Vec<String>,
) {
    match c {
        None => errs.push(format!("{what} `{name}` is not declared in the header")),
        Some(c) if *c != rust => errs.push(format!("{what} `{name}`: Rust {rust:?}, header {c:?}")),
        _ => {}
    }
}

/// Every disagreement between `ffi` and `header`; empty when they match.
fn check(header: &str, ffi: &str) -> Vec<String> {
    let clean = strip_comments(header);
    let lines: Vec<&str> = clean.lines().collect();
    let (funcs, cbs, structs) = parse_header(&clean);
    let mut errs = Vec::new();
    // The three C aliases `c_ty` and `rust_ty` rely on.
    let stmts = c_statements(&clean);
    for need in [
        "typedef bool NG_BOOL;",
        "typedef struct ngcomplex ngcomplex_t;",
        "*pvector_info;",
    ] {
        if !stmts.iter().any(|s| s.contains(need)) {
            errs.push(format!("the header no longer says `{need}`"));
        }
    }
    let decls = rust_decls(ffi);
    for kind in ['s', 't', 'f'] {
        if !decls.iter().any(|d| d.kind == kind) {
            errs.push(format!(
                "ffi.rs has no declaration of kind `{kind}`: the parser found nothing to check"
            ));
        }
    }
    for d in &decls {
        let c_name = match d.kind {
            's' => c_struct(&d.name),
            't' => c_callback(&d.name, &cbs),
            _ => Some(d.name.as_str()),
        };
        match (d.kind, c_name) {
            (_, None) => errs.push(format!("`{}` has no counterpart in the header", d.name)),
            ('s', Some(c)) => {
                let fields = d
                    .body
                    .lines()
                    .filter_map(|l| l.trim_end_matches(',').split_once(':'));
                let fields: Fields = fields
                    .map(|(n, t)| {
                        (
                            n.trim_start_matches("pub ")
                                .trim()
                                .trim_start_matches("r#")
                                .to_owned(),
                            rust_ty(t, &cbs),
                        )
                    })
                    .collect();
                compare("struct", &d.name, fields, structs.get(c), &mut errs);
            }
            ('t', Some(c)) => compare(
                "callback",
                &d.name,
                rust_sig(&d.body, false, &cbs),
                cbs.get(c),
                &mut errs,
            ),
            (_, Some(c)) => compare(
                "function",
                &d.name,
                rust_sig(&d.body, true, &cbs),
                funcs.get(c),
                &mut errs,
            ),
        }
        let cites = citations(&d.doc);
        if cites.is_empty() {
            errs.push(format!("`{}` cites no header lines", d.name));
        }
        for (first, last, before) in cites {
            let want = if before.contains("NG_BOOL") {
                "NG_BOOL"
            } else {
                c_name.unwrap_or(&d.name)
            };
            let text = lines
                .get(first - 1..last)
                .map(|l| l.join(" "))
                .unwrap_or_default();
            if !has_word(&text, want) {
                errs.push(format!(
                    "`{}` cites L{first}-{last}, which does not name `{want}`",
                    d.name
                ));
            }
        }
    }
    errs
}

#[test]
fn every_declaration_matches_the_vendored_header() {
    let errs = check(HEADER, FFI);
    assert!(
        errs.is_empty(),
        "ffi.rs disagrees with sharedspice.h:\n{}",
        errs.join("\n")
    );
}

#[test]
fn the_comparator_rejects_wrong_declarations() {
    // (text in ffi.rs, wrong replacement, a name the report must mention)
    let wrong_rust = [
        (
            "fn ngSpice_SetBkpt(time: c_double) -> bool;",
            "fn ngSpice_SetBkpt(time: c_int) -> bool;",
            "ngSpice_SetBkpt",
        ),
        (
            "fn ngSpice_SetBkpt(time: c_double) -> bool;",
            "fn ngSpice_SetBkpt(time: c_double) -> c_int;",
            "ngSpice_SetBkpt",
        ),
        (
            "fn ngSpice_Command(command: *mut c_char)",
            "fn ngSpice_Command(command: *const c_char)",
            "ngSpice_Command",
        ),
        ("fn ngSpice_Circ(", "fn ngSpice_Circle(", "ngSpice_Circle"),
        (
            "printfcn: Option<SendChar>,",
            "printfcn: Option<SendStat>,",
            "ngSpice_Init",
        ),
        (
            "SendChar = extern \"C\" fn(*mut c_char, c_int, ",
            "SendChar = extern \"C\" fn(*mut c_char, ",
            "SendChar",
        ),
        ("pub v_flags: i16,", "pub v_flags: c_int,", "VectorInfo"),
        ("    pub cx_imag: c_double,\n", "", "NgComplex"),
        ("/// L451-452.", "/// L371-373.", "ngSpice_SetBkpt"),
        ("/// L390-391.", "/// L385-386.", "ngSpice_Command"),
    ];
    for (from, to, want) in wrong_rust {
        assert!(
            FFI.contains(from),
            "the mutant `{from}` no longer applies to ffi.rs"
        );
        let errs = check(HEADER, &FFI.replace(from, to));
        assert!(
            errs.iter().any(|e| e.contains(want)),
            "`{to}` was accepted: {errs:?}"
        );
    }
    let from = "int  ngSpice_Command(char* command);";
    assert!(
        HEADER.contains(from),
        "the mutant `{from}` no longer applies to the header"
    );
    let errs = check(
        &HEADER.replace(from, "int  ngSpice_Command(char* command, int extra);"),
        FFI,
    );
    assert!(
        errs.iter().any(|e| e.contains("ngSpice_Command")),
        "a changed header was accepted: {errs:?}"
    );
}
