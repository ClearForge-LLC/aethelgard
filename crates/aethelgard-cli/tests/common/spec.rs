//! What a walker of committed JSON may skip (`AET-WO-4400g` F5, `INV-06`).

use std::path::Path;

use aethelgard_ir::{SPECIFICATION_SCHEMA_VERSION, Specification};

/// The committed specification in `bytes` (read from `path`), or `None` for
/// JSON that is not one. An object with both `mcu` and `board` is a
/// specification (every one that parses has both, and no other committed
/// JSON does). It must parse and carry the current schema version, or the
/// walker fails naming it. It never skips one, since a skipped
/// specification reads as a pass: every committed specification moves with
/// the schema version.
pub fn committed(path: &Path, bytes: &[u8]) -> Option<Specification> {
    let v: serde_json::Value = serde_json::from_slice(bytes).unwrap_or_else(|e| {
        panic!(
            "{}: committed JSON that does not parse: {e}",
            path.display()
        )
    });
    if v.get("mcu").is_none() || v.get("board").is_none() {
        return None;
    }
    let spec: Specification = serde_json::from_slice(bytes).unwrap_or_else(|e| {
        panic!(
            "{}: a specification that does not parse: {e}",
            path.display()
        )
    });
    assert_eq!(
        spec.schema_version,
        SPECIFICATION_SCHEMA_VERSION,
        "{}: a committed specification at another schema version",
        path.display()
    );
    Some(spec)
}
