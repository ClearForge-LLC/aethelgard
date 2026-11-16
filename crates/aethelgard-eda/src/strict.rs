//! The one strict reader for every KiCad file a caller hands a gate
//! (`AET-WO-1306`): `check drc` (with or without `--spec` and
//! `--schematic`), `check erc`, `extract`, and `verify`'s copies of each.
//!
//! **Why.** [`Sx::child`](crate::sexpr::Sx::child) returns a repeated
//! child's first match, but KiCad 10.0.7 takes the last: its parser sets a
//! field once per child it meets, so a second `(thickness 0.2)` overwrites
//! the first (`pcb_io_kicad_sexpr_parser.cpp`, `parseGeneralSection`). A
//! gate that read the first judged one board while KiCad, and the
//! fabricator after it, used another (`AET-WO-1305` F6).
//!
//! **What it refuses**, before any tool runs and before any judgement:
//! - what [`sexpr::parse`] refuses (every byte sequence KiCad could tokenise
//!   otherwise), and a file whose top-level list is not the kind expected;
//! - in every node a gate reads ([`RULES`]), a second child that sets a
//!   value KiCad holds once, whichever spelling sets it (`tstamp` and
//!   `uuid`; a footprint's `layer` and `layers`). It is refused, never
//!   resolved first-or-last;
//! - a field named twice: a footprint's `property` and its legacy
//!   `fp_text reference`/`fp_text value` set one field, compared exactly as
//!   `FOOTPRINT::GetField` does; a symbol's too;
//! - a schematic field spelling a mandatory name otherwise than KiCad's
//!   canonical one (`"VALUE"`): `parseSchField` folds a mandatory name
//!   case-insensitively, so KiCad reads it as the mandatory field, and a
//!   gate comparing names exactly would not. With every mandatory name
//!   canonical, exact comparison is KiCad's, so a symbol's fields need no
//!   folding to be found twice.
//!
//! - a value of a numeric node ([`NUMERIC`], [`KEYED`] past its keyword)
//!   that is not a bare atom of
//!   `-?[0-9]+(\.[0-9]{1,6})?` (`AET-WO-3303` A11): quoted, `+`, `--`, an
//!   exponent, a leading `.`, a seventh decimal. [`version`] separately pins
//!   a file's `(version …)` for the gate that judges it alone (`extract`).
//!
//! Each refusal is `MalformedBoard` or `MalformedSchematic` and names the
//! path to the node (`INV-06`).
//!
//! **What a gate gets.** A [`Tree`], which only this module produces, and
//! [`Node`]s inside it. A `Node` has no first-match lookup: [`Node::one`]
//! refuses a second match, so nothing a gate reads through it is resolved
//! first-or-last, whatever [`RULES`] states. An unknown child is not
//! refused here: whether one matters depends on the gate (`extract`, whose
//! bound depends on every child of the copper it reads, refuses them; KiCad
//! judges them for DRC and ERC).

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::sexpr::{self, SexprError, Sx};

/// Which of KiCad's files a tree is: a board, a schematic, or the netlist
/// `kicad-cli sch export netlist --format kicadsexpr` writes of a schematic
/// (`AET-WO-4300g`: the support gate's input).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Board,
    Schematic,
    Netlist,
}

impl Kind {
    fn root(self) -> &'static str {
        match self {
            Kind::Board => "kicad_pcb",
            Kind::Schematic => "kicad_sch",
            Kind::Netlist => "export",
        }
    }
    fn noun(self) -> &'static str {
        match self {
            Kind::Board => "board",
            Kind::Schematic => "schematic",
            Kind::Netlist => "netlist",
        }
    }
}

/// Why the strict reader refused a file.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("the {noun} is not one KiCad S-expression: {cause}", noun = .0.noun(), cause = .1)]
    Unreadable(Kind, SexprError),
    #[error("the {noun}'s top-level list is not `{root}`", noun = .0.noun(), root = .0.root())]
    NotThisKind(Kind),
    #[error(
        "{path} has `{child}` after `{first}`, which sets the same value; KiCad reads the last, so neither is read"
    )]
    Repeated {
        kind: Kind,
        path: String,
        first: String,
        child: String,
    },
    #[error("{path} names the field {name:?} twice; KiCad keeps one, so neither is read")]
    FieldTwice {
        kind: Kind,
        path: String,
        name: String,
    },
    #[error(
        "{path} spells the mandatory field {canonical:?} as {name:?}, which KiCad reads as {canonical:?}"
    )]
    FieldSpelling {
        kind: Kind,
        path: String,
        name: String,
        canonical: &'static str,
    },
    #[error("{path} quotes its keyword {word:?}, which KiCad does not read as one")]
    Quoted {
        kind: Kind,
        path: String,
        word: String,
    },
    #[error("{path} has `{child}` twice where the reader takes exactly one")]
    Ambiguous {
        kind: Kind,
        path: String,
        child: String,
    },
    #[error(
        "{path} gives {text:?} where KiCad reads a number; a number is `-?[0-9]+(.[0-9]{{1,6}})?`, bare"
    )]
    Number {
        kind: Kind,
        path: String,
        text: String,
    },
    #[error("{path} is version {found}; KiCad 10.0.7 writes {want}, and no other is read")]
    Version {
        kind: Kind,
        path: String,
        found: String,
        want: &'static str,
    },
    #[error(
        "{path} holds a list whose keyword is {bytes} bytes long; KiCad's keywords are short, and the reader takes none longer than {MAX_KEYWORD_BYTES}"
    )]
    Keyword {
        kind: Kind,
        path: String,
        bytes: usize,
    },
    #[error(
        "{path} names the layer {name:?}, which is not a canonical name in the board's layer table{wildcards}"
    )]
    Layer {
        kind: Kind,
        path: String,
        name: String,
        wildcards: String,
    },
}

impl Refusal {
    fn file(&self) -> Kind {
        match self {
            Self::Unreadable(k, _) | Self::NotThisKind(k) => *k,
            Self::Repeated { kind, .. }
            | Self::FieldTwice { kind, .. }
            | Self::FieldSpelling { kind, .. }
            | Self::Quoted { kind, .. }
            | Self::Number { kind, .. }
            | Self::Version { kind, .. }
            | Self::Keyword { kind, .. }
            | Self::Layer { kind, .. }
            | Self::Ambiguous { kind, .. } => *kind,
        }
    }

    /// The refusal's `kind` in the CLI's envelope.
    pub fn kind(&self) -> &'static str {
        match self.file() {
            Kind::Board => "MalformedBoard",
            Kind::Schematic => "MalformedSchematic",
            Kind::Netlist => "MalformedNetlist",
        }
    }

    /// The path to the refused node, such as
    /// `kicad_pcb/footprint[3]/pad[2]`; none when the file as a whole is.
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Unreadable(..) | Self::NotThisKind(_) => None,
            Self::Repeated { path, .. }
            | Self::FieldTwice { path, .. }
            | Self::FieldSpelling { path, .. }
            | Self::Quoted { path, .. }
            | Self::Number { path, .. }
            | Self::Version { path, .. }
            | Self::Keyword { path, .. }
            | Self::Layer { path, .. }
            | Self::Ambiguous { path, .. } => Some(path),
        }
    }
}

