//! `AET-WO-1211c`: the twin's witnesses of a pin device's signals, judged
//! from the plan the twin computed from the specification and its topology
//! alone (`aethelgard_twin::pins`, `INV-07`).
//!
//! - **Stimuli**, before the run and again after each rendered reset: each
//!   declared input driven at its plan's level (`OnGPIO`, `1211` §4 G3) and
//!   each ADC input set to its plan's value (`SetVoltage`, A3, A5), the
//!   monitor commands measured in `spike/1211/twin/` and `spike/1211c/kit/`.
//! - **An output** is witnessed by its port's `BSRR` and `MODER` writes
//!   (`anchor::gpio_hook`): the first `BSRR` word touching the pin's set or
//!   reset bit must carry exactly the declared level's and come before the
//!   first `MODER` write making the pin an output. Never a pad level (G1).
//! - **An input** is witnessed by its `IDR` bit read after the run: it must
//!   be the level the twin drove. A read at the declared active level is the
//!   device reporting itself asserted, named `pin_device_input_asserted`.
//! - **An ADC input** is witnessed by the firmware's `DR` read
//!   (`anchor::dr_hook`) and the code `DR` holds after the run, which must be
//!   the plan's code. One input per ADC (`pins`), so that code is the one
//!   conversion's.
//!
//! The findings are named once the boot reached its fixed point, as every
//! other witness's are; replay holds them to kinds 33 to 36
//! (`aethelgard_chaos`).

use aethelgard_chaos::campaign::{
    ADC_MISMATCH_FINDING, ADC_UNWITNESSED_FINDING, GPIO_INPUT_ASSERTED_FINDING,
    GPIO_MISMATCH_FINDING, GPIO_UNWITNESSED_FINDING,
};
use aethelgard_twin::pins::Pins;

use crate::anchor::Hook;
use crate::verdict::Finding;

/// The stimuli, in the plan's order: inputs, then ADC inputs.
pub fn stimuli(pins: &Pins) -> Vec<String> {
    let inputs = pins
        .inputs
        .iter()
        .map(|i| format!("{} OnGPIO {} {}", i.renode, i.bit, i.high));
    let analog = pins
        .analog
        .iter()
        .map(|a| format!("{} SetVoltage {} {}", a.renode, a.microvolts, a.channel));
    inputs.chain(analog).collect()
}

/// The hooks: each output's port's `BSRR` and `MODER`, once per address, then
/// each ADC input's `DR`.
pub fn hooks(pins: &Pins) -> Vec<String> {
    let mut at: Vec<u32> = Vec::new();
    for o in &pins.outputs {
        for a in [o.bsrr_at, o.moder_at] {
            if !at.contains(&a) {
                at.push(a);
            }
        }
    }
    at.into_iter()
        .map(crate::anchor::gpio_hook)
        .chain(
            pins.analog
                .iter()
                .enumerate()
                .map(|(k, a)| crate::anchor::dr_hook(k, a.dr_at)),
        )
        .collect()
}

/// What the twin showed of one GPIO signal (kind 34).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpioWitness {
    pub net: String,
    pub output: bool,
    /// One of [`aethelgard_chaos::record::GPIO_STATUSES`].
    pub status: &'static str,
    pub bsrr: Option<u32>,
    pub ordered: Option<bool>,
    pub level: Option<u8>,
}

/// What the twin showed of one ADC input (kind 36).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdcWitness {
    pub net: String,
    /// One of [`aethelgard_chaos::record::ADC_STATUSES`].
    pub status: &'static str,
    pub reads: u64,
    pub code: Option<u32>,
}

/// Each output's witness from the judged boot's hook lines, in order; `None`
/// when the log could not be read whole.
pub fn outputs(pins: &Pins, hooks: Option<&[(usize, Hook, u64)]>) -> Vec<GpioWitness> {
    pins.outputs
        .iter()
        .map(|o| {
            let mut w = GpioWitness {
                net: o.net.clone(),
                output: true,
                status: "unreadable",
                bsrr: None,
                ordered: None,
                level: None,
            };
            let Some(hooks) = hooks else {
                return w;
            };
            let touches = o.bsrr_bit() | o.bsrr_other();
            let field = |word: u32| (word >> (2 * o.bit)) & 0b11;
            let bsrr = hooks.iter().find_map(|(i, h, _)| match h {
                Hook::Gpio { address, value, .. }
                    if *address == o.bsrr_at && value & touches != 0 =>
                {
                    Some((*i, *value))
                }
                _ => None,
            });
            let moder = hooks.iter().find_map(|(i, h, _)| match h {
                Hook::Gpio {
                    address,
                    value,
                    before,
                    ..
                } if *address == o.moder_at && field(*value) == 0b01 && field(*before) != 0b01 => {
                    Some(*i)
                }
                _ => None,
            });
            w.bsrr = bsrr.map(|b| b.1);
            w.status = match (bsrr, moder) {
                (Some((b, word)), Some(m)) => {
                    w.ordered = Some(b < m);
                    if b < m && word & o.bsrr_bit() != 0 && word & o.bsrr_other() == 0 {
                        "witnessed"
                    } else {
                        "mismatch"
                    }
                }
                _ => "unwitnessed",
            };
            w
        })
        .collect()
}

