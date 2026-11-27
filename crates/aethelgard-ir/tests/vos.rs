//! `AET-WO-4240v` §1.3: the voltage-scaling rule. Every bus clock and every
//! enabled kernel is judged at its fastest against DS12117 Rev 10 Table 111
//! and RM0433 Rev 9 Table 60, at each scale; the declaration's refusals.

use aethelgard_ir::vos::{
    self, BUS_LIMITS_REV_V, Clock, CoreSupplyMode, Family, VOS_LIMIT_AT_BOUNDARY, Vos,
};
use aethelgard_ir::{SolvedClock, SolvedTopology, Specification};
use serde_json::{Value, json};

/// HSI's fastest, by the rule's own copy: 64 MHz × (1 + 14,984 ppm), rounded
/// up.
const FAST_64: u64 = 64_958_976;

/// A topology with TIM1, USART1 and UART4, I2C1 and I2C4, and SPI1, SPI4
/// and SPI6, on the reset tree: only the fields the rule reads are filled.
fn topology(vos: Vos) -> SolvedTopology {
    let port = |peripheral: &str| {
        json!({"index": 0, "peripheral": peripheral, "tx": "T", "rx": "R", "role": "console",
               "framing": {"baud": 115200, "data_bits": 8, "parity": "none", "stop_bits": 1, "inverted": false},
               "negotiated": null, "peer_deviation_ppm": 5000, "kernel_hz": 64000000, "brr": 556,
               "actual_millibaud": 115107913, "error_ppm": -799})
    };
    let spi = |kernel: &str| {
        json!({"kernel": kernel, "kernel_hz": 64000000, "declared_hz": 20000000, "mbr": 1,
               "achieved_hz": 16000000, "achieved_max_hz": 16239744})
    };
    let i2c = json!({"kernel_hz": 64000000, "cr1": 0, "timingr": {}});
    let mut clock = serde_json::to_value(SolvedClock::reset()).unwrap();
    clock["vos"] = serde_json::to_value(vos).unwrap();
    serde_json::from_value(json!({
        "schema_version": aethelgard_ir::SOLVED_SCHEMA_VERSION,
        "specification_sha256": "0".repeat(64),
        "assignments": {"M1": {"pin": "PE9", "peripheral": "TIM1", "signal": "TIM1_CH1", "alternate_function": 1}},
        "gpio": {}, "adc_inputs": {}, "i2c_ceilings": {}, "i2c_speeds": {}, "analog_lines": {}, "i2c_devices": {},
        "spi_devices": {}, "spi_lines": {}, "clock": clock, "motor_outputs": {}, "power": null,
        "serial_ports": {"A": port("USART1"), "B": port("UART4")},
        "spi_controllers": {"SPI1": spi("per_ck"), "SPI4": spi("hsi_ker_ck"), "SPI6": spi("hsi_ker_ck")},
        "i2c_timings": {"I2C1": i2c.clone(), "I2C4": i2c},
        "bor_level": 0, "core_supply": "ldo", "mcu": "STM32H753ZITx", "pin_speeds": {}, "foreign_clock": null
    }))
    .unwrap()
}

fn over(v: &vos::Verdict) -> Vec<String> {
    v.clocks
        .iter()
        .filter(|c| c.holds == Some(false))
        .map(|c| c.clock.clone())
        .collect()
}

