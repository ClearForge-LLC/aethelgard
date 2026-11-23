//! The board under judgement, prepared for DRC by the gate (`RUL-03`,
//! `INV-07`): every rule it carries is refused or replaced, and nothing else
//! in it changes.
//!
//! - The board is read by [`crate::strict`] first (`AET-WO-1306`), and every
//!   judgement below reads that tree.
//! - A `(clearance …)` anywhere outside the top-level `(setup …)` block is
//!   refused by name: a pad's, a footprint's, or a zone's (inside its
//!   `connect_pads`). KiCad judges a locally overridden item only against the
//!   board minimum, never by the gate's custom rules (`AET-WO-1300` S3; ruled
//!   D4 on PR #40, `INV-06`).
//! - A `(net_class …)`, KiCad 5's in-board net class, is refused the same way.
//! - The top-level `(setup …)` block, which can carry a `clearance_min` that
//!   beats the project's, is replaced by the gate's ([`crate::rules::SETUP`]).
//!   A board without one gets it.

use std::rc::Rc;

use crate::sexpr::{self, SexprError};
use crate::strict::{self, Node, Tree};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BoardRefusal {
    #[error("the board is not one KiCad S-expression: {0}")]
    Malformed(#[from] SexprError),
    #[error("the board's top-level list is not `kicad_pcb`")]
    NotABoard,
    #[error("{0}")]
    Strict(strict::Refusal),
    #[error("the board has {0} `setup` blocks; KiCad reads one")]
    SetupTwice(usize),
    #[error(
        "{0} holds a list whose head is not a lowercase keyword, which KiCad would not read as one"
    )]
    OddList(String),
    #[error(
        "{0} carries a local clearance, which KiCad would judge by instead of the gate's rules; remove it"
    )]
    LocalClearance(String),
    #[error("{0} defines a net class inside the board; the gate's project supplies the only one")]
    NetClass(String),
    #[error("{0} waives a DRC check for itself; the gate's rules decide every check")]
    Waiver(String),
}

impl From<strict::Refusal> for BoardRefusal {
    fn from(r: strict::Refusal) -> Self {
        match r {
            strict::Refusal::Unreadable(_, e) => Self::Malformed(e),
            strict::Refusal::NotThisKind(_) => Self::NotABoard,
            r => Self::Strict(r),
        }
    }
}

impl BoardRefusal {
    /// The refusal's `kind` in the CLI's envelope.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Malformed(_) | Self::NotABoard | Self::SetupTwice(_) | Self::OddList(_) => {
                "MalformedBoard"
            }
            Self::Strict(r) => r.kind(),
            Self::LocalClearance(_) => "LocalClearance",
            Self::NetClass(_) => "BoardNetClass",
            Self::Waiver(_) => "BoardWaiver",
        }
    }

    /// What carried the refused rule, such as `pad 1 of footprint R7`, or the
    /// path to the node the strict reader refused.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::LocalClearance(at) | Self::NetClass(at) | Self::Waiver(at) => Some(at),
            Self::Strict(r) => r.id(),
            _ => None,
        }
    }
}

/// What carries an item: the nearest named item around it, and what carries
/// that. Each label is held once and shared by every child scanned inside
/// it, and the chain is rendered only into a refusal (`AET-WO-1322`; `1311`
/// adversarial C: a copy per child took 3.0 GB for a 10 kB reference over
/// 300 000 children).
struct Within {
    label: String,
    footprint: bool,
    parent: Option<Rc<Within>>,
}

/// `within` as a refusal names it, such as `pad 2 of footprint R7`; empty
/// for an item at the top level.
fn render(within: Option<&Within>) -> String {
    let mut out = String::new();
    let mut at = within;
    while let Some(w) = at {
        if !out.is_empty() {
            out.push_str(" of ");
        }
        out.push_str(&w.label);
        at = w.parent.as_deref();
    }
    out
}

/// A human name for an item that can carry a rule, without what carries it.
/// The file's text in it is [`strict::shown`]: as written when short and
/// plain, else quoted and bounded (`AET-WO-1322`; `1311` adversarial J).
fn name(item: &Node<'_>) -> Result<Option<String>, BoardRefusal> {
    use strict::shown;
    Ok(match item.head() {
        Some("footprint" | "module") => {
            let reference = item
                .field("Reference")?
                .or_else(|| item.arg(1))
                .unwrap_or("?");
            Some(format!("footprint {}", shown(reference)))
        }
        Some("pad") => Some(format!("pad {}", shown(item.arg(1).unwrap_or("?")))),
        Some("zone") => {
            let net = item
                .one("net_name")?
                .and_then(|n| n.arg(1))
                .filter(|n| !n.is_empty())
                .map_or_else(
                    || "with no net".to_owned(),
                    |n| format!("on net {}", shown(n)),
                );
            Some(format!("zone {net}"))
        }
        _ => None,
    })
}

