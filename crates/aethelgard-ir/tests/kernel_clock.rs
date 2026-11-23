//! `AET-WO-4240e`: the kernel-clock switch rule. Every enabled
//! kernel-clocked peripheral has a running kernel clock at its start, by
//! RM0433 Rev 9 §8.5.10 (pp. 371–372): a mux leaves its reset input only
//! while both inputs run. The topologies are synthetic: only the fields the
//! rule reads are filled.

use aethelgard_ir::clock::{SolvedAdcClock, SolvedHse, SolvedKernelSwitch};
use aethelgard_ir::kernel_clock::WordFault as W;
use aethelgard_ir::kernel_clock::{self, DeadInput, KernelClockRefusal as R, NotRunning, Unknown};
use aethelgard_ir::vos::Vos;
use aethelgard_ir::{SolvedClock, SolvedTopology, SpiKernel};
use serde_json::{Value, json};

/// A topology on the reset tree at VOS Scale 2 with the SPI controllers
/// given (peripheral, kernel) and nothing else enabled; `edit` adds more.
fn topology(spi: &[(&str, &str)], edit: impl FnOnce(&mut Value)) -> SolvedTopology {
    let controllers: serde_json::Map<String, Value> = spi
        .iter()
        .map(|(p, k)| {
            let c = json!({"kernel": k, "kernel_hz": 64000000, "declared_hz": 20000000, "mbr": 1,
                           "achieved_hz": 16000000, "achieved_max_hz": 16239744});
            ((*p).to_owned(), c)
        })
        .collect();
    let mut v = json!({
        "schema_version": aethelgard_ir::SOLVED_SCHEMA_VERSION,
        "specification_sha256": "0".repeat(64),
        "assignments": {}, "gpio": {}, "adc_inputs": {}, "i2c_ceilings": {}, "i2c_speeds": {},
        "analog_lines": {}, "i2c_devices": {}, "spi_devices": {}, "spi_lines": {},
        "clock": serde_json::to_value(SolvedClock::reset()).unwrap(),
        "motor_outputs": {}, "power": null, "serial_ports": {},
        "spi_controllers": controllers, "i2c_timings": {},
        "bor_level": 0, "core_supply": "ldo", "mcu": "STM32H753ZITx", "pin_speeds": {},
        "foreign_clock": null
    });
    edit(&mut v);
    serde_json::from_value(v).unwrap()
}

fn spi(p: &str, k: &str) -> SolvedTopology {
    topology(&[(p, k)], |_| {})
}

fn switch(pllckselr: u32, pllcfgr: u32, pll1divr: u32) -> Option<SolvedKernelSwitch> {
    Some(SolvedKernelSwitch {
        pllckselr,
        pllcfgr,
        pll1divr,
    })
}

/// The ruled words: HSI / 5 = 12.8 MHz (`PLL1RGE` `11`, wide VCO), × 16 =
/// 204.8 MHz, Q / 2 = 102.4 MHz, `DIVQ1EN` set. Across the HSI's band
/// (62,342,784 to 64,958,976 Hz) the reference is 12,468,556 to 12,991,795
/// Hz (inside 8–16 MHz and 2–16 MHz), the VCO 199,496,908 to 207,868,723 Hz
/// (inside 192–836 MHz) and Q at most 103,934,361 Hz (under 300 MHz).
fn valid() -> Option<SolvedKernelSwitch> {
    switch(0x50, 0x0002_000C, 0x0101_020F)
}

/// An HSE tree whose `RCC_PLLCFGR` is `pllcfgr`; the rule reads no other
/// field of it.
fn hse(pllcfgr: u32) -> Option<SolvedHse> {
    serde_json::from_value(json!({
        "declared": {"external": {"hz_num": 8000000, "hz_den": 1, "ppm": 50, "source": "test",
                     "startup_max_us": 20000, "startup_source": "test"}},
        "fast_ppm": 50, "slow_ppm": 50, "pllckselr": 0x52, "pllcfgr": pllcfgr,
        "pll1divr": 0x0101_049F, "flash_acr": 0x13
    }))
    .unwrap()
}

const DIVP1EN: u32 = 1 << 16;
const DIVQ1EN: u32 = 1 << 17;

fn kinds(r: &[R]) -> Vec<(&'static str, String)> {
    r.iter()
        .map(|r| (r.kind(), r.peripheral().to_owned()))
        .collect()
}