/// The reset tree at VOS3 (the reset scale) is refused, naming every APB
/// clock and each enabled USART, I2C and SPI4–6 kernel at 64,958,976 Hz
/// against 50 MHz; fCPU, fHCLK, TIM and SPI1–3 hold (200, 100, 100, 100 MHz).
#[test]
fn the_reset_tree_at_vos3_is_refused_naming_each_clock_over_50_mhz() {
    let v = vos::judge_generated(&topology(Vos::Scale3));
    assert_eq!(
        over(&v),
        [
            "fPCLK1",
            "fPCLK2",
            "fPCLK3",
            "fPCLK4",
            "SPI4 kernel",
            "SPI6 kernel",
            "I2C1 kernel",
            "I2C4 kernel",
            "USART1 kernel",
            "UART4 kernel"
        ]
    );
    for c in v.clocks.iter().filter(|c| c.holds == Some(false)) {
        assert_eq!(
            (c.fastest_hz, c.limit_hz),
            (Some(FAST_64), Some(50_000_000)),
            "{c:?}"
        );
    }
    assert!(!v.holds());
    assert_eq!(v.conditions, [BUS_LIMITS_REV_V]);
    // Through `validate`, by the first clock's name.
    let e = topology(Vos::Scale3).validate().unwrap_err();
    assert_eq!(
        (e.kind(), e.id()),
        ("VosLimitExceeded", Some("fPCLK1")),
        "{e}"
    );
    assert!(
        e.to_string().contains("64958976 Hz") && e.to_string().contains("50000000 Hz"),
        "{e}"
    );
}

/// At VOS2 every clock holds: 75 MHz for the APB clocks and the USART, I2C
/// and SPI4–6 kernels, 150 MHz for TIM and SPI1–3, and `FLASH_ACR` at reset
/// against Table 17's 1 WS row (]55; 110] MHz).
#[test]
fn the_reset_tree_at_vos2_holds_with_its_margins() {
    let v = vos::judge_generated(&topology(Vos::Scale2));
    assert!(v.holds(), "{v:?}");
    for c in &v.clocks {
        let limit = match c.family {
            Family::Cpu => 300_000_000,
            Family::Tim | Family::Spi123 | Family::Hclk => 150_000_000,
            _ => 75_000_000,
        };
        assert_eq!(
            (c.fastest_hz, c.limit_hz),
            (Some(FAST_64), Some(limit)),
            "{c:?}"
        );
    }
    assert_eq!(v.clocks.len(), 6 + 1 + 3 + 2 + 2);
    assert_eq!(v.flash.needed, Some((1, 0b01)));
    assert_eq!((v.flash.latency, v.flash.wrhighfreq), (7, 0b11));
}

/// Any scale but 2 on the reset tree that passes the rule is `UnsupportedClock`:
/// the only tree this engine generates is the reset tree at Scale 2.
#[test]
fn a_tree_at_another_scale_within_its_limits_is_unsupported() {
    for vos in [Vos::Scale0, Vos::Scale1] {
        assert!(vos::judge_generated(&topology(vos)).holds());
        let mut t = topology(vos);
        t.spi_controllers.clear();
        t.i2c_timings.clear();
        t.serial_ports.clear();
        t.assignments.clear();
        assert_eq!(t.validate().unwrap_err().kind(), "UnsupportedClock");
    }
}

fn one(family: Family, hz: u64, vos: Vos) -> Option<bool> {
    let c = [Clock {
        name: "x".into(),
        family,
        nominal_hz: hz,
    }];
    vos::judge(vos, &c, vos::hsi_fastest)[0].holds
}

/// The smallest nominal whose fastest exceeds `limit`.
fn first_over(limit: u64) -> u64 {
    let mut n = limit * 1_000_000 / 1_014_984;
    while vos::hsi_fastest(n).unwrap() <= limit {
        n += 1;
    }
    n
}

/// Each family at VOS2: the largest nominal whose fastest is at or under its
/// maximum passes, and the next fails; and every cell of the table against
/// its document, each column (`AET-WO-4240v` §1.3, re-read at source).
#[test]
fn each_family_holds_exactly_to_its_maximum_at_every_scale() {
    let table: [(Family, [u64; 4]); 12] = [
        (Family::Cpu, [480, 400, 300, 200]),
        (Family::Hclk, [240, 200, 150, 100]),
        (Family::Pclk, [120, 100, 75, 50]),
        (Family::Tim, [240, 200, 150, 100]),
        (Family::Spi123, [200, 200, 150, 100]),
        (Family::Spi45, [125, 100, 75, 50]),
        (Family::Spi6, [125, 100, 75, 50]),
        (Family::I2c123, [125, 100, 75, 50]),
        (Family::I2c4, [125, 100, 75, 50]),
        (Family::Usart16, [125, 100, 75, 50]),
        (Family::Usart234578, [125, 100, 75, 50]),
        (Family::Lpuart1, [125, 100, 75, 50]),
    ];
    for (family, mhz) in table {
        for (vos, m) in [Vos::Scale0, Vos::Scale1, Vos::Scale2, Vos::Scale3]
            .into_iter()
            .zip(mhz)
        {
            let limit = m * 1_000_000;
            assert_eq!(
                vos::limit_hz(family, vos).map(|l| l.0),
                Some(limit),
                "{family:?} {vos}"
            );
            let n = first_over(limit);
            assert_eq!(one(family, n - 1, vos), Some(true), "{family:?} {vos}");
            assert_eq!(one(family, n, vos), Some(false), "{family:?} {vos}");
        }
    }
}

