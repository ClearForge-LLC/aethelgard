//! `AET-WO-4408` (`4407` Q1): the files the ArduPilot image is built from all
//! trigger the workflow that builds and judges it.
//!
//! `ardupilot-image.yml` runs only when a pull request or a push to `main`
//! changes a file its `paths:` lists name. If a file the image `COPY`s is
//! missing from those lists, a pull request that changes only that file never
//! builds the image it changes. `debian-trixie.sources` was such a file: it
//! pins apt's snapshot date.
//!
//! The test reads every `COPY` source of `container/ardupilot.Dockerfile` (its
//! build context is `container/`), plus the Dockerfile itself. Each must match
//! a glob in both lists. It is a tier-1 test because the workflow cannot guard
//! itself: on a pull request that misses its `paths:`, it does not run.

use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const DOCKERFILE: &str = "container/ardupilot.Dockerfile";
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
fn unlisted(workflow: &str, sources: &[String]) -> Vec<(&'static str, String)> {
    let mut missing = Vec::new();
    for event in ["pull_request", "push"] {
        let globs = paths(workflow, event);
        for s in sources {
            if !globs.iter().any(|g| glob(g.as_bytes(), s.as_bytes())) {
                missing.push((event, s.clone()));
            }
        }
    }
    missing
}

#[test]
fn every_file_the_ardupilot_image_is_built_from_triggers_its_workflow() {
    let dockerfile = fs::read_to_string(root().join(DOCKERFILE)).unwrap();
    let workflow = fs::read_to_string(root().join(WORKFLOW)).unwrap();
    let mut sources = copy_sources(&dockerfile);
    // The reader is not vacuous: six COPYs from the context today.
    assert!(sources.len() >= 6, "{sources:?}");
    for s in &sources {
        assert!(root().join(s).is_file(), "{s} is not a file");
    }
    sources.push(DOCKERFILE.to_owned());
    assert_eq!(unlisted(&workflow, &sources), []);
}

/// The reader on planted text: a file missing from one list is named under
/// that event only; a stage copy is skipped; the glob forms match as GitHub's
/// do.
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
#[should_panic(expected = "leaves the context")]
fn a_source_outside_the_context_is_refused() {
    copy_sources("COPY ../secret /x\n");
}
