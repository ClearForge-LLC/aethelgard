//! `aethelgard-bridge --rail <request.json>` (`AET-WO-2301`): one declared
//! supply droop, solved once. The request is a strict JSON object of integers;
//! the bridge writes the deck itself from them, so no text of the request
//! reaches ngspice (`INV-06`; the domain name is checked and not used).
//!
//! ```json
//! {"domain":"VDD","nominal_microvolts":3300000,"floor_microvolts":2400000,
//!  "start_ns":1000000,"fall_ns":20000,"hold_ns":200000,"rise_ns":20000,
//!  "source_milliohms":500,"load_microamps":100000,"decoupling_picofarads":400000,
//!  "span_ns":100000000,"levels":[{"name":"VBOR3_falling","microvolts":2610000}]}
//! ```

use serde_json::{Map, Value};

use crate::decimal;
use crate::request::{integer, object, string};

const KEYS: [&str; 12] = [
    "domain",
    "nominal_microvolts",
    "floor_microvolts",
    "start_ns",
    "fall_ns",
    "hold_ns",
    "rise_ns",
    "source_milliohms",
    "load_microamps",
    "decoupling_picofarads",
    "span_ns",
    "levels",
];
const MAX_LEVELS: usize = 16;

/// A request that passed every bound.
#[derive(Debug, Clone, PartialEq)]
pub struct Rail {
    pub nominal_uv: u64,
    pub floor_uv: u64,
    /// The four vertices of the droop, in ns: start, end of the fall, end of
    /// the hold, end of the rise.
    pub vertices_ns: [u64; 4],
    pub source_mohm: u64,
    pub load_ua: u64,
    pub decoupling_pf: u64,
    pub span_ns: u64,
    pub levels: Vec<Level>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    pub name: String,
    pub microvolts: u64,
}

pub fn parse(bytes: &[u8]) -> Result<Rail, String> {
    let v: Value = serde_json::from_slice(bytes).map_err(|e| format!("not JSON: {e}"))?;
    let o = object(&v, "the request", &KEYS)?;
    printable(&string(o, "domain")?, "`domain`")?;
    let nominal_uv = bounded(o, "nominal_microvolts", 1, 5_000_000)?;
    let floor_uv = integer(o, "floor_microvolts")?;
    if floor_uv >= nominal_uv {
        return Err(format!(
            "`floor_microvolts` {floor_uv} is not below `nominal_microvolts` {nominal_uv}"
        ));
    }
    let start = bounded(o, "start_ns", 1, u64::MAX)?;
    let fall = bounded(o, "fall_ns", 1, 10_000_000)?;
    let hold = bounded(o, "hold_ns", 1, 100_000_000)?;
    let rise = bounded(o, "rise_ns", 1, 10_000_000)?;
    let span_ns = bounded(o, "span_ns", 1, 100_000_000)?;
    let t2 = start.checked_add(fall);
    let t3 = t2.and_then(|t| t.checked_add(hold));
    let t4 = t3.and_then(|t| t.checked_add(rise));
    let (Some(t2), Some(t3), Some(t4)) = (t2, t3, t4) else {
        return Err("the droop's times overflow".to_owned());
    };
    if t4 > span_ns {
        return Err(format!(
            "the droop ends at {t4} ns, after `span_ns` {span_ns}"
        ));
    }
    let source_mohm = bounded(o, "source_milliohms", 1, 1_000_000)?;
    let load_ua = bounded(o, "load_microamps", 0, 10_000_000)?;
    let decoupling_pf = bounded(o, "decoupling_picofarads", 1, 1_000_000_000)?;
    let list = o["levels"].as_array().ok_or("`levels` is not an array")?;
    if list.is_empty() || list.len() > MAX_LEVELS {
        return Err(format!(
            "`levels` holds {}, not 1 to {MAX_LEVELS}",
            list.len()
        ));
    }
    let mut levels: Vec<Level> = Vec::with_capacity(list.len());
    for (i, l) in list.iter().enumerate() {
        let lo = object(l, &format!("levels[{i}]"), &["name", "microvolts"])?;
        let name = string(lo, "name").map_err(|e| format!("levels[{i}]: {e}"))?;
        printable(&name, &format!("levels[{i}].name"))?;
        if levels.iter().any(|x| x.name == name) {
            return Err(format!("levels[{i}]: the name `{name}` appears twice"));
        }
        let microvolts = integer(lo, "microvolts").map_err(|e| format!("levels[{i}]: {e}"))?;
        if microvolts == 0 || microvolts >= nominal_uv {
            return Err(format!(
                "levels[{i}].microvolts {microvolts} is not 1 to below `nominal_microvolts`"
            ));
        }
        levels.push(Level { name, microvolts });
    }
    Ok(Rail {
        nominal_uv,
        floor_uv,
        vertices_ns: [start, t2, t3, t4],
        source_mohm,
        load_ua,
        decoupling_pf,
        span_ns,
        levels,
    })
}

