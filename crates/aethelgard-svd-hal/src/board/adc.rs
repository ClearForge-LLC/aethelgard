//! A pin device's ADC inputs (`AET-WO-1211b`; `1211` §2.7 (c)): each ADC the
//! topology solves a device input onto (ADC1 or ADC3, never ADC2, `1211` Q8)
//! is enabled, calibrated and converted once at bring-up, on the reset clock.
//!
//! The sequence is RM0433 Rev 9 ch. 25's, every value cited where it is
//! written; the contract it was read from is `spike/1211b/adc-contract.md`.
//! Every setting holds on both silicon revisions (Y and V), since the
//! revision a part carries is not known:
//! - the clock is the synchronous one, `ADCx_CCR.CKMODE` `11` (adc_sclk / 4):
//!   `RCC_D3CCIPR.ADCSEL` resets to `pll2_p_ck` (p. 419), which no generated
//!   firmware starts; revision V divides the analog clock by a further 2
//!   (p. 1039), so fADC is 16.24 MHz at most on revision Y (DS12117 Rev 10
//!   Table 76 p. 166, 36 MHz with `BOOST`) and 8.12 MHz on revision V (Table
//!   173 p. 272, 12.5 MHz with `BOOST` `01`);
//! - `ADC_CR.BOOST` `01`: bit 8, revision Y's one `BOOST` bit and revision V's
//!   6.25–12.5 MHz range;
//! - `ADC_CFGR.RES` from the declared resolution by the codes both revisions
//!   share (p. 1013): 16 `000`, 14 `001`, 12 `010`, 10 `011`; 8 bits has no
//!   common code (`100` revision Y, `111` revision V) and is refused by name;
//! - every channel at the longest sampling time, `111`, 810.5 cycles (pp.
//!   1019–1020): at the fastest fADC 49.9 µs, above every minimum of DS12117
//!   Table 174 (p. 276), the largest 3.69 µs;
//! - a linearity calibration (`ADCALLIN` 1), so `ADEN` and `ADSTART` are each
//!   written read-modify-write with every other command bit 0: the
//!   calibration leaves `LINCALRDYW1`–`6` set by hardware, and a whole word
//!   would clear them at once (RM0433 Rev 9 pp. 931–932; the architect's
//!   ruling Q-A, 2026-11-20).
//!
//! Each wait is bounded and its expiry a named fault (`AdcFault`): the
//! regulator's start-up is a fixed count of reads, the others polls. The
//! bounds are in reads, each at least one core cycle: since fADC is the core
//! clock over 8 (revision V) or 4 (revision Y), a count of core cycles is
//! exact whatever HSI's error. DS12117 states revision V's calibration,
//! stabilisation and conversion times in the Min column only (Table 173
//! p. 273), and a bound at a minimum faults a healthy part, so each bound is
//! **twice** the stated figure at revision V's 8 core cycles per ADC cycle:
//! the architect's factor (ruling Q-B, 2026-11-20), not the datasheet's.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use aethelgard_ir::SolvedTopology;

use super::{BoardError, Gate, gate};
use crate::cmsis::{self, RegisterFacts};

/// `tADCVREG_STUP`, 10 µs (DS12117 Rev 10 Table 76 p. 167, Table 173
/// p. 273), at HSI's fastest, 64,958,976 Hz, rounded up: 650 reads.
pub(super) const REGULATOR_READS: u32 = 650;
/// ruled (Q-B): 2 × tCAL 165,010 ADC cycles (offset and linearity; DS12117
/// Table 173 p. 273, Min column; Table 76 p. 167) × 8 core cycles.
pub(super) const CALIBRATION_POLLS: u32 = 2 * 165_010 * 8;
/// ruled (Q-B): 2 × tSTAB, "1 conversion cycle" (Table 173 p. 273, Min
/// column), read as one conversion at the longest sampling time and 16 bits,
/// 810.5 + 0.5 + 8 = 819 ADC cycles, × 8 core cycles.
pub(super) const READY_POLLS: u32 = 2 * 819 * 8;
/// ruled (Q-B): 2 × (tCONV at 810.5 cycles and 12 bits, 817 ADC cycles, plus
/// tLATR at most 2.25, Table 173 p. 273: 819.25) × 8 core cycles.
pub(super) const CONVERSION_POLLS: u32 = 2 * 81_925 * 8 / 100;