/// How a node's fields are named, if it has any a gate reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fields {
    None,
    /// `property "Name"`, plus `fp_text reference` and `fp_text value`,
    /// compared exactly (`FOOTPRINT::GetField`).
    Footprint,
    /// `property "Name"`, compared exactly: a mandatory name KiCad would
    /// fold (`parseSchField`) is refused by its spelling first.
    Symbol,
}

/// One node a gate reads: where it sits (its parent's head), and the groups
/// of children that each set one value KiCad holds once. Every child is
/// established from KiCad 10.0.7's parser (gitlab tag `10.0.7`,
/// `pcb_io_kicad_sexpr_parser.cpp` and `sch_io_kicad_sexpr_parser.cpp`) as
/// a setter, which keeps the last of several; a child the parser appends
/// (a footprint's `pad`, a zone's `polygon`, a footprint's `attr`, whose
/// flags accumulate) is in no group and may repeat.
struct Rule {
    kind: Kind,
    parents: &'static [&'static str],
    node: &'static [&'static str],
    once: &'static [&'static [&'static str]],
    fields: Fields,
    /// Positions that must be bare atoms: KiCad reads a keyword there, and a
    /// quoted string is not one (`parsePAD` reads `smd` or `roundrect` as
    /// tokens, and will not load `"smd"`, which a reader of text takes as
    /// `smd`).
    keywords: &'static [usize],
}

const B: Kind = Kind::Board;
const S: Kind = Kind::Schematic;
const N: Kind = Kind::Netlist;

/// The nodes the gates read, with what each may hold once. A node absent
/// here is read by no gate; its children are KiCad's alone.
const RULES: &[Rule] = &[
    // A netlist (KiCad 10.0.7, `netlist_exporter_kicad.cpp`): each section,
    // each component's reference and value, each net's name, each node's
    // reference and pin are written once, and the support gate reads them.
    Rule {
        kind: N,
        parents: &[""],
        node: &["export"],
        once: &[
            &["version"],
            &["design"],
            &["components"],
            &["libparts"],
            &["libraries"],
            &["nets"],
        ],
        fields: Fields::None,
        keywords: &[],
    },
    Rule {
        kind: N,
        parents: &["components"],
        node: &["comp"],
        once: &[&["ref"], &["value"], &["footprint"], &["libsource"]],
        fields: Fields::None,
        keywords: &[],
    },
    Rule {
        kind: N,
        parents: &["nets"],
        node: &["net"],
        once: &[&["code"], &["name"], &["class"]],
        fields: Fields::None,
        keywords: &[],
    },
    Rule {
        kind: N,
        parents: &["net"],
        node: &["node"],
        once: &[&["ref"], &["pin"], &["pinfunction"], &["pintype"]],
        fields: Fields::None,
        keywords: &[],
    },
    // parseBOARD_unchecked, parseHeader: each top-level section sets the
    // board's one copy.
    Rule {
        kind: B,
        parents: &[""],
        node: &["kicad_pcb"],
        once: &[
            &["version"],
            &["generator"],
            &["generator_version"],
            &["general"],
            &["paper", "page"],
            &["title_block"],
            &["layers"],
            &["embedded_fonts"],
        ],
        fields: Fields::None,
        keywords: &[],
    },
    // parseGeneralSection.
    Rule {
        kind: B,
        parents: &["kicad_pcb"],
        node: &["general"],
        once: &[&["thickness"], &["legacy_teardrops"]],
        fields: Fields::None,
        keywords: &[],
    },
    // parsePCB_TRACK: `layer` and `layers` both set the track's layers.
    Rule {
        kind: B,
        parents: &["kicad_pcb"],
        node: &["segment"],
        once: &[
            &["start"],
            &["end"],
            &["width"],
            &["layer", "layers"],
            &["solder_mask_margin"],
            &["net"],
            &["uuid", "tstamp"],
            &["status"],
            &["locked"],
        ],
        fields: Fields::None,
        keywords: &[],
    },
    // parsePCB_VIA.
    Rule {
        kind: B,
        parents: &["kicad_pcb"],
        node: &["via"],
        once: &[
            &["at"],
            &["size"],
            &["drill"],
            &["layers"],
            &["net"],
            &["remove_unused_layers"],
            &["keep_end_layers"],
            &["start_end_only"],
            &["zone_layer_connections"],
            &["padstack"],
            &["teardrops"],
            &["tenting"],
            &["covering"],
            &["plugging"],
            &["filling"],
            &["capping"],
            &["uuid", "tstamp"],
            &["status"],
            &["locked"],
            &["free"],
            &["backdrill"],
            &["tertiary_drill"],
            &["front_post_machining"],
            &["back_post_machining"],
        ],
        fields: Fields::None,
        keywords: &[],
    },
    // parsePCB_SHAPE, as far as a `gr_rect` goes.
    Rule {
        kind: B,
        parents: &["kicad_pcb"],
        node: &["gr_rect"],
        once: &[
            &["start"],
            &["end"],
            &["layer", "layers"],
            &["solder_mask_margin"],
            &["width", "stroke"],
            &["uuid", "tstamp"],
            &["fill"],
            &["locked"],
            &["net"],
        ],
        fields: Fields::None,
        keywords: &[],
    },
    Rule {
        kind: B,
        parents: &["gr_rect"],
        node: &["stroke"],
        once: &[&["width"], &["type"], &["color"]],
        fields: Fields::None,
        keywords: &[],
    },
    // parseFOOTPRINT_unchecked. `module` is KiCad 5's name for it.
    Rule {
        kind: B,
        parents: &["kicad_pcb"],
        node: &["footprint", "module"],
        once: &[
            &["version"],
            &["generator"],
            &["generator_version"],
            &["locked"],
            &["placed"],
            &["layer"],
            &["tedit"],
            &["uuid", "tstamp"],
            &["at"],
            &["descr"],
            &["tags"],
            &["path"],
            &["sheetname"],
            &["sheetfile"],
            &["units"],
            &["private_layers"],
            &["net_tie_pad_groups"],
            &["duplicate_pad_numbers_are_jumpers"],
            &["jumper_pad_groups"],
            &["solder_mask_margin"],
            &["solder_paste_margin"],
            &["solder_paste_margin_ratio", "solder_paste_ratio"],
            &["clearance"],
            &["zone_connect"],
            &["thermal_width"],
            &["thermal_gap"],
            &["stackup"],
            &["embedded_fonts"],
            &["embedded_files"],
            &["component_classes"],
        ],
        fields: Fields::Footprint,
        keywords: &[],
    },
    // parsePAD. A pad's `property` sets its one fabrication property.
    Rule {
        kind: B,
        parents: &["footprint", "module"],
        node: &["pad"],
        once: &[
            &["size"],
            &["at"],
            &["rect_delta"],
            &["drill"],
            &["backdrill"],
            &["tertiary_drill"],
            &["layers"],
            &["net"],
            &["pinfunction"],
            &["pintype"],
            &["die_length"],
            &["die_delay"],
            &["solder_mask_margin"],
            &["solder_paste_margin"],
            &["solder_paste_margin_ratio"],
            &["clearance"],
            &["teardrops"],
            &["zone_connect"],
            &["thermal_width", "thermal_bridge_width"],
            &["thermal_bridge_angle"],
            &["thermal_gap"],
            &["roundrect_rratio"],
            &["chamfer_ratio"],
            &["chamfer"],
            &["property"],
            &["options"],
            &["padstack"],
            &["primitives"],
            &["remove_unused_layers"],
            &["keep_end_layers"],
            &["tenting"],
            &["zone_layer_connections"],
            &["locked"],
            &["uuid", "tstamp"],
            &["front_post_machining"],
            &["back_post_machining"],
        ],
        fields: Fields::None,
        keywords: &[2, 3],
    },
    // parseZONE. A zone's `polygon` and `filled_polygon` are appended.
    Rule {
        kind: B,
        parents: &["kicad_pcb", "footprint", "module"],
        node: &["zone"],
        once: &[
            &["net"],
            &["net_name"],
            &["layer", "layers"],
            &["uuid", "tstamp"],
            &["hatch"],
            &["priority"],
            &["connect_pads"],
            &["min_thickness"],
            &["filled_areas_thickness"],
            &["fill"],
            &["placement"],
            &["keepout"],
            &["name"],
            &["attr"],
            &["locked"],
        ],
        fields: Fields::None,
        keywords: &[],
    },
    Rule {
        kind: B,
        parents: &["zone"],
        node: &["connect_pads"],
        once: &[&["clearance"]],
        fields: Fields::None,
        keywords: &[],
    },
    Rule {
        kind: B,
        parents: &["zone"],
        node: &["fill"],
        once: &[
            &["mode"],
            &["hatch_thickness"],
            &["hatch_gap"],
            &["hatch_orientation"],
            &["hatch_smoothing_level"],
            &["hatch_smoothing_value"],
            &["hatch_border_algorithm"],
            &["hatch_min_hole_area"],
            &["arc_segments"],
            &["thermal_gap"],
            &["thermal_bridge_width"],
            &["smoothing"],
            &["radius"],
            &["island_removal_mode"],
            &["island_area_min"],
        ],
        fields: Fields::None,
        keywords: &[],
    },
    // ParseSchematic.
    Rule {
        kind: S,
        parents: &[""],
        node: &["kicad_sch"],
        once: &[
            &["version"],
            &["generator"],
            &["generator_version"],
            &["uuid"],
            &["paper", "page"],
            &["title_block"],
            &["lib_symbols"],
            &["embedded_fonts"],
        ],
        fields: Fields::None,
        keywords: &[],
    },
    // parseSchematicSymbol: a placed symbol.
    Rule {
        kind: S,
        parents: &["kicad_sch"],
        node: &["symbol"],
        once: &[
            &["lib_name"],
            &["lib_id"],
            &["at"],
            &["mirror"],
            &["unit"],
            &["convert", "body_style"],
            &["exclude_from_sim"],
            &["in_bom"],
            &["on_board"],
            &["in_pos_files"],
            &["dnp"],
            &["fields_autoplaced"],
            &["uuid"],
            &["default_instance"],
            &["instances"],
        ],
        fields: Fields::Symbol,
        keywords: &[],
    },
    // parseLibSymbol: a symbol in the schematic's own library, and its
    // units, whose fields name nets too (a power symbol's `Value`).
    Rule {
        kind: S,
        parents: &["lib_symbols", "symbol"],
        node: &["symbol"],
        once: &[],
        fields: Fields::Symbol,
        keywords: &[],
    },
];

