//! What `check drc --schematic` refuses before KiCad's parity check runs
//! (`AET-WO-1301` §1.4 and its adversarial pass): the schematic copied in
//! beside the board, and whatever in the board would make KiCad 10.0.7's
//! parity check (`drc_test_provider_schematic_parity.cpp`) pass a board that
//! does not match its schematic. Each was measured passing parity in the
//! tools image:
//!
//! - a footprint with `(attr … board_only)`: parity skips it as neither
//!   extra nor duplicate, so copper the schematic does not hold passes;
//! - a pad whose `pintype` says `no_connect`: parity accepts its net if it
//!   starts with the schematic's net and `_`, so the pad can sit on another
//!   real net;
//! - a pad net starting with `unconnected-`: parity accepts it if it starts
//!   with the schematic's net, so two pins left unconnected can share one,
//!   or a pin on a rail whose name starts `unconnected-` can leave it.
//!
//! So the board may carry no `board_only`, no `no_connect` pad on a real
//! net, and no `unconnected-` net on two pads; and no label or value in the
//! schematic may start with `unconnected-`, KiCad's own prefix.

use std::collections::BTreeMap;

use crate::sexpr::{self, SexprError, Sx};

/// KiCad's prefix for the net of a pin the schematic leaves unconnected.
pub const UNCONNECTED: &str = "unconnected-";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParityRefusal {
    #[error("the schematic is not one KiCad S-expression: {0}")]
    Malformed(#[from] SexprError),
    #[error("the schematic's top-level list is not `kicad_sch`")]
    NotASchematic,
    #[error(
        "the schematic has a hierarchical sheet, which names a file the gate has not copied or hashed"
    )]
    Sheet,
    #[error("the schematic names a net `{0}`, which starts with KiCad's own `unconnected-`")]
    ReservedNetName(String),
    #[error("{0} is `board_only`, which KiCad's parity check skips")]
    BoardOnly(String),
    #[error(
        "{0} is a `no_connect` pad on a net, which KiCad's parity check matches by prefix alone"
    )]
    NoConnectPad(String),
    #[error("`{0}` is on more than one pad; KiCad gives each unconnected pin its own")]
    SharedUnconnectedNet(String),
}

impl ParityRefusal {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Malformed(_) | Self::NotASchematic => "MalformedSchematic",
            Self::Sheet => "HierarchicalSchematic",
            Self::ReservedNetName(_) => "ReservedNetName",
            Self::BoardOnly(_) => "BoardOnlyFootprint",
            Self::NoConnectPad(_) => "NoConnectPadOnNet",
            Self::SharedUnconnectedNet(_) => "SharedUnconnectedNet",
        }
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            Self::ReservedNetName(id)
            | Self::BoardOnly(id)
            | Self::NoConnectPad(id)
            | Self::SharedUnconnectedNet(id) => Some(id),
            _ => None,
        }
    }
}

/// Every list in `root`, depth first, without recursion.
fn lists(root: &Sx) -> impl Iterator<Item = &Sx> {
    let mut stack = vec![root];
    std::iter::from_fn(move || {
        let item = stack.pop()?;
        stack.extend(item.items().iter().filter(|c| matches!(c, Sx::List(_))));
        Some(item)
    })
}

/// The schematic parity is judged against: one `kicad_sch` that the gate's
/// reader and KiCad's read alike, with no `(sheet …)` anywhere, since KiCad
/// would read the file a sheet names (`AET-WO-1300` A8), and no label or
/// `Value` that KiCad would name a net `unconnected-…` by.
pub fn schematic(text: &str) -> Result<(), ParityRefusal> {
    let root = sexpr::parse(text)?;
    if root.head() != Some("kicad_sch") {
        return Err(ParityRefusal::NotASchematic);
    }
    for item in lists(&root) {
        let named = match item.head() {
            Some("sheet") => return Err(ParityRefusal::Sheet),
            Some("label" | "global_label" | "hierarchical_label") => item.arg(1),
            Some("property") if item.arg(1) == Some("Value") => item.arg(2),
            _ => None,
        };
        if let Some(name) = named.filter(|n| n.starts_with(UNCONNECTED)) {
            return Err(ParityRefusal::ReservedNetName(name.to_owned()));
        }
    }
    Ok(())
}

