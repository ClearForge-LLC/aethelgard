//! AET-WO-1300 §5, the adversarial pass on the DRC gate's board preparation,
//! as regression tests. Every `attack_*` case was measured in the tools
//! image (KiCad 10.0.7, `--network none`): KiCad honoured the hidden rule and,
//! before the fix, the gate's verdict was `pass` on a board with a 0.05 mm
//! violation. Each must now be refused before KiCad runs.

use aethelgard_eda::board::{BoardRefusal, prepare};
use aethelgard_eda::rules::SETUP;
use aethelgard_eda::sexpr::SexprError;

fn refused(board: &str) -> BoardRefusal {
    prepare(board, SETUP).expect_err(board)
}

// ---- the gate's reader and KiCad's must not disagree ----

/// KiCad skips a line whose first non-blank byte is `#`. With `#"` lines the
/// board's own setup and a pad clearance were a string to the gate, and
/// rules to KiCad.
#[test]
fn attack_a_hash_comment_line_is_refused() {
    let board = "(kicad_pcb\n\t(version 20241229)\n#\"\n\t(setup (pad_to_mask_clearance 0) (clearance_min 0.01))\n#\"\n\t(footprint \"R:R\"\n#\"\n\t\t(pad 1 smd rect (at 0 0) (size 1 1) (layers F.Cu) (clearance 0.01))\n#\"\n\t)\n)\n";
    assert!(matches!(
        refused(board),
        BoardRefusal::Malformed(SexprError::CommentLine(_))
    ));
    let nest = "(kicad_pcb\n\t(version 20241229)\n  #(footprint \"R:R\"\n\t(setup (clearance_min 0.01))\n#)\n)\n";
    assert!(matches!(
        refused(nest),
        BoardRefusal::Malformed(SexprError::CommentLine(_))
    ));
}

/// KiCad reads NUL as whitespace; the gate read it as part of an atom, so
/// `(\0setup …)` and `(\0clearance 0.01)` reached KiCad unseen. VT, FF, SOH:
/// KiCad will not load them, and they are refused here too.
#[test]
fn attack_a_control_byte_is_refused() {
    for b in ['\0', '\u{0b}', '\u{0c}', '\u{01}', '\u{1f}', '\u{7f}'] {
        for board in [
            format!("(kicad_pcb (footprint \"R:R\" (pad \"1\" smd rect ({b}clearance 0.01))))"),
            format!("(kicad_pcb ({b}setup (clearance_min 0.01)))"),
            format!("(kicad_pcb (setup{b}(clearance_min 0.01)))"),
            format!("(kicad_pcb (gr_text \"a{b}b\" (at 0 0)))"),
        ] {
            assert!(
                matches!(
                    refused(&board),
                    BoardRefusal::Malformed(
                        SexprError::ForbiddenByte(_) | SexprError::ControlInString(_)
                    )
                ),
                "{board:?}"
            );
        }
    }
}

/// KiCad keeps a `"` inside an atom; the gate ended the atom and opened a
/// string there, hiding `(clearance 0.01)` between two of them.
#[test]
fn attack_a_quote_touching_an_atom_is_refused() {
    for board in [
        "(kicad_pcb (footprint \"R:R\" (descr x\"y) (clearance 0.01) (descr z\"w)))",
        "(kicad_pcb (descr \"y\"x))",
        // Only the opening quote touches the atom here.
        "(kicad_pcb (descr x\"y\"))",
    ] {
        assert!(
            matches!(
                refused(board),
                BoardRefusal::Malformed(SexprError::QuoteInAtom(_))
            ),
            "{board}"
        );
    }
}

