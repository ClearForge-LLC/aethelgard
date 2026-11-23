//! `AET-WO-4400b`: the generator on the three CI boards' topologies, and on
//! `serial-dshot-9600-open` (`AET-WO-4300j` D8, every motor pull `open`,
//! solved from `spike/4300j/open/`), byte for byte; every emitted keyword
//! one ArduPilot `Copter-4.7.2` reads; every
//! refusal by name; and the generator calls none of the rules the pinout
//! check judges by (`RUL-20`).

use std::collections::BTreeSet;

use aethelgard_hwdef::{Refusal, generate};
use aethelgard_ir::SolvedTopology;
use serde_json::{Value, json};

const BOARDS: [(&str, &str, &str); 4] = [
    (
        "sentinel-01",
        include_str!("fixtures/sentinel-01.topology.json"),
        include_str!("fixtures/sentinel-01.hwdef.dat"),
    ),
    (
        "sentinel-02",
        include_str!("fixtures/sentinel-02.topology.json"),
        include_str!("fixtures/sentinel-02.hwdef.dat"),
    ),
    (
        "serial-dshot-9600",
        include_str!("fixtures/serial-dshot-9600.topology.json"),
        include_str!("fixtures/serial-dshot-9600.hwdef.dat"),
    ),
    (
        "serial-dshot-9600-open",
        include_str!("fixtures/serial-dshot-9600-open.topology.json"),
        include_str!("fixtures/serial-dshot-9600-open.hwdef.dat"),
    ),
];

fn topology(json: &str) -> SolvedTopology {
    serde_json::from_str(json).unwrap()
}

fn sentinel_01() -> Value {
    serde_json::from_str(BOARDS[0].1).unwrap()
}

fn refused(v: &Value) -> Refusal {
    let t: SolvedTopology = serde_json::from_value(v.clone()).unwrap();
    generate(&t).expect_err("the edited topology must be refused")
}

/// Each CI board's file is exactly the committed one, and generating twice
/// gives the same bytes.
#[test]
fn each_board_generates_its_committed_file() {
    for (name, t, want) in BOARDS {
        let t = topology(t);
        let got = generate(&t).unwrap().text;
        assert_eq!(got, want, "{name}");
        assert_eq!(generate(&t).unwrap().text, got, "{name}");
    }
}

