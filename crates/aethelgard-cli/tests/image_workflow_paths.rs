//! `AET-WO-4408` (`4407` Q1), widened by `AET-WO-1917`: every file the
//! ArduPilot image workflow's jobs read triggers that workflow.
//!
//! `ardupilot-image.yml` runs only when a pull request or a push to `main`
//! changes a file its `paths:` lists name. If a file a job reads is missing
//! from those lists, a pull request that changes only that file never runs the
//! jobs it changes. `debian-trixie.sources` (4408), `container/IMAGE` and
//! `bridge/**` (review 3, A15) and `boards/sentinel-01/**` (1917 §1.1) were
//! such files.
//!
//! The inputs are derived, never listed here (1917 D2). Four readers:
//! - every `COPY` source of every Dockerfile in `container/` (the build
//!   context), and the Dockerfiles themselves;
//! - every repository path a job names after `jobs:` (`$(cat F)`, `bash F`, a
//!   `-v "$PWD/D:…"` mount, a matrix `spec:`, an argument), and every path
//!   named by a script so named, recursively; a directory stands for every
//!   file under it;
//! - a `Cargo.toml` so named (`--manifest-path`) stands for its whole
//!   package directory;
//! - every `path =` dependency of the root `Cargo.toml`: every
//!   `--locked` build in the workspace loads it.
//!
//! Each must match a glob in both lists. Each reader is proven by a planted
//! miss below. It is a tier-1 test because the workflow cannot guard itself:
//! on a pull request that misses its `paths:`, it does not run.
//!
//! What no reader here sees: a path a script builds from a variable or a glob
//! (`check-pins.sh` reads every `*.sources` beside its Dockerfile). Those
//! live in `container/`, which the lists name whole.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const CONTEXT: &str = "container";
const WORKFLOW: &str = ".github/workflows/ardupilot-image.yml";

/// The repository paths of the sources of each `COPY` from the build
/// context. A `COPY --from=` reads another stage and is skipped. A form this
/// reader does not take panics rather than being misread: the JSON form, a
/// continuation line, a wildcard, or a source outside the context.
fn copy_sources(dockerfile: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (n, line) in dockerfile.lines().enumerate() {
        let words: Vec<&str> = line.split_whitespace().collect();
        if !words
            .first()
            .is_some_and(|w| w.eq_ignore_ascii_case("COPY"))
        {
            continue;
        }
        let at = n + 1;
        assert!(
            !line.trim_end().ends_with('\\'),
            "line {at}: a continued COPY is not read here"
        );
        let args: Vec<&str> = words[1..]
            .iter()
            .copied()
            .filter(|w| !w.starts_with("--"))
            .collect();
        if words[1..].iter().any(|w| w.starts_with("--from=")) {
            continue;
        }
        assert!(
            args.len() >= 2 && !args[0].starts_with('['),
            "line {at}: not `COPY SRC... DEST`"
        );
        for src in &args[..args.len() - 1] {
            assert!(
                !src.contains(['*', '?', '[']) && !src.split('/').any(|c| c == ".."),
                "line {at}: source `{src}` is a wildcard or leaves the context"
            );
            out.push(format!("{CONTEXT}/{}", src.trim_start_matches("./")));
        }
    }
    out
}

/// The Dockerfiles of the build context: `Dockerfile` and `*.Dockerfile`.
fn dockerfiles(root: &Path) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(root.join(CONTEXT))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n == "Dockerfile" || n.ends_with(".Dockerfile"))
        .map(|n| format!("{CONTEXT}/{n}"))
        .collect();
    out.sort();
    out
}

/// Every entry under `dir` that is not a directory (repository-relative; a
/// symlink is an entry, dangling or not), skipping `target/`, `.git/` and
/// `__pycache__/`; a file stands for itself.
fn files_under(root: &Path, dir: &str) -> Vec<String> {
    let p = root.join(dir);
    if !fs::symlink_metadata(&p).unwrap().is_dir() {
        return vec![dir.to_owned()];
    }
    let mut out = Vec::new();
    for e in fs::read_dir(&p).unwrap() {
        let e = e.unwrap();
        let name = e.file_name().into_string().unwrap();
        if ["target", ".git", "__pycache__"].contains(&name.as_str()) {
            continue;
        }
        out.extend(files_under(root, &format!("{dir}/{name}")));
    }
    out
}