/// The board under parity: no `board_only` footprint, no `no_connect` pad
/// on a net other than an `unconnected-` one, and no `unconnected-` net on
/// two pads. Call after `board::prepare` accepted it.
pub fn board(text: &str) -> Result<(), ParityRefusal> {
    let root = sexpr::parse(text)?;
    let mut unconnected: BTreeMap<&str, usize> = BTreeMap::new();
    for fp in root.children("footprint") {
        let reference = fp.property("Reference").unwrap_or("?");
        if fp.children("attr").any(|a| {
            a.items()[1..]
                .iter()
                .any(|x| x.text() == Some("board_only"))
        }) {
            return Err(ParityRefusal::BoardOnly(format!("footprint {reference}")));
        }
        for pad in lists(fp).filter(|i| i.head() == Some("pad")) {
            let Some(net) = pad.child("net").and_then(|n| n.items().last()?.text()) else {
                continue;
            };
            let no_connect = pad
                .child("pintype")
                .and_then(|p| p.arg(1))
                .is_some_and(|t| t.contains("no_connect"));
            if net.starts_with(UNCONNECTED) {
                let n = unconnected.entry(net).or_default();
                *n += 1;
                if *n > 1 {
                    return Err(ParityRefusal::SharedUnconnectedNet(net.to_owned()));
                }
            } else if no_connect && !net.is_empty() {
                return Err(ParityRefusal::NoConnectPad(format!(
                    "pad {} of footprint {reference}",
                    pad.arg(1).unwrap_or("?")
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_schematic_passes_and_a_sheet_anywhere_is_refused() {
        assert_eq!(schematic("(kicad_sch (version 1) (label \"A\"))"), Ok(()));
        assert_eq!(
            schematic("(kicad_sch (x (sheet (at 0 0))))"),
            Err(ParityRefusal::Sheet)
        );
        assert_eq!(schematic("(kicad_pcb)"), Err(ParityRefusal::NotASchematic));
        assert!(matches!(
            schematic("(kicad_sch"),
            Err(ParityRefusal::Malformed(_))
        ));
    }

    #[test]
    fn a_name_kicad_reserves_is_refused_in_the_schematic() {
        for text in [
            "(kicad_sch (label \"unconnected-x\"))",
            "(kicad_sch (global_label \"unconnected-(U1-PA0-Pad34)\"))",
            "(kicad_sch (symbol (property \"Value\" \"unconnected-V\")))",
        ] {
            let e = schematic(text).unwrap_err();
            assert_eq!(e.kind(), "ReservedNetName", "{text}");
        }
        assert_eq!(
            schematic("(kicad_sch (symbol (property \"Description\" \"unconnected-x\")))"),
            Ok(())
        );
    }

    #[test]
    fn what_would_pass_parity_unmatched_is_refused_in_the_board() {
        let fp = |body: &str| {
            format!("(kicad_pcb (footprint \"R:R\" (property \"Reference\" \"R7\") {body}))")
        };
        let cases = [
            (
                "(attr smd board_only)",
                "BoardOnlyFootprint",
                "footprint R7",
            ),
            (
                "(pad \"1\" smd rect (net \"/A_X\") (pintype \"passive+no_connect\"))",
                "NoConnectPadOnNet",
                "pad 1 of footprint R7",
            ),
            (
                "(pad \"1\" smd rect (net \"unconnected-(T-A-Pad1)\")) (pad \"2\" smd rect (net \"unconnected-(T-A-Pad1)\"))",
                "SharedUnconnectedNet",
                "unconnected-(T-A-Pad1)",
            ),
        ];
        for (body, kind, id) in cases {
            let e = board(&fp(body)).unwrap_err();
            assert_eq!((e.kind(), e.id()), (kind, Some(id)), "{body}");
        }
        for body in [
            "(attr smd)",
            "(pad \"1\" smd rect (net \"unconnected-(R7-Pad1)\") (pintype \"passive+no_connect\"))",
            "(pad \"1\" smd rect (pintype \"passive+no_connect\"))",
            "(pad \"1\" smd rect (net \"GND\")) (pad \"2\" smd rect (net \"GND\"))",
        ] {
            assert_eq!(board(&fp(body)), Ok(()), "{body}");
        }
    }
}