/// `ADC_SMPRx.SMPy` `111`: 810.5 ADC cycles (RM0433 Rev 9 pp. 1019–1020).
const SMP_LONGEST: u32 = 0b111;
/// `ADCx_CCR.CKMODE` `11`: adc_sclk / 4 (RM0433 Rev 9 p. 1039).
/// The topology's `clock.adc.ckmode` must equal it (`AET-WO-4240e`).
pub(super) const CKMODE_SCLK_DIV4: u32 = 0b11;
/// `ADC_CR.BOOST` `01` (bit 8 alone).
const BOOST_01: u32 = 0b01;
/// The most inputs one ADC converts at bring-up: `SQ1`–`SQ4`, all in
/// `ADC_SQR1` (RM0433 Rev 9 p. 1023).
const SEQUENCE_MAX: usize = 4;

/// One input of an ADC's sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AdcInput {
    pub net: String,
    pub pin: String,
    /// `INP5` is 5.
    pub channel: u32,
}

/// One ADC's bring-up: its inputs in the sequence's order (by channel), and
/// the resolution they share.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AdcPlan {
    pub inputs: Vec<AdcInput>,
    pub bits: u8,
}

/// Each ADC this generator brings up, with the names its other parts take:
/// (ADC, the header's common instance, its RCC gate (ADC1 and ADC2 share
/// `ADC12EN`), its block variant, its common block's variant).
const NAMES: [(&str, &str, &str, &str, &str); 2] = [
    ("ADC1", "ADC12_COMMON", "ADC12", "Adc1", "Adc12Common"),
    ("ADC3", "ADC3_COMMON", "ADC3", "Adc3", "Adc3Common"),
];

/// `adc`'s row of [`NAMES`]; [`plan`] admits no other ADC.
fn names(adc: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    let (_, common, gate, block, common_block) = NAMES
        .iter()
        .find(|n| n.0 == adc)
        .copied()
        .expect("plan admits ADC1 and ADC3 only");
    (common, gate, block, common_block)
}

/// The header's common instance (`ADC12_COMMON`).
pub(super) fn common(adc: &str) -> &'static str {
    names(adc).0
}

/// The block's variant in the board crate.
pub(super) fn block(adc: &str) -> String {
    names(adc).2.to_owned()
}

/// The common block's variant.
pub(super) fn common_block(adc: &str) -> String {
    names(adc).3.to_owned()
}

/// The generator's `RES` code for `bits`, the code both revisions share
/// (RM0433 Rev 9 p. 1013); `None` for 8 bits, which has none.
pub(super) fn res_code(bits: u8) -> Option<u32> {
    match bits {
        16 => Some(0b000),
        14 => Some(0b001),
        12 => Some(0b010),
        10 => Some(0b011),
        _ => None,
    }
}

/// Each ADC a device input is solved onto, with its inputs and resolution.
/// `bits` gives a device input's declared resolution; `inputs` are the
/// device inputs ([`super::device_adc_inputs`]).
pub(super) fn plan<'a>(
    inputs: impl Iterator<Item = (&'a String, &'a aethelgard_ir::pin_devices::SolvedAdcInput)>,
    bits: impl Fn(&str) -> Option<u8>,
) -> Result<BTreeMap<String, AdcPlan>, BoardError> {
    let mut out: BTreeMap<String, AdcPlan> = BTreeMap::new();
    for (net, a) in inputs {
        if !NAMES.iter().any(|n| n.0 == a.peripheral) {
            return Err(BoardError::AdcNotGenerated {
                net: net.clone(),
                peripheral: a.peripheral.clone(),
            });
        }
        let channel = a
            .channel
            .strip_prefix("INP")
            .and_then(|n| n.parse::<u32>().ok())
            .filter(|n| *n <= 19)
            .ok_or_else(|| BoardError::AdcNotGenerated {
                net: net.clone(),
                peripheral: a.peripheral.clone(),
            })?;
        let b = bits(net).ok_or_else(|| BoardError::AdcResolutionNotSolved { net: net.clone() })?;
        if res_code(b).is_none() {
            return Err(BoardError::AdcResolutionNotCommon {
                net: net.clone(),
                bits: b,
            });
        }
        let p = out.entry(a.peripheral.clone()).or_insert(AdcPlan {
            inputs: Vec::new(),
            bits: b,
        });
        if p.bits != b {
            return Err(BoardError::AdcResolutionsDiffer {
                peripheral: a.peripheral.clone(),
            });
        }
        p.inputs.push(AdcInput {
            net: net.clone(),
            pin: a.pin.clone(),
            channel,
        });
    }
    for (name, p) in &mut out {
        p.inputs.sort_by_key(|i| i.channel);
        if p.inputs.len() > SEQUENCE_MAX {
            return Err(BoardError::AdcSequenceTooLong {
                peripheral: name.clone(),
                inputs: p.inputs.len(),
            });
        }
    }
    Ok(out)
}

