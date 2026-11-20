//! The HSE bring-up as the twin judges it (`AET-WO-4240b` §1.6): on a board
//! whose solved clock has an HSE (`topology.clock.hse`), the RCC's and the
//! flash controller's accesses must be exactly the H1 bring-up's, in order.
//!
//! Every expected word comes from the [`SolvedTopology`], RM0433 Rev 9 and
//! ST's header, never from the generator (`INV-07`, `RUL-20`): the `RCC_CR`
//! words are RM0433's reset value (§8.7.2, p. 383) with the header's bit
//! positions set one by one; the PLL words and `FLASH_ACR` are the
//! topology's (`clock.hse`, which `aethelgard_ir::clock::judge_tree` and
//! `aethelgard_ir::vos::flash_at_reset` judged); `RCC_CFGR` is its reset value
//! 0 (§8.7.7, p. 391) with the header's `RCC_CFGR_SW_PLL1`.
//!
//! A `null` board has no [`ClockRule`], and nothing here touches its
//! verdict.
//!
//! What the twin cannot see of the sequence is [`HSE_RESIDUAL`].

use aethelgard_ir::SolvedTopology;
use aethelgard_svd_hal::cmsis;

use crate::TwinError;
use crate::boot::FLASH_RENODE;
use crate::serial::{Access, PWR_RENODE, RCC_RENODE, width_name};

/// `RCC_CR`'s reset value (RM0433 Rev 9 §8.7.2, p. 383): `HSION` alone. The
/// read-only status bits Renode also returns at reset (`HSIRDY`, `HSIDIVF`,
/// measured `0x25`) are never written.
pub const RCC_CR_RESET: u32 = 0x0000_0001;
/// `RCC_CFGR`'s reset value (RM0433 Rev 9 §8.7.7, p. 391).
pub const RCC_CFGR_RESET: u32 = 0x0000_0000;

/// What the twin cannot show of the HSE bring-up (`AET-WO-4240b` §1.6), as a
/// generated HSE board's verdict states it: Renode 1.17.0's `STM32H7_RCC`
/// measured (`spike/4240b/renode/results.md`) and read at source
/// (`AET-WO-4240` FEEDBACK §6).
pub const HSE_RESIDUAL: &str = "HSE (AET-WO-4240b): the twin's RCC reads HSERDY and PLL1RDY as their ON bits at once, so no wait times out and the twin under-counts the boot against silicon by up to the crystal's start-up time plus 150 us of PLL lock; SWS follows SW at once; HSEBYP and RCC_PLLCFGR are tags, written and never stored, so crystal and bypass run the same and PLL1RGE, PLL1VCOSEL and the DIVxEN bits act on nothing; HSECSSON is stored with no clock security system behind it, so no HSE failure is detected and no NMI is taken; every kernel select register reads 0 whatever is written, so only the access rule sees a select written away from reset; FLASH_ACR stores the word with no wait state behind it";

/// One RCC word write of the bring-up: its register, offset, the whole word,
/// and what it does.
/// A register field from the header, as `cmsis::RegisterFacts::fields_of`
/// gives it: (name, position, mask).
pub type Field = (String, u32, u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub register: String,
    pub offset: u32,
    pub word: u32,
    pub what: &'static str,
}

