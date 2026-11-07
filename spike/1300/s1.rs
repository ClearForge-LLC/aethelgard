//! S1: which library symbols and footprints the image has for the parts, and
//! under what licence.

use std::fs;
use std::path::Path;

use super::{Ctx, answer, kilib, run as run_cmd, sha256_hex};

const SYMBOLS: [&str; 6] = [
    "STM32H753ZI",
    "STM32H743ZI",
    "BME280",
    "TMP117",
    "W25Q64",
    "W25Q",
];

/// (library dir, name prefix)
const FOOTPRINTS: [(&str, &str); 7] = [
    ("Package_QFP", "LQFP-144"),
    ("Resistor_SMD", "R_0402_"),
    ("Resistor_SMD", "R_0603_"),
    ("Capacitor_SMD", "C_0402_"),
    ("Capacitor_SMD", "C_0603_"),
    ("Package_LGA", "Bosch_LGA-8"),
    ("Package_SON", "WSON-6"),
];

/// Every top-level symbol name in a library file's text (`(symbol "Name"`
/// lines that are not `Name_<unit>_<body>` sub-symbols).
fn names(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        let Some(rest) = t.strip_prefix("(symbol \"") else {
            continue;
        };
        let Some(name) = rest.split('"').next() else {
            continue;
        };
        let mut tail = name.rsplitn(3, '_');
        let sub = matches!(
            (
                tail.next().map(|b| b.parse::<u32>().is_ok()),
                tail.next().map(|u| u.parse::<u32>().is_ok())
            ),
            (Some(true), Some(true))
        );
        if !sub {
            out.push(name.to_owned());
        }
    }
    out
}

