//! The device arms (`AET-WO-1319`): one table, [`ARMS`], with a row for each
//! device part the emitter draws, and one interpreter, [`arm`], that draws
//! any of them.
//!
//! A row carries its part, its bus or none, its symbol and footprint, the
//! pin → role map, the passives it requires (each with its value, and for a
//! pin device its rating and page), and an optional stand-in. [`check`]
//! holds the table to what a row may say, at compile time.
//!
//! **A bus device** (I2C or SPI): the row's pins are the whole map, and a
//! symbol pin the row does not list is left open. Its pins are drawn in the
//! row's order, so the straps' and own nets' R and C numbers follow it: a
//! reordered row is a byte change. Its passives follow the part in the row's
//! order.
//!
//! **A pin device** (`AET-WO-1211d`): a driver IC with no bus, read from its
//! parts row (`aethelgard_ir::pin_parts`, `parts.json` `pin_parts`) and its
//! declaration (`pin_devices`), with the passives its datasheet requires.
//! What comes from where:
//! - **the parts row:** every pin's number and kind;
//! - **the declaration:** the supply's domain, each strap's tie or resistor,
//!   each signal's net and pull, each analog output's resistor and reservoir,
//!   each bridge's input and output nets;
//! - **its [`Arm`]:** the symbol, the footprint, the thermal pad and the
//!   passives the datasheet requires but the parts row does not hold
//!   (`AET-WO-1211` Q6), each with its page. A passive whose value the
//!   datasheet does not state is refused by name (`PassiveValueUnstated`),
//!   never invented (ruled S1, 2026-11-20), unless it is the bulk the parts
//!   row leaves unstated (`bulk_unstated`) and the declaration's `bulk`
//!   answers it (`AET-WO-1211f`): that part is drawn in its place.
//!
//! Every numbered pin of a pin device's parts row ends drawn on a net or a
//! rail, or, for an open-drain output the declaration leaves out, on a
//! no-connect. Anything else is refused (`PinDeviceNotDrawable`), so no pin
//! is left floating by an omission here. Each bridge output must reach a
//! declared connector pin (`BridgeOutputNotOnConnector`).
//!
//! Capacitor ratings: the emitter's passives carry a value and a footprint
//! only, so each pin device's capacitor value names the rating the datasheet
//! asks for (`16V`, or `>` the VM domain's maximum where it asks for
//! "VM-rated"). No gate judges a rating (ruled 18:18Z: a finding, folded into
//! `4600` D4's proposal).

use std::collections::{BTreeMap, BTreeSet};

use aethelgard_ir::SolvedTopology;
use aethelgard_ir::pin_devices::{PinDevice, SignalPull, Strap, Transfer};
use aethelgard_ir::pin_parts::{PinKind, PinPart};

use super::{Builder, Conn, EmitError, Part};

/// A pin of the part: a bus device's symbol pin number, or a pin device's
/// row name.
type Pin = &'static str;

