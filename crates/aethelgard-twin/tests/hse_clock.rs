//! `AET-WO-4240b` §1.6: on a generated HSE board the twin allows exactly the
//! H1 bring-up's RCC and `FLASH_ACR` accesses, in order, and names every
//! departure; on synthetic access traces, since the generator that performs
//! the sequence is another branch's. Every expected word here is the PM's
//! contract's literal (`~/.cache/aet-4240b/contract.md`), never the rule's
//! own arithmetic.

use std::fs;
use std::path::Path;

use aethelgard_ir::{SolvedTopology, Specification, solve};
use aethelgard_twin::boot::{self, FLASH_RENODE};
use aethelgard_twin::hse::{self, ClockRule, judge_clock};
use aethelgard_twin::platform;
use aethelgard_twin::serial::{self, Access, PWR_RENODE, RCC_RENODE};

fn topology(fixture: &str) -> SolvedTopology {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../aethelgard-cli/tests/fixtures")
        .join(format!("{fixture}.json"));
    let spec: Specification = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    solve(&spec, &table, &"0".repeat(64)).unwrap()
}

const CRYSTAL: &str = "hse-crystal-16";
const EXTERNAL: &str = "hse-external-8";

// RM0433 Rev 9 offsets: RCC §8.7, FLASH §4.9.1, PWR §6.8.6.
const CR: u32 = 0x000;
const CFGR: u32 = 0x010;
const PLLCKSELR: u32 = 0x028;
const PLLCFGR: u32 = 0x02C;
const PLL1DIVR: u32 = 0x030;
const D2CCIP1R: u32 = 0x050;
const D3CCIPR: u32 = 0x058;
const AHB4ENR: u32 = 0x0E0;
const FLASH_ACR: u32 = 0x000;
const FLASH_OPTSR_CUR: u32 = 0x01C;
const PWR_CR3: u32 = 0x00C;
const PWR_D3CR: u32 = 0x018;

fn w(source: &str, offset: u32, value: u32) -> Access {
    Access {
        source: source.to_owned(),
        write: true,
        bits: 32,
        offset,
        value: u64::from(value),
        refused: false,
    }
}

fn rd(source: &str, offset: u32, value: u32) -> Access {
    Access {
        write: false,
        ..w(source, offset, value)
    }
}

/// The contract's words for each fixture: the `RCC_CR` words of steps 1, 2,
/// 4 and 6, then `PLLCKSELR`, `PLLCFGR`, `PLL1DIVR` and `FLASH_ACR`.
struct Words {
    cr: &'static [u32],
    pll: [u32; 3],
    acr: u32,
}

const CRYSTAL_WORDS: Words = Words {
    cr: &[0x0001_0001, 0x0009_0001, 0x0109_0001],
    pll: [0x52, 0x0003_0004, 0x0103_064F],
    acr: 0x11,
};
const EXTERNAL_WORDS: Words = Words {
    cr: &[0x0004_0001, 0x0005_0001, 0x000D_0001, 0x010D_0001],
    pll: [0x52, 0x0003_0002, 0x0103_069F],
    acr: 0x11,
};