/// A symbol's mandatory fields, by KiCad's canonical names.
const MANDATORY: &[&str] = &[
    "Reference",
    "Value",
    "Footprint",
    "Datasheet",
    "Description",
];

fn rule(kind: Kind, parent: &str, head: &str) -> Option<&'static Rule> {
    RULES
        .iter()
        .find(|r| r.kind == kind && r.node.contains(&head) && r.parents.contains(&parent))
}

/// The `i`th element of `node` as a number in nanometres: a bare atom of
/// KiCad's number grammar ([`sexpr::nm`]), never a quoted string. (A free
/// function, not a `Node` method: the fence pins `Node`'s lookups.)
pub fn number(node: &Node<'_>, i: usize) -> Option<i64> {
    match node.sx.items().get(i)? {
        Sx::Atom(a) => sexpr::nm(a),
        _ => None,
    }
}

/// A copper layer by KiCad's canonical name: `F.Cu`, `B.Cu`, `In1.Cu` to
/// `In30.Cu`. A layer's user name, which a `(layers …)` row may give and
/// KiCad then accepts for the layer, is never one (`AET-WO-3303` A11).
pub fn copper_layer(name: &str) -> bool {
    match name {
        "F.Cu" | "B.Cu" => true,
        _ => name
            .strip_prefix("In")
            .and_then(|n| n.strip_suffix(".Cu"))
            .is_some_and(|n| {
                !n.starts_with('0')
                    && n.bytes().all(|b| b.is_ascii_digit())
                    && n.parse::<u8>().is_ok_and(|n| (1..=30).contains(&n))
            }),
    }
}

/// A pad's copper layer: [`copper_layer`] or the `*.Cu` wildcard.
pub fn copper_pad_layer(name: &str) -> bool {
    name == "*.Cu" || copper_layer(name)
}

/// A file the strict reader accepted. Only this module makes one.
pub struct Tree {
    root: Sx,
    kind: Kind,
}

/// Reads a `.kicad_pcb`'s text strictly.
pub fn board(text: &str) -> Result<Tree, Refusal> {
    read(text, Kind::Board)
}

/// Reads a `.kicad_sch`'s text strictly.
pub fn schematic(text: &str) -> Result<Tree, Refusal> {
    read(text, Kind::Schematic)
}

/// The netlist format KiCad 10.0.7 writes (`netlist_exporter_kicad.cpp`).
pub const NETLIST_VERSION: &str = "E";

/// Reads a KiCad netlist's text (`--format kicadsexpr`) strictly.
pub fn netlist(text: &str) -> Result<Tree, Refusal> {
    read(text, Kind::Netlist)
}

/// Judges a footprint this engine placed (`route::part_of`'s caller in
/// `pcb`) as if it sat at the top of a board.
pub fn footprint(fp: &Sx) -> Result<Tree, Refusal> {
    let tree = Tree {
        root: Sx::List(vec![Sx::atom("kicad_pcb"), fp.clone()]),
        kind: Kind::Board,
    };
    check(&tree)?;
    Ok(tree)
}

