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

use crate::sexpr::{self, SexprError, Sx};

/// Which of KiCad's two files a tree is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Board,
    Schematic,
}

impl Kind {
    fn root(self) -> &'static str {
        match self {
            Kind::Board => "kicad_pcb",
            Kind::Schematic => "kicad_sch",
        }
    }
    fn noun(self) -> &'static str {
        match self {
            Kind::Board => "board",
            Kind::Schematic => "schematic",
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
}

impl Refusal {
    fn file(&self) -> Kind {
        match self {
            Self::Unreadable(k, _) | Self::NotThisKind(k) => *k,
            Self::Repeated { kind, .. }
            | Self::FieldTwice { kind, .. }
            | Self::FieldSpelling { kind, .. }
            | Self::Quoted { kind, .. }
            | Self::Ambiguous { kind, .. } => *kind,
        }
    }

    /// The refusal's `kind` in the CLI's envelope.
    pub fn kind(&self) -> &'static str {
        match self.file() {
            Kind::Board => "MalformedBoard",
            Kind::Schematic => "MalformedSchematic",
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

/// The nodes the gates read, with what each may hold once. A node absent
/// here is read by no gate; its children are KiCad's alone.
const RULES: &[Rule] = &[
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

/// Every node [`RULES`] names, judged; depth first, without recursion.
fn check(tree: &Tree) -> Result<(), Refusal> {
    let mut stack = vec![(tree.root(), "")];
    while let Some((node, parent)) = stack.pop() {
        let head = node.head().unwrap_or("");
        if let Some(rule) = rule(tree.kind, parent, head) {
            judge(&node, rule)?;
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

fn judge(node: &Node<'_>, rule: &Rule) -> Result<(), Refusal> {
    for &i in rule.keywords {
        if let Some(Sx::Str(word)) = node.sx.items().get(i) {
            return Err(Refusal::Quoted {
                kind: node.kind,
                path: node.path.clone(),
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
                path: node.path.clone(),
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
                path: node.path.clone(),
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
            path: node.path.clone(),
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
            path: self.kind.root().to_owned(),
        }
    }
}

/// A list inside a [`Tree`], with its path from the root. It offers no
/// first-match lookup: [`Node::one`] refuses a second match.
#[derive(Clone, Debug)]
pub struct Node<'a> {
    sx: &'a Sx,
    kind: Kind,
    path: String,
}

impl<'a> Node<'a> {
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
    pub fn len(&self) -> usize {
        self.sx.items().len()
    }

    /// True for a list with nothing in it, `()`.
    pub fn is_empty(&self) -> bool {
        self.sx.items().is_empty()
    }

    /// Every atom and string after the head.
    pub fn texts(&self) -> impl Iterator<Item = &'a str> + 'a {
        self.sx.items().iter().skip(1).filter_map(Sx::text)
    }

    /// Where this node sits, such as `kicad_pcb/footprint[3]/pad[2]`.
    pub fn path(&self) -> &str {
        &self.path
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
                    path: format!("{}/{head}[{n}]", self.path),
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
                path: self.path.clone(),
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
                path: self.path.clone(),
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
}
