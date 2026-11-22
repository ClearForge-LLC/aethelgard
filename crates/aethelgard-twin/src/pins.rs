//! `AET-WO-1211c`: what the twin drives and reads of a pin device, from the
//! specification and its solved topology alone (`INV-07`).
//!
//! The twin models no pin device (`device_unmodelled`, `AET-WO-1211` §2.7
//! (a)): it witnesses the MCU's side of each signal only, as `1211` §4
//! measured Renode 1.17.0's reach:
//! - **an output's initial level** only as the `BSRR` write and its order
//!   before `MODER` makes the pin an output (G1: Renode's `IDR` and `ODR`
//!   read one state in any mode, so a pad level is never witnessed);
//! - **an input** is driven by the twin (`OnGPIO`, G3) at a level the
//!   specification states, read back through `IDR` at the boot's end. A held
//!   level survives the firmware's port configuration (`1211c` M4g). No
//!   generated step reads an input, so the read shows that the level reached
//!   the pin, never that the firmware reacted. **A pull is never witnessed**
//!   (G2: an undriven input reads 0 whatever its `PUPDR`), so every declared
//!   input is driven explicitly, at the level its declared pull holds when
//!   the device asserts nothing: that level is the twin's assumption about a
//!   device it does not model (`1211c` M4);
//! - **an ADC input** is set explicitly (`SetVoltage`, A3, A5: an unset
//!   channel reads 0 V, A2, which would be a substituted value, `INV-06`),
//!   and the code the ADC holds after the firmware's read is judged against
//!   [`aethelgard_chaos::record::adc_code`]. The transfer, the source
//!   impedance against the sampling time, the regulator's and calibration's
//!   timing (A0, A1) and ADC2 (A6) are never witnessed. Only `RES` `010`
//!   (12 bits) is measured (`1211c` P4e), so no other resolution is run.
//!
//! What the twin cannot run is refused by name, `PinDeviceNotSimulated`,
//! with what it cannot run.

use std::collections::BTreeMap;

use aethelgard_ir::motor::Level;
use aethelgard_ir::pin_devices::{Direction, McuPull, PinDevice, Transfer};
use aethelgard_ir::{SolvedTopology, Specification};
use aethelgard_svd_hal::cmsis;

use crate::platform::STOCK;
use crate::{TwinError, split_pin};

/// The one resolution the twin's ADC is measured at (`1211c` P4e: `RES`
/// `010` converts at 12 bits in Renode's model).
pub const MEASURED_BITS: u8 = 12;

/// Each ADC a pin device's input may be solved onto, with Renode's name of
/// its stock instance (`stm32h7.repl`: `adcM1S2` at `0x40022000`, ADC1 with
/// ADC2 as its unmodelled slave, L231–239; `adc3` at `0x58026000`, L84–91;
/// `1211` §4).
pub const ADCS: [(&str, &str); 2] = [("ADC1", "adcM1S2"), ("ADC3", "adc3")];

/// A pin device's output signal: its declared initial level, and the
/// registers whose writes show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub net: String,
    pub pin: String,
    /// The pin's number in its port.
    pub bit: u32,
    pub high: bool,
    pub bsrr_at: u32,
    pub moder_at: u32,
}

impl Output {
    /// The `BSRR` bit that writes the declared level: set (`n`) for high,
    /// reset (`n + 16`) for low (RM0433 Rev 9 §11.4.7).
    pub fn bsrr_bit(&self) -> u32 {
        1 << (self.bit + if self.high { 0 } else { 16 })
    }

    /// The `BSRR` bit of the other level.
    pub fn bsrr_other(&self) -> u32 {
        1 << (self.bit + if self.high { 16 } else { 0 })
    }
}

/// A pin device's input signal: the level the twin drives it at for this
/// run, its declared active level, and where its level is read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub net: String,
    pub pin: String,
    /// Renode's name of the pin's port, `gpioPortA`.
    pub renode: &'static str,
    pub bit: u32,
    pub high: bool,
    pub active_high: bool,
    /// A scenario's `pin_held` set the level; otherwise it is the declared
    /// pull's.
    pub held: bool,
    pub idr_at: u32,
}

