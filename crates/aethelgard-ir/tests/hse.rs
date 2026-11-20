//! `AET-WO-4240a`: `mcu.hse`, its budget, the solver's H1 words and the
//! clock-tree judge, on a small synthetic `PinTable` (this crate may depend
//! on no other member). Predictions P4–P10 (`spike/4240a/PREDICTIONS.md`).

use std::collections::BTreeMap;

use aethelgard_ir::clock::{self, Budget, Hse};
use aethelgard_ir::power_parts::{self, Kind};
use aethelgard_ir::{
    IrValidationError, Pin, PinSource, PinTable, SolvedTopology, Specification, serial, silicon,
    solve,
};
use serde_json::{Value, json};

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const PART: &str = "ABM8AIG-16.000MHZ-12-2Z-T3";

/// The reference specification's declared board (`examples/sentinel-01-minimal.json`).
const BOARD: &str = r#"{"copper_layers": 2, "layers": [{"name": "F.Cu", "role": "signal", "net": null}, {"name": "B.Cu", "role": "signal", "net": null}], "stackup": {"finished_thickness_um": {"nominal": 1600, "min": 340, "max": 2000, "source": "AET-WO-1305 conditions: 1.6 mm declared; 0.34 mm = 2 x 70 um + 0.2 mm; 2.0 mm = 1.6 mm + 25 %"}, "dielectrics": [{"between": ["F.Cu", "B.Cu"], "min_thickness_um": 200, "source": "implied: the finished minimum 340 um less twice the copper maximum 70 um"}], "copper_max_um": {"value": 70, "source": "AET-WO-1305 condition: twice KiCad's default 35 um (2 oz)"}, "relative_permittivity_max_permille": {"value": 5400, "source": "the IPC-4101 FR-4 maximum, specified at 1 MHz"}}, "rules": {"clearance_um": 200, "track_width_um": 200, "via_drill_um": 300, "via_diameter_um": 600}}"#;

/// USART1 on PA9/PA10, I2C1 on PB8/PB9, and the HSE pins, which carry
/// I2C3 here so a reservation that fails would show.
fn table() -> PinTable {
    let pins: &[(&str, &[(&str, u8)])] = &[
        ("PA10", &[("USART1_RX", 7)]),
        ("PA9", &[("USART1_TX", 7)]),
        ("PB8", &[("I2C1_SCL", 4)]),
        ("PB9", &[("I2C1_SDA", 4)]),
        ("PH0", &[("I2C3_SDA", 4)]),
        ("PH1", &[("I2C3_SCL", 4)]),
    ];
    PinTable {
        part: "TEST_PART".into(),
        package: "TEST".into(),
        source: PinSource {
            repository: "test".into(),
            commit: "test".into(),
            files: BTreeMap::new(),
        },
        pins: pins
            .iter()
            .enumerate()
            .map(|(i, (name, afs))| {
                let pin = Pin {
                    position: i as u16 + 1,
                    alternate_functions: afs.iter().map(|(s, af)| ((*s).into(), *af)).collect(),
                    adc_channels: Default::default(),
                };
                ((*name).into(), pin)
            })
            .collect(),
    }
}

fn crystal(edit: impl FnOnce(&mut Value)) -> Value {
    let mut c = json!({"part": PART, "load_caps_femtofarads": [18000, 18000], "pull_ppm": 30,
        "temperature_celsius": {"min": -20, "max": 85}, "life_hours": 8000,
        "startup_max_us": 20000, "startup_source": "architect ruling 2026-11-20: 10x DS12117 Rev 10 Table 33 tSU(HSE) typical 2 ms; no maximum is published (Note 4)"});
    edit(&mut c);
    json!({ "crystal": c })
}

fn external(num: u64, den: u64, ppm: u32) -> Value {
    json!({"external": {"hz_num": num, "hz_den": den, "ppm": ppm,
        "source": "a declared bench oscillator (test)", "startup_max_us": 20000, "startup_source": "architect ruling 2026-11-20: 10x DS12117 Rev 10 Table 33 tSU(HSE) typical 2 ms; no maximum is published (Note 4)"}})
}