/// The fault SPI1 on `per_ck` meets with these words at `vos`, read off the
/// dead-switch refusal; `None` when the topology passes.
fn fault(ks: Option<SolvedKernelSwitch>, vos: Vos) -> Option<W> {
    let mut t = spi("SPI1", "per_ck");
    t.clock.kernel_switch = ks;
    t.clock.vos = vos;
    let r = kernel_clock::check(&t);
    match r.as_slice() {
        [] => None,
        [
            R::KernelClockDeadSwitch {
                peripheral, dead, ..
            },
        ] if peripheral == "SPI1" => match dead.as_slice() {
            [
                DeadInput {
                    input: "pll1_q_ck",
                    why: NotRunning::Pll1WordsInvalid { fault },
                },
            ] => Some(fault.clone()),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
}

// ---------------------------------------------------------------- (1), (2)

/// (1) The scar (`4240e` §1.1): SPI1 on `per_ck` on an HSI board with no
/// switch. `SPI123SEL` resets to `pll1_q_ck`, PLL1 never runs, so the move to
/// `per_ck` never happens and SPI1 has no kernel clock.
#[test]
fn the_scar_spi1_per_ck_with_no_switch_is_a_dead_switch() {
    let r = kernel_clock::check(&spi("SPI1", "per_ck"));
    assert_eq!(
        r,
        [R::KernelClockDeadSwitch {
            peripheral: "SPI1".into(),
            selector: "SPI123SEL",
            from: "pll1_q_ck",
            to: "per_ck",
            dead: vec![DeadInput {
                input: "pll1_q_ck",
                why: NotRunning::Pll1NotStarted,
            }],
            pages:
                "RM0433 Rev 9 §8.5.10, pp. 371–372; RM0433 Rev 9 §8.7.20, p. 414; Figure 51, p. 359"
                    .into(),
        }]
    );
    let text = r[0].to_string();
    for word in [
        "SPI1",
        "SPI123SEL",
        "pll1_q_ck",
        "per_ck",
        "pp. 371–372",
        "p. 414",
    ] {
        assert!(text.contains(word), "{word}: {text}");
    }
    let j = serde_json::to_value(&r[0]).unwrap();
    assert_eq!(j["kind"], "KernelClockDeadSwitch", "{j}");
    assert_eq!(j["dead"][0]["why"]["why"], "pll1_not_started", "{j}");
}

/// (2) The ruled fix: the same board with the ruled words passes, at every
/// scale that states a Q maximum the words stay under.
#[test]
fn the_ruled_kernel_switch_passes() {
    for vos in [Vos::Scale1, Vos::Scale2, Vos::Scale3] {
        assert_eq!(fault(valid(), vos), None, "{vos}");
    }
}

// ---------------------------------------------------------------- (3)

/// (3) Each invalid word: PLL1 does not run, so `pll1_q_ck` is dead across
/// the select and SPI1 gets no clock. Each word differs from a passing one
/// in one fault only.
#[test]
fn pllsrc_divm1_and_divq1en_are_read() {
    let s2 = Vos::Scale2;
    assert_eq!(
        fault(switch(0x51, 0x0002_000C, 0x0101_020F), s2),
        Some(W::PllsrcNotHsi { pllsrc: 1 })
    );
    assert_eq!(
        fault(switch(0x52, 0x0002_000C, 0x0101_020F), s2),
        Some(W::PllsrcNotHsi { pllsrc: 2 })
    );
    assert_eq!(
        fault(switch(0x00, 0x0002_000C, 0x0101_020F), s2),
        Some(W::Divm1Zero)
    );
    assert_eq!(
        fault(switch(0x50, 0x0000_000C, 0x0101_020F), s2),
        Some(W::Divq1enClear)
    );
}

/// The reference at the band's edges against `PLL1RGE`: 64 MHz / 8 = 8 MHz
/// nominal is inside both `10` (4–8) and `11` (8–16), but the HSI's fastest
/// (8,119,872 Hz) leaves `10` and its slowest (7,792,848 Hz) leaves `11`.
#[test]
fn the_reference_is_judged_at_both_band_edges_against_pll1rge() {
    let s2 = Vos::Scale2;
    assert!(matches!(
        fault(switch(0x80, 0x0002_0008, 0x0101_021F), s2),
        Some(W::ReferenceOutsidePll1rge {
            rge: 0b10,
            fastest_hz: 8_119_872,
            max_hz: 8_000_000,
            ..
        })
    ));
    assert!(matches!(
        fault(switch(0x80, 0x0002_000C, 0x0101_021F), s2),
        Some(W::ReferenceOutsidePll1rge {
            rge: 0b11,
            slowest_hz: 7_792_848,
            min_hz: 8_000_000,
            ..
        })
    ));
    // DIVM1 10 → 6.4 MHz nominal, 6.23 to 6.50 MHz: inside `10`; × 32.
    assert_eq!(fault(switch(0xA0, 0x0002_0008, 0x0101_021F), s2), None);
}

/// `fPLL_IN` per VCO range, apart from `PLL1RGE`: 64 / 34 = 1.88 MHz is
/// inside `PLL1RGE` `00` (1–2 MHz) and below the wide VCO's 2 MHz; 64 / 21 =
/// 3.05 MHz is inside `01` (2–4 MHz) and above the medium VCO's 2 MHz.
#[test]
fn the_reference_is_judged_against_fpll_in_for_its_vco_range() {
    let s2 = Vos::Scale2;
    assert!(matches!(
        fault(switch(0x220, 0x0002_0000, 0x0101_026D), s2),
        Some(W::ReferenceOutsideFpllIn {
            vco: "wide",
            min_hz: 2_000_000,
            ..
        })
    ));
    assert!(matches!(
        fault(switch(0x150, 0x0002_0006, 0x0101_0263), s2),
        Some(W::ReferenceOutsideFpllIn {
            vco: "medium",
            max_hz: 2_000_000,
            ..
        })
    ));
}

/// The VCO across the band: × 65 is 832 MHz nominal and 844,466,688 Hz at
/// the fastest, over revision Y's 836 MHz though under revision V's 960;
/// × 15 is 192 MHz nominal and 187,028,352 Hz at the slowest. The medium
/// range: 64 / 40 × 262 = 419.2 MHz nominal, 425,481,292 Hz at the fastest.
#[test]
fn the_vco_is_judged_across_the_band_on_the_stricter_revision() {
    let s2 = Vos::Scale2;
    assert!(matches!(
        fault(switch(0x50, 0x0002_000C, 0x0103_0240), s2),
        Some(W::VcoOutOfRange {
            vco: "wide",
            fastest_hz: 844_466_688,
            max_hz: 836_000_000,
            ..
        })
    ));
    assert!(matches!(
        fault(switch(0x50, 0x0002_000C, 0x0101_020E), s2),
        Some(W::VcoOutOfRange {
            vco: "wide",
            slowest_hz: 187_028_352,
            min_hz: 192_000_000,
            ..
        })
    ));
    assert!(matches!(
        fault(switch(0x280, 0x0002_0002, 0x0103_0305), s2),
        Some(W::VcoOutOfRange {
            vco: "medium",
            max_hz: 420_000_000,
            ..
        })
    ));
    // The medium VCO at 1.6 MHz × 200 = 320 MHz, Q / 2 = 160 MHz: passes.
    assert_eq!(fault(switch(0x280, 0x0002_0002, 0x0101_02C7), s2), None);
}

/// `pll1_q_ck` at the band's top against `fPLL_Q_OUT` at the clock's scale:
/// each word is under the maximum at nominal and over it at the fastest.
#[test]
fn pll1_q_is_judged_at_the_fastest_against_the_scale_maximum() {
    let q_over = |ks, vos| match fault(ks, vos) {
        Some(W::QOverMaximum { max_hz, .. }) => Some(max_hz),
        other => panic!("{other:?}"),
    };
    // Wide, Q = VCO: 64 / 6 × 28 = 298.67 MHz nominal, 303.1 MHz fastest.
    let w300 = switch(0x60, 0x0002_000C, 0x0100_021B);
    assert_eq!(q_over(w300.clone(), Vos::Scale2), Some(300_000_000));
    assert_eq!(fault(w300, Vos::Scale1), None);
    // Wide, Q = VCO / 2: 12.8 × 31 / 2 = 198.4 MHz nominal, 201.4 fastest.
    let w200 = switch(0x50, 0x0002_000C, 0x0101_021E);
    assert_eq!(q_over(w200.clone(), Vos::Scale3), Some(200_000_000));
    assert_eq!(fault(w200, Vos::Scale2), None);
    // Wide, Q = VCO / 2: 12.8 × 62 / 2 = 396.8 MHz nominal, 402.7 fastest.
    let w400 = switch(0x50, 0x0002_000C, 0x0101_023D);
    assert_eq!(q_over(w400, Vos::Scale1), Some(400_000_000));
    // Medium, Q = VCO: 1.6 × 130 = 208 MHz nominal, 211.1 MHz fastest:
    // over the medium range's 210 MHz at VOS2, where the wide allows 300.
    let m210 = switch(0x280, 0x0002_0002, 0x0100_0281);
    assert_eq!(q_over(m210, Vos::Scale2), Some(210_000_000));
    // No table states Q at VOS0.
    assert_eq!(
        fault(valid(), Vos::Scale0),
        Some(W::NoQMaximumAtScale { vos: Vos::Scale0 })
    );
}

// ---------------------------------------------------------------- (4), (5)

/// (4) `pll1_q_ck` as the kernel on an HSI board: with the switch, PLL1 is
/// stopped after the window, a PLL stopped while a peripheral uses it; with
/// none, PLL1 never runs. Either way no switch is needed, and the input does
/// not run at SPI1's start.
#[test]
fn pll1_q_on_an_hsi_board_is_not_running() {
    let mut t = spi("SPI2", "pll1_q");
    t.clock.kernel_switch = valid();
    let r = kernel_clock::check(&t);
    assert_eq!(
        r,
        [R::KernelClockNotRunning {
            peripheral: "SPI2".into(),
            selector: "SPI123SEL",
            input: "pll1_q_ck",
            why: NotRunning::Pll1StoppedAfterWindow,
            pages:
                "RM0433 Rev 9 §8.5.10, pp. 371–372; RM0433 Rev 9 §8.7.20, p. 414; Figure 51, p. 359"
                    .into(),
        }]
    );
    assert!(
        r[0].to_string()
            .contains("a PLL stopped while a peripheral uses it")
    );
    t.clock.kernel_switch = None;
    assert!(matches!(
        kernel_clock::check(&t).as_slice(),
        [R::KernelClockNotRunning {
            input: "pll1_q_ck",
            why: NotRunning::Pll1NotStarted,
            ..
        }]
    ));
}

/// (5) An HSE board: PLL1 runs from the HSE tree throughout, so
/// `pll1_q_ck` runs exactly when `DIVQ1EN` is set in its words, both for
/// SPI1–3 left on it and for a move to `per_ck`.
#[test]
fn an_hse_board_runs_pll1_q_when_divq1en_is_set() {
    let mut t = topology(&[("SPI1", "pll1_q"), ("SPI3", "per_ck")], |_| {});
    t.clock.hse = hse(DIVP1EN | DIVQ1EN | 0b0010);
    assert_eq!(kernel_clock::check(&t), []);
    t.clock.hse = hse(DIVP1EN | 0b0010);
    let r = kernel_clock::check(&t);
    let disabled = NotRunning::OutputDisabled { field: "DIVQ1EN" };
    assert!(
        matches!(
            r.as_slice(),
            [
                R::KernelClockNotRunning { peripheral: p1, input: "pll1_q_ck", why: w1, .. },
                R::KernelClockDeadSwitch { peripheral: p3, from: "pll1_q_ck", to: "per_ck", dead, .. },
            ] if p1 == "SPI1" && p3 == "SPI3" && *w1 == disabled
                && dead == &[DeadInput { input: "pll1_q_ck", why: disabled.clone() }]
        ),
        "{r:?}"
    );
}

// ---------------------------------------------------------------- (6), (8)

/// (6) SPI4–6 on `hsi_ker_ck` or their APB clock: both inputs run from
/// reset, so the move happens with no switch.
#[test]
fn spi4_to_6_pass_on_both_their_kernels() {
    for k in ["hsi_ker_ck", "pclk"] {
        let t = topology(&[("SPI4", k), ("SPI5", k), ("SPI6", k)], |_| {});
        assert_eq!(kernel_clock::check(&t), [], "{k}");
    }
}

/// (8) I2C and USART/UART selects are never written: their reset input, an
/// APB clock, runs from reset.
#[test]
fn i2c_and_usart_pass_on_their_reset_inputs() {
    let t = topology(&[], |v| {
        let i2c = json!({"kernel_hz": 64000000, "cr1": 0, "timingr": {}});
        v["i2c_timings"] = json!({"I2C1": i2c, "I2C2": i2c, "I2C3": i2c, "I2C4": i2c});
        let mut ports = serde_json::Map::new();
        for p in [
            "USART1", "USART2", "USART3", "UART4", "UART5", "USART6", "UART7", "UART8",
        ] {
            ports.insert(p.into(), port(p));
        }
        v["serial_ports"] = Value::Object(ports);
    });
    assert_eq!(kernel_clock::check(&t), []);
}

fn port(peripheral: &str) -> Value {
    json!({"index": 0, "peripheral": peripheral, "tx": "T", "rx": "R", "role": "console",
           "framing": {"baud": 115200, "data_bits": 8, "parity": "none", "stop_bits": 1, "inverted": false},
           "negotiated": null, "peer_deviation_ppm": 5000, "kernel_hz": 64000000, "brr": 556,
           "actual_millibaud": 115107913, "error_ppm": -799})
}

// ---------------------------------------------------------------- (7), (11)

fn with_adc(inputs: Value, sense: Option<Value>) -> SolvedTopology {
    topology(&[], |v| {
        v["adc_inputs"] = inputs;
        if let Some(s) = sense {
            v["power"] = json!({"domains": {}, "input_steady_milliamps": 0,
                                "input_fault_milliamps": 0, "conditions": [], "sense": s});
        }
    })
}

fn adc_input(peripheral: &str) -> Value {
    json!({"pin": "PC0", "peripheral": peripheral, "channel": "INP10"})
}

/// (7) An ADC on `CKMODE` ≠ `00` takes the synchronous clock, which runs;
/// on `00` it takes `ADCSEL`'s reset `pll2_p_ck`, which nothing starts; with
/// no `clock.adc` it cannot be judged; a `ckmode` over 3 is no code.
#[test]
fn an_adc_runs_on_ckmode_and_not_on_pll2_p() {
    let mut t = with_adc(json!({"CUR": adc_input("ADC1")}), None);
    for ckmode in 1..=3 {
        t.clock.adc = Some(SolvedAdcClock { ckmode });
        assert_eq!(kernel_clock::check(&t), [], "{ckmode}");
    }
    t.clock.adc = Some(SolvedAdcClock { ckmode: 0 });
    assert!(matches!(
        kernel_clock::check(&t).as_slice(),
        [R::KernelClockNotRunning {
            peripheral,
            selector: "ADCSEL",
            input: "pll2_p_ck",
            why: NotRunning::NeverStarted,
            ..
        }] if peripheral == "ADC1"
    ));
    t.clock.adc = Some(SolvedAdcClock { ckmode: 4 });
    assert!(matches!(
        kernel_clock::check(&t).as_slice(),
        [R::KernelClockUnknown {
            why: Unknown::CkmodeNotACode { ckmode: 4 },
            ..
        }]
    ));
    t.clock.adc = None;
    let r = kernel_clock::check(&t);
    assert!(matches!(
        r.as_slice(),
        [R::KernelClockUnknown { peripheral, why: Unknown::AdcClockAbsent, .. }] if peripheral == "ADC1"
    ));
    assert_eq!(
        serde_json::to_value(&r[0]).unwrap()["kind"],
        "KernelClockUnknown"
    );
}

/// (11) A `power.sense` net's ADC input enables no ADC this rule judges; a
/// pin device's input beside it does.
#[test]
fn a_power_sense_input_is_not_an_enabled_adc() {
    let sense = json!({"VBAT": {"pin": "PC0", "position": 1, "channels": ["ADC3_INP10"]}});
    let t = with_adc(json!({"VBAT": adc_input("ADC3")}), Some(sense.clone()));
    assert_eq!(kernel_clock::check(&t), []);
    let t = with_adc(
        json!({"VBAT": adc_input("ADC3"), "CUR": adc_input("ADC2")}),
        Some(sense),
    );
    assert_eq!(
        kinds(&kernel_clock::check(&t)),
        [("KernelClockUnknown", "ADC2".to_owned())]
    );
}

// ---------------------------------------------------------------- (9), (10)

/// (9) A `kernel_switch` beside an HSE tree is refused first, by name, even
/// when no peripheral needs PLL1.
#[test]
fn a_kernel_switch_beside_an_hse_tree_is_refused() {
    let mut t = spi("SPI4", "pclk");
    t.clock.hse = hse(DIVP1EN | DIVQ1EN | 0b0010);
    t.clock.kernel_switch = valid();
    let r = kernel_clock::check(&t);
    assert!(matches!(
        r.as_slice(),
        [R::KernelClockUnknown { peripheral, why: Unknown::KernelSwitchBesideHse, .. }] if peripheral == "PLL1"
    ));
    t.clock.kernel_switch = None;
    assert_eq!(kernel_clock::check(&t), []);
}

/// (10) An enabled peripheral with no row, an SPI kernel no input of its
/// select gives, and an SPI peripheral with pins and no controller are each
/// refused by name; a timer takes no mux and passes.
#[test]
fn a_peripheral_the_table_cannot_judge_is_unknown() {
    let pin = |p: &str| json!({"pin": "PA0", "peripheral": p, "signal": format!("{p}_X"), "alternate_function": 1});
    let t = topology(&[], |v| {
        v["assignments"] =
            json!({"A": pin("TIM1"), "B": pin("TIM17"), "C": pin("LPTIM1"), "D": pin("SPI2")});
    });
    let r = kernel_clock::check(&t);
    assert!(
        matches!(
            r.as_slice(),
            [
                R::KernelClockUnknown { peripheral: lp, why: Unknown::NoRow, .. },
                R::KernelClockUnknown { peripheral: s2, why: Unknown::NoSpiController, .. },
            ] if lp == "LPTIM1" && s2 == "SPI2"
        ),
        "{r:?}"
    );
    for (p, k, kernel, selector) in [
        ("SPI1", "hsi_ker_ck", SpiKernel::HsiKerCk, "SPI123SEL"),
        ("SPI1", "pclk", SpiKernel::Pclk, "SPI123SEL"),
        ("SPI4", "per_ck", SpiKernel::PerCk, "SPI45SEL"),
        ("SPI6", "pll1_q", SpiKernel::Pll1Q, "SPI6SEL"),
    ] {
        let r = kernel_clock::check(&spi(p, k));
        assert!(
            matches!(
                r.as_slice(),
                [R::KernelClockUnknown { why: Unknown::SpiKernelNotOnSelect { kernel: got, selector: s }, .. }]
                    if *got == kernel && *s == selector
            ),
            "{p} {k}: {r:?}"
        );
    }
}

/// Refusals come one per peripheral, sorted by peripheral, whatever the
/// order the topology's maps give.
#[test]
fn refusals_are_one_per_peripheral_in_name_order() {
    let mut t = topology(
        &[("SPI3", "per_ck"), ("SPI1", "pll1_q"), ("SPI4", "per_ck")],
        |v| {
            v["adc_inputs"] = json!({"Z": adc_input("ADC3")});
        },
    );
    t.clock.adc = Some(SolvedAdcClock { ckmode: 0 });
    assert_eq!(
        kinds(&kernel_clock::check(&t)),
        [
            ("KernelClockNotRunning", "ADC3".to_owned()),
            ("KernelClockNotRunning", "SPI1".to_owned()),
            ("KernelClockDeadSwitch", "SPI3".to_owned()),
            ("KernelClockUnknown", "SPI4".to_owned()),
        ]
    );
}

/// Every place a topology enables a peripheral is read: an I2C controller,
/// a serial port, an SPI or I2C device and a pin, each on a peripheral the
/// table has no row for (or an SPI with no controller), is refused by name.
#[test]
fn every_source_of_an_enabled_peripheral_is_read() {
    let i2c = json!({"kernel_hz": 64000000, "cr1": 0, "timingr": {}});
    let dev = |p: &str| json!({"index": 0, "part": "X", "bus": "b", "peripheral": p, "address": 8});
    let spi_dev =
        |p: &str| json!({"index": 0, "part": "X", "bus": "b", "peripheral": p, "cs": "CS"});
    let cases: [(&str, &str, Value); 4] = [
        ("i2c_timings", "I2C5", json!({"I2C5": i2c})),
        ("serial_ports", "LPUART1", json!({"P": port("LPUART1")})),
        ("i2c_devices", "I2C6", json!({"U1": dev("I2C6")})),
        ("spi_devices", "SPI3", json!({"U2": spi_dev("SPI3")})),
    ];
    for (key, peripheral, value) in cases {
        let t = topology(&[], |v| v[key] = value);
        assert_eq!(
            kinds(&kernel_clock::check(&t)),
            [("KernelClockUnknown", peripheral.to_owned())],
            "{key}"
        );
    }
}