/// The boundary at the fastest: fastest = maximum passes, maximum + 1 Hz
/// fails (an exact fastest by a `fastest` of identity).
#[test]
fn fastest_equal_to_the_maximum_passes_and_one_hz_more_fails() {
    let c = |hz| {
        [Clock {
            name: "fPCLK1".into(),
            family: Family::Pclk,
            nominal_hz: hz,
        }]
    };
    let at = vos::judge(Vos::Scale2, &c(75_000_000), Some)[0].holds;
    let past = vos::judge(Vos::Scale2, &c(75_000_001), Some)[0].holds;
    assert_eq!((at, past), (Some(true), Some(false)));
}

/// #240 S14: a nominal at or under the maximum whose fastest is over it is
/// refused. 74 MHz is within 75 MHz, and 74 MHz × 1.014984 is not.
#[test]
fn a_nominal_within_the_maximum_is_refused_when_its_fastest_is_not() {
    assert_eq!(one(Family::Pclk, 74_000_000, Vos::Scale2), Some(false));
    assert_eq!(vos::hsi_fastest(74_000_000), Some(75_108_816));
}

/// A clock whose fastest is unknown is not judged, never passed.
#[test]
fn an_unknown_fastest_is_not_judged() {
    let c = [Clock {
        name: "x".into(),
        family: Family::Pclk,
        nominal_hz: 1,
    }];
    assert_eq!(vos::judge(Vos::Scale2, &c, |_| None)[0].holds, None);
}

/// ArduPilot's RCC registers at the end of its run (`AET-WO-4400g` §2.1).
fn ardupilot() -> aethelgard_ir::motor::RccRegisters {
    aethelgard_ir::motor::RccRegisters::from_values([
        0x3F00_3025,
        0x801B,
        0x48,
        0x440,
        0x40_8040,
        0x0109_0231,
    ])
}

/// D-4: ArduPilot's RCC registers at the end of its run (`AET-WO-4400g`
/// §2.1, measured) at VOS1 (`STM32_VOS_SCALE1`, `stm32h7_mcuconf.h` L109):
/// every bus clock and both timer kernels are at VOS1's maximum, so each is
/// over it at the HSI's fastest, and each is a named condition, never a
/// refusal; the USARTs at 72 MHz hold; `fPCLK4` is not judged.
#[test]
fn ardupilot_at_vos1_names_each_clock_at_the_boundary() {
    let regs = ardupilot();
    let decoded = SolvedClock::from_rcc(&regs, 0, Vos::Scale1).unwrap();
    let usarts = aethelgard_ir::serial::PERIPHERALS
        .iter()
        .map(|p| ((*p).to_owned(), 72_000_000))
        .collect();
    let v = vos::judge_foreign(&regs, &decoded, &usarts, None, None);
    let named: Vec<&str> = v
        .conditions
        .iter()
        .filter(|c| c.starts_with(VOS_LIMIT_AT_BOUNDARY))
        .map(|c| c.split(' ').nth(1).unwrap())
        .collect();
    assert_eq!(
        named,
        [
            "fCPU", "fHCLK", "fPCLK1", "fPCLK2", "fPCLK3", "APB1", "APB2"
        ]
    );
    assert!(v.conditions.contains(&BUS_LIMITS_REV_V.to_owned()));
    assert_eq!(v.not_judged, ["fPCLK4: D3PPRE is not read"]);
    assert!(
        v.clocks
            .iter()
            .filter(|c| c.clock.ends_with("kernel") && c.clock.contains("ART"))
            .all(|c| c.holds == Some(true))
    );
}

