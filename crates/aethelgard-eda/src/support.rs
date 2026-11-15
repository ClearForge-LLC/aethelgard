//! The support gate (`AET-WO-4300g` §1.5): the MCU's support network judged
//! from the emitted schematic's netlist, as KiCad exports it, against the
//! MCU's row in `power-parts.json` and the specification, never against the
//! emitter's choices (`RUL-20`, `INV-07`). The board supplies geometry
//! alone: where each pad sits, for the distances a rule bounds.
//!
//! **What it judges, in order:**
//! 1. the MCU, found in the netlist by the specification's part number;
//! 2. every supply-group pin on the MCU's declared supply, every ground-group
//!    pin on `GND`, every `powered` pin on a declared voltage domain;
//! 3. each strap: its pin on the net of the group it names;
//! 4. each capacitor role, one at each owner pin or one for its group: a
//!    distinct capacitor between the owner's net and the ground group's net,
//!    of the role's value class, and where the row bounds a distance, with
//!    its pad on the owner's net within that distance of the owner pin's pad,
//!    copper edge to copper edge. Every pin shares one supply net, so a
//!    capacitor's ownership is its distance; the roles are matched to
//!    capacitors one to one (a capacitor serves one pin);
//! 5. the programming path: a declared connector carrying the nets of the
//!    MCU's SWDIO and SWCLK pins, `GND` and the supply; whether NRST reaches
//!    it too is recorded, not required (DS-009's debug ports carry none);
//! 6. the declared supply source's pin on the supply.
//!
//! Each failure is a named finding. A netlist or board the gate cannot read
//! is refused, never judged in part.

use std::collections::{BTreeMap, BTreeSet};

use aethelgard_ir::Specification;
use aethelgard_ir::power_parts::{self, Kind, Support, SupportCapacitor};

use crate::route::{Board, Pad};
use crate::strict::{self, Node, Tree};

/// KiCad's prefix for a pin the schematic leaves unconnected.
const UNCONNECTED: &str = "unconnected-";

/// One component of the netlist: its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Component {
    pub value: String,
}

/// The netlist as the gate reads it: each component by reference, and each
/// `(reference, pin)`'s net. A pin KiCad names `unconnected-…` has no net.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Netlist {
    pub components: BTreeMap<String, Component>,
    pub nets: BTreeMap<(String, String), String>,
}

impl Netlist {
    fn net(&self, reference: &str, pin: &str) -> Option<&str> {
        self.nets
            .get(&(reference.to_owned(), pin.to_owned()))
            .map(String::as_str)
    }

    /// Each pin of `reference` with its net.
    fn pins_of<'n>(&'n self, reference: &'n str) -> impl Iterator<Item = (&'n str, &'n str)> + 'n {
        self.nets
            .range((reference.to_owned(), String::new())..)
            .take_while(move |((r, _), _)| r == reference)
            .map(|((_, p), n)| (p.as_str(), n.as_str()))
    }
}

/// The one value of `node`'s one child `head`.
fn text<'a>(node: &Node<'a>, head: &str) -> Result<&'a str, String> {
    let child = node
        .one(head)
        .map_err(|r| r.to_string())?
        .ok_or_else(|| format!("a `{}` has no `{head}`", node.head().unwrap_or("")))?;
    match (child.len(), child.arg(1)) {
        (2, Some(s)) => Ok(s),
        _ => Err(format!(
            "a `{}`'s `{head}` is not one value",
            node.head().unwrap_or("")
        )),
    }
}

