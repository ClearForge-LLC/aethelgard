//! `AET-WO-4408` (the Sentinel-01 review F7, `INV-07`): ST's pin table,
//! the single source `verify`'s gates read for each AF number and pad
//! position, held against two sources that do not come from it.
//!
//! - **DS12117 Rev 10** (`tests/fixtures/ds12117/`, transcribed by hand from
//!   the PDF): the LQFP144 pad of every pin (Table 8), and the AF number of
//!   every signal in the families the solver assigns (Table 9). Tier 1.
//! - **ArduPilot's `STM32H753xx.py`** at `Copter-4.7.2` (`fb773567`), the
//!   table `check pinout`'s ArduPilot resolves `hwdef.dat` against. It is
//!   GPL-3.0-or-later, and no byte of it is committed here
//!   (`container/ARDUPILOT-LICENSE.md`). `ardupilot-image.yml` copies it out
//!   of the pinned image and runs the one `#[ignore]`d test with
//!   `AETHELGARD_ARDUPILOT_AF_TABLE` naming it. Its sha256 is pinned below.
//!
//! Each pair is compared on its own, and every difference fails, naming the
//! row and both values. There is no two-of-three vote: a row that ST and
//! ArduPilot agree on still fails when DS12117 differs.

use std::collections::{BTreeMap, BTreeSet};

use aethelgard_svd_hal::pin_table;
use sha2::{Digest, Sha256};

/// (pin, signal) to AF number.
type Afs = BTreeMap<(String, String), u8>;

const PADS: &str = include_str!("fixtures/ds12117/pads.tsv");
const DS_AFS: &str = include_str!("fixtures/ds12117/af.tsv");

const ARDUPILOT_ENV: &str = "AETHELGARD_ARDUPILOT_AF_TABLE";
/// `libraries/AP_HAL_ChibiOS/hwdef/scripts/STM32H753xx.py` at ArduPilot
/// `fb773567896014da007bca58358704d4a0d8516f`, as the pinned ArduPilot image
/// holds it (measured, `spike/4408`).
const ARDUPILOT_SHA256: &str = "58fd32c3dd2c2049d541bb18189f16eb6fc6dc6c4cc04dba7c36a6bb4542c0b4";

/// The signals the solver can assign a pin (`solve.rs`: `Kind::I2c`,
/// `Kind::Spi`, a serial port's `{p}_TX`/`{p}_RX`, `motor::timer_channel`),
/// written here from their names, not imported.
fn in_families(signal: &str) -> bool {
    let Some((inst, sig)) = signal.split_once('_') else {
        return false;
    };
    let numbered = |prefix: &str| {
        inst.strip_prefix(prefix)
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    };
    (numbered("I2C") && matches!(sig, "SCL" | "SDA"))
        || (numbered("SPI") && matches!(sig, "SCK" | "MISO" | "MOSI"))
        || ((numbered("USART") || numbered("UART")) && matches!(sig, "TX" | "RX"))
        || (numbered("TIM") && matches!(sig, "CH1" | "CH2" | "CH3" | "CH4"))
}

/// The pin a pad name prints, if it is a GPIO: the name up to its first
/// `-`, space or `(` must be `P[A-K]` and one or two digits. `PC14-OSC32_IN
/// (OSC32_IN)` is `PC14`; `PA0_C`, an analog pad, is no GPIO.
fn gpio(name: &str) -> Option<&str> {
    let head = name.split(['-', ' ', '(']).next().unwrap_or("");
    let b = head.as_bytes();
    let ok = b.len() >= 3
        && b.len() <= 4
        && b[0] == b'P'
        && (b'A'..=b'K').contains(&b[1])
        && b[2..].iter().all(u8::is_ascii_digit);
    ok.then_some(head)
}

/// The data lines of a fixture: `#` comments and blank lines skipped, each
/// line split on tabs into exactly `n` fields.
fn rows(text: &str, n: usize) -> impl Iterator<Item = Vec<&str>> {
    text.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(move |l| {
            let f: Vec<&str> = l.split('\t').collect();
            assert_eq!(f.len(), n, "not {n} fields: {l:?}");
            f
        })
}

/// DS12117's LQFP144 pads: position to the name as printed.
fn ds_pads() -> BTreeMap<u16, String> {
    let mut pads = BTreeMap::new();
    for f in rows(PADS, 3) {
        let pos: u16 = f[0].parse().unwrap();
        assert!(
            pads.insert(pos, f[1].to_owned()).is_none(),
            "pad {pos} twice"
        );
    }
    pads
}