/// The header's facts for one ADC: its block and its common block, the clock
/// gate, and every offset and position the bring-up writes.
#[derive(Debug, Clone)]
pub(super) struct AdcFacts {
    pub base: u32,
    pub common_base: u32,
    pub gate: Gate,
    pub isr: u32,
    pub cr: u32,
    pub cfgr: u32,
    pub smpr: [u32; 2],
    pub pcsel: u32,
    pub sqr1: u32,
    pub dr: u32,
    pub ccr: u32,
    pub pos: AdcPositions,
}

/// The bit positions, by ST's CMSIS names.
#[derive(Debug, Clone)]
pub(super) struct AdcPositions {
    pub advregen: u32,
    pub boost: u32,
    pub adcallin: u32,
    pub adcal: u32,
    pub aden: u32,
    pub addis: u32,
    pub adstart: u32,
    pub adstp: u32,
    pub jadstart: u32,
    pub jadstp: u32,
    pub adrdy: u32,
    pub eoc: u32,
    pub jqdis: u32,
    pub res: u32,
    pub ckmode: u32,
    pub l: u32,
    pub sq: [u32; SEQUENCE_MAX],
    pub smp: [u32; 20],
}

impl AdcPositions {
    /// `ADC_CR`'s command bits, each `rs` (RM0433 Rev 9 p. 101: writing 0 has
    /// no effect): `ADCAL`, `ADEN`, `ADDIS`, `ADSTART`, `ADSTP`, `JADSTART`,
    /// `JADSTP`.
    pub fn command(&self) -> u32 {
        [
            self.adcal,
            self.aden,
            self.addis,
            self.adstart,
            self.adstp,
            self.jadstart,
            self.jadstp,
        ]
        .iter()
        .fold(0, |m, p| m | (1u32 << p))
    }
}

pub(super) fn facts(
    f: &RegisterFacts,
    rcc: &cmsis::Layout,
    adc: &str,
) -> Result<AdcFacts, BoardError> {
    let a = f.layout("ADC")?;
    let c = f.layout("ADC_Common")?;
    let p = |name: &str| f.position(name);
    let mut sq = [0; SEQUENCE_MAX];
    for (i, slot) in sq.iter_mut().enumerate() {
        *slot = p(&format!("ADC_SQR1_SQ{}_Pos", i + 1))?;
    }
    let mut smp = [0; 20];
    for (ch, slot) in smp.iter_mut().enumerate() {
        let r = if ch < 10 { 1 } else { 2 };
        *slot = p(&format!("ADC_SMPR{r}_SMP{ch}_Pos"))?;
    }
    Ok(AdcFacts {
        base: f.base(adc)?,
        common_base: f.base(common(adc))?,
        gate: gate(f, rcc, names(adc).1)?,
        isr: a.offset("ISR", 0)?,
        cr: a.offset("CR", 0)?,
        cfgr: a.offset("CFGR", 0)?,
        smpr: [a.offset("SMPR1", 0)?, a.offset("SMPR2", 0)?],
        pcsel: a.offset("PCSEL", 0)?,
        sqr1: a.offset("SQR1", 0)?,
        dr: a.offset("DR", 0)?,
        ccr: c.offset("CCR", 0)?,
        pos: AdcPositions {
            advregen: p("ADC_CR_ADVREGEN_Pos")?,
            boost: p("ADC_CR_BOOST_Pos")?,
            adcallin: p("ADC_CR_ADCALLIN_Pos")?,
            adcal: p("ADC_CR_ADCAL_Pos")?,
            aden: p("ADC_CR_ADEN_Pos")?,
            addis: p("ADC_CR_ADDIS_Pos")?,
            adstart: p("ADC_CR_ADSTART_Pos")?,
            adstp: p("ADC_CR_ADSTP_Pos")?,
            jadstart: p("ADC_CR_JADSTART_Pos")?,
            jadstp: p("ADC_CR_JADSTP_Pos")?,
            adrdy: p("ADC_ISR_ADRDY_Pos")?,
            eoc: p("ADC_ISR_EOC_Pos")?,
            jqdis: p("ADC_CFGR_JQDIS_Pos")?,
            res: p("ADC_CFGR_RES_Pos")?,
            ckmode: p("ADC_CCR_CKMODE_Pos")?,
            l: p("ADC_SQR1_L_Pos")?,
            sq,
            smp,
        },
    })
}