fn bounded(o: &Map<String, Value>, key: &str, min: u64, max: u64) -> Result<u64, String> {
    let n = integer(o, key)?;
    if n < min || n > max {
        return Err(format!("`{key}` is {n}, not {min} to {max}"));
    }
    Ok(n)
}

fn printable(s: &str, what: &str) -> Result<(), String> {
    if (1..=64).contains(&s.len()) && s.bytes().all(|b| (0x20..=0x7E).contains(&b)) {
        Ok(())
    } else {
        Err(format!("{what} is not 1 to 64 printable ASCII bytes"))
    }
}

impl Rail {
    /// The deck, written from the integers alone.
    pub fn deck(&self) -> Vec<String> {
        let [t1, t2, t3, t4] = self.vertices_ns;
        let (nom, floor) = (decimal(self.nominal_uv, 6), decimal(self.floor_uv, 6));
        vec![
            "* aethelgard rail".to_owned(),
            format!("vsrc src 0 pwl(0 {nom} {t1}n {nom} {t2}n {floor} {t3}n {floor} {t4}n {nom})"),
            format!("rs src rail {}", decimal(self.source_mohm, 3)),
            format!("cdec rail 0 {}", decimal(self.decoupling_pf, 12)),
            format!("iload rail 0 dc {}", decimal(self.load_ua, 6)),
            ".options reltol=1e-3 abstol=1e-12 vntol=1e-6 method=trap".to_owned(),
            format!(".tran 10n {}n 0 1000n", self.span_ns),
            ".end".to_owned(),
        ]
    }
}

/// One crossing of a level, by linear interpolation of the accepted points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crossing {
    /// Index into the request's `levels`.
    pub level: usize,
    pub falling: bool,
    /// Seconds.
    pub time: f64,
}

/// Every crossing of `level` volts: where `v[i-1] >= level` differs from
/// `v[i] >= level`, at the straight line between the two points.
pub fn crossings(time: &[f64], v: &[f64], level: f64, index: usize) -> Vec<Crossing> {
    let mut out = Vec::new();
    for (t, vs) in time.windows(2).zip(v.windows(2)) {
        let (below0, below1) = (vs[0] < level, vs[1] < level);
        if below0 != below1 {
            let at = t[0] + (level - vs[0]) * (t[1] - t[0]) / (vs[1] - vs[0]);
            out.push(Crossing {
                level: index,
                falling: below1,
                time: at,
            });
        }
    }
    out
}