/// A pin device's ADC input: the value the twin sets, the code it must
/// convert to, and the data register the firmware reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analog {
    pub net: String,
    pub pin: String,
    pub peripheral: String,
    /// Renode's name of the ADC, `adc3`.
    pub renode: &'static str,
    pub channel: u8,
    pub bits: u8,
    pub microvolts: u64,
    /// A scenario's `analog_value` set the value; otherwise it is the
    /// declared idle's.
    pub held: bool,
    pub code: u32,
    pub dr_at: u32,
}

/// Everything the twin drives and reads of the specification's pin devices,
/// in the specification's order. Empty for a specification with none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pins {
    /// Each pin device, `(reference, part)`: none is modelled.
    pub devices: Vec<(String, String)>,
    pub outputs: Vec<Output>,
    pub inputs: Vec<Input>,
    pub analog: Vec<Analog>,
}

impl Pins {
    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }
}

/// What a scenario drives instead of the declared idle: each held input's
/// level (true for high), and each set ADC input's millivolts, by net.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overrides {
    pub held: BTreeMap<String, bool>,
    pub millivolts: BTreeMap<String, u32>,
}

fn not_simulated(d: &PinDevice, why: String) -> TwinError {
    TwinError::PinDeviceNotSimulated {
        reference: d.reference.clone(),
        part: d.part.clone(),
        why,
    }
}

/// The level a declared input's pull holds it at when nothing drives it:
/// the MCU's pull, or the resistor's, high to a domain and low to `GND`.
/// `None` when the specification states none (a pull of `none` and no
/// resistor), or states two that disagree.
fn pulled(s: &aethelgard_ir::pin_devices::GpioSignal) -> Option<bool> {
    let pull = s.pull.as_ref()?;
    let mcu = match pull.mcu {
        Some(McuPull::Up) => Some(true),
        Some(McuPull::Down) => Some(false),
        Some(McuPull::None) | None => None,
    };
    let resistor = pull.resistor.as_ref().map(|r| r.to != "GND");
    match (mcu, resistor) {
        (Some(a), Some(b)) if a != b => None,
        (a, b) => a.or(b),
    }
}