/// The table's pins: name to position, and its AFs in the families.
fn st() -> (BTreeMap<String, u16>, Afs) {
    let table = pin_table("STM32H753ZITx").unwrap();
    let mut positions = BTreeMap::new();
    let mut afs = Afs::new();
    for (name, pin) in &table.pins {
        positions.insert(name.clone(), pin.position);
        for (signal, af) in &pin.alternate_functions {
            afs.insert((name.clone(), signal.clone()), *af);
        }
    }
    (positions, afs)
}

fn families(afs: &Afs) -> Afs {
    afs.iter()
        .filter(|((_, s), _)| in_families(s))
        .map(|(k, v)| (k.clone(), *v))
        .collect()
}

/// DS12117's AF rows; each pin must be one of the package's.
fn ds_afs(on_package: &BTreeSet<String>) -> Afs {
    let mut afs = Afs::new();
    for f in rows(DS_AFS, 4) {
        let (pin, af, signal) = (f[0], f[1].parse::<u8>().unwrap(), f[2]);
        assert!(af <= 15 && in_families(signal), "row {f:?}");
        assert!(on_package.contains(pin), "{pin} is not on the package");
        if let Some(old) = afs.insert((pin.to_owned(), signal.to_owned()), af) {
            panic!("{pin} {signal} twice (AF{old}, AF{af})");
        }
    }
    afs
}

/// Every key of either side whose AF differs or is missing on the other,
/// named with both values.
fn diff(a_name: &str, a: &Afs, b_name: &str, b: &Afs) -> Vec<String> {
    let keys: BTreeSet<_> = a.keys().chain(b.keys()).collect();
    let show = |v: Option<&u8>| v.map_or("none".to_owned(), |af| format!("AF{af}"));
    keys.into_iter()
        .filter(|k| a.get(*k) != b.get(*k))
        .map(|(p, s)| {
            format!(
                "{p} {s}: {a_name} {}, {b_name} {}",
                show(a.get(&(p.clone(), s.clone()))),
                show(b.get(&(p.clone(), s.clone())))
            )
        })
        .collect()
}

/// The pads where DS12117 prints `Pxy_C` and the table holds `Pxy`, each
/// with its reason: (pad, name as printed, the table's pin).
///
/// DS12117 Table 8 note 4 marks LQFP144 pads 28 and 29: "There is a direct
/// path between Pxy_C and Pxy pins/balls, through an analog switch. Pxy
/// alternate functions are available on Pxy_C when the analog switch is
/// closed." RM0433 Rev 9 §12.3.2 (p. 549, `SYSCFG_PMCR`): "PXnSO reset value
/// is 0 when the corresponding PXn_C pin is available on the package but PXn
/// is not", so on this package the switch is closed from reset. ST's XML also
/// names these pads `PC2_C` and `PC3_C`, and gives them PC2's and PC3's
/// signals. `pin_table` reads them as `PC2` and `PC3`.
const SWITCHED: [(u16, &str, &str); 2] = [(28, "PC2_C", "PC2"), (29, "PC3_C", "PC3")];

/// The pin a pad carries: a GPIO's own name, or a `SWITCHED` pad's pin.
fn pad_pin(pos: u16, name: &str) -> Option<&str> {
    gpio(name).or_else(|| {
        SWITCHED
            .iter()
            .find(|(p, n, _)| *p == pos && *n == name)
            .map(|(_, _, pin)| *pin)
    })
}

/// The pad map, both ways: each pin of the table sits at the pad DS12117
/// prints for it, and each GPIO pad DS12117 prints is a pin of the table.
fn pad_diff(st_positions: &BTreeMap<String, u16>, pads: &BTreeMap<u16, String>) -> Vec<String> {
    let mut out = Vec::new();
    for (pin, pos) in st_positions {
        match pads.get(pos) {
            Some(name) if pad_pin(*pos, name) == Some(pin.as_str()) => {}
            other => out.push(format!(
                "{pin}: st pad {pos}, ds12117 pad {pos} is {other:?}"
            )),
        }
    }
    for (pos, name) in pads {
        if let Some(pin) = pad_pin(*pos, name)
            && st_positions.get(pin) != Some(pos)
        {
            out.push(format!(
                "pad {pos} {name:?}: ds12117 {pin}, st {:?}",
                st_positions.get(pin)
            ));
        }
    }
    out
}

