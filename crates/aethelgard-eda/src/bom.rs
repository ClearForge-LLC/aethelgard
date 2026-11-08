//! The bill of materials: one CSV row per part, sorted by reference, quoted
//! by RFC 4180 (a reference may hold `,` or `"`: `RUL-14` admits both).

use crate::schematic::BomRow;

pub const HEADER: &str = "Reference,Value,Footprint,Symbol,Note";

fn field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

pub fn write(rows: &[BomRow]) -> String {
    let mut sorted: Vec<&BomRow> = rows.iter().collect();
    sorted.sort_by(|a, b| a.reference.cmp(&b.reference));
    let mut out = String::from(HEADER);
    out.push_str("\r\n");
    for r in sorted {
        let line = [&r.reference, &r.value, &r.footprint, &r.symbol, &r.note]
            .map(|s| field(s))
            .join(",");
        out.push_str(&line);
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_sorted_and_quoted() {
        let row = |r: &str| BomRow {
            reference: r.into(),
            value: "1k".into(),
            footprint: "F".into(),
            symbol: "Device:R".into(),
            note: String::new(),
        };
        let out = write(&[row("R2"), row("A,\"B"), row("R1")]);
        assert_eq!(
            out,
            "Reference,Value,Footprint,Symbol,Note\r\n\"A,\"\"B\",1k,F,Device:R,\r\nR1,1k,F,Device:R,\r\nR2,1k,F,Device:R,\r\n"
        );
    }
}