/// Seconds to the nearest picosecond.
pub fn picoseconds(seconds: f64) -> u64 {
    (seconds * 1e12).round() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{"domain":"VDD","nominal_microvolts":3300000,"floor_microvolts":2400000,
        "start_ns":1000000,"fall_ns":20000,"hold_ns":200000,"rise_ns":20000,
        "source_milliohms":500,"load_microamps":100000,"decoupling_picofarads":400000,
        "span_ns":100000000,"levels":[{"name":"VBOR3_falling","microvolts":2610000},
        {"name":"VBOR3_rising","microvolts":2700000}]}"#;

    fn edit(f: impl FnOnce(&mut Value)) -> Vec<u8> {
        let mut v: Value = serde_json::from_str(GOOD).unwrap();
        f(&mut v);
        serde_json::to_vec(&v).unwrap()
    }

    #[test]
    fn the_reference_request_parses_and_its_deck_is_exact() {
        let rail = parse(GOOD.as_bytes()).unwrap();
        assert_eq!(
            rail.vertices_ns,
            [1_000_000, 1_020_000, 1_220_000, 1_240_000]
        );
        assert_eq!(rail.levels.len(), 2);
        assert_eq!(
            rail.deck().join("\n"),
            "* aethelgard rail\n\
             vsrc src 0 pwl(0 3.3 1000000n 3.3 1020000n 2.4 1220000n 2.4 1240000n 3.3)\n\
             rs src rail 0.5\n\
             cdec rail 0 0.0000004\n\
             iload rail 0 dc 0.1\n\
             .options reltol=1e-3 abstol=1e-12 vntol=1e-6 method=trap\n\
             .tran 10n 100000000n 0 1000n\n\
             .end"
        );
    }

    #[test]
    fn zero_load_and_whole_units_are_plain_decimals() {
        let rail = parse(&edit(|v| {
            v["load_microamps"] = 0.into();
            v["source_milliohms"] = 2000.into();
        }))
        .unwrap();
        let deck = rail.deck();
        assert_eq!(deck[2], "rs src rail 2");
        assert_eq!(deck[4], "iload rail 0 dc 0");
    }

    #[test]
    fn each_bound_and_shape_is_refused() {
        type Edit = Box<dyn FnOnce(&mut Value)>;
        let cases: Vec<(&str, Edit)> = vec![
            ("unknown key", Box::new(|v| v["extra"] = 1.into())),
            (
                "missing key",
                Box::new(|v| drop(v.as_object_mut().unwrap().remove("hold_ns"))),
            ),
            ("float", Box::new(|v| v["fall_ns"] = 20000.5.into())),
            (
                "float spelled integer",
                Box::new(|v| v["fall_ns"] = serde_json::json!(20000.0)),
            ),
            ("negative", Box::new(|v| v["fall_ns"] = (-1).into())),
            ("domain empty", Box::new(|v| v["domain"] = "".into())),
            (
                "domain long",
                Box::new(|v| v["domain"] = "x".repeat(65).into()),
            ),
            ("domain control", Box::new(|v| v["domain"] = "a\nb".into())),
            ("domain not string", Box::new(|v| v["domain"] = 3.into())),
            (
                "nominal 0",
                Box::new(|v| v["nominal_microvolts"] = 0.into()),
            ),
            (
                "nominal high",
                Box::new(|v| v["nominal_microvolts"] = 5_000_001.into()),
            ),
            (
                "floor = nominal",
                Box::new(|v| v["floor_microvolts"] = 3_300_000.into()),
            ),
            ("start 0", Box::new(|v| v["start_ns"] = 0.into())),
            ("fall 0", Box::new(|v| v["fall_ns"] = 0.into())),
            ("fall high", Box::new(|v| v["fall_ns"] = 10_000_001.into())),
            ("rise 0", Box::new(|v| v["rise_ns"] = 0.into())),
            ("rise high", Box::new(|v| v["rise_ns"] = 10_000_001.into())),
            ("hold 0", Box::new(|v| v["hold_ns"] = 0.into())),
            ("hold high", Box::new(|v| v["hold_ns"] = 100_000_001.into())),
            ("span 0", Box::new(|v| v["span_ns"] = 0.into())),
            ("span high", Box::new(|v| v["span_ns"] = 100_000_001.into())),
            (
                "droop past span",
                Box::new(|v| v["span_ns"] = 1_239_999.into()),
            ),
            (
                "sum overflows",
                Box::new(|v| v["start_ns"] = u64::MAX.into()),
            ),
            ("source 0", Box::new(|v| v["source_milliohms"] = 0.into())),
            (
                "source high",
                Box::new(|v| v["source_milliohms"] = 1_000_001.into()),
            ),
            (
                "load high",
                Box::new(|v| v["load_microamps"] = 10_000_001.into()),
            ),
            ("cap 0", Box::new(|v| v["decoupling_picofarads"] = 0.into())),
            (
                "cap high",
                Box::new(|v| v["decoupling_picofarads"] = 1_000_000_001u64.into()),
            ),
            (
                "no levels",
                Box::new(|v| v["levels"] = serde_json::json!([])),
            ),
            (
                "17 levels",
                Box::new(|v| {
                    v["levels"] = (1..=17)
                        .map(|i| serde_json::json!({"name": format!("L{i}"), "microvolts": 1000 + i}))
                        .collect()
                }),
            ),
            (
                "level 0",
                Box::new(|v| v["levels"][0]["microvolts"] = 0.into()),
            ),
            (
                "level = nominal",
                Box::new(|v| v["levels"][0]["microvolts"] = 3_300_000.into()),
            ),
            (
                "level name dup",
                Box::new(|v| v["levels"][1]["name"] = "VBOR3_falling".into()),
            ),
            (
                "level name empty",
                Box::new(|v| v["levels"][0]["name"] = "".into()),
            ),
            (
                "level extra key",
                Box::new(|v| v["levels"][0]["x"] = 1.into()),
            ),
            (
                "level missing key",
                Box::new(|v| drop(v["levels"][0].as_object_mut().unwrap().remove("name"))),
            ),
            (
                "level float",
                Box::new(|v| v["levels"][0]["microvolts"] = 2610000.5.into()),
            ),
            ("not an object", Box::new(|v| *v = serde_json::json!([]))),
        ];
        for (what, f) in cases {
            assert!(parse(&edit(f)).is_err(), "{what} was accepted");
        }
        assert!(parse(b"{").is_err());
    }

    #[test]
    fn the_largest_accepted_request_still_parses() {
        let rail = parse(&edit(|v| {
            v["nominal_microvolts"] = 5_000_000.into();
            v["start_ns"] = 1.into();
            v["fall_ns"] = 10_000_000.into();
            v["hold_ns"] = 79_999_998.into();
            v["rise_ns"] = 10_000_000.into();
            v["span_ns"] = 100_000_000.into();
            v["source_milliohms"] = 1_000_000.into();
            v["load_microamps"] = 10_000_000.into();
            v["decoupling_picofarads"] = 1_000_000_000u64.into();
        }))
        .unwrap();
        assert_eq!(rail.vertices_ns[3], 99_999_999);
        assert_eq!(rail.deck()[3], "cdec rail 0 0.001");
    }

    #[test]
    fn crossings_are_interpolated_in_both_directions() {
        // Falls through 2.0 between t=1 and 2, rises through it between 4 and 5,
        // falls again between 6 and 7.
        let time = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let v = [3.0, 3.0, 1.0, 1.0, 1.0, 3.0, 3.0, 1.0];
        let c = crossings(&time, &v, 2.0, 7);
        let got: Vec<(bool, f64)> = c.iter().map(|c| (c.falling, c.time)).collect();
        assert_eq!(got, [(true, 1.5), (false, 4.5), (true, 6.5)]);
        assert!(c.iter().all(|c| c.level == 7));
    }

    #[test]
    fn a_point_exactly_on_the_level_is_one_crossing() {
        // `v >= level` is "high": the point on the level is high, so the fall
        // is the step after it, interpolated to that point's own time, and the
        // rise is the step back up to it.
        let time = [0.0, 1.0, 2.0, 3.0];
        let v = [3.0, 2.0, 1.0, 2.0];
        let c = crossings(&time, &v, 2.0, 0);
        let got: Vec<(bool, f64)> = c.iter().map(|c| (c.falling, c.time)).collect();
        assert_eq!(got, [(true, 1.0), (false, 3.0)]);
        // A level never reached has none.
        assert!(crossings(&time, &v, 5.0, 0).is_empty());
        assert!(crossings(&time, &v, 0.5, 0).is_empty());
    }

    #[test]
    fn picoseconds_round_to_nearest() {
        assert_eq!(picoseconds(1.014_422_222e-3), 1_014_422_222);
        assert_eq!(picoseconds(0.4e-12), 0);
        assert_eq!(picoseconds(0.6e-12), 1);
        assert_eq!(picoseconds(2.5e-12), 3);
    }
}