/// Every keyword the generator emits is one ArduPilot's parser reads at the
/// tag (an unknown keyword is stored and never read, `chibios_hwdef.py:2885`),
/// each with where it is read; every `define` names a macro the tag reads;
/// and every pin token is one of ArduPilot's.
#[test]
fn every_emitted_keyword_is_one_ardupilot_reads() {
    // (keyword, where ArduPilot Copter-4.7.2 reads it)
    const KEYWORDS: [(&str, &str); 12] = [
        ("MCU", "chibios_hwdef.py:2894-2897"),
        ("OSCILLATOR_HZ", "chibios_hwdef.py:885"),
        ("FLASH_SIZE_KB", "chibios_hwdef.py:1022"),
        ("APJ_BOARD_ID", "hwdef.py:132-139"),
        ("FLASH_RESERVE_START_KB", "chibios_hwdef.py:1026-1032"),
        ("STORAGE_FLASH_PAGE", "chibios_hwdef.py:834, 1056-1059"),
        (
            "STM32_ST_USE_TIMER",
            "chibios_hwdef.py:1004-1006 (an STM32_ line is written as a define)",
        ),
        ("SERIAL_ORDER", "chibios_hwdef.py:1682"),
        ("I2C_ORDER", "chibios_hwdef.py:1931-1942"),
        ("SPIDEV", "chibios_hwdef.py:2898, 1451-1485"),
        ("BARO", "hwdef.py:255, 557-589"),
        ("IMU", "hwdef.py:248-249, 382-425"),
    ];
    // (define, where the tag reads it)
    const DEFINES: [(&str, &str); 5] = [
        (
            "AP_BOOTLOADER_FLASHING_ENABLED",
            "chibios_hwdef.py (4400 step A's refusal names it)",
        ),
        (
            "HAL_STORAGE_SIZE",
            "chibios_hwdef.py:859; StorageManager.h:30",
        ),
        ("HAL_I2C_MAX_CLOCK", "I2CDevice.cpp:53-54, 209"),
        ("HAL_LOGGING_DATAFLASH_ENABLED", "AP_Logger_config.h:15-17"),
        (
            "HAL_USE_ADC",
            "4400 step C's refusal: STM32_ADC_ADC12_DMA_STREAM",
        ),
    ];
    // Each SERIALn default an AP_GROUPINFO reads (AP_SerialManager.cpp at the
    // tag), exactly: SERIAL0 reads only its BAUD (its PROTOCOL default is a
    // literal, line 180, and it has no OPTIONS), adversarial A4; SERIAL7 and
    // 8 read a literal 0 for OPTIONS (lines 357, 376), AET-WO-4404.
    const SERIAL: [(&str, &str); 26] = [
        ("DEFAULT_SERIAL0_BAUD", "AP_SerialManager.cpp:172, 419"),
        ("DEFAULT_SERIAL1_PROTOCOL", "AP_SerialManager.cpp:191"),
        ("DEFAULT_SERIAL1_BAUD", "AP_SerialManager.cpp:199"),
        ("DEFAULT_SERIAL1_OPTIONS", "AP_SerialManager.cpp:281"),
        ("DEFAULT_SERIAL2_PROTOCOL", "AP_SerialManager.cpp:207"),
        ("DEFAULT_SERIAL2_BAUD", "AP_SerialManager.cpp:213"),
        ("DEFAULT_SERIAL2_OPTIONS", "AP_SerialManager.cpp:288"),
        ("DEFAULT_SERIAL3_PROTOCOL", "AP_SerialManager.cpp:221"),
        ("DEFAULT_SERIAL3_BAUD", "AP_SerialManager.cpp:227"),
        ("DEFAULT_SERIAL3_OPTIONS", "AP_SerialManager.cpp:295"),
        ("DEFAULT_SERIAL4_PROTOCOL", "AP_SerialManager.cpp:235"),
        ("DEFAULT_SERIAL4_BAUD", "AP_SerialManager.cpp:241"),
        ("DEFAULT_SERIAL4_OPTIONS", "AP_SerialManager.cpp:302"),
        ("DEFAULT_SERIAL5_PROTOCOL", "AP_SerialManager.cpp:249"),
        ("DEFAULT_SERIAL5_BAUD", "AP_SerialManager.cpp:255"),
        ("DEFAULT_SERIAL5_OPTIONS", "AP_SerialManager.cpp:309"),
        ("DEFAULT_SERIAL6_PROTOCOL", "AP_SerialManager.cpp:265"),
        ("DEFAULT_SERIAL6_BAUD", "AP_SerialManager.cpp:271"),
        ("DEFAULT_SERIAL6_OPTIONS", "AP_SerialManager.cpp:316"),
        ("DEFAULT_SERIAL7_PROTOCOL", "AP_SerialManager.cpp:346"),
        ("DEFAULT_SERIAL7_BAUD", "AP_SerialManager.cpp:352"),
        ("DEFAULT_SERIAL8_PROTOCOL", "AP_SerialManager.cpp:365"),
        ("DEFAULT_SERIAL8_BAUD", "AP_SerialManager.cpp:371"),
        ("DEFAULT_SERIAL9_PROTOCOL", "AP_SerialManager.cpp:384"),
        ("DEFAULT_SERIAL9_BAUD", "AP_SerialManager.cpp:390"),
        ("DEFAULT_SERIAL9_OPTIONS", "AP_SerialManager.cpp:395"),
    ];
    const PIN_TOKENS: [&str; 6] = [
        "SPEED_VERYLOW",
        "SPEED_LOW",
        "PUSHPULL",
        "OPENDRAIN",
        "FLOATING",
        "PULLDOWN",
    ];
    for (name, t, _) in BOARDS {
        let text = generate(&topology(t)).unwrap().text;
        for line in text.lines() {
            let line = line.split('#').next().unwrap().trim();
            if line.is_empty() {
                continue;
            }
            let w: Vec<&str> = line.split_whitespace().collect();
            let pin = w[0].len() >= 3
                && w[0].starts_with('P')
                && w[0][2..].bytes().all(|b| b.is_ascii_digit());
            if pin {
                for tok in &w[3..] {
                    let pwm = tok
                        .strip_prefix("PWM(")
                        .and_then(|r| r.strip_suffix(')'))
                        .is_some_and(|n| n.parse::<u32>().is_ok());
                    assert!(
                        pwm || PIN_TOKENS.contains(tok),
                        "{name}: pin token {tok} in `{line}`"
                    );
                }
            } else if w[0] == "define" {
                assert!(
                    DEFINES.iter().chain(&SERIAL).any(|(d, _)| w[1] == *d),
                    "{name}: define {} in `{line}`",
                    w[1]
                );
            } else {
                assert!(
                    KEYWORDS.iter().any(|(k, _)| *k == w[0]),
                    "{name}: keyword {} in `{line}`",
                    w[0]
                );
            }
        }
        assert!(
            !text.lines().any(|l| l.starts_with("DEFAULTGPIO")),
            "{name}: 4400a D-1"
        );
    }
}