/// ArduPilot's `AltFunction_map`, read as text and never run: one
/// `"PIN:SIGNAL" : AF,` per line between `AltFunction_map = {` and the first
/// `}` line, with comments and blank lines skipped. Any other line in the
/// block, or a key given twice, is refused.
fn ardupilot(text: &str) -> Afs {
    let mut lines = text
        .lines()
        .skip_while(|l| !l.starts_with("AltFunction_map = {"));
    assert!(lines.next().is_some(), "no AltFunction_map");
    let mut afs = Afs::new();
    for line in lines {
        let t = line.trim();
        if t == "}" {
            return afs;
        }
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let parsed = t
            .strip_prefix('"')
            .and_then(|r| r.split_once('"'))
            .and_then(|(key, rest)| {
                let af = rest
                    .trim()
                    .strip_prefix(':')?
                    .trim()
                    .strip_suffix(',')?
                    .trim();
                let (pin, signal) = key.split_once(':')?;
                Some((pin, signal, af.parse::<u8>().ok()?))
            });
        let Some((pin, signal, af)) = parsed else {
            panic!("AltFunction_map line not read: {line:?}");
        };
        assert!(
            afs.insert((pin.to_owned(), signal.to_owned()), af)
                .is_none(),
            "{pin}:{signal} twice"
        );
    }
    panic!("AltFunction_map is not closed");
}

/// Known ArduPilot rows in the families that ST's data does not carry, each
/// with its reason. Empty: none was found (`spike/4408`).
const ARDUPILOT_ONLY: [&str; 0] = [];

// ------------------------------------------------------------ the sources

#[test]
fn st_pads_equal_ds12117_lqfp144() {
    let (positions, _) = st();
    let pads = ds_pads();
    assert_eq!(pads.len(), 144);
    assert_eq!(
        pads.keys().copied().collect::<Vec<_>>(),
        (1..=144).collect::<Vec<_>>()
    );
    // Not vacuous: the table's pins are the package's GPIO pads.
    assert!(positions.len() >= 100, "{}", positions.len());
    assert_eq!(pad_diff(&positions, &pads), Vec::<String>::new());
}

#[test]
fn st_afs_equal_ds12117_in_the_solver_families() {
    let (positions, afs) = st();
    let on_package: BTreeSet<String> = positions.keys().cloned().collect();
    let ds = ds_afs(&on_package);
    // Not vacuous: 167 rows on the package (measured, `spike/4408`).
    assert!(ds.len() >= 150, "{}", ds.len());
    eprintln!("ANSWER ds12117 af_rows={}", ds.len());
    assert_eq!(
        diff("st", &families(&afs), "ds12117", &ds),
        Vec::<String>::new()
    );
}

#[test]
#[ignore = "reads ArduPilot's STM32H753xx.py, which is not committed; ardupilot-image.yml runs it (AET-WO-4408)"]
fn st_afs_equal_ardupilot() {
    let path = std::env::var(ARDUPILOT_ENV)
        .unwrap_or_else(|_| panic!("{ARDUPILOT_ENV} names no file: set it to STM32H753xx.py"));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        sha, ARDUPILOT_SHA256,
        "{path} is not the pinned STM32H753xx.py"
    );
    let ap = ardupilot(std::str::from_utf8(&bytes).unwrap());
    let (positions, afs) = st();
    let ap: Afs = ap
        .into_iter()
        .filter(|((p, _), _)| positions.contains_key(p))
        .collect();
    // Every (pin, signal) the two tables share carries one AF, whatever its family.
    let shared: Afs = afs
        .iter()
        .filter(|(k, _)| ap.contains_key(*k))
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    assert!(shared.len() >= 500, "{}", shared.len());
    let ap_shared: Afs = ap
        .iter()
        .filter(|(k, _)| shared.contains_key(*k))
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    assert_eq!(
        diff("st", &shared, "ardupilot", &ap_shared),
        Vec::<String>::new()
    );
    eprintln!(
        "ANSWER ardupilot shared={} families={}",
        shared.len(),
        families(&ap).len()
    );
    // In the solver's families, the same rows both ways.
    let mut ap_families = families(&ap);
    for known in ARDUPILOT_ONLY {
        let (p, s) = known.split_once(':').unwrap();
        assert!(
            ap_families.remove(&(p.to_owned(), s.to_owned())).is_some(),
            "{known}"
        );
    }
    assert_eq!(
        diff("st", &families(&afs), "ardupilot", &ap_families),
        Vec::<String>::new()
    );
}

// ------------------------------------------------------------ planted