/// Reads a strict netlist tree: version `"E"`, every component's reference
/// and value, every net's name and nodes. A reference or a pin given twice,
/// or a node naming no component, is refused.
pub fn netlist(tree: &Tree) -> Result<Netlist, String> {
    strict::version(tree).map_err(|r| r.to_string())?;
    let root = tree.root();
    let mut out = Netlist::default();
    let components = root
        .one("components")
        .map_err(|r| r.to_string())?
        .ok_or("the netlist has no `components`")?;
    for comp in components.all("comp") {
        let reference = text(&comp, "ref")?;
        let value = text(&comp, "value")?;
        let previous = out.components.insert(
            reference.to_owned(),
            Component {
                value: value.to_owned(),
            },
        );
        if previous.is_some() {
            return Err(format!("the netlist names component `{reference}` twice"));
        }
    }
    let nets = root
        .one("nets")
        .map_err(|r| r.to_string())?
        .ok_or("the netlist has no `nets`")?;
    let mut names = BTreeSet::new();
    for net in nets.all("net") {
        let name = text(&net, "name")?;
        if !names.insert(name) {
            return Err(format!("the netlist names net `{name}` twice"));
        }
        for node in net.all("node") {
            let reference = text(&node, "ref")?;
            let pin = text(&node, "pin")?;
            if !out.components.contains_key(reference) {
                return Err(format!(
                    "net `{name}` names `{reference}`, which is no component"
                ));
            }
            let key = (reference.to_owned(), pin.to_owned());
            if out.nets.contains_key(&key) {
                return Err(format!("{reference} pin {pin} is on two nets"));
            }
            if !name.starts_with(UNCONNECTED) {
                out.nets.insert(key, name.to_owned());
            }
        }
    }
    Ok(out)
}

/// A capacitor value as a schematic writes it, in pF: `100n`, `4u7`, `2u2`,
/// `470p`, `1u`, optionally followed by `F`, then optionally a space and a
/// part number. Anything else is not a capacitance. The gate's own reading,
/// shared with no emitter (`RUL-20`).
pub fn picofarads(value: &str) -> Option<i64> {
    let token = value.split(' ').next()?;
    let token = token.strip_suffix('F').unwrap_or(token);
    let i = token.find(['p', 'n', 'u'])?;
    let (whole, rest) = token.split_at(i);
    let scale: i64 = match &rest[..1] {
        "p" => 1,
        "n" => 1_000,
        _ => 1_000_000,
    };
    let frac = &rest[1..];
    let digits = |d: &str| !d.is_empty() && d.len() <= 6 && d.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || !(frac.is_empty() || digits(frac)) || frac.len() > 3 {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let fraction: i64 = if frac.is_empty() {
        0
    } else {
        frac.parse().ok()?
    };
    let places = 10i64.pow(u32::try_from(frac.len()).ok()?);
    let total = whole.checked_mul(scale)? + fraction.checked_mul(scale)? / places;
    (fraction * scale % places == 0).then_some(total)
}

/// One failure, by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub kind: &'static str,
    pub message: String,
}

/// One capacitor role at one pin, as judged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleRecord {
    pub role: &'static str,
    /// The owner pin, or the group's name for a role held once per group.
    pub pin: String,
    /// The capacitor matched to it, if any.
    pub part: Option<String>,
    pub picofarads: Option<i64>,
    pub gap_um: Option<i64>,
    pub bound_um: Option<i64>,
    pub rule: String,
    pub verdict: &'static str,
}

/// The programming connector the netlist shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Programming {
    pub connector: String,
    /// Whether NRST also reaches it: recorded, not required.
    pub nrst: bool,
}

/// The gate's record for the verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub mcu: String,
    pub roles: Vec<RoleRecord>,
    pub straps: Vec<String>,
    pub programming: Option<Programming>,
    pub supply_source: Option<String>,
    /// Each rule the gate cannot check from the files, named.
    pub conditions: Vec<String>,
    pub findings: Vec<Finding>,
}

/// The MCU row's support network for `part`, or why there is none.
pub fn row(part: &str) -> Result<&'static Support, String> {
    let table = power_parts::table().map_err(|e| e.to_string())?;
    match table.get(part).map(|r| &r.kind) {
        Some(Kind::Mcu(m)) => Ok(&m.support),
        _ => Err(format!("the power parts table has no MCU row for `{part}`")),
    }
}

fn pad<'b>(board: &'b Board, reference: &str, number: &str) -> Option<&'b Pad> {
    let part = board.parts.iter().find(|p| p.reference == reference)?;
    let mut found = part.pads.iter().filter(|p| p.number == number);
    match (found.next(), found.next()) {
        (Some(p), None) => Some(p),
        _ => None,
    }
}

/// A two-pin component's value in pF and its two pins' nets.
struct Capacitor<'n> {
    reference: &'n str,
    picofarads: i64,
    pins: [(&'n str, &'n str); 2],
}

fn capacitors(n: &Netlist) -> Vec<Capacitor<'_>> {
    n.components
        .iter()
        .filter_map(|(reference, c)| {
            let picofarads = picofarads(&c.value)?;
            let pins: Vec<_> = n.pins_of(reference).collect();
            let [a, b] = pins.as_slice() else { return None };
            Some(Capacitor {
                reference,
                picofarads,
                pins: [*a, *b],
            })
        })
        .collect()
}