/// The repository paths named in `text`: each run of path characters that
/// holds a `/`, or follows `$PWD/` (the checkout: `-v "$PWD/container:…"`),
/// does not start at a `/` (an absolute or in-container path), does not leave
/// the tree, is not under `target/`, and exists. Comment lines are skipped.
fn named_paths(root: &Path, text: &str) -> Vec<String> {
    let is_path = |c: char| c.is_ascii_alphanumeric() || "_./-".contains(c);
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        // (token, whether it follows `$PWD/`)
        let mut toks: Vec<(&str, bool)> = Vec::new();
        for (i, part) in line.split("$PWD/").enumerate() {
            let mut words = part.split(|c: char| !is_path(c));
            if i > 0 {
                toks.push((words.next().unwrap_or(""), true));
            }
            toks.extend(words.map(|w| (w, false)));
        }
        for (tok, checkout) in toks {
            let tok = tok.trim_start_matches("./").trim_end_matches(['/', '.']);
            // `"$boards/$name"` leaves `boards/`: a bare segment is a path only
            // after `$PWD/`.
            if tok.is_empty()
                || (!checkout && !tok.contains('/'))
                || tok.starts_with(['/', '-'])
                || tok.split('/').any(|c| c == "..")
                || tok.starts_with("target/")
            {
                continue;
            }
            if fs::symlink_metadata(root.join(tok)).is_ok() {
                out.push(tok.to_owned());
            }
        }
    }
    out
}

/// The `path = "…"` dependencies of a `Cargo.toml`.
fn path_dependencies(cargo_toml: &str) -> Vec<String> {
    cargo_toml
        .lines()
        .filter_map(|l| {
            let (_, rest) = l.split_once("path = \"")?;
            Some(rest.split('"').next()?.to_owned())
        })
        .collect()
}

/// Every file the workflow's jobs read, by the four readers above.
fn inputs(root: &Path, workflow: &str, cargo_toml: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for d in dockerfiles(root) {
        out.extend(copy_sources(&fs::read_to_string(root.join(&d)).unwrap()));
        out.insert(d);
    }
    let jobs = workflow
        .split_once("\njobs:\n")
        .expect("no `jobs:` in the workflow")
        .1;
    let mut todo: Vec<String> = named_paths(root, jobs);
    let mut seen = BTreeSet::new();
    while let Some(p) = todo.pop() {
        if !seen.insert(p.clone()) {
            continue;
        }
        // A manifest stands for its package; a script for what it names.
        let dir = match p.strip_suffix("/Cargo.toml") {
            Some(d) => d.to_owned(),
            None => p.clone(),
        };
        for f in files_under(root, &dir) {
            if f.ends_with(".sh") || f.ends_with(".py") {
                todo.extend(named_paths(root, &fs::read_to_string(root.join(&f)).unwrap()));
            }
            out.insert(f);
        }
    }
    for d in path_dependencies(cargo_toml) {
        out.extend(files_under(root, &d));
    }
    out
}

/// The `paths:` list under `on.<event>` of the workflow, read by indentation:
/// the event at two spaces, `paths:` at four, its items at six.
fn paths(workflow: &str, event: &str) -> Vec<String> {
    let mut lines = workflow.lines().skip_while(|l| *l != "on:").skip(1);
    let head = format!("  {event}:");
    lines
        .by_ref()
        .find(|l| *l == head)
        .unwrap_or_else(|| panic!("no `on.{event}` in the workflow"));
    lines
        .by_ref()
        .find(|l| *l == "    paths:")
        .unwrap_or_else(|| panic!("no `on.{event}.paths`"));
    let items: Vec<String> = lines
        .map_while(|l| l.strip_prefix("      - "))
        .map(|item| item.trim().trim_matches('"').to_owned())
        .collect();
    assert!(!items.is_empty(), "`on.{event}.paths` is empty");
    items
}

/// GitHub's path filter for the forms the workflow uses: `**` matches any
/// run of characters, `/` included; `*` matches any run without `/`.
fn glob(pattern: &[u8], path: &[u8]) -> bool {
    match pattern {
        [] => path.is_empty(),
        [b'*', b'*', rest @ ..] => (0..=path.len()).any(|i| glob(rest, &path[i..])),
        [b'*', rest @ ..] => (0..=path.len())
            .take_while(|&i| i == 0 || path[i - 1] != b'/')
            .any(|i| glob(rest, &path[i..])),
        [c, rest @ ..] => path.first() == Some(c) && glob(rest, &path[1..]),
    }
}

/// The files of `sources` that no pattern of each event's list matches,
/// as `(event, file)`.
fn unlisted<'a>(
    workflow: &str,
    sources: impl IntoIterator<Item = &'a String> + Clone,
) -> Vec<(&'static str, String)> {
    let mut missing = Vec::new();
    for event in ["pull_request", "push"] {
        let globs = paths(workflow, event);
        for s in sources.clone() {
            if !globs.iter().any(|g| glob(g.as_bytes(), s.as_bytes())) {
                missing.push((event, s.clone()));
            }
        }
    }
    missing
}