/// The contract's sequence as Renode logs it, from VOS Scale 2 to the first
/// enable write: the reset-clock read, the CR words with the `HSERDY` read
/// after `HSEON` and the `PLL1RDY` read after `PLL1ON` (Renode's measured
/// values), the PLL words, `FLASH_ACR` written and read, `SW` and the `SWS`
/// read.
fn sequence(words: &Words) -> Vec<Access> {
    let n = words.cr.len();
    let mut v = vec![
        rd(PWR_RENODE, PWR_CR3, 0x46),
        w(PWR_RENODE, PWR_CR3, 0x42),
        w(PWR_RENODE, PWR_D3CR, 0x8000),
        rd(RCC_RENODE, CR, 0x25),
        rd(RCC_RENODE, CFGR, 0x0),
    ];
    for &c in &words.cr[..n - 3] {
        v.push(w(RCC_RENODE, CR, c));
    }
    v.push(w(RCC_RENODE, CR, words.cr[n - 3]));
    v.push(rd(RCC_RENODE, CR, 0x0003_0025));
    v.push(w(RCC_RENODE, CR, words.cr[n - 2]));
    v.push(w(RCC_RENODE, PLLCKSELR, words.pll[0]));
    v.push(w(RCC_RENODE, PLLCFGR, words.pll[1]));
    v.push(w(RCC_RENODE, PLL1DIVR, words.pll[2]));
    v.push(w(RCC_RENODE, CR, words.cr[n - 1]));
    v.push(rd(RCC_RENODE, CR, 0x030B_0025));
    v.push(w(FLASH_RENODE, FLASH_ACR, words.acr));
    v.push(rd(FLASH_RENODE, FLASH_ACR, words.acr));
    v.push(w(RCC_RENODE, CFGR, 0x3));
    v.push(rd(RCC_RENODE, CFGR, 0x1B));
    v.push(w(RCC_RENODE, AHB4ENR, 0x1));
    v
}

fn rule(fixture: &str) -> ClockRule {
    hse::clock_rule(&topology(fixture)).unwrap().unwrap()
}

fn judged(rule: &ClockRule, boot: &[Access]) -> Vec<String> {
    judge_clock(rule, &[boot.to_vec()])
}

fn position(boot: &[Access], offset: u32, value: u32) -> usize {
    boot.iter()
        .position(|a| a.write && a.offset == offset && a.value == u64::from(value))
        .unwrap()
}

#[test]
fn the_rule_s_words_are_the_contract_s_for_both_fixtures() {
    for (fixture, words) in [(CRYSTAL, CRYSTAL_WORDS), (EXTERNAL, EXTERNAL_WORDS)] {
        let r = rule(fixture);
        let mut want: Vec<(u32, u32)> = words.cr[..words.cr.len() - 1]
            .iter()
            .map(|&c| (CR, c))
            .collect();
        want.extend([
            (PLLCKSELR, words.pll[0]),
            (PLLCFGR, words.pll[1]),
            (PLL1DIVR, words.pll[2]),
            (CR, words.cr[words.cr.len() - 1]),
            (CFGR, 0x3),
        ]);
        let got: Vec<(u32, u32)> = r.steps.iter().map(|s| (s.offset, s.word)).collect();
        assert_eq!(got, want, "{fixture}");
        assert_eq!((r.acr, r.acr_word), (FLASH_ACR, words.acr), "{fixture}");
        assert_eq!((r.cr, r.cfgr, r.d3cr), (CR, CFGR, PWR_D3CR));
        assert_eq!((r.hserdy, r.pll1rdy), (1 << 17, 1 << 25));
        assert_eq!((r.sws_mask, r.sws_pll1), (0x38, 0x18));
    }
}

#[test]
fn the_contract_s_sequence_passes_for_both_fixtures() {
    for (fixture, words) in [(CRYSTAL, CRYSTAL_WORDS), (EXTERNAL, EXTERNAL_WORDS)] {
        let r = rule(fixture);
        let boot = sequence(&words);
        assert_eq!(judged(&r, &boot), Vec::<String>::new(), "{fixture}");
        // A boot cut by a reset anywhere is judged as far as it went.
        for cut in 0..boot.len() {
            assert_eq!(
                judge_clock(&r, &[boot[..cut].to_vec(), boot.clone()]),
                Vec::<String>::new(),
                "{fixture} cut at {cut}"
            );
        }
        // Each ready read polled many times, as silicon would make it.
        let mut polled = boot.clone();
        let at = position(&polled, CR, words.cr[words.cr.len() - 3]) + 1;
        polled.insert(at, rd(RCC_RENODE, CR, 0x0001_0025));
        polled.insert(at, rd(RCC_RENODE, CR, 0x0001_0025));
        assert_eq!(judged(&r, &polled), Vec::<String>::new(), "{fixture}");
    }
}