/// The rule a generated HSE board's RCC and flash controller accesses are
/// held to (`AET-WO-4240b` §1.6, the PM's contract):
/// 1. external only: `RCC_CR` ← reset | `HSEBYP`;
/// 2. `RCC_CR` ← … | `HSEON`;
/// 3. a 32-bit read of `RCC_CR` with `HSERDY` 1;
/// 4. `RCC_CR` ← … | `HSECSSON` (the header's `CSSHSEON`);
/// 5. `RCC_PLLCKSELR`, `RCC_PLLCFGR`, `RCC_PLL1DIVR` ← the topology's words;
/// 6. `RCC_CR` ← … | `PLL1ON`;
/// 7. a 32-bit read of `RCC_CR` with `PLL1RDY` 1;
/// 8. `FLASH_ACR` ← the topology's `flash_acr`, then one 32-bit read of it;
/// 9. `RCC_CFGR` ← `SW` = PLL1;
/// 10. a 32-bit read of `RCC_CFGR` with `SWS` = PLL1;
///
/// all after `PWR_D3CR`'s write (`AET-WO-4240v`), and every RCC enable
/// register write after step 10. No kernel select register is written: the
/// topology's SPI kernels are `pll1_q` and `pclk`, whose codes are the reset
/// `000` (`AET-WO-4240` D6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockRule {
    /// Every RCC register: its name and offset, from the header.
    pub registers: Vec<(String, u32)>,
    /// The RCC word writes, in order (steps 1, 2, 4, 5, 6 and 9).
    pub steps: Vec<Step>,
    /// The offsets of the registers [`Self::steps`] write: `CR`, `PLLCKSELR`,
    /// `PLLCFGR`, `PLL1DIVR`, `CFGR`.
    pub clock_registers: Vec<u32>,
    pub cr: u32,
    pub cfgr: u32,
    pub hserdy: u32,
    pub pll1rdy: u32,
    /// `RCC_CFGR_SWS_Msk` and `RCC_CFGR_SWS_PLL1`.
    pub sws_mask: u32,
    pub sws_pll1: u32,
    /// The kernel select registers no write may touch on an HSE board, by
    /// name and offset (`D2CCIP1R`, `D3CCIPR`), with each one's fields.
    pub selects: Vec<(String, u32, Vec<Field>)>,
    /// The RCC enable registers (`*ENR`, not `*LPENR`), by name and offset.
    pub enables: Vec<(String, u32)>,
    /// `FLASH_ACR`'s offset and the topology's word.
    pub acr: u32,
    pub acr_word: u32,
    /// `PWR_D3CR`'s offset.
    pub d3cr: u32,
}

impl ClockRule {
    fn index(&self, what: &str) -> usize {
        self.steps
            .iter()
            .position(|s| s.what == what)
            .expect("every rule has each of its own steps")
    }

    fn name(&self, offset: u32) -> String {
        self.registers
            .iter()
            .find(|r| r.1 == offset && !r.0.starts_with("RESERVED"))
            .map_or_else(|| format!("{offset:#X}"), |r| r.0.clone())
    }
}