/// What a bus device's pin is tied to.
#[derive(Clone, Copy, Debug)]
pub(super) enum Role {
    Gnd,
    /// The device's supply.
    Rail,
    Sda,
    Scl,
    Sck,
    Mosi,
    Miso,
    Cs,
    /// Strapped to the supply through a resistor, on the own net
    /// `<reference>_<name>`.
    Strap(&'static str),
    /// The address strap: the device's address picks one target, in the
    /// parts table's order (`strapped`). With a name the pin reaches its
    /// target through a resistor, on the own net `<reference>_<name>`;
    /// without, it is wired to the target directly.
    Addr(Option<&'static str>, &'static [Role]),
    /// The own net `<reference>_<name>`, closed by the row's
    /// [`Passive::OwnCap`] of the same name.
    Own(&'static str),
}

/// How a row's pins are drawn.
#[derive(Clone, Copy, Debug)]
enum Wiring {
    /// On an I2C bus: each pin's role, in draw order.
    I2c(&'static [(Pin, Role)]),
    /// On an SPI bus (through a crossing's B side, where there is one): each
    /// pin's role, in draw order.
    Spi(&'static [(Pin, Role)]),
    /// A pin device: every pin from its parts row and its declaration.
    Pins {
        /// The symbol's thermal pad, which the parts row lists unnumbered.
        pad: &'static str,
        /// The datasheet's placement rule for its required passives.
        near: &'static str,
    },
}

/// One end of a required passive: a row pin (its drawn connection), or
/// ground.
#[derive(Clone, Copy, Debug)]
enum End {
    Pin(Pin),
    Ground,
}

/// The voltage a capacitor must be rated for.
#[derive(Clone, Copy, Debug)]
enum Rating {
    /// A stated rating, as the datasheet writes it (`16V`).
    Volts(&'static str),
    /// "VM-rated": above the VM domain's maximum, from the specification,
    /// written `>12.6V`, a minimum and not a part's rating (SLVSAR1E §9.1
    /// p. 14: "higher than the operating voltage").
    Vm,
}

/// A passive a pin device's datasheet requires and its parts row does not
/// hold.
#[derive(Clone, Copy, Debug)]
struct Required {
    /// The datasheet's designator, or a name for it where it gives none.
    name: &'static str,
    /// Its value as schematics write it; `None` where the datasheet states
    /// none, which is refused unless the declared `bulk` answers it.
    value: Option<&'static str>,
    rating: Rating,
    /// The dielectric the datasheet names, if any.
    dielectric: Option<&'static str>,
    one: End,
    two: End,
    /// The datasheet's words for it, with the page.
    cite: &'static str,
}

/// A passive a row requires, drawn after the part, in the row's order.
#[derive(Clone, Copy, Debug)]
enum Passive {
    /// A bus device's decoupling capacitor, supply to GND, owned by it.
    Decouple(&'static str),
    /// A bus device's capacitor from its [`Role::Own`] net of this name to
    /// GND (not a decoupler), with its value.
    OwnCap(&'static str, &'static str),
    /// A pin device's, anchored by the pins it joins.
    Required(Required),
}

/// One part's row.
#[derive(Clone, Copy, Debug)]
pub(super) struct Arm {
    pub(super) part: &'static str,
    pub(super) symbol: &'static str,
    /// The symbol's own `Footprint` (a test holds them equal).
    pub(super) footprint: &'static str,
    wiring: Wiring,
    /// In draw order. For a pin device, a refusal comes after every other
    /// passive is drawn.
    passives: &'static [Passive],
    /// The ordering code and the note, where the symbol stands in for the
    /// part.
    stand_in: Option<(&'static str, &'static str)>,
}

/// How close each pin device's required passive sits to its pins: an engine
/// choice, as the power tree's are (`schematic_power`'s `NEAR_NM`): each
/// datasheet asks for "as close as possible" and states no distance.
const NEAR_NM: i64 = 2_000_000;

use Role::{Addr, Cs, Gnd, Miso, Mosi, Own, Rail, Sck, Scl, Sda};

/// SDO to GND, then to VDDIO (BME280 datasheet §6.2).
const BME280: Arm = Arm {
    part: "BME280",
    symbol: "Sensor:BME280",
    footprint: "Package_LGA:Bosch_LGA-8_2.5x2.5mm_P0.65mm_ClockwisePinNumbering",
    wiring: Wiring::I2c(&[
        ("5", Addr(Some("SDO"), &[Gnd, Rail])),
        ("1", Gnd),
        ("7", Gnd),
        ("6", Rail),
        ("8", Rail),
        ("2", Rail),
        ("3", Sda),
        ("4", Scl),
    ]),
    passives: &[Passive::Decouple("100n")],
    stand_in: None,
};

/// ADD0 to GND, V+, SDA, then SCL (TMP117 datasheet Table 7-2).
const TMP117: Arm = Arm {
    part: "TMP117",
    symbol: "Sensor_Temperature:TMP117xxDRV",
    footprint: "Package_SON:WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm",
    wiring: Wiring::I2c(&[
        ("4", Addr(None, &[Gnd, Rail, Sda, Scl])),
        ("1", Scl),
        ("2", Gnd),
        ("5", Rail),
        ("6", Sda),
    ]),
    passives: &[Passive::Decouple("100n")],
    stand_in: None,
};

/// BST-BMP581-DS004-13 Table 28 and Fig. 26 (I2C), pp. 46 and 48: VDDIO and
/// VDD on the supply, each bypassed by 100 nF; the three VSS to GND; SCK is
/// SCL and SDI is SDA; SDO selects the address LSB (low, then high: the
/// parts table's order); CSB to VDDIO ("In I2C mode, the CSB pin must be
/// connected to VDDIO"); INT to GND, allowed "as long as the IRQ pin is not
/// activated" (Table 28 (a)), and the bring-up leaves INT_CONFIG at its
/// reset, `int_en` 0 (p. 52).
const BMP581: Arm = Arm {
    part: "BMP581",
    symbol: "Aethelgard_Sensor:BMP581",
    footprint: "Aethelgard_Sensor:Bosch_LGA-10_2x2mm_P0.5mm",
    wiring: Wiring::I2c(&[
        ("5", Addr(Some("SDO"), &[Gnd, Rail])),
        ("1", Rail),
        ("10", Rail),
        ("3", Gnd),
        ("8", Gnd),
        ("9", Gnd),
        ("2", Scl),
        ("4", Sda),
        ("6", Rail),
        ("7", Gnd),
    ]),
    // One on VDD and one on VDDIO (Fig. 26).
    passives: &[Passive::Decouple("100n"), Passive::Decouple("100n")],
    stand_in: None,
};

/// WP and HOLD strapped high, in that order.
const W25Q64JVXIQ: Arm = Arm {
    part: "W25Q64JVxIQ",
    symbol: "Memory_Flash:W25Q32JVSS",
    footprint: "Package_SO:SOIC-8_5.3x5.3mm_P1.27mm",
    wiring: Wiring::Spi(&[
        ("3", Role::Strap("WP")),
        ("7", Role::Strap("HOLD")),
        ("1", Cs),
        ("2", Miso),
        ("4", Gnd),
        ("5", Mosi),
        ("6", Sck),
        ("8", Rail),
    ]),
    passives: &[Passive::Decouple("100n")],
    // The ordering code in full: package `SS` (SOIC-8 208-mil, the
    // footprint), then `IQ`, whose JEDEC identity the twin reads
    // (`AET-WO-4300c` F11).
    stand_in: Some((
        "W25Q64JVSSIQ",
        "symbol Memory_Flash:W25Q32JVSS stands in for W25Q64JVSSIQ: same family, same SOIC-8 208-mil pinout (AET-WO-1300 D2)",
    )),
};

/// DS-000189 p. 20, 4.1 and 4.3: VDD and VDDIO on the supply, each bypassed;
/// REGOUT's 0.1 uF filter capacitor (C1); FSYNC and RESV pin 20 to GND; RESV
/// pin 19 and the auxiliary I2C master pins left open. SPI: SCLK, SDI, SDO
/// and nCS (`AET-WO-4300d2` D2 (a)).
const ICM20948: Arm = Arm {
    part: "ICM20948",
    symbol: "Sensor_Motion:ICM-20948",
    footprint: "Sensor_Motion:InvenSense_QFN-24_3x3mm_P0.4mm",
    wiring: Wiring::Spi(&[
        ("10", Own("REGOUT")),
        ("8", Rail),
        ("13", Rail),
        ("18", Gnd),
        ("20", Gnd),
        ("11", Gnd),
        ("22", Cs),
        ("23", Sck),
        ("24", Mosi),
        ("9", Miso),
    ]),
    passives: &[
        Passive::OwnCap("REGOUT", "100n"),
        Passive::Decouple("100n"),
        Passive::Decouple("100n"),
    ],
    stand_in: None,
};

/// DS-000347 Table 10 p. 20 and Fig. 7 p. 21 (4-wire SPI): VDD bypassed by
/// 0.1 uF and 2.2 uF, VDDIO by 10 nF; GND and RESV pin 7 to GND; INT2/FSYNC
/// to GND, "Connect to GND if FSYNC not used"; RESV pins 2, 3, 10 and 11 and
/// INT1 left open ("No Connect or Connect to GND"; no interrupt is armed).
/// AP_CS, AP_SCLK, AP_SDI and AP_SDO are CS, SCK, MOSI and MISO.
const ICM42688P: Arm = Arm {
    part: "ICM42688P",
    symbol: "Aethelgard_Sensor:ICM-42688-P",
    footprint: "Aethelgard_Sensor:TDK_LGA-14_3x2.5mm_P0.5mm",
    wiring: Wiring::Spi(&[
        ("5", Rail),
        ("8", Rail),
        ("6", Gnd),
        ("7", Gnd),
        ("9", Gnd),
        ("12", Cs),
        ("13", Sck),
        ("14", Mosi),
        ("1", Miso),
    ]),
    passives: &[
        Passive::Decouple("100n"),
        Passive::Decouple("2u2"),
        Passive::Decouple("10n"),
    ],
    stand_in: None,
};

/// SLVSDS7B (DRV8876, Aug 2019 rev. Nov 2019). The symbol and footprint are
/// our own (`data/aethelgard/`, cited there): KiCad 10.0.7 has neither, and
/// no HTSSOP-16 variant of its matches PWP0016A's (3.3) by (3.3) mask
/// opening (pp. 45–47).
const DRV8876: Arm = Arm {
    part: "DRV8876PWPR",
    symbol: "Aethelgard_Driver:DRV8876PWP",
    footprint: "Aethelgard_Driver:TI_HTSSOP-16-1EP_4.4x5mm_P0.65mm_EP3.4x5mm_Mask3.3x3.3mm",
    wiring: Wiring::Pins {
        pad: "17",
        near: "SLVSDS7B §10.1 p. 32: \"The VM power supply and VCP, CPH, CPL charge pump capacitors should be placed as close to the device as possible\" (2.0 mm, an engine choice: no distance is stated)",
    },
    passives: &[
        Passive::Required(Required {
            name: "CVM1",
            value: Some("100n"),
            rating: Rating::Vm,
            dielectric: None,
            one: End::Pin("VM"),
            two: End::Ground,
            cite: "SLVSDS7B Table 1 p. 10: CVM1, VM to GND, \"0.1-µF, low ESR ceramic capacitor, VM-rated\"; p. 3, VM",
        }),
        Passive::Required(Required {
            name: "CVCP",
            value: Some("100n"),
            rating: Rating::Volts("16V"),
            dielectric: Some("X5R/X7R"),
            one: End::Pin("VCP"),
            two: End::Pin("VM"),
            cite: "SLVSDS7B Table 1 p. 10: CVCP, VCP to VM, \"X5R or X7R, 100-nF, 16-V ceramic capacitor\"; p. 3, VCP",
        }),
        Passive::Required(Required {
            name: "CFLY",
            value: Some("22n"),
            rating: Rating::Vm,
            dielectric: Some("X5R/X7R"),
            one: End::Pin("CPH"),
            two: End::Pin("CPL"),
            cite: "SLVSDS7B Table 1 p. 10: CFLY, CPH to CPL, \"X5R or X7R, 22-nF, VM-rated ceramic capacitor\"; p. 3, CPH",
        }),
        Passive::Required(Required {
            name: "CVM2",
            value: None,
            rating: Rating::Vm,
            dielectric: None,
            one: End::Pin("VM"),
            two: End::Ground,
            cite: "SLVSDS7B Table 1 p. 10: CVM2, VM to GND, \"Bulk Capacitance, VM-rated\", no value; p. 3: \"sufficient Bulk Capacitance rated for VM\"; §9.1 p. 31: \"system level testing is required to determine the appropriately sized bulk capacitor\"",
        }),
    ],
    stand_in: None,
};

/// SLVSAR1E (DRV8833, Jan 2011 rev. Jul 2015). Our own symbol (ruled Q3:
/// AISEN and BISEN `passive`, since p. 10 grounds them directly when current
/// control is not needed), and KiCad 10.0.7's PWP0016C footprint without
/// thermal vias (PWP0016C note 10, p. 27: "Vias are optional"), vendored
/// byte for byte.
const DRV8833: Arm = Arm {
    part: "DRV8833PWPR",
    symbol: "Aethelgard_Driver:DRV8833PWP",
    footprint: "Package_SO:HTSSOP-16-1EP_4.4x5mm_P0.65mm_EP3.4x5mm_Mask2.46x2.31mm",
    wiring: Wiring::Pins {
        pad: "17",
        near: "SLVSAR1E §10.1 p. 15: each capacitor \"as close to the pin(s) as possible\" (2.0 mm, an engine choice: no distance is stated)",
    },
    passives: &[
        Passive::Required(Required {
            name: "CVM",
            value: Some("10u"),
            rating: Rating::Vm,
            dielectric: None,
            one: End::Pin("VM"),
            two: End::Ground,
            cite: "SLVSAR1E p. 3, VM: \"A 10-µF (minimum) ceramic bypass capacitor to GND is recommended\"; §10.1 p. 15: \"a recommended value of 10-μF rated for VM\"",
        }),
        Passive::Required(Required {
            name: "CVCP",
            value: Some("10n"),
            rating: Rating::Volts("16V"),
            dielectric: Some("X7R"),
            one: End::Pin("VCP"),
            two: End::Pin("VM"),
            cite: "SLVSAR1E p. 3, VCP: \"Connect a 0.01-μF, 16-V (minimum) X7R ceramic capacitor to VM\"; §10.1 p. 15",
        }),
        Passive::Required(Required {
            name: "CVINT",
            value: Some("2u2"),
            rating: Rating::Volts("6.3V"),
            dielectric: None,
            one: End::Pin("VINT"),
            two: End::Ground,
            cite: "SLVSAR1E p. 3, VINT: \"Bypass to GND with 2.2-μF, 6.3-V ceramic capacitor\"; §10.1 p. 15",
        }),
    ],
    stand_in: None,
};

/// Every device part the emitter draws, in [`super::KNOWN_PARTS`]'s order (a
/// test holds the two equal).
pub(super) const ARMS: &[Arm] = &[
    BME280,
    TMP117,
    BMP581,
    W25Q64JVXIQ,
    ICM20948,
    ICM42688P,
    DRV8876,
    DRV8833,
];

// The table is refused at compile time where a row says what it may not.
const _: () = match check(ARMS, DECOUPLERS) {
    Ok(()) => {}
    Err(why) => panic!("{}", why),
};

const fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// belt-and-suspenders: each bus part's decoupler count, stated here once
/// more than its row states it (`AET-WO-1320` asks for the restatement), so a row that drops or adds a decoupler is refused when the
/// crate compiles, not only by the byte sweep (`AET-WO-1320`): no other
/// fact of the row predicts it.
const DECOUPLERS: &[(&str, usize)] = &[
    ("BME280", 1),
    ("TMP117", 1),
    ("BMP581", 2),
    ("W25Q64JVxIQ", 1),
    ("ICM20948", 2),
    ("ICM42688P", 3),
];

/// Whether `decouplers` states `n` for `part`.
const fn decouplers_stated(decouplers: &[(&str, usize)], part: &str, n: usize) -> bool {
    let mut i = 0;
    while i < decouplers.len() {
        if same(decouplers[i].0, part) {
            return decouplers[i].1 == n;
        }
        i += 1;
    }
    false
}

/// Whether `rows` has a bus row for `part`.
const fn has_bus_row(rows: &[Arm], part: &str) -> bool {
    let mut r = 0;
    while r < rows.len() {
        if same(rows[r].part, part) && !matches!(rows[r].wiring, Wiring::Pins { .. }) {
            return true;
        }
        r += 1;
    }
    false
}

/// The own net a bus row's pin draws (`<reference>_<name>`), if any.
const fn own_name(role: Role) -> Option<&'static str> {
    match role {
        Role::Strap(name) | Own(name) | Addr(Some(name), _) => Some(name),
        _ => None,
    }
}

/// Whether two of a pin device's passives share a pin end.
const fn share_a_pin(a: &Required, b: &Required) -> bool {
    const fn on(e: End, f: End) -> bool {
        matches!((e, f), (End::Pin(x), End::Pin(y)) if same(x, y))
    }
    on(a.one, b.one) || on(a.one, b.two) || on(a.two, b.one) || on(a.two, b.two)
}

/// Whether `pins` holds `Own(name)` exactly once.
const fn owns_once(pins: &[(Pin, Role)], name: &str) -> bool {
    let mut n = 0;
    let mut i = 0;
    while i < pins.len() {
        if let Own(own) = pins[i].1
            && same(own, name)
        {
            n += 1;
        }
        i += 1;
    }
    n == 1
}

/// What a table may not say, each refused by name:
/// - two rows for one part (one would shadow the other);
/// - an I2C line on an SPI row, an SPI line on an I2C row, an address strap
///   on an SPI row, or two on one row;
/// - an address strap whose targets are other than GND, the supply and the
///   I2C lines;
/// - an own-net capacitor that names no [`Role::Own`] pin of its row, or an
///   own net closed by none, or by two;
/// - a bus row with no decoupler, or an I2C row whose first passive is not
///   the 100 nF decoupler every I2C device had before the table;
/// - a pin device's passive on a bus row, or a bus device's on a pin row;
/// - and since `AET-WO-1320`: two unstated passives on one pin (one `bulk`
///   would answer both); an I2C row with no address strap; a bus row whose
///   decoupler count is not its part's in `counts` ([`DECOUPLERS`]); one
///   own-net name on two pins of a row (a strap and an own net, say); a pin
///   listed twice in a row;
/// - and since `AET-WO-1321`: an entry of `counts` for a part with no
///   bus row.
const fn check(rows: &[Arm], counts: &[(&str, usize)]) -> Result<(), &'static str> {
    let mut r = 0;
    while r < rows.len() {
        let row = &rows[r];
        let mut s = r + 1;
        while s < rows.len() {
            if same(row.part, rows[s].part) {
                return Err("two rows for one part");
            }
            s += 1;
        }
        let (pins, i2c) = match row.wiring {
            Wiring::I2c(pins) => (pins, true),
            Wiring::Spi(pins) => (pins, false),
            Wiring::Pins { .. } => {
                let mut p = 0;
                while p < row.passives.len() {
                    let Passive::Required(a) = row.passives[p] else {
                        return Err("a bus device's passive on a pin device's row");
                    };
                    let mut q = p + 1;
                    while q < row.passives.len() {
                        if let Passive::Required(b) = row.passives[q]
                            && a.value.is_none()
                            && b.value.is_none()
                            && share_a_pin(&a, &b)
                        {
                            return Err("two unstated passives on one pin");
                        }
                        q += 1;
                    }
                    p += 1;
                }
                r += 1;
                continue;
            }
        };
        let mut addresses = 0;
        let mut i = 0;
        while i < pins.len() {
            match pins[i].1 {
                Sda | Scl if !i2c => return Err("an I2C line on an SPI row"),
                Cs | Sck | Mosi | Miso if i2c => return Err("an SPI line on an I2C row"),
                Addr(_, targets) => {
                    if !i2c {
                        return Err("an address strap on an SPI row");
                    }
                    addresses += 1;
                    let mut t = 0;
                    while t < targets.len() {
                        if !matches!(targets[t], Gnd | Rail | Sda | Scl) {
                            return Err(
                                "an address strap to other than GND, the supply or a bus line",
                            );
                        }
                        t += 1;
                    }
                }
                Own(name) => {
                    let (mut closed, mut p) = (0, 0);
                    while p < row.passives.len() {
                        if let Passive::OwnCap(cap, _) = row.passives[p]
                            && same(cap, name)
                        {
                            closed += 1;
                        }
                        p += 1;
                    }
                    if closed != 1 {
                        return Err("an own net not closed by exactly one capacitor of its row");
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if addresses > 1 {
            return Err("two address straps on one row");
        }
        if i2c && addresses == 0 {
            return Err("an I2C row with no address strap");
        }
        let mut i = 0;
        while i < pins.len() {
            let mut j = i + 1;
            while j < pins.len() {
                if same(pins[i].0, pins[j].0) {
                    return Err("a pin listed twice in one row");
                }
                if let (Some(a), Some(b)) = (own_name(pins[i].1), own_name(pins[j].1))
                    && same(a, b)
                {
                    return Err("one own-net name on two pins of a row");
                }
                j += 1;
            }
            i += 1;
        }
        let (mut decouplers, mut p) = (0, 0);
        while p < row.passives.len() {
            match row.passives[p] {
                Passive::Decouple(_) => decouplers += 1,
                Passive::OwnCap(name, _) => {
                    if !owns_once(pins, name) {
                        return Err("an own-net capacitor that names no Own pin of its row");
                    }
                }
                Passive::Required(_) => return Err("a pin device's passive on a bus row"),
            }
            p += 1;
        }
        if decouplers == 0 {
            return Err("a bus row with no decoupler");
        }
        if i2c && !matches!(row.passives[0], Passive::Decouple(v) if same(v, "100n")) {
            return Err("an I2C row whose first passive is not its 100 nF decoupler");
        }
        if !decouplers_stated(counts, row.part, decouplers) {
            return Err("a bus row whose decoupler count is not its part's");
        }
        r += 1;
    }
    let mut e = 0;
    while e < counts.len() {
        if !has_bus_row(rows, counts[e].0) {
            return Err("a DECOUPLERS entry with no bus row");
        }
        e += 1;
    }
    Ok(())
}

/// The bus a bus device is declared on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Bus {
    I2c,
    Spi,
}

/// The row for `part` on a bus of `kind`, or `UnknownPart` naming
/// `reference`: a pin device's part, or one on the other bus, has none.
pub(super) fn on_bus(kind: Bus, reference: &str, part: &str) -> Result<&'static Arm, EmitError> {
    ARMS.iter()
        .find(|a| {
            a.part == part
                && matches!(
                    (kind, a.wiring),
                    (Bus::I2c, Wiring::I2c(_)) | (Bus::Spi, Wiring::Spi(_))
                )
        })
        .ok_or_else(|| EmitError::UnknownPart {
            reference: reference.to_owned(),
            part: part.to_owned(),
            known: super::KNOWN_PARTS,
        })
}

/// The row for pin device part `part`, if the table has one.
fn pin_arm(part: &str) -> Option<&'static Arm> {
    ARMS.iter()
        .find(|a| a.part == part && matches!(a.wiring, Wiring::Pins { .. }))
}

/// Whether the table has a pin device row for `part`.
pub(super) fn draws(part: &str) -> bool {
    pin_arm(part).is_some()
}

fn not_drawable(d: &PinDevice, why: impl Into<String>) -> EmitError {
    EmitError::PinDeviceNotDrawable {
        reference: d.reference.clone(),
        part: d.part.clone(),
        why: why.into(),
    }
}

/// `mv` as volts the way a capacitor's rating is written: `12.6V`.
fn volts(mv: u64) -> String {
    let (whole, frac) = (mv / 1000, mv % 1000);
    if frac == 0 {
        return format!("{whole}V");
    }
    let mut digits = format!("{frac:03}");
    while digits.ends_with('0') {
        digits.pop();
    }
    format!("{whole}.{digits}V")
}

/// Draws each of `devices` (the specification's, read by `schematic`), in
/// document order, by its row. `on_connectors` is every net a declared
/// connector pin carries, where a bridge output must end; `mcu` is the MCU's
/// reference, by whose ADC pins an analog output's reservoir sits.
pub(super) fn pin_devices(
    b: &mut Builder<'_>,
    devices: &[PinDevice],
    on_connectors: &BTreeSet<String>,
    solved: &SolvedTopology,
    table: &aethelgard_ir::PinTable,
    mcu: &str,
) -> Result<(), EmitError> {
    let parts = aethelgard_ir::parts::table().map_err(EmitError::InvalidSpecification)?;
    for d in devices {
        // belt-and-suspenders: `design` refuses a part with no row first
        // (`PinDeviceNotDrawn`), and `validate` a part with no parts row.
        let row = pin_arm(&d.part).ok_or_else(|| EmitError::PinDeviceNotDrawn {
            reference: d.reference.clone(),
            part: d.part.clone(),
        })?;
        let pins = parts
            .pin_part(&d.part)
            .ok_or_else(|| not_drawable(d, "no parts row"))?;
        let around = Around {
            solved,
            table,
            mcu,
            on_connectors,
        };
        arm(
            b,
            row,
            Declared::Pins {
                device: d,
                row: pins,
                around: &around,
            },
        )?;
    }
    Ok(())
}

/// What a row is drawn against: the declared device, with what it needs
/// beside the row.
pub(super) enum Declared<'a> {
    /// A bus device. `line` gives a bus role's connection (for SPI, through
    /// a crossing's B side, falling back to the bus's own nets); `i2c` is
    /// the declared device on an I2C bus, whose address picks its strap.
    Bus {
        reference: &'a str,
        supply: &'a str,
        i2c: Option<&'a aethelgard_ir::I2cDevice>,
        line: &'a dyn Fn(Role) -> Option<Conn>,
    },
    /// A pin device, with its parts row.
    Pins {
        device: &'a PinDevice,
        row: &'a PinPart,
        around: &'a Around<'a>,
    },
}

/// What a pin device is drawn against, beside its rows and declaration.
pub(super) struct Around<'a> {
    solved: &'a SolvedTopology,
    table: &'a aethelgard_ir::PinTable,
    mcu: &'a str,
    on_connectors: &'a BTreeSet<String>,
}

/// A pin device's declared resistors (straps, pulls), each with its two
/// connections, and each analog output's transfer, net and ADC pin
/// position: drawn after the part, before the row's passives.
type Deferred<'a> = (
    Vec<(aethelgard_ir::power::Resistor, Conn, Conn)>,
    Vec<(&'a aethelgard_ir::pin_devices::CurrentOutput, Conn, String)>,
);

/// A passive's two connections and the pads it is anchored by (its own, the
/// device's).
type Ends = (Conn, Conn, Vec<(String, String)>);

/// What the row's passives are drawn against, beside its part.
enum Context<'a> {
    Bus {
        supply: &'a str,
    },
    Pins {
        device: &'a PinDevice,
        near: &'static str,
        mcu: &'a str,
        /// The VM domain's maximum, in mV, for a "VM-rated" capacitor.
        vm: Option<u64>,
    },
}

/// The one interpreter: draws `row` against `declared`. In order: the pins
/// (a bus device's straps and own nets drawn as each is reached, in row
/// order), each passive's two ends, the part, a pin device's declared
/// resistors and analog outputs, then the row's passives in row order.
pub(super) fn arm(b: &mut Builder<'_>, row: &Arm, declared: Declared<'_>) -> Result<(), EmitError> {
    let gnd = b.ground();
    let (reference, conns, deferred, context, pad_of) = match (row.wiring, declared) {
        (
            Wiring::I2c(pins) | Wiring::Spi(pins),
            Declared::Bus {
                reference,
                supply,
                i2c,
                line,
            },
        ) => {
            let conns = bus_pins(b, row.part, pins, reference, supply, i2c, line)?;
            let context = Context::Bus { supply };
            (reference, conns, None, context, None)
        }
        (
            Wiring::Pins { pad, near },
            Declared::Pins {
                device,
                row: parts_row,
                around,
            },
        ) => {
            let (conns, deferred) = pin_pins(b, device, pad, row.symbol, parts_row, around)?;
            let spec = b.spec;
            let vm = device
                .supplies
                .get("VM")
                .and_then(|domain| spec.voltage_domains.iter().find(|v| &v.id == domain))
                .map(|v| {
                    u64::from(v.nominal_millivolts) * (1000 + u64::from(v.tolerance_permille))
                        / 1000
                });
            let context = Context::Pins {
                device,
                near,
                mcu: around.mcu,
                vm,
            };
            let pad_of = Some((device, pad, parts_row));
            (
                device.reference.as_str(),
                conns,
                Some(deferred),
                context,
                pad_of,
            )
        }
        _ => {
            return Err(EmitError::Internal(format!(
                "{}: its row drawn against another kind of device",
                row.part
            )));
        }
    };
    // Each passive's two ends and the pads it is anchored by, resolved
    // against the pins before the part is drawn; a decoupler's, by
    // `Builder::decouple` when it is drawn.
    let mut ends: Vec<Option<Ends>> = Vec::new();
    for p in row.passives {
        ends.push(match (p, pad_of) {
            (Passive::Decouple(_), _) => None,
            // The own net from its own pin's connection, never respelled.
            (Passive::OwnCap(name, _), _) => {
                let (Wiring::I2c(pins) | Wiring::Spi(pins)) = row.wiring else {
                    return Err(EmitError::Internal(format!(
                        "{}: an own net off a bus",
                        row.part
                    )));
                };
                let net = pins
                    .iter()
                    .find(|(_, r)| matches!(r, Own(n) if n == name))
                    .and_then(|(pin, _)| conns.get(*pin).cloned())
                    .ok_or_else(|| {
                        EmitError::Internal(format!("{}: own net {name} not drawn", row.part))
                    })?;
                Some((net, gnd.clone(), Vec::new()))
            }
            (Passive::Required(r), Some((d, pad, parts_row))) => {
                let mut pads = Vec::new();
                let mut end = |e: End, own: &str| -> Result<Conn, EmitError> {
                    match e {
                        End::Ground => Ok(gnd.clone()),
                        End::Pin(name) => {
                            let n = number(d, pad, parts_row, name)?;
                            // Anchored by its first pin end only: CFLY
                            // anchored to both CPH and CPL (adjacent pins,
                            // 0.65 mm apart) left `U2_CPH` unroutable
                            // (measured on D1, `spike/1211d/RESULTS.md`).
                            if pads.is_empty() {
                                pads.push((own.to_owned(), n.clone()));
                            }
                            conns
                                .get(&n)
                                .cloned()
                                .ok_or_else(|| not_drawable(d, format!("`{name}` is not drawn")))
                        }
                    }
                };
                let one = end(r.one, "1")?;
                let two = end(r.two, "2")?;
                Some((one, two, pads))
            }
            (Passive::Required(_), None) => {
                return Err(EmitError::Internal(format!(
                    "{}: a pin device's passive on a bus row",
                    row.part
                )));
            }
        });
    }
    // A pin device's value is its part; a bus device's, its symbol's, or
    // the stand-in's ordering code.
    let value = match (&context, row.stand_in) {
        (Context::Pins { .. }, _) => row.part.to_owned(),
        (Context::Bus { .. }, stand_in) => {
            let value = super::symbol_value(row.symbol)?;
            stand_in.map_or(value, |(code, _)| code.to_owned())
        }
    };
    b.parts.push(Part {
        lib_id: row.symbol,
        reference: reference.to_owned(),
        value,
        footprint: row.footprint.to_owned(),
        conns,
        note: row.stand_in.map(|(_, note)| note.to_owned()),
        owner: None,
        offboard: BTreeMap::new(),
        seat: None,
    });
    // A pin device's declared resistors (straps, pulls), then each analog
    // output's resistor to GND and its reservoir by the MCU's ADC pin: all
    // before the row's own passives, so that a passive refused there
    // (`PassiveValueUnstated`) is refused with every other part drawn.
    if let (Some((resistors, analog)), Context::Pins { mcu, .. }) = (deferred, &context) {
        for (r, one, two) in resistors {
            super::power::resistor(b, &r, one, two)?;
        }
        for (c, net, position) in analog {
            super::power::resistor(b, &c.resistor, net.clone(), gnd.clone())?;
            if let Some(res) = &c.reservoir {
                super::power::reservoir(b, mcu, res, net, gnd.clone(), &position)?;
            }
        }
    }
    // The row's passives, in its order; a pin device's each anchored to the
    // pins it joins.
    let mut bulk_drawn = false;
    for (p, end) in row.passives.iter().zip(ends) {
        match (p, end, &context) {
            (Passive::Decouple(v), _, Context::Bus { supply }) => {
                b.decouple(reference, supply, v)?;
            }
            (Passive::OwnCap(_, v), Some((one, two, _)), _) => {
                b.passive('C', (*v).to_owned(), one, two)?;
            }
            (
                Passive::Required(r),
                Some((one, two, pads)),
                Context::Pins {
                    device: d,
                    near,
                    vm,
                    ..
                },
            ) => {
                let Some(value) = r.value else {
                    // Drawn from the declared `bulk` where it `answers` this
                    // passive; otherwise refused by name.
                    let answered = match (pad_of, &d.bulk) {
                        (Some((_, _, parts_row)), Some(bulk)) => parts_row
                            .bulk_unstated
                            .as_ref()
                            .filter(|u| answers(r, u))
                            .map(|_| (bulk, parts_row.document.as_str())),
                        _ => None,
                    };
                    let Some((bulk, document)) = answered else {
                        return Err(EmitError::PassiveValueUnstated {
                            reference: d.reference.clone(),
                            part: d.part.clone(),
                            passive: r.name.to_owned(),
                            cite: r.cite.to_owned(),
                        });
                    };
                    declared_bulk(b, r.name, document, bulk, one, two)?;
                    b.anchor_last(&d.reference, pads, Some((NEAR_NM, near)), false);
                    bulk_drawn = true;
                    continue;
                };
                let rating = match r.rating {
                    Rating::Volts(v) => v.to_owned(),
                    Rating::Vm => format!(
                        ">{}",
                        volts(vm.ok_or_else(|| not_drawable(d, "its VM names no domain"))?)
                    ),
                };
                let text = match r.dielectric {
                    Some(x) => format!("{value} {rating} {x}"),
                    None => format!("{value} {rating}"),
                };
                b.passive('C', text, one, two)?;
                b.anchor_last(&d.reference, pads, Some((NEAR_NM, near)), false);
            }
            _ => {
                return Err(EmitError::Internal(format!(
                    "{}: a passive its kind of row cannot draw",
                    row.part
                )));
            }
        }
    }
    // belt-and-suspenders: `validate` refuses a `bulk` on a row that states
    // its own (`BulkNotAllowed`); here a declared part is never dropped.
    if let Context::Pins { device: d, .. } = &context
        && d.bulk.is_some()
        && !bulk_drawn
    {
        return Err(not_drawable(
            d,
            "its `bulk` answers no unstated passive of its row",
        ));
    }
    Ok(())
}

/// Whether a declared `bulk` answers `r`: it is the bulk the parts row
/// leaves unstated, from that supply pin to GND (`AET-WO-1211f`).
fn answers(r: &Required, u: &aethelgard_ir::pin_parts::BulkUnstated) -> bool {
    matches!((r.one, r.two), (End::Pin(p), End::Ground) if p == u.supply)
}

/// The declared bulk capacitor in `r`'s place (`AET-WO-1211f`): its
/// reference from the declaration, its symbol, footprint, capacitance and
/// rating from its power-parts row (a `capacitor` or `bulk_capacitor`, as
/// `validate` admits: pin 1, a polarised row's positive, on `r`'s supply
/// end), and its source, verbatim, in the BOM's note, since `document`
/// states no value for `name`.
fn declared_bulk(
    b: &mut Builder<'_>,
    name: &str,
    document: &str,
    bulk: &aethelgard_ir::pin_devices::Bulk,
    one: Conn,
    two: Conn,
) -> Result<(), EmitError> {
    use aethelgard_ir::power_parts::Kind;
    let table =
        aethelgard_ir::power_parts::table().map_err(|e| EmitError::Internal(e.to_string()))?;
    let row = table.get(&bulk.part);
    let (pf, mv) = match row.map(|x| &x.kind) {
        Some(Kind::Capacitor(c)) => (&c.rated_picofarads, &c.rated_millivolts),
        Some(Kind::BulkCapacitor(c)) => (&c.rated_picofarads, &c.rated_millivolts),
        // belt-and-suspenders: `validate` refuses any other part
        // (`PassivePartUnknown`) before any emitter runs.
        _ => {
            return Err(EmitError::PowerNotDrawable {
                reference: bulk.reference.clone(),
                why: format!(
                    "`{}` is not a capacitor or bulk_capacitor row of the power parts table",
                    bulk.part
                ),
            });
        }
    };
    let (Ok(pf), Ok(mv)) = (u64::try_from(pf.value), u64::try_from(mv.value)) else {
        return Err(EmitError::Internal(format!(
            "`{}`: a negative rating",
            bulk.part
        )));
    };
    super::power::two_pin(
        b,
        &bulk.reference,
        &bulk.part,
        format!("{} {} {}", super::power::farads(pf), volts(mv), bulk.part),
        one,
        two,
        Some(format!(
            "{name} declared, {document} states no value: {}",
            bulk.source
        )),
    )
}

/// A bus device's pins, by `pins`' roles in order: a strap's resistor and an
/// own net are drawn (claimed) as each is reached.
fn bus_pins(
    b: &mut Builder<'_>,
    part: &str,
    pins: &[(Pin, Role)],
    reference: &str,
    supply: &str,
    i2c: Option<&aethelgard_ir::I2cDevice>,
    line: &dyn Fn(Role) -> Option<Conn>,
) -> Result<BTreeMap<String, Conn>, EmitError> {
    let rail = b.rail(supply);
    let gnd = b.ground();
    let resolve = |r: Role| -> Result<Conn, EmitError> {
        match r {
            Gnd => Ok(gnd.clone()),
            Rail => Ok(rail.clone()),
            other => line(other).ok_or_else(|| {
                EmitError::Internal(format!("{part}: a row names a line its bus lacks"))
            }),
        }
    };
    let mut conns = BTreeMap::new();
    for (pin, role) in pins {
        let conn = match *role {
            Role::Strap(name) => b.strap(reference, name, rail.clone())?,
            Own(name) => Conn::Net(b.own_net(format!("{reference}_{name}"))?),
            Addr(name, targets) => {
                let d = i2c.ok_or_else(|| {
                    EmitError::Internal(format!("{part}: an address strap off I2C"))
                })?;
                let targets = targets
                    .iter()
                    .map(|t| resolve(*t))
                    .collect::<Result<Vec<_>, _>>()?;
                let target = super::strapped(d, &targets)?;
                match name {
                    Some(n) => b.strap(reference, n, target)?,
                    None => target,
                }
            }
            plain => resolve(plain)?,
        };
        conns.insert((*pin).to_owned(), conn);
    }
    Ok(conns)
}

/// The symbol pin of parts-row pin `name`: its number, or `pad` for the
/// row's unnumbered thermal pad.
fn number(d: &PinDevice, pad: &str, row: &PinPart, name: &str) -> Result<String, EmitError> {
    let p = row
        .pins
        .get(name)
        .ok_or_else(|| not_drawable(d, format!("its row names no pin `{name}`")))?;
    Ok(p.number.map_or_else(|| pad.to_owned(), |n| n.to_string()))
}

/// A pin device's pins, from its parts row and its declaration, and what
/// its declaration draws after the part.
fn pin_pins<'d>(
    b: &mut Builder<'_>,
    d: &'d PinDevice,
    pad: &str,
    symbol: &'static str,
    row: &PinPart,
    around: &Around<'_>,
) -> Result<(BTreeMap<String, Conn>, Deferred<'d>), EmitError> {
    let Around {
        solved,
        table,
        mcu: _,
        on_connectors,
    } = *around;
    let spec = b.spec;
    let gnd = b.ground();
    let mut conns: BTreeMap<String, Conn> = BTreeMap::new();
    let put = |pin: String, c: Conn, conns: &mut BTreeMap<String, Conn>| {
        // A GPIO bridge input is declared twice, as a signal and as the
        // bridge's input, on one net (`AET-WO-1211` Q7). belt-and-suspenders:
        // `validate` binds each device pin to one net, so any other second
        // connection is refused here, never drawn.
        match conns.insert(pin.clone(), c.clone()) {
            Some(old) if old != c => Err(not_drawable(
                d,
                format!("pin {pin} is drawn on two connections"),
            )),
            _ => Ok(()),
        }
    };
    // Supplies and grounds.
    for (pin, domain) in &d.supplies {
        let c = b.rail(domain);
        put(number(d, pad, row, pin)?, c, &mut conns)?;
    }
    for (name, p) in &row.pins {
        if p.kind == PinKind::Ground {
            let g = b.ground();
            put(number(d, pad, row, name)?, g, &mut conns)?;
        }
    }
    // Straps: a tie straight to its rail (a strap through 10 kΩ would select
    // another level: SLVSDS7B p. 5's IMODE bands start at 18.6 kΩ), or the
    // declared resistor from the pin's own net to GND.
    let mut resistors: Vec<(aethelgard_ir::power::Resistor, Conn, Conn)> = Vec::new();
    for (pin, s) in &d.straps {
        let n = number(d, pad, row, pin)?;
        match s {
            Strap::Tie(net) => {
                let c = b.declared(net);
                put(n, c, &mut conns)?;
            }
            Strap::Resistor(r) => {
                let net = b.own_net(format!("{}_{pin}", d.reference))?;
                put(n, Conn::Net(net.clone()), &mut conns)?;
                resistors.push((r.clone(), Conn::Net(net), gnd.clone()));
            }
        }
    }
    // Signals, then analog outputs, then bridges: each on its declared net.
    for s in &d.signals {
        put(
            number(d, pad, row, &s.device_pin)?,
            Conn::Net(s.net.clone()),
            &mut conns,
        )?;
        if let Some(SignalPull {
            resistor: Some(r), ..
        }) = &s.pull
        {
            let to = b.declared(&r.to);
            resistors.push((
                aethelgard_ir::power::Resistor {
                    reference: r.reference.clone(),
                    part: r.part.clone(),
                    ohms: r.ohms,
                },
                to,
                Conn::Net(s.net.clone()),
            ));
        }
    }
    let mut analog = Vec::new();
    for a in &d.analog {
        put(
            number(d, pad, row, &a.device_pin)?,
            Conn::Net(a.net.clone()),
            &mut conns,
        )?;
        let Transfer::CurrentOutput(c) = &a.transfer;
        let pin = solved
            .adc_inputs
            .get(&a.net)
            .ok_or_else(|| EmitError::UnsolvedNet(a.net.clone()))?;
        let position = table
            .pins
            .get(&pin.pin)
            .ok_or_else(|| EmitError::UnknownPin {
                net: a.net.clone(),
                pin: pin.pin.clone(),
            })?
            .position
            .to_string();
        analog.push((c, Conn::Net(a.net.clone()), position));
    }
    for bridge in &d.bridges {
        for (pin, net) in &bridge.inputs {
            put(
                number(d, pad, row, pin)?,
                Conn::Net(net.clone()),
                &mut conns,
            )?;
        }
        for (pin, net) in &bridge.outputs {
            // A motor terminal is a net of its own: on `GND` or a domain it
            // would short the bridge to a rail (`validate` refuses a domain's
            // name; `GND` reached the drawing, adversarial pass #3).
            if net == aethelgard_ir::GND || spec.voltage_domains.iter().any(|v| &v.id == net) {
                return Err(not_drawable(
                    d,
                    format!(
                        "its bridge output `{pin}` names `{net}`, a rail: a motor terminal is a net of its own"
                    ),
                ));
            }
            if !on_connectors.contains(net) {
                return Err(EmitError::BridgeOutputNotOnConnector {
                    reference: d.reference.clone(),
                    pin: pin.clone(),
                    net: net.clone(),
                });
            }
            put(
                number(d, pad, row, pin)?,
                Conn::Net(net.clone()),
                &mut conns,
            )?;
        }
    }
    // The pins only a passive of the row's joins: each on its own net. A
    // `power_in` pin there (the DRV8833's VINT, a regulator the part
    // drives itself) takes its net as a rail, so it carries a `PWR_FLAG`.
    for (name, p) in &row.pins {
        if p.kind == PinKind::Passive {
            let net = b.own_net(format!("{}_{name}", d.reference))?;
            let n = number(d, pad, row, name)?;
            let c = if super::power_in(symbol, &n)? {
                b.rail(&net)
            } else {
                Conn::Net(net)
            };
            put(n, c, &mut conns)?;
        }
    }
    // Every numbered pin is drawn; an open-drain output left undeclared is
    // a no-connect (its pull-up is the declaration's), and anything else is
    // refused rather than left floating.
    for (name, p) in &row.pins {
        let n = number(d, pad, row, name)?;
        if conns.contains_key(&n) {
            continue;
        }
        match p.kind {
            PinKind::OpenDrainOutput { .. } => {
                conns.insert(n, Conn::NoConnect);
            }
            PinKind::CurrentOutput { .. } => {
                return Err(not_drawable(
                    d,
                    format!(
                        "`{name}` is declared neither as an analog output nor tied: SLVSDS7B p. 13 asks for RIPROPI to GND, or IPROPI tied to GND with VREF above GND, and a pin device cannot declare that tie"
                    ),
                ));
            }
            _ => {
                return Err(not_drawable(
                    d,
                    format!(
                        "its {} pin `{name}` is declared nowhere, and would float",
                        p.kind.name()
                    ),
                ));
            }
        }
    }
    Ok((conns, (resistors, analog)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `AET-WO-1319`: the rows are [`super::super::KNOWN_PARTS`], each once and
    /// in its order, and each row's kind is its parts row's: an I2C row for
    /// an I2C part, an SPI row for an SPI part, a pin device's row for a pin
    /// part. A row added for a part the list does not name, or a part named
    /// with no row, fails here by name; `parts_arms_tier1.rs` draws them all.
    #[test]
    fn the_arms_are_the_known_parts() {
        let rows: Vec<&str> = ARMS.iter().map(|a| a.part).collect();
        assert_eq!(rows.join(", "), super::super::KNOWN_PARTS);
        let parts = aethelgard_ir::parts::table().unwrap();
        for a in ARMS {
            match a.wiring {
                Wiring::I2c(_) => assert!(
                    matches!(
                        parts.get(a.part).map(|p| &p.bus),
                        Some(aethelgard_ir::parts::Bus::I2c { .. })
                    ),
                    "{}: an I2C row, and no I2C parts row",
                    a.part
                ),
                Wiring::Spi(_) => assert!(
                    matches!(
                        parts.get(a.part).map(|p| &p.bus),
                        Some(aethelgard_ir::parts::Bus::Spi { .. })
                    ),
                    "{}: an SPI row, and no SPI parts row",
                    a.part
                ),
                Wiring::Pins { .. } => assert!(
                    parts.pin_part(a.part).is_some(),
                    "{}: a pin device's row, and no pin parts row",
                    a.part
                ),
            }
        }
    }

    /// Each bus row's pins are its symbol's (each once is [`check`]'s).
    #[test]
    fn each_bus_rows_pins_are_its_symbols() {
        let mut checked = 0;
        for a in ARMS {
            let (Wiring::I2c(pins) | Wiring::Spi(pins)) = a.wiring else {
                continue;
            };
            let sym = crate::library::symbol(a.symbol).unwrap();
            let numbers: Vec<String> = super::super::pins(&sym)
                .unwrap()
                .into_iter()
                .map(|p| p.number)
                .collect();
            for (pin, _) in pins {
                assert!(numbers.iter().any(|n| n == pin), "{}: pin {pin}", a.part);
            }
            checked += 1;
        }
        assert_eq!(checked, 6);
    }

    /// The own nets a bus row draws (`<reference>_<name>`: straps, address
    /// straps through a resistor, own nets), in row order, are the ones
    /// `aethelgard_ir::names::device_nets` restates for its part, which
    /// `validate` refuses a specification's net by (`AET-WO-1206`). The list
    /// is not derived from the rows (`AET-WO-1319`: a schema-epoch
    /// decision), so it is held to them here.
    #[test]
    fn each_bus_rows_own_nets_are_device_nets() {
        for a in ARMS {
            let (Wiring::I2c(pins) | Wiring::Spi(pins)) = a.wiring else {
                continue;
            };
            let own: Vec<&str> = pins
                .iter()
                .filter_map(|(_, r)| match r {
                    Role::Strap(n) | Own(n) | Addr(Some(n), _) => Some(*n),
                    _ => None,
                })
                .collect();
            assert_eq!(own, aethelgard_ir::names::device_nets(a.part), "{}", a.part);
        }
    }

    const ROW: Arm = ICM20948;

    /// [`check`]'s refusal of `rows`, held to [`DECOUPLERS`]: its reverse
    /// line runs after every row's, so it takes no row's refusal.
    fn refused(rows: &[Arm]) -> &'static str {
        check(rows, DECOUPLERS).unwrap_err()
    }

    /// `row`'s own [`DECOUPLERS`] entry, so that one row is held to its own
    /// count and not to the other parts' rows (`AET-WO-1321`): none for a pin
    /// row or a part the table does not name.
    const fn entry(row: &Arm) -> &'static [(&'static str, usize)] {
        let mut i = 0;
        while i < DECOUPLERS.len() {
            if same(DECOUPLERS[i].0, row.part) {
                return core::slice::from_ref(&DECOUPLERS[i]);
            }
            i += 1;
        }
        &[]
    }