/// `AET-WO-4412` D2 (ruled 2026-11-26): an HSE tree is judged at the
/// declared crystal's fast ppm. Sentinel-03's RCC registers as unmodified
/// ArduPilot leaves them on its 16 MHz crystal (measured, `spike/4412`
/// `arm-1`: `PLLSRC` `10`, PLL1 400 MHz), with Sentinel-03's solved budget
/// (±102 ppm): every clock but `fPCLK4` is judged, none is left "not
/// declared", and every one of the 16 (`fPCLK4` at `D3PPRE` as read, D2
/// 11:51Z), each at VOS1's maximum (the USARTs' 100 MHz kernels too, which
/// the HSI tree's 72 MHz kept under theirs), is a named condition at the
/// crystal's fastest, reported and never refused.
/// With no deviation declared the same tree judges nothing.
#[test]
fn an_hse_tree_is_judged_at_the_crystals_fast_ppm() {
    let regs = aethelgard_ir::motor::RccRegisters::from_values([
        0x3F03_3025,
        0x0000_801B,
        0x48,
        0x440,
        0x0040_2022,
        0x0109_0263,
    ]);
    let decoded = SolvedClock::from_rcc(&regs, 16_000_000, Vos::Scale1).unwrap();
    let usarts: std::collections::BTreeMap<String, u64> = aethelgard_ir::serial::PERIPHERALS
        .iter()
        .map(|p| ((*p).to_owned(), 100_000_000))
        .collect();
    let crystal = aethelgard_ir::clock::Budget {
        fast_ppm: 102,
        slow_ppm: 102,
    };
    // `D3PPRE` unread: `fPCLK4` named, the other 15 judged.
    let unread = vos::judge_foreign(&regs, &decoded, &usarts, Some(crystal), None);
    assert_eq!(
        unread.not_judged,
        ["fPCLK4: D3PPRE is not read"],
        "{unread:?}"
    );
    assert_eq!(unread.clocks.len(), 15, "{unread:?}");
    // Read, as ArduPilot leaves it (`STM32_D3PPRE4_DIV2`, `stm32h7_mcuconf.h`
    // L294: `D3PPRE` 100, `RCC_D3CFGR` 0x40): all 16 judged, `fPCLK4` at
    // 100 MHz, VOS1's maximum.
    let v = vos::judge_foreign(&regs, &decoded, &usarts, Some(crystal), Some(0x40));
    assert!(v.not_judged.is_empty(), "{v:?}");
    assert_eq!(v.clocks.len(), 16, "{v:?}");
    let p4 = v.clocks.iter().find(|c| c.clock == "fPCLK4").unwrap();
    assert_eq!(p4.nominal_hz, 100_000_000);
    for c in &v.clocks {
        assert_eq!(
            c.fastest_hz,
            Some(crystal.fastest_hz(c.nominal_hz)),
            "{c:?}"
        );
    }
    let named: Vec<&str> = v
        .conditions
        .iter()
        .filter(|c| c.starts_with(VOS_LIMIT_AT_BOUNDARY))
        .map(|c| c.split(' ').nth(1).unwrap())
        .collect();
    assert_eq!(
        named,
        [
            "fCPU", "fHCLK", "fPCLK1", "fPCLK2", "fPCLK3", "fPCLK4", "APB1", "APB2", "UART4",
            "UART5", "UART7", "UART8", "USART1", "USART2", "USART3", "USART6"
        ],
        "{v:?}"
    );
    assert!(v.clocks.iter().all(|c| c.holds == Some(false)), "{v:?}");
    let unjudged = vos::judge_foreign(&regs, &decoded, &usarts, None, None);
    assert!(unjudged.clocks.is_empty(), "{unjudged:?}");
    assert_eq!(unjudged.not_judged.len(), 16, "{unjudged:?}");
    assert!(unjudged.not_judged[1].contains("PLLSRC 10"), "{unjudged:?}");
}

