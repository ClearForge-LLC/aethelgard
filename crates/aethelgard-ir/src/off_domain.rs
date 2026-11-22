//! Paths onto an MCU pin from off the MCU's domain, judged at VDD = 0
//! (`AET-WO-1216b`, `1216` ruled 2026-11-22: D1 (c1) with (b)'s exemption,
//! D2, D4, D5).
//!
//! [`paths`] lists every path onto an MCU net from anything but
//! `mcu.supply` or `GND`, by the classes `1216`'s walk 1 lists: the battery
//! sense divider, an I2C bus's pull-ups, a crossing's A side and its
//! chip-select pull, a power-tree control wire, a pin device's pull and its
//! current output, a connector pin, a test point and a foreign clock. Each
//! carries the most its source can put on the pin while `mcu.supply` is
//! down, with its source and what kind of figure it is, or why no bound is
//! stated.
//!
//! **Which paths are judged.** A path whose rails can be up while
//! `mcu.supply` is below 1 V is judged at VDD = 0. A rail is exempt, and
//! judged by A1 alone with the MCU powered (`AET-WO-1211e`), only by drawn
//! wiring, never by a declaration (`INV-07`):
//! - its regulator chain passes through `mcu.supply` (it is downstream), and
//!   no connector carries a rail of that chain below `mcu.supply`; or
//! - its regulator's `EN` is wired to the `PG` of a regulator whose output is
//!   `mcu.supply` or downstream of it, and no connector carries the rail.
//!
//! A rail a connector carries can be fed from off the board, so it is never
//! exempt (`1216` F8). The battery and a rail no drawn regulator outputs
//! always lead. A device with two supplies is judged when either leads.
//!
//! **The limit** is DS12117 Rev 10 Table 10's absolute maximum at VDD = 0
//! (the MCU row's `io.absolute_maximum`): 4.0 V on every pin but `BOOT0`,
//! which takes 9.0 V. Table 11 note 3 (p. 104) says positive injection "does
//! not occur for input voltages lower than the specified maximum value", so
//! a voltage at or under it is admitted (D4); the voltage is the whole test.
//!
//! [`validate`] is `validate`'s step 25: the first judged path whose stated
//! figure is above its limit is refused, `OffDomainAboveLimit`, naming the
//! figure, its source and whether it is a maximum or a recommended operating
//! condition (D5). **A path with no stated bound is never passed:** it is not
//! judged here, and `verify` names it in `twin_limits` (D2), so no `validate`
//! byte moves.
//!
//! Nothing here is dynamic: power-down, brown-out and hot-plug are not
//! judged.

use std::collections::{BTreeMap, BTreeSet};

use crate::pin_parts::{Basis, PinKind};
use crate::power::{self, BATTERY_NET};
use crate::{IrValidationError, Specification, maximum_millivolts};

/// What kind of path a row is, by `1216`'s walk-1 class names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    I2cPull,
    BatteryDivider,
    CrossingASide,
    CrossingCsPull,
    ControlWire,
    PinDevicePull,
    PinDeviceCurrentOutput,
    Connector,
    TestPoint,
    ForeignClock,
}

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Self::I2cPull => "i2c_pull",
            Self::BatteryDivider => "battery_divider",
            Self::CrossingASide => "crossing_a_side",
            Self::CrossingCsPull => "crossing_cs_pull",
            Self::ControlWire => "control_wire",
            Self::PinDevicePull => "pin_device_pull",
            Self::PinDeviceCurrentOutput => "pin_device_current_output",
            Self::Connector => "connector",
            Self::TestPoint => "test_point",
            Self::ForeignClock => "foreign_clock",
        }
    }
}

/// Whether a path's source can be up while `mcu.supply` is below 1 V, and
/// why, as the drawn wiring says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lead {
    /// It can: judged at VDD = 0.
    Leads(String),
    /// The drawn tree places every rail of it after `mcu.supply`: judged by
    /// A1 alone.
    After(String),
}

/// The most a path's source puts on the pin while `mcu.supply` is down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bound {
    /// A stated figure, mV, what kind of figure it is, and where it comes
    /// from.
    Stated {
        millivolts: i64,
        basis: Basis,
        figure: String,
    },
    /// No figure is stated, and why.
    Unbounded(String),
}