    /// `1211d` §4's hazard: a duplicate row would be shadowed silently.
    #[test]
    fn a_part_with_two_rows_is_refused() {
        assert_eq!(refused(&[BME280, TMP117, BME280]), "two rows for one part");
        assert_eq!(refused(&[DRV8833, DRV8833]), "two rows for one part");
    }

    /// `1211d` §4's hazard: an address role on an SPI row, a compile error
    /// before the table, is refused when the table is built.
    #[test]
    fn an_address_strap_on_an_spi_row_is_refused() {
        let row = Arm {
            wiring: Wiring::Spi(&[
                ("10", Own("REGOUT")),
                ("8", Addr(Some("SDO"), &[Gnd, Rail])),
            ]),
            ..ROW
        };
        assert_eq!(refused(&[row]), "an address strap on an SPI row");
        let row = Arm {
            wiring: Wiring::Spi(&[("10", Own("REGOUT")), ("3", Sda)]),
            ..ROW
        };
        assert_eq!(refused(&[row]), "an I2C line on an SPI row");
        let row = Arm {
            wiring: Wiring::I2c(&[("4", Cs)]),
            ..BME280
        };
        assert_eq!(refused(&[row]), "an SPI line on an I2C row");
        let row = Arm {
            wiring: Wiring::I2c(&[("4", Addr(None, &[Gnd])), ("5", Addr(None, &[Rail]))]),
            ..BME280
        };
        assert_eq!(refused(&[row]), "two address straps on one row");
        let row = Arm {
            wiring: Wiring::I2c(&[("4", Addr(None, &[Gnd, Role::Strap("X")]))]),
            ..BME280
        };
        assert_eq!(
            refused(&[row]),
            "an address strap to other than GND, the supply or a bus line"
        );
    }