#[test]
fn a_raw_line_break_inside_a_string_and_non_ascii_outside_one_are_refused() {
    assert!(matches!(
        refused("(kicad_pcb (gr_text \"a\nb\" (at 0 0)))"),
        BoardRefusal::Malformed(SexprError::ControlInString(_))
    ));
    // A control byte escaped by a backslash is still a control byte.
    for escaped in ["\\\n", "\\\0", "\\\u{7f}"] {
        let board = format!("(kicad_pcb (gr_text \"a{escaped}b\" (at 0 0)))");
        assert!(
            matches!(
                refused(&board),
                BoardRefusal::Malformed(SexprError::ControlInString(_))
            ),
            "{board:?}"
        );
    }
    assert!(matches!(
        refused("(kicad_pcb (gr_text \u{e9} (at 0 0)))"),
        BoardRefusal::Malformed(SexprError::ForbiddenByte(_))
    ));
    // Non-ASCII inside a string is text, as KiCad writes it.
    assert!(prepare("(kicad_pcb (gr_text \"caf\u{e9}\" (at 0 0)))", SETUP).is_ok());
}

/// KiCad will not load an upper-case or quoted keyword; the gate refuses
/// them itself rather than relying on that.
#[test]
fn an_upper_case_or_quoted_keyword_is_refused() {
    for board in [
        "(kicad_pcb (footprint \"R:R\" (pad \"1\" smd rect (CLEARANCE 0.01))))",
        "(kicad_pcb (footprint \"R:R\" (pad \"1\" smd rect (Clearance 0.01))))",
        "(kicad_pcb (footprint \"R:R\" (pad \"1\" smd rect (\"clearance\" 0.01))))",
        "(kicad_pcb (NET_CLASS Default \"\" (clearance 0.01)))",
        "(kicad_pcb (\"setup\" (clearance_min 0.01)))",
        "(kicad_pcb (SETUP (clearance_min 0.01)))",
        "(kicad_pcb ())",
    ] {
        assert!(
            matches!(refused(board), BoardRefusal::OddList(_)),
            "{board}"
        );
    }
}

// ---- footprint tokens that waive a check ----

/// Measured: `net_tie_pad_groups` turned two clearance, a short and two mask
/// bridges into no finding; `allow_soldermask_bridges` removed a mask bridge.
#[test]
fn attack_a_footprint_that_waives_a_check_is_refused() {
    for (board, token) in [
        (
            "(kicad_pcb (footprint \"X:NT\" (property \"Reference\" \"NT1\") (net_tie_pad_groups \"1,2\")))",
            "net_tie_pad_groups",
        ),
        (
            "(kicad_pcb (footprint \"X:J\" (jumper_pad_groups (\"1\" \"2\"))))",
            "jumper_pad_groups",
        ),
        (
            "(kicad_pcb (footprint \"X:J\" (duplicate_pad_numbers_are_jumpers yes)))",
            "duplicate_pad_numbers_are_jumpers",
        ),
        (
            "(kicad_pcb (footprint \"X:A\" (attr smd allow_missing_courtyard)))",
            "attr",
        ),
        (
            "(kicad_pcb (footprint \"X:A\" (attr smd allow_soldermask_bridges)))",
            "attr",
        ),
        (
            "(kicad_pcb (footprint \"X:A\" (allow_missing_courtyard yes)))",
            "allow_missing_courtyard",
        ),
        (
            "(kicad_pcb (footprint \"X:A\" (allow_soldermask_bridges yes)))",
            "allow_soldermask_bridges",
        ),
    ] {
        match refused(board) {
            BoardRefusal::Waiver(at) => assert!(at.ends_with(&format!("({token})")), "{at}"),
            other => panic!("{board}: {other:?}"),
        }
    }
    // The vendored footprints all say `no`, which waives nothing.
    assert!(
        prepare(
            "(kicad_pcb (footprint \"X\" (duplicate_pad_numbers_are_jumpers no) (attr smd)))",
            SETUP
        )
        .is_ok()
    );
}