/// A board with an I2C bus and one CRSF port at `baud`, and `hse`.
fn spec_json(hse: Value, baud: u32) -> Value {
    let line = |net: &str| {
        json!({"net_name": net, "max_capacitance_femtofarads": 100000,
        "max_rise_time_nanoseconds": 300, "impedance_ohms": null})
    };
    json!({
        "schema_version": aethelgard_ir::SPECIFICATION_SCHEMA_VERSION,
        "accept_lifecycle": [],
        "acknowledge_unwitnessed": [],
        "project_name": "t",
        "board": serde_json::from_str::<Value>(BOARD).unwrap(),
        "mcu": {"part": "TEST_PART", "supply": "VDD_3V3", "hse": hse, "bor_level": 0,
            "core_supply": {"mode": "ldo", "source": "DS12117 Rev 10 sec. 3.5.1 p. 27"},
            "supply_source": {"connector": "J91", "pin": "1"}},
        "voltage_domains": [{"id": "VDD_3V3", "nominal_millivolts": 3300, "tolerance_permille": 50}],
        "i2c_buses": [{"id": "SENS", "peripheral": null, "speed_hz": 400000,
            "pull_up_ohms": 2200, "domain": "VDD_3V3",
            "sda": {"net": "I2C_SDA", "pin": null}, "scl": {"net": "I2C_SCL", "pin": null}}],
        "spi_buses": [], "i2c_devices": [], "spi_devices": [],
        "net_constraints": [line("I2C_SDA"), line("I2C_SCL")],
        "connectors": [
            {"reference": "J100", "part": "SM04B-GHS-TB", "standard": null, "pins": [
                {"pin": "1", "net": "RC_TX", "offboard": null},
                {"pin": "2", "net": "RC_RX", "offboard": null},
                {"pin": "3", "net": null, "offboard": null},
                {"pin": "4", "net": "GND", "offboard": null}]},
            {"reference": "J90", "part": "SM06B-SRSS-TB", "standard": null, "pins": [
                {"pin": "1", "net": "VDD_3V3", "offboard": null},
                {"pin": "2", "net": null, "offboard": null},
                {"pin": "3", "net": null, "offboard": null},
                {"pin": "4", "net": "SWDIO", "offboard": null},
                {"pin": "5", "net": "SWCLK", "offboard": null},
                {"pin": "6", "net": "GND", "offboard": null}]},
            {"reference": "J91", "part": "SM04B-GHS-TB", "standard": null, "pins": [
                {"pin": "1", "net": "VDD_3V3", "offboard": null},
                {"pin": "2", "net": null, "offboard": null},
                {"pin": "3", "net": null, "offboard": null},
                {"pin": "4", "net": "GND", "offboard": null}]}],
        "net_classes": [],
        "diff_pairs": [],
        "motor_outputs": [],
        "power": null,
        "serial_ports": [{"label": "RC", "peripheral": null,
            "tx": {"net": "RC_TX", "pin": null}, "rx": {"net": "RC_RX", "pin": null},
            "role": "rc_crsf",
            "framing": {"baud": baud, "data_bits": 8, "parity": "none", "stop_bits": 1, "inverted": false},
            "negotiated": null, "peer_deviation_ppm": 8002, "peer_deviation_source": "cited"}],
    })
}

fn parse(v: Value) -> Specification {
    serde_json::from_value(v).expect("a specification")
}

fn solved(v: Value) -> SolvedTopology {
    solve(&parse(v), &table(), HASH).expect("solved")
}

fn refused(v: Value) -> IrValidationError {
    parse(v).validate().expect_err("refused")
}

fn hse_kind(v: Value) -> &'static str {
    match refused(v) {
        e @ IrValidationError::Hse(_) => e.kind(),
        e => panic!("not an HSE refusal: {e}"),
    }
}