/// Whether `item` is a footprint's keepout zone whose `connect_pads` is
/// KiCad's `(connect_pads (clearance <number>))` and nothing else: true only
/// for a zone directly inside a footprint that carries a `keepout`. A
/// `connect_pads` with anything else in it is refused as a local clearance.
fn admitted_keepout(item: &Node<'_>, within: Option<&Within>) -> Result<bool, BoardRefusal> {
    if item.head() != Some("zone")
        || !within.is_some_and(|w| w.footprint && w.parent.is_none())
        || item.one("keepout")?.is_none()
    {
        return Ok(false);
    }
    let Some(c) = item.one("connect_pads")? else {
        return Ok(true);
    };
    let lists: Vec<_> = c.lists().collect();
    let plain = c.texts().count() == 0
        && matches!(lists.as_slice(), [l] if l.head() == Some("clearance")
            && l.len() == 2
            && l.arg(1).is_some_and(|v| crate::sexpr::nm(v).is_some()));
    if plain {
        Ok(true)
    } else {
        Err(BoardRefusal::LocalClearance(format!(
            "the keepout zone of {}",
            render(within)
        )))
    }
}

/// A footprint token that excuses its items from a check (measured on KiCad
/// 10.0.7 by the adversarial pass: `net_tie_pad_groups` turned two clearance,
/// a short and two mask bridges into nothing). Mask and paste margins are not
/// here: they are the design's geometry, which DRC then judges.
fn waives(item: &Node<'_>) -> bool {
    let yes = || item.arg(1) == Some("yes");
    match item.head() {
        Some("net_tie_pad_groups" | "jumper_pad_groups") => true,
        Some(
            "duplicate_pad_numbers_are_jumpers"
            | "allow_missing_courtyard"
            | "allow_soldermask_bridges",
        ) => yes(),
        Some("attr") => item
            .texts()
            .any(|a| matches!(a, "allow_missing_courtyard" | "allow_soldermask_bridges")),
        _ => false,
    }
}

/// Reads the board strictly: the tree every gate after it reads.
pub fn read(text: &str) -> Result<Tree, BoardRefusal> {
    Ok(strict::board(text)?)
}

/// [`read`], then [`prepare_read`].
pub fn prepare(text: &str, setup: &str) -> Result<String, BoardRefusal> {
    prepare_read(&read(text)?, text, setup)
}