/// Each input's witness from its `IDR` word read after the run, in order;
/// `None` for a read that gave no word.
pub fn inputs(pins: &Pins, idr: &[Option<u32>]) -> Vec<GpioWitness> {
    pins.inputs
        .iter()
        .zip(idr)
        .map(|(i, word)| {
            let level = word.map(|w| u8::from(w >> i.bit & 1 == 1));
            GpioWitness {
                net: i.net.clone(),
                output: false,
                status: match level {
                    None => "unreadable",
                    Some(l) if (l == 1) == i.high => "witnessed",
                    Some(_) => "mismatch",
                },
                bsrr: None,
                ordered: None,
                level,
            }
        })
        .collect()
}

/// Each ADC input's witness: the firmware's `DR` reads the hooks showed, and
/// the word `DR` held after the run.
pub fn analog(
    pins: &Pins,
    hooks: Option<&[(usize, Hook, u64)]>,
    dr: &[Option<u32>],
) -> Vec<AdcWitness> {
    pins.analog
        .iter()
        .zip(dr)
        .enumerate()
        .map(|(k, (a, word))| {
            let reads = hooks.map_or(0, |h| {
                h.iter()
                    .filter(|(_, x, _)| matches!(x, Hook::Dr { input, .. } if *input == k))
                    .count() as u64
            });
            let (status, code) = match (reads, word) {
                (0, _) | (_, None) => ("unwitnessed", None),
                (_, Some(c)) if *c == a.code => ("witnessed", Some(*c)),
                (_, Some(c)) => ("mismatch", Some(*c)),
            };
            AdcWitness {
                net: a.net.clone(),
                status,
                reads,
                code,
            }
        })
        .collect()
}

/// The findings of every witness, once the boot reached its fixed point.
pub fn findings(pins: &Pins, gpio: &[GpioWitness], adc: &[AdcWitness]) -> Vec<Finding> {
    let mut out = Vec::new();
    for w in gpio {
        let what = if w.output { "output" } else { "input" };
        match w.status {
            "witnessed" if !w.output => {
                let i = pins.inputs.iter().find(|i| i.net == w.net);
                if let Some(i) = i.filter(|i| i.high == i.active_high) {
                    out.push(Finding::plain(
                        GPIO_INPUT_ASSERTED_FINDING,
                        format!(
                            "{}: the input on {} reads its declared active level ({}), {}: the device reports itself asserted",
                            i.net,
                            i.pin,
                            if i.active_high { "high" } else { "low" },
                            if i.held {
                                "as the scenario holds it"
                            } else {
                                "where its declared pull holds it"
                            }
                        ),
                    ));
                }
            }
            "witnessed" => {}
            "mismatch" => out.push(Finding::plain(
                GPIO_MISMATCH_FINDING,
                match w.output {
                    true => format!(
                        "{}: the output's first BSRR word touching its pin is {}, {}; the declared level's bit must be written alone, before MODER makes the pin an output",
                        w.net,
                        w.bsrr.map_or("none".to_owned(), |b| format!("{b:#010x}")),
                        match w.ordered {
                            Some(true) => "before MODER",
                            Some(false) => "after MODER",
                            None => "with no MODER write",
                        }
                    ),
                    false => format!(
                        "{}: the input reads {} after the run, not the level the twin drove",
                        w.net,
                        w.level.map_or("nothing".to_owned(), |l| l.to_string())
                    ),
                },
            )),
            _ => out.push(Finding::plain(
                GPIO_UNWITNESSED_FINDING,
                format!(
                    "{}: the {what} was not witnessed ({}): {}",
                    w.net,
                    w.status,
                    match w.output {
                        true => "no BSRR word touching its pin, or no MODER write making it an output, in the boot the judge reads",
                        false => "its IDR read gave no word",
                    }
                ),
            )),
        }
    }
    for w in adc {
        match w.status {
            "witnessed" => {}
            "mismatch" => {
                let want = pins.analog.iter().find(|a| a.net == w.net).map(|a| a.code);
                out.push(Finding::plain(
                    ADC_MISMATCH_FINDING,
                    format!(
                        "{}: DR holds {} after the firmware's read, not the code {} the value the twin set converts to",
                        w.net,
                        w.code.map_or("nothing".to_owned(), |c| c.to_string()),
                        want.map_or("?".to_owned(), |c| c.to_string())
                    ),
                ));
            }
            _ => out.push(Finding::plain(
                ADC_UNWITNESSED_FINDING,
                format!(
                    "{}: the firmware read its ADC's DR {} times in the boot the judge reads",
                    w.net, w.reads
                ),
            )),
        }
    }
    out
}