    /// `1211d` §4's hazard: the own-net capacitor's net was rebuilt by a
    /// second `format!`; it is now its `Own` pin's connection, and a row
    /// whose capacitor names no `Own` pin of its own (or whose own net no
    /// capacitor closes) is refused.
    #[test]
    fn an_own_net_capacitor_names_its_own_pin() {
        let row = Arm {
            passives: &[
                Passive::OwnCap("REGOUTX", "100n"),
                Passive::Decouple("100n"),
            ],
            ..ROW
        };
        assert_eq!(
            refused(&[row]),
            "an own net not closed by exactly one capacitor of its row"
        );
        let row = Arm {
            passives: &[
                Passive::OwnCap("REGOUT", "100n"),
                Passive::OwnCap("REGOUT", "100n"),
                Passive::Decouple("100n"),
            ],
            ..ROW
        };
        assert_eq!(
            refused(&[row]),
            "an own net not closed by exactly one capacitor of its row"
        );
        let row = Arm {
            passives: &[Passive::Decouple("100n"), Passive::OwnCap("SDO", "100n")],
            ..BME280
        };
        assert_eq!(
            refused(&[row]),
            "an own-net capacitor that names no Own pin of its row"
        );
        let row = Arm {
            wiring: Wiring::Spi(&[("10", Own("REGOUT")), ("11", Own("REGOUT"))]),
            ..ROW
        };
        assert_eq!(refused(&[row]), "one own-net name on two pins of a row");
    }