/// P4, P8, P9, P10: the 16 MHz crystal solves to `4240` §4.5's H1 words,
/// its budget is the row's cited terms plus the declared pull, written by
/// hand, and CRSF 416666, refused on the HSI, holds on it.
#[test]
fn the_16_mhz_crystal_solves_to_the_predicted_words_and_budget() {
    let t = solved(spec_json(crystal(|_| {}), 416_666));
    assert_eq!(t.validate(), Ok(()));
    let c = &t.clock;
    assert_eq!(
        (c.source.as_str(), c.sysclk_hz, c.hse_hz),
        ("PLL1", 64_000_000, 16_000_000)
    );
    let h = c.hse.as_ref().unwrap();
    assert_eq!(
        (h.pllckselr, h.pllcfgr, h.pll1divr),
        (0x0000_0052, 0x0003_0004, 0x0103_064F)
    );
    // tolerance 20 (option 2) + stability 50 (option Z) + aging 2 (first
    // year) + pull 30, each side.
    assert_eq!(
        (h.fast_ppm, h.slow_ppm),
        (20 + 50 + 2 + 30, 20 + 50 + 2 + 30)
    );
    assert_eq!(
        c.budget(),
        Budget {
            fast_ppm: 102,
            slow_ppm: 102
        }
    );
    let v = clock::judge_tree(c).unwrap().unwrap();
    assert_eq!(v.reference_hz, (16_000_000, 5));
    assert_eq!(v.range_hz, (2_000_000, 4_000_000));
    assert_eq!(v.vco_hz, (16_000_000 * 80, 5));
    assert_eq!((v.pll1_p_hz, v.pll1_q_hz), (64_000_000, Some(64_000_000)));
    let g = v.gm_crit.unwrap();
    // 4 · 70 Ω · (2π · 16 MHz)² · (2 + 12 pF)²: 554,643.3 nA/V with π
    // exact, 554,644 with (355/113)² and rounded up (computed apart, in
    // Python's `fractions`).
    assert_eq!(g.nano_amps_per_volt, 554_644);
    assert!(g.holds);
    assert_eq!(g.an2867_margin_hundredths_uncited, 270);
    // The receiver's term is the crystal's 102 ppm, not the HSI's 25,894.
    let rc = serial::check_silicon(c, &t.serial_ports["RC"]);
    assert_eq!((rc.brr, rc.hsi_ppm, rc.quantization_ppm), (154, 102, 2_596));
    // Table 407's 33,300 less the receiver's 2,698 and the peer's 8,002
    // (P10: near 22,600; `4240` M2 measured 22,652 at ±50).
    assert_eq!((rc.receiver_ppm, rc.worst_case_ppm), (2_698, 10_700));
    assert_eq!(rc.margin_ppm, Some(22_600));
    assert!(rc.holds);
    // The same port on the HSI is refused (`4240` M2: margin -3,125).
    assert_eq!(
        refused(spec_json(Value::Null, 416_666)).kind(),
        "SerialBaudTolerance"
    );
    // Every TIMINGR is judged at the crystal's extremes, and held.
    let i2c = &t.i2c_timings["I2C1"];
    assert!(silicon::check_i2c_on(c.budget(), "I2C1", i2c).holds);
    assert_eq!(
        i2c.timingr,
        [
            ("Fast400k".to_owned(), 0x10C7_123B),
            ("FastPlus1M".to_owned(), 0x00A2_0E2D),
            ("Standard100k".to_owned(), 0x50D3_2A3F),
        ]
        .into_iter()
        .collect()
    );
    // PH0 and PH1 are the crystal's: the bus goes elsewhere.
    assert_eq!(t.assignments["I2C_SDA"].pin, "PB9");
}

/// P5: an 8 MHz external clock (`4240` D7 (c)) solves to `DIVM1` 5,
/// `DIVN1` 159 and `DIVP1` 3, the reference 1.6 MHz on the medium VCO.
#[test]
fn an_8_mhz_external_clock_solves_to_divm1_5_divn1_159_divp1_3() {
    let t = solved(spec_json(external(8_000_000, 1, 50), 420_000));
    let h = t.clock.hse.as_ref().unwrap();
    assert_eq!(
        (h.pllckselr, h.pllcfgr, h.pll1divr),
        (0x0000_0052, 0x0003_0002, 0x0103_069F)
    );
    assert_eq!(h.pllckselr >> 4 & 0x3F, 5);
    assert_eq!(h.pll1divr & 0x1FF, 159);
    assert_eq!(h.pll1divr >> 9 & 0x7F, 3);
    assert_eq!(
        t.clock.budget(),
        Budget {
            fast_ppm: 50,
            slow_ppm: 50
        }
    );
    let v = clock::judge_tree(&t.clock).unwrap().unwrap();
    assert_eq!(
        (v.reference_hz, v.range_hz),
        ((8_000_000, 5), (1_000_000, 2_000_000))
    );
    assert_eq!(v.gm_crit, None);
    assert_eq!(t.validate(), Ok(()));
}