/// `RUL-20`: the generator calls none of the rules the pinout check judges
/// output type, speed and pull by, and reads no specification.
#[test]
fn the_generator_calls_none_of_the_check_s_rules() {
    let src = include_str!("../src/lib.rs");
    for name in [
        "implied_pupd",
        "implied_open_drain",
        "speeds::",
        ".choose(",
        "Specification",
        "aethelgard_pincheck",
    ] {
        assert!(!src.contains(name), "src/lib.rs names `{name}`");
    }
}

/// D1, D10: the serial refusals, each by name.
#[test]
fn a_topology_ardupilot_cannot_run_a_console_on_is_refused() {
    // No serial port at all: the port and its two pins go.
    let mut v = sentinel_01();
    v["serial_ports"] = json!({});
    for net in ["CONSOLE_TX", "CONSOLE_RX"] {
        v["assignments"].as_object_mut().unwrap().remove(net);
        v["pin_speeds"].as_object_mut().unwrap().remove(net);
    }
    assert_eq!(refused(&v), Refusal::NoSerialPort);
    // A port, but not the console.
    let mut v = sentinel_01();
    v["serial_ports"]["CONSOLE"]["role"] = "mavlink".into();
    assert_eq!(refused(&v), Refusal::NoConsole);
    // An inverted console.
    let mut v = sentinel_01();
    v["serial_ports"]["CONSOLE"]["framing"]["inverted"] = true.into();
    assert_eq!(refused(&v).kind(), "FramingNotImplied");
    // Parity on a console.
    let mut v = sentinel_01();
    v["serial_ports"]["CONSOLE"]["framing"]["parity"] = "even".into();
    let err = refused(&v);
    assert_eq!(err.kind(), "FramingNotImplied");
    assert_eq!(err.id(), Some("CONSOLE"));
}

/// D2: the speed token is the stored speed, never a choice of the
/// generator. Since `AET-WO-4400b2` F4 a stored speed `solve` would not give
/// is refused by the topology before the generator reads it.
#[test]
fn each_pin_takes_the_stored_speed() {
    let mut v = sentinel_01();
    let text = generate(&topology(&v.to_string())).unwrap().text;
    assert!(text.contains("PA5 SPI1_SCK SPI1 SPEED_LOW PUSHPULL FLOATING"));
    v["pin_speeds"]["SPI_SCK"] = 0.into();
    let err = generate(&topology(&v.to_string())).unwrap_err();
    assert_eq!(err.kind(), "InvalidTopology", "{err}");
    assert!(err.to_string().contains("pin_speeds.SPI_SCK"), "{err}");
}

