//! D3 (`AET-WO-3303`): every version line in this crate's README names the
//! constant in the code. Uses only `aethelgard_ir` and `std` (`workspace_shape`).

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