/// P7: 25 MHz / 3 (`4240` F-HZ) reaches no whole-Hz reference.
#[test]
fn an_8_33_mhz_external_clock_is_unreachable() {
    assert_eq!(
        refused(spec_json(external(25_000_000, 3, 50), 420_000)),
        IrValidationError::Hse(clock::HseRefusal::HseTreeUnreachable {
            hz_num: 25_000_000,
            hz_den: 3
        })
    );
}

/// Each declaration refusal by name (`AET-WO-4240a` §1.4, §5).
#[test]
fn a_budget_that_would_undercount_is_refused_by_name() {
    let c = |e: fn(&mut Value)| spec_json(crystal(e), 420_000);
    assert_eq!(
        hse_kind(c(|c| c["part"] = json!("LMR33630ADDA"))),
        "HsePartNotCrystal"
    );
    assert_eq!(
        hse_kind(c(|c| c["pull_ppm"] = json!(0))),
        "HseDeclarationZero"
    );
    assert_eq!(
        hse_kind(c(|c| c["life_hours"] = json!(0))),
        "HseDeclarationZero"
    );
    assert_eq!(
        hse_kind(c(|c| c["load_caps_femtofarads"] = json!([18000, 0]))),
        "HseDeclarationZero"
    );
    // The part's stability holds over -40 to +125 °C; one degree past either
    // end has no cited term.
    for t in [
        json!({"min": -41, "max": 85}),
        json!({"min": -20, "max": 126}),
    ] {
        let v = c(|_| {});
        let mut v = v;
        v["mcu"]["hse"]["crystal"]["temperature_celsius"] = t;
        assert_eq!(hse_kind(v), "HseTemperatureOutsidePart");
    }
    assert_eq!(
        parse(c(|c| c["life_hours"] = json!(8760))).validate(),
        Ok(())
    );
    assert_eq!(
        hse_kind(c(|c| c["life_hours"] = json!(8761))),
        "HseLifeBeyondAging"
    );
    assert_eq!(
        hse_kind(spec_json(external(8_000_000, 1, 0), 420_000)),
        "HseDeclarationZero"
    );
    assert_eq!(
        hse_kind(spec_json(external(3_999_999, 1, 50), 420_000)),
        "HseFrequencyOutOfRange"
    );
    assert_eq!(
        hse_kind(spec_json(external(50_000_001, 1, 50), 420_000)),
        "HseFrequencyOutOfRange"
    );
    assert_eq!(
        refused(spec_json(
            json!({"external": {"hz_num": 8_000_000, "hz_den": 1, "ppm": 50, "source": "", "startup_max_us": 20000, "startup_source": "architect ruling 2026-11-20: 10x DS12117 Rev 10 Table 33 tSU(HSE) typical 2 ms; no maximum is published (Note 4)"}}),
            420_000
        ))
        .kind(),
        "InvalidSource"
    );
}

/// `Gm_crit` at the row's maxima (`4240` §3.1): the 16 MHz CL 12 part
/// passes; the same family at 8 MHz, CL 18 pF and 500 Ω (the row `4240`
/// found in stock) fails DS12117's 1.5 mA/V, and a row that omits `C0`
/// cannot be parsed.
#[test]
fn gm_crit_refuses_the_8_mhz_cl_18_crystal() {
    let table = power_parts::table().unwrap();
    let Kind::Crystal(row) = &table.get(PART).unwrap().kind else {
        panic!("a crystal row");
    };
    assert!(clock::gm_crit(row).holds);
    let mut eight = row.clone();
    eight.frequency_hz.value = 8_000_000;
    eight.load_femtofarads.value = 18_000;
    eight.esr_max_milliohms.value = 500_000;
    let g = clock::gm_crit(&eight);
    // 2,021,295.0 nA/V with π exact (`4240` §3.1: 2.021 mA/V).
    assert_eq!(g.nano_amps_per_volt, 2_021_296);
    assert!(!g.holds);
    // The refusal, on a row of that crystal.
    let mut row = table.get(PART).unwrap().clone();
    row.kind = Kind::Crystal(eight);
    let c: Hse = serde_json::from_value(crystal(|_| {})).unwrap();
    let Hse::Crystal(c) = c else {
        panic!("a crystal")
    };
    assert_eq!(
        clock::crystal_budget(&c, &row).unwrap_err().kind(),
        "HseGmCritExceeded"
    );
    assert_eq!(
        clock::crystal_budget(&c, table.get(PART).unwrap()),
        Ok(Budget {
            fast_ppm: 102,
            slow_ppm: 102
        })
    );
    let omitted = power_parts::POWER_PARTS_JSON.replacen(
        "\"c0_max_femtofarads\": {\n            \"value\": 2000,",
        "\"c0_max_femtofarad\": {\n            \"value\": 2000,",
        1,
    );
    assert_ne!(omitted, power_parts::POWER_PARTS_JSON);
    assert!(power_parts::parse_power_parts(&omitted).is_err());
}