pub fn run(ctx: &mut Ctx) -> Result<(), String> {
    let Some(libs) = &ctx.libs else {
        answer(
            "S1",
            "blocked: no KiCad library here (it exists only in the tools image)",
        );
        return Ok(());
    };
    let sym_dir = libs.sym_dir.clone();
    let fp_dir = libs.fp_dir.clone();
    let mut files = 0;
    let mut dirs = 0;
    let mut lib_names = Vec::new();
    for e in fs::read_dir(&sym_dir).map_err(|e| e.to_string())? {
        let p = e.map_err(|e| e.to_string())?.path();
        let n = p.file_name().unwrap().to_string_lossy().into_owned();
        if let Some(lib) = n.strip_suffix(".kicad_sym") {
            files += 1;
            lib_names.push((lib.to_owned(), p.clone()));
        } else if let Some(lib) = n.strip_suffix(".kicad_symdir") {
            dirs += 1;
            lib_names.push((lib.to_owned(), p.clone()));
        }
    }
    lib_names.sort();
    let fp_libs = fs::read_dir(&fp_dir)
        .map_err(|e| e.to_string())?
        .filter(|e| {
            e.as_ref()
                .is_ok_and(|e| e.file_name().to_string_lossy().ends_with(".pretty"))
        })
        .count();
    answer(
        "S1",
        format!(
            "symbol libraries: {files} .kicad_sym files, {dirs} .kicad_symdir dirs in {}; footprint libraries: {fp_libs} .pretty in {}",
            sym_dir.display(),
            fp_dir.display()
        ),
    );
    if let Some((lib, path)) = lib_names.iter().find(|(l, _)| l == "Device") {
        let head: String = fs::read_to_string(if path.is_dir() {
            path.join("R.kicad_sym")
        } else {
            path.clone()
        })
        .unwrap_or_default()
        .lines()
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");
        answer("S1", format!("{lib} library header: {head}"));
    }
    provenance()?;
    // Symbol scan.
    for (lib, path) in &lib_names {
        let texts: Vec<String> = if path.is_dir() {
            fs::read_dir(path)
                .map_err(|e| e.to_string())?
                .filter_map(|e| e.ok())
                .filter_map(|e| fs::read_to_string(e.path()).ok())
                .collect()
        } else {
            vec![fs::read_to_string(path).map_err(|e| e.to_string())?]
        };
        for text in texts {
            for name in names(&text) {
                let up = name.to_ascii_uppercase();
                if SYMBOLS.iter().any(|s| up.contains(s)) {
                    ctx.found.insert(name.clone(), format!("{lib}:{name}"));
                }
            }
        }
    }
    let libs = ctx.libs.as_ref().unwrap();
    for (name, lib_id) in &ctx.found {
        let detail = match libs.symbol(lib_id) {
            Ok(sym) => {
                let pins = kilib::pins(&sym).map(|p| p.len()).unwrap_or(0);
                format!(
                    "pins={pins} Footprint={:?} ki_fp_filters={:?} extends_flattened=ok",
                    sym.property("Footprint").unwrap_or(""),
                    sym.property("ki_fp_filters").unwrap_or("")
                )
            }
            Err(e) => format!("load error: {e}"),
        };
        answer("S1", format!("symbol {lib_id} ({name}): {detail}"));
    }
    for want in SYMBOLS {
        if !ctx
            .found
            .keys()
            .any(|n| n.to_ascii_uppercase().contains(want))
        {
            answer("S1", format!("symbol matching {want}: NONE"));
        }
    }
    // Footprint scan.
    for (lib, prefix) in FOOTPRINTS {
        let dir = fp_dir.join(format!("{lib}.pretty"));
        let mut hits: Vec<String> = fs::read_dir(&dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .filter(|n| n.starts_with(prefix) || n.contains(prefix))
                    .map(|n| n.trim_end_matches(".kicad_mod").to_owned())
                    .collect()
            })
            .unwrap_or_default();
        hits.sort();
        answer(
            "S1",
            format!(
                "footprints {lib}:{prefix}*: {}",
                if hits.is_empty() {
                    "NONE".to_owned()
                } else {
                    hits.join(", ")
                }
            ),
        );
    }
    // The footprints the found symbols name.
    let libs = ctx.libs.as_ref().unwrap();
    for lib_id in ctx.found.values() {
        if let Ok(sym) = libs.symbol(lib_id)
            && let Some(fp) = sym.property("Footprint").filter(|f| !f.is_empty())
        {
            let exists = fp
                .split_once(':')
                .map(|(l, n)| libs.footprint_file(l, n).is_file())
                .unwrap_or(false);
            answer(
                "S1",
                format!("{lib_id} names footprint {fp}: exists={exists}"),
            );
        }
    }
    // Licences.
    let roots = [
        sym_dir.as_path(),
        fp_dir.as_path(),
        sym_dir.parent().unwrap_or(Path::new("/")),
        Path::new("/usr/share/doc"),
    ];
    let mut seen = Vec::new();
    for root in roots {
        let found = run_cmd(
            "find",
            &[
                &root.to_string_lossy(),
                "-maxdepth",
                "3",
                "(",
                "-iname",
                "*licen*",
                "-o",
                "-iname",
                "copying*",
                "-o",
                "-iname",
                "copyright",
                ")",
                "-type",
                "f",
            ],
            Path::new("/"),
        )?;
        for line in found.stdout.lines() {
            let lower = line.to_ascii_lowercase();
            let relevant = lower.contains("kicad")
                || line.starts_with(&*sym_dir.to_string_lossy())
                || line.starts_with(&*fp_dir.to_string_lossy());
            if !relevant || seen.contains(&line.to_owned()) {
                continue;
            }
            seen.push(line.to_owned());
            let bytes = fs::read(line).unwrap_or_default();
            answer(
                "S1",
                format!(
                    "licence file {line}: {} bytes sha256 {}",
                    bytes.len(),
                    sha256_hex(&bytes)
                ),
            );
            if line.contains("/demos/") {
                let first = String::from_utf8_lossy(&bytes)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_owned();
                answer(
                    "S1",
                    format!("  (a demo project's licence, not the libraries'): {first}"),
                );
                continue;
            }
            let text = String::from_utf8_lossy(&bytes);
            let mut shown = 0;
            for l in text.lines() {
                let ll = l.to_ascii_lowercase();
                if [
                    "cc-by",
                    "creative commons",
                    "exception",
                    "waive",
                    "adapted material",
                    "generated",
                    "license:",
                    "licence",
                ]
                .iter()
                .any(|k| ll.contains(k))
                {
                    answer("S1", format!("  {}: {}", line, l.trim()));
                    shown += 1;
                    if shown >= 25 {
                        break;
                    }
                }
            }
        }
    }
    if seen.is_empty() {
        answer(
            "S1",
            "licence file: none found under the library roots or /usr/share/doc",
        );
    }
    // The library files the probe embeds from, pinned by hash.
    for lib in ["Device", "power", "Sensor", "MCU_ST_STM32H7"] {
        let p = libs.symbol_file(lib);
        if p.is_file() {
            let bytes = fs::read(&p).unwrap_or_default();
            answer(
                "S1",
                format!(
                    "library file {}: sha256 {}",
                    p.display(),
                    sha256_hex(&bytes)
                ),
            );
        }
    }
    Ok(())
}

/// Where the libraries came from, so the licence can be fetched at the
/// matching version: the installed packages, and whether the image drops
/// `/usr/share/doc` (where Debian packages put their copyright files).
fn provenance() -> Result<(), String> {
    let cmds = [
        "dpkg-query -W -f '${Package} ${Version}\\n' 'kicad*' 2>&1",
        "ls /etc/dpkg/dpkg.cfg.d/ 2>&1; cat /etc/dpkg/dpkg.cfg.d/* 2>&1 | grep -v '^#' | grep -v '^$'",
        "for p in $(dpkg-query -W -f '${Package}\\n' 'kicad*' 2>/dev/null); do dpkg -L \"$p\" 2>/dev/null | grep -i -E 'copyright|licen|copying'; done",
        "find / -xdev -ipath '*kicad*' \\( -iname '*licen*' -o -iname copyright -o -iname 'copying*' \\) -not -path '*/demos/*' 2>/dev/null",
        "ls /usr/share/doc 2>&1 | grep -i kicad",
    ];
    for c in cmds {
        let r = run_cmd("sh", &["-c", c], Path::new("/"))?;
        let out: Vec<&str> = r
            .stdout
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        answer(
            "S1",
            format!(
                "provenance `{c}`: {}",
                if out.is_empty() {
                    "(nothing)".to_owned()
                } else {
                    out.join(" / ")
                }
            ),
        );
    }
    Ok(())
}