/// D6: the SPIDEV speed is the controller's achieved rate (16 MHz on
/// Sentinel-01's SPI1, 2 MHz on Sentinel-02's SPI2). The rounding and the
/// divider refusal are tested in `src/lib.rs` and, on a solved
/// specification, in the CLI's `hwdef_tier1.rs`.
#[test]
fn spidev_speeds_are_the_achieved_rates() {
    let t = topology(BOARDS[0].1);
    assert!(
        generate(&t)
            .unwrap()
            .text
            .contains("SPIDEV dataflash SPI1 DEVID1 FLASH_CS MODE0 16*MHZ 16*MHZ")
    );
    let t = topology(BOARDS[1].1);
    assert!(
        generate(&t)
            .unwrap()
            .text
            .contains("SPIDEV icm42688 SPI2 DEVID1 IMU_CS MODE0 2*MHZ 2*MHZ")
    );
}

/// D7: a solved battery sense pin is refused until `SCALE` has a source.
#[test]
fn a_topology_with_sense_pins_is_refused() {
    let mut v = sentinel_01();
    v["power"] = json!({
        "domains": {}, "input_steady_milliamps": 0, "input_fault_milliamps": 0,
        "conditions": [], "sense": {"VBAT_SENSE": {"pin": "PC0", "position": 0, "channels": ["ADC1_INP10"]}}
    });
    // The sense is also its ADC input (`AET-WO-1211a`).
    v["adc_inputs"] =
        json!({"VBAT_SENSE": {"pin": "PC0", "peripheral": "ADC1", "channel": "INP10"}});
    let t: SolvedTopology = serde_json::from_value(v).unwrap();
    t.validate().unwrap();
    assert_eq!(
        generate(&t).unwrap_err(),
        Refusal::SenseNotEmitted {
            net: "VBAT_SENSE".into()
        }
    );
}

/// The adversarial pass's case: two buses at different speeds run at the
/// slower, so neither runs faster than declared.
#[test]
fn the_i2c_clock_is_the_slowest_declared_bus() {
    let mut v = sentinel_01();
    v["i2c_speeds"]["SENSORS"] = 100_000.into();
    let t: SolvedTopology = serde_json::from_value(v).unwrap();
    t.validate().unwrap();
    assert!(
        generate(&t)
            .unwrap()
            .text
            .contains("define HAL_I2C_MAX_CLOCK 100000")
    );
}

/// The system timer moves off a motor's timer, and both taken is refused.
#[test]
fn the_system_timer_avoids_every_motor_timer() {
    let t = topology(BOARDS[2].1);
    assert!(generate(&t).unwrap().text.contains("STM32_ST_USE_TIMER 5"));
    let mut v: Value = serde_json::from_str(BOARDS[2].1).unwrap();
    for (i, timer) in [(1, "TIM5"), (2, "TIM2")] {
        let net = format!("MOTOR{i}");
        let a = &mut v["assignments"][&net];
        a["peripheral"] = timer.into();
        a["signal"] = format!("{timer}_CH1").into();
    }
    let t: SolvedTopology = serde_json::from_value(v.clone()).unwrap();
    t.validate().unwrap();
    assert_eq!(generate(&t).unwrap_err(), Refusal::NoSystemTimer);
    // TIM5 alone taken: the tick moves to TIM2.
    v["assignments"]["MOTOR2"]["peripheral"] = "TIM1".into();
    v["assignments"]["MOTOR2"]["signal"] = "TIM1_CH2".into();
    let t: SolvedTopology = serde_json::from_value(v).unwrap();
    assert!(generate(&t).unwrap().text.contains("STM32_ST_USE_TIMER 2"));
}

fn sentinel_02() -> Value {
    serde_json::from_str(BOARDS[1].1).unwrap()
}