    /// `1211d` §4's hazard: the I2C 100 nF decoupler, drawn for every I2C
    /// device before the table, is row data, so a row could leave it out.
    #[test]
    fn an_i2c_row_without_its_decoupler_is_refused() {
        let row = Arm {
            passives: &[],
            ..TMP117
        };
        assert_eq!(refused(&[row]), "a bus row with no decoupler");
        let row = Arm {
            passives: &[Passive::Decouple("10n"), Passive::Decouple("100n")],
            ..BMP581
        };
        assert_eq!(
            refused(&[row]),
            "an I2C row whose first passive is not its 100 nF decoupler"
        );
        let row = Arm {
            passives: &[Passive::Decouple("2u2")],
            ..W25Q64JVXIQ
        };
        assert_eq!(
            check(&[row], entry(&row)),
            Ok(()),
            "an SPI row's decouplers are its own"
        );
    }

    /// `AET-WO-1211f`: a declared `bulk` answers only the unstated passive
    /// from the row's `bulk_unstated` supply pin to GND, never another.
    #[test]
    fn a_bulk_answers_only_its_supply_to_ground() {
        let u = aethelgard_ir::pin_parts::BulkUnstated {
            supply: "VM".into(),
            page: String::new(),
        };
        let r = |one, two| Required {
            name: "C",
            value: None,
            rating: Rating::Vm,
            dielectric: None,
            one,
            two,
            cite: "",
        };
        assert!(answers(&r(End::Pin("VM"), End::Ground), &u));
        assert!(!answers(&r(End::Ground, End::Pin("VM")), &u));
        assert!(!answers(&r(End::Pin("VCP"), End::Pin("VM")), &u));
        assert!(!answers(&r(End::Pin("VINT"), End::Ground), &u));
        assert!(!answers(&r(End::Pin("VM"), End::Pin("VCP")), &u));
    }

