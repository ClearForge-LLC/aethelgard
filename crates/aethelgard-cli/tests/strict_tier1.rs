//! `AET-WO-1306`: one strict reader for every KiCad file a caller hands a
//! gate. Tier 1: every refusal happens before any tool is looked for, so
//! these run with an empty `PATH`; what the strict reader accepts reaches
//! the tool lookup (`ToolMissing`, exit 4) or, for `extract`, which runs no
//! tool, its verdict. Tier 2 (`board_tier2.rs`) runs KiCad on the rest.

mod common;

use common::Scratch;
use common::corpus::{self, Case};

/// The boards of `AET-WO-1305`'s adversarial corpus (`FINDINGS.md`, 32
/// boards; 31 refused by `extract`, and `user_layer_table_only`, which
/// every gate passes).
const CORPUS_1305: &[&str] = &[
    "arc_on_copper",
    "big_plate",
    "cap_as_pullup",
    "cap_refvalue_only",
    "controller_renamed",
    "dup_end",
    "dup_fp_at",
    "dup_net_seg",
    "dup_pad_at",
    "dup_pad_net",
    "dup_pad_size",
    "dup_thickness",
    "dup_via_size",
    "dup_width",
    "gr_rect_fcu",
    "gr_rect_top",
    "gr_text_copper",
    "huge_coord",
    "island_same_name",
    "pad_circle",
    "pad_circle2",
    "pad_nolayers",
    "pad_offset",
    "pad_starcu",
    "segment_empty_net",
    "segment_no_net",
    "stackup",
    "user_layer_name",
    "user_layer_table_only",
    "via_padstack",
    "zone_clearance",
    "zone_fill_close",
];

fn corpus(scratch: &Scratch) -> (Vec<Case>, String, String) {
    let (board, schematic) = corpus::reference(&scratch.0);
    (corpus::cases(&board, &schematic), board, schematic)
}

/// §3: every corpus file gets, from each gate, the outcome its file states:
/// a strict refusal is exit 3 naming its kind and path, before any tool;
/// a file the reader accepts reaches the tool (exit 4 with no tool on
/// `PATH`), or `extract`'s own verdict.
#[test]
fn every_corpus_file_is_refused_or_reaches_its_gate_as_stated() {
    let scratch = Scratch::new("1306-corpus");
    let (cases, board, schematic) = corpus(&scratch);
    let mut refused = 0;
    for case in &cases {
        for e in &case.expect {
            let label = format!("{}/{} {}", case.class, case.name, e.gate);
            let (code, v) = corpus::gate(case, &e.gate, &scratch.0, (&board, &schematic), Some(""));
            let (what, id) = corpus::outcome(&v);
            if e.code == 3 || e.gate == "extract" {
                assert_eq!(
                    (code, what, id),
                    (e.code, e.what.clone(), e.id.clone()),
                    "{label}: {v}"
                );
                refused += usize::from(e.code == 3);
            } else {
                assert_eq!((code, what.as_str()), (4, "ToolMissing"), "{label}: {v}");
            }
        }
    }
    assert!(refused > 0);
}

/// §3: `dup_thickness` is refused by `check drc`, exit 3, naming the path;
/// main passed it. Stated here as well as in its file, so a file edited to
/// say otherwise is caught.
#[test]
fn dup_thickness_is_refused_by_check_drc_naming_the_path() {
    let scratch = Scratch::new("1306-thickness");
    let (cases, board, schematic) = corpus(&scratch);
    let case = cases.iter().find(|c| c.name == "dup_thickness").unwrap();
    for gate in ["drc", "drc-parity", "extract"] {
        let (code, v) = corpus::gate(case, gate, &scratch.0, (&board, &schematic), Some(""));
        assert_eq!(code, 3, "{gate}: {v}");
        assert_eq!(
            corpus::outcome(&v),
            (
                "MalformedBoard".to_owned(),
                Some("kicad_pcb/general[1]".to_owned())
            ),
            "{gate}"
        );
    }
}

/// §1.4: the corpus holds all of 1305's boards, each file named by its
/// class; every repeated child and every field named twice is refused by
/// every gate reading that kind; a schematic counterpart exists for each
/// class that applies to a schematic.
#[test]
fn the_corpus_is_1305s_and_its_schematic_counterparts_by_class() {
    let scratch = Scratch::new("1306-classes");
    let (cases, _, _) = corpus(&scratch);
    for name in CORPUS_1305 {
        assert!(
            cases.iter().any(|c| c.board && c.name == *name),
            "{name} is missing"
        );
    }
    for case in &cases {
        if matches!(
            case.class.as_str(),
            "repeated-child" | "field-twice" | "field-spelling"
        ) {
            let kind = if case.board {
                "MalformedBoard"
            } else {
                "MalformedSchematic"
            };
            for e in &case.expect {
                assert_eq!(
                    (e.code, e.what.as_str()),
                    (3, kind),
                    "{}/{} {}",
                    case.class,
                    case.name,
                    e.gate
                );
                assert!(e.id.is_some(), "{}: names no path", case.name);
            }
        }
    }
    for class in ["repeated-child", "field-twice", "control"] {
        assert!(
            cases.iter().any(|c| !c.board && c.class == class),
            "no schematic {class}"
        );
    }
    let control = cases
        .iter()
        .find(|c| c.name == "user_layer_table_only")
        .unwrap();
    assert!(
        control.expect.iter().all(|e| e.code == 0),
        "the control must pass"
    );
}

/// §3: the reference board and schematic pass the strict reader and reach
/// every gate's tool, and `extract` passes the board.
#[test]
fn the_reference_reaches_every_gate() {
    let scratch = Scratch::new("1306-reference");
    let (board, schematic) = corpus::reference(&scratch.0);
    assert!(aethelgard_eda::strict::board(&board).is_ok());
    assert!(aethelgard_eda::strict::schematic(&schematic).is_ok());
    let reference = Case {
        board: true,
        class: "control".into(),
        name: "reference".into(),
        text: board.clone(),
        expect: vec![],
    };
    for gate in ["drc", "drc-parity"] {
        let (code, v) = corpus::gate(&reference, gate, &scratch.0, (&board, &schematic), Some(""));
        assert_eq!(
            (code, corpus::outcome(&v).0.as_str()),
            (4, "ToolMissing"),
            "{gate}: {v}"
        );
    }
    let (code, v) = corpus::gate(
        &reference,
        "extract",
        &scratch.0,
        (&board, &schematic),
        Some(""),
    );
    assert_eq!((code, corpus::outcome(&v).0.as_str()), (0, "pass"), "{v}");
}