#[test]
fn a_missing_word_a_repeat_and_a_reorder_are_named() {
    let r = rule(CRYSTAL);
    let good = sequence(&CRYSTAL_WORDS);

    let mut missing = good.clone();
    missing.remove(position(&missing, PLLCFGR, 0x0003_0004));
    assert_eq!(
        judged(&r, &missing),
        ["RCC.PLLCFGR 0x30004 (PLLCFGR) never written in the boot the reads judge"]
    );
    // An earlier boot may stop short of it; the last may not.
    assert!(judge_clock(&r, &[missing.clone(), good.clone()]).is_empty());

    let mut repeat = good.clone();
    let at = position(&repeat, CR, 0x0001_0001);
    repeat.insert(at + 1, w(RCC_RENODE, CR, 0x0001_0001));
    assert_eq!(
        judged(&r, &repeat),
        ["RCC.CR 0x10001 written twice in a boot"]
    );
    let mut cfgr_twice = good.clone();
    cfgr_twice.push(w(RCC_RENODE, CFGR, 0x3));
    assert_eq!(
        judged(&r, &cfgr_twice),
        ["RCC.CFGR 0x3 written twice in a boot"]
    );

    let mut reorder = good.clone();
    let a = position(&reorder, PLLCFGR, 0x0003_0004);
    reorder.swap(a, a + 1);
    assert_eq!(
        judged(&r, &reorder),
        ["RCC.PLLCFGR 0x30004 (PLLCFGR) written after RCC.PLL1DIVR 0x103064F (PLL1DIVR)"]
    );
    // HSECSSON before HSEON: a reorder, and no HSERDY read could precede it.
    let mut css_first = good.clone();
    let on = position(&css_first, CR, 0x0001_0001);
    let css = position(&css_first, CR, 0x0009_0001);
    css_first.swap(on, css);
    assert_eq!(
        judged(&r, &css_first),
        [
            "RCC.CR 0x90001 (HSECSSON) written before a read of RCC.CR showed HSERDY 1 after HSEON",
            "RCC.CR 0x10001 (HSEON) written after RCC.CR 0x90001 (HSECSSON)",
        ]
    );
}

#[test]
fn sw_before_the_pll1rdy_read_is_named() {
    let r = rule(CRYSTAL);
    let mut boot = sequence(&CRYSTAL_WORDS);
    // The mutant: SW straight after the FLASH_ACR read-back, the PLL1RDY
    // read moved after it.
    let ready = boot
        .iter()
        .position(|a| !a.write && a.offset == CR && a.value == 0x030B_0025)
        .unwrap();
    let read = boot.remove(ready);
    let sw = position(&boot, CFGR, 0x3);
    boot.insert(sw + 1, read);
    assert_eq!(
        judged(&r, &boot),
        [
            "FLASH_ACR written before a read of RCC.CR showed PLL1RDY 1",
            "RCC.CFGR 0x3 (SW = PLL1) written before a read of RCC.CR showed PLL1RDY 1 after PLL1ON",
        ]
    );
    // A PLL1RDY read before PLL1ON does not count.
    let mut early = sequence(&CRYSTAL_WORDS);
    let ready = early
        .iter()
        .position(|a| !a.write && a.offset == CR && a.value == 0x030B_0025)
        .unwrap();
    let read = early.remove(ready);
    let on = position(&early, CR, 0x0109_0001);
    early.insert(on, read);
    assert!(
        judged(&r, &early).contains(
            &"RCC.CFGR 0x3 (SW = PLL1) written before a read of RCC.CR showed PLL1RDY 1 after PLL1ON"
                .to_owned()
        ),
        "{:?}",
        judged(&r, &early)
    );
}

