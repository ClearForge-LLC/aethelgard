//! DS-009's port pinouts, cited as facts (ruled D2 (a) on PR #134,
//! provisionally, pending Scotty): the document is held by sha256 and not
//! vendored, and its mark is used nowhere. A port is named by the document,
//! as `"DS-009 TELEM"`.
//!
//! Everything that judges a connector against DS-009 is in this module: the
//! ports, their roles and [`judge`]. Ruling (c), explicit maps only, would be
//! this module, the `standard` field and their refusals deleted.
//!
//! The ports are the ones a vendored part carries (D4) and a work order needs:
//! TELEM and both GPS ports (`AET-WO-4300b`), I2C, and the debug port
//! (Sentinel-02's SWD). The analog power port is `AET-WO-4300d`'s (D6); the
//! SPI and full debug ports have no vendored part; CAN (a GH 4-pin) is left
//! out until a work order declares a CAN bus. A role whose net this schema
//! cannot yet declare (a safety switch, a LED, a buzzer, SWD) refuses every
//! net until the work order that declares it, unless DS-009 lets the pin
//! float. Since `AET-WO-4300b` declares serial ports, `"DS-009 I2C"`,
//! `"DS-009 TELEM"` and `"DS-009 GPS"` can validate; `"DS-009 GPS FULL"`
//! (safety switch, LED, buzzer) and `"DS-009 DEBUG MINI"` (SWD) cannot.
//! [`Role::name`] gives DS-009's signal and voltage columns together for a
//! supply (`"VCC +5V"`). DS-009's "1.5K pullup on autopilot" on its I2C port
//! is not judged.
//!
//! **Serial direction.** DS-009's pin tables are the autopilot's side: "TX
//! (OUT)" is the autopilot's output, and the peripheral crosses the lines
//! ("Cables should be 1:1 to the peripheral and TX / RX as well as CTS / RTS
//! should be crossed on the peripheral side (e.g. a radio modem would have
//! its RX port on pin 2, TX port on pin 3 …)", Telemetry Port, PDF p. 6,
//! printed 5). So a `TX (OUT)` pin carries a serial port's `tx` net and an
//! `RX (IN)` pin its `rx` net, and the two pins of one port are one serial
//! port's, as the two I2C pins are one bus's.

use std::collections::BTreeMap;

use crate::connectors::{Connector, NetKind, Part, Series};
use crate::{IrValidationError as E, SerialRole, Specification};

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
    /// The autopilot's serial output: a serial port's `tx` net. DS-009's words.
    SerialTx(&'static str),
    /// The autopilot's serial input: a serial port's `rx` net. DS-009's words.
    SerialRx(&'static str),
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
            Role::SerialTx(n) | Role::SerialRx(n) | Role::Undeclared(n) | Role::MayFloat(n) => n,
        }
    }

    /// What a pin of this role takes, in words, for a refusal.
    pub fn rule(self) -> &'static str {
        match self {
            Role::Vcc5V | Role::TargetVoltage | Role::Vdd3V3 => {
                "a supply pin takes its voltage domain"
            }
            Role::Ground => "a ground pin takes `GND`",
            Role::I2cScl | Role::I2cSda => {
                "an I2C pin takes its own line of one bus, both pins the same bus's"
            }
            Role::SerialTx(_) | Role::SerialRx(_) => {
                "a `TX (OUT)` pin takes a serial port's `tx` net and an `RX (IN)` pin its `rx` net, both pins the same port's, and neither floats"
            }
            Role::Undeclared(_) => "a role this schema cannot yet declare takes no net",
            Role::MayFloat(_) => {
                "a pin the standard lets float is null until its net can be declared"
            }
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
    /// The one serial role the port's own name gives its serial pins
    /// (`AET-WO-4300b` adversarial F8): `gps` on both GPS ports. `None`
    /// where DS-009 names no device: TELEM is "any serial port".
    pub serial_role: Option<SerialRole>,
}

use Role::*;

pub const PORTS: [Port; 5] = [
    Port {
        name: "DS-009 TELEM",
        series: Series::JstGh,
        roles: &[
            Vcc5V,
            SerialTx("TX (OUT)"),
            SerialRx("RX (IN)"),
            MayFloat("CTS (IN)"),
            MayFloat("RTS (OUT)"),
            Ground,
        ],
        section: "PDF p. 6 (printed 5), Telemetry Port: CTS and RTS float without flow control",
        serial_role: None,
    },
    Port {
        name: "DS-009 GPS",
        series: Series::JstGh,
        roles: &[
            Vcc5V,
            SerialTx("TX (OUT)"),
            SerialRx("RX (IN)"),
            I2cScl,
            I2cSda,
            Ground,
        ],
        section: "PDF p. 8 (printed 7), Basic GPS port",
        serial_role: Some(SerialRole::Gps),
    },
    Port {
        name: "DS-009 GPS FULL",
        series: Series::JstGh,
        roles: &[
            Vcc5V,
            SerialTx("TX (OUT)"),
            SerialRx("RX (IN)"),
            I2cScl,
            I2cSda,
            Undeclared("SAFETY SWITCH IN"),
            Undeclared("SAFETY LED"),
            Vdd3V3,
            Undeclared("BUZZER"),
            Ground,
        ],
        section: "PDF p. 7 (printed 6), Full GPS plus Safety Switch Port",
        serial_role: Some(SerialRole::Gps),
    },
    Port {
        name: "DS-009 I2C",
        series: Series::JstGh,
        roles: &[Vcc5V, I2cScl, I2cSda, Ground],
        section: "PDF p. 9 (printed 8), I2C port",
        serial_role: None,
    },
    Port {
        name: "DS-009 DEBUG MINI",
        series: Series::JstSh,
        roles: &[
            TargetVoltage,
            SerialTx("CONSOLE TX (OUT)"),
            SerialRx("CONSOLE RX (IN)"),
            Undeclared("SWDIO"),
            Undeclared("SWCLK"),
            Ground,
        ],
        section: "PDF p. 12 (printed 11), Debug Mini: JST SH SM06B",
        serial_role: None,
    },
];

/// The row named `name`.
pub fn port(name: &str) -> Option<&'static Port> {
    PORTS.iter().find(|p| p.name == name)
}

/// Refuses `c` where it contradicts the port it names: an unknown port, a
/// part of another series or pin count, a supply pin whose net is not the
/// supply the port names (D5), or another pin whose net is not its role's.
/// The two I2C pins of one port must be one bus's, and its two serial pins
/// one serial port's, of the port's [`Port::serial_role`] where it has one.
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
    let mut serial_port: Option<&str> = None;
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
            (SerialTx(_), Some(NetKind::SerialTx(label)))
            | (SerialRx(_), Some(NetKind::SerialRx(label))) => {
                *serial_port.get_or_insert(label) == label
            }
            (MayFloat(_), None) => true,
            _ => false,
        };
        if holds {
            if let (Some(wants), Some(NetKind::SerialTx(label) | NetKind::SerialRx(label))) =
                (port.serial_role, kind)
                && let Some(declared) = spec
                    .serial_ports
                    .iter()
                    .find(|p| p.label == label)
                    .map(|p| p.role)
                    .filter(|r| *r != wants)
            {
                return Err(E::ConnectorSerialRoleContradictsStandard {
                    id: id(),
                    pin: number,
                    standard: port.name,
                    label: label.to_owned(),
                    declared: declared.name(),
                });
            }
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
            rule: role.rule(),
        });
    }
    Ok(())
}