/// `AET-WO-4400d` D5: `sentinel-02-own` declares U5 with X right and Z up
/// in the board frame, so the driver's chip frame is flipped about the
/// board's forward-right diagonal: `ROTATION_ROLL_180_YAW_90`
/// (`vector3.cpp` L80-82: `x = y; y = x; z = -z`), the lower alias of 27.
#[test]
fn the_imu_line_carries_the_declared_rotation() {
    let text = generate(&topology(BOARDS[1].1)).unwrap().text;
    let imu: Vec<&str> = text.lines().filter(|l| l.starts_with("IMU ")).collect();
    assert_eq!(imu.len(), 1, "{text}");
    assert!(
        imu[0].starts_with("IMU Invensensev3 SPI:icm42688 ROTATION_ROLL_180_YAW_90 "),
        "{}",
        imu[0]
    );
    assert!(
        text.contains("z down through F.Cu to B.Cu"),
        "the D3 header"
    );
    assert!(!text.contains("Not emitted: U5"), "{text}");
    // Each other declaration: every one of the 24 right-angle pairs names a
    // rotation, no two the same, and turning the board's right into its
    // forward turns `ROLL_180_YAW_90` into `ROLL_180`.
    let words = ["forward", "aft", "right", "left", "up", "down"];
    let mut names = std::collections::BTreeSet::new();
    for x in words {
        for z in words {
            let mut v = sentinel_02();
            v["spi_devices"]["U5"]["orientation"] = json!({"x": x, "z": z});
            let t: SolvedTopology = serde_json::from_value(v).unwrap();
            let perpendicular = x != z
                && !matches!(
                    (x, z),
                    ("forward", "aft")
                        | ("aft", "forward")
                        | ("right", "left")
                        | ("left", "right")
                        | ("up", "down")
                        | ("down", "up")
                );
            match generate(&t) {
                Ok(h) => {
                    assert!(perpendicular, "{x} {z}");
                    let line = h
                        .text
                        .lines()
                        .find(|l| l.starts_with("IMU "))
                        .unwrap()
                        .to_owned();
                    let name = line.split_whitespace().nth(3).unwrap().to_owned();
                    if (x, z) == ("forward", "up") {
                        assert_eq!(name, "ROTATION_ROLL_180");
                    }
                    assert!(names.insert(name), "{x} {z}: {line}");
                }
                Err(e) => {
                    assert!(!perpendicular, "{x} {z}: {e}");
                    assert_eq!(e.kind(), "InvalidTopology", "{e}");
                }
            }
        }
    }
    assert_eq!(names.len(), 24);
}

/// `INV-06`: an IMU with no declared orientation is refused by name, never
/// given a default rotation.
#[test]
fn an_imu_with_no_declared_orientation_is_refused_by_name() {
    let mut v = sentinel_02();
    v["spi_devices"]["U5"]
        .as_object_mut()
        .unwrap()
        .remove("orientation");
    let e = refused(&v);
    assert_eq!(
        e,
        Refusal::ImuOrientationUndeclared {
            reference: "U5".into(),
            part: "ICM42688P".into()
        }
    );
    assert_eq!((e.kind(), e.id()), ("ImuOrientationUndeclared", Some("U5")));
}

/// `AET-WO-4404` (§7.1 L760, `AET-WO-4400b` A9): a chip select whose name
/// starts with one of ArduPilot's alternate-function prefixes
/// (`chibios_hwdef.py:33`) is refused by name, case-sensitive as
/// ArduPilot's `startswith` is; ArduPilot refused each at configure
/// (measured, FEEDBACK §2.3). A name holding a prefix elsewhere, or a
/// lower-case one, generates.
#[test]
fn a_chip_select_named_like_an_alternate_function_is_refused() {
    let renamed = |name: &str| -> Result<String, Refusal> {
        let t = topology(&BOARDS[0].1.replace("\"FLASH_CS\"", &format!("\"{name}\"")));
        generate(&t).map(|h| h.text)
    };
    for prefix in [
        "USART", "UART", "SPI", "I2C", "SDIO", "SDMMC", "OTG", "JT", "TIM", "CAN", "QUADSPI",
        "OCTOSPI", "ETH", "MCO",
    ] {
        let net = format!("{prefix}_FLASH_CS");
        let e = renamed(&net).unwrap_err();
        assert_eq!(
            e,
            Refusal::LabelTakesAfPrefix {
                net: net.clone(),
                prefix: prefix.into()
            }
        );
        assert_eq!(
            (e.kind(), e.id()),
            ("LabelTakesAfPrefix", Some(net.as_str()))
        );
    }
    // ArduPilot's own carve-outs configure, but the label still reads as a
    // peripheral's: refused all the same (FEEDBACK §2.3).
    assert_eq!(
        renamed("SPI_FLASH_TXINV").unwrap_err().kind(),
        "LabelTakesAfPrefix"
    );
    for name in ["spi_cs", "FLASH_SPI_CS", "SP_CS"] {
        let text = renamed(name).unwrap();
        assert!(text.contains(&format!(" {name} CS ")), "{name}");
    }
}

