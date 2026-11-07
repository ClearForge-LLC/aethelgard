//! KiCad library access for the spike: find the image's symbol and footprint
//! libraries, copy one symbol out flattened (`extends` resolved) and renamed
//! to its `lib:name` id, and read a symbol's pins.

use std::fs;
use std::path::{Path, PathBuf};

use super::sexpr::{self, Sx};

pub struct Libs {
    pub sym_dir: PathBuf,
    pub fp_dir: PathBuf,
}

impl Libs {
    /// KiCad's own variables first, then the official image's install path.
    pub fn find() -> Result<Libs, String> {
        let pick = |vars: &[&str], fallback: &str| -> Option<PathBuf> {
            vars.iter()
                .filter_map(|v| std::env::var_os(v).map(PathBuf::from))
                .chain([PathBuf::from(fallback)])
                .find(|p| p.is_dir())
        };
        let sym_dir = pick(
            &["KICAD10_SYMBOL_DIR", "KICAD9_SYMBOL_DIR"],
            "/usr/share/kicad/symbols",
        )
        .ok_or("no KiCad symbol library directory")?;
        let fp_dir = pick(
            &["KICAD10_FOOTPRINT_DIR", "KICAD9_FOOTPRINT_DIR"],
            "/usr/share/kicad/footprints",
        )
        .ok_or("no KiCad footprint library directory")?;
        Ok(Libs { sym_dir, fp_dir })
    }

    pub fn symbol_file(&self, lib: &str) -> PathBuf {
        self.sym_dir.join(format!("{lib}.kicad_sym"))
    }

    pub fn footprint_file(&self, lib: &str, name: &str) -> PathBuf {
        self.fp_dir
            .join(format!("{lib}.pretty"))
            .join(format!("{name}.kicad_mod"))
    }

    /// `lib:name`, flattened, as `lib_symbols` embeds it. A library is a
    /// `.kicad_sym` file or a `.kicad_symdir` directory of one file per symbol.
    pub fn symbol(&self, lib_id: &str) -> Result<Sx, String> {
        let (lib, name) = lib_id
            .split_once(':')
            .ok_or_else(|| format!("`{lib_id}` has no library"))?;
        let path = self.symbol_file(lib);
        if path.is_file() {
            let file = read_sx(&path)?;
            return embed(&|n: &str| top_symbol(&file, n), lib, name);
        }
        let dir = self.sym_dir.join(format!("{lib}.kicad_symdir"));
        if dir.is_dir() {
            let find = |n: &str| top_symbol(&read_sx(&dir.join(format!("{n}.kicad_sym")))?, n);
            return embed(&find, lib, name);
        }
        Err(format!("no library `{lib}` in {}", self.sym_dir.display()))
    }

    pub fn footprint(&self, lib_id: &str) -> Result<Sx, String> {
        let (lib, name) = lib_id
            .split_once(':')
            .ok_or_else(|| format!("`{lib_id}` has no library"))?;
        read_sx(&self.footprint_file(lib, name))
    }
}

pub fn read_sx(path: &Path) -> Result<Sx, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    sexpr::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn top_symbol(file: &Sx, name: &str) -> Result<Sx, String> {
    file.children("symbol")
        .find(|s| s.arg(1) == Some(name))
        .cloned()
        .ok_or_else(|| format!("no symbol `{name}`"))
}

type Find<'a> = &'a dyn Fn(&str) -> Result<Sx, String>;

/// The library's `name`, flattened and renamed `lib:name`.
pub fn embed(find: Find<'_>, lib: &str, name: &str) -> Result<Sx, String> {
    let mut flat = flatten(find, name, 0)?;
    if let Some(items) = flat.items_mut() {
        items[1] = Sx::str(format!("{lib}:{name}"));
    }
    Ok(flat)
}

const FLAGS: [&str; 8] = [
    "power",
    "pin_names",
    "pin_numbers",
    "exclude_from_sim",
    "in_bom",
    "on_board",
    "embedded_fonts",
    "duplicate_pin_numbers_are_jumpers",
];