    /// A pin device's passive on a bus row, and a bus device's on a pin row.
    #[test]
    fn a_passive_of_the_other_kind_is_refused() {
        const MIXED: &[Passive] = &[Passive::Decouple("100n"), DRV8833.passives[0]];
        let row = Arm {
            passives: MIXED,
            ..BME280
        };
        assert_eq!(refused(&[row]), "a pin device's passive on a bus row");
        let row = Arm {
            passives: &[Passive::Decouple("100n")],
            ..DRV8833
        };
        assert_eq!(
            refused(&[row]),
            "a bus device's passive on a pin device's row"
        );
    }

    /// [`check`] over the one row `$row` and its own [`DECOUPLERS`] entry,
    /// evaluated by the compiler, as it is over [`ARMS`] (`AET-WO-1320`).
    macro_rules! at_compile_time {
        ($row:ident) => {{
            const WHY: Result<(), &str> = check(&[$row], entry(&$row));
            WHY
        }};
    }

    const fn unstated(p: Passive) -> Passive {
        match p {
            Passive::Required(r) => Passive::Required(Required { value: None, ..r }),
            _ => panic!("not a pin device's passive"),
        }
    }

    /// `1211f` F2: two unstated passives on one pin, which one `bulk` would
    /// answer twice. Unstated passives on two pins stay allowed.
    #[test]
    fn two_unstated_passives_on_one_pin_are_refused() {
        // DRV8876's CVM2 (VM to GND, no value) twice; with its CVCP (VCP to
        // VM) unstated, meeting at each pair of ends; beside DRV8833's CVINT (VINT to
        // GND) unstated; and beside its own CVM1, stated, as in the row.
        const CVM1: Passive = DRV8876.passives[0];
        const CVM2: Passive = DRV8876.passives[3];
        const CVCP: Passive = unstated(DRV8876.passives[1]);
        const CVINT: Passive = unstated(DRV8833.passives[2]);
        // CVCP moved to CPH: it meets CVCP only at their second ends, VM.
        const CPH_TO_VM: Passive = match CVCP {
            Passive::Required(r) => Passive::Required(Required {
                one: End::Pin("CPH"),
                ..r
            }),
            _ => panic!(),
        };
        const TWICE: Arm = Arm {
            passives: &[CVM2, CVM2],
            ..DRV8876
        };
        const SHARED: Arm = Arm {
            passives: &[CVM2, CVCP],
            ..DRV8876
        };
        const SHARED_TWO: Arm = Arm {
            passives: &[CVCP, CVM2],
            ..DRV8876
        };
        const SHARED_BOTH_TWO: Arm = Arm {
            passives: &[CVCP, CPH_TO_VM],
            ..DRV8876
        };
        const STATED: Arm = Arm {
            passives: &[CVM2, CVM1],
            ..DRV8876
        };
        const APART: Arm = Arm {
            passives: &[CVM2, CVINT],
            ..DRV8833
        };
        const WHY: Result<(), &str> = Err("two unstated passives on one pin");
        assert_eq!(at_compile_time!(TWICE), WHY);
        assert_eq!(at_compile_time!(SHARED), WHY, "VCP to VM shares VM");
        assert_eq!(
            at_compile_time!(SHARED_TWO),
            WHY,
            "VM, the first's second end"
        );
        assert_eq!(
            at_compile_time!(SHARED_BOTH_TWO),
            WHY,
            "VM, both second ends"
        );
        assert_eq!(at_compile_time!(STATED), Ok(()), "CVM1 states its value");
        assert_eq!(at_compile_time!(APART), Ok(()), "VM and VINT are two pins");
    }