/// `OscillatorNotCited`'s first test (`AET-WO-4240a` §1.5, `4240` §8): the
/// reference board on the 16 MHz crystal's HSE tree, its I2C words re-solved
/// for the crystal's budget, is a valid topology, and the generator refuses
/// it by name: the MCU row cites ArduPilot's SPI kernel clocks only at
/// `OSCILLATOR_HZ` 0.
#[test]
fn a_topology_on_an_hse_the_mcu_row_does_not_cite_is_refused() {
    let mut v = sentinel_01();
    v["clock"] = json!({"source": "PLL1", "sysclk_hz": 64_000_000, "hse_hz": 16_000_000,
        "d1cpre": 1, "hpre": 1, "d1ppre": 1, "d2ppre1": 1, "d2ppre2": 1, "d3ppre": 1,
        "timpre": false, "vos": "scale2",
        "hse": {"declared": {"crystal": {"part": "ABM8AIG-16.000MHZ-12-2Z-T3",
            "load_caps_femtofarads": [18000, 18000], "pull_ppm": 30,
            "temperature_celsius": {"min": -20, "max": 85}, "life_hours": 8000,
            "startup_max_us": 20000, "startup_source": "architect ruling 2026-11-20: 10x DS12117 Rev 10 Table 33 tSU(HSE) typical 2 ms; no maximum is published (Note 4)"}},
            "fast_ppm": 102, "slow_ppm": 102,
            "pllckselr": 0x52, "pllcfgr": 0x0003_0004, "pll1divr": 0x0103_064F,
            "flash_acr": 0x11}});
    v["i2c_timings"]["I2C1"]["timingr"] =
        json!({"Fast400k": 0x10C7_123B, "FastPlus1M": 0x00A2_0E2D, "Standard100k": 0x50D3_2A3F});
    // `AET-WO-4240b` Q3: under H1 SPI1 runs on `pll1_q_ck`, its select at reset.
    for (_, c) in v["spi_controllers"].as_object_mut().unwrap().iter_mut() {
        c["kernel"] = json!("pll1_q");
        // ÷4 at the crystal's fastest, 64 MHz × (1 + 102 ppm) = 64,006,528 Hz.
        c["achieved_max_hz"] = json!(16_001_632);
    }
    let t: SolvedTopology = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(t.validate(), Ok(()));
    let e = refused(&v);
    assert_eq!(e.kind(), "OscillatorNotCited", "{e}");
    assert!(matches!(
        e,
        Refusal::OscillatorNotCited {
            hse_hz: 16_000_000,
            cited: 0
        }
    ));
}

/// `AET-WO-1211a`: ArduPilot's hwdef has no `GPIO(n)` line this generator
/// writes for a pin device's pins, so a topology with one is refused by the
/// device's reference and part.
#[test]
fn a_topology_with_a_pin_device_is_refused_as_not_emitted() {
    let mut v = sentinel_01();
    v["pin_devices"] = json!({"U9": {"part": "WAKE_SWITCH", "bridges": {}, "supplies": {}}});
    v["gpio"]["WAKE"] = json!({"pin": "PB5", "direction": "input", "pull": "up", "active": "low", "pulled": "high"});
    v["pin_speeds"]["WAKE"] = json!(0);
    let t: SolvedTopology = serde_json::from_value(v).unwrap();
    t.validate().unwrap();
    let err = generate(&t).unwrap_err();
    assert_eq!(
        err,
        Refusal::GpioNotEmitted {
            reference: "U9".into(),
            part: "WAKE_SWITCH".into()
        }
    );
    assert_eq!((err.kind(), err.id()), ("GpioNotEmitted", Some("U9")));
}