/// Refuses a board carrying any rule of its own, judged on `tree` (the
/// strict reading of `text`), then writes `setup` over its setup block. The
/// result differs from `text` only there.
pub fn prepare_read(tree: &Tree, text: &str, setup: &str) -> Result<String, BoardRefusal> {
    let root = tree.root();
    // Depth-first, in document order, without recursion. The top-level
    // setup block is skipped: the gate replaces it whole.
    let mut stack: Vec<(Node<'_>, Option<Rc<Within>>)> = root
        .lists()
        .filter(|c| c.head() != Some("setup"))
        .map(|c| (c, None))
        .collect();
    stack.reverse();
    while let Some((item, within)) = stack.pop() {
        // What carries the item, for a refusal's `id`.
        let at = || match &within {
            None => "the board".to_owned(),
            Some(w) => render(Some(w)),
        };
        // KiCad's keywords are lowercase atoms; a quoted, upper-case or
        // missing head is refused rather than guessed at (`CLEARANCE`, or
        // `("clearance" …)`, measured: KiCad will not load them).
        if !item
            .head()
            .is_some_and(|a| !a.bytes().any(|b| b.is_ascii_uppercase()))
        {
            return Err(BoardRefusal::OddList(at()));
        }
        let here = match name(&item)? {
            Some(label) => Some(Rc::new(Within {
                label,
                footprint: matches!(item.head(), Some("footprint" | "module")),
                parent: within.clone(),
            })),
            None => within.clone(),
        };
        match item.head() {
            // Every clearance is a rule but a custom pad's shape option,
            // `(clearance outline)` or `(clearance convexhull)`, which stays. Any
            // other value, whatever its spelling, is refused.
            Some("clearance")
                if !(item.len() == 2 && matches!(item.arg(1), Some("outline" | "convexhull"))) =>
            {
                return Err(BoardRefusal::LocalClearance(at()));
            }
            Some("net_class") => {
                let class = item.arg(1).unwrap_or("?");
                return Err(BoardRefusal::NetClass(format!(
                    "net class {}",
                    strict::shown(class)
                )));
            }
            Some(token) if waives(&item) => {
                return Err(BoardRefusal::Waiver(format!("{} ({token})", at())));
            }
            _ => {}
        }
        let children: Vec<_> = item.lists().collect();
        let keepout = admitted_keepout(&item, within.as_deref())?;
        for child in children.into_iter().rev() {
            // A vendored footprint's keepout zone carries KiCad's own
            // `(connect_pads (clearance N))`, which binds nothing in a zone
            // that fills nothing; the keepout itself is judged by
            // `route::check` and by KiCad's DRC (`RUL-03`, amended for
            // `AET-WO-4300d2` D2 (a)).
            if keepout && child.head() == Some("connect_pads") {
                continue;
            }
            stack.push((child, here.clone()));
        }
    }
    let (spans, (_, root_end)) = sexpr::top_level_spans(text)?;
    let setups: Vec<(usize, usize)> = root
        .lists()
        .zip(spans)
        .filter(|(c, _)| c.head() == Some("setup"))
        .map(|(_, span)| span)
        .collect();
    let mut out = String::with_capacity(text.len() + setup.len());
    match setups.as_slice() {
        [] => {
            // Before the top-level list's closing parenthesis.
            let close = root_end - 1;
            out.push_str(&text[..close]);
            out.push('\t');
            out.push_str(setup);
            out.push('\n');
            out.push_str(&text[close..]);
        }
        [(start, end)] => {
            out.push_str(&text[..*start]);
            out.push_str(setup);
            out.push_str(&text[*end..]);
        }
        many => return Err(BoardRefusal::SetupTwice(many.len())),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::SETUP;

    const BOARD: &str = "(kicad_pcb\n\t(version 20240108)\n\t(setup\n\t\t(pad_to_mask_clearance 0)\n\t\t(clearance_min 0.05)\n\t\t(trace_clearance 0.05)\n\t)\n\t(net 0 \"\")\n)\n";

    #[test]
    fn the_setup_block_is_replaced_and_nothing_else_changes() {
        let out = prepare(BOARD, SETUP).unwrap();
        assert!(!out.contains("clearance_min") && !out.contains("trace_clearance"));
        assert_eq!(
            out,
            BOARD.replace(
                "(setup\n\t\t(pad_to_mask_clearance 0)\n\t\t(clearance_min 0.05)\n\t\t(trace_clearance 0.05)\n\t)",
                SETUP
            )
        );
    }

    #[test]
    fn a_board_without_setup_gets_the_gates() {
        let out = prepare("(kicad_pcb (version 20240108))\n", SETUP).unwrap();
        let parsed = strict::board(&out).unwrap();
        assert_eq!(parsed.root().all("setup").count(), 1);
        assert!(out.starts_with("(kicad_pcb (version 20240108)\t(setup"));
    }

    #[test]
    fn local_clearances_are_refused_by_name() {
        let cases = [
            (
                "(kicad_pcb (footprint \"R:R\" (property \"Reference\" \"R7\") (pad \"2\" smd rect (clearance 0.05))))",
                "pad 2 of footprint R7",
            ),
            (
                "(kicad_pcb (footprint \"R:R\" (property \"Reference\" \"R7\") (clearance 0.05)))",
                "footprint R7",
            ),
            (
                "(kicad_pcb (zone (net 1) (net_name \"GND\") (connect_pads (clearance 0.5))))",
                "zone on net GND",
            ),
            (
                "(kicad_pcb (footprint \"X\" (zone (net_name \"\") (connect_pads yes (clearance 0.1)))))",
                "zone with no net of footprint X",
            ),
            (
                "(kicad_pcb (module \"X\" (fp_text reference \"U9\" (at 0 0)) (pad 1 smd rect (clearance 0.1))))",
                "pad 1 of footprint U9",
            ),
            ("(kicad_pcb (clearance 0.05))", "the board"),
        ];
        for (board, at) in cases {
            let err = prepare(board, SETUP).unwrap_err();
            assert_eq!(err, BoardRefusal::LocalClearance(at.to_owned()), "{board}");
            assert_eq!(err.kind(), "LocalClearance");
            assert_eq!(err.id(), Some(at));
        }
    }

    #[test]
    fn a_clearance_inside_setup_is_replaced_not_refused() {
        assert!(
            prepare(
                "(kicad_pcb (setup (clearance_min 0.05) (clearance 0.05)))",
                SETUP
            )
            .is_ok()
        );
    }

    #[test]
    fn a_net_class_in_the_board_is_refused() {
        let err = prepare(
            "(kicad_pcb (net_class Default \"x\" (clearance 0.05) (add_net A)))",
            SETUP,
        )
        .unwrap_err();
        assert_eq!(err, BoardRefusal::NetClass("net class Default".into()));
        assert_eq!(err.kind(), "BoardNetClass");
    }

    #[test]
    fn what_is_not_one_board_is_refused() {
        assert_eq!(prepare("(kicad_sch)", SETUP), Err(BoardRefusal::NotABoard));
        assert!(matches!(
            prepare("(kicad_pcb", SETUP),
            Err(BoardRefusal::Malformed(SexprError::Unclosed(1)))
        ));
        assert_eq!(
            prepare("(kicad_pcb (setup) (setup))", SETUP),
            Err(BoardRefusal::SetupTwice(2))
        );
        let deep = format!("(kicad_pcb {}{})", "(a ".repeat(400), ")".repeat(400));
        assert!(matches!(
            prepare(&deep, SETUP),
            Err(BoardRefusal::Malformed(SexprError::TooDeep(_)))
        ));
    }

    #[test]
    fn a_string_that_says_clearance_is_only_text() {
        assert!(prepare("(kicad_pcb (gr_text \"(clearance 0.05)\" (at 0 0)))", SETUP).is_ok());
    }
}