/// The bring-up's whole-word writes before calibration ends, in order, as
/// the generator composes them: (common block?, offset, value, what).
pub(super) fn writes(p: &AdcPlan, a: &AdcFacts) -> Vec<(bool, u32, u32, &'static str)> {
    let q = &a.pos;
    let reg = (1u32 << q.advregen) | (BOOST_01 << q.boost);
    let res = res_code(p.bits).expect("planned bits have a common code");
    let mut smpr = [0u32; 2];
    for i in &p.inputs {
        smpr[usize::from(i.channel >= 10)] |= SMP_LONGEST << q.smp[i.channel as usize];
    }
    let mut out = vec![
        (
            true,
            a.ccr,
            CKMODE_SCLK_DIV4 << q.ckmode,
            "CCR: CKMODE 11, adc_sclk / 4 (RM0433 Rev 9 p. 1039)",
        ),
        (
            false,
            a.cr,
            0,
            "CR: DEEPPWD 0, out of deep power-down (p. 926)",
        ),
        (
            false,
            a.cr,
            reg,
            "CR: ADVREGEN 1 and BOOST 01 (pp. 926, 1006)",
        ),
    ];
    let cfgr = (1u32 << q.jqdis) | (res << q.res);
    out.push((
        false,
        a.cfgr,
        cfgr,
        "CFGR: JQDIS as at reset, RES the declared resolution (p. 1013)",
    ));
    let pcsel = p.inputs.iter().fold(0u32, |m, i| m | (1 << i.channel));
    out.push((
        false,
        a.pcsel,
        pcsel,
        "PCSEL: every sequenced channel preselected (p. 1021)",
    ));
    for (k, word) in smpr.iter().enumerate() {
        if *word != 0 {
            out.push((
                false,
                a.smpr[k],
                *word,
                "SMPR: 810.5 cycles on every sequenced channel (pp. 1019-1020)",
            ));
        }
    }
    let n = u32::try_from(p.inputs.len()).expect("at most four");
    let sqr1 = p
        .inputs
        .iter()
        .enumerate()
        .fold((n - 1) << q.l, |w, (k, i)| w | (i.channel << q.sq[k]));
    out.push((
        false,
        a.sqr1,
        sqr1,
        "SQR1: the sequence, one conversion per input (p. 1023)",
    ));
    out.push((
        false,
        a.cr,
        reg | (1 << q.adcallin),
        "CR: ADCALLIN 1, single-ended (p. 928)",
    ));
    out.push((
        false,
        a.cr,
        reg | (1 << q.adcallin) | (1 << q.adcal),
        "CR: ADCAL 1, the calibration starts (p. 928)",
    ));
    out
}

/// The items every ADC shares, once: the fault, the bounds and the wait.
pub(super) fn emit_shared(w: &mut String) {
    let _ = writeln!(
        w,
        "/// Why an ADC's bring-up stopped (`AET-WO-1211b`): each bounded wait by its own\n\
         /// name, never a silent fall-through (the architect's ruling Q-B).\n\
         #[derive(Clone, Copy, PartialEq, Eq, Debug)]\n\
         pub enum AdcFault {{\n    \
             /// `ADC_CR.ADCAL` still 1 at the calibration's bound (`AdcCalibrationTimeout`).\n    \
             CalibrationTimeout,\n    \
             /// `ADC_ISR.ADRDY` still 0 at the enable's bound.\n    \
             ReadyTimeout,\n    \
             /// `ADC_ISR.EOC` still 0 at a conversion's bound.\n    \
             ConversionTimeout,\n\
         }}\n\
         \n\
         /// The bounds of an ADC's bring-up, in reads (each at least one core cycle).\n\
         pub struct AdcPolls {{\n    \
             pub regulator: u32,\n    \
             pub calibration: u32,\n    \
             pub ready: u32,\n    \
             pub conversion: u32,\n\
         }}\n\
         \n\
         /// The firmware's bounds: tADCVREG_STUP's 10 us at HSI's fastest (DS12117 Rev 10\n\
         /// Table 173 p. 273), and each wait twice DS12117's figure at revision V's 8 core\n\
         /// cycles per ADC cycle, a factor ruled by the architect (Q-B), not stated: tCAL\n\
         /// 165,010, tSTAB 819 and tCONV + tLATR 819.25 ADC cycles.\n\
         pub const ADC_POLLS: AdcPolls = AdcPolls {{\n    \
             regulator: {REGULATOR_READS},\n    \
             calibration: {CALIBRATION_POLLS},\n    \
             ready: {READY_POLLS},\n    \
             conversion: {CONVERSION_POLLS},\n\
         }};\n\
         \n\
         /// Reads `offset` of `block` at most `polls` times, until the bits of `mask` read\n\
         /// `want`.\n\
         fn adc_wait<R: Registers>(r: &mut R, block: Block, offset: usize, mask: u32, want: u32, polls: u32) -> bool {{\n    \
             for _ in 0..polls {{\n        \
                 if r.read(TOKEN, block, offset) & mask == want {{\n            \
                     return true;\n        \
                 }}\n    \
             }}\n    \
             false\n\
         }}\n"
    );
}