/// A row ST and ArduPilot agree on (`PB6 I2C1_SCL` AF4, `check pinout`'s
/// own Sentinel-01 pin) changed in DS12117 alone fails the DS12117 leg,
/// naming the row and both values (WO §5).
#[test]
fn a_row_only_ds12117_contradicts_fails() {
    let (positions, afs) = st();
    let on_package: BTreeSet<String> = positions.keys().cloned().collect();
    let key = ("PB6".to_owned(), "I2C1_SCL".to_owned());
    assert_eq!(afs.get(&key), Some(&4));
    let ap = ardupilot("AltFunction_map = {\n\t\"PB6:I2C1_SCL\"\t:\t4,\n}\n");
    assert_eq!(ap.get(&key), Some(&4));
    let mut ds = ds_afs(&on_package);
    assert_eq!(ds.insert(key, 6), Some(4));
    assert_eq!(
        diff("st", &families(&afs), "ds12117", &ds),
        ["PB6 I2C1_SCL: st AF4, ds12117 AF6"]
    );
}

/// Each kind of difference is named: an AF that differs, a row on one side
/// only, a pad that moved, a GPIO pad the table lacks.
#[test]
fn each_planted_difference_is_named() {
    let ap = ardupilot(
        "x = 1\nAltFunction_map = {\n\t# comment\n\t\"PA9:USART1_TX\"\t:\t7,\n    \"PA9:CAN1_RX\"\t\t:\t9,\n\t\"PA10:USART1_RX\" : 6,\n}\nother = {\n}\n",
    );
    let st: Afs = [
        (("PA9".into(), "USART1_TX".into()), 7),
        (("PA10".into(), "USART1_RX".into()), 7),
        (("PB6".into(), "I2C1_SCL".into()), 4),
    ]
    .into();
    assert_eq!(
        diff("st", &st, "ardupilot", &ap),
        [
            "PA10 USART1_RX: st AF7, ardupilot AF6",
            "PA9 CAN1_RX: st none, ardupilot AF9",
            "PB6 I2C1_SCL: st AF4, ardupilot none",
        ]
    );
    assert!(families(&ap).keys().all(|(_, s)| s != "CAN1_RX"));

    let positions: BTreeMap<String, u16> = [("PA9".into(), 101), ("PA10".into(), 102)].into();
    let pads: BTreeMap<u16, String> = [
        (101, "PA10".to_owned()),
        (102, "PA9".to_owned()),
        (103, "PA11".to_owned()),
        (104, "PA0_C".to_owned()),
        (105, "PC14-OSC32_IN (OSC32_IN)".to_owned()),
    ]
    .into();
    assert_eq!(
        pad_diff(&positions, &pads),
        [
            "PA10: st pad 102, ds12117 pad 102 is Some(\"PA9\")",
            "PA9: st pad 101, ds12117 pad 101 is Some(\"PA10\")",
            "pad 101 \"PA10\": ds12117 PA10, st Some(102)",
            "pad 102 \"PA9\": ds12117 PA9, st Some(101)",
            "pad 103 \"PA11\": ds12117 PA11, st None",
            "pad 105 \"PC14-OSC32_IN (OSC32_IN)\": ds12117 PC14, st None",
        ]
    );
}

#[test]
fn the_family_and_pad_readers() {
    for s in [
        "I2C4_SDA",
        "SPI6_MISO",
        "USART10_RX",
        "UART8_TX",
        "TIM15_CH2",
    ] {
        assert!(in_families(s), "{s}");
    }
    for s in [
        "I2C1_SMBA",
        "SPI1_NSS",
        "LPUART1_TX",
        "TIM1_CH1N",
        "TIM1_CH5",
        "USART_TX",
        "CAN1_RX",
    ] {
        assert!(!in_families(s), "{s}");
    }
    assert_eq!(gpio("PE2"), Some("PE2"));
    assert_eq!(gpio("PC15-OSC32_OUT (OSC32_OUT)"), Some("PC15"));
    assert_eq!(gpio("PA0_C"), None);
    // The switched pads map only at their own pad, by their own name.
    assert_eq!(pad_pin(28, "PC2_C"), Some("PC2"));
    assert_eq!(pad_pin(27, "PC2_C"), None);
    assert_eq!(pad_pin(28, "PC3_C"), None);
    assert_eq!(pad_pin(34, "PA0_C"), None);
    assert_eq!(gpio("VDD"), None);
    assert_eq!(gpio("PDR_ON"), None);
    assert_eq!(gpio("PA123"), None);
}

#[test]
#[should_panic(expected = "AltFunction_map line not read")]
fn an_unread_ardupilot_line_is_refused() {
    ardupilot("AltFunction_map = {\n\t\"PA9:USART1_TX\" : 7\n}\n");
}

#[test]
#[should_panic(expected = "twice")]
fn a_twice_given_ardupilot_key_is_refused() {
    ardupilot("AltFunction_map = {\n\"PA9:X\":1,\n\"PA9:X\":1,\n}\n");
}
