//! `AET-WO-1211d`: our own symbol for the TI DRV8833PWP (below), and our own
//! symbol and footprint for the TI DRV8876PWP
//! (PWP0016A PowerPAD HTSSOP-16), held to citation tables written from TI
//! SLVSDS7B (August 2019, revised November 2019), apart from the files (a
//! table derived from a file would only check the file against itself,
//! `RUL-20`). A land off its cited dimension, a stray pad or via, a swapped
//! pin or a wrong electrical type fails here.
//!
//! Coordinates are KiCad's: the top view, x to the right, y down, in nm.
//! p. 45 draws the part in the top view, pin 1 at the left row's top and
//! the numbers running counter-clockwise (8 at the left row's foot, 9 at the
//! right row's foot, 16 at its top), as read here.

use aethelgard_eda::library;
use aethelgard_eda::sexpr::{self, Sx};

/// A pad as the file holds it: number, centre, size (nm) and layers.
type ReadPad = (String, (i64, i64), (i64, i64), Vec<String>);
type ReadPin = (String, String, String);

/// One pad: number, centre, size (nm), layers.
type Pad = (
    &'static str,
    (i64, i64),
    (i64, i64),
    &'static [&'static str],
);

const SIGNAL: &[&str] = &["F.Cu", "F.Mask", "F.Paste"];

/// p. 46, LAND PATTERN EXAMPLE: 16X (1.5) by 16X (0.45) at 14X (0.65),
/// the rows (5.8) apart. The (5.8)'s extension lines are the dashed centre
/// lines through each row of lands, so the rows sit at x = ±2.9 and each
/// land runs from 2.15 to 3.65 from the centre. That reading is checked
/// against p. 45's lead: its tip lies 3.1 to 3.3 from the centre (6.6/6.2
/// TYP) and its foot is 0.5 to 0.75 long (DETAIL A), so every foot lies
/// within 2.35 to 3.3, inside the land. Read outer edge to outer edge
/// (lands at 1.4 to 2.9) the land would lose the toe; inner to inner (2.9
/// to 4.4), the heel. y is ±0.325, ±0.975, ±1.625, ±2.275 (seven pitches
/// of 0.65 span the boxed `2X 4.55` of p. 45). p. 46 draws pads 1-16 both
/// ways (SOLDER MASK DETAILS); non-solder-mask defined is taken here, as
/// KiCad's own footprints take it, so each is on `F.Cu`, `F.Mask` and
/// `F.Paste` at its land's size; p. 47 prints its paste at the same 16X
/// (1.5) by (0.45).
const LANDS: [Pad; 16] = [
    ("1", (-2_900_000, -2_275_000), (1_500_000, 450_000), SIGNAL),
    ("2", (-2_900_000, -1_625_000), (1_500_000, 450_000), SIGNAL),
    ("3", (-2_900_000, -975_000), (1_500_000, 450_000), SIGNAL),
    ("4", (-2_900_000, -325_000), (1_500_000, 450_000), SIGNAL),
    ("5", (-2_900_000, 325_000), (1_500_000, 450_000), SIGNAL),
    ("6", (-2_900_000, 975_000), (1_500_000, 450_000), SIGNAL),
    ("7", (-2_900_000, 1_625_000), (1_500_000, 450_000), SIGNAL),
    ("8", (-2_900_000, 2_275_000), (1_500_000, 450_000), SIGNAL),
    ("9", (2_900_000, 2_275_000), (1_500_000, 450_000), SIGNAL),
    ("10", (2_900_000, 1_625_000), (1_500_000, 450_000), SIGNAL),
    ("11", (2_900_000, 975_000), (1_500_000, 450_000), SIGNAL),
    ("12", (2_900_000, 325_000), (1_500_000, 450_000), SIGNAL),
    ("13", (2_900_000, -325_000), (1_500_000, 450_000), SIGNAL),
    ("14", (2_900_000, -975_000), (1_500_000, 450_000), SIGNAL),
    ("15", (2_900_000, -1_625_000), (1_500_000, 450_000), SIGNAL),
    ("16", (2_900_000, -2_275_000), (1_500_000, 450_000), SIGNAL),
];