/// A tree not on the HSI judges nothing: no deviation is declared for it.
#[test]
fn a_foreign_tree_off_the_hsi_is_not_judged() {
    let mut regs = ardupilot();
    // `SW` and `SWS` CSI (RM0433 Rev 9 §8.7.7), `CSIRDY` set (§8.7.2).
    regs.cfgr = (regs.cfgr & !0b11_1111) | (0b001 << 3) | 0b001;
    regs.cr |= 1 << 8;
    let decoded = SolvedClock::from_rcc(&regs, 0, Vos::Scale1).unwrap();
    let v = vos::judge_foreign(&regs, &decoded, &Default::default(), None, None);
    assert!(v.clocks.is_empty() && v.conditions.is_empty(), "{v:?}");
    assert!(
        v.not_judged.iter().any(|n| n.contains("the tree is CSI")),
        "{v:?}"
    );
}

/// RM0433 Rev 9 Table 34 and §6.8.4: `ldo` writes `LDOEN` 1, `BYPASS` 0,
/// `bypass` the reverse, and `SCUEN` 0 in both; the read-back judges
/// `LDOEN` and `BYPASS`. `PWR_D3CR` Scale 2 is `0x8000` (§6.8.6).
#[test]
fn the_supply_and_vos_words_are_rm0433_s() {
    assert_eq!(vos::cr3_supply_bits(CoreSupplyMode::Ldo), 0b010);
    assert_eq!(vos::cr3_supply_bits(CoreSupplyMode::Bypass), 0b001);
    assert_eq!(vos::CR3_SUPPLY_MASK, 0b111);
    assert_eq!(vos::CR3_SUPPLY_CONFIG_MASK, 0b011);
    assert_eq!(vos::D3CR_VOS_SCALE2, 0x0000_8000);
    assert_eq!(vos::D3CR_VOS_MASK, 0x0000_C000);
}

fn spec_value() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/sentinel-01.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn parse(v: Value) -> Result<Specification, String> {
    serde_json::from_value::<Specification>(v).map_err(|e| e.to_string())
}

/// `mcu.core_supply` is required, and every malformed form is refused by
/// name: missing, `null`, an unknown or mixed-case mode, and an empty,
/// 201-byte or non-printable `source`.
#[test]
fn a_malformed_core_supply_is_refused_by_name() {
    let mut v = spec_value();
    v["mcu"].as_object_mut().unwrap().remove("core_supply");
    assert!(
        parse(v)
            .unwrap_err()
            .contains("missing field `core_supply`")
    );
    let mut v = spec_value();
    v["mcu"]["core_supply"] = Value::Null;
    assert!(parse(v).unwrap_err().contains("CoreSupply"));
    for mode in ["LDO", "Ldo", "smps", ""] {
        let mut v = spec_value();
        v["mcu"]["core_supply"]["mode"] = json!(mode);
        let e = parse(v).unwrap_err();
        assert!(
            e.contains("unknown variant") && e.contains("`ldo` or `bypass`"),
            "{e}"
        );
    }
    for source in [
        "".to_owned(),
        "x".repeat(201),
        "DS12117\tRev 10".to_owned(),
        " x".to_owned(),
    ] {
        let mut v = spec_value();
        v["mcu"]["core_supply"]["source"] = json!(source);
        let e = parse(v).unwrap().validate().unwrap_err();
        assert_eq!(e.kind(), "InvalidSource", "{e}");
        assert!(e.to_string().contains("mcu.core_supply.source"), "{e}");
    }
    let mut v = spec_value();
    v["mcu"]["core_supply"]["source"] = json!("x".repeat(200));
    assert_eq!(parse(v).unwrap().validate(), Ok(()));
}