    /// `1319` F3: an I2C row with no address strap (TMP117's ADD0 to GND).
    #[test]
    fn an_i2c_row_with_no_address_strap_is_refused() {
        const BAD: Arm = Arm {
            wiring: Wiring::I2c(&[("4", Gnd), ("1", Scl), ("2", Gnd), ("5", Rail), ("6", Sda)]),
            ..TMP117
        };
        assert_eq!(
            at_compile_time!(BAD),
            Err("an I2C row with no address strap")
        );
    }

    /// `1319` F3: the second, third or fourth decoupler, held against the
    /// count [`DECOUPLERS`] states for the part, and a bus part it does not
    /// name.
    #[test]
    fn a_decoupler_dropped_or_added_is_refused() {
        const DROPPED: Arm = Arm {
            passives: &[Passive::Decouple("100n")],
            ..BMP581
        };
        const ADDED: Arm = Arm {
            passives: &[
                Passive::Decouple("100n"),
                Passive::Decouple("2u2"),
                Passive::Decouple("10n"),
                Passive::Decouple("10n"),
            ],
            ..ICM42688P
        };
        const UNNAMED: Arm = Arm {
            part: "BME281",
            ..BME280
        };
        const WHY: Result<(), &str> = Err("a bus row whose decoupler count is not its part's");
        assert_eq!(at_compile_time!(DROPPED), WHY, "dropped");
        assert_eq!(at_compile_time!(ADDED), WHY, "added");
        assert_eq!(at_compile_time!(UNNAMED), WHY, "not named");
    }