/// `serial-dshot-9600` (four DShot600 outputs on TIM1 CH1..CH4, PA8..PA11)
/// with the motors-pwm fixture's four PWM outputs on the same pins, timer and
/// speeds in their place: a PWM board this generator emits (it has a console).
fn pwm_board() -> SolvedTopology {
    let pwm: Value = serde_json::from_str(include_str!(
        "../../aethelgard-svd-hal/tests/ui/motors-pwm/topology.json"
    ))
    .unwrap();
    let mut v: Value = serde_json::from_str(BOARDS[2].1).unwrap();
    v["motor_outputs"] = pwm["motor_outputs"].clone();
    let t: SolvedTopology = serde_json::from_value(v).unwrap();
    t.validate().unwrap();
    t
}

/// The `PWM(n)` lines of a generated file, comments dropped.
fn pwm_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.split('#').next().unwrap().trim().to_owned())
        .filter(|l| l.split_whitespace().any(|w| w.starts_with("PWM(")))
        .collect()
}

/// `AET-WO-4300j` D6: a motor line's pull follows its declared fitment, pin
/// by pin. `fitted` (`4300i` D0): the line ends `PUSHPULL FLOATING`,
/// ArduPilot's own default for a PWM line, written explicitly
/// (`chibios_hwdef.py:387-427` at the pinned tag), since the declared
/// external resistor holds it; `open`: `PUSHPULL PULLDOWN`, the link not
/// fitted. No line carries `PULLUP`. The committed `serial-dshot-9600` is
/// `fitted` on every output and `serial-dshot-9600-open` `open`; each motor's
/// line is found by its own pin, in each committed file and in what the
/// generator writes, and one output flipped alone moves its own line and no
/// other.
#[test]
fn each_motor_line_s_pull_follows_its_fitment() {
    use aethelgard_ir::motor::PullFitment;
    // Each motor's line in `text`, found by its own pin, against its fitment.
    let check_text = |t: &SolvedTopology, text: &str| {
        let lines = pwm_lines(text);
        assert_eq!(lines.len(), t.motor_outputs.len(), "{text}");
        for (net, m) in &t.motor_outputs {
            let pin = &t.assignments[net].pin;
            let line = lines
                .iter()
                .find(|l| l.split_whitespace().next() == Some(pin.as_str()))
                .unwrap_or_else(|| panic!("{net}: no line on {pin}"));
            let want = match m.pull.fitment {
                PullFitment::Fitted => " PUSHPULL FLOATING",
                PullFitment::Open => " PUSHPULL PULLDOWN",
            };
            assert!(line.ends_with(want), "{net}: `{line}`");
            assert!(!line.contains("PULLUP"), "`{line}`");
        }
    };
    let check = |t: &SolvedTopology| check_text(t, &generate(t).unwrap().text);
    let fitted = topology(BOARDS[2].1);
    let open = topology(BOARDS[3].1);
    // The committed files themselves, pin by pin.
    check_text(&fitted, BOARDS[2].2);
    check_text(&open, BOARDS[3].2);
    for (t, want) in [
        (&fitted, PullFitment::Fitted),
        (&pwm_board(), PullFitment::Fitted),
        (&open, PullFitment::Open),
    ] {
        assert_eq!(t.motor_outputs.len(), 4);
        assert!(t.motor_outputs.values().all(|m| m.pull.fitment == want));
        check(t);
    }
    // One output flipped alone, either way.
    for (base, to) in [(&fitted, PullFitment::Open), (&open, PullFitment::Fitted)] {
        let mut t = base.clone();
        t.motor_outputs.get_mut("MOTOR2").unwrap().pull.fitment = to;
        t.validate().unwrap();
        check(&t);
        let (a, b) = (
            pwm_lines(&generate(base).unwrap().text),
            pwm_lines(&generate(&t).unwrap().text),
        );
        let moved: Vec<usize> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
        assert_eq!(moved, [1], "{a:?} {b:?}");
    }
}

