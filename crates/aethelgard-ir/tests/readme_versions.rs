//! D3 (`AET-WO-3303`): every version line in this crate's README names the
//! constant in the code, and (`AET-WO-1215`) every row of `versions::CHANGES`
//! is named in its version's sentence. Uses only `aethelgard_ir` and `std` (`workspace_shape`).

use aethelgard_ir::versions::CHANGES;
use aethelgard_ir::{SOLVED_SCHEMA_VERSION, SPECIFICATION_SCHEMA_VERSION};

fn readme() -> String {
    std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")).unwrap()
}

#[test]
fn the_specification_version_lines_name_the_constant() {
    let text = readme();
    let n = SPECIFICATION_SCHEMA_VERSION;
    let heading = text
        .lines()
        .find(|l| l.starts_with("## `Specification`, version "))
        .expect("the Specification heading");
    assert_eq!(heading, format!("## `Specification`, version {n}"));
    assert!(
        text.lines()
            .any(|l| l == format!("  \"schema_version\": {n},")),
        "the example's schema_version line is not {n}"
    );
    let rule = text
        .lines()
        .find(|l| l.starts_with("| 1 | `schema_version` is "))
        .expect("validation rule 1");
    assert!(
        rule.starts_with(&format!("| 1 | `schema_version` is {n} |")),
        "{rule}"
    );
}

#[test]
fn the_solved_version_line_names_the_constant() {
    let n = SOLVED_SCHEMA_VERSION;
    let line = readme()
        .lines()
        .find(|l| l.starts_with("- `schema_version` ("))
        .expect("the SolvedTopology schema_version bullet")
        .to_owned();
    assert!(
        line.starts_with(&format!("- `schema_version` ({n};")),
        "{line}"
    );
}

/// `AET-WO-1215`: every row of `versions::CHANGES` is the README's prose for
/// its version, row by row: the paragraph names its work order, and each of
/// its paths' last key and each of its rules.

#[test]
fn every_row_is_the_readme_prose_for_its_version() {
    let text = readme();
    let prose = text
        .lines()
        .find(|l| l.starts_with("Version 2 ("))
        .expect("the version paragraph");
    for (i, c) in CHANGES.iter().enumerate() {
        let start = prose
            .find(&format!("Version {} (`{}`", c.version, c.work_order))
            .unwrap_or_else(|| {
                panic!("README names no `Version {} ({})`", c.version, c.work_order)
            });
        let end = CHANGES
            .get(i + 1)
            .and_then(|n| prose.find(&format!("Version {} (", n.version)))
            .unwrap_or_else(|| prose.find("A file of another version is refused").unwrap());
        let text = &prose[start..end];
        let paths = c
            .requires
            .iter()
            .chain(c.optional)
            .copied()
            .chain(c.replaced.iter().flat_map(|r| [r.path, r.by]))
            .chain(c.removed.iter().copied());
        for path in paths {
            let key = path.rsplit('.').next().unwrap();
            assert!(
                text.contains(key),
                "{}: README's prose names no `{key}`",
                c.version
            );
        }
        for kind in c.rules {
            assert!(
                text.contains(kind),
                "{}: README's prose names no `{kind}`",
                c.version
            );
        }
    }
}