/// One role at one pin (or once for its group), waiting for a capacitor.
struct Slot<'r> {
    rule: &'r SupportCapacitor,
    label: String,
    /// The owner pins a matched capacitor's pad is measured to: the one pin,
    /// or each pin of the group (the nearest counts).
    pins: Vec<String>,
    owner_net: Option<String>,
    ground_net: Option<String>,
}

/// The gap from `c`'s pad on `net` to the nearest of `pins`, µm rounded up;
/// `None` when the board lacks a pad either side.
fn gap_um(board: &Board, mcu: &str, c: &Capacitor<'_>, net: &str, pins: &[String]) -> Option<i64> {
    let own = c.pins.iter().find(|(_, n)| *n == net)?.0;
    let own = pad(board, c.reference, own)?;
    pins.iter()
        .filter_map(|p| pad(board, mcu, p))
        .map(|p| {
            let g = crate::loops::gap_nm(own, p);
            g.div_euclid(1000) + i64::from(g.rem_euclid(1000) != 0)
        })
        .min()
}

/// Whether `c` can fill `s`: the value class, the two nets, and the bound.
fn fits(board: &Board, mcu: &str, s: &Slot<'_>, c: &Capacitor<'_>) -> Option<Option<i64>> {
    let (lo, hi) = s.rule.class();
    if !(lo..=hi).contains(&c.picofarads) {
        return None;
    }
    let (owner, ground) = (s.owner_net.as_deref()?, s.ground_net.as_deref()?);
    if owner == ground {
        return None;
    }
    let nets: BTreeSet<&str> = c.pins.iter().map(|(_, n)| *n).collect();
    if nets != BTreeSet::from([owner, ground]) {
        return None;
    }
    let gap = gap_um(board, mcu, c, owner, &s.pins);
    match (s.rule.bound_um, gap) {
        (Some(b), Some(g)) if g <= b => Some(Some(g)),
        (Some(_), _) => None,
        (None, g) => Some(g),
    }
}

/// Kuhn's augmenting path: slot `i` takes a capacitor, displacing another
/// slot's only if that slot can move.
fn augment(i: usize, edges: &[Vec<usize>], seen: &mut [bool], owner: &mut [Option<usize>]) -> bool {
    for &c in &edges[i] {
        if seen[c] {
            continue;
        }
        seen[c] = true;
        if owner[c].is_none_or(|j| augment(j, edges, seen, owner)) {
            owner[c] = Some(i);
            return true;
        }
    }
    false
}