/// One ADC's typestate and its bring-up, taking each input's analog pin.
pub(super) fn emit(
    w: &mut String,
    name: &str,
    p: &AdcPlan,
    a: &AdcFacts,
    pin_type: &dyn Fn(&str) -> String,
) {
    let t = block(name);
    let c = common_block(name);
    let q = &a.pos;
    let params: Vec<String> = p
        .inputs
        .iter()
        .map(|i| format!("_in{}: {}<Analog>", i.channel, pin_type(&i.pin)))
        .collect();
    let nets: Vec<String> = p
        .inputs
        .iter()
        .map(|i| format!("`{}` (INP{}, {})", i.net, i.channel, i.pin))
        .collect();
    let n = p.inputs.len();
    let _ = writeln!(
        w,
        "/// `{name}`: {nets}, at {bits} bits. It starts only from `Clocked`, on the reset\n\
         /// clock, and only with every input's pin analog.\n\
         pub struct {t}<S> {{\n    \
             _state: PhantomData<S>,\n\
         }}\n\
         \n\
         impl {t}<Clocked> {{\n    \
             /// Enabled, calibrated and converted once, in RM0433 Rev 9's order (§25.4.6-\n    \
             /// §25.4.9, pp. 926-933): every word below is written whole but `ADEN` and\n    \
             /// `ADSTART`, which are read-modify-write with every other command bit 0, since\n    \
             /// the linearity calibration leaves `LINCALRDYW1`-`6` set by hardware and only\n    \
             /// one may change at once (pp. 931-932). Each wait is bounded by `polls`; the\n    \
             /// result holds each input's conversion, in channel order.\n    \
             pub fn bring_up<R: Registers>(\n        \
                 self,\n        \
                 r: &mut R,\n        \
                 _clock: &ResetClock,\n        \
                 {params},\n        \
                 polls: &AdcPolls,\n    \
             ) -> Result<[u16; {n}], AdcFault> {{",
        nets = nets.join(", "),
        bits = p.bits,
        params = params.join(",\n        "),
    );
    let ws = writes(p, a);
    for (k, (on_common, offset, value, what)) in ws.iter().enumerate() {
        let b = if *on_common { &c } else { &t };
        let _ = writeln!(
            w,
            "        r.write(TOKEN, Block::{b}, {offset:#04x}, {value:#010x}); // {what}"
        );
        if k == 2 {
            let _ = writeln!(
                w,
                "        // tADCVREG_STUP (DS12117 Rev 10 Table 173 p. 273): a fixed count of reads;\n        \
                 // LDORDY exists on revision V only and is not read.\n        \
                 for _ in 0..polls.regulator {{\n            \
                     let _ = r.read(TOKEN, Block::{t}, {isr:#04x});\n        \
                 }}",
                isr = a.isr
            );
        }
    }
    let command = q.command();
    let _ = writeln!(
        w,
        "        if !adc_wait(r, Block::{t}, {cr:#04x}, {adcal:#010x}, 0, polls.calibration) {{\n            \
                 return Err(AdcFault::CalibrationTimeout);\n        \
             }}\n        \
             r.write(TOKEN, Block::{t}, {isr:#04x}, {adrdy:#010x}); // ISR: ADRDY cleared, rc_w1 (p. 998)\n        \
             // ADEN read-modify-write: the calibration's LINCALRDYW bits kept, every other\n        \
             // command bit 0 (RM0433 Rev 9 pp. 931-932).\n        \
             r.modify(TOKEN, Block::{t}, {cr:#04x}, |v| (v & !{command:#010x}) | {aden:#010x});\n        \
             if !adc_wait(r, Block::{t}, {isr:#04x}, {adrdy:#010x}, {adrdy:#010x}, polls.ready) {{\n            \
                 return Err(AdcFault::ReadyTimeout);\n        \
             }}\n        \
             // ADSTART read-modify-write, as ADEN (pp. 931-932, 938).\n        \
             r.modify(TOKEN, Block::{t}, {cr:#04x}, |v| (v & !{command:#010x}) | {adstart:#010x});\n        \
             let mut out = [0u16; {n}];\n        \
             for slot in out.iter_mut() {{\n            \
                 if !adc_wait(r, Block::{t}, {isr:#04x}, {eoc:#010x}, {eoc:#010x}, polls.conversion) {{\n                \
                     return Err(AdcFault::ConversionTimeout);\n            \
                 }}\n            \
                 // Reading DR clears EOC (p. 1000).\n            \
                 *slot = (r.read(TOKEN, Block::{t}, {dr:#04x}) & 0xffff) as u16;\n        \
             }}\n        \
             Ok(out)\n    \
         }}\n\
         }}\n",
        cr = a.cr,
        isr = a.isr,
        dr = a.dr,
        adcal = 1u32 << q.adcal,
        adrdy = 1u32 << q.adrdy,
        aden = 1u32 << q.aden,
        adstart = 1u32 << q.adstart,
        eoc = 1u32 << q.eoc,
    );
}

