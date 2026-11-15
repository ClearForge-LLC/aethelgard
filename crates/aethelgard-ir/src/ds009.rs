//! DS-009's port pinouts, cited as facts (ruled D2 (a) on PR #134,
//! provisionally, pending Scotty): the document is held by sha256 and not
//! vendored, and its mark is used nowhere. A port is named by the document,
//! as `"DS-009 TELEM"`.
//!
//! Everything that judges a connector against DS-009 is in this module: the
//! ports, their roles and [`judge`]. Ruling (c), explicit maps only, would be
//! this module, the `standard` field and their refusals deleted.
//!
//! The ports are the ones a vendored part can carry (D4). The analog power
//! port is `AET-WO-4300d`'s (D6), and the SPI and full debug ports have no
//! vendored part. A role whose net this schema cannot yet declare (a serial
//! port's TX, a safety switch, SWD) refuses every net until the work order
//! that declares it, unless DS-009 lets the pin float.

use std::collections::BTreeMap;

use crate::connectors::{Connector, NetKind, Part, Series};
use crate::{IrValidationError as E, Specification};

/// DS-009 rev 0.1.0, at commit `48dd6d1d` (2020-04-24), its only commit.
pub const DOCUMENT: &str = "DS-009 Connector Standard, rev 0.1.0, commit 48dd6d1d";
pub const SHA256: &str = "dbc6b94e0367ef0700f3ae2ed5958fd00e8938435a12c7cf528a772ec1a5bbdb";

/// What a port pin must carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// "VCC +5V": a voltage domain of nominal 5000 mV (ruled D5).
    Vcc5V,
    /// "TARGET PROCESSOR VOLTAGE": the microcontroller's supply domain.
    TargetVoltage,
    /// "VDD_3V3": a voltage domain of nominal 3300 mV.
    Vdd3V3,
    Ground,
    I2cScl,
    I2cSda,
    /// A role no net of this schema can carry yet: DS-009's words.
    Undeclared(&'static str),
    /// A role DS-009 lets float: `null`, until its net can be declared.
    MayFloat(&'static str),
}

impl Role {
    /// DS-009's words for the pin.
    pub fn name(self) -> &'static str {
        match self {
            Role::Vcc5V => "VCC +5V",
            Role::TargetVoltage => "TARGET PROCESSOR VOLTAGE",
            Role::Vdd3V3 => "VDD_3V3",
            Role::Ground => "GND",
            Role::I2cScl => "I2C SCL",
            Role::I2cSda => "I2C SDA",
            Role::Undeclared(n) | Role::MayFloat(n) => n,
        }
    }
}

/// One port: its name, its series and pin 1 to N's roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Port {
    pub name: &'static str,
    pub series: Series,
    pub roles: &'static [Role],
    /// The PDF page, then the page number printed on it.
    pub section: &'static str,
}

use Role::*;

pub const PORTS: [Port; 5] = [
    Port {
        name: "DS-009 TELEM",
        series: Series::JstGh,
        roles: &[
            Vcc5V,
            Undeclared("TX (OUT)"),
            Undeclared("RX (IN)"),
            MayFloat("CTS (IN)"),
            MayFloat("RTS (OUT)"),
            Ground,
        ],
        section: "PDF p. 6 (printed 5), Telemetry Port: CTS and RTS float without flow control",
    },
    Port {
        name: "DS-009 GPS",
        series: Series::JstGh,
        roles: &[
            Vcc5V,
            Undeclared("TX (OUT)"),
            Undeclared("RX (IN)"),
            I2cScl,
            I2cSda,
            Ground,
        ],
        section: "PDF p. 8 (printed 7), Basic GPS port",
    },
    Port {
        name: "DS-009 GPS FULL",
        series: Series::JstGh,
        roles: &[
            Vcc5V,
            Undeclared("TX (OUT)"),
            Undeclared("RX (IN)"),
            I2cScl,
            I2cSda,
            Undeclared("SAFETY SWITCH IN"),
            Undeclared("SAFETY LED"),
            Vdd3V3,
            Undeclared("BUZZER"),
            Ground,
        ],
        section: "PDF p. 7 (printed 6), Full GPS plus Safety Switch Port",
    },
    Port {
        name: "DS-009 I2C",
        series: Series::JstGh,
        roles: &[Vcc5V, I2cScl, I2cSda, Ground],
        section: "PDF p. 9 (printed 8), I2C port",
    },
    Port {
        name: "DS-009 DEBUG MINI",
        series: Series::JstSh,
        roles: &[
            TargetVoltage,
            Undeclared("CONSOLE TX (OUT)"),
            Undeclared("CONSOLE RX (IN)"),
            Undeclared("SWDIO"),
            Undeclared("SWCLK"),
            Ground,
        ],
        section: "PDF p. 12 (printed 11), Debug Mini: JST SH SM06B",
    },
];

/// The row named `name`.
pub fn port(name: &str) -> Option<&'static Port> {
    PORTS.iter().find(|p| p.name == name)
}

/// Refuses `c` where it contradicts the port it names: an unknown port, a
/// part of another series or pin count, a supply pin whose net is not the
/// supply the port names (D5), or another pin whose net is not its role's.
/// The two I2C pins of one port must be one bus's.
pub(crate) fn judge(
    spec: &Specification,
    c: &Connector,
    part: &Part,
    standard: &str,
    kinds: &BTreeMap<&str, NetKind<'_>>,
) -> Result<(), E> {
    let id = || c.reference.clone();
    let port = port(standard).ok_or_else(|| E::UnknownConnectorStandard {
        id: id(),
        standard: standard.to_owned(),
    })?;
    if port.series != part.series || port.roles.len() != usize::from(part.pins) {
        return Err(E::ConnectorPartContradictsStandard {
            id: id(),
            part: part.name.to_owned(),
            standard: port.name.to_owned(),
        });
    }
    let mcu_supply = spec
        .voltage_domains
        .iter()
        .find(|d| d.id == spec.mcu.supply)
        .map(|d| d.nominal_millivolts);
    let mut i2c_bus: Option<&str> = None;
    for (i, role) in port.roles.iter().enumerate() {
        let number = (i + 1).to_string();
        let Some(pin) = c.pins.iter().find(|p| p.pin == number) else {
            continue;
        };
        let kind = pin.net.as_deref().and_then(|n| kinds.get(n)).copied();
        let supply = match role {
            Vcc5V => Some(5000),
            Vdd3V3 => Some(3300),
            TargetVoltage => mcu_supply,
            _ => None,
        };
        let holds = match (role, kind) {
            (Vcc5V | Vdd3V3, Some(NetKind::Domain(mv))) => Some(mv) == supply,
            (TargetVoltage, Some(NetKind::Domain(_))) => pin.net.as_ref() == Some(&spec.mcu.supply),
            (Ground, Some(NetKind::Ground)) => true,
            (I2cScl, Some(NetKind::I2cScl(bus))) | (I2cSda, Some(NetKind::I2cSda(bus))) => {
                *i2c_bus.get_or_insert(bus) == bus
            }
            (MayFloat(_), None) => true,
            _ => false,
        };
        if holds {
            continue;
        }
        if matches!(role, Vcc5V | Vdd3V3 | TargetVoltage) {
            return Err(E::ConnectorSupplyContradictsStandard {
                id: id(),
                pin: number,
                role: role.name(),
                net: pin.net.clone(),
                required_millivolts: supply,
            });
        }
        return Err(E::ConnectorPinContradictsStandard {
            id: id(),
            pin: number,
            role: role.name(),
            net: pin.net.clone(),
        });
    }
    Ok(())
}