#[test]
fn a_select_written_on_an_hse_board_is_named() {
    let r = rule(CRYSTAL);
    let good = sequence(&CRYSTAL_WORDS);
    let enable = good.len() - 1;
    // The reset word itself is a write the contract forbids.
    let mut reset = good.clone();
    reset.insert(enable, w(RCC_RENODE, D2CCIP1R, 0x0));
    assert_eq!(
        judged(&r, &reset),
        [
            "RCC.D2CCIP1R 0x0 written (every field at reset): on an HSE board every kernel select stays at reset, the topology's SPI kernels being pll1_q and pclk, code 000"
        ]
    );
    // The mutant: SPI123SEL left at 100 (per_ck), as a null board writes it.
    let mut per_ck = good.clone();
    per_ck.insert(enable, w(RCC_RENODE, D2CCIP1R, 0x4000));
    assert_eq!(
        judged(&r, &per_ck),
        [
            "RCC.D2CCIP1R 0x4000 written (SPI123SEL 100 away from reset): on an HSE board every kernel select stays at reset, the topology's SPI kernels being pll1_q and pclk, code 000"
        ]
    );
    let mut d3 = good.clone();
    d3.insert(enable, w(RCC_RENODE, D3CCIPR, 0x3000_0000));
    assert_eq!(
        judged(&r, &d3),
        [
            "RCC.D3CCIPR 0x30000000 written (SPI6SEL 011 away from reset): on an HSE board every kernel select stays at reset, the topology's SPI kernels being pll1_q and pclk, code 000"
        ]
    );
}