/// One path onto an MCU net.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    pub net: String,
    pub class: Class,
    /// The source, as the specification declares it.
    pub source: String,
    /// The JSON path of the declaration.
    pub at: String,
    /// The rails the source is powered from: `battery`, domain ids, or none
    /// for an off-board end.
    pub rails: Vec<String>,
    /// The MCU pin, where the specification fixes it (a hand pin, a debug
    /// net, `OSC_IN`); `None` for one the solver places.
    pub pin: Option<String>,
    pub lead: Lead,
    pub bound: Bound,
}

impl Path {
    /// Whether step 25 judges it: its source leads and its bound is stated.
    pub fn judged(&self) -> bool {
        matches!(self.lead, Lead::Leads(_)) && matches!(self.bound, Bound::Stated { .. })
    }
}

/// The battery's rail name in [`Path::rails`].
pub const BATTERY: &str = "battery";

/// The net name a foreign clock's path is listed on: the MCU's `OSC_IN`
/// group, on `PH0`.
pub const FOREIGN_CLOCK_NET: &str = "OSC_IN";

/// Step 25: the first judged path above its limit at VDD = 0, in
/// [`paths`]' order, refused by `OffDomainAboveLimit`.
pub(crate) fn validate(spec: &Specification) -> Result<(), IrValidationError> {
    let all = paths(spec)?;
    if !all.iter().any(Path::judged) {
        return Ok(());
    }
    let io = mcu_io(spec)?;
    let pp = crate::power_parts::table()?;
    for p in all.iter().filter(|p| p.judged()) {
        let (
            Lead::Leads(why),
            Bound::Stated {
                millivolts,
                basis,
                figure,
            },
        ) = (&p.lead, &p.bound)
        else {
            continue;
        };
        let (limit, at) = io.absolute_maximum(p.pin.as_deref(), 0);
        if *millivolts > limit {
            let pin = p
                .pin
                .as_deref()
                .map_or("any pin the solver may place it on".to_owned(), |x| {
                    format!("`{x}`")
                });
            return Err(IrValidationError::OffDomainAboveLimit {
                net: p.net.clone(),
                path: p.at.clone(),
                millivolts: *millivolts,
                limit_millivolts: limit,
                message: format!(
                    "`{}`: {} can hold the pin at up to {millivolts} mV while `{}` is below 1 V ({why}); at VDD = 0, {pin} takes at most {limit} mV ({}). The figure is {}: {figure}",
                    p.net,
                    p.source,
                    spec.mcu.supply,
                    pp.cite(at),
                    match basis {
                        Basis::Maximum => "a stated maximum",
                        Basis::RecommendedOperating =>
                            "a recommended operating condition, not a stated maximum",
                    }
                ),
            });
        }
    }
    Ok(())
}

/// The MCU row's I/O facts; a judged path with no row to judge it by is
/// refused, never passed (`INV-06`).
fn mcu_io(spec: &Specification) -> Result<&'static crate::power_parts::McuIo, IrValidationError> {
    match crate::power_parts::table()?
        .get(&spec.mcu.part)
        .map(|r| &r.kind)
    {
        Some(crate::power_parts::Kind::Mcu(m)) => Ok(&m.io),
        _ => Err(IrValidationError::PartWithoutRow {
            reference: crate::names::mcu_reference(spec),
            part: spec.mcu.part.clone(),
        }),
    }
}