fn read(text: &str, kind: Kind) -> Result<Tree, Refusal> {
    let root = sexpr::parse(text).map_err(|e| Refusal::Unreadable(kind, e))?;
    if root.head() != Some(kind.root()) {
        return Err(Refusal::NotThisKind(kind));
    }
    let tree = Tree { root, kind };
    check(&tree)?;
    Ok(tree)
}

/// The wildcards KiCad 10.0.7 reads in a pad's `(layers …)`, and only
/// there, that the vendored footprints use (`AET-WO-1311` R5): every copper
/// layer and both masks.
pub const PAD_WILDCARDS: &[&str] = &["*.Cu", "*.Mask"];

/// Every layer an item names is one of the board's own: a canonical name in
/// its `(layers …)` table, or on a pad one of [`PAD_WILDCARDS`]
/// (`AET-WO-3303` F4, `AET-WO-1311` §1.1). KiCad refuses to load a board
/// naming a layer it does not know (`"F&B.Cu"` or `"Top"` on a footprint
/// line, measured by `3303`), so a gate that reads it alone would pass a
/// board no tool can; and a row's user name (`"F.Courtyard"`), which KiCad
/// accepts for the layer, would put an item on a layer the gates read by
/// its canonical name, unseen. Names are compared exactly: a case or a
/// look-alike is another name. A board with no table is not judged here:
/// KiCad then reads its own names, and every gate that reads copper itself
/// refuses it first (`route::declared`: "the board has no layer table").
///
/// Judged: a `layer`'s first value (KiCad may append a flag, such as a
/// text's `knockout`) and every value of a `layers`, but the table itself,
/// a `stackup`'s rows, which describe layers (KiCad names its dielectrics
/// there) and place nothing, and a `padstack`'s `"Inner"` row, KiCad's own
/// keyword for every inner layer.
fn layer_names(node: &Node<'_>, parent: &str, table: &[&str]) -> Result<(), Refusal> {
    let head = node.head().unwrap_or("");
    let names: Vec<&str> = match head {
        "layers" if parent == node.kind.root() => Vec::new(),
        "layer" if parent == "stackup" => Vec::new(),
        // A padstack's row for every inner layer (`PADSTACK::INNER_LAYERS`).
        "layer" if parent == "padstack" && node.arg(1) == Some("Inner") => Vec::new(),
        "layer" => node.arg(1).into_iter().collect(),
        "layers" => node.texts().collect(),
        _ => Vec::new(),
    };
    let pad = head == "layers" && parent == "pad";
    for name in names {
        if !table.contains(&name) && !(pad && PAD_WILDCARDS.contains(&name)) {
            return Err(Refusal::Layer {
                kind: node.kind,
                path: node.path(),
                name: name.to_owned(),
                wildcards: if pad {
                    PAD_WILDCARDS.iter().map(|w| format!(" or `{w}`")).collect()
                } else {
                    String::new()
                },
            });
        }
    }
    Ok(())
}

/// The one `(version …)` a file may declare: the engine's own emitter's
/// (`AET-WO-3303` A11), as a bare atom; a netlist's, `"E"`, quoted as KiCad
/// 10.0.7 writes it. KiCad reads another version's grammar differently; none
/// is read here.
pub fn version(tree: &Tree) -> Result<(), Refusal> {
    let root = tree.root();
    let (want, quoted) = match tree.kind {
        Kind::Board => (crate::pcb::VERSION, false),
        Kind::Schematic => (crate::schematic::VERSION, false),
        Kind::Netlist => (NETLIST_VERSION, true),
    };
    let child = root.one("version")?;
    let found = match child.as_ref().map(|v| v.sx.items()) {
        Some([_, Sx::Atom(v)]) if !quoted => Some(v.as_str()),
        Some([_, Sx::Str(v)]) if quoted => Some(v.as_str()),
        _ => None,
    };
    if found == Some(want) {
        return Ok(());
    }
    Err(Refusal::Version {
        kind: tree.kind,
        path: child.map_or_else(|| root.path(), |v| v.path()),
        found: found.map_or_else(
            || "missing, quoted or not a single value".to_owned(),
            str::to_owned,
        ),
        want,
    })
}

/// The longest keyword a list may have (`AET-WO-1311` §1.2). The longest in
/// any held KiCad file or emitted board is 33 bytes
/// (`duplicate_pad_numbers_are_jumpers`, measured); the cap bounds a
/// rendered path.
pub const MAX_KEYWORD_BYTES: usize = 64;

/// The longest path a refusal renders: one segment per level, each a `/`, a
/// keyword, and its ordinal in brackets (at most seven digits, since a file
/// holds at most [`sexpr::MAX_TOKENS`] tokens).
pub const MAX_PATH_BYTES: usize = sexpr::MAX_DEPTH * (MAX_KEYWORD_BYTES + 10);

/// Every node [`RULES`] names, judged; depth first, without recursion. A
/// list's keyword longer than [`MAX_KEYWORD_BYTES`] is refused first.
fn check(tree: &Tree) -> Result<(), Refusal> {
    // A board's layer table, whose names alone its items may name
    // ([`layer_names`]); a footprint judged alone has none.
    let table: Option<Vec<&str>> = match tree.kind {
        Kind::Board => tree
            .root()
            .one("layers")?
            .map(|t| t.lists().filter_map(|row| row.arg(1)).collect()),
        _ => None,
    };
    let mut stack = vec![(tree.root(), "")];
    while let Some((node, parent)) = stack.pop() {
        let head = node.head().unwrap_or("");
        if head.len() > MAX_KEYWORD_BYTES {
            return Err(Refusal::Keyword {
                kind: tree.kind,
                path: node.at.up.as_deref().map_or_else(String::new, render),
                bytes: head.len(),
            });
        }
        if let Some(rule) = rule(tree.kind, parent, head) {
            judge(&node, rule)?;
        }
        numbers(&node)?;
        if let Some(table) = &table {
            layer_names(&node, parent, table)?;
        }
        if tree.kind == Kind::Schematic {
            spelling(&node)?;
        }
        for child in node.lists() {
            stack.push((child, head));
        }
    }
    Ok(())
}

/// The nodes whose every value KiCad reads as a number (`parseDouble` and
/// `parseInt` take a bare number token only: a quoted one, a `+`, a second
/// `-` or an exponent is an error there, `AET-WO-3303` A11).
const NUMERIC: &[&str] = &[
    "at",
    "start",
    "end",
    "mid",
    "center",
    "xy",
    "size",
    "width",
    "thickness",
    "roundrect_rratio",
    // Every other child KiCad reads as a number (adversarial F4).
    "min_thickness",
    "zone_connect",
    "thermal_gap",
    "thermal_bridge_width",
    "thermal_bridge_angle",
];

/// A node whose leading keyword is one of these words and whose every other
/// value is a number: `(hatch edge 0.5)`, `(drill 0.3)`, `(drill oval 0.5 1)`.
/// `offset` takes no keyword: a drill's is numbers, a model's holds `xyz`.
const KEYED: &[(&str, &[&str])] = &[
    ("hatch", &["edge", "full", "none"]),
    ("drill", &["oval"]),
    ("offset", &[]),
];