    /// `1319` F3: one own-net name on two pins of a row, which `emit` would
    /// refuse as `ReservedName`, blaming the specification: ICM20948's VDD
    /// strapped as REGOUT, W25Q64JVxIQ's HOLD strapped as WP, and BME280's
    /// VDDIO pin 2, five pins after its SDO address strap, strapped as SDO.
    #[test]
    fn one_own_net_name_on_two_pins_is_refused() {
        const STRAP_AND_OWN: Arm = Arm {
            wiring: Wiring::Spi(&[
                ("10", Own("REGOUT")),
                ("8", Role::Strap("REGOUT")),
                ("13", Rail),
                ("18", Gnd),
                ("20", Gnd),
                ("11", Gnd),
                ("22", Cs),
                ("23", Sck),
                ("24", Mosi),
                ("9", Miso),
            ]),
            ..ICM20948
        };
        const TWO_STRAPS: Arm = Arm {
            wiring: Wiring::Spi(&[
                ("3", Role::Strap("WP")),
                ("7", Role::Strap("WP")),
                ("1", Cs),
                ("2", Miso),
                ("4", Gnd),
                ("5", Mosi),
                ("6", Sck),
                ("8", Rail),
            ]),
            ..W25Q64JVXIQ
        };
        const ADDR_AND_STRAP: Arm = Arm {
            wiring: Wiring::I2c(&[
                ("5", Addr(Some("SDO"), &[Gnd, Rail])),
                ("1", Gnd),
                ("7", Gnd),
                ("6", Rail),
                ("8", Rail),
                ("2", Role::Strap("SDO")),
                ("3", Sda),
                ("4", Scl),
            ]),
            ..BME280
        };
        const WHY: Result<(), &str> = Err("one own-net name on two pins of a row");
        assert_eq!(at_compile_time!(STRAP_AND_OWN), WHY);
        assert_eq!(at_compile_time!(TWO_STRAPS), WHY);
        assert_eq!(at_compile_time!(ADDR_AND_STRAP), WHY);
    }

    /// `1319` F3: a pin listed twice in a row: ICM42688P's VDDIO (8) listed
    /// as a second VDD (5), and its RESV (9) as a second GND (6), two rows
    /// apart.
    #[test]
    fn a_pin_listed_twice_is_refused() {
        const BAD: Arm = Arm {
            wiring: Wiring::Spi(&[
                ("5", Rail),
                ("5", Rail),
                ("6", Gnd),
                ("7", Gnd),
                ("9", Gnd),
                ("12", Cs),
                ("13", Sck),
                ("14", Mosi),
                ("1", Miso),
            ]),
            ..ICM42688P
        };
        const APART: Arm = Arm {
            wiring: Wiring::Spi(&[
                ("5", Rail),
                ("8", Rail),
                ("6", Gnd),
                ("7", Gnd),
                ("6", Gnd),
                ("12", Cs),
                ("13", Sck),
                ("14", Mosi),
                ("1", Miso),
            ]),
            ..ICM42688P
        };
        const WHY: Result<(), &str> = Err("a pin listed twice in one row");
        assert_eq!(at_compile_time!(BAD), WHY);
        assert_eq!(at_compile_time!(APART), WHY);
    }

    /// `1320` F2: a [`DECOUPLERS`] entry for a part with no bus row, first
    /// (DRV8876PWPR, whose row, in the table, is a pin device's), in the
    /// middle (BMP582) and last (TMP116). Entries in an order other than the
    /// rows' are accepted.
    #[test]
    fn a_decoupler_entry_with_no_bus_row_is_refused() {
        const FIRST: &[(&str, usize)] = &[(DRV8876.part, 0), ("BME280", 1), ("BMP581", 2)];
        const MIDDLE: &[(&str, usize)] = &[("BME280", 1), ("BMP582", 2), ("BMP581", 2)];
        const LAST: &[(&str, usize)] = &[("BME280", 1), ("BMP581", 2), ("TMP116", 1)];
        const REVERSED: &[(&str, usize)] = &[("BMP581", 2), ("BME280", 1)];
        const WHY: Result<(), &str> = Err("a DECOUPLERS entry with no bus row");
        const ROWS: &[Arm] = &[BME280, BMP581, DRV8876];
        const R_FIRST: Result<(), &str> = check(ROWS, FIRST);
        const R_MIDDLE: Result<(), &str> = check(ROWS, MIDDLE);
        const R_LAST: Result<(), &str> = check(ROWS, LAST);
        const R_REVERSED: Result<(), &str> = check(ROWS, REVERSED);
        assert_eq!(R_FIRST, WHY, "a pin device's row is not a bus row");
        assert_eq!(R_MIDDLE, WHY, "middle");
        assert_eq!(R_LAST, WHY, "last");
        assert_eq!(R_REVERSED, Ok(()), "the entries in the other order");
        // The forward count is the table given, not DECOUPLERS.
        const TWO: Result<(), &str> = check(&[BME280], &[("BME280", 2)]);
        assert_eq!(
            TWO,
            Err("a bus row whose decoupler count is not its part's")
        );
    }
}
