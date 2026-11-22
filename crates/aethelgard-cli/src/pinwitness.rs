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

/// The hooks: each output's port's `BSRR`, `MODER` and `ODR` writes, once
/// per address, then each ADC input's `CR` writes (its `ADSTART`) and `DR`
/// reads.
pub fn hooks(pins: &Pins) -> Vec<String> {
    let mut at: Vec<u32> = Vec::new();
    for a in pins
        .outputs
        .iter()
        .flat_map(|o| [o.bsrr_at, o.moder_at, o.odr_at])
        .chain(pins.analog.iter().map(|a| a.cr_at))
    {
        if !at.contains(&a) {
            at.push(a);
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
/// when the log could not be read whole. Its level is the one the pin holds
/// when `MODER` first makes it an output, and nothing drives it before that
/// (analog mode, RM0433 Rev 9 p. 539): the last `BSRR` word touching the pin
/// before that write must carry the declared level's bit alone, and no `ODR`
/// write may come before it (adversarial F2: a later word, or an `ODR` write,
/// would set another level). `bsrr` is that word (or the first after
/// `MODER`, when none came before).
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
            let words: Vec<(usize, u32)> = hooks
                .iter()
                .filter_map(|(i, h, _)| match h {
                    Hook::Gpio { address, value, .. }
                        if *address == o.bsrr_at && value & touches != 0 =>
                    {
                        Some((*i, *value))
                    }
                    _ => None,
                })
                .collect();
            let Some(m) = moder else {
                w.bsrr = words.first().map(|x| x.1);
                w.status = "unwitnessed";
                return w;
            };
            let last = words.iter().rev().find(|x| x.0 < m).map(|x| x.1);
            let odr = hooks.iter().any(|(i, h, _)| {
                matches!(h, Hook::Gpio { address, .. } if *address == o.odr_at) && *i < m
            });
            match last {
                None => {
                    w.bsrr = words.first().map(|x| x.1);
                    w.ordered = w.bsrr.map(|_| false);
                    w.status = if w.bsrr.is_some() {
                        "mismatch"
                    } else {
                        "unwitnessed"
                    };
                }
                Some(last) => {
                    w.bsrr = Some(last);
                    w.ordered = Some(true);
                    let alone = last & o.bsrr_bit() != 0 && last & o.bsrr_other() == 0;
                    w.status = if alone && !odr {
                        "witnessed"
                    } else {
                        "mismatch"
                    };
                }
            }
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

/// Each ADC input's witness: a conversion started (`ADSTART` written to its
/// ADC's `CR`) before the firmware's first `DR` read, that read, and the code
/// `DR` holds after the run, which must be the plan's (adversarial F3: at the
/// idle's 0 V the code is also `DR`'s reset value, so the read and the code
/// alone would pass a firmware that never converts).
pub fn analog(
    pins: &Pins,
    hooks: Option<&[(usize, Hook, u64)]>,
    dr: &[Option<u32>],
) -> Vec<AdcWitness> {
    // `ADSTART`, `ADC_CR` bit 2 (RM0433 Rev 9 §25.6.3).
    const ADSTART: u32 = 1 << 2;
    pins.analog
        .iter()
        .zip(dr)
        .enumerate()
        .map(|(k, (a, word))| {
            let reads: Vec<usize> = hooks.map_or_else(Vec::new, |h| {
                h.iter()
                    .filter(|(_, x, _)| matches!(x, Hook::Dr { input, .. } if *input == k))
                    .map(|(i, _, _)| *i)
                    .collect()
            });
            let started = hooks.is_some_and(|h| {
                h.iter().any(|(i, x, _)| {
                    matches!(x, Hook::Gpio { address, value, .. }
                        if *address == a.cr_at && value & ADSTART != 0)
                        && reads.first().is_some_and(|r| i < r)
                })
            });
            // `started` holds a read after the conversion started.
            let (status, code) = match (started, word) {
                (false, _) | (_, None) => ("unwitnessed", None),
                (true, Some(c)) if *c == a.code => ("witnessed", Some(*c)),
                (true, Some(c)) => ("mismatch", Some(*c)),
            };
            AdcWitness {
                net: a.net.clone(),
                status,
                reads: reads.len() as u64,
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
                            crate::verdict::level_name(i.active_high),
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
                        "{}: the output's last BSRR word touching its pin before MODER (or its first, with none before) is {}, {}; the declared level's bit must be that word's alone, and no ODR write may come before MODER",
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
                    "{}: the firmware read its ADC's DR {} times in the boot the judge reads, with no conversion started (ADSTART) before the first, or no DR word after the run",
                    w.net, w.reads
                ),
            )),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D1's plan, as the twin builds it from D1's solved topology (solved by
    /// the CLI's own path, so this file names no specification, `RUL-16`).
    fn plan() -> Pins {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../aethelgard-ir/tests/fixtures/d1-drv8876.json");
        let (_, t, _) = crate::solved(&path).expect("D1 solves");
        aethelgard_twin::pins::plan(&t, &Default::default()).unwrap()
    }

    const BSRR: u32 = 0x5802_0018;
    const MODER: u32 = 0x5802_0000;
    const ODR: u32 = 0x5802_0014;
    const ADC3_CR: u32 = 0x5802_6008;

    fn gpio(i: usize, address: u32, value: u32, before: u32) -> (usize, Hook, u64) {
        (
            i,
            Hook::Gpio {
                address,
                value,
                before,
                count: 0,
            },
            0,
        )
    }

    /// nSLEEP (PA1, low): BR1 alone before MODER makes PA1 an output is
    /// witnessed; after it, or carrying BS1 too, or with no MODER write, it
    /// is not; another pin's BSRR write is never its; a log not read whole
    /// is unreadable.
    #[test]
    fn an_output_is_witnessed_by_its_level_s_bsrr_write_before_moder() {
        let p = plan();
        let status = |h: &[(usize, Hook, u64)]| outputs(&p, Some(h))[0].status;
        let set_mode = gpio(2, MODER, 0x0000_0004, 0);
        assert_eq!(
            status(&[gpio(1, BSRR, 1 << 17, 0), set_mode.clone()]),
            "witnessed"
        );
        assert_eq!(
            status(&[set_mode.clone(), gpio(3, BSRR, 1 << 17, 0)]),
            "mismatch"
        );
        assert_eq!(
            status(&[gpio(1, BSRR, 1 << 1, 0), set_mode.clone()]),
            "mismatch"
        );
        assert_eq!(
            status(&[gpio(1, BSRR, (1 << 17) | (1 << 1), 0), set_mode.clone()]),
            "mismatch"
        );
        assert_eq!(status(&[gpio(1, BSRR, 1 << 17, 0)]), "unwitnessed");
        // Another pin's write first is not nSLEEP's; nor a MODER write that
        // leaves PA1's mode as it was.
        assert_eq!(
            status(&[
                gpio(0, BSRR, 1 << 20, 0),
                gpio(1, BSRR, 1 << 17, 0),
                set_mode.clone()
            ]),
            "witnessed"
        );
        assert_eq!(
            status(&[gpio(1, BSRR, 1 << 17, 0), gpio(2, MODER, 4, 4)]),
            "unwitnessed"
        );
        assert_eq!(outputs(&p, None)[0].status, "unreadable");
        // Adversarial F2: a second word setting the other level before MODER,
        // or an ODR write before it, sets another level: not witnessed.
        assert_eq!(
            status(&[
                gpio(1, BSRR, 1 << 17, 0),
                gpio(2, BSRR, 1 << 1, 0),
                gpio(3, MODER, 4, 0)
            ]),
            "mismatch"
        );
        assert_eq!(
            status(&[
                gpio(1, BSRR, 1 << 17, 0),
                gpio(2, ODR, 2, 0),
                gpio(3, MODER, 4, 0)
            ]),
            "mismatch"
        );
        // The other level first, the declared one last before MODER: the
        // pin holds the declared level when it becomes an output.
        assert_eq!(
            status(&[
                gpio(1, BSRR, 1 << 1, 0),
                gpio(2, BSRR, 1 << 17, 0),
                gpio(3, MODER, 4, 0)
            ]),
            "witnessed"
        );
        // Two words carrying the declared level alone: witnessed, the last.
        let w = outputs(
            &p,
            Some(&[
                gpio(1, BSRR, 1 << 17, 0),
                gpio(2, BSRR, 1 << 17, 0),
                gpio(3, MODER, 4, 0),
            ]),
        );
        assert_eq!(
            (w[0].status, w[0].bsrr, w[0].ordered),
            ("witnessed", Some(1 << 17), Some(true))
        );
    }

    /// nFAULT (PA11, driven high): IDR bit 11 high is witnessed and passes;
    /// low is a mismatch; at its active level (held low, read low) it is
    /// witnessed and named asserted; no word is unreadable.
    #[test]
    fn an_input_is_read_back_and_named_asserted_at_its_active_level() {
        let mut p = plan();
        assert_eq!(inputs(&p, &[Some(1 << 11)])[0].status, "witnessed");
        assert_eq!(inputs(&p, &[Some(0)])[0].status, "mismatch");
        assert_eq!(inputs(&p, &[None])[0].status, "unreadable");
        assert!(findings(&p, &inputs(&p, &[Some(1 << 11)]), &[]).is_empty());
        p.inputs[0].high = false;
        p.inputs[0].held = true;
        let w = inputs(&p, &[Some(0)]);
        assert_eq!(w[0].status, "witnessed");
        let f = findings(&p, &w, &[]);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, GPIO_INPUT_ASSERTED_FINDING);
    }

    /// IPROPI (code 0 at idle): a conversion started (ADSTART) before the
    /// firmware's read, and DR holding the code, is witnessed; another code
    /// is a mismatch; no read, no conversion started before it (adversarial
    /// F3: DR's reset value is the idle's code), or no word, is unwitnessed.
    #[test]
    fn an_adc_input_needs_a_conversion_the_firmware_s_read_and_its_code() {
        let p = plan();
        let start = gpio(0, ADC3_CR, 0x1FC1_0105, 0x1FC1_0101);
        let read = (1, Hook::Dr { input: 0, count: 9 }, 0);
        let seen = [start.clone(), read.clone()];
        assert_eq!(analog(&p, Some(&seen), &[Some(0)])[0].status, "witnessed");
        assert_eq!(analog(&p, Some(&seen), &[Some(7)])[0].status, "mismatch");
        assert_eq!(analog(&p, Some(&seen), &[None])[0].status, "unwitnessed");
        // The read alone, or before the conversion starts: unwitnessed.
        assert_eq!(
            analog(&p, Some(std::slice::from_ref(&read)), &[Some(0)])[0].status,
            "unwitnessed"
        );
        let late = [
            (0, Hook::Dr { input: 0, count: 9 }, 0),
            gpio(1, ADC3_CR, 0x1FC1_0105, 0),
        ];
        assert_eq!(analog(&p, Some(&late), &[Some(0)])[0].status, "unwitnessed");
        assert_eq!(
            analog(&p, Some(std::slice::from_ref(&start)), &[Some(0)])[0].status,
            "unwitnessed"
        );
        assert_eq!(analog(&p, None, &[Some(0)])[0].status, "unwitnessed");
        let other = [start, (1, Hook::Dr { input: 1, count: 9 }, 0)];
        assert_eq!(analog(&p, Some(&other), &[Some(0)])[0].reads, 0);
    }

    /// The stimuli drive each input at its plan's level and set each ADC
    /// input to its plan's value, in the measured monitor forms; the hooks
    /// watch nSLEEP's port's BSRR, MODER and ODR once each, and IPROPI's
    /// ADC's CR and DR.
    #[test]
    fn the_stimuli_and_hooks_are_the_measured_commands() {
        let p = plan();
        assert_eq!(
            stimuli(&p),
            ["gpioPortA OnGPIO 11 true", "adc3 SetVoltage 0 5"]
        );
        let h = hooks(&p);
        assert_eq!(h.len(), 5, "{h:?}");
        for (k, at) in ["0x58020018", "0x58020000", "0x58020014", "0x58026008"]
            .into_iter()
            .enumerate()
        {
            assert!(
                h[k].starts_with(&format!("sysbus AddWatchpointHook {at} DoubleWord Write")),
                "{h:?}"
            );
        }
        assert!(h[4].starts_with("sysbus AddWatchpointHook 0x58026040 DoubleWord Read"));
    }
}