/// Each value of a [`NUMERIC`] node is a bare atom of KiCad's number
/// grammar ([`sexpr::nm`]).
fn numbers(node: &Node<'_>) -> Result<(), Refusal> {
    let Some(head) = node.head() else {
        return Ok(());
    };
    let (keyed, keywords): (bool, &[&str]) = match KEYED.iter().find(|(h, _)| *h == head) {
        Some((_, words)) => (true, words),
        None if NUMERIC.contains(&head) => (false, &[]),
        None => return Ok(()),
    };
    for (i, v) in node.sx.items()[1..].iter().enumerate() {
        // A keyed node's keyword, first and bare; its lists (a drill's
        // `offset`, a model's `xyz`) are judged as nodes of their own.
        if keyed && matches!(v, Sx::List(_))
            || i == 0 && matches!(v, Sx::Atom(a) if keywords.contains(&a.as_str()))
        {
            continue;
        }
        let ok = matches!(v, Sx::Atom(a) if sexpr::nm(a).is_some());
        if !ok {
            return Err(Refusal::Number {
                kind: node.kind,
                path: node.path(),
                text: match v {
                    Sx::Atom(t) => t.clone(),
                    Sx::Str(t) => format!("\"{t}\""),
                    Sx::List(_) => "(…)".to_owned(),
                },
            });
        }
    }
    Ok(())
}

fn judge(node: &Node<'_>, rule: &Rule) -> Result<(), Refusal> {
    for &i in rule.keywords {
        if let Some(Sx::Str(word)) = node.sx.items().get(i) {
            return Err(Refusal::Quoted {
                kind: node.kind,
                path: node.path(),
                word: word.clone(),
            });
        }
    }
    let mut seen: BTreeMap<usize, &str> = BTreeMap::new();
    let mut fields: Vec<String> = Vec::new();
    for child in node.lists() {
        let Some(head) = child.head() else { continue };
        if let Some(group) = rule.once.iter().position(|g| g.contains(&head))
            && let Some(first) = seen.insert(group, head)
        {
            return Err(Refusal::Repeated {
                kind: node.kind,
                path: node.path(),
                first: first.to_owned(),
                child: head.to_owned(),
            });
        }
        let name = match (rule.fields, head) {
            (Fields::Footprint | Fields::Symbol, "property") => {
                child.named().map(|(n, _)| n.to_owned())
            }
            (Fields::Footprint, "fp_text") => match child.arg(1) {
                Some("reference") => Some("Reference".to_owned()),
                Some("value") => Some("Value".to_owned()),
                _ => None,
            },
            _ => None,
        };
        let Some(name) = name else { continue };
        if fields.contains(&name) {
            return Err(Refusal::FieldTwice {
                kind: node.kind,
                path: node.path(),
                name,
            });
        }
        fields.push(name);
    }
    Ok(())
}

/// A schematic `property` spelling a mandatory field otherwise than its
/// canonical name.
fn spelling(node: &Node<'_>) -> Result<(), Refusal> {
    if node.head() != Some("property") {
        return Ok(());
    }
    let Some((name, _)) = node.named() else {
        return Ok(());
    };
    let lower = name.to_lowercase();
    match MANDATORY.iter().find(|m| m.to_lowercase() == lower) {
        Some(canonical) if *canonical != name => Err(Refusal::FieldSpelling {
            kind: node.kind,
            path: node.path(),
            name: name.to_owned(),
            canonical,
        }),
        _ => Ok(()),
    }
}

impl Tree {
    /// The top-level list.
    pub fn root(&self) -> Node<'_> {
        Node {
            sx: &self.root,
            kind: self.kind,
            at: Rc::new(Place {
                up: None,
                head: self.kind.root(),
                n: 0,
            }),
        }
    }
}

/// Where a list sits: its parent's place, its head, and its ordinal among
/// its parent's lists with that head. A node holds one of these, never its
/// path as text (review 2, A16): the path is rendered only into a refusal,
/// so a wide, deep file costs one small record per list.
#[derive(Debug)]
struct Place<'a> {
    up: Option<Rc<Place<'a>>>,
    head: &'a str,
    n: usize,
}

/// A place's path from the root, rendered.
fn render(place: &Place<'_>) -> String {
    let mut places = vec![place];
    while let Some(up) = places.last().and_then(|p| p.up.as_deref()) {
        places.push(up);
    }
    let mut out = String::new();
    for p in places.iter().rev() {
        if p.up.is_some() {
            out.push_str(&format!("/{}[{}]", p.head, p.n));
        } else {
            out.push_str(p.head);
        }
    }
    out
}

/// A list inside a [`Tree`], with its place from the root. It offers no
/// first-match lookup: [`Node::one`] refuses a second match.
#[derive(Clone, Debug)]
pub struct Node<'a> {
    sx: &'a Sx,
    kind: Kind,
    at: Rc<Place<'a>>,
}

impl<'a> Node<'a> {
    /// The path from the root, such as `kicad_pcb/footprint[3]/pad[2]`, for
    /// a refusal: at most [`MAX_PATH_BYTES`], since every list's keyword is
    /// at most [`MAX_KEYWORD_BYTES`] ([`check`]) and its depth at most
    /// [`sexpr::MAX_DEPTH`].
    fn path(&self) -> String {
        render(&self.at)
    }

