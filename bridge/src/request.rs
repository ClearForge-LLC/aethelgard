//! The request file the CLI writes (`AET-WO-2200` §1.3): one entry per I2C
//! line, each a netlist from `aethelgard-twin::analog` and what to measure.
//! Everything is integers, so a value reaches ngspice exactly as the twin
//! wrote it. Anything malformed, unknown or out of range is refused before
//! ngspice sees it (`INV-06`); nothing is defaulted.
//!
//! ```json
//! {"lines":[{"net":"I2C_SCL","netlist":["* …",".end"],"source":"vctl",
//!   "stimulus":[[0,3300],[2500000,3300],[2501000,0]],"probe":"line",
//!   "release_ps":2500000,"low_uv":990000,"high_uv":2310000,"stop_ps":22500000}]}
//! ```

use serde_json::{Map, Value};

/// The CLI's own input cap (`RUL-13`).
pub const MAX_BYTES: u64 = 16 * 1024 * 1024;
const MAX_LINES: usize = 64;
const MAX_NETLIST_LINES: usize = 64;
const MAX_CARD_BYTES: usize = 256;
const MAX_POINTS: usize = 64;
/// 1 ms of simulated time: fifty times the 22.5 µs window.
const MAX_STOP_PS: u64 = 1_000_000_000;
/// 100 V, beyond any rail an I2C bus has.
const MAX_MILLIVOLTS: u64 = 100_000;

/// A piecewise-linear waveform: strictly increasing times in seconds, from 0.
#[derive(Debug, Clone, PartialEq)]
pub struct Pwl {
    pub points: Vec<(f64, f64)>,
}

impl Pwl {
    /// The value at `t`, linear between points and held outside them.
    pub fn at(&self, t: f64) -> f64 {
        let p = &self.points;
        let Some(&(t0, v0)) = p.first() else {
            return 0.0;
        };
        if t <= t0 {
            return v0;
        }
        for w in p.windows(2) {
            let ((ta, va), (tb, vb)) = (w[0], w[1]);
            if t < tb {
                return va + (vb - va) * (t - ta) / (tb - ta);
            }
        }
        p[p.len() - 1].1
    }

    /// Every point after the first: where ngspice must place a breakpoint.
    pub fn vertices(&self) -> impl Iterator<Item = f64> + '_ {
        self.points.iter().skip(1).map(|&(t, _)| t)
    }
}

/// One line to measure.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub net: String,
    pub netlist: Vec<String>,
    pub source: String,
    pub stimulus: Pwl,
    pub probe: String,
    /// Where the driver starts to release the line, in seconds.
    pub release: f64,
    pub release_ps: u64,
    /// The 30 % and 70 % thresholds, in volts and in microvolts.
    pub low: f64,
    pub high: f64,
    pub low_uv: u64,
    pub high_uv: u64,
    /// The transient's end, in seconds.
    pub stop: f64,
}

pub fn parse(bytes: &[u8]) -> Result<Vec<Line>, String> {
    let v: Value = serde_json::from_slice(bytes).map_err(|e| format!("not JSON: {e}"))?;
    let top = object(&v, "the request", &["lines"])?;
    let lines = top["lines"].as_array().ok_or("`lines` is not an array")?;
    if lines.is_empty() || lines.len() > MAX_LINES {
        return Err(format!(
            "`lines` holds {}, not 1 to {MAX_LINES}",
            lines.len()
        ));
    }
    let mut out: Vec<Line> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let line = parse_line(l).map_err(|e| format!("lines[{i}]: {e}"))?;
        if out.iter().any(|o| o.net == line.net) {
            return Err(format!("lines[{i}]: net `{}` appears twice", line.net));
        }
        out.push(line);
    }
    Ok(out)
}

const KEYS: [&str; 9] = [
    "net",
    "netlist",
    "source",
    "stimulus",
    "probe",
    "release_ps",
    "low_uv",
    "high_uv",
    "stop_ps",
];

fn parse_line(v: &Value) -> Result<Line, String> {
    let o = object(v, "a line", &KEYS)?;
    let net = string(o, "net")?;
    if !(1..=64).contains(&net.len())
        || !net
            .bytes()
            .all(|b| (0x21..=0x7E).contains(&b) && !b"/{}$[]".contains(&b))
    {
        return Err("`net` is not an identifier (RUL-14)".to_owned());
    }
    let source = name(o, "source")?;
    if !source.starts_with('v') {
        return Err("`source` does not name a voltage source".to_owned());
    }
    let probe = name(o, "probe")?;
    let netlist = netlist(o, &source)?;
    let release_ps = integer(o, "release_ps")?;
    let stop_ps = integer(o, "stop_ps")?;
    if stop_ps == 0 || stop_ps > MAX_STOP_PS {
        return Err(format!("`stop_ps` is {stop_ps}, not 1 to {MAX_STOP_PS}"));
    }
    if release_ps == 0 || release_ps >= stop_ps {
        return Err("`release_ps` is not inside the transient".to_owned());
    }
    let low_uv = integer(o, "low_uv")?;
    let high_uv = integer(o, "high_uv")?;
    if low_uv == 0 || low_uv >= high_uv || high_uv > MAX_MILLIVOLTS * 1000 {
        return Err("`low_uv` and `high_uv` are not 0 < low < high <= 100 V".to_owned());
    }
    let stimulus = stimulus(o, stop_ps)?;
    Ok(Line {
        net,
        netlist,
        source,
        stimulus,
        probe,
        release: seconds(release_ps),
        release_ps,
        low: volts_uv(low_uv),
        high: volts_uv(high_uv),
        low_uv,
        high_uv,
        stop: seconds(stop_ps),
    })
}

