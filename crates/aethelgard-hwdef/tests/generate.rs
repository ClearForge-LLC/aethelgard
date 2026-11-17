//! `AET-WO-4400b`: the generator on the three CI boards' topologies, byte
//! for byte; every emitted keyword one ArduPilot `Copter-4.7.2` reads; every
//! refusal by name; and the generator calls none of the rules the pinout
//! check judges by (`RUL-20`).

use aethelgard_hwdef::{Refusal, generate};
use aethelgard_ir::SolvedTopology;
use serde_json::{Value, json};

const BOARDS: [(&str, &str, &str); 3] = [
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
    const KEYWORDS: [(&str, &str); 11] = [
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
    ];
    // (define, where the tag reads it)
    const DEFINES: [(&str, &str); 6] = [
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
        ("DEFAULT_SERIAL0_BAUD", "AP_SerialManager.cpp:172, 419"),
    ];
    // SERIAL1 to SERIAL9 each read their PROTOCOL, BAUD and OPTIONS defaults
    // in an AP_GROUPINFO (AP_SerialManager.cpp:191-395); SERIAL0 reads only
    // its BAUD (its PROTOCOL default is a literal, line 180, and it has no
    // OPTIONS), adversarial A4.
    let serial_n = |d: &str| {
        d.strip_prefix("DEFAULT_SERIAL").is_some_and(|r| {
            let (n, k) = r.split_at(1);
            n.bytes().all(|b| (b'1'..=b'9').contains(&b))
                && ["_PROTOCOL", "_BAUD", "_OPTIONS"].contains(&k)
        })
    };
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
                    DEFINES.iter().any(|(d, _)| w[1] == *d) || serial_n(w[1]),
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
        "motor_output_pupd",
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

/// D2: the speed token is the stored speed, never a choice of the generator.
#[test]
fn each_pin_takes_the_stored_speed() {
    let mut v = sentinel_01();
    let text = generate(&topology(&v.to_string())).unwrap().text;
    assert!(text.contains("PA5 SPI1_SCK SPI1 SPEED_LOW PUSHPULL FLOATING"));
    v["pin_speeds"]["SPI_SCK"] = 0.into();
    let text = generate(&topology(&v.to_string())).unwrap().text;
    assert!(
        text.contains("PA5 SPI1_SCK SPI1 SPEED_VERYLOW PUSHPULL FLOATING"),
        "{text}"
    );
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