#[test]
fn every_other_departure_is_named() {
    let r = rule(CRYSTAL);
    let good = sequence(&CRYSTAL_WORDS);
    let without = |offset: u32, write: bool, source: &str| -> Vec<Access> {
        let mut b = good.clone();
        let i = b
            .iter()
            .position(|a| a.source == source && a.offset == offset && a.write == write)
            .unwrap();
        b.remove(i);
        b
    };
    let replaced = |offset: u32, value: u32, by: u32| -> Vec<Access> {
        let mut b = good.clone();
        let i = position(&b, offset, value);
        b[i].value = u64::from(by);
        b
    };

    // A word other than the solved one is named, and the solved word is
    // still owed.
    assert_eq!(
        judged(&r, &replaced(PLLCKSELR, 0x52, 0x62)),
        [
            "RCC.PLLCKSELR 0x62 written, not the solved word 0x52 (PLLCKSELR)",
            "RCC.PLLCKSELR 0x52 (PLLCKSELR) never written in the boot the reads judge",
        ]
    );
    // The external fixture's HSEBYP missing: crystal's HSEON word is not
    // the bypass's.
    assert_eq!(
        judged(&r, &replaced(CR, 0x0001_0001, 0x0005_0001))[0],
        "RCC.CR 0x50001 written, not the solved word 0x10001 (HSEON)"
    );
    let mut extra = good.clone();
    extra.insert(4, w(RCC_RENODE, CR, 0x0000_0081));
    assert_eq!(
        judged(&r, &extra),
        ["RCC.CR 0x81 written, not the solved word 0x10001 (HSEON)"]
    );
    let mut stray = good.clone();
    stray.push(w(RCC_RENODE, PLLCFGR, 0x0100_0000));
    assert_eq!(
        judged(&r, &stray),
        ["RCC.PLLCFGR 0x1000000 written, not a word of the solved bring-up"]
    );
    let x = rule(EXTERNAL);
    let mut no_bypass = sequence(&EXTERNAL_WORDS);
    no_bypass.remove(position(&no_bypass, CR, 0x0004_0001));
    assert_eq!(
        judged(&x, &no_bypass),
        ["RCC.CR 0x40001 (HSEBYP) never written in the boot the reads judge"]
    );
    let mut byte = good.clone();
    byte.insert(
        4,
        Access {
            bits: 8,
            offset: CR + 2,
            ..w(RCC_RENODE, 0, 0x1)
        },
    );
    assert_eq!(judged(&r, &byte), ["WriteByte to RCC.CR 0x2"]);

    assert_eq!(
        judged(&r, &without(CR, false, RCC_RENODE)[..]).first(),
        None,
        "the reset read is not the rule's"
    );
    let mut no_ready = good.clone();
    no_ready.retain(|a| !(a.source == RCC_RENODE && !a.write && a.value == 0x0003_0025));
    assert_eq!(
        judged(&r, &no_ready),
        ["RCC.CR 0x90001 (HSECSSON) written before a read of RCC.CR showed HSERDY 1 after HSEON"]
    );
    // A CR read after HSEON whose HSERDY is 0 is not the ready read.
    let not_ready = good.clone().into_iter().map(|mut a| {
        if !a.write && a.value == 0x0003_0025 {
            a.value = 0x0001_0025;
        }
        a
    });
    assert_eq!(
        judged(&r, &not_ready.collect::<Vec<_>>()),
        ["RCC.CR 0x90001 (HSECSSON) written before a read of RCC.CR showed HSERDY 1 after HSEON"]
    );

    let mut no_sws = good.clone();
    no_sws.retain(|a| !(!a.write && a.offset == CFGR && a.value == 0x1B));
    assert_eq!(
        judged(&r, &no_sws),
        [
            "RCC.AHB4ENR written before a read of RCC.CFGR showed SWS = PLL1 after SW",
            "no read of RCC.CFGR showed SWS = PLL1 after SW in the boot the reads judge"
        ]
    );
    let mut no_sws_no_enable = no_sws.clone();
    no_sws_no_enable.pop();
    assert_eq!(
        judged(&r, &no_sws_no_enable),
        ["no read of RCC.CFGR showed SWS = PLL1 after SW in the boot the reads judge"]
    );
    let mut sws_hsi = good.clone();
    let i = sws_hsi
        .iter()
        .position(|a| !a.write && a.offset == CFGR && a.value == 0x1B)
        .unwrap();
    sws_hsi[i].value = 0x3;
    assert_eq!(judged(&r, &sws_hsi), judged(&r, &no_sws));

    assert_eq!(
        judged(&r, &without(PWR_D3CR, true, PWR_RENODE))[0],
        "RCC.CR 0x10001 (HSEON) written before PWR.D3CR"
    );

    assert_eq!(
        judged(&r, &without(FLASH_ACR, true, FLASH_RENODE)),
        [
            "FLASH_ACR read before it was written",
            "RCC.CFGR 0x3 (SW = PLL1) written before FLASH_ACR was written and read back",
            "FLASH_ACR never written in the boot the reads judge",
        ]
    );
    assert_eq!(
        judged(&r, &without(FLASH_ACR, false, FLASH_RENODE)),
        [
            "RCC.CFGR 0x3 (SW = PLL1) written before FLASH_ACR was written and read back",
            "FLASH_ACR never read back in the boot the reads judge",
        ]
    );
    assert_eq!(
        judged(&r, &replaced(FLASH_ACR, 0x11, 0x37)),
        ["FLASH_ACR 0x37 written, not the solved word 0x11"]
    );
    let mut acr_twice = good.clone();
    let at = position(&acr_twice, FLASH_ACR, 0x11);
    acr_twice.insert(at, w(FLASH_RENODE, FLASH_ACR, 0x11));
    assert_eq!(
        judged(&r, &acr_twice),
        ["FLASH_ACR written twice in a boot"]
    );
    let mut read_twice = good.clone();
    read_twice.insert(at + 2, rd(FLASH_RENODE, FLASH_ACR, 0x11));
    assert_eq!(
        judged(&r, &read_twice),
        ["FLASH_ACR read back twice in a boot"]
    );
    let mut late = good.clone();
    late.push(rd(FLASH_RENODE, FLASH_ACR, 0x11));
    assert_eq!(
        judged(&r, &late),
        [
            "FLASH_ACR read after RCC.CFGR 0x3 (SW = PLL1)",
            "FLASH_ACR read back twice in a boot"
        ]
    );
    // The flash controller's other words are the brown-out judge's.
    let mut optsr = good.clone();
    optsr.insert(1, rd(FLASH_RENODE, FLASH_OPTSR_CUR, 0x0406_AAFC));
    assert!(judged(&r, &optsr).is_empty());
}