/// The oracle's `RES` code (`AET-WO-1211b`), from the codes both revisions
/// share (RM0433 Rev 9 p. 1013).
///
/// belt-and-suspenders: this restates [`res_code`] on purpose; the proof's
/// expected words must not be what the crate under proof composes
/// (`INV-07`).
fn oracle_res(bits: u8) -> u32 {
    match bits {
        16 => 0b000,
        14 => 0b001,
        12 => 0b010,
        _ => 0b011,
    }
}

/// The oracle's whole-word writes before calibration ends, from the
/// topology's inputs and the header's positions, never from [`writes`]:
/// (common block?, offset, value).
///
/// belt-and-suspenders: this restates [`writes`]' values and order on
/// purpose (`INV-07`).
fn oracle(o: &OracleAdc, a: &AdcFacts) -> Vec<(bool, u32, u32)> {
    let q = &a.pos;
    let reg = (1u32 << q.advregen) | (1u32 << q.boost);
    let mut out = vec![
        (true, a.ccr, 0b11 << q.ckmode),
        (false, a.cr, 0),
        (false, a.cr, reg),
    ];
    out.push((
        false,
        a.cfgr,
        (1 << q.jqdis) | (oracle_res(o.bits) << q.res),
    ));
    let mut pcsel = 0;
    let mut smpr = [0u32; 2];
    let mut sqr1 = u32::try_from(o.channels.len().saturating_sub(1)).unwrap_or(0) << q.l;
    for (k, ch) in o.channels.iter().enumerate() {
        pcsel |= 1 << ch;
        let (r, pos) = if *ch < 10 {
            (0, 3 * ch)
        } else {
            (1, 3 * (ch - 10))
        };
        smpr[r] |= 0b111 << pos;
        sqr1 |= ch << (6 + 6 * k as u32);
    }
    out.push((false, a.pcsel, pcsel));
    for (r, word) in smpr.iter().enumerate() {
        if *word != 0 {
            out.push((false, a.smpr[r], *word));
        }
    }
    out.push((false, a.sqr1, sqr1));
    out.push((false, a.cr, reg | (1 << q.adcallin)));
    out.push((false, a.cr, reg | (1 << q.adcallin) | (1 << q.adcal)));
    out
}

/// The oracle's own reading of one ADC (`INV-07`; the adversarial pass's
/// F4): its device inputs' channels in ascending order and their declared
/// resolution, read from `solved.adc_inputs` apart from [`plan`].
#[derive(Debug, Clone)]
pub(super) struct OracleAdc {
    pub channels: Vec<u32>,
    pub bits: u8,
}

/// Each ADC's [`OracleAdc`], every `power.sense` input left out.
///
/// belt-and-suspenders: this restates [`plan`]'s reading on purpose
/// (`INV-07`).
pub(super) fn oracle_of(solved: &SolvedTopology) -> BTreeMap<String, OracleAdc> {
    let sense: std::collections::BTreeSet<&String> =
        solved.power.iter().flat_map(|p| p.sense.keys()).collect();
    let mut out: BTreeMap<String, OracleAdc> = BTreeMap::new();
    for (_, a) in solved.adc_inputs.iter().filter(|(n, _)| !sense.contains(n)) {
        let o = out.entry(a.peripheral.clone()).or_insert(OracleAdc {
            channels: Vec::new(),
            bits: a.bits.unwrap_or(0),
        });
        o.channels.extend(
            a.channel
                .strip_prefix("INP")
                .and_then(|c| c.parse::<u32>().ok()),
        );
    }
    for o in out.values_mut() {
        o.channels.sort_unstable();
    }
    out
}