fn judged(edit: impl FnOnce(&mut aethelgard_ir::clock::SolvedHse)) -> Result<(), &'static str> {
    let mut t = solved(spec_json(crystal(|_| {}), 420_000));
    edit(t.clock.hse.as_mut().unwrap());
    t.validate().map_err(|e| e.kind())
}

/// The judge (`AET-WO-4240a` §1.5) on words the solver never emits, each
/// refused by name: a reference on a range's end (F-RGE), a VCO outside its
/// select's range, a select that excludes the reference, `DIVP1EN` 0, a
/// source other than HSE, a fractional PLL, a budget that is not the
/// declaration's, and words that decode to another clock.
#[test]
fn the_judge_refuses_each_tree_by_name() {
    assert_eq!(judged(|_| {}), Ok(()));
    // DIVM1 4: 4 MHz, PLL1RGE 01's top end; ×64 = 256 MHz; ÷4 = 64 MHz.
    assert_eq!(
        judged(|h| {
            h.pllckselr = 0b10 | 4 << 4;
            h.pll1divr = 1 << 24 | 3 << 16 | 3 << 9 | 63;
        }),
        Err("PllReferenceNotInsideRange")
    );
    // The same at PLL1RGE 10, whose bottom end it is.
    assert_eq!(
        judged(|h| {
            h.pllckselr = 0b10 | 4 << 4;
            h.pllcfgr = 0x0003_0008;
            h.pll1divr = 1 << 24 | 3 << 16 | 3 << 9 | 63;
        }),
        Err("PllReferenceNotInsideRange")
    );
    // DIVN1 ×50: 160 MHz, under the wide VCO's 192.
    assert_eq!(
        judged(|h| h.pll1divr = 1 << 24 | 1 << 16 | 1 << 9 | 49),
        Err("PllVcoOutOfRange")
    );
    // ×275: 880 MHz, over revision Y's 836 (F-REV).
    assert_eq!(
        judged(|h| h.pll1divr = 1 << 24 | 13 << 16 | 13 << 9 | 274),
        Err("PllVcoOutOfRange")
    );
    assert_eq!(judged(|h| h.pllcfgr |= 0b10), Err("PllVcoSelectMismatch"));
    assert_eq!(
        judged(|h| h.pllcfgr &= !(1 << 16)),
        Err("PllOutputDisabled")
    );
    assert_eq!(judged(|h| h.pllckselr &= !0b11), Err("PllWordsUndecodable"));
    // PLLSRC HSI with words whose every check on the HSE's 16 MHz holds
    // (3.2 MHz reference, ×60 = 192 MHz VCO) and which decode, on the HSI's
    // 64 MHz, to exactly 64 MHz (768 MHz ÷ 12): only the source tells.
    assert_eq!(
        judged(|h| {
            h.pllckselr = 5 << 4;
            h.pll1divr = 1 << 24 | 11 << 16 | 11 << 9 | 59;
        }),
        Err("PllWordsUndecodable")
    );
    assert_eq!(judged(|h| h.pllcfgr |= 1), Err("PllWordsUndecodable"));
    assert_eq!(judged(|h| h.fast_ppm = 72), Err("HseClockMismatch"));
    // DIVP1 ÷2: 128 MHz, which the clock does not state.
    assert_eq!(
        judged(|h| h.pll1divr = 1 << 24 | 3 << 16 | 1 << 9 | 79),
        Err("HseClockMismatch")
    );
    // An HSE clock without its tree, or the reverse, is refused too.
    let mut t = solved(spec_json(crystal(|_| {}), 420_000));
    t.clock.hse = None;
    assert_eq!(t.validate().map_err(|e| e.kind()), Err("UnsupportedClock"));
}