/// The rule of `topology`'s generated firmware, `None` for a board with no
/// HSE (`clock.hse` absent). A solved SPI controller whose kernel select
/// word is not the reset `0` on an HSE board is refused: the contract writes
/// no select there.
pub fn clock_rule(topology: &SolvedTopology) -> Result<Option<ClockRule>, TwinError> {
    let Some(hse) = &topology.clock.hse else {
        return Ok(None);
    };
    let f = cmsis::register_facts()?;
    let rcc = f.layout("RCC")?;
    let bit = |name: &str| -> Result<u32, TwinError> { Ok(1 << f.position(name)?) };
    let reg = |name: &str| -> Result<u32, TwinError> { Ok(rcc.offset(name, 0)?) };
    let cr = reg("CR")?;
    let cfgr = reg("CFGR")?;
    let step = |register: &str, word: u32, what: &'static str| -> Result<Step, TwinError> {
        Ok(Step {
            register: register.to_owned(),
            offset: reg(register)?,
            word,
            what,
        })
    };
    let external = matches!(hse.declared, aethelgard_ir::clock::Hse::External(_));
    let mut steps = Vec::new();
    let mut word = RCC_CR_RESET;
    if external {
        word |= bit("RCC_CR_HSEBYP_Pos")?;
        steps.push(step("CR", word, "HSEBYP")?);
    }
    word |= bit("RCC_CR_HSEON_Pos")?;
    steps.push(step("CR", word, "HSEON")?);
    word |= bit("RCC_CR_CSSHSEON_Pos")?;
    steps.push(step("CR", word, "HSECSSON")?);
    steps.push(step("PLLCKSELR", hse.pllckselr, "PLLCKSELR")?);
    steps.push(step("PLLCFGR", hse.pllcfgr, "PLLCFGR")?);
    steps.push(step("PLL1DIVR", hse.pll1divr, "PLL1DIVR")?);
    word |= bit("RCC_CR_PLL1ON_Pos")?;
    steps.push(step("CR", word, "PLL1ON")?);
    steps.push(step(
        "CFGR",
        RCC_CFGR_RESET | f.value("RCC_CFGR_SW_PLL1")?,
        "SW = PLL1",
    )?);
    for p in topology.spi_controllers.keys() {
        let words =
            aethelgard_ir::silicon::spi_words(&topology.spi_controllers, p).ok_or_else(|| {
                TwinError::InvalidTopology(
                    aethelgard_ir::IrValidationError::SpiControllerMismatch { id: p.clone() },
                )
            })?;
        if words.kernel_select.1 != 0 {
            return Err(TwinError::InvalidTopology(
                aethelgard_ir::IrValidationError::SpiControllerMismatch { id: p.clone() },
            ));
        }
    }
    let selects = ["D2CCIP1R", "D3CCIPR"]
        .into_iter()
        .map(|r| -> Result<_, TwinError> {
            Ok((r.to_owned(), reg(r)?, f.fields_of(&format!("RCC_{r}"))?))
        })
        .collect::<Result<_, _>>()?;
    let (sws_pos, sws_width) = f.field("RCC_CFGR_SWS")?;
    Ok(Some(ClockRule {
        registers: rcc
            .fields
            .iter()
            .flat_map(|r| (0..r.words).map(move |k| (r.name.clone(), r.offset + 4 * k)))
            .collect(),
        clock_registers: {
            let mut v: Vec<u32> = steps.iter().map(|s| s.offset).collect();
            v.sort_unstable();
            v.dedup();
            v
        },
        steps,
        cr,
        cfgr,
        hserdy: bit("RCC_CR_HSERDY_Pos")?,
        pll1rdy: bit("RCC_CR_PLL1RDY_Pos")?,
        sws_mask: ((1u32 << sws_width) - 1) << sws_pos,
        sws_pll1: f.value("RCC_CFGR_SWS_PLL1")?,
        selects,
        enables: rcc
            .fields
            .iter()
            .filter(|r| r.name.ends_with("ENR") && !r.name.ends_with("LPENR"))
            .map(|r| (r.name.clone(), r.offset))
            .collect(),
        acr: f.layout("FLASH")?.offset("ACR", 0)?,
        acr_word: hse.flash_acr,
        d3cr: f.layout("PWR")?.offset("D3CR", 0)?,
    }))
}

/// The offsets of the RCC words [`judge_clock`] judges on an HSE board, for
/// [`crate::serial::judge_rcc`] to leave to it: the clock registers and the
/// kernel select registers.
pub fn owned_offsets(rule: &ClockRule) -> Vec<u32> {
    let mut v = rule.clock_registers.clone();
    v.extend(rule.selects.iter().map(|s| s.1));
    v
}