#[test]
fn the_per_port_rcc_rule_leaves_the_clock_to_the_clock_rule() {
    let t = topology(CRYSTAL);
    let (label, port) = t.serial_ports.iter().next().unwrap();
    let pins: Vec<String> = [&port.tx, &port.rx]
        .iter()
        .map(|net| t.assignments[net.as_str()].pin.clone())
        .collect();
    let pins: Vec<&str> = pins.iter().map(String::as_str).collect();
    let rule = serial::rcc_rule(&t, &port.peripheral, &pins).unwrap();
    assert!(
        rule.selects.is_empty() && rule.spi_gates.is_empty(),
        "{label}"
    );
    assert!(!t.spi_controllers.is_empty(), "the fixture solves an SPI");
    let mut boot = sequence(&CRYSTAL_WORDS);
    boot.push(w(RCC_RENODE, D2CCIP1R, 0x4000));
    // Every SPI gate set with no select written: nothing for this rule.
    for p in t.spi_controllers.keys() {
        let f = aethelgard_svd_hal::cmsis::register_facts().unwrap();
        let (register, bit) = f.enable_bit(p).unwrap();
        let offset = f.layout("RCC").unwrap().offset(&register, 0).unwrap();
        boot.push(w(RCC_RENODE, offset, 1 << bit));
    }
    assert_eq!(serial::judge_rcc(&rule, &[boot]), Vec::<String>::new());
    // A prescaler is still not an enable register.
    let d2cfgr = vec![w(RCC_RENODE, 0x01C, 0x40)];
    assert_eq!(
        serial::judge_rcc(&rule, &[d2cfgr]),
        ["RCC.D2CFGR 0x40 written, not an enable register"]
    );
}

#[test]
fn a_board_without_an_hse_has_no_clock_rule_and_its_flash_judge_is_unchanged() {
    let null = topology("motors-pwm");
    assert!(null.clock.hse.is_none());
    assert_eq!(hse::clock_rule(&null).unwrap(), None);
    assert_eq!(boot::brown_out(&null).unwrap().clock_acr, None);
    let b = boot::brown_out(&null).unwrap();
    assert_eq!(
        b.judge_accesses(
            &[vec![
                rd(FLASH_RENODE, FLASH_OPTSR_CUR, 0x0406_AAFC),
                w(FLASH_RENODE, FLASH_ACR, 0x11)
            ]],
            0x0406_AAFC
        ),
        ["the firmware wrote the flash controller (32 bits at 0x0, value 0x11)"]
    );
    // On an HSE board FLASH_ACR is the clock rule's; OPTSR_CUR still the
    // brown-out judge's.
    let b = boot::brown_out(&topology(CRYSTAL)).unwrap();
    assert_eq!(b.clock_acr, Some(FLASH_ACR));
    let boot = sequence(&CRYSTAL_WORDS);
    let mut with_optsr = boot.clone();
    with_optsr.insert(0, rd(FLASH_RENODE, FLASH_OPTSR_CUR, 0x0406_AAFC));
    assert!(b.judge_accesses(&[with_optsr], 0x0406_AAFC).is_empty());
    assert_eq!(
        b.judge_accesses(&[boot], 0x0406_AAFC),
        ["the firmware read FLASH_OPTSR_CUR 0 times in the boot the reads judge, not once"]
    );
}

#[test]
fn only_an_hse_board_s_filter_keeps_the_rcc_s_polled_reads() {
    let cr = "[INFO] rcc: [cpu: 0x8000ABC] ReadUInt32 from 0x0 (ClockControl), returned 0x30025";
    let cfgr =
        "[INFO] rcc: [cpu: 0x8000ABE] ReadUInt32 from 0x10 (ClockConfiguration), returned 0x1B";
    let pll = "[INFO] rcc: [cpu: 0x8000AC0] ReadUInt32 from 0x28 (PLLClockSource), returned 0x52";
    let write = "[INFO] rcc: [cpu: 0x8000AC2] WriteUInt32 to 0x0 (ClockControl), value 0x10001";
    for line in [cr, cfgr, pll, write] {
        assert!(serial::access(line).is_some(), "{line}");
    }
    assert!(serial::is_unjudged_access(cr) && serial::is_unjudged_access(cfgr));
    assert!(!serial::is_unjudged_access_hse(cr) && !serial::is_unjudged_access_hse(cfgr));
    assert!(serial::is_unjudged_access(pll) && serial::is_unjudged_access_hse(pll));
    assert!(!serial::is_unjudged_access(write) && !serial::is_unjudged_access_hse(write));
    let usart = "[INFO] usart1: [cpu: 0x8000ABC] ReadUInt32 from 0x1C (InterruptAndStatus), returned 0x206000C0";
    assert!(serial::is_unjudged_access_hse(usart));
}