/// Every path onto an MCU net from off `mcu.supply`, in the order the
/// classes are listed above and in document order within each. A path whose
/// every rail is `mcu.supply` is not a path and is not listed.
pub fn paths(spec: &Specification) -> Result<Vec<Path>, IrValidationError> {
    let w = Walk::new(spec)?;
    let mut out = Vec::new();
    // belt-and-suspenders: step 5a puts every I2C bus on `mcu.supply`, so no
    // pull-up is off the domain today; the class is listed so a later bus
    // on another domain is judged, not missed.
    for (i, b) in spec.i2c_buses.iter().enumerate() {
        for line in [&b.sda, &b.scl] {
            w.push_rail(
                &mut out,
                &line.net,
                Class::I2cPull,
                format!("`{}`'s {} ohm pull-up", b.id, b.pull_up_ohms),
                format!("i2c_buses[{i}].pull_up_ohms"),
                &b.domain,
                line.pin.clone(),
            );
        }
    }
    if let Some(p) = &spec.power {
        let bmax = power::battery_max_millivolts(&p.battery);
        for (i, s) in p.sense.iter().enumerate() {
            let et = w.resistor_ppm(&s.top.part)?;
            let eb = w.resistor_ppm(&s.bottom.part)?;
            let mv = ceil_div(power::sense_max_microvolts(bmax, s, et, eb), 1000);
            out.push(w.path(
                &s.net,
                Class::BatteryDivider,
                format!(
                    "the battery through `power.sense[{i}]` ({} {} ohm over {} {} ohm)",
                    s.top.reference, s.top.ohms, s.bottom.reference, s.bottom.ohms
                ),
                format!("power.sense[{i}]"),
                vec![BATTERY.to_owned()],
                None,
                Bound::Stated {
                    millivolts: i64::try_from(mv).unwrap_or(i64::MAX),
                    basis: Basis::Maximum,
                    figure: format!(
                        "the battery's declared maximum, {} x {} mV, through the divider's largest ratio (each resistor at its tolerance, TCR over `power.conditions` and drift)",
                        p.battery.cells.max, p.battery.cell_max_millivolts.value
                    ),
                },
            ));
        }
        // belt-and-suspenders: step 16 puts a crossing's A side on its bus's
        // domain and step 5a that bus on `mcu.supply`, so no crossing row is
        // off the domain today.
        for (i, c) in p.crossings.iter().enumerate() {
            for net in crossing_nets(spec, &c.bus) {
                w.push_rail(
                    &mut out,
                    net,
                    Class::CrossingASide,
                    format!("`{}`'s A side (VCCA)", c.reference),
                    format!("power.crossings[{i}].a_domain"),
                    &c.a_domain,
                    w.declared_pin(net),
                );
            }
            for d in spec
                .declared_spi_devices()
                .iter()
                .filter(|d| d.bus == c.bus)
            {
                w.push_rail(
                    &mut out,
                    &d.cs.net,
                    Class::CrossingCsPull,
                    format!(
                        "`{}`'s chip-select pull-up `{}` ({} ohm)",
                        c.reference, c.cs_pull_up.reference, c.cs_pull_up.ohms
                    ),
                    format!("power.crossings[{i}].cs_pull_up"),
                    &c.a_domain,
                    d.cs.pin.clone(),
                );
            }
        }
        let owners = p
            .regulators
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    format!("power.regulators[{i}]"),
                    &r.reference,
                    &r.output,
                    &r.controls,
                )
            })
            .chain(p.crossings.iter().enumerate().map(|(i, c)| {
                (
                    format!("power.crossings[{i}]"),
                    &c.reference,
                    &c.b_domain,
                    &c.controls,
                )
            }));
        for (at, reference, rail, controls) in owners {
            for (k, c) in controls.iter().enumerate() {
                let Some(wire) = c
                    .wire
                    .as_ref()
                    .filter(|x| w.mcu_nets.contains(x.net.as_str()))
                else {
                    continue;
                };
                if let Some(lead) = w.lead(std::slice::from_ref(rail)) {
                    out.push(Path {
                        net: wire.net.clone(),
                        class: Class::ControlWire,
                        source: format!("`{reference}`'s {} pin, wired to the net", c.pin),
                        at: format!("{at}.controls[{k}].wire"),
                        rails: vec![rail.clone()],
                        pin: w.declared_pin(&wire.net),
                        lead,
                        bound: Bound::Unbounded(format!(
                            "the power parts table states no level for `{reference}`'s {} pin onto another net",
                            c.pin
                        )),
                    });
                }
            }
        }
    }
    let table = crate::parts::table()?;
    for (i, d) in spec.declared_pin_devices().iter().enumerate() {
        let device = format!("pin_devices[{i}]");
        // Validated by step 24: every part is a busless row.
        let Some(row) = table.pin_part(&d.part) else {
            continue;
        };
        for (k, s) in d.signals.iter().enumerate() {
            let Some(r) = s.pull.as_ref().and_then(|p| p.resistor.as_ref()) else {
                continue;
            };
            w.push_rail(
                &mut out,
                &s.net,
                Class::PinDevicePull,
                format!(
                    "`{}`'s pull `{}` ({} ohm) to `{}`",
                    d.reference, r.reference, r.ohms, r.to
                ),
                format!("{device}.signals[{k}].pull.resistor"),
                &r.to,
                s.pin.clone(),
            );
        }
        let rails: Vec<String> = d
            .supplies
            .values()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        for (k, a) in d.analog.iter().enumerate() {
            let Some(PinKind::CurrentOutput {
                output_max_ua,
                output_max_basis,
                ..
            }) = row.pins.get(&a.device_pin).map(|p| &p.kind)
            else {
                continue;
            };
            let crate::pin_devices::Transfer::CurrentOutput(c) = &a.transfer;
            let at = format!(".analog[{k}].transfer.current_output.resistor");
            let r_hi =
                crate::pin_devices::resistor_high_ohms(spec, d, row, &c.resistor, &device, &at)?;
            let Some(lead) = w.lead(&rails) else {
                continue;
            };
            out.push(Path {
                net: a.net.clone(),
                class: Class::PinDeviceCurrentOutput,
                source: format!(
                    "`{}`'s {} into `{}` ({} ohm)",
                    d.reference, a.device_pin, c.resistor.reference, c.resistor.ohms
                ),
                at: format!("{device}{at}"),
                rails: rails.clone(),
                pin: a.pin.clone(),
                lead,
                bound: Bound::Stated {
                    millivolts: i64::try_from(ceil_div(i128::from(*output_max_ua) * r_hi, 1000))
                        .unwrap_or(i64::MAX),
                    basis: *output_max_basis,
                    figure: format!(
                        "`{}`'s {} uA (the parts table's `output_max_ua`, {} {}) into `{}` at its highest, {r_hi} ohm over the device's temperature, as if the device drives: no held page states its level asleep or its clamp's",
                        d.part, output_max_ua, row.document, row.pins[&a.device_pin].page, c.resistor.reference
                    ),
                },
            });
        }
    }
    for (j, c) in spec.declared_connectors().iter().enumerate() {
        let carried: Vec<&str> = c
            .pins
            .iter()
            .filter_map(|p| p.net.as_deref())
            .filter(|n| w.is_rail(n))
            .collect();
        for (k, pin) in c.pins.iter().enumerate() {
            let Some(net) = pin.net.as_deref().filter(|n| w.mcu_nets.contains(n)) else {
                continue;
            };
            out.push(w.off_board(
                net,
                Class::Connector,
                format!("`{}`.{} ({})", c.reference, pin.pin, c.part),
                format!("connectors[{j}].pins[{k}]"),
                if carried.is_empty() {
                    "no specification field bounds what an off-board end drives; the connector carries no rail".to_owned()
                } else {
                    format!(
                        "no specification field bounds what an off-board end drives; the connector also carries {}, so a peer it powers can drive the pin",
                        carried.join(", ")
                    )
                },
            ));
        }
    }
    for (k, t) in spec.declared_test_points().iter().enumerate() {
        if w.mcu_nets.contains(t.net.as_str()) {
            out.push(w.off_board(
                &t.net,
                Class::TestPoint,
                format!("test point `{}`", t.reference),
                format!("test_points[{k}]"),
                "a probe, when attached, drives what the bench drives".to_owned(),
            ));
        }
    }
    if let Some(crate::clock::Hse::External(x)) = &spec.mcu.hse {
        let mut p = w.off_board(
            FOREIGN_CLOCK_NET,
            Class::ForeignClock,
            format!("the external clock ({})", x.source),
            "mcu.hse.external".to_owned(),
            "the declaration states its frequency and deviation, not its level".to_owned(),
        );
        p.pin = Some("PH0".to_owned());
        out.push(p);
    }
    Ok(out)
}