/// Mask and paste margins are the design's geometry, which DRC judges; they
/// are not waivers, and stay (FEEDBACK).
#[test]
fn mask_and_paste_margins_are_geometry_and_stay() {
    assert!(prepare("(kicad_pcb (footprint \"X:M\" (solder_mask_margin -0.5) (pad \"1\" smd rect (solder_mask_margin -0.5) (solder_paste_margin -0.5))))", SETUP).is_ok());
}

/// A custom pad's `(options (clearance outline))` is its shape, not a rule;
/// every other clearance value, whatever its spelling, is refused.
#[test]
fn a_custom_pad_clearance_option_stays_and_every_other_clearance_is_refused() {
    for shape in ["outline", "convexhull"] {
        let board = format!(
            "(kicad_pcb (footprint \"X:C\" (pad \"1\" smd custom (options (clearance {shape}) (anchor rect)) (primitives))))"
        );
        assert!(prepare(&board, SETUP).is_ok(), "{board}");
    }
    for value in ["0.01", "5e-2", "1E-2", ".01", "+0.01", "outline 0.01", "x"] {
        let board =
            format!("(kicad_pcb (footprint \"X:C\" (pad \"1\" smd rect (clearance {value}))))");
        assert!(
            matches!(refused(&board), BoardRefusal::LocalClearance(_)),
            "{board}"
        );
    }
}

// ---- withstood at the first pass, kept ----

#[test]
fn a_setup_net_class_or_padstack_clearance_nested_anywhere_is_scanned() {
    assert!(matches!(
        refused("(kicad_pcb (footprint \"X\" (setup (clearance 0.01))))"),
        BoardRefusal::LocalClearance(_)
    ));
    assert!(matches!(
        refused("(kicad_pcb (footprint \"X\" (net_class A \"\" (add_net B))))"),
        BoardRefusal::NetClass(_)
    ));
    assert!(matches!(
        refused(
            "(kicad_pcb (footprint \"X\" (pad \"1\" thru_hole circle (padstack (mode custom) (layer \"In1.Cu\" (clearance 0.01))))))"
        ),
        BoardRefusal::LocalClearance(_)
    ));
}

#[test]
fn escapes_inside_strings_do_not_desync_the_scan() {
    // `\"` keeps the string open, as in KiCad: the clearance is only text.
    assert!(
        prepare(
            "(kicad_pcb (gr_text \"a\\\" (clearance 0.01) \\\"b\" (at 0 0)))",
            SETUP
        )
        .is_ok()
    );
    // `\\` before the quote closes it, as in KiCad: the clearance is a list.
    assert!(matches!(
        refused(
            "(kicad_pcb (gr_text \"a\\\\\" (footprint \"X\" (pad \"1\" smd rect (clearance 0.01)))))"
        ),
        BoardRefusal::LocalClearance(_)
    ));
}

#[test]
fn trailing_data_a_bom_and_a_second_top_level_list_are_refused() {
    for board in [
        "(kicad_pcb (version 1)) (setup (clearance_min 0.01))",
        "(kicad_pcb (version 1)) x",
        "\u{feff}(kicad_pcb (version 1))",
        "(setup (clearance_min 0.01)) (kicad_pcb (version 1))",
    ] {
        assert!(
            matches!(refused(board), BoardRefusal::Malformed(_)),
            "{board:?}"
        );
    }
}

/// 16 MiB (the input cap) of `()` cost the reader 663 MiB before the list
/// cap; now it is refused, in bounded time and memory.
#[test]
fn a_board_of_millions_of_tokens_is_refused_quickly() {
    let mut board = String::from("(kicad_pcb ");
    while board.len() < 16 * 1024 * 1024 - 2 {
        board.push_str("(a)");
    }
    board.push(')');
    let t = std::time::Instant::now();
    assert!(matches!(
        refused(&board),
        BoardRefusal::Malformed(SexprError::TooManyTokens)
    ));
    assert!(t.elapsed().as_secs() < 30, "{:?}", t.elapsed());
}
