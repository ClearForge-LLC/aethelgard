//! The vendored KiCad 10.0.7 symbols and footprints (`data/kicad-10.0.7/`,
//! ruled D1 on PR #40): each file's sha256 is checked before any is used, and
//! symbols come out flattened, renamed to their `lib:name` id and without
//! `ki_fp_filters` (D3: KiCad 10's parity check asks for that field on a
//! footprint, where no property of that name satisfies it).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use sha2::{Digest, Sha256};

use crate::sexpr::{self, Sx};

/// The library's release, as the tools image carries it.
pub const KICAD_TAG: &str = "10.0.7";

/// Every vendored file: its path under `data/kicad-10.0.7/`, its sha256, and
/// its bytes.
type Vendored = (&'static str, &'static str, &'static [u8]);

const FILES: [Vendored; 17] = [
    (
        "footprints/Capacitor_SMD.pretty/C_0402_1005Metric.kicad_mod",
        "0403382fc4583ed510b461b1fa4a36dfaec6f4c0d9b1a67e6b0027837a54e1b5",
        include_bytes!(
            "../data/kicad-10.0.7/footprints/Capacitor_SMD.pretty/C_0402_1005Metric.kicad_mod"
        ),
    ),
    (
        "footprints/Package_LGA.pretty/Bosch_LGA-8_2.5x2.5mm_P0.65mm_ClockwisePinNumbering.kicad_mod",
        "14464b7a437e7efb0fa9947b89a7d3825242f3ed82d649fe2ca45fad5d3dda86",
        include_bytes!(
            "../data/kicad-10.0.7/footprints/Package_LGA.pretty/Bosch_LGA-8_2.5x2.5mm_P0.65mm_ClockwisePinNumbering.kicad_mod"
        ),
    ),
    (
        "footprints/Package_QFP.pretty/LQFP-144_20x20mm_P0.5mm.kicad_mod",
        "c56afb80b84306b4ae3639fdd270987fc47b46ee531bbae036cc2ee25f2cf165",
        include_bytes!(
            "../data/kicad-10.0.7/footprints/Package_QFP.pretty/LQFP-144_20x20mm_P0.5mm.kicad_mod"
        ),
    ),
    (
        "footprints/Package_SON.pretty/WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm.kicad_mod",
        "5b7efafe23ba0a80faf2aa5786e7ceed526eb3802b254bc28577f1fc06abb633",
        include_bytes!(
            "../data/kicad-10.0.7/footprints/Package_SON.pretty/WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm.kicad_mod"
        ),
    ),
    (
        "footprints/Package_SO.pretty/SOIC-8_5.3x5.3mm_P1.27mm.kicad_mod",
        "cc86a7083c9df1c1bf808dfa469e913d446455fd7ab7a7941e18681b80e8b9b5",
        include_bytes!(
            "../data/kicad-10.0.7/footprints/Package_SO.pretty/SOIC-8_5.3x5.3mm_P1.27mm.kicad_mod"
        ),
    ),
    (
        "footprints/Resistor_SMD.pretty/R_0402_1005Metric.kicad_mod",
        "e05c7605248c220836f642ed1f526133edf0374acdfd5bb2631e58b4e377acfb",
        include_bytes!(
            "../data/kicad-10.0.7/footprints/Resistor_SMD.pretty/R_0402_1005Metric.kicad_mod"
        ),
    ),
    (
        "symbols/Device.kicad_symdir/C.kicad_sym",
        "533df5a016c224a83701e2e66b767864f39fc44c972a2a4f821987f1fd99573e",
        include_bytes!("../data/kicad-10.0.7/symbols/Device.kicad_symdir/C.kicad_sym"),
    ),
    (
        "symbols/Device.kicad_symdir/R.kicad_sym",
        "db1dd3eb60dbcc9e239d107e2d123ee253aefdd841186076e9df39444d4e11c7",
        include_bytes!("../data/kicad-10.0.7/symbols/Device.kicad_symdir/R.kicad_sym"),
    ),
    (
        "symbols/MCU_ST_STM32H7.kicad_symdir/STM32H753ZITx.kicad_sym",
        "257de01b80c04c8157fd1b617b5d045d9c8a4cf516ad9e1e23156f2ebe716e6b",
        include_bytes!(
            "../data/kicad-10.0.7/symbols/MCU_ST_STM32H7.kicad_symdir/STM32H753ZITx.kicad_sym"
        ),
    ),
    (
        "symbols/Memory_Flash.kicad_symdir/W25Q32JVSS.kicad_sym",
        "49133cae4075c838798a68067881f10d0836a3316f7de485f334d92d71569550",
        include_bytes!(
            "../data/kicad-10.0.7/symbols/Memory_Flash.kicad_symdir/W25Q32JVSS.kicad_sym"
        ),
    ),
    (
        "symbols/power.kicad_symdir/+3V3.kicad_sym",
        "7a07c7939a39de227db5a8ed79e8be88d1b1e60b3d9b5dd8afc2f4be41ad2d57",
        include_bytes!("../data/kicad-10.0.7/symbols/power.kicad_symdir/+3V3.kicad_sym"),
    ),
    (
        "symbols/power.kicad_symdir/GND.kicad_sym",
        "2505811766064acf321cd3aae4e72ef96d35da64c1ed4ca8d6a24271c9cf7734",
        include_bytes!("../data/kicad-10.0.7/symbols/power.kicad_symdir/GND.kicad_sym"),
    ),
    (
        "symbols/power.kicad_symdir/PWR_FLAG.kicad_sym",
        "5308c8c77530d5ba458ca658925119069cb6cb8ad6fbf424d88afb1fdb4e3ef1",
        include_bytes!("../data/kicad-10.0.7/symbols/power.kicad_symdir/PWR_FLAG.kicad_sym"),
    ),
    (
        "symbols/Sensor.kicad_symdir/BME280.kicad_sym",
        "d194dfcae6dc11d852b349df938c2b95f08ef803bababdf2792ee2bec11cb55a",
        include_bytes!("../data/kicad-10.0.7/symbols/Sensor.kicad_symdir/BME280.kicad_sym"),
    ),
    (
        "symbols/Sensor_Temperature.kicad_symdir/TMP102xxDRL.kicad_sym",
        "ba51c59e932af56e44ae300fa8712a53ae1396d3af3f9ef7c609e665037b453f",
        include_bytes!(
            "../data/kicad-10.0.7/symbols/Sensor_Temperature.kicad_symdir/TMP102xxDRL.kicad_sym"
        ),
    ),
    (
        "symbols/Sensor_Temperature.kicad_symdir/TMP117xxDRV.kicad_sym",
        "1d1a1427556c4e89fc7d30fcbd418120ebbe9c0b774cabda08843ac5f6742848",
        include_bytes!(
            "../data/kicad-10.0.7/symbols/Sensor_Temperature.kicad_symdir/TMP117xxDRV.kicad_sym"
        ),
    ),
    (
        "LICENSE.md",
        "45d2bce75e5a4208f5afb01b8fb2c406e700371c4fe2b5f5cd5c443d46db4d8f",
        include_bytes!("../data/kicad-10.0.7/LICENSE.md"),
    ),
];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LibraryError {
    #[error("vendored `{path}` has sha256 {actual}, not {expected}: a corrupted build")]
    HashMismatch {
        path: &'static str,
        expected: &'static str,
        actual: String,
    },
    #[error("vendored `{0}` is not a KiCad S-expression: {1}")]
    Unparseable(&'static str, sexpr::SexprError),
    #[error("no vendored symbol `{0}`")]
    NoSymbol(String),
    #[error("no vendored footprint `{0}`")]
    NoFootprint(String),
    #[error("symbol `{0}` extends past a depth of 4")]
    ExtendsTooDeep(String),
}

/// The checked files, or the first that failed its hash; checked once.
fn checked() -> Result<&'static [Vendored], LibraryError> {
    static CHECK: OnceLock<Result<(), LibraryError>> = OnceLock::new();
    CHECK
        .get_or_init(|| {
            for (path, expected, bytes) in FILES {
                let actual: String = Sha256::digest(bytes)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                if actual != expected {
                    return Err(LibraryError::HashMismatch {
                        path,
                        expected,
                        actual,
                    });
                }
            }
            Ok(())
        })
        .clone()
        .map(|()| &FILES[..])
}

fn file(path: &str) -> Result<Option<(&'static str, &'static [u8])>, LibraryError> {
    Ok(checked()?
        .iter()
        .find(|(p, _, _)| *p == path)
        .map(|(p, _, b)| (*p, *b)))
}

/// One symbol file's top-level `(symbol "name" …)`.
fn raw_symbol(lib: &str, name: &str) -> Result<Sx, LibraryError> {
    let path = format!("symbols/{lib}.kicad_symdir/{name}.kicad_sym");
    let (path, bytes) =
        file(&path)?.ok_or_else(|| LibraryError::NoSymbol(format!("{lib}:{name}")))?;
    let text = std::str::from_utf8(bytes)
        .map_err(|_| LibraryError::Unparseable(path, sexpr::SexprError::NotUtf8(0)))?;
    let lib_file = sexpr::parse(text).map_err(|e| LibraryError::Unparseable(path, e))?;
    lib_file
        .children("symbol")
        .find(|s| s.arg(1) == Some(name))
        .cloned()
        .ok_or_else(|| LibraryError::NoSymbol(format!("{lib}:{name}")))
}

/// The flags a derived symbol may restate; its own value wins.
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

fn flatten(lib: &str, name: &str, depth: usize) -> Result<Sx, LibraryError> {
    if depth > 4 {
        return Err(LibraryError::ExtendsTooDeep(format!("{lib}:{name}")));
    }
    let sym = raw_symbol(lib, name)?;
    let Some(parent_name) = sym
        .child("extends")
        .and_then(|e| e.arg(1))
        .map(str::to_owned)
    else {
        return Ok(sym);
    };
    let parent = flatten(lib, &parent_name, depth + 1)?;
    let own: Vec<&Sx> = sym.items()[2..]
        .iter()
        .filter(|i| i.head() != Some("extends"))
        .collect();
    let overridden = |item: &Sx| {
        own.iter().any(|o| {
            o.head() == item.head() && (item.head() != Some("property") || o.arg(1) == item.arg(1))
        })
    };
    let mut out = vec![Sx::atom("symbol"), Sx::str(name)];
    out.extend(own.iter().map(|o| (*o).clone()));
    for item in &parent.items()[2..] {
        let head = item.head().unwrap_or("");
        if (head == "property" || FLAGS.contains(&head)) && overridden(item) {
            continue;
        }
        let mut item = item.clone();
        if head == "symbol"
            && let Some(rest) = item
                .arg(1)
                .and_then(|s| s.strip_prefix(parent_name.as_str()))
                .map(str::to_owned)
            && let Some(items) = item.items_mut()
        {
            items[1] = Sx::str(format!("{name}{rest}"));
        }
        out.push(item);
    }
    Ok(Sx::List(out))
}

/// `lib:name`, flattened and renamed as a schematic's `lib_symbols` embeds
/// it, without `ki_fp_filters`.
pub fn symbol(lib_id: &str) -> Result<Sx, LibraryError> {
    let (lib, name) = lib_id
        .split_once(':')
        .ok_or_else(|| LibraryError::NoSymbol(lib_id.to_owned()))?;
    let mut sym = flatten(lib, name, 0)?;
    if let Some(items) = sym.items_mut() {
        items[1] = Sx::str(lib_id);
        items.retain(|i| !(i.head() == Some("property") && i.arg(1) == Some("ki_fp_filters")));
    }
    Ok(sym)
}

/// A footprint, `lib:name`, as its `.kicad_mod` holds it.
pub fn footprint(lib_id: &str) -> Result<Sx, LibraryError> {
    let (lib, name) = lib_id
        .split_once(':')
        .ok_or_else(|| LibraryError::NoFootprint(lib_id.to_owned()))?;
    let path = format!("footprints/{lib}.pretty/{name}.kicad_mod");
    let (path, bytes) = file(&path)?.ok_or_else(|| LibraryError::NoFootprint(lib_id.to_owned()))?;
    let text = std::str::from_utf8(bytes)
        .map_err(|_| LibraryError::Unparseable(path, sexpr::SexprError::NotUtf8(0)))?;
    sexpr::parse(text).map_err(|e| LibraryError::Unparseable(path, e))
}

/// Every vendored file the gate writes into its scratch directory, by its
/// path relative to a `lib/` directory there, the licence included.
pub fn files() -> Result<Vec<(&'static str, &'static [u8])>, LibraryError> {
    Ok(checked()?.iter().map(|(p, _, b)| (*p, *b)).collect())
}

/// `(sym-lib-table, fp-lib-table)`, project-local, naming every vendored
/// library under `lib_dir` (an absolute path the gate chose). The gate
/// writes both beside the copy it judges (ruled D3).
pub fn tables(lib_dir: &str) -> Result<(String, String), LibraryError> {
    let mut sym = BTreeMap::new();
    let mut fp = BTreeMap::new();
    for (path, _, _) in checked()? {
        let mut parts = path.split('/');
        match (parts.next(), parts.next()) {
            (Some("symbols"), Some(dir)) => {
                if let Some(lib) = dir.strip_suffix(".kicad_symdir") {
                    sym.insert(lib, format!("{lib_dir}/symbols/{dir}"));
                }
            }
            (Some("footprints"), Some(dir)) => {
                if let Some(lib) = dir.strip_suffix(".pretty") {
                    fp.insert(lib, format!("{lib_dir}/footprints/{dir}"));
                }
            }
            _ => {}
        }
    }
    let table = |kind: &str, libs: &BTreeMap<&str, String>| {
        let mut out = format!("({kind}\n\t(version 7)\n");
        for (name, uri) in libs {
            out.push_str(&format!(
                "\t(lib (name {}) (type \"KiCad\") (uri {}) (options \"\") (descr \"\"))\n",
                sexpr::quote(name),
                sexpr::quote(uri)
            ));
        }
        out.push_str(")\n");
        out
    };
    Ok((table("sym_lib_table", &sym), table("fp_lib_table", &fp)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vendored_file_matches_its_hash() {
        assert_eq!(files().unwrap().len(), FILES.len());
    }

    #[test]
    fn a_derived_symbol_is_flattened_and_keeps_its_own_fields() {
        let tmp117 = symbol("Sensor_Temperature:TMP117xxDRV").unwrap();
        assert_eq!(tmp117.arg(1), Some("Sensor_Temperature:TMP117xxDRV"));
        assert!(tmp117.child("extends").is_none());
        assert_eq!(
            tmp117.property("Footprint"),
            Some("Package_SON:WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm")
        );
        assert_eq!(tmp117.property("Value"), Some("TMP117xxDRV"));
        let subs: Vec<&str> = tmp117.children("symbol").filter_map(|s| s.arg(1)).collect();
        assert!(
            subs.iter().all(|s| s.starts_with("TMP117xxDRV_")),
            "{subs:?}"
        );
        assert_eq!(crate::schematic::pins(&tmp117).unwrap().len(), 6);
    }

    #[test]
    fn ki_fp_filters_is_dropped_from_every_symbol() {
        for id in [
            "Device:R",
            "Device:C",
            "Sensor:BME280",
            "Sensor_Temperature:TMP117xxDRV",
            "Memory_Flash:W25Q32JVSS",
            "MCU_ST_STM32H7:STM32H753ZITx",
        ] {
            let s = symbol(id).unwrap();
            assert_eq!(s.property("ki_fp_filters"), None, "{id}");
            assert!(s.property("Reference").is_some(), "{id}");
        }
    }

    #[test]
    fn footprints_parse_and_carry_no_local_clearance() {
        for (path, _, bytes) in FILES {
            if path.starts_with("footprints/") {
                let text = std::str::from_utf8(bytes).unwrap();
                assert_eq!(sexpr::parse(text).unwrap().head(), Some("footprint"));
                assert!(!text.contains("(clearance"), "{path}");
            }
        }
    }

    #[test]
    fn the_tables_name_every_vendored_library() {
        let (sym, fp) = tables("/s/lib").unwrap();
        assert!(sym.contains(
            "(lib (name \"Device\") (type \"KiCad\") (uri \"/s/lib/symbols/Device.kicad_symdir\")"
        ));
        assert!(fp.contains("(uri \"/s/lib/footprints/Package_QFP.pretty\")"));
        assert_eq!(sym.matches("(lib ").count(), 6);
        assert_eq!(fp.matches("(lib ").count(), 6);
        assert!(sexpr::parse(&sym).is_ok() && sexpr::parse(&fp).is_ok());
    }
}