/// `bypass` is a true declaration: `validate` and `solve` accept it.
#[test]
fn bypass_validates() {
    let mut v = spec_value();
    v["mcu"]["core_supply"]["mode"] = json!("bypass");
    assert_eq!(parse(v).unwrap().validate(), Ok(()));
}

/// A version-17 or version-18 file (`2304b`'s) is refused by its version
/// with the new key, and without it by the key; a version-19 file without it
/// is refused by the key.
#[test]
fn an_older_file_is_refused_with_or_without_core_supply() {
    for version in [17, 18] {
        let mut v = spec_value();
        v["schema_version"] = json!(version);
        let e = parse(v.clone()).unwrap().validate().unwrap_err();
        assert_eq!(e.kind(), "UnsupportedSchemaVersion", "{version}");
        v["mcu"].as_object_mut().unwrap().remove("core_supply");
        assert!(parse(v).is_err(), "{version}");
    }
}

/// An older solved file is refused, never given a `vos` or a supply: one
/// carrying both is refused by its version, one without either by the key.
#[test]
fn an_older_solved_file_is_refused_not_defaulted() {
    let mut v = serde_json::to_value(topology(Vos::Scale2)).unwrap();
    v["schema_version"] = json!(11);
    let t: SolvedTopology = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(t.validate().unwrap_err().kind(), "UnsupportedSchemaVersion");
    let mut no_supply = v.clone();
    no_supply.as_object_mut().unwrap().remove("core_supply");
    let e = serde_json::from_value::<SolvedTopology>(no_supply).unwrap_err();
    assert!(e.to_string().contains("missing field `core_supply`"), "{e}");
    v["clock"].as_object_mut().unwrap().remove("vos");
    let e = serde_json::from_value::<SolvedTopology>(v).unwrap_err();
    assert!(e.to_string().contains("missing field `vos`"), "{e}");
}

/// A foreign clock declared without `vos` is refused by the key.
#[test]
fn a_foreign_clock_without_vos_is_refused() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../aethelgard-cli/tests/fixtures/sentinel-02-own.json"
    );
    let mut v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(parse(v.clone()).unwrap().validate(), Ok(()));
    v["mcu"]["foreign_clock"]
        .as_object_mut()
        .unwrap()
        .remove("vos");
    assert!(parse(v).unwrap_err().contains("missing field `vos`"));
}

/// D-7 at its fastest (adversarial pass A5): at VOS2 a 54.5 MHz AXI clock is
/// in Table 17's 0 WS row at nominal and in the 1 WS row at its fastest
/// (55,316,628 Hz), so judged at nominal the row would be `(0, 00)`.
#[test]
fn flash_wait_states_are_judged_at_the_fastest_axi_clock() {
    let mut c = SolvedClock::reset();
    c.sysclk_hz = 54_500_000;
    let f = vos::flash_at_reset(&c);
    assert_eq!(f.hclk_fastest_hz, 55_316_628);
    assert_eq!(f.needed, Some((1, 0b01)));
    assert!(f.holds);
    // Past the table (above 225 MHz at VOS2) no row needs anything: refused.
    c.sysclk_hz = 225_000_000;
    let f = vos::flash_at_reset(&c);
    assert_eq!((f.needed, f.holds), (None, false));
}

/// `AET-WO-4412` D2: `RCC_D3CFGR.D3PPRE` (bits 6:4) as its divider, each row
/// of RM0433 Rev 9 §8.7.10 (p. 397); the other bits are not read.
#[test]
fn d3ppre_decodes_each_row_of_rm0433() {
    use aethelgard_ir::motor::{RCC_D3CFGR, d3ppre};
    assert_eq!(RCC_D3CFGR, ("RCC_D3CFGR", 0x020));
    for (field, divider) in [
        (0b000, 1),
        (0b011, 1),
        (0b100, 2),
        (0b101, 4),
        (0b110, 8),
        (0b111, 16),
    ] {
        assert_eq!(d3ppre(field << 4), divider, "{field:03b}");
        assert_eq!(
            d3ppre((field << 4) | 0xFFFF_FF8F),
            divider,
            "{field:03b} among other bits"
        );
    }
}