/// Judges the support network of `spec`'s MCU from `netlist` and `board`.
/// Every failure is a finding in the record; `Err` is a row or a file the
/// gate cannot judge from.
pub fn judge(spec: &Specification, netlist: &Netlist, board: &Board) -> Result<Record, String> {
    let s = row(&spec.mcu.part)?;
    let mut findings = Vec::new();
    let mut fail = |kind: &'static str, message: String| findings.push(Finding { kind, message });
    // 1. The MCU, by its declared part.
    let mcus: Vec<&String> = netlist
        .components
        .iter()
        .filter(|(_, c)| c.value == spec.mcu.part)
        .map(|(r, _)| r)
        .collect();
    let [mcu] = mcus.as_slice() else {
        return Err(format!(
            "the netlist holds {} components of value `{}`, not one",
            mcus.len(),
            spec.mcu.part
        ));
    };
    let mcu = mcu.as_str();
    let net = |pin: &str| netlist.net(mcu, pin).map(str::to_owned);
    let group_net = |group: &str| -> Option<String> {
        let mut nets = s.group(group).iter().map(|p| net(p));
        let first = nets.next()??;
        nets.all(|n| n.as_deref() == Some(first.as_str()))
            .then_some(first)
    };
    let supply = spec.mcu.supply.as_str();
    let gnd = aethelgard_ir::GND;
    // 2. The supply, the grounds, the powered groups.
    let on = |group: &str, want: &dyn Fn(Option<&str>) -> bool| -> Vec<String> {
        s.group(group)
            .iter()
            .filter(|p| !want(netlist.net(mcu, p)))
            .cloned()
            .collect()
    };
    let off = on(&s.supply, &|n| n == Some(supply));
    if !off.is_empty() {
        fail(
            "support_pin_off_net",
            format!(
                "{mcu} {} pins {} are not on the supply `{supply}` ({})",
                s.supply,
                off.join(", "),
                power_parts::table().map_or_else(|_| String::new(), |t| t.cite(&s.powered_at))
            ),
        );
    }
    for g in &s.grounds {
        let off = on(g, &|n| n == Some(gnd));
        if !off.is_empty() {
            fail(
                "support_pin_off_net",
                format!("{mcu} {g} pins {} are not on `{gnd}`", off.join(", ")),
            );
        }
    }
    let domains: BTreeSet<&str> = spec.voltage_domains.iter().map(|d| d.id.as_str()).collect();
    for g in &s.powered {
        let off = on(g, &|n| n.is_some_and(|n| domains.contains(n)));
        if !off.is_empty() {
            fail(
                "support_pin_off_net",
                format!(
                    "{mcu} {g} pins {} are on no declared voltage domain",
                    off.join(", ")
                ),
            );
        }
    }
    // 3. The straps.
    let mut straps = Vec::new();
    for strap in &s.straps {
        let pin = &s.group(&strap.pin)[0];
        let have = net(pin);
        let want = group_net(&strap.to);
        if have.is_some() && have == want {
            straps.push(format!("{} (pin {pin}) on {}'s net", strap.pin, strap.to));
        } else {
            fail(
                "strap_wrong",
                format!(
                    "{mcu} {} (pin {pin}) is on {}, not on {}'s net {}",
                    strap.pin,
                    have.map_or_else(|| "no net".to_owned(), |n| format!("`{n}`")),
                    strap.to,
                    want.map_or_else(
                        || "(none, or more than one)".to_owned(),
                        |n| format!("`{n}`")
                    )
                ),
            );
        }
    }
    // 4. The capacitors, one to one.
    let mut slots: Vec<Slot<'_>> = Vec::new();
    for rule in &s.capacitors {
        let ground_net = group_net(&rule.ground);
        if rule.per_pin {
            for p in s.group(&rule.owner) {
                slots.push(Slot {
                    rule,
                    label: p.clone(),
                    pins: vec![p.clone()],
                    owner_net: net(p),
                    ground_net: ground_net.clone(),
                });
            }
        } else {
            slots.push(Slot {
                rule,
                label: rule.owner.clone(),
                pins: s.group(&rule.owner).to_vec(),
                owner_net: group_net(&rule.owner),
                ground_net: ground_net.clone(),
            });
        }
    }
    let caps = capacitors(netlist);
    let edges: Vec<Vec<usize>> = slots
        .iter()
        .map(|s| {
            (0..caps.len())
                .filter(|&c| fits(board, mcu, s, &caps[c]).is_some())
                .collect()
        })
        .collect();
    let mut owner: Vec<Option<usize>> = vec![None; caps.len()];
    for i in 0..slots.len() {
        augment(i, &edges, &mut vec![false; caps.len()], &mut owner);
    }
    let mut roles = Vec::new();
    for (i, slot) in slots.iter().enumerate() {
        let matched = owner.iter().position(|o| *o == Some(i));
        let gap = matched.and_then(|c| fits(board, mcu, slot, &caps[c]).flatten());
        let rule = power_parts::table()
            .map_or_else(|_| slot.rule.at.section.clone(), |t| t.cite(&slot.rule.at));
        if matched.is_none() {
            let (lo, hi) = slot.rule.class();
            fail(
                "support_capacitor_missing",
                format!(
                    "{mcu} {} {}: no capacitor of {lo} to {hi} pF between {} and {}{} serves it ({rule})",
                    slot.rule.owner,
                    slot.label,
                    slot.owner_net
                        .as_deref()
                        .map_or_else(|| "no net".to_owned(), |n| format!("`{n}`")),
                    slot.ground_net
                        .as_deref()
                        .map_or_else(|| "no net".to_owned(), |n| format!("`{n}`")),
                    slot.rule
                        .bound_um
                        .map_or_else(String::new, |b| format!(" within {b} um of the pin")),
                ),
            );
        }
        roles.push(RoleRecord {
            role: slot.rule.role.name(),
            pin: slot.label.clone(),
            part: matched.map(|c| caps[c].reference.to_owned()),
            picofarads: matched.map(|c| caps[c].picofarads),
            gap_um: gap,
            bound_um: slot.rule.bound_um,
            rule,
            verdict: if matched.is_some() { "pass" } else { "fail" },
        });
    }
    // 5. Every declared connector pin on the supply, `GND` or a debug net is
    // on that net in the netlist, pin for pin: a board whose connector swaps
    // SWDIO and SWCLK is not the board declared. The programming connector
    // is the specification's (`aethelgard_ir::connectors`); NRST reaching it
    // is recorded.
    let debug = |group: &str| s.group(group).first().and_then(|p| net(p));
    let expected = |declared: &str| -> Option<Option<String>> {
        Some(match declared {
            aethelgard_ir::SWDIO => debug("SWDIO"),
            aethelgard_ir::SWCLK => debug("SWCLK"),
            aethelgard_ir::NRST => debug("NRST"),
            d if d == gnd || d == supply => Some(d.to_owned()),
            _ => return None,
        })
    };
    let mut off: BTreeSet<&str> = BTreeSet::new();
    for c in spec.declared_connectors() {
        for p in &c.pins {
            let Some(want) = p.net.as_deref().and_then(expected) else {
                continue;
            };
            let have = netlist.net(&c.reference, &p.pin);
            if want.is_none() || have != want.as_deref() {
                off.insert(c.reference.as_str());
                fail(
                    "connector_pin_off_net",
                    format!(
                        "{} pin {} is declared on `{}` and is on {} in the netlist, where {mcu}'s pin for it is on {}",
                        c.reference,
                        p.pin,
                        p.net.as_deref().unwrap_or(""),
                        have.map_or_else(|| "no net".to_owned(), |n| format!("`{n}`")),
                        want.map_or_else(|| "no net".to_owned(), |n| format!("`{n}`")),
                    ),
                );
            }
        }
    }
    let programming = aethelgard_ir::connectors::programming_connector(spec)
        .filter(|(c, _)| !off.contains(c.reference.as_str()))
        .map(|(c, nrst)| Programming {
            connector: c.reference.clone(),
            nrst,
        });
    if programming.is_none() {
        fail(
            "no_programming_path",
            format!(
                "no declared connector carries SWDIO, SWCLK, `{gnd}` and `{supply}` to {mcu} in the netlist"
            ),
        );
    }
    // 6. The declared supply source.
    let supply_source = match &spec.mcu.supply_source {
        None => None,
        Some(src) => {
            if netlist.net(&src.connector, &src.pin) == Some(supply) {
                Some(format!("{} pin {}", src.connector, src.pin))
            } else {
                fail(
                    "supply_source_off_net",
                    format!(
                        "the declared source, {} pin {}, is not on `{supply}` in the netlist",
                        src.connector, src.pin
                    ),
                );
                None
            }
        }
    };
    let mut conditions: Vec<String> = s
        .capacitors
        .iter()
        .filter(|c| c.bound_um.is_none())
        .map(|c| format!("{} placement: {}", c.role.name(), c.bound_basis))
        .collect();
    conditions.push(
        "each capacitor's dielectric, rated voltage and ESR (VCAP: below 100 mOhm, DS12117 Table 113) are not chosen or judged: the BOM names a value and a 0402 footprint only".to_owned(),
    );
    if spec.mcu.supply_source.is_none() {
        conditions.push(
            "the supply's source is a regulator of the power tree, judged by the power gates, not here".to_owned(),
        );
    }
    Ok(Record {
        mcu: mcu.to_owned(),
        roles,
        straps,
        programming,
        supply_source,
        conditions,
        findings,
    })
}

#[cfg(test)]
mod tests {
    use super::picofarads;

    #[test]
    fn values_read_as_schematics_write_them_and_nothing_else() {
        for (v, pf) in [
            ("100n", 100_000),
            ("4u7", 4_700_000),
            ("2u2", 2_200_000),
            ("1u", 1_000_000),
            ("10n", 10_000),
            ("470p", 470),
            ("100nF", 100_000),
            ("10u GCM32EC71H106KA03", 10_000_000),
        ] {
            assert_eq!(picofarads(v), Some(pf), "{v}");
        }
        for v in [
            "", "n", "100", "1.5u", "u7", "100k", "2k2", "-1u", "1u0000", "1x",
        ] {
            assert_eq!(picofarads(v), None, "{v}");
        }
    }
}