fn flatten(find: Find<'_>, name: &str, depth: usize) -> Result<Sx, String> {
    if depth > 8 {
        return Err(format!("`{name}` extends too deep"));
    }
    let sym = find(name)?;
    let Some(parent_name) = sym.child("extends").and_then(|e| e.arg(1)) else {
        return Ok(sym);
    };
    let parent = flatten(find, parent_name, depth + 1)?;
    let own: Vec<&Sx> = sym.items()[2..]
        .iter()
        .filter(|i| i.head() != Some("extends"))
        .collect();
    let overrides = |item: &Sx| {
        own.iter().any(|o| {
            o.head() == item.head() && (item.head() != Some("property") || o.arg(1) == item.arg(1))
        })
    };
    let mut out = vec![Sx::atom("symbol"), Sx::str(name)];
    out.extend(own.iter().map(|o| (*o).clone()));
    for item in &parent.items()[2..] {
        let head = item.head().unwrap_or("");
        if (head == "property" || FLAGS.contains(&head)) && overrides(item) {
            continue;
        }
        let mut item = item.clone();
        if head == "symbol"
            && let Some(sub) = item.arg(1).map(str::to_owned)
            && let Some(rest) = sub.strip_prefix(parent_name)
            && let Some(items) = item.items_mut()
        {
            items[1] = Sx::str(format!("{name}{rest}"));
        }
        out.push(item);
    }
    Ok(Sx::list(out))
}

#[derive(Clone, Debug)]
pub struct Pin {
    pub number: String,
    pub name: String,
    pub etype: String,
    /// Library coordinates, nanometres, Y up.
    pub at: (i64, i64),
    pub angle: i64,
    pub hidden: bool,
}

/// Unit 1 (and the common unit 0), body style 1, of an embedded symbol.
pub fn pins(sym: &Sx) -> Result<Vec<Pin>, String> {
    let mut out = Vec::new();
    for sub in sym.children("symbol") {
        let sub_name = sub.arg(1).unwrap_or("");
        let mut parts = sub_name.rsplitn(3, '_');
        let body = parts.next().and_then(|b| b.parse::<u32>().ok());
        let unit = parts.next().and_then(|u| u.parse::<u32>().ok());
        let (Some(unit), Some(body)) = (unit, body) else {
            return Err(format!("sub-symbol `{sub_name}` has no unit/body suffix"));
        };
        if unit > 1 || body > 1 {
            continue;
        }
        for pin in sub.children("pin") {
            let at = pin.child("at").ok_or("pin without at")?;
            let coord = |i| {
                at.arg(i)
                    .ok_or_else(|| "short at".to_owned())
                    .and_then(sexpr::nm)
            };
            let angle = at.arg(3).unwrap_or("0");
            out.push(Pin {
                number: pin
                    .child("number")
                    .and_then(|n| n.arg(1))
                    .ok_or("pin without number")?
                    .to_owned(),
                name: pin
                    .child("name")
                    .and_then(|n| n.arg(1))
                    .ok_or("pin without name")?
                    .to_owned(),
                etype: pin.arg(1).ok_or("pin without type")?.to_owned(),
                at: (coord(1)?, coord(2)?),
                angle: angle
                    .parse::<f64>()
                    .map_err(|_| format!("pin angle `{angle}`"))? as i64,
                hidden: pin.items().iter().any(|i| i.text() == Some("hide"))
                    || pin.child("hide").and_then(|h| h.arg(1)) == Some("yes"),
            });
        }
    }
    Ok(out)
}