/// The plan for `spec`'s pin devices on `topology`, with `overrides`
/// applied. A held net that is not a declared input is refused
/// `PinHeldIsOutput` when it is a declared output and `PinHeldUnknown`
/// otherwise; a set net that is not a pin device's ADC input,
/// `AnalogValueUnknown`.
pub fn plan(
    spec: &Specification,
    topology: &SolvedTopology,
    overrides: &Overrides,
) -> Result<Pins, TwinError> {
    let f = cmsis::register_facts()?;
    let gpio = f.layout("GPIO")?;
    let adc = f.layout("ADC")?;
    let mut out = Pins::default();
    for d in spec.declared_pin_devices() {
        out.devices.push((d.reference.clone(), d.part.clone()));
        for s in &d.signals {
            let solved = topology.gpio.get(&s.net).ok_or_else(|| {
                not_simulated(d, format!("its signal `{}` has no solved pin", s.net))
            })?;
            let (port, bit) = split_pin(&solved.pin)?;
            let base = f.base(&format!("GPIO{port}"))?;
            match s.direction {
                Direction::Output => {
                    let high = match solved.initial {
                        Some(Level::High) => true,
                        Some(Level::Low) => false,
                        None => {
                            return Err(not_simulated(
                                d,
                                format!("its output `{}` has no solved initial level", s.net),
                            ));
                        }
                    };
                    out.outputs.push(Output {
                        net: s.net.clone(),
                        pin: solved.pin.clone(),
                        bit,
                        high,
                        bsrr_at: base + gpio.offset("BSRR", 0)?,
                        moder_at: base + gpio.offset("MODER", 0)?,
                    });
                }
                Direction::Input => {
                    let active_high = match s.active {
                        Some(Level::High) => true,
                        Some(Level::Low) => false,
                        None => {
                            return Err(not_simulated(
                                d,
                                format!("its input `{}` declares no active level", s.net),
                            ));
                        }
                    };
                    let (high, held) = match overrides.held.get(&s.net) {
                        Some(&h) => (h, true),
                        None => (
                            pulled(s).ok_or_else(|| {
                                not_simulated(
                                    d,
                                    format!(
                                        "its input `{}` states no level its pull holds it at (a pull of `none` and no resistor, or two that disagree), so the twin has no level to drive it at; Renode applies no pull (AET-WO-1211 §4 G2)",
                                        s.net
                                    ),
                                )
                            })?,
                            false,
                        ),
                    };
                    let renode = STOCK
                        .iter()
                        .find(|p| p.0 == format!("GPIO{port}"))
                        .map(|p| p.1)
                        .ok_or_else(|| TwinError::NotInRenode {
                            peripheral: format!("GPIO{port}"),
                            file: crate::platform::STOCK_FILE,
                        })?;
                    out.inputs.push(Input {
                        net: s.net.clone(),
                        pin: solved.pin.clone(),
                        renode,
                        bit,
                        high,
                        active_high,
                        held,
                        idr_at: base + gpio.offset("IDR", 0)?,
                    });
                }
            }
        }
        for a in &d.analog {
            let solved = topology.adc_inputs.get(&a.net).ok_or_else(|| {
                not_simulated(
                    d,
                    format!("its ADC input `{}` has no solved channel", a.net),
                )
            })?;
            let renode = ADCS
                .iter()
                .find(|(p, _)| *p == solved.peripheral)
                .map(|(_, r)| *r)
                .ok_or_else(|| {
                    not_simulated(
                        d,
                        format!(
                            "its ADC input `{}` is on {}, which Renode does not model (AET-WO-1211 §4 A6)",
                            a.net, solved.peripheral
                        ),
                    )
                })?;
            let channel: u8 = solved
                .channel
                .strip_prefix("INP")
                .and_then(|n| n.parse().ok())
                .filter(|&n| n <= 19)
                .ok_or_else(|| {
                    not_simulated(
                        d,
                        format!(
                            "its ADC input `{}` is on channel `{}`, not INP0 to INP19",
                            a.net, solved.channel
                        ),
                    )
                })?;
            let bits = solved.bits.unwrap_or(a.adc_resolution_bits);
            if bits != MEASURED_BITS {
                return Err(not_simulated(
                    d,
                    format!(
                        "its ADC input `{}` converts at {bits} bits; the twin's ADC is measured at {MEASURED_BITS} bits alone (AET-WO-1211c P4e)",
                        a.net
                    ),
                ));
            }
            let (microvolts, held) = match overrides.millivolts.get(&a.net) {
                Some(&mv) => (u64::from(mv) * 1000, true),
                // A current output at a bridge's non-driving idle carries no
                // load current, so it reads 0 V (the bridge's idle is held
                // to a non-driving row at generate, `BridgeIdleNotSafe`).
                None => match (&a.transfer, d.bridges.is_empty()) {
                    (Transfer::CurrentOutput(_), false) => (0, false),
                    (Transfer::CurrentOutput(_), true) => {
                        return Err(not_simulated(
                            d,
                            format!(
                                "its ADC input `{}` is a current output on a device with no bridge, so no idle states its load current and the twin has no value to set it to",
                                a.net
                            ),
                        ));
                    }
                },
            };
            // The firmware converts once and reads `DR` once per input; the
            // read hook shows the read and not its value (measured, `1211c`
            // H-probe), so the code is read back from `DR` at the boot's
            // end, which shows one conversion: one input per ADC.
            if out.analog.iter().any(|o| o.peripheral == solved.peripheral) {
                return Err(not_simulated(
                    d,
                    format!(
                        "its ADC input `{}` shares {} with another input; the twin reads one conversion's code per ADC",
                        a.net, solved.peripheral
                    ),
                ));
            }
            let base = f.base(&solved.peripheral)?;
            out.analog.push(Analog {
                net: a.net.clone(),
                pin: solved.pin.clone(),
                peripheral: solved.peripheral.clone(),
                renode,
                channel,
                bits,
                microvolts,
                held,
                code: aethelgard_chaos::record::adc_code(microvolts, bits),
                dr_at: base + adc.offset("DR", 0)?,
            });
        }
    }
    for net in overrides.held.keys() {
        if !out.inputs.iter().any(|i| &i.net == net) {
            return Err(
                match out.outputs.iter().any(|o| &o.net == net)
                    || topology.gpio.contains_key(net)
                    || topology.assignments.contains_key(net)
                {
                    true => TwinError::PinHeldIsOutput { net: net.clone() },
                    false => TwinError::PinHeldUnknown { net: net.clone() },
                },
            );
        }
    }
    for net in overrides.millivolts.keys() {
        if !out.analog.iter().any(|a| &a.net == net) {
            return Err(TwinError::AnalogValueUnknown { net: net.clone() });
        }
    }
    Ok(out)
}