/// `AET-WO-4240b` Q1 (ruled): the start-up bound is declared on both modes,
/// with its source; a 0, a missing field, or a source that is not 1–200
/// printable ASCII bytes is refused.
#[test]
fn the_startup_bound_is_declared_and_never_zero() {
    let c = |e: fn(&mut Value)| spec_json(crystal(e), 420_000);
    assert_eq!(
        hse_kind(c(|c| c["startup_max_us"] = json!(0))),
        "HseDeclarationZero"
    );
    let mut x = external(8_000_000, 1, 50);
    x["external"]["startup_max_us"] = json!(0);
    assert_eq!(hse_kind(spec_json(x, 420_000)), "HseDeclarationZero");
    assert!(matches!(
        parse(c(|c| c["startup_source"] = json!(""))).validate(),
        Err(e) if !matches!(e, IrValidationError::Hse(_))
    ));
    for key in ["startup_max_us", "startup_source"] {
        let mut v = c(|_| {});
        v["mcu"]["hse"]["crystal"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert!(serde_json::from_value::<Specification>(v).is_err(), "{key}");
    }
    let t = solved(c(|_| {}));
    let h = t.clock.hse.as_ref().unwrap();
    use clock::Startup as _;
    assert_eq!(h.declared.startup_max_us(), 20_000);
}

/// `AET-WO-4240b` Q6 (ruled): the solver writes Table 17's VOS2 row for
/// `hclk` 64 MHz at the crystal's fastest, `LATENCY` 1 and `WRHIGHFREQ` `01`
/// (RM0433 Rev 9 p. 160, ]55; 110] MHz), written by hand here; the rule
/// judges the written word, a word below the row and a reserved bit by name,
/// and the copy check sees an edited word.
#[test]
fn flash_acr_is_table_17_s_vos2_row_and_judged() {
    let t = solved(spec_json(crystal(|_| {}), 420_000));
    assert_eq!(t.clock.hse.as_ref().unwrap().flash_acr, 0x0000_0011);
    let x = solved(spec_json(external(8_000_000, 1, 50), 420_000));
    assert_eq!(x.clock.hse.as_ref().unwrap().flash_acr, 0x0000_0011);
    let v = aethelgard_ir::vos::flash_at_reset(&t.clock);
    assert_eq!(
        (v.latency, v.wrhighfreq, v.needed, v.holds),
        (1, 0b01, Some((1, 0b01)), true)
    );
    assert_eq!(
        judged(|h| h.flash_acr = 0x10),
        Err("FlashWaitStatesBelowTable")
    );
    assert_eq!(
        judged(|h| h.flash_acr = 0x01),
        Err("FlashWaitStatesBelowTable")
    );
    assert_eq!(
        judged(|h| h.flash_acr = 0x51),
        Err("FlashWaitStatesBelowTable")
    );
    // A word above the row holds the rule, but is not the solver's.
    assert_eq!(judged(|h| h.flash_acr = 0x37), Err("HseCopyNotSolved"));
}

/// `AET-WO-4240b` Q3 (ruled; `4240` D6): on an HSE tree the SPI kernels are
/// the reset selects', `pll1_q_ck` (SPI1–3) and the APB clock (SPI4–6), each
/// 64 MHz here and judged at the crystal's budget; the HSI-derived kernels
/// keep the HSI's.
#[test]
fn the_spi_kernels_of_an_hse_tree_are_the_reset_selects() {
    use aethelgard_ir::SpiKernel;
    let t = solved(spec_json(crystal(|_| {}), 420_000));
    for (p, k) in [
        ("SPI1", SpiKernel::Pll1Q),
        ("SPI4", SpiKernel::Pclk),
        ("SPI6", SpiKernel::Pclk),
    ] {
        assert_eq!(
            silicon::spi_kernel_on(p, k, &t.clock),
            Some(64_000_000),
            "{p}"
        );
        assert_eq!(
            silicon::spi_kernel_budget(k, &t.clock),
            Budget {
                fast_ppm: 102,
                slow_ppm: 102
            }
        );
        assert_eq!(silicon::SpiSelect::of(p).unwrap().code(k), Some(0));
    }
    assert_eq!(
        silicon::spi_kernel_on("SPI1", SpiKernel::Pclk, &t.clock),
        None
    );
    assert_eq!(
        silicon::spi_kernel_on("SPI4", SpiKernel::Pll1Q, &t.clock),
        None
    );
    let reset = aethelgard_ir::SolvedClock::reset();
    assert_eq!(
        silicon::spi_kernel_on("SPI1", SpiKernel::Pll1Q, &reset),
        None
    );
    assert_eq!(
        silicon::spi_kernel_budget(SpiKernel::PerCk, &t.clock),
        Budget::HSI
    );
}

/// `AET-WO-4240a` F3 (`4240b` §1.3): a topology's copied declaration. Words
/// the judge holds (3.2 MHz × 120 = 384 MHz, ÷ 6 = 64 MHz exactly, `pll1_q`
/// 64 MHz) but which are not the solver's choice for the declaration the
/// topology carries are refused by name, as are the budget's sides moved
/// together with the declaration's pull while its other fields stay.
#[test]
fn a_topology_whose_clock_is_not_its_declaration_s_is_refused() {
    let words =
        |h: &mut aethelgard_ir::clock::SolvedHse| h.pll1divr = 1 << 24 | 5 << 16 | 5 << 9 | 119;
    let mut t = solved(spec_json(crystal(|_| {}), 420_000));
    words(t.clock.hse.as_mut().unwrap());
    assert_eq!(clock::judge_tree(&t.clock).map(|_| ()), Ok(()));
    assert_eq!(judged(words), Err("HseCopyNotSolved"));
    // The declaration and the budget edited together, the words left: the
    // budget follows the declaration, so only the re-solve could tell, and it
    // agrees; this is the residue a topology alone cannot close.
    assert_eq!(
        judged(|h| {
            let Hse::Crystal(c) = &mut h.declared else {
                panic!()
            };
            c.pull_ppm = 1;
            (h.fast_ppm, h.slow_ppm) = (73, 73);
        }),
        Ok(())
    );
}

/// The VOS limits are judged at the HSE's own fastest (`AET-WO-4240a`
/// §1.5): an external clock declared ±20 % runs `pclk` past Scale 2's 75
/// MHz at its fastest, which the HSI's +1.5 % never does.
#[test]
fn the_vos_limits_take_the_hse_s_fastest() {
    let mut t = solved(spec_json(external(8_000_000, 1, 50), 420_000));
    let h = t.clock.hse.as_mut().unwrap();
    h.declared = serde_json::from_value(json!({"external": {"hz_num": 8_000_000, "hz_den": 1,
        "ppm": 200_000, "source": "a declared bench oscillator (test)", "startup_max_us": 20000, "startup_source": "architect ruling 2026-11-20: 10x DS12117 Rev 10 Table 33 tSU(HSE) typical 2 ms; no maximum is published (Note 4)"}}))
    .unwrap();
    (h.fast_ppm, h.slow_ppm) = (200_000, 200_000);
    assert_eq!(t.validate().map_err(|e| e.kind()), Err("VosLimitExceeded"));
}

/// The declaration round-trips, and an unknown mode, a second mode or an
/// unknown key is refused at parsing.
#[test]
fn the_hse_object_is_one_mode_and_nothing_else() {
    let v = crystal(|_| {});
    let h: Hse = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(serde_json::to_value(&h).unwrap(), v);
    for bad in [
        json!({"oscillator": {}}),
        json!({"crystal": v["crystal"], "external": external(8_000_000, 1, 50)["external"]}),
        crystal(|c| c["extra"] = json!(1)),
        json!({}),
        json!(16_000_000),
    ] {
        assert!(serde_json::from_value::<Hse>(bad.clone()).is_err(), "{bad}");
    }
}

/// Adversarial A1: a budget side of 100 % or more is refused by name, never
/// a wrapped or panicking `10⁶ − ppm`.
#[test]
fn an_unbounded_budget_is_refused_not_a_panic() {
    for ppm in [1_000_000u64, u64::from(u32::MAX)] {
        assert_eq!(
            hse_kind(spec_json(external(8_000_000, 1, ppm as u32), 420_000)),
            "HseBudgetUnbounded"
        );
    }
    let v = spec_json(crystal(|c| c["pull_ppm"] = json!(u32::MAX)), 420_000);
    assert_eq!(hse_kind(v), "HseBudgetUnbounded");
    // Just under: refused by the serial check, by name.
    let v = refused(spec_json(external(8_000_000, 1, 999_999), 420_000));
    assert_eq!(v.kind(), "SerialBaudTolerance");
}

/// Adversarial A2: a key given twice anywhere under `mcu` is refused at
/// parsing, as on `main`.
#[test]
fn a_duplicate_key_under_mcu_is_refused() {
    let text = spec_json(crystal(|_| {}), 420_000).to_string();
    for (once, twice) in [
        (r#""mode":"ldo""#, r#""mode":"ldo","mode":"bypass""#),
        (r#""pull_ppm":30"#, r#""pull_ppm":30,"pull_ppm":1"#),
        (
            r#""supply_source":{"connector":"J91","pin":"1"}"#,
            r#""supply_source":{"connector":"J91","pin":"1","pin":"3"}"#,
        ),
    ] {
        assert_eq!(text.matches(once).count(), 1, "{once}");
        let e = serde_json::from_str::<Specification>(&text.replace(once, twice)).unwrap_err();
        assert!(e.to_string().contains("duplicate field"), "{twice}: {e}");
    }
}

/// Adversarial A3: the load capacitors must be equal and, in series, at
/// most the crystal's CL: stray capacitance only adds to them.
#[test]
fn load_caps_that_overload_the_crystal_are_refused() {
    let caps = |c: Value| spec_json(crystal(move |v| v["load_caps_femtofarads"] = c), 420_000);
    assert_eq!(
        hse_kind(caps(json!([100_000, 100_000]))),
        "HseLoadCapsUnfit"
    );
    assert_eq!(hse_kind(caps(json!([18_000, 15_000]))), "HseLoadCapsUnfit");
    assert_eq!(hse_kind(caps(json!([24_002, 24_002]))), "HseLoadCapsUnfit");
    // 24 pF and 24 pF are 12 pF in series: CL exactly, with no stray.
    assert_eq!(parse(caps(json!([24_000, 24_000]))).validate(), Ok(()));
}

/// Adversarial A4: every bit the judge does not read is clear, `DIVR1` is
/// not 0, and `pll1_q_ck` is held to SPI1–3's kernel limit at Scale 2.
#[test]
fn the_judge_reads_every_bit_of_the_three_words() {
    assert_eq!(judged(|h| h.pllcfgr |= 1 << 18), Err("PllWordsUndecodable"));
    assert_eq!(judged(|h| h.pllcfgr |= 1 << 25), Err("PllWordsUndecodable"));
    assert_eq!(
        judged(|h| h.pllckselr |= 1 << 2),
        Err("PllWordsUndecodable")
    );
    assert_eq!(
        judged(|h| h.pllckselr |= 4 << 12),
        Err("PllWordsUndecodable")
    );
    assert_eq!(
        judged(|h| h.pll1divr |= 1 << 31),
        Err("PllWordsUndecodable")
    );
    assert_eq!(
        judged(|h| h.pll1divr &= !(0x7F << 24)),
        Err("PllWordsUndecodable")
    );
    // DIVQ1 ÷1: pll1_q_ck is the 256 MHz VCO, over Table 60's 150 MHz.
    assert_eq!(
        judged(|h| h.pll1divr = 1 << 24 | 3 << 9 | 79),
        Err("VosLimitExceeded")
    );
}

/// Adversarial A5: an SPI kernel is `hsi_ker_ck` on an HSE board too, so the
/// VOS rule judges it at the HSI's fastest; the buses at the crystal's.
#[test]
fn the_vos_rule_judges_spi_kernels_at_the_hsi_s_fastest() {
    let mut t = solved(spec_json(crystal(|_| {}), 420_000));
    t.spi_controllers.insert(
        "SPI1".into(),
        serde_json::from_value(json!({"kernel": "per_ck", "kernel_hz": 64_000_000,
            "declared_hz": 20_000_000, "mbr": 1, "achieved_hz": 16_000_000,
            "achieved_max_hz": 16_239_744}))
        .unwrap(),
    );
    let v = aethelgard_ir::vos::judge_generated(&t);
    let fastest = |name: &str| {
        v.clocks
            .iter()
            .find(|c| c.clock == name)
            .unwrap()
            .fastest_hz
    };
    assert_eq!(fastest("SPI1 kernel"), Some(64_958_976));
    assert_eq!(fastest("fPCLK1"), Some(64_006_528));
}