#[test]
fn the_platform_sets_hse_frequency_on_an_hse_board_only() {
    for (fixture, hz) in [(CRYSTAL, 16_000_000), (EXTERNAL, 8_000_000)] {
        let t = topology(fixture);
        let text = platform::emit(&t).unwrap();
        let block = format!(
            "\n// The HSE (AET-WO-4240b): the stock rcc's hseFrequency (8 MHz by default) is the solved\n// clock.hse_hz.\nrcc:\n    hseFrequency: {hz}\n"
        );
        assert_eq!(text.matches(&block).count(), 1, "{fixture}");
        assert_eq!(text.matches("\nrcc:").count(), 1, "{fixture}");
        assert!(
            text.contains(&format!(
                "at the solved clock's (PLL1 from the HSE at {hz} Hz, sys_ck 64000000 Hz, every prescaler 1, AET-WO-4240b)"
            )),
            "{fixture}"
        );
        // After the stock include, before the PWR's override.
        let at = text.find(&block).unwrap();
        assert!(
            text.find("using \"platforms/cpus/stm32h753.repl\"")
                .unwrap()
                < at
        );
        assert!(at < text.find("\npwr:\n").unwrap());
    }
    let null = platform::emit(&topology("motors-pwm")).unwrap();
    assert!(!null.contains("hseFrequency") && !null.contains("\nrcc:"));
    assert!(
        null.contains("the solved clock's (the reset clock, HSI at 64 MHz, RM0433 Rev 9 §8.7.2)")
    );
}

#[test]
fn an_hse_board_lists_what_the_twin_cannot_see() {
    let unobservable = boot::expected(&topology(CRYSTAL)).unwrap().unobservable;
    for x in [
        "RCC.CR.HSERDY",
        "RCC.CR.PLL1RDY",
        "RCC.CR.HSEBYP",
        "RCC.CR.CSSHSEON",
        "RCC.PLLCFGR",
        "RCC.D2CCIP1R.SPI123SEL",
        "RCC.D2CCIP1R.SPI45SEL",
        "RCC.D3CCIPR.SPI6SEL",
    ] {
        assert!(unobservable.iter().any(|u| u == x), "{x}: {unobservable:?}");
    }
    let null = boot::expected(&topology("motors-pwm"))
        .unwrap()
        .unobservable;
    assert!(!null.iter().any(|u| u.starts_with("RCC.")), "{null:?}");
    for x in [
        "HSERDY",
        "PLL1RDY",
        "HSEBYP",
        "PLLCFGR",
        "HSECSSON",
        "kernel select",
        "FLASH_ACR",
    ] {
        assert!(hse::HSE_RESIDUAL.contains(x), "{x}");
    }
}

#[test]
fn each_hse_boot_word_names_its_fault() {
    let name = |w| boot::hse_boot_word(w).map(|n| n.0);
    assert_eq!(name(0x2100_B0F4), Some("boot_hse_not_ready"));
    assert_eq!(name(0x2100_B0F5), Some("boot_pll1_not_locked"));
    assert_eq!(name(0x2100_B0F6), Some("boot_sw_not_taken"));
    assert_eq!(name(0x2100_B0F7), Some("boot_hse_css_fault"));
    assert_eq!(name(0x2100_B0F8), Some("boot_flash_acr_not_taken"));
    for other in [boot::BOOT_MAGIC, 0x2100_B0F1, 0x2100_B0F3, 0x2100_B0F9, 0] {
        assert_eq!(name(other), None, "{other:#x}");
    }
}