#[test]
fn every_file_the_ardupilot_image_workflow_reads_triggers_it() {
    let root = root();
    let workflow = fs::read_to_string(root.join(WORKFLOW)).unwrap();
    let cargo = fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let inputs = inputs(&root, &workflow, &cargo);
    // The readers are not vacuous: each finds what 1917 §1.1 measured.
    for want in [
        "container/ardupilot.Dockerfile",
        "container/debian-trixie.sources",
        "container/engine.Dockerfile",
        "container/TOOLS-SOURCES.md",
        "container/IMAGE",
        "container/ardupilot-build.sh",
        "container/fixtures/check-pins/apt-config-env.Dockerfile",
        "ci/foreign-judge.sh",
        "bridge/build.rs",
        "bridge/src/main.rs",
        "boards/sentinel-01/Cargo.toml",
        "examples/sentinel-01.json",
        "crates/aethelgard-pincheck/fixtures/step-g.allow.json",
        "crates/aethelgard-cli/tests/fixtures/four-port.json",
    ] {
        assert!(inputs.contains(want), "{want} not derived: {inputs:?}");
    }
    for s in &inputs {
        assert!(fs::symlink_metadata(root.join(s)).is_ok(), "{s} is not an entry");
    }
    assert_eq!(unlisted(&workflow, &inputs), []);
}

/// The COPY reader on planted text: a file missing from one list is named
/// under that event only; a stage copy is skipped; the glob forms match as
/// GitHub's do.
#[test]
fn a_planted_unlisted_copy_is_named() {
    let dockerfile = "FROM x\nCOPY a.sources /etc/apt/a\ncopy --chmod=0644 b/c.txt ./d.md /opt/\nCOPY --from=s /out/x /y\n";
    let sources = copy_sources(dockerfile);
    assert_eq!(
        sources,
        ["container/a.sources", "container/b/c.txt", "container/d.md"]
    );
    let workflow = "on:\n  pull_request:\n    paths:\n      - \"container/*.md\"\n      - \"container/b/**\"\n      - \"container/a.sources\"\n  push:\n    branches: [main]\n    paths:\n      - \"container/*\"\n\npermissions:\n";
    assert_eq!(
        unlisted(workflow, &sources),
        [("push", "container/b/c.txt".to_owned())]
    );

    assert!(glob(
        b"container/ardupilot*",
        b"container/ardupilot.Dockerfile"
    ));
    assert!(!glob(b"container/*", b"container/b/c.txt"));
    assert!(glob(b"container/**", b"container/b/c.txt"));
    assert!(glob(b"crates/**", b"crates/a/b/c.rs"));
    assert!(!glob(
        b"container/debian-trixie.source",
        b"container/debian-trixie.sources"
    ));
}

#[test]
fn a_source_outside_the_context_is_refused() {
    let r = std::panic::catch_unwind(|| copy_sources("COPY ../secret /x\n"));
    let e = r.expect_err("a source outside the context must panic");
    assert!(e.downcast_ref::<String>().unwrap().contains("leaves the context"));
}

/// A planted tree, one input per reader, and a workflow whose lists name only
/// `container/a.Dockerfile`. Each reader's file is named as unlisted, under
/// both events; and a path a job names only in a comment, an in-container
/// path, and `target/` are not inputs.
#[test]
fn each_reader_names_a_planted_miss() {
    let t = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("aet-1917-paths-{}", std::process::id()));
    let _ = fs::remove_dir_all(&t);
    for (f, body) in [
        ("container/a.Dockerfile", "FROM x\nCOPY copied.txt /c\n"),
        ("container/copied.txt", "c"),
        ("container/IMAGE", "i"),
        ("pkg/Cargo.toml", "[package]"),
        ("pkg/src/main.rs", "fn main() {}"),
        ("mounted/m.json", "{}"),
        ("tools/s.sh", "cat data/by-script.txt\n"),
        ("data/by-script.txt", "d"),
        ("dep/Cargo.toml", "[package]"),
        ("dep/src/lib.rs", ""),
        ("spec/x.json", "{}"),
        ("commented/c.txt", "c"),
        ("target/debug/bin", ""),
    ] {
        let p = t.join(f);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
    let paths = "on:\n  pull_request:\n    paths:\n      - \"container/a.Dockerfile\"\n  push:\n    paths:\n      - \"container/a.Dockerfile\"\n";
    let jobs = "jobs:\n  j:\n    steps:\n      # commented/c.txt is only in a comment\n      - run: docker pull \"$(cat container/IMAGE)\"\n      - run: cargo build --manifest-path pkg/Cargo.toml && target/debug/bin\n      - run: docker run -v \"$PWD/mounted:/m:ro\" x bash /m/in-container.sh && bash tools/s.sh\n        with:\n          spec: spec/x.json\n";
    let workflow = format!("{paths}\n{jobs}");
    let cargo = "[workspace.dependencies]\nd = { path = \"dep\" }\n";
    let got = unlisted(&workflow, &inputs(&t, &workflow, cargo));
    let want: Vec<(&str, String)> = ["pull_request", "push"]
        .into_iter()
        .flat_map(|e| {
            [
                "container/IMAGE",
                "container/copied.txt",
                "data/by-script.txt",
                "dep/Cargo.toml",
                "dep/src/lib.rs",
                "mounted/m.json",
                "pkg/Cargo.toml",
                "pkg/src/main.rs",
                "spec/x.json",
                "tools/s.sh",
            ]
            .map(|f| (e, f.to_owned()))
        })
        .collect();
    assert_eq!(got, want);
    fs::remove_dir_all(&t).unwrap();
}