/// The proof of one ADC's bring-up on a spy of its own: every word the
/// oracle's, in order; the calibration's hardware-set bits kept by the
/// `ADEN` and `ADSTART` writes; no `ADC_CR` write with two command bits;
/// each wait bounded and each expiry its own fault, with nothing written
/// after it.
pub(super) fn emit_proof(
    w: &mut String,
    name: &str,
    p: &AdcPlan,
    o: &OracleAdc,
    a: &AdcFacts,
    setup: &str,
) {
    let t = block(name);
    let c = common_block(name);
    let q = &a.pos;
    let f = name.to_ascii_lowercase();
    let expect = oracle(o, a);
    let k = expect.len();
    let items: Vec<String> = expect
        .iter()
        .map(|(on_common, offset, value)| {
            format!(
                "(Block::{}, {offset:#04x}, {value:#010x})",
                if *on_common { &c } else { &t }
            )
        })
        .collect();
    let n = o.channels.len();
    let args: Vec<String> = p
        .inputs
        .iter()
        .map(|i| i.pin.to_ascii_lowercase())
        .collect();
    let command = q.command();
    let _ = writeln!(
        w,
        "\n\
         /// `{name}` as its bring-up may touch it. `phase`: 0 the whole words, 1 the\n\
         /// calibration's wait, 2 after `ISR`'s clear, 3 the enable's wait, 4 the\n\
         /// conversions. Every read is any word; `cr` is the last `ADC_CR` read.\n\
         struct {t}Spy {{\n    \
             next: usize,\n    \
             phase: u8,\n    \
             cr: u32,\n    \
             regulator_reads: u32,\n    \
             cal_reads: u32,\n    \
             calibrated: bool,\n    \
             ready_reads: u32,\n    \
             ready: bool,\n    \
             eoc_reads: u32,\n    \
             eoc: bool,\n    \
             dr_reads: u32,\n\
         }}\n\
         \n\
         impl sealed::Sealed for {t}Spy {{}}\n\
         \n\
         impl Registers for {t}Spy {{\n    \
             fn read(&mut self, _: sealed::Token, block: Block, offset: usize) -> u32 {{\n        \
                 kani::assert(block == Block::{t}, \"the bring-up read a block other than {name}\");\n        \
                 let v: u32 = kani::any();\n        \
                 if self.phase == 0 && offset == {isr:#04x} {{\n            \
                     kani::assert(self.next == 3, \"the regulator's wait is not after ADVREGEN and before CFGR\");\n            \
                     self.regulator_reads += 1;\n        \
                 }} else if self.phase == 1 && offset == {cr:#04x} {{\n            \
                     kani::assert(!self.calibrated, \"ADCAL polled after it read 0\");\n            \
                     self.cal_reads += 1;\n            \
                     self.calibrated = v & {adcal:#010x} == 0;\n            \
                     self.cr = v;\n        \
                 }} else if (self.phase == 2 || (self.phase == 3 && self.ready)) && offset == {cr:#04x} {{\n            \
                     self.cr = v;\n        \
                 }} else if self.phase == 3 && offset == {isr:#04x} {{\n            \
                     kani::assert(!self.ready, \"ADRDY polled after it read 1\");\n            \
                     self.ready_reads += 1;\n            \
                     self.ready = v & {adrdy:#010x} != 0;\n        \
                 }} else if self.phase == 4 && offset == {isr:#04x} {{\n            \
                     kani::assert(!self.eoc, \"EOC polled after it read 1\");\n            \
                     self.eoc_reads += 1;\n            \
                     self.eoc = v & {eoc:#010x} != 0;\n        \
                 }} else if self.phase == 4 && offset == {dr:#04x} {{\n            \
                     kani::assert(self.eoc, \"DR read before EOC read 1\");\n            \
                     self.eoc = false;\n            \
                     self.dr_reads += 1;\n        \
                 }} else {{\n            \
                     kani::assert(false, \"the bring-up read a register out of its order\");\n        \
                 }}\n        \
                 v\n    \
             }}\n    \
             fn write(&mut self, _: sealed::Token, block: Block, offset: usize, value: u32) {{\n        \
                 const EXPECT: [(Block, usize, u32); {k}] = [{items}];\n        \
                 if block == Block::{t} && offset == {cr:#04x} {{\n            \
                     kani::assert(\n                \
                         (value & {command:#010x}).count_ones() <= 1,\n                \
                         \"an ADC_CR write sets two command bits\",\n            \
                     );\n        \
                 }}\n        \
                 if self.phase == 0 {{\n            \
                     kani::assert(self.next < {k}, \"a write past the whole words\");\n            \
                     let (b, o, v) = EXPECT[self.next];\n            \
                     kani::assert(block == b && offset == o && value == v, \"not the oracle's word in the oracle's order\");\n            \
                     self.next += 1;\n            \
                     if self.next == {k} {{\n                \
                         self.phase = 1;\n            \
                     }}\n        \
                 }} else if self.phase == 1 && block == Block::{t} && offset == {isr:#04x} {{\n            \
                     kani::assert(self.calibrated, \"ISR cleared before ADCAL read 0\");\n            \
                     kani::assert(value == {adrdy:#010x}, \"ISR written other than ADRDY alone\");\n            \
                     self.phase = 2;\n        \
                 }} else if self.phase == 2 && block == Block::{t} && offset == {cr:#04x} {{\n            \
                     kani::assert(\n                \
                         value == (self.cr & !{command:#010x}) | {aden:#010x},\n                \
                         \"ADEN's write is not CR as read, calibration bits kept, with ADEN alone of the command bits\",\n            \
                     );\n            \
                     self.phase = 3;\n        \
                 }} else if self.phase == 3 && block == Block::{t} && offset == {cr:#04x} {{\n            \
                     kani::assert(self.ready, \"ADSTART written before ADRDY read 1\");\n            \
                     kani::assert(\n                \
                         value == (self.cr & !{command:#010x}) | {adstart:#010x},\n                \
                         \"ADSTART's write is not CR as read with ADSTART alone of the command bits\",\n            \
                     );\n            \
                     self.phase = 4;\n        \
                 }} else {{\n            \
                     kani::assert(false, \"a write out of the bring-up's order\");\n        \
                 }}\n    \
             }}\n    \
             fn read8(&mut self, _: sealed::Token, _: Block, _: usize) -> u8 {{\n        \
                 kani::assert(false, \"the bring-up made a byte access\");\n        \
                 0\n    \
             }}\n    \
             fn write8(&mut self, _: sealed::Token, _: Block, _: usize, _: u8) {{\n        \
                 kani::assert(false, \"the bring-up made a byte access\");\n    \
             }}\n\
         }}\n\
         \n\
         /// `{name}`'s bring-up at bounds of two reads each, on any words: the oracle's\n\
         /// words in order, `ADEN` and `ADSTART` read-modify-write keeping every other\n\
         /// bit, no write with two command bits, and each outcome by its own name.\n\
         #[kani::proof]\n\
         #[kani::unwind({unwind})]\n\
         fn {f}_bring_up_is_in_order_and_bounded() {{\n\
         {setup}    \
             let mut spy = {t}Spy {{\n        \
                 next: 0,\n        \
                 phase: 0,\n        \
                 cr: 0,\n        \
                 regulator_reads: 0,\n        \
                 cal_reads: 0,\n        \
                 calibrated: false,\n        \
                 ready_reads: 0,\n        \
                 ready: false,\n        \
                 eoc_reads: 0,\n        \
                 eoc: false,\n        \
                 dr_reads: 0,\n    \
             }};\n    \
             let polls = AdcPolls {{ regulator: 2, calibration: 2, ready: 2, conversion: 2 }};\n    \
             let result = {f}.bring_up(&mut spy, &clock, {args}, &polls);\n    \
             kani::assert(\n        \
                 spy.regulator_reads == 2 && spy.cal_reads <= 2 && spy.ready_reads <= 2 && spy.eoc_reads <= 2 * {n},\n        \
                 \"a wait read past its bound\",\n    \
             );\n    \
             match result {{\n        \
                 Ok(_) => {{\n            \
                     kani::assert(spy.phase == 4 && spy.dr_reads == {n}, \"converted without every step\");\n            \
                     kani::cover!(true, \"{name} converted\");\n        \
                 }}\n        \
                 Err(AdcFault::CalibrationTimeout) => {{\n            \
                     kani::assert(spy.phase == 1 && !spy.calibrated && spy.cal_reads == 2, \"CalibrationTimeout other than at its bound\");\n            \
                     kani::cover!(true, \"{name} calibration timed out\");\n        \
                 }}\n        \
                 Err(AdcFault::ReadyTimeout) => {{\n            \
                     kani::assert(spy.phase == 3 && !spy.ready && spy.ready_reads == 2, \"ReadyTimeout other than at its bound\");\n        \
                 }}\n        \
                 Err(AdcFault::ConversionTimeout) => {{\n            \
                     kani::assert(spy.phase == 4 && !spy.eoc, \"ConversionTimeout other than at its bound\");\n        \
                 }}\n    \
             }}\n\
         }}",
        isr = a.isr,
        cr = a.cr,
        dr = a.dr,
        adcal = 1u32 << q.adcal,
        adrdy = 1u32 << q.adrdy,
        aden = 1u32 << q.aden,
        adstart = 1u32 << q.adstart,
        eoc = 1u32 << q.eoc,
        items = items.join(", "),
        args = args.join(", "),
        // One more than the longest loop: the bounds' 2 reads, or the inputs'
        // conversions (the adversarial pass's F1).
        unwind = n.max(2) + 2,
    );
}

/// The device ADC inputs of `solved`, as the generator plans them.
pub(super) fn of(solved: &SolvedTopology) -> Result<BTreeMap<String, AdcPlan>, BoardError> {
    plan(super::device_adc_inputs(solved), |net| {
        adc_bits(solved, net)
    })
}

/// A device ADC input's declared resolution, as the topology carries it
/// (`SolvedAdcInput::bits`, `AET-WO-1211b` S1); none is substituted
/// (`INV-06`).
fn adc_bits(solved: &SolvedTopology, net: &str) -> Option<u8> {
    solved.adc_inputs.get(net)?.bits
}