/// The exposed pad, centred on both SYMM lines.
/// - Copper: p. 46, exposed metal (3.4) wide by (5) along the rows, NOTE 9;
///   pad `17` on `F.Cu` alone, so the mask and paste below set the
///   openings, not the copper.
/// - Mask: p. 46, a SOLDER MASK DEFINED PAD, (3.3) by (3.3).
/// - Paste: p. 47, one (3.3) by (3.3) opening, "BASED ON 0.125 THICK
///   STENCIL" (its table: 3.3 X 3.3 (SHOWN) at 0.125).
///
/// TI's (Ø0.2) TYP vias at (1.1) TYP pitch are not drawn: the drill is
/// below the boards' 300 µm via drill (`AET-WO-1211d`, ruled).
const EXPOSED: [Pad; 3] = [
    ("17", (0, 0), (3_400_000, 5_000_000), &["F.Cu"]),
    ("", (0, 0), (3_300_000, 3_300_000), &["F.Mask"]),
    ("", (0, 0), (3_300_000, 3_300_000), &["F.Paste"]),
];

/// One pin: number, name and KiCad electrical type.
type Pin = (&'static str, &'static str, &'static str);

/// SLVSDS7B p. 3, "Pin Functions", the PWP column; the TYPE column is
/// footnoted "PWR = power, I = input, O = output, NC = no connection,
/// OD = open-drain". Each type is chosen so that KiCad's ERC (its default
/// pin matrix) passes on a correctly wired board:
///
/// - EN/IN1, PH/IN2, `~{SLEEP}` (p. 3: I): `input`, each driven by an MCU
///   pin or tied.
/// - `~{FAULT}` (OD): `open_collector`, its pull-up a passive resistor.
/// - VREF (I): `input`. It is tied to a rail ("the VREF pin is tied to
///   VCC", p. 28) or fed by a divider; an input on a rail its `PWR_FLAG` or
///   a regulator's `power_out` drives, or on a resistor (passive), is driven.
/// - IMODE (I, quad-level) and PMODE (I, tri-level): `input`, as p. 3 says,
///   since each is tied to `GND` or a rail or set by a resistor to ground,
///   all of which drive an input. Their Hi-Z level (pin left open) needs a
///   no-connect flag under `input` and `passive` alike, so `passive` would
///   buy nothing.
/// - IPROPI (O): `passive`, not `output`. The pin sources a current into
///   an external resistor to ground (p. 12), but p. 13 also allows "tying
///   IPROPI to GND" to disable regulation. `GND` carries a `PWR_FLAG`
///   (`power_out`), and KiCad's default matrix makes output against
///   power_out an error, so `output` would fail ERC on a board TI allows.
///   (The IR has no strap for a current output today, so no board it draws
///   ties IPROPI to `GND`, and `output` would pass on each of them.)
/// - OUT1, OUT2 (O): `output`, not `power_out` as KiCad's DRV8833PWP has.
///   They drive a motor (passive connector pins), never a rail. The emitter
///   counts a rail a `power_out` pin reaches as driven and leaves its
///   `PWR_FLAG` off (`schematic.rs`, `AET-WO-4300d2` D12), so `power_out`
///   could unflag a rail a mis-wired output touched. `output` still makes
///   OUT1 and OUT2 on one net an error.
/// - VM, GND, PGND (PWR): `power_in`, each on a flagged rail.
/// - VCP, CPH, CPL (PWR): `passive`. They are charge-pump nodes that only
///   capacitors reach (p. 3: CPH to CPL 22 nF, VCP to VM 100 nF). A
///   `power_in` there has no `power_out` driver and would fail ERC.
/// - Pin 17, the thermal pad ("PAD ... Connect to system ground", p. 3):
///   `passive`, named `GND`, as KiCad's DRV8833PWP draws its pad. It is a
///   visible pin of its own, so the board ties it to `GND` by a wire.
const PINS: [Pin; 17] = [
    ("1", "EN/IN1", "input"),
    ("2", "PH/IN2", "input"),
    ("3", "~{SLEEP}", "input"),
    ("4", "~{FAULT}", "open_collector"),
    ("5", "VREF", "input"),
    ("6", "IPROPI", "passive"),
    ("7", "IMODE", "input"),
    ("8", "OUT1", "output"),
    ("9", "PGND", "power_in"),
    ("10", "OUT2", "output"),
    ("11", "VM", "power_in"),
    ("12", "VCP", "passive"),
    ("13", "CPH", "passive"),
    ("14", "CPL", "passive"),
    ("15", "GND", "power_in"),
    ("16", "PMODE", "input"),
    ("17", "GND", "passive"),
];

const SYMBOL: &str = "Aethelgard_Driver:DRV8876PWP";
const FOOTPRINT: &str =
    "Aethelgard_Driver:TI_HTSSOP-16-1EP_4.4x5mm_P0.65mm_EP3.4x5mm_Mask3.3x3.3mm";

fn nm(item: &Sx, i: usize) -> i64 {
    sexpr::nm(item.arg(i).unwrap()).unwrap()
}

/// Every pad of `fp`, in file order: each `smd rect`, square to the axes.
fn pads(fp: &Sx) -> Vec<ReadPad> {
    fp.children("pad")
        .map(|p| {
            assert_eq!(p.arg(2), Some("smd"), "{p:?}");
            assert_eq!(p.arg(3), Some("rect"), "{p:?}");
            let at = p.child("at").unwrap();
            assert_eq!(at.arg(3), None, "a pad at an angle: {p:?}");
            let size = p.child("size").unwrap();
            let layers = p
                .child("layers")
                .unwrap()
                .items()
                .iter()
                .skip(1)
                .filter_map(Sx::text)
                .map(str::to_owned)
                .collect();
            (
                p.arg(1).unwrap().to_owned(),
                (nm(at, 1), nm(at, 2)),
                (nm(size, 1), nm(size, 2)),
                layers,
            )
        })
        .collect()
}

fn want(table: &[Pad]) -> Vec<ReadPad> {
    table
        .iter()
        .map(|&(n, at, size, layers)| {
            (
                n.to_owned(),
                at,
                size,
                layers.iter().map(|l| (*l).to_owned()).collect(),
            )
        })
        .collect()
}

fn read() -> Vec<ReadPad> {
    let fp = library::footprint(FOOTPRINT).unwrap();
    assert_eq!(fp.children("zone").count(), 0, "a keepout or zone");
    pads(&fp)
}

#[test]
fn the_lands_are_pwp0016a_s_land_pattern() {
    let got: Vec<ReadPad> = read().into_iter().filter(|p| p.3 == SIGNAL).collect();
    assert_eq!(got, want(&LANDS));
}

#[test]
fn the_exposed_pad_is_its_copper_mask_and_paste() {
    let got: Vec<ReadPad> = read().into_iter().filter(|p| p.3 != SIGNAL).collect();
    let mut want = want(&EXPOSED);
    // KiCad writes unnumbered pads first.
    want.rotate_left(1);
    assert_eq!(got, want);
}

/// Nothing but the 16 lands and the exposed pad's three: no stray pad, no
/// via drawn as a pad.
#[test]
fn the_footprint_holds_nineteen_pads() {
    assert_eq!(read().len(), LANDS.len() + EXPOSED.len());
}

/// §10.1 (p. 32): the thermal pad is joined to the ground plane, so pad
/// `17` meets a zone solidly (`zone_connect 2`) and is marked a heatsink,
/// as KiCad's own PowerPAD footprints mark theirs.
#[test]
fn the_exposed_pad_joins_its_plane_solidly() {
    let fp = library::footprint(FOOTPRINT).unwrap();
    let pad = fp.children("pad").find(|p| p.arg(1) == Some("17")).unwrap();
    assert_eq!(pad.child("zone_connect").and_then(|z| z.arg(1)), Some("2"));
    assert_eq!(
        pad.child("property").and_then(|p| p.arg(1)),
        Some("pad_prop_heatsink")
    );
}

/// p. 46 states no minimum spacing between lands, so this is the table's
/// own sanity: no two lands of pads 1-16 touch, and none reaches the
/// exposed pad's copper (the gaps are 0.2 between neighbours and 0.45 to
/// the exposed pad). The exposed pad's mask opening lies inside its copper,
/// as a solder-mask-defined pad's must, and its paste inside the opening.
#[test]
fn no_land_touches_another_or_the_exposed_pad() {
    let gap = |a: &Pad, b: &Pad| {
        let g = |c: i64, d: i64, s: i64, t: i64| (c - d).abs() - (s + t) / 2;
        g(a.1.0, b.1.0, a.2.0, b.2.0).max(g(a.1.1, b.1.1, a.2.1, b.2.1))
    };
    for (i, a) in LANDS.iter().enumerate() {
        for b in LANDS[i + 1..].iter().chain(&EXPOSED[..1]) {
            assert!(gap(a, b) > 0, "{} {}: {}", a.0, b.0, gap(a, b));
        }
    }
    let inset =
        |outer: &Pad, inner: &Pad| ((outer.2.0 - inner.2.0) / 2).min((outer.2.1 - inner.2.1) / 2);
    assert!(inset(&EXPOSED[0], &EXPOSED[1]) >= 0);
    assert!(inset(&EXPOSED[1], &EXPOSED[2]) >= 0);
}

fn pins(lib_id: &str) -> Vec<ReadPin> {
    let sym = library::symbol(lib_id).unwrap();
    let mut out: Vec<ReadPin> = sym
        .children("symbol")
        .flat_map(|u| u.children("pin"))
        .map(|p| {
            (
                p.child("number").unwrap().arg(1).unwrap().to_owned(),
                p.child("name").unwrap().arg(1).unwrap().to_owned(),
                p.arg(1).unwrap().to_owned(),
            )
        })
        .collect();
    out.sort_by_key(|p| p.0.parse::<u32>().unwrap());
    out
}

#[test]
fn the_symbol_s_pins_are_the_pin_functions_table() {
    let want: Vec<ReadPin> = PINS
        .iter()
        .map(|&(n, name, kind)| (n.to_owned(), name.to_owned(), kind.to_owned()))
        .collect();
    assert_eq!(pins(SYMBOL), want);
}

/// The symbol names its own footprint, so the board draws the lands above.
#[test]
fn the_symbol_names_its_own_footprint() {
    let sym = library::symbol(SYMBOL).unwrap();
    assert_eq!(sym.property("Footprint"), Some(FOOTPRINT));
    assert_eq!(sym.property("Value"), Some("DRV8876PWP"));
}

/// One DRV8833 pin: number, name, TI's I/O column (`—` where it states
/// none) and the symbol's KiCad electrical type.
type TiPin = (&'static str, &'static str, &'static str, &'static str);

/// TI SLVSAR1E (January 2011, revised July 2015), Pin Functions, pp. 3–4,
/// the HTSSOP column; the PowerPAD is pin 17 ("must be connected to
/// ground"). `I` is `input`, `O` `output` (each drives a motor winding, never
/// a rail), `OD` `open_collector` and `IO` `bidirectional`, except AISEN and
/// BISEN, which are `passive` (ruled Q3, 2026-11-20): each is its bridge's
/// ground return, and p. 10 says "If current control is not needed, the
/// xISEN pins should be connected directly to ground", which ERC refuses for
/// a `bidirectional` pin on a flagged rail. Where TI states no type (`—`:
/// VM, VINT, GND, the PowerPAD) the supply pins are `power_in`, as KiCad's
/// own `Driver_Motor:DRV8833PWP` has them, and the pad is `passive`.
const DRV8833_PINS: [TiPin; 17] = [
    ("1", "~{SLEEP}", "I", "input"),
    ("2", "AOUT1", "O", "output"),
    ("3", "AISEN", "IO", "passive"),
    ("4", "AOUT2", "O", "output"),
    ("5", "BOUT2", "O", "output"),
    ("6", "BISEN", "IO", "passive"),
    ("7", "BOUT1", "O", "output"),
    ("8", "~{FAULT}", "OD", "open_collector"),
    ("9", "BIN1", "I", "input"),
    ("10", "BIN2", "I", "input"),
    ("11", "VCP", "IO", "bidirectional"),
    ("12", "VM", "—", "power_in"),
    ("13", "GND", "—", "power_in"),
    ("14", "VINT", "—", "power_in"),
    ("15", "AIN2", "I", "input"),
    ("16", "AIN1", "I", "input"),
    ("17", "GND", "—", "passive"),
];

const DRV8833_SYMBOL: &str = "Aethelgard_Driver:DRV8833PWP";

#[test]
fn the_drv8833_symbol_s_pins_are_its_pin_functions_table() {
    let want: Vec<ReadPin> = DRV8833_PINS
        .iter()
        .map(|&(n, name, _, kind)| (n.to_owned(), name.to_owned(), kind.to_owned()))
        .collect();
    assert_eq!(pins(DRV8833_SYMBOL), want);
}

/// Exactly AISEN and BISEN depart from TI's stated type (ruled Q3).
#[test]
fn only_aisen_and_bisen_depart_from_ti_s_types() {
    let natural = |ti: &str| match ti {
        "I" => Some("input"),
        "O" => Some("output"),
        "OD" => Some("open_collector"),
        "IO" => Some("bidirectional"),
        _ => None,
    };
    let departed: Vec<&str> = DRV8833_PINS
        .iter()
        .filter(|&&(_, _, ti, kind)| natural(ti).is_some_and(|k| k != kind))
        .map(|&(_, name, _, _)| name)
        .collect();
    assert_eq!(departed, ["AISEN", "BISEN"]);
}

/// Its footprint is KiCad's vendored PWP0016C, without thermal vias.
#[test]
fn the_drv8833_symbol_names_the_vendored_pwp0016c_footprint() {
    assert_eq!(
        library::symbol(DRV8833_SYMBOL)
            .unwrap()
            .property("Footprint"),
        Some("Package_SO:HTSSOP-16-1EP_4.4x5mm_P0.65mm_EP3.4x5mm_Mask2.46x2.31mm")
    );
}