/// The deck, card by card. Only the cards the twin writes are accepted:
/// a title, R, C, S and V elements, and `.model`, `.options`, `.tran` and a
/// final `.end`. A `.control` block, an `.include` or any other card could
/// make ngspice run a command or read a file, and so can a comment: ngspice
/// runs a card starting `*#` as a command (adversarial A1), so no card but
/// the title may be a comment.
fn netlist(o: &Map<String, Value>, source: &str) -> Result<Vec<String>, String> {
    let cards = o["netlist"].as_array().ok_or("`netlist` is not an array")?;
    if cards.len() < 3 || cards.len() > MAX_NETLIST_LINES {
        return Err(format!(
            "`netlist` holds {} cards, not 3 to {MAX_NETLIST_LINES}",
            cards.len()
        ));
    }
    let mut out = Vec::with_capacity(cards.len());
    let mut external = 0;
    for (i, c) in cards.iter().enumerate() {
        let c = c
            .as_str()
            .ok_or_else(|| format!("netlist[{i}] is not a string"))?;
        if c.is_empty()
            || c.len() > MAX_CARD_BYTES
            || !c.bytes().all(|b| (0x20..=0x7E).contains(&b))
        {
            return Err(format!(
                "netlist[{i}] is not 1 to {MAX_CARD_BYTES} bytes of printable ASCII"
            ));
        }
        let lower = c.to_ascii_lowercase();
        let first = lower.split_whitespace().next().unwrap_or("");
        let last = i == cards.len() - 1;
        let ok = if i == 0 {
            // ngspice reads the first card as the title, but a title of
            // `*ng_script` makes the deck a script, so it is `* ` and text.
            lower.starts_with("* ")
        } else if last {
            lower == ".end"
        } else if first.starts_with('.') {
            matches!(first, ".model" | ".options" | ".tran")
        } else {
            matches!(first.as_bytes().first(), Some(b'r' | b'c' | b's' | b'v'))
        };
        if !ok {
            return Err(format!(
                "netlist[{i}] is a card the bridge does not accept: `{c}`"
            ));
        }
        if lower.split_whitespace().any(|w| w == "external") {
            if first != source {
                return Err(format!(
                    "netlist[{i}] declares an external source other than `{source}`"
                ));
            }
            external += 1;
        }
        out.push(c.to_owned());
    }
    if external != 1 {
        return Err(format!(
            "the netlist declares {external} external sources, not exactly `{source}`"
        ));
    }
    Ok(out)
}

fn stimulus(o: &Map<String, Value>, stop_ps: u64) -> Result<Pwl, String> {
    let points = o["stimulus"]
        .as_array()
        .ok_or("`stimulus` is not an array")?;
    if points.len() < 2 || points.len() > MAX_POINTS {
        return Err(format!(
            "`stimulus` holds {} points, not 2 to {MAX_POINTS}",
            points.len()
        ));
    }
    let mut out = Vec::with_capacity(points.len());
    let mut last: Option<u64> = None;
    for (i, p) in points.iter().enumerate() {
        let pair = p
            .as_array()
            .filter(|a| a.len() == 2)
            .ok_or_else(|| format!("stimulus[{i}] is not [ps, mV]"))?;
        let (Some(t), Some(mv)) = (pair[0].as_u64(), pair[1].as_u64()) else {
            return Err(format!("stimulus[{i}] is not two non-negative integers"));
        };
        if i == 0 && t != 0 {
            return Err("the stimulus does not start at 0".to_owned());
        }
        if last.is_some_and(|l| t <= l) || t > stop_ps {
            return Err(format!(
                "stimulus[{i}] is not after the point before it and inside the transient"
            ));
        }
        if mv > MAX_MILLIVOLTS {
            return Err(format!("stimulus[{i}] is above 100 V"));
        }
        last = Some(t);
        out.push((seconds(t), volts_mv(mv)));
    }
    Ok(Pwl { points: out })
}

fn object<'a>(v: &'a Value, what: &str, keys: &[&str]) -> Result<&'a Map<String, Value>, String> {
    let o = v
        .as_object()
        .ok_or_else(|| format!("{what} is not an object"))?;
    if let Some(k) = o.keys().find(|k| !keys.contains(&k.as_str())) {
        return Err(format!("{what} has the unknown key `{k}`"));
    }
    if let Some(k) = keys.iter().find(|k| !o.contains_key(**k)) {
        return Err(format!("{what} lacks `{k}`"));
    }
    Ok(o)
}