/// The nets a crossing's A side drives toward the MCU: its bus's lines and
/// each chip select of a device on it.
fn crossing_nets<'s>(spec: &'s Specification, bus: &str) -> Vec<&'s String> {
    let mut nets: Vec<&String> = spec
        .spi_buses
        .iter()
        .filter(|b| b.id == bus)
        .flat_map(|b| [&b.sck.net, &b.mosi.net, &b.miso.net])
        .collect();
    nets.extend(
        spec.declared_spi_devices()
            .iter()
            .filter(|d| d.bus == bus)
            .map(|d| &d.cs.net),
    );
    nets.sort();
    nets.dedup();
    nets
}

fn ceil_div(a: i128, b: i128) -> i128 {
    (a + b - 1) / b
}

/// What the walk reads once: the MCU's nets, the drawn tree, the connector
/// rails and the declared pins.
struct Walk<'s> {
    spec: &'s Specification,
    supply: &'s str,
    mcu_nets: BTreeSet<&'s str>,
    /// Each regulator by its output.
    sources: BTreeMap<&'s str, &'s power::Regulator>,
    /// Each rail any connector pin carries, with the first connector.
    carried: BTreeMap<&'s str, &'s str>,
    pins: BTreeMap<&'s str, &'s str>,
}

impl<'s> Walk<'s> {
    fn new(spec: &'s Specification) -> Result<Self, IrValidationError> {
        let domains: BTreeSet<&str> = spec.voltage_domains.iter().map(|d| d.id.as_str()).collect();
        let mut mcu_nets: BTreeSet<&str> = crate::names::spelled_nets(spec)
            .into_iter()
            .filter(|n| !domains.contains(n))
            .collect();
        for d in spec.declared_pin_devices() {
            mcu_nets.extend(d.signals.iter().map(|s| s.net.as_str()));
            mcu_nets.extend(d.analog.iter().map(|a| a.net.as_str()));
        }
        mcu_nets.extend(crate::DEBUG_NETS);
        let mut pins: BTreeMap<&str, &str> = BTreeMap::from([
            (crate::SWDIO, "PA13"),
            (crate::SWCLK, "PA14"),
            (crate::NRST, crate::NRST),
            (crate::BOOT0, crate::BOOT0),
        ]);
        let mut hand = |net: &'s str, pin: &'s Option<String>| {
            if let Some(p) = pin {
                pins.insert(net, p.as_str());
            }
        };
        for b in &spec.i2c_buses {
            hand(&b.sda.net, &b.sda.pin);
            hand(&b.scl.net, &b.scl.pin);
        }
        for b in &spec.spi_buses {
            for l in [&b.sck, &b.mosi, &b.miso] {
                hand(&l.net, &l.pin);
            }
        }
        for d in spec.declared_spi_devices() {
            hand(&d.cs.net, &d.cs.pin);
        }
        for p in &spec.serial_ports {
            hand(&p.tx.net, &p.tx.pin);
            hand(&p.rx.net, &p.rx.pin);
        }
        for m in &spec.motor_outputs {
            hand(&m.label, &m.pin);
        }
        for d in spec.declared_pin_devices() {
            for s in &d.signals {
                hand(&s.net, &s.pin);
            }
            for a in &d.analog {
                hand(&a.net, &a.pin);
            }
        }
        let sources = spec
            .power
            .iter()
            .flat_map(|p| &p.regulators)
            .map(|r| (r.output.as_str(), r))
            .collect();
        let mut carried = BTreeMap::new();
        for c in spec.declared_connectors() {
            for n in c.pins.iter().filter_map(|p| p.net.as_deref()) {
                if domains.contains(n) || n == BATTERY_NET {
                    carried.entry(n).or_insert(c.reference.as_str());
                }
            }
        }
        Ok(Self {
            spec,
            supply: &spec.mcu.supply,
            mcu_nets,
            sources,
            carried,
            pins,
        })
    }