    /// The head, when it is a bare atom (KiCad's keywords are).
    pub fn head(&self) -> Option<&'a str> {
        self.sx.head()
    }

    /// The `i`th element's text (0 is the head).
    pub fn arg(&self, i: usize) -> Option<&'a str> {
        self.sx.arg(i)
    }

    /// The text of the last element, such as a pad net's name in either
    /// `(net "N")` or KiCad 9's `(net 3 "N")`.
    pub fn last(&self) -> Option<&'a str> {
        self.sx.items().last().and_then(Sx::text)
    }

    /// The number of elements, head included.
    pub(crate) fn len(&self) -> usize {
        self.sx.items().len()
    }

    /// Every atom and string after the head.
    pub fn texts(&self) -> impl Iterator<Item = &'a str> + 'a {
        self.sx.items().iter().skip(1).filter_map(Sx::text)
    }

    /// Every list inside this one, in order. A list's path segment is its
    /// head and its ordinal among the lists with that head.
    pub fn lists(&self) -> impl Iterator<Item = Node<'a>> + '_ {
        let mut ordinal: BTreeMap<&'a str, usize> = BTreeMap::new();
        self.sx
            .items()
            .iter()
            .filter(|c| matches!(c, Sx::List(_)))
            .map(move |c| {
                let head = c.head().unwrap_or("()");
                let n = ordinal.entry(head).or_default();
                *n += 1;
                Node {
                    sx: c,
                    kind: self.kind,
                    at: Rc::new(Place {
                        up: Some(Rc::clone(&self.at)),
                        head,
                        n: *n,
                    }),
                }
            })
    }

    /// Every child list whose head is `head`.
    pub fn all<'h>(&self, head: &'h str) -> impl Iterator<Item = Node<'a>> + use<'a, '_, 'h> {
        self.lists().filter(move |c| c.head() == Some(head))
    }

    /// The child whose head is `head`, if there is one; two are refused,
    /// never resolved first-or-last.
    pub fn one(&self, head: &str) -> Result<Option<Node<'a>>, Refusal> {
        let mut found = self.all(head);
        let first = found.next();
        if found.next().is_some() {
            return Err(Refusal::Ambiguous {
                kind: self.kind,
                path: self.path(),
                child: head.to_owned(),
            });
        }
        Ok(first)
    }

    /// A `(property …)`'s name and value as KiCad reads them. In a schematic,
    /// `parseSchField` takes a bare `private` before the name as a flag
    /// (`T_private`); a quoted `"private"` is a name. A board's parser has
    /// no such flag.
    pub fn named(&self) -> Option<(&'a str, Option<&'a str>)> {
        let items = self.sx.items();
        let skip = usize::from(
            self.kind == Kind::Schematic
                && matches!(items.get(1), Some(Sx::Atom(a)) if a == "private"),
        );
        let name = items.get(1 + skip).and_then(Sx::text)?;
        Some((name, items.get(2 + skip).and_then(Sx::text)))
    }

    /// A footprint's field `name` (`property`, or the legacy `fp_text`
    /// form of `Reference` and `Value`): one value, never one of several.
    pub fn field(&self, name: &str) -> Result<Option<&'a str>, Refusal> {
        let legacy = match name {
            "Reference" => Some("reference"),
            "Value" => Some("value"),
            _ => None,
        };
        let mut found = self.lists().filter_map(|c| match c.head() {
            Some("property") if c.arg(1) == Some(name) => c.arg(2),
            Some("fp_text") if legacy.is_some() && c.arg(1) == legacy => c.arg(2),
            _ => None,
        });
        let first = found.next();
        if found.next().is_some() {
            return Err(Refusal::FieldTwice {
                kind: self.kind,
                path: self.path(),
                name: name.to_owned(),
            });
        }
        Ok(first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refused(text: &str) -> Refusal {
        board(text).err().expect("refused")
    }

    #[test]
    fn a_child_kicad_holds_once_is_refused_when_repeated_by_path() {
        let r = refused("(kicad_pcb (general (thickness 1.6) (thickness 0.2)))");
        assert_eq!(r.kind(), "MalformedBoard");
        assert_eq!(r.id(), Some("kicad_pcb/general[1]"));
        let r = refused(
            "(kicad_pcb (footprint \"R\" (pad \"1\" smd rect (size 1 1) (size 1 4))) (footprint \"R\" (pad \"2\" smd rect (net \"A\") (net \"B\"))))",
        );
        assert_eq!(r.id(), Some("kicad_pcb/footprint[2]/pad[1]"));
        // Two spellings of one value are one value twice.
        let r = refused("(kicad_pcb (segment (layer \"F.Cu\") (layers \"B.Cu\")))");
        assert!(
            matches!(r, Refusal::Repeated { ref first, ref child, .. } if first == "layer" && child == "layers")
        );
        // A pad's type and shape are keywords to KiCad.
        let r = refused("(kicad_pcb (footprint \"R\" (pad \"1\" \"smd\" rect)))");
        assert!(matches!(r, Refusal::Quoted { .. }), "{r:?}");
        refused("(kicad_pcb (footprint \"R\" (pad \"1\" smd \"roundrect\")))");
        // A zone inside a footprint is judged as one on the board.
        let r = refused("(kicad_pcb (footprint \"R\" (zone (net_name \"A\") (net_name \"B\"))))");
        assert_eq!(r.id(), Some("kicad_pcb/footprint[1]/zone[1]"));
        // A second general section sets the thickness again.
        refused("(kicad_pcb (general (thickness 1.6)) (general (thickness 0.2)))");
    }

    #[test]
    fn what_kicad_appends_may_repeat() {
        let ok = "(kicad_pcb (net 0 \"\") (net 1 \"A\") (footprint \"R\" (attr smd) (attr board_only) (pad \"1\" smd rect) (pad \"2\" smd rect) (model a) (model b) (zone (polygon (pts (xy 0 0))) (polygon (pts (xy 1 1))))) (gr_rect (start 0 0) (end 1 1)) (gr_rect (start 0 0) (end 1 1)) (setup) (setup))";
        assert!(board(ok).is_ok());
    }

    #[test]
    fn a_footprint_field_named_twice_is_refused_in_either_form() {
        for fp in [
            "(property \"Reference\" \"R1\") (property \"Reference\" \"C9\")",
            "(property \"Reference\" \"R1\") (fp_text reference \"C9\" (at 0 0))",
            "(fp_text value \"10k\" (at 0 0)) (property \"Value\" \"100n\")",
        ] {
            let r = refused(&format!("(kicad_pcb (footprint \"R\" {fp}))"));
            assert!(matches!(r, Refusal::FieldTwice { .. }), "{fp}: {r:?}");
        }
        // KiCad compares a footprint's field names exactly.
        assert!(board("(kicad_pcb (footprint \"R\" (property \"Reference\" \"R1\") (property \"reference\" \"x\")))").is_ok());
    }

    #[test]
    fn a_symbol_field_is_spelled_as_kicad_reads_it_and_named_once() {
        let r =
            schematic("(kicad_sch (symbol (property \"Value\" \"a\") (property \"VALUE\" \"b\")))")
                .err()
                .unwrap();
        assert_eq!(r.kind(), "MalformedSchematic");
        assert!(
            matches!(
                r,
                Refusal::FieldSpelling {
                    canonical: "Value",
                    ..
                }
            ),
            "{r:?}"
        );
        let r =
            schematic("(kicad_sch (symbol (property \"Value\" \"a\") (property \"Value\" \"b\")))")
                .err()
                .unwrap();
        assert!(matches!(r, Refusal::FieldTwice { .. }), "{r:?}");
        let r =
            schematic("(kicad_sch (symbol (lib_id \"x\") (property \"value\" \"unconnected-x\")))")
                .err()
                .unwrap();
        assert!(
            matches!(
                r,
                Refusal::FieldSpelling {
                    canonical: "Value",
                    ..
                }
            ),
            "{r:?}"
        );
        assert!(
            schematic(
                "(kicad_sch (symbol (property \"Value\" \"a\") (property \"Voltage\" \"b\")))"
            )
            .is_ok()
        );
        // `private` is a flag before the name, not the name.
        let r = schematic("(kicad_sch (symbol (property private \"VALUE\" \"x\")))")
            .err()
            .unwrap();
        assert!(
            matches!(
                r,
                Refusal::FieldSpelling {
                    canonical: "Value",
                    ..
                }
            ),
            "{r:?}"
        );
        let r = schematic(
            "(kicad_sch (symbol (property \"Value\" \"a\") (property private \"Value\" \"b\")))",
        )
        .err()
        .unwrap();
        assert!(matches!(r, Refusal::FieldTwice { .. }), "{r:?}");
        assert!(schematic("(kicad_sch (symbol (property \"private\" \"x\")))").is_ok());
        let r = schematic("(kicad_sch (symbol (lib_id \"a\") (lib_id \"b\")))")
            .err()
            .unwrap();
        assert_eq!(r.id(), Some("kicad_sch/symbol[1]"));
    }

    #[test]
    fn the_wrong_kind_or_an_unreadable_file_names_no_path() {
        assert_eq!(
            board("(kicad_sch)").err(),
            Some(Refusal::NotThisKind(Kind::Board))
        );
        let r = schematic("(kicad_sch").err().unwrap();
        assert_eq!((r.kind(), r.id()), ("MalformedSchematic", None));
    }

    #[test]
    fn one_refuses_a_second_match_wherever_the_rules_are_silent() {
        let tree = board("(kicad_pcb (x (y 1) (y 2)))").unwrap();
        let x = tree.root().one("x").unwrap().unwrap();
        assert!(matches!(x.one("y"), Err(Refusal::Ambiguous { .. })));
        assert_eq!(x.all("y").count(), 2);
        // A footprint no rule reaches still never gives one of two fields.
        let tree = board(
            "(kicad_pcb (x (footprint \"R\" (property \"Reference\" \"A\") (fp_text reference \"B\"))))",
        )
        .unwrap();
        let fp = tree
            .root()
            .one("x")
            .unwrap()
            .unwrap()
            .one("footprint")
            .unwrap()
            .unwrap();
        assert!(matches!(
            fp.field("Reference"),
            Err(Refusal::FieldTwice { .. })
        ));
    }

    #[test]
    fn a_number_is_kicads_bare_atom_or_refused_by_name() {
        let with = |v: &str| format!("(kicad_pcb (segment (start 0 0) (end 1 1) (width {v})))");
        for good in ["0", "5", "-5", "-0", "0.2", "-0.000001", "12.123456"] {
            assert!(board(&with(good)).is_ok(), "{good}");
        }
        // `+`, a second `-`, an exponent, a leading or trailing `.`, a
        // seventh decimal, U+2212 and a quoted number: none is a number KiCad
        // writes (it loads an exponent, a leading `.` and a seventh decimal,
        // measured: this reader is the stricter).
        for bad in [
            "+0.2",
            "--5",
            "1e3",
            "1E-3",
            ".2",
            "-.2",
            "1.",
            "0.1234567",
            "\"0.2\"",
            "\"\"",
            "(1)",
        ] {
            let r = refused(&with(bad));
            assert!(
                matches!(&r, Refusal::Number { path, .. } if path == "kicad_pcb/segment[1]/width[1]"),
                "{bad}: {r:?}"
            );
            assert_eq!(r.kind(), "MalformedBoard");
            assert!(r.to_string().contains("kicad_pcb/segment[1]/width[1]"));
        }
        // U+2212 never reaches a number: the tokenizer refuses the byte.
        assert!(matches!(
            refused(&with("\u{2212}5")),
            Refusal::Unreadable(..)
        ));
        // Each numeric node is judged, a rotation and a size among them.
        for text in [
            "(kicad_pcb (footprint \"R\" (at 1 2 \"90\")))",
            "(kicad_pcb (footprint \"R\" (at 1 +2)))",
            "(kicad_pcb (footprint \"R\" (pad \"1\" smd rect (size 1 \"1\"))))",
            "(kicad_pcb (general (thickness 1e0)))",
            "(kicad_pcb (footprint \"R\" (pad \"1\" smd roundrect (roundrect_rratio .25))))",
        ] {
            assert!(matches!(refused(text), Refusal::Number { .. }), "{text}");
        }
        let r = schematic("(kicad_sch (symbol (at 1 2 \"0\")))")
            .err()
            .unwrap();
        assert!(matches!(r, Refusal::Number { .. }));
        assert_eq!(r.kind(), "MalformedSchematic");
    }

    /// Adversarial F4: every child KiCad reads as a number is judged as one,
    /// a keyed node past its keyword; the forms KiCad writes still read.
    #[test]
    fn every_numeric_child_is_judged_past_its_keyword() {
        let zone = |inner: &str| format!("(kicad_pcb (zone (net 1) {inner}))");
        for good in [
            "(min_thickness 0.25)",
            "(hatch edge 0.5)",
            "(fill yes (thermal_gap 0.5) (thermal_bridge_width 0.5))",
        ] {
            assert!(board(&zone(good)).is_ok(), "{good}");
        }
        for bad in [
            "(min_thickness \"0.25\")",
            "(hatch edge +0.5)",
            "(hatch edge \"0.5\")",
            "(hatch edge inf)",
            "(fill yes (thermal_gap 0x2))",
        ] {
            assert!(board(&zone(bad)).is_err(), "{bad}");
        }
        let pad = |inner: &str| {
            format!(
                "(kicad_pcb (footprint \"x\" (pad \"1\" thru_hole circle (at 0 0) (size 1 1) {inner} (zone_connect 2))))"
            )
        };
        for good in [
            "(drill 0.3)",
            "(drill oval 0.5 1)",
            "(drill 0.3 (offset 0.1 0))",
        ] {
            assert!(board(&pad(good)).is_ok(), "{good}");
        }
        for bad in [
            "(drill \"0.3\")",
            "(drill oval 0.5 +1)",
            "(drill 0.3 (offset .1 0))",
        ] {
            assert!(board(&pad(bad)).is_err(), "{bad}");
        }
        assert!(
            board(&pad("(drill 0.3)").replace("(zone_connect 2)", "(zone_connect \"2\")")).is_err()
        );
    }

    #[test]
    fn a_node_reads_a_number_only_from_a_bare_atom() {
        let tree = board("(kicad_pcb (x (y 0.5 \"0.5\" -2)))").unwrap();
        let y = tree.root().one("x").unwrap().unwrap().one("y").unwrap();
        let y = y.unwrap();
        assert_eq!(number(&y, 1), Some(500_000));
        assert_eq!(number(&y, 2), None);
        assert_eq!(number(&y, 3), Some(-2_000_000));
        assert_eq!(number(&y, 4), None);
    }

    #[test]
    fn the_version_is_the_emitters_or_refused() {
        let ok = |text: &str| version(&board(text).unwrap());
        assert!(ok("(kicad_pcb (version 20260206))").is_ok());
        for bad in [
            "(kicad_pcb)",
            "(kicad_pcb (version 20240108))",
            "(kicad_pcb (version \"20260206\"))",
            "(kicad_pcb (version 20260206 20260206))",
            "(kicad_pcb (version))",
            "(kicad_pcb (version +20260206))",
        ] {
            let r = ok(bad).unwrap_err();
            assert!(matches!(r, Refusal::Version { .. }), "{bad}: {r:?}");
            assert_eq!(r.kind(), "MalformedBoard");
        }
        // A second `(version …)` is refused by the same rule that refuses any.
        assert!(
            board("(kicad_pcb (version 20260206) (version 20240108))").is_err(),
            "repeated"
        );
        let sch = schematic("(kicad_sch (version 20260306))").unwrap();
        assert!(version(&sch).is_ok());
        let sch = schematic("(kicad_sch (version 20260206))").unwrap();
        assert_eq!(version(&sch).unwrap_err().kind(), "MalformedSchematic");
    }

    #[test]
    fn a_copper_layer_is_kicads_canonical_name() {
        for yes in ["F.Cu", "B.Cu", "In1.Cu", "In9.Cu", "In30.Cu"] {
            assert!(copper_layer(yes), "{yes}");
        }
        for no in [
            "TOP", "TOP.Cu", "f.Cu", "F.Cu ", "In0.Cu", "In01.Cu", "In31.Cu", "In.Cu", "In+1.Cu",
            "*.Cu", "F.Mask", "Cu", "",
        ] {
            assert!(!copper_layer(no), "{no}");
        }
        assert!(copper_pad_layer("*.Cu") && copper_pad_layer("In2.Cu"));
        assert!(!copper_pad_layer("X.Cu"));
    }

    /// The emitter's layer table, as `pcb` writes it, around `items`.
    fn tabled(items: &str) -> String {
        format!(
            "(kicad_pcb (layers (0 \"F.Cu\" signal) (2 \"B.Cu\" signal) (31 \"F.CrtYd\" user \"F.Courtyard\") (5 \"F.SilkS\" user \"F.Silkscreen\") (1 \"F.Mask\" user) (13 \"F.Paste\" user) (25 \"Edge.Cuts\" user)) {items})"
        )
    }

    /// `AET-WO-1311` §1.1 (`3303` F4): an item on a layer outside the
    /// board's table is refused by name, wherever it is; a case variant, a
    /// Unicode look-alike (Cyrillic `С`, U+0421, for `C`), KiCad's `F&B.Cu`
    /// and a row's user name are other names. A pad's `layers` may also
    /// name `*.Cu` and `*.Mask`, and nothing else may.
    #[test]
    fn a_layer_outside_the_boards_table_is_refused_by_name() {
        let line = |layer: &str| {
            tabled(&format!(
                "(footprint \"X\" (layer \"F.Cu\") (fp_line (start 0 0) (end 1 0) (layer \"{layer}\")))"
            ))
        };
        for ok in ["F.Cu", "F.SilkS", "F.CrtYd", "Edge.Cuts"] {
            assert!(board(&line(ok)).is_ok(), "{ok}");
        }
        for no in [
            "F&B.Cu",
            "Top",
            "f.cu",
            "F.cu",
            "F.Cu ",
            "F.\u{421}u",
            "F.Courtyard",
            "*.Cu",
            "In1.Cu",
            "",
        ] {
            let r = refused(&line(no));
            assert!(
                matches!(&r, Refusal::Layer { name, path, .. } if name == no && path == "kicad_pcb/footprint[1]/fp_line[1]/layer[1]"),
                "{no:?}: {r:?}"
            );
            assert_eq!(r.kind(), "MalformedBoard");
        }
        let pad = |layers: &str| {
            tabled(&format!(
                "(footprint \"X\" (pad \"1\" thru_hole circle (at 0 0) (size 1 1) (drill 0.5) (layers {layers})))"
            ))
        };
        assert!(board(&pad("\"*.Cu\" \"*.Mask\"")).is_ok());
        assert!(board(&pad("\"F.Cu\" \"F.Mask\" \"F.Paste\"")).is_ok());
        for no in [
            "\"*.Cu\" \"*.Paste\"",
            "\"F&B.Cu\"",
            "\"*.cu\"",
            "\"B.Mask\"",
        ] {
            assert!(matches!(refused(&pad(no)), Refusal::Layer { .. }), "{no}");
        }
        // A wildcard is a pad's only: on a via or a segment it is refused.
        for item in [
            "(via (at 1 1) (size 0.6) (drill 0.3) (layers \"*.Cu\") (net \"/A\"))",
            "(segment (start 1 1) (end 2 1) (width 0.2) (layer \"*.Cu\") (net \"/A\"))",
        ] {
            assert!(
                matches!(refused(&tabled(item)), Refusal::Layer { .. }),
                "{item}"
            );
        }
        // What names no placed item is not judged: the table itself, a
        // stackup's rows (KiCad names its dielectrics there), a padstack's
        // `Inner`; and a text's flag after its layer is not a name.
        for ok in [
            "(setup (stackup (layer \"F.Cu\" (type \"copper\")) (layer \"dielectric 1\" (type \"core\"))))",
            "(via (at 1 1) (size 0.6) (drill 0.3) (layers \"F.Cu\" \"B.Cu\") (padstack (mode front_inner_back) (layer \"Inner\" (size 0.6))) (net \"/A\"))",
            "(gr_text \"x\" (at 1 1) (layer \"F.SilkS\" knockout))",
        ] {
            assert!(board(&tabled(ok)).is_ok(), "{ok}");
        }
        assert!(matches!(
            refused(&tabled(
                "(via (at 1 1) (padstack (layer \"Outer\" (size 0.6))))"
            )),
            Refusal::Layer { .. }
        ));
        // With no table, KiCad's own names are KiCad's to judge, and every
        // gate that reads copper refuses the board (`route::declared`).
        assert!(board("(kicad_pcb (gr_line (layer \"F.SilkS\")))").is_ok());
    }

    /// `AET-WO-1311` §1.2: a list's keyword is at most
    /// [`MAX_KEYWORD_BYTES`]; one byte more is refused, naming its parent's
    /// path. A path at the nesting limit with every keyword at that length
    /// renders within [`MAX_PATH_BYTES`].
    #[test]
    fn a_keyword_past_the_cap_is_refused_and_a_path_is_bounded() {
        let at = "k".repeat(MAX_KEYWORD_BYTES);
        assert!(board(&format!("(kicad_pcb ({at} 1))")).is_ok());
        let over = format!("{at}k");
        let r = refused(&format!("(kicad_pcb (general ({over} 1)))"));
        assert!(
            matches!(&r, Refusal::Keyword { path, bytes, .. } if path == "kicad_pcb/general[1]" && *bytes == MAX_KEYWORD_BYTES + 1),
            "{r:?}"
        );
        // The deepest path: every level a keyword at the cap, the last a
        // repeated `width`, which the number grammar refuses there.
        let depth = sexpr::MAX_DEPTH - 2;
        let text = format!(
            "(kicad_pcb {}(width x){})",
            format!("({at} ").repeat(depth),
            ")".repeat(depth)
        );
        let r = refused(&text);
        let Refusal::Number { path, .. } = &r else {
            panic!("{r:?}");
        };
        assert_eq!(path.matches(&at).count(), depth);
        assert!(path.len() <= MAX_PATH_BYTES, "{}", path.len());
    }
}
