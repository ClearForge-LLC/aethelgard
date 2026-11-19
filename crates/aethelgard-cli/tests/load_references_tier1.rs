//! `AET-WO-1209` D3, as ruled: the CLI's `validate` holds each
//! `power.loads` reference to the parts the emitter draws for that
//! specification (`LoadNamesUndrawnPart`), beyond the form step 20a holds.
//! The measured case: the test helpers kept a removed crossing's CS pull-up
//! (`R40`), an `R<n>` the pattern passes and the emitter no longer draws.

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// `aethelgard validate` on `v` through stdin: its exit code and its one line.
fn validate(v: &Value) -> (Option<i32>, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .arg("validate")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(v.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    (out.status.code(), serde_json::from_str(&stdout).unwrap())
}

fn with_load(mut v: Value, reference: &str) -> Value {
    v["power"]["loads"].as_array_mut().unwrap().push(json!({
        "domain": "VDD_3V3", "milliamps": 1,
        "source": "test: AET-WO-1209", "reference": reference
    }));
    v
}

#[test]
fn a_load_naming_a_resistor_the_emitter_does_not_draw_is_refused() {
    // The helper's board, as it stands, validates: its loads name drawn parts.
    let (code, v) = validate(&common::power::power_tree());
    assert_eq!(code, Some(0), "{v}");
    // The removed crossing's CS pull-up, and a number the emitter never
    // reaches: both pass step 20a's pattern, and neither is drawn.
    let at = common::power::power_tree()["power"]["loads"]
        .as_array()
        .unwrap()
        .len();
    for r in ["R40", "R99"] {
        let (code, v) = validate(&with_load(common::power::power_tree(), r));
        assert_eq!(code, Some(2), "{v}");
        let path = format!("power.loads[{at}].reference");
        assert_eq!(v["error"]["kind"], "LoadNamesUndrawnPart", "{v}");
        assert_eq!(v["error"]["id"], r, "{v}");
        assert_eq!(v["error"]["path"], path.as_str(), "{v}");
        assert_eq!(
            v["error"]["message"],
            format!(
                "{path} on `VDD_3V3` names `{r}`, which the emitter does not draw for this specification"
            ),
            "{v}"
        );
    }
    // With its crossing, the fixture draws `R40`: the same load passes.
    let full: Value = serde_json::from_str(include_str!(
        "../../aethelgard-ir/tests/fixtures/sentinel-02-power.json"
    ))
    .unwrap();
    let (code, v) = validate(&full);
    assert_eq!(code, Some(0), "{v}");
}
