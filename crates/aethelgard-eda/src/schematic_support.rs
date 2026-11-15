//! The MCU's support network on the sheet (`AET-WO-4300g` §1.3), drawn from
//! the MCU's row in `power-parts.json` (`kind.mcu.support`), never from the
//! symbol's pin names: each pin's connection by its package position, then
//! each capacitor role, one at each owner pin or one for the group, anchored
//! by the pin it serves so the placer puts it there.
//!
//! What each group is tied to:
//! - the supply group (`VDD`) and every `powered` group (`VDDA`, `VDD33USB`,
//!   `VBAT`) to the MCU's supply: no other domain is declared for them, an
//!   engine choice the row's straps confirm for `VBAT`;
//! - the ground groups to `GND`;
//! - each strap's pin to its group's connection (`BOOT0` to `GND`, `PDR_ON`
//!   to the supply, `VREF+` to `VDDA`'s);
//! - `VCAP` pins each to its own net, `NRST` to [`NRST`], and the debug
//!   pins to [`SWDIO`] and [`SWCLK`], which a declared connector carries.
//!
//! The support gate (`crate::support`) re-reads the emitted netlist against
//! the same row; it never reads this module's choices (`RUL-20`).

use std::collections::BTreeMap;

use aethelgard_ir::power_parts::{self, Kind, Support};
use aethelgard_ir::{NRST, SWCLK, SWDIO};

use super::{Anchor, Builder, Conn, EmitError, Owner};

/// The MCU's row's support network.
pub(super) fn row() -> Result<&'static Support, EmitError> {
    let table = power_parts::table().map_err(|e| EmitError::Internal(e.to_string()))?;
    table
        .rows()
        .iter()
        .find_map(|r| match &r.kind {
            Kind::Mcu(m) => Some(&m.support),
            _ => None,
        })
        .ok_or_else(|| EmitError::Internal("the power parts table has no MCU row".into()))
}

/// Each support pin's connection by package position, and the capacitors,
/// drawn after the MCU's reference is taken and before its part is pushed.
pub(super) fn draw(b: &mut Builder<'_>, mcu: &str) -> Result<BTreeMap<String, Conn>, EmitError> {
    let s = row()?;
    let supply = b.spec.mcu.supply.clone();
    let mut by_group: BTreeMap<&str, Conn> = BTreeMap::new();
    by_group.insert(s.supply.as_str(), b.rail(&supply));
    for g in &s.powered {
        by_group.insert(g.as_str(), b.rail(&supply));
    }
    for g in &s.grounds {
        by_group.insert(g.as_str(), b.ground());
    }
    for (group, net) in [("NRST", NRST), ("SWDIO", SWDIO), ("SWCLK", SWCLK)] {
        if !s.group(group).is_empty() {
            by_group.insert(group, Conn::Net(b.own_net(net.to_owned())?));
        }
    }
    for strap in &s.straps {
        let to = by_group.get(strap.to.as_str()).cloned().ok_or_else(|| {
            EmitError::Internal(format!(
                "strap {} names {}, which has no connection",
                strap.pin, strap.to
            ))
        })?;
        by_group.insert(strap.pin.as_str(), to);
    }
    let mut conns = BTreeMap::new();
    for (group, pins) in &s.pins {
        if group == "VCAP" {
            for (i, pin) in pins.iter().enumerate() {
                let net = b.own_net(format!("{mcu}_VCAP{}", i + 1))?;
                conns.insert(pin.clone(), Conn::Net(net));
            }
            continue;
        }
        let Some(c) = by_group.get(group.as_str()) else {
            return Err(EmitError::Internal(format!(
                "support group {group} has no connection"
            )));
        };
        for pin in pins {
            conns.insert(pin.clone(), c.clone());
        }
    }
    for cap in &s.capacitors {
        let ground = by_group
            .get(cap.ground.as_str())
            .cloned()
            .ok_or_else(|| EmitError::Internal(format!("{} has no ground", cap.role.name())))?;
        let value = super::power::farads(
            u64::try_from(cap.picofarads)
                .map_err(|_| EmitError::Internal("a negative capacitance".into()))?,
        );
        let owners = s.group(&cap.owner);
        let served: &[String] = if cap.per_pin { owners } else { &owners[..1] };
        for pin in served {
            let at = conns.get(pin).cloned().ok_or_else(|| {
                EmitError::Internal(format!("{}: pin {pin} has no connection", cap.role.name()))
            })?;
            let decouples = matches!(at, Conn::Rail(_));
            b.passive('C', value.clone(), at, ground.clone())?;
            if let Some(c) = b.parts.last_mut() {
                c.owner = Some(Owner {
                    ic: mcu.to_owned(),
                    anchor: Some(Anchor {
                        pads: vec![("1".into(), pin.clone())],
                        loop_rule: None,
                    }),
                    decouples,
                });
            }
        }
    }
    Ok(conns)
}