/// A built-in library used only when the image's is absent (Furnace), so the
/// emitter's mechanics and determinism can be checked without KiCad. It is
/// shaped like KiCad's `Device:R`, `power:+3V3` and `power:PWR_FLAG`.
pub const SYNTHETIC: &str = r##"(kicad_symbol_lib (version 20231120) (generator "spike")
 (symbol "R" (pin_numbers (hide yes)) (pin_names (offset 0)) (exclude_from_sim no) (in_bom yes) (on_board yes)
  (property "Reference" "R" (at 2.032 0 90) (effects (font (size 1.27 1.27))))
  (property "Value" "R" (at 0 0 90) (effects (font (size 1.27 1.27))))
  (property "Footprint" "" (at -1.778 0 90) (effects (font (size 1.27 1.27)) (hide yes)))
  (property "Datasheet" "~" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
  (symbol "R_0_1" (rectangle (start -1.016 -2.54) (end 1.016 2.54) (stroke (width 0.254) (type default)) (fill (type none))))
  (symbol "R_1_1"
   (pin passive line (at 0 3.81 270) (length 1.27) (name "~" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))
   (pin passive line (at 0 -3.81 90) (length 1.27) (name "~" (effects (font (size 1.27 1.27)))) (number "2" (effects (font (size 1.27 1.27)))))))
 (symbol "SENS" (extends "R")
  (property "Reference" "U" (at 0 0 0) (effects (font (size 1.27 1.27))))
  (property "Value" "SENS" (at 0 0 0) (effects (font (size 1.27 1.27)))))
 (symbol "+3V3" (power) (pin_numbers (hide yes)) (pin_names (offset 0) (hide yes)) (exclude_from_sim no) (in_bom yes) (on_board yes)
  (property "Reference" "#PWR" (at 0 -3.81 0) (effects (font (size 1.27 1.27)) (hide yes)))
  (property "Value" "+3V3" (at 0 3.556 0) (effects (font (size 1.27 1.27))))
  (property "Footprint" "" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
  (property "Datasheet" "" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
  (symbol "+3V3_1_1" (pin power_in line (at 0 0 90) (length 0) (hide yes) (name "+3V3" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27)))))))
 (symbol "PWR_FLAG" (power) (pin_numbers (hide yes)) (pin_names (offset 0) (hide yes)) (exclude_from_sim no) (in_bom yes) (on_board yes)
  (property "Reference" "#FLG" (at 0 1.905 0) (effects (font (size 1.27 1.27)) (hide yes)))
  (property "Value" "PWR_FLAG" (at 0 3.81 0) (effects (font (size 1.27 1.27))))
  (property "Footprint" "" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
  (property "Datasheet" "~" (at 0 0 0) (effects (font (size 1.27 1.27)) (hide yes)))
  (symbol "PWR_FLAG_0_1" (pin power_out line (at 0 0 90) (length 0) (name "pwr" (effects (font (size 1.27 1.27)))) (number "1" (effects (font (size 1.27 1.27))))))))
"##;

/// A footprint shaped like KiCad's `R_0603_1608Metric`, for the same
/// Furnace-only checks as `SYNTHETIC`.
pub const SYNTHETIC_FP: &str = r#"(footprint "R_0603_1608Metric" (version 20240108) (generator "spike") (layer "F.Cu")
 (property "Reference" "REF**" (at 0 -1.43 0) (layer "F.SilkS") (uuid "00000000-0000-0000-0000-000000000001") (effects (font (size 1 1) (thickness 0.15))))
 (property "Value" "R" (at 0 1.43 0) (layer "F.Fab") (uuid "00000000-0000-0000-0000-000000000002") (effects (font (size 1 1) (thickness 0.15))))
 (attr smd)
 (fp_rect (start -1.48 -0.73) (end 1.48 0.73) (stroke (width 0.05) (type solid)) (fill none) (layer "F.CrtYd") (uuid "00000000-0000-0000-0000-000000000003"))
 (pad "1" smd roundrect (at -0.825 0) (size 0.8 0.95) (layers "F.Cu" "F.Mask" "F.Paste") (roundrect_rratio 0.25) (uuid "00000000-0000-0000-0000-000000000004"))
 (pad "2" smd roundrect (at 0.825 0) (size 0.8 0.95) (layers "F.Cu" "F.Mask" "F.Paste") (roundrect_rratio 0.25) (uuid "00000000-0000-0000-0000-000000000005")))
"#;
