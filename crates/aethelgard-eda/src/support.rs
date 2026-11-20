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

/// The symbol a capacitor is drawn with: KiCad's own (`Device:C`), as every
/// capacitor row of the power parts table names it.
const CAPACITOR: &str = "Device:C";

/// KiCad's prefix for a pin the schematic leaves unconnected.
const UNCONNECTED: &str = "unconnected-";

/// One component of the netlist: its value, its symbol (`lib:part`), and
/// whether a property marks it unpopulated (`dnp`, `exclude_from_bom`,
/// `exclude_from_board`: adversarial A4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Component {
    pub value: String,
    pub symbol: Option<String>,
    pub unpopulated: bool,
}

/// Properties that take a part off the built board.
const UNPOPULATED: [&str; 3] = ["dnp", "exclude_from_bom", "exclude_from_board"];

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
        let symbol = match comp.one("libsource").map_err(|r| r.to_string())? {
            Some(l) => Some(format!("{}:{}", text(&l, "lib")?, text(&l, "part")?)),
            None => None,
        };
        let mut unpopulated = false;
        for p in comp.all("property") {
            if UNPOPULATED.contains(&text(&p, "name")?) {
                unpopulated = true;
            }
        }
        let previous = out.components.insert(
            reference.to_owned(),
            Component {
                value: value.to_owned(),
                symbol,
                unpopulated,
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
/// `470p`, `1u`, optionally followed by `F`, then optionally one space and a
/// capacitor part of the power parts table (`10u GCM32EC71H106KA03`).
/// Anything else (`100n DNP`) is not a capacitance. The gate's own reading,
/// shared with no emitter (`RUL-20`).
pub fn picofarads(value: &str) -> Option<i64> {
    let (token, part) = match value.split_once(' ') {
        Some((t, p)) => (t, Some(p)),
        None => (value, None),
    };
    if let Some(p) = part {
        let table = power_parts::table().ok()?;
        if !matches!(table.get(p).map(|r| &r.kind), Some(Kind::Capacitor(_))) {
            return None;
        }
    }
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

/// A net, quoted, or "no net".
fn quoted(n: Option<&str>) -> String {
    n.map_or_else(|| "no net".to_owned(), |n| format!("`{n}`"))
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
            if c.unpopulated || c.symbol.as_deref() != Some(CAPACITOR) {
                return None;
            }
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
    let table = power_parts::table().map_err(|e| e.to_string())?;
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
                "{mcu} {} pins {} are not on the supply `{supply}`",
                s.supply,
                off.join(", "),
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
    // Each powered group on a declared domain whose whole tolerance band lies
    // in the group's range (adversarial A2: Table 108 note 1's "in the
    // permitted range").
    let band = |id: &str| {
        spec.voltage_domains.iter().find(|d| d.id == id).map(|d| {
            let nominal = i64::from(d.nominal_millivolts);
            let spread = nominal * i64::from(d.tolerance_permille) / 1000;
            (nominal - spread, nominal + spread)
        })
    };
    for p in &s.powered {
        let r = p.millivolts;
        let off = on(&p.group, &|n| {
            n.and_then(band)
                .is_some_and(|(lo, hi)| lo >= r.min && hi <= r.max)
        });
        if !off.is_empty() {
            fail(
                "support_pin_off_net",
                format!(
                    "{mcu} {} pins {} are on no declared voltage domain held within {} to {} mV ({})",
                    p.group,
                    off.join(", "),
                    r.min,
                    r.max,
                    table.cite(&p.at)
                ),
            );
        }
    }
    // Each pin of an own-net group on a net no declared domain, `GND` or other
    // support pin of the MCU shares (adversarial A1).
    let support_pins: Vec<&String> = s.pins.values().flatten().collect();
    for o in &s.own_nets {
        for p in s.group(&o.group) {
            let mine = net(p);
            let shared = mine.as_deref().is_none_or(|n| {
                n == gnd
                    || band(n).is_some()
                    || support_pins
                        .iter()
                        .any(|q| *q != p && netlist.net(mcu, q) == Some(n))
            });
            if shared {
                fail(
                    "support_pin_off_net",
                    format!(
                        "{mcu} {} (pin {p}) is on {}, which is not a net of its own ({})",
                        o.group,
                        quoted(mine.as_deref()),
                        table.cite(&o.at)
                    ),
                );
            }
        }
    }
    // 3. The straps. A connector pin or test point that names `BOOT0` takes
    // the boot pin off its strap onto its own net, through one pull-down to
    // the strap's net judged against DS12117 (`AET-WO-1312`).
    let mut straps = Vec::new();
    let boot0_carried = aethelgard_ir::bench::carries_boot0(spec);
    let vdd = match table.get(&spec.mcu.part).map(|r| &r.kind) {
        Some(Kind::Mcu(m)) => &m.vdd_millivolts.range,
        _ => {
            return Err(format!(
                "the power parts table has no MCU row for `{}`",
                spec.mcu.part
            ));
        }
    };
    for strap in &s.straps {
        let pin = &s.group(&strap.pin)[0];
        if boot0_carried && strap.pin == aethelgard_ir::BOOT0 {
            match boot0_pull_down(spec, netlist, mcu, pin, group_net(&strap.to), vdd) {
                Ok(how) => straps.push(how),
                Err(why) => fail("strap_wrong", format!("{mcu} BOOT0 (pin {pin}): {why}")),
            }
            continue;
        }
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
                    quoted(have.as_deref()),
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
        let rule = table.cite(&slot.rule.at);
        if matched.is_none() {
            let (lo, hi) = slot.rule.class();
            fail(
                "support_capacitor_missing",
                format!(
                    "{mcu} {} {}: no capacitor of {lo} to {hi} pF between {} and {}{} serves it ({rule})",
                    slot.rule.owner,
                    slot.label,
                    quoted(slot.owner_net.as_deref()),
                    quoted(slot.ground_net.as_deref()),
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
            aethelgard_ir::BOOT0 => debug("BOOT0"),
            d if d == gnd || d == supply => Some(d.to_owned()),
            _ => return None,
        })
    };
    let mut off: BTreeSet<&str> = BTreeSet::new();
    for c in spec.declared_connectors() {
        for p in &c.pins {
            // A pin declared not connected is on no net (adversarial A5).
            if p.net.is_none() {
                if let Some(n) = netlist.net(&c.reference, &p.pin) {
                    off.insert(c.reference.as_str());
                    fail(
                        "connector_pin_off_net",
                        format!(
                            "{} pin {} is declared not connected and is on `{n}` in the netlist",
                            c.reference, p.pin
                        ),
                    );
                }
                continue;
            }
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
                        quoted(have),
                        quoted(want.as_deref()),
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
    // 5a. Bench access (`AET-WO-1312`): each test point's pad on the net it
    // declares, the boot and debug nets at the MCU's pin, and the reset switch
    // between the MCU's NRST and `GND` when it is declared, and only then.
    for t in spec.declared_test_points() {
        // As a connector pin's (step 5); any other domain bare, a signal as
        // its label.
        let want = expected(&t.net).unwrap_or_else(|| {
            Some(if spec.voltage_domains.iter().any(|v| v.id == t.net) {
                t.net.clone()
            } else {
                crate::extract::board_net(&t.net)
            })
        });
        let have = netlist.net(&t.reference, "1");
        // A test point is KiCad's one-pin test point and nothing else
        // (adversarial F4: a 0 ohm resistor declared as one shorted the
        // supply to GND and passed).
        let shape = netlist
            .components
            .get(&t.reference)
            .is_some_and(|c| c.symbol.as_deref() == Some(crate::schematic::TEST_POINT))
            && !netlist
                .nets
                .keys()
                .any(|(r, p)| *r == t.reference && p != "1");
        if !shape {
            fail(
                "test_point_off_net",
                format!(
                    "test point {} is not a one-pin `{}` in the netlist",
                    t.reference,
                    crate::schematic::TEST_POINT
                ),
            );
        } else if want.is_none() || have != want.as_deref() {
            fail(
                "test_point_off_net",
                format!(
                    "test point {} is declared on `{}` and is on {} in the netlist, not {}",
                    t.reference,
                    t.net,
                    quoted(have),
                    quoted(want.as_deref())
                ),
            );
        }
    }
    for (r, c) in &netlist.components {
        if c.symbol.as_deref() == Some(crate::schematic::TEST_POINT)
            && !spec
                .declared_test_points()
                .iter()
                .any(|t| t.reference == *r)
        {
            fail(
                "test_point_off_net",
                format!("{r} is a test point the specification does not declare"),
            );
        }
    }
    let switches: Vec<&String> = netlist
        .components
        .iter()
        .filter(|(_, c)| c.symbol.as_deref() == Some(crate::schematic::SWITCH))
        .map(|(r, _)| r)
        .collect();
    let nrst = debug("NRST");
    match (spec.mcu.reset_switch, switches.as_slice()) {
        (false, []) => {}
        (true, [sw]) => {
            let mut pins = [netlist.net(sw, "1"), netlist.net(sw, "2")];
            pins.sort_unstable();
            let mut want = [nrst.as_deref(), Some(gnd)];
            want.sort_unstable();
            let other = netlist
                .nets
                .keys()
                .any(|(r, p)| r == *sw && p != "1" && p != "2");
            if nrst.is_none() || pins != want || other || netlist.components[*sw].unpopulated {
                fail(
                    "reset_switch_wrong",
                    format!(
                        "reset switch {sw} is on {} and {}, not between {mcu}'s NRST {} and `{gnd}`, or is unpopulated",
                        quoted(pins[0]),
                        quoted(pins[1]),
                        quoted(nrst.as_deref())
                    ),
                );
            }
        }
        (declared, found) => fail(
            "reset_switch_wrong",
            format!(
                "mcu.reset_switch is {declared} and the netlist holds {} reset switch{}",
                found.len(),
                if found.len() == 1 { "" } else { "es" }
            ),
        ),
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
    // 7. The crystal beside a declared one (`AET-WO-4240c` §1.3).
    crystal(spec, s, netlist, mcu, &mut fail)?;
    let mut conditions: Vec<String> = s
        .capacitors
        .iter()
        .filter(|c| c.bound_um.is_none())
        .map(|c| format!("{} placement: {}", c.role.name(), c.bound_basis))
        .collect();
    conditions.push(
        "each capacitor's dielectric, rated voltage, package and ESR (VCAP: below 100 mOhm, DS12117 Table 113) are not judged: the gate reads a value and a symbol".to_owned(),
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

/// The crystal's pads by role, read from ABM8AIG "REVISED 06-13-19" p. 3
/// (pads 2 and 4 are ground, the lid; 1 and 3 the crystal) and KiCad's
/// `Crystal_GND24` ("GND on pins 2 and 4"): the gate's own copy, never the
/// emitter's (`RUL-20`).
const CRYSTAL_ENDS: [&str; 2] = ["1", "3"];
const CRYSTAL_GROUNDS: [&str; 2] = ["2", "4"];

/// Step 7 (`AET-WO-4240c` §1.3): beside a declared crystal, the MCU's
/// `OSC_IN` and `OSC_OUT` pins (the row's groups) each on a net of its own;
/// exactly one populated crystal of the declared part, its two ends on those
/// two nets and its two ground pads on `GND`; on each net exactly one
/// capacitor, to `GND`, of the declared value (`CL1` on `OSC_IN`, `CL2` on
/// `OSC_OUT`), named by a C0G part of the power parts table rated that value
/// and stated within ±1 % or ±0.25 pF; and no other pin on either net, the
/// MCU's included. With no crystal declared, no crystal may be drawn.
fn crystal(
    spec: &Specification,
    s: &Support,
    netlist: &Netlist,
    mcu: &str,
    fail: &mut impl FnMut(&'static str, String),
) -> Result<(), String> {
    let table = power_parts::table().map_err(|e| e.to_string())?;
    let gnd = aethelgard_ir::GND;
    let is_crystal = |c: &Component| {
        c.symbol.as_deref().is_some_and(|sym| {
            table
                .rows()
                .iter()
                .any(|r| matches!(r.kind, Kind::Crystal(_)) && r.symbol == sym)
        })
    };
    let Some(aethelgard_ir::clock::Hse::Crystal(declared)) = &spec.mcu.hse else {
        for (r, _) in netlist.components.iter().filter(|(_, c)| is_crystal(c)) {
            fail(
                "hse_crystal_undeclared",
                format!("{r} is a crystal, and the specification declares none"),
            );
        }
        return Ok(());
    };
    let row = match table.get(&declared.part) {
        Some(r) if matches!(r.kind, Kind::Crystal(_)) => r,
        _ => return Err(format!("`{}` is not a crystal row", declared.part)),
    };
    // The MCU's two oscillator pins, each on a net no other support pin,
    // domain or `GND` shares.
    let mut nets = Vec::new();
    for group in ["OSC_IN", "OSC_OUT"] {
        let [pin] = s.group(group) else {
            return Err(format!("the MCU's row has no single `{group}` pin"));
        };
        let n = netlist.net(mcu, pin);
        let shared = n.is_none_or(|n| {
            n == gnd
                || spec.voltage_domains.iter().any(|d| d.id == n)
                || s.pins
                    .values()
                    .flatten()
                    .any(|q| q != pin && netlist.net(mcu, q) == Some(n))
        });
        if shared {
            fail(
                "hse_pin_off_net",
                format!(
                    "{mcu} {group} (pin {pin}) is on {}, not a net of its own",
                    quoted(n)
                ),
            );
            return Ok(());
        }
        nets.push((pin.clone(), n.unwrap_or_default().to_owned()));
    }
    let (osc_in, osc_out) = (nets[0].1.as_str(), nets[1].1.as_str());
    // Exactly one crystal of the declared part.
    let found: Vec<&String> = netlist
        .components
        .iter()
        .filter(|(_, c)| {
            !c.unpopulated
                && c.value == row.part
                && c.symbol.as_deref() == Some(row.symbol.as_str())
        })
        .map(|(r, _)| r)
        .collect();
    let [y] = found.as_slice() else {
        fail(
            "hse_crystal_missing",
            format!(
                "the netlist holds {} populated `{}` drawn as `{}`, not one",
                found.len(),
                row.part,
                row.symbol
            ),
        );
        return Ok(());
    };
    let mut ends: Vec<Option<&str>> = CRYSTAL_ENDS.iter().map(|p| netlist.net(y, p)).collect();
    ends.sort_unstable();
    let mut want = vec![Some(osc_in), Some(osc_out)];
    want.sort_unstable();
    let other = netlist
        .pins_of(y)
        .any(|(p, _)| !CRYSTAL_ENDS.contains(&p) && !CRYSTAL_GROUNDS.contains(&p));
    if ends != want || other {
        fail(
            "hse_crystal_off_net",
            format!(
                "crystal {y}'s pads 1 and 3 are on {} and {}, and must be on `{osc_in}` and `{osc_out}`, with no pin but 1 to 4",
                quoted(ends[0]),
                quoted(ends[1])
            ),
        );
    }
    for p in CRYSTAL_GROUNDS {
        let n = netlist.net(y, p);
        if n != Some(gnd) {
            fail(
                "hse_crystal_ground_floating",
                format!("crystal {y} pad {p} is on {}, not `{gnd}`", quoted(n)),
            );
        }
    }
    // One load capacitor per oscillator net, to `GND`, of the declared value
    // and a C0G part; nothing else on either net.
    for ((pin, net), ff, name) in [
        (&nets[0], declared.load_caps_femtofarads[0], "CL1"),
        (&nets[1], declared.load_caps_femtofarads[1], "CL2"),
    ] {
        let net = net.as_str();
        // Every pin on the net but the MCU's own oscillator pin and the
        // crystal's: another MCU pin there is a part on it too (adversarial
        // F1).
        let on: Vec<&String> = netlist
            .nets
            .iter()
            .filter(|((r, p), n)| n.as_str() == net && !(r == mcu && p == pin) && r != *y)
            .map(|((r, _), _)| r)
            .collect();
        let caps: Vec<&String> = on
            .iter()
            .copied()
            .filter(|r| {
                let c = &netlist.components[r.as_str()];
                !c.unpopulated && c.symbol.as_deref() == Some(CAPACITOR) && {
                    let mut pins: Vec<&str> = netlist.pins_of(r).map(|(_, n)| n).collect();
                    pins.sort_unstable();
                    pins == [gnd, net] || pins == [net, gnd]
                }
            })
            .collect();
        if let Some(r) = on.iter().find(|r| !caps.contains(r)) {
            fail(
                "hse_net_extra_part",
                format!("{r} is on `{net}`, which carries only the crystal and its load capacitor"),
            );
        }
        let [c] = caps.as_slice() else {
            fail(
                "hse_load_capacitor_missing",
                format!(
                    "`{net}` has {} capacitors to `{gnd}`, not one ({name})",
                    caps.len()
                ),
            );
            continue;
        };
        let value = &netlist.components[c.as_str()].value;
        if picofarads(value).and_then(|pf| pf.checked_mul(1000)) != i64::try_from(ff).ok() {
            fail(
                "hse_load_capacitor_value",
                format!("{c} on `{net}` is `{value}`, and {name} is declared {ff} fF"),
            );
        }
        // The named row: C0G, rated exactly the declared value, and within
        // `4240` §9's family, ±1 % or ±0.25 pF (adversarial F2: the token
        // alone could name a row of another value).
        let row = value
            .split_once(' ')
            .and_then(|(_, part)| table.get(part))
            .and_then(|r| match &r.kind {
                Kind::Capacitor(k) => Some(k),
                _ => None,
            });
        let Some(k) = row.filter(|k| k.dielectric == power_parts::Dielectric::C0g) else {
            fail(
                "hse_load_capacitor_dielectric",
                format!(
                    "{c} on `{net}` is `{value}`, which names no C0G capacitor of the power parts table"
                ),
            );
            continue;
        };
        if k.rated_picofarads.value.checked_mul(1000) != i64::try_from(ff).ok() {
            fail(
                "hse_load_capacitor_value",
                format!(
                    "{c} on `{net}` names a part rated {} pF, and {name} is declared {ff} fF",
                    k.rated_picofarads.value
                ),
            );
        }
        let close = match &k.tolerance {
            Some(power_parts::Tolerance::Permille(l)) => l.value <= 10,
            Some(power_parts::Tolerance::Femtofarads(l)) => l.value <= 250,
            None => false,
        };
        if !close {
            fail(
                "hse_load_capacitor_tolerance",
                format!(
                    "{c} on `{net}` names a part whose tolerance is not stated within 1 % or 0.25 pF"
                ),
            );
        }
    }
    Ok(())
}

/// Resistances as schematics write them (`330`, `10k`, `2k2`, `1M`), in
/// ohms; `None` for anything else.
fn ohms(value: &str) -> Option<u64> {
    let (whole, unit, frac) = match value.find(['k', 'M']) {
        Some(i) => (&value[..i], &value[i..=i], &value[i + 1..]),
        None => (value, "", ""),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || !(frac.is_empty() || digits(frac)) {
        return None;
    }
    let scale: u64 = match unit {
        "k" => 1_000,
        "M" => 1_000_000,
        _ if frac.is_empty() => 1,
        _ => return None,
    };
    let frac_ohms = if frac.is_empty() {
        0
    } else {
        let places = u32::try_from(frac.len()).ok()?;
        let div = 10u64.checked_pow(places)?;
        frac.parse::<u64>().ok()?.checked_mul(scale)? / div
    };
    whole
        .parse::<u64>()
        .ok()?
        .checked_mul(scale)?
        .checked_add(frac_ohms)
}

/// BOOT0 on its own net with exactly one resistor from it to `ground` (the
/// strap's group's net), of a value [`aethelgard_ir::bench::judge_boot0_pull_down`]
/// passes over the MCU's supply range (`AET-WO-1312`); the record's line, or why not.
fn boot0_pull_down(
    spec: &Specification,
    netlist: &Netlist,
    mcu: &str,
    pin: &str,
    ground: Option<String>,
    vdd: &power_parts::Range,
) -> Result<String, String> {
    let Some(ground) = ground else {
        return Err("its strap's ground net is not one net".into());
    };
    let Some(boot0) = netlist.net(mcu, pin) else {
        return Err("it is on no net, so it floats".into());
    };
    if boot0 == ground {
        return Err(format!(
            "a connector pin or test point names BOOT0, and it is wired to `{ground}`, so nothing can drive it high"
        ));
    }
    if netlist
        .nets
        .iter()
        .any(|((r, p), n)| n == boot0 && r == mcu && p != pin)
    {
        return Err(format!("`{boot0}` is shared with another of {mcu}'s pins"));
    }
    let resistors: Vec<(&String, &Component)> = netlist
        .components
        .iter()
        .filter(|(r, c)| {
            let on = |n: &str| {
                ["1", "2"]
                    .iter()
                    .filter(|p| netlist.net(r, p) == Some(n))
                    .count()
            };
            c.symbol.as_deref() == Some(crate::schematic::R) && on(boot0) == 1 && on(&ground) == 1
        })
        .collect();
    let [(reference, part)] = resistors.as_slice() else {
        return Err(format!(
            "{} resistors join `{boot0}` to `{ground}`, not one",
            resistors.len()
        ));
    };
    if part.unpopulated {
        return Err(format!(
            "its pull-down {reference} is unpopulated, so it floats"
        ));
    }
    // Nothing else on the net but what the specification puts there: a
    // connector pin or test point that names BOOT0 (adversarial F1: a
    // pull-up beside the pull-down, or a device pin, passed).
    let declared = |r: &str, p: &str| {
        spec.declared_connectors().iter().any(|c| {
            c.reference == r
                && c.pins
                    .iter()
                    .any(|q| q.pin == p && q.net.as_deref() == Some(aethelgard_ir::BOOT0))
        }) || spec
            .declared_test_points()
            .iter()
            .any(|t| t.reference == r && p == "1" && t.net == aethelgard_ir::BOOT0)
    };
    if let Some((r, p)) = netlist.nets.iter().find_map(|((r, p), n)| {
        (n == boot0 && r != mcu && r != *reference && !declared(r, p)).then_some((r, p))
    }) {
        return Err(format!(
            "`{boot0}` also holds {r} pin {p}, which the specification does not put on BOOT0"
        ));
    }
    let value = ohms(&part.value).ok_or_else(|| {
        format!(
            "its pull-down {reference} has value `{}`, not a resistance",
            part.value
        )
    })?;
    let (lo, hi) = (
        u64::try_from(vdd.min).unwrap_or(0),
        u64::try_from(vdd.max).unwrap_or(0),
    );
    aethelgard_ir::bench::judge_boot0_pull_down(value, lo, hi)
        .map_err(|why| format!("its pull-down {reference} ({}): {why}", part.value))?;
    Ok(format!(
        "BOOT0 (pin {pin}) on `{boot0}`, pulled to `{ground}` by {reference} ({}), judged at {lo} to {hi} mV (DS12117 Tables 49 and 50)",
        part.value
    ))
}

#[cfg(test)]
mod tests {
    use super::{ohms, picofarads};

    #[test]
    fn resistances_read_as_schematics_write_them_and_nothing_else() {
        for (v, r) in [
            ("330", 330),
            ("10k", 10_000),
            ("2k2", 2_200),
            ("1M", 1_000_000),
            ("4M7", 4_700_000),
        ] {
            assert_eq!(ohms(v), Some(r), "{v}");
        }
        for v in ["", "k", "10K", "1.5k", "10k5k", "-1", "10 k", "1x", "100n"] {
            assert_eq!(ohms(v), None, "{v}");
        }
    }

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