fn string(o: &Map<String, Value>, key: &str) -> Result<String, String> {
    o[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("`{key}` is not a string"))
}

/// A node or source name: 1 to 16 of `[a-z0-9_]`, starting with a letter.
fn name(o: &Map<String, Value>, key: &str) -> Result<String, String> {
    let s = string(o, key)?;
    let ok = (1..=16).contains(&s.len())
        && s.as_bytes()[0].is_ascii_lowercase()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if ok {
        Ok(s)
    } else {
        Err(format!("`{key}` is not 1 to 16 of [a-z0-9_]"))
    }
}

fn integer(o: &Map<String, Value>, key: &str) -> Result<u64, String> {
    o[key]
        .as_u64()
        .ok_or_else(|| format!("`{key}` is not a non-negative integer"))
}

/// Integer picoseconds to seconds, correctly rounded once.
fn seconds(ps: u64) -> f64 {
    ps as f64 / 1e12
}

fn volts_mv(mv: u64) -> f64 {
    mv as f64 / 1e3
}

fn volts_uv(uv: u64) -> f64 {
    uv as f64 / 1e6
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(edit: impl FnOnce(&mut Map<String, Value>)) -> Vec<u8> {
        let mut line: Map<String, Value> = serde_json::from_str(
            r#"{"net":"I2C_SCL","netlist":["* t","Vdd vdd 0 dc 3.3","Vctl ctl 0 dc 0 external",".end"],
                "source":"vctl","stimulus":[[0,3300],[2500000,3300],[2501000,0]],"probe":"line",
                "release_ps":2500000,"low_uv":990000,"high_uv":2310000,"stop_ps":22500000}"#,
        )
        .unwrap();
        edit(&mut line);
        serde_json::to_vec(&serde_json::json!({ "lines": [line] })).unwrap()
    }

    #[test]
    fn the_reference_shape_parses() {
        let lines = parse(&request(|_| {})).unwrap();
        assert_eq!(lines[0].release, 2.5e-6);
        assert_eq!(lines[0].low, 0.99);
        assert_eq!(lines[0].stimulus.at(2.5005e-6), 1.65);
    }

    #[test]
    fn hostile_requests_are_refused() {
        type Edit = Box<dyn FnOnce(&mut Map<String, Value>)>;
        let cases: Vec<Edit> = vec![
            Box::new(|l| {
                l.insert("extra".into(), 1.into());
            }),
            Box::new(|l| {
                l.remove("probe");
            }),
            Box::new(|l| l["netlist"][1] = ".control".into()),
            Box::new(|l| l["netlist"][1] = ".include /etc/passwd".into()),
            Box::new(|l| l["netlist"][1] = "B1 a 0 v=1".into()),
            Box::new(|l| l["netlist"][1] = "Vx x 0 dc 0 external".into()),
            Box::new(|l| l["netlist"][3] = ".endc".into()),
            Box::new(|l| l["netlist"][0] = "Vdd vdd 0 dc 3.3".into()),
            // Adversarial A1: a `*#` card is a command, `*ng_script` a script.
            Box::new(|l| l["netlist"][1] = "*# shell touch x".into()),
            Box::new(|l| l["netlist"][1] = "*# alter R1=2200".into()),
            Box::new(|l| l["netlist"][1] = "* a comment".into()),
            Box::new(|l| l["netlist"][0] = "*ng_script".into()),
            Box::new(|l| l["netlist"][0] = "*#title".into()),
            // Adversarial A4: no external source at all.
            Box::new(|l| l["netlist"][2] = "Vctl ctl 0 dc 0".into()),
            Box::new(|l| l["netlist"][1] = "R1 a b 1\u{7}".into()),
            Box::new(|l| l["stop_ps"] = 0.into()),
            Box::new(|l| l["release_ps"] = 22500000.into()),
            Box::new(|l| l["low_uv"] = 0.into()),
            Box::new(|l| l["high_uv"] = 990000.into()),
            Box::new(|l| l["low_uv"] = (-1.5).into()),
            Box::new(|l| l["stimulus"][1][0] = 0.into()),
            Box::new(|l| l["stimulus"][0][0] = 5.into()),
            Box::new(|l| l["stimulus"][2][1] = 100001.into()),
            Box::new(|l| l["stimulus"][2] = serde_json::json!([2501000, 0, 1])),
            Box::new(|l| l["net"] = "I2C/SCL".into()),
            Box::new(|l| l["probe"] = "v(line)".into()),
        ];
        for (i, edit) in cases.into_iter().enumerate() {
            assert!(parse(&request(edit)).is_err(), "case {i} was accepted");
        }
    }

    #[test]
    fn a_repeated_net_is_refused() {
        let one: Value = serde_json::from_slice(&request(|_| {})).unwrap();
        let line = one["lines"][0].clone();
        let two =
            serde_json::to_vec(&serde_json::json!({ "lines": [line.clone(), line] })).unwrap();
        assert!(parse(&two).unwrap_err().contains("twice"));
    }
}