    fn is_rail(&self, net: &str) -> bool {
        net == BATTERY_NET || self.spec.voltage_domains.iter().any(|d| d.id == net)
    }

    fn declared_pin(&self, net: &str) -> Option<String> {
        self.pins.get(net).map(|p| (*p).to_owned())
    }

    /// A regulator chain from `rail` upward: `rail`, its regulator's input,
    /// and so on to a rail no regulator outputs.
    fn chain(&self, rail: &'s str) -> Vec<&'s str> {
        let mut out = vec![rail];
        let mut at = rail;
        while let Some(r) = self.sources.get(at) {
            at = r.input.as_str();
            // A loop is refused by step 16; stop rather than spin.
            if out.contains(&at) {
                break;
            }
            out.push(at);
        }
        out
    }

    /// Whether `rails` can lead `mcu.supply`, the first that does; `None`
    /// when every one is `mcu.supply` itself (not a path).
    fn lead(&self, rails: &[String]) -> Option<Lead> {
        let mut after = Vec::new();
        for rail in rails.iter().filter(|r| r.as_str() != self.supply) {
            match self.rail_lead(rail) {
                Lead::Leads(why) => return Some(Lead::Leads(why)),
                Lead::After(why) => after.push(why),
            }
        }
        (!after.is_empty()).then(|| Lead::After(after.join("; ")))
    }

    fn rail_lead(&self, rail: &str) -> Lead {
        let supply = self.supply;
        if rail == BATTERY {
            return Lead::Leads("the battery is the tree's root".to_owned());
        }
        let Some((&rail, reg)) = self.sources.get_key_value(rail) else {
            return Lead::Leads(format!(
                "no regulator in the drawn tree outputs `{rail}`, so nothing orders it after `{supply}`"
            ));
        };
        let chain = self.chain(rail);
        let on_connector = |rails: &[&str]| {
            rails
                .iter()
                .find_map(|r| self.carried.get(r).map(|c| (*r, *c)))
                .map(|(r, c)| {
                    Lead::Leads(format!(
                        "connector `{c}` carries `{r}`, which can be fed from off the board, so the tree's order does not hold for it"
                    ))
                })
        };
        if let Some(at) = chain.iter().position(|r| *r == supply) {
            return on_connector(&chain[..at]).unwrap_or_else(|| {
                Lead::After(format!(
                    "`{rail}` is downstream of `{supply}` ({})",
                    chain.join(" <- ")
                ))
            });
        }
        let en = reg
            .controls
            .iter()
            .find(|k| k.pin == "EN")
            .and_then(|k| k.wire.as_ref())
            .map(|w| w.net.as_str());
        // A PG on `mcu.supply`'s chain at or below it: the PG of a regulator
        // whose output is `mcu.supply` or downstream of it. A PG upstream of
        // `mcu.supply` does not order a sibling after it.
        let gated = self.sources.values().find_map(|r| {
            let below = self.chain(r.output.as_str()).contains(&supply);
            r.controls
                .iter()
                .filter(|k| k.pin == "PG")
                .filter_map(|k| k.wire.as_ref())
                .find(|w| below && Some(w.net.as_str()) == en)
                .map(|w| (r.reference.as_str(), w.net.as_str()))
        });
        match gated {
            Some((by, net)) => on_connector(&[rail]).unwrap_or_else(|| {
                Lead::After(format!(
                    "`{}`'s EN is wired to `{net}`, `{by}`'s PG on `{supply}`'s chain",
                    reg.reference
                ))
            }),
            None => Lead::Leads(format!(
                "`{rail}` ({}) and `{supply}` ({}) share no chain, and no EN on `{rail}`'s is wired to a PG on `{supply}`'s",
                chain.join(" <- "),
                self.chain(supply).join(" <- ")
            )),
        }
    }

    fn resistor_ppm(&self, part: &str) -> Result<i128, IrValidationError> {
        let pp = crate::power_parts::table()?;
        let (Some(crate::power_parts::Kind::Resistor(r)), Some(p)) =
            (pp.get(part).map(|row| &row.kind), self.spec.power.as_ref())
        else {
            // Step 16 refused a sense resistor with no resistor row.
            return Err(IrValidationError::PartWithoutRow {
                reference: "power.sense".to_owned(),
                part: part.to_owned(),
            });
        };
        Ok(power::resistor_error_ppm(
            r,
            &p.conditions.board_temperature_celsius,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn path(
        &self,
        net: &str,
        class: Class,
        source: String,
        at: String,
        rails: Vec<String>,
        pin: Option<String>,
        bound: Bound,
    ) -> Path {
        let lead = self
            .lead(&rails)
            .unwrap_or_else(|| Lead::After(format!("on `{}`", self.supply)));
        Path {
            net: net.to_owned(),
            class,
            source,
            at,
            rails,
            pin: pin.or_else(|| self.declared_pin(net)),
            lead,
            bound,
        }
    }

    /// A pull or line to `rail`'s maximum; nothing when `rail` is
    /// `mcu.supply`.
    #[allow(clippy::too_many_arguments)]
    fn push_rail(
        &self,
        out: &mut Vec<Path>,
        net: &str,
        class: Class,
        source: String,
        at: String,
        rail: &str,
        pin: Option<String>,
    ) {
        if rail == self.supply {
            return;
        }
        let Some(d) = self.spec.voltage_domains.iter().find(|d| d.id == rail) else {
            return;
        };
        out.push(self.path(
            net,
            class,
            source,
            at,
            vec![rail.to_owned()],
            pin,
            Bound::Stated {
                millivolts: i64::from(maximum_millivolts(d)),
                basis: Basis::Maximum,
                figure: format!(
                    "`{rail}`'s declared maximum, {} mV + {} permille, open circuit (the pin's leakage is its only load)",
                    d.nominal_millivolts, d.tolerance_permille
                ),
            },
        ));
    }

    fn off_board(&self, net: &str, class: Class, source: String, at: String, why: String) -> Path {
        Path {
            net: net.to_owned(),
            class,
            source,
            at,
            rails: Vec::new(),
            pin: self.declared_pin(net),
            lead: Lead::Leads("an off-board end".to_owned()),
            bound: Bound::Unbounded(why),
        }
    }
}