/// `AET-WO-4300j` §1 (4): one comment line above each motor line names, from
/// the topology alone, the net, the fitment, the resistor, its ohms, the
/// link and the bound the resistor was judged against. ArduPilot discards
/// it (`hwdef.py:209-212`). No `HAL_SERIAL_ESC_COMM_ENABLED` is written, and
/// the comment lines set no width: every line with a source has its `#` in
/// one column.
#[test]
fn a_comment_line_above_each_motor_line_names_its_fitment_and_bound() {
    for (name, t, _) in &BOARDS[2..] {
        let t = topology(t);
        let text = generate(&t).unwrap().text;
        assert!(!text.contains("HAL_SERIAL_ESC_COMM_ENABLED"), "{name}");
        let lines: Vec<&str> = text.lines().collect();
        for (net, m) in &t.motor_outputs {
            let p = &m.pull;
            let want = match p.fitment {
                aethelgard_ir::motor::PullFitment::Fitted => format!(
                    "# {net} pull: fitted, {} {} ohm to GND via link {}; bound {}..{} ohm, checked for BLHeli passthrough idle with it fitted, the ESC threshold a stand-in (AET-WO-4300j R1)",
                    p.resistor, p.ohms, p.link, p.floor_ohms, p.ceiling_ohms
                ),
                aethelgard_ir::motor::PullFitment::Open => format!(
                    "# {net} pull: open (link {} not fitted), PULLDOWN in firmware; {} {} ohm, bound {}..{} ohm (AET-WO-4300j)",
                    p.link, p.resistor, p.ohms, p.floor_ohms, p.ceiling_ohms
                ),
            };
            let at = lines.iter().position(|l| *l == want).unwrap_or_else(|| {
                panic!(
                    "{name}: no `{want}`
{text}"
                )
            });
            let next = lines[at + 1];
            assert!(
                next.starts_with(&format!("{} ", t.assignments[net].pin))
                    && next.contains(&format!(" PWM({}) ", m.index + 1)),
                "{name}: `{next}` after `{want}`"
            );
        }
        let comments = lines
            .iter()
            .filter(|l| {
                l.starts_with("# ")
                    && l.contains(" pull: ")
                    && (l.ends_with("(AET-WO-4300j)") || l.ends_with("(AET-WO-4300j R1)"))
            })
            .count();
        assert_eq!(comments, t.motor_outputs.len(), "{name}");
        // Every line with a source shares one width, its longest line's: no
        // comment line set it.
        let sourced: Vec<&str> = lines
            .iter()
            .copied()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .collect();
        let widths: BTreeSet<usize> = sourced.iter().map(|l| l.find("  # ").unwrap()).collect();
        let longest = sourced
            .iter()
            .map(|l| l[..l.find("  # ").unwrap()].trim_end().len())
            .max();
        assert_eq!(
            widths.into_iter().collect::<Vec<_>>(),
            Vec::from_iter(longest),
            "{name}"
        );
    }
}

/// With no internal motor pull, an output that idles high is emitted like
/// any other: `IdleHighNotEmitted` (`AET-WO-1211a`) refused it only because
/// the `PULLDOWN` line fought a high idle, and that line is gone (D0). The
/// file is the one a pulsed idle gets, byte for byte (hwdef.dat carries no
/// idle).
#[test]
fn a_motor_output_that_idles_high_is_emitted_like_any_other() {
    let before = generate(&pwm_board()).unwrap().text;
    let mut t = pwm_board();
    let (_, m) = t
        .motor_outputs
        .iter_mut()
        .find(|(_, m)| m.pwm.is_some())
        .unwrap();
    m.pwm.as_mut().unwrap().idle =
        aethelgard_ir::motor::Idle::Level(aethelgard_ir::motor::Level::High);
    // An idle high's CCR is ARR + 1, as `solve` writes it.
    m.ccr = m.arr + 1;
    t.validate().unwrap();
    assert_eq!(generate(&t).unwrap().text, before);
}
