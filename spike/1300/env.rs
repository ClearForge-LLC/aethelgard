//! The library-configuration treatments S2 and S4 measure. In the image,
//! `kicad-cli` has no symbol or footprint library table, so ERC and DRC warn
//! that each library is "not in the current configuration". Each treatment is
//! something a gate could write into its scratch directory.

use std::fs;
use std::path::Path;

use super::{Ctx, answer, write};

#[derive(Clone, Copy, Debug)]
pub enum Treat {
    /// The file alone, as `aethelgard check` copies it today.
    Bare,
    /// Project-local `sym-lib-table` and `fp-lib-table`, no project file.
    Tables,
    /// The same tables beside a minimal `.kicad_pro`.
    TablesPro,
    /// A `.kicad_pro` that sets the library checks' severities to ignore.
    Severities,
    /// `KICAD_CONFIG_HOME` holding KiCad's own template tables, with the
    /// library directory variables set.
    ConfigHome,
}

pub const ALL: [Treat; 5] = [
    Treat::Bare,
    Treat::Tables,
    Treat::TablesPro,
    Treat::Severities,
    Treat::ConfigHome,
];

const SYM_LIBS: [&str; 4] = ["Device", "power", "Sensor", "MCU_ST_STM32H7"];
const FP_LIBS: [&str; 4] = [
    "Resistor_SMD",
    "Capacitor_SMD",
    "Package_QFP",
    "Package_LGA",
];
const TEMPLATE: &str = "/usr/share/kicad/template";

fn tables(ctx: &Ctx, dir: &Path) -> Result<(), String> {
    let libs = ctx.libs.as_ref().ok_or("no libraries")?;
    let mut sym = String::from("(sym_lib_table\n\t(version 7)\n");
    for l in SYM_LIBS {
        sym.push_str(&format!(
            "\t(lib (name \"{l}\") (type \"KiCad\") (uri \"{}\") (options \"\") (descr \"\"))\n",
            libs.symbol_file(l).display()
        ));
    }
    sym.push_str(")\n");
    let mut fp = String::from("(fp_lib_table\n\t(version 7)\n");
    for l in FP_LIBS {
        fp.push_str(&format!(
            "\t(lib (name \"{l}\") (type \"KiCad\") (uri \"{}\") (options \"\") (descr \"\"))\n",
            libs.fp_dir.join(format!("{l}.pretty")).display()
        ));
    }
    fp.push_str(")\n");
    write(&dir.join("sym-lib-table"), &sym)?;
    write(&dir.join("fp-lib-table"), &fp)
}

pub fn minimal_pro(stem: &str) -> String {
    format!("{{\n  \"meta\": {{ \"filename\": \"{stem}.kicad_pro\", \"version\": 1 }}\n}}\n")
}

pub fn severities_pro(stem: &str) -> String {
    format!(
        r#"{{
  "board": {{ "design_settings": {{ "meta": {{ "version": 2 }}, "rule_severities": {{ "lib_footprint_issues": "ignore", "lib_footprint_mismatch": "ignore" }} }} }},
  "erc": {{ "meta": {{ "version": 0 }}, "rule_severities": {{ "lib_symbol_issues": "ignore", "lib_symbol_mismatch": "ignore", "footprint_link_issues": "ignore" }} }},
  "meta": {{ "filename": "{stem}.kicad_pro", "version": 1 }}
}}
"#
    )
}

/// Writes the treatment into `dir` (whose design files are `<stem>.*`) and
/// returns the environment it needs. A `.kicad_pro` already there is kept
/// unless the treatment writes its own.
pub fn prepare(
    ctx: &Ctx,
    dir: &Path,
    stem: &str,
    t: Treat,
) -> Result<Vec<(String, String)>, String> {
    let libs = ctx.libs.as_ref().ok_or("no libraries")?;
    match t {
        Treat::Bare => Ok(Vec::new()),
        Treat::Tables => tables(ctx, dir).map(|_| Vec::new()),
        Treat::TablesPro => {
            tables(ctx, dir)?;
            let pro = dir.join(format!("{stem}.kicad_pro"));
            if !pro.exists() {
                write(&pro, &minimal_pro(stem))?;
            }
            Ok(Vec::new())
        }
        Treat::Severities => {
            write(
                &dir.join(format!("{stem}.kicad_pro")),
                &severities_pro(stem),
            )?;
            Ok(Vec::new())
        }
        Treat::ConfigHome => {
            let cfg = dir.join("kicad-config");
            let ver = cfg.join("10.0");
            fs::create_dir_all(&ver).map_err(|e| e.to_string())?;
            for t in ["sym-lib-table", "fp-lib-table"] {
                fs::copy(Path::new(TEMPLATE).join(t), ver.join(t))
                    .map_err(|e| format!("{TEMPLATE}/{t}: {e}"))?;
            }
            Ok(vec![
                (
                    "KICAD_CONFIG_HOME".into(),
                    cfg.to_string_lossy().into_owned(),
                ),
                (
                    "KICAD10_SYMBOL_DIR".into(),
                    libs.sym_dir.to_string_lossy().into_owned(),
                ),
                (
                    "KICAD10_FOOTPRINT_DIR".into(),
                    libs.fp_dir.to_string_lossy().into_owned(),
                ),
            ])
        }
    }
}

/// The template tables' first lines, once, so the table syntax is on record.
pub fn show_templates() {
    for t in ["sym-lib-table", "fp-lib-table"] {
        let p = Path::new(TEMPLATE).join(t);
        let text = fs::read_to_string(&p).unwrap_or_default();
        let head: Vec<&str> = text.lines().take(3).map(str::trim).collect();
        answer(
            "ENV",
            format!(
                "template {}: {} lines; head {}",
                p.display(),
                text.lines().count(),
                head.join(" ")
            ),
        );
    }
}