/// What the RCC's, the flash controller's and the PWR's accesses in `boots`
/// (each boot's, in log order, the last the boot the reads judge) show
/// against `rule`, each once, in the order first seen. Named:
/// - a write to a clock register that is not a word write at its offset
///   (`WriteByte to RCC.CR`);
/// - a word of [`ClockRule::steps`] written after a later one, named at the
///   earlier word (`RCC.PLLCFGR 0x30004 (PLLCFGR) written after RCC.PLL1DIVR
///   0x103064F (PLL1DIVR)`), so a word never written is only missing; or a
///   word written a second time
///   (`RCC.CR 0x10001 written twice in a boot`);
/// - a clock register written with another word where the next step writes
///   it (`RCC.CR 0x30001 written, not the solved word 0x10001`), or with a
///   word the bring-up never writes;
/// - a step written before `PWR_D3CR`;
/// - `HSECSSON` written before a 32-bit read of `RCC_CR` showed `HSERDY` 1
///   after `HSEON`;
/// - `SW` written before a 32-bit read of `RCC_CR` showed `PLL1RDY` 1 after
///   `PLL1ON`, or before `FLASH_ACR` was written and read back;
/// - any write to a kernel select register, with the fields it moves away
///   from reset (`SPI123SEL 100`);
/// - an RCC enable register written before a 32-bit read of `RCC_CFGR`
///   showed `SWS` = PLL1 after `SW`;
/// - `FLASH_ACR` accessed other than by a word at its offset, written twice,
///   with another word, before the `PLL1RDY` read or after `SW`, read before
///   its write, read twice, or read after `SW`;
/// - in the boot the reads judge, a step never written, no `SWS` read after
///   `SW`, and `FLASH_ACR` never written or never read back.
///
/// A boot cut by a reset is judged as far as it went: an earlier boot may
/// stop anywhere in the sequence.
pub fn judge_clock(rule: &ClockRule, boots: &[Vec<Access>]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut note = |x: String| {
        if !out.contains(&x) {
            out.push(x);
        }
    };
    let n = rule.steps.len();
    let (hseon, cssh, pll1on, sw) = (
        rule.index("HSEON"),
        rule.index("HSECSSON"),
        rule.index("PLL1ON"),
        rule.index("SW = PLL1"),
    );
    let said = |s: &Step| format!("RCC.{} {:#X} ({})", s.register, s.word, s.what);
    for (k, boot) in boots.iter().enumerate() {
        let last = k + 1 == boots.len();
        let mut done = vec![false; n];
        let mut d3cr = false;
        let (mut hse_ready, mut pll_ready, mut sws) = (false, false, false);
        let (mut acr_written, mut acr_read) = (0usize, 0usize);
        for a in boot {
            let value = u32::try_from(a.value).unwrap_or(u32::MAX);
            if a.source == PWR_RENODE {
                d3cr |= a.write && a.bits == 32 && a.offset == rule.d3cr;
                continue;
            }
            if a.source == FLASH_RENODE {
                if a.offset & !3 != rule.acr {
                    continue;
                }
                let op = if a.write { "Write" } else { "Read" };
                if a.bits != 32 || a.offset != rule.acr {
                    note(format!(
                        "{op}{} at FLASH_ACR {:#X}",
                        width_name(a.bits),
                        a.offset
                    ));
                    continue;
                }
                if done[sw] {
                    note(format!(
                        "FLASH_ACR {} after {}",
                        if a.write { "written" } else { "read" },
                        said(&rule.steps[sw])
                    ));
                }
                if a.write {
                    acr_written += 1;
                    if acr_written > 1 {
                        note("FLASH_ACR written twice in a boot".to_owned());
                        continue;
                    }
                    if value != rule.acr_word {
                        note(format!(
                            "FLASH_ACR {value:#X} written, not the solved word {:#X}",
                            rule.acr_word
                        ));
                    }
                    if !pll_ready {
                        note(
                            "FLASH_ACR written before a read of RCC.CR showed PLL1RDY 1".to_owned(),
                        );
                    }
                } else if acr_written == 0 {
                    note("FLASH_ACR read before it was written".to_owned());
                } else {
                    acr_read += 1;
                    if acr_read > 1 {
                        note("FLASH_ACR read back twice in a boot".to_owned());
                    }
                }
                continue;
            }
            if a.source != RCC_RENODE {
                continue;
            }
            if !a.write {
                if a.bits == 32 && a.offset == rule.cr {
                    if done[hseon] && !done[cssh] && value & rule.hserdy != 0 {
                        hse_ready = true;
                    }
                    if done[pll1on] && !done[sw] && value & rule.pll1rdy != 0 {
                        pll_ready = true;
                    }
                }
                if a.bits == 32
                    && a.offset == rule.cfgr
                    && done[sw]
                    && value & rule.sws_mask == rule.sws_pll1
                {
                    sws = true;
                }
                continue;
            }
            let word_at = a.offset & !3;
            if let Some((name, _, fields)) = rule.selects.iter().find(|s| s.1 == word_at) {
                if a.bits != 32 || a.offset != word_at {
                    note(format!(
                        "Write{} to RCC.{name} {:#X}",
                        width_name(a.bits),
                        a.offset
                    ));
                    continue;
                }
                let moved: Vec<String> = fields
                    .iter()
                    .filter(|(_, _, mask)| value & mask != 0)
                    .map(|(field, pos, mask)| {
                        let width = (mask >> pos).count_ones() as usize;
                        format!("{field} {:0width$b}", (value & mask) >> pos)
                    })
                    .collect();
                note(format!(
                    "RCC.{name} {value:#X} written ({}): on an HSE board every kernel select stays at reset, the topology's SPI kernels being pll1_q and pclk, code 000",
                    if moved.is_empty() {
                        "every field at reset".to_owned()
                    } else {
                        format!("{} away from reset", moved.join(", "))
                    }
                ));
                continue;
            }
            if rule.clock_registers.contains(&word_at) {
                let at = rule.name(word_at);
                if a.bits != 32 || a.offset != word_at {
                    note(format!(
                        "Write{} to RCC.{at} {:#X}",
                        width_name(a.bits),
                        a.offset
                    ));
                    continue;
                }
                let next = done.iter().position(|d| !d);
                let matching = |want: bool| {
                    rule.steps
                        .iter()
                        .zip(&done)
                        .position(|(s, &d)| d == want && s.offset == a.offset && s.word == value)
                };
                let taken = if let Some(i) = matching(false) {
                    // A reorder is named at the earlier step, written after a
                    // later one: a step never written is only missing.
                    if let Some(j) = (i + 1..n).find(|&j| done[j]) {
                        note(format!(
                            "{} written after {}",
                            said(&rule.steps[i]),
                            said(&rule.steps[j])
                        ));
                    }
                    Some(i)
                } else if matching(true).is_some() {
                    note(format!("RCC.{at} {value:#X} written twice in a boot"));
                    None
                } else if let Some(i) = next.filter(|&i| rule.steps[i].offset == a.offset) {
                    // Not taken as the step: its word is still owed.
                    note(format!(
                        "RCC.{at} {value:#X} written, not the solved word {:#X} ({})",
                        rule.steps[i].word, rule.steps[i].what
                    ));
                    None
                } else {
                    note(format!(
                        "RCC.{at} {value:#X} written, not a word of the solved bring-up"
                    ));
                    None
                };
                let Some(i) = taken else { continue };
                done[i] = true;
                if !d3cr {
                    note(format!("{} written before PWR.D3CR", said(&rule.steps[i])));
                }
                if i == cssh && !hse_ready {
                    note(format!(
                        "{} written before a read of RCC.CR showed HSERDY 1 after HSEON",
                        said(&rule.steps[i])
                    ));
                }
                if i == sw {
                    if !pll_ready {
                        note(format!(
                            "{} written before a read of RCC.CR showed PLL1RDY 1 after PLL1ON",
                            said(&rule.steps[i])
                        ));
                    }
                    if acr_written == 0 || acr_read == 0 {
                        note(format!(
                            "{} written before FLASH_ACR was written and read back",
                            said(&rule.steps[i])
                        ));
                    }
                }
                continue;
            }
            if !sws && let Some((e, _)) = rule.enables.iter().find(|e| e.1 == word_at) {
                note(format!(
                    "RCC.{e} written before a read of RCC.CFGR showed SWS = PLL1 after SW"
                ));
            }
        }
        if last {
            for (s, _) in rule.steps.iter().zip(&done).filter(|(_, d)| !**d) {
                note(format!(
                    "{} never written in the boot the reads judge",
                    said(s)
                ));
            }
            if done[sw] && !sws {
                note(
                    "no read of RCC.CFGR showed SWS = PLL1 after SW in the boot the reads judge"
                        .to_owned(),
                );
            }
            if acr_written == 0 {
                note("FLASH_ACR never written in the boot the reads judge".to_owned());
            } else if acr_read == 0 {
                note("FLASH_ACR never read back in the boot the reads judge".to_owned());
            }
        }
    }
    out
}
