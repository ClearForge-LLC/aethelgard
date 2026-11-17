//! Tier 1: the identity judge (`AET-WO-4400b2`) on the reference topology and
//! a `hwdef.h` holding ArduPilot's device tables as it writes them, with the
//! ELF's serial table given as decoded entries. Every mismatch is named; the
//! work order's §5 cases are each here.

mod common;

use aethelgard_ir::SolvedTopology;
use aethelgard_pincheck::elf::SerialEntry;
use aethelgard_pincheck::{Refusal, header, identity};
use common::{Elf, IDENTITY, SD1, topology};
use serde_json::{Value, json};

const EMPTY: SerialEntry = SerialEntry {
    instance: 0,
    is_usb: false,
    tx_line: 0,
    rx_line: 0,
    empty: true,
    driver_is_instance: false,
};

/// The findings as `kind at [expected | decoded]`, or the refusal's kind.
fn judge(t: &SolvedTopology, text: &str, serial: &[SerialEntry]) -> Result<Vec<String>, Refusal> {
    let parts = aethelgard_ir::parts::table().unwrap();
    let pre = header::preprocess(text)?;
    Ok(identity::judge(t, parts, &pre, serial)?
        .iter()
        .map(|f| format!("{} {} [{} | {}]", f.kind, f.pin, f.expected, f.decoded))
        .collect())
}

fn kinds(found: &[String]) -> Vec<&str> {
    found.iter().map(|f| f.split(' ').next().unwrap()).collect()
}

#[test]
fn the_reference_tables_pass() {
    assert_eq!(
        judge(&topology(), IDENTITY, &[EMPTY]).unwrap(),
        Vec::<String>::new()
    );
}

/// Each SPI fact, wrong alone, is named: chip select, bus, mode, each speed.
#[test]
fn each_spi_fact_is_judged() {
    let cases = [
        (
            "PAL_LINE(GPIOA,0U)",
            "PAL_LINE(GPIOA,4U)",
            vec!["WrongChipSelect"],
        ),
        ("SPIDEV_MODE0", "SPIDEV_MODE3", vec!["WrongSpiMode"]),
        ("16*MHZ,  16*MHZ", "16*MHZ,  20*MHZ", vec!["WrongSpiSpeed"]),
        (
            "16*MHZ,  16*MHZ",
            "15998*KHZ,  16*MHZ",
            vec!["WrongSpiSpeed"],
        ),
        ("16*MHZ,  16*MHZ", "16000*KHZ,  16*MHZ", vec![]),
        // The window's edges: 1 kHz below is out, 999 Hz below is in.
        (
            "16*MHZ,  16*MHZ",
            "15999*KHZ,  16*MHZ",
            vec!["WrongSpiSpeed"],
        ),
        ("16*MHZ,  16*MHZ", "15999001,  16*MHZ", vec![]),
        (
            "\"dataflash\"",
            "\"flash\"",
            vec!["MissingSpiDevice", "UnknownSpiDevice"],
        ),
    ];
    for (from, to, want) in cases {
        let text = IDENTITY.replacen(from, to, 1);
        assert_ne!(text, IDENTITY, "{from}");
        let found = judge(&topology(), &text, &[EMPTY]).unwrap();
        assert_eq!(kinds(&found), want, "{to}: {found:?}");
    }
    // A second bus listed first moves the device to SPI2.
    let text = IDENTITY.replace(
        "#define HAL_SPI_BUS_LIST HAL_SPI1_CONFIG",
        "#define HAL_SPI2_CONFIG { &SPID2, 2, X, PAL_LINE(GPIOB,13U) }\n#define HAL_SPI_BUS_LIST HAL_SPI2_CONFIG,HAL_SPI1_CONFIG",
    );
    assert_eq!(
        kinds(&judge(&topology(), &text, &[EMPTY]).unwrap()),
        ["WrongSpiBus"]
    );
}

/// §5: two devices whose chip selects are swapped *and* whose modes differ:
/// every fact of both is named, not the first only.
#[test]
fn swapped_selects_with_wrong_modes_are_each_named() {
    let mut v: Value = serde_json::to_value(topology()).unwrap();
    v["gpio"]["IMU_CS"] = "PA1".into();
    v["pin_speeds"]["IMU_CS"] = 0.into();
    v["spi_devices"]["U5"] = json!({"index": 1, "part": "ICM42688P", "bus": "FLASH", "peripheral": "SPI1", "cs": "IMU_CS"});
    let t: SolvedTopology = serde_json::from_value(v).unwrap();
    let text = IDENTITY
        .replace("PAL_LINE(GPIOA,0U) , SPIDEV_MODE0", "PAL_LINE(GPIOA,1U) , SPIDEV_MODE3")
        .replace(
            "#define HAL_SPI_DEVICE_LIST",
            "#define HAL_SPI_DEVICE1  SPIDesc(\"icm42688\", 0, 2, PAL_LINE(GPIOA,0U) , SPIDEV_MODE1, 16*MHZ, 16*MHZ)\n#define HAL_SPI_DEVICE_LIST",
        )
        .replace("   HAL_SPI_DEVICE0", "   HAL_SPI_DEVICE0,HAL_SPI_DEVICE1");
    let found = judge(&t, &text, &[EMPTY]).unwrap();
    assert_eq!(
        kinds(&found),
        [
            "WrongChipSelect",
            "WrongSpiMode",
            "WrongChipSelect",
            "WrongSpiMode"
        ],
        "{found:?}"
    );
}

/// A topology with four motors on TIM1 CH1-CH4, PWM(1) to PWM(4).
fn motors() -> SolvedTopology {
    let mut v: Value = serde_json::to_value(topology()).unwrap();
    for (i, pin) in ["PA8", "PA9", "PA10", "PA11"].iter().enumerate() {
        let net = format!("MOTOR{}", i + 1);
        v["assignments"][&net] = json!({"pin": pin, "peripheral": "TIM1", "signal": format!("TIM1_CH{}", i + 1), "alternate_function": 1});
        v["motor_outputs"][&net] = json!({"index": i, "protocol": "dshot600", "pwm": null, "psc": 0, "arr": 107, "ccr": 40});
    }
    serde_json::from_value(v).unwrap()
}

fn with_group(chan: &str) -> String {
    format!(
        "{IDENTITY}#define HAL_PWM_GROUP1 {{ true, {{{chan}}}, {{ 1000000, 20000, NULL, {{ {{PWM_OUTPUT_ACTIVE_HIGH, NULL}} }}, 0, 0}}, &PWMD1, 1, HAL_PWM1_DMA_CONFIG, {{ 1, 1, 1, 1 }}, {{ PAL_LINE(GPIOA,8U) }}}}\n#define HAL_PWM_GROUPS HAL_PWM_GROUP1\n"
    )
}

/// The motors: the right map passes; a swap names two channels (A1); a
/// rotation (§5) names all four; a channel no motor drives is named; a
/// timer with no group is named.
#[test]
fn each_motor_channel_is_judged() {
    let t = motors();
    assert_eq!(
        judge(&t, &with_group("0, 1, 2, 3"), &[EMPTY]).unwrap(),
        Vec::<String>::new()
    );
    let found = judge(&t, &with_group("1, 0, 2, 3"), &[EMPTY]).unwrap();
    assert_eq!(
        found,
        [
            "WrongMotorChannel TIM1_CH1 [PWM(1) | PWM(2)]",
            "WrongMotorChannel TIM1_CH2 [PWM(2) | PWM(1)]"
        ]
    );
    let found = judge(&t, &with_group("1, 2, 3, 0"), &[EMPTY]).unwrap();
    assert_eq!(kinds(&found), ["WrongMotorChannel"; 4]);
    let found = judge(&t, &with_group("0, 1, 2, 255"), &[EMPTY]).unwrap();
    assert_eq!(found, ["WrongMotorChannel TIM1_CH4 [PWM(4) | disabled]"]);
    let found = judge(&topology(), &with_group("0, 255, 255, 255"), &[EMPTY]).unwrap();
    assert_eq!(kinds(&found), ["UnexpectedPwmOutput"]);
    assert_eq!(
        kinds(&judge(&t, IDENTITY, &[EMPTY]).unwrap()),
        ["MissingPwmGroup"; 4]
    );
}

/// The I2C clock: above the slowest declared bus is named; at it, or by
/// ArduPilot's 100 kHz default, passes.
#[test]
fn the_i2c_clock_is_never_above_the_slowest_bus() {
    let t = topology();
    let text = IDENTITY.replace("HAL_I2C_MAX_CLOCK 400000", "HAL_I2C_MAX_CLOCK 1000000");
    assert_eq!(
        kinds(&judge(&t, &text, &[EMPTY]).unwrap()),
        ["I2cClockAboveDeclared"]
    );
    let text = IDENTITY.replace("#define HAL_I2C_MAX_CLOCK 400000\n", "");
    assert_eq!(judge(&t, &text, &[EMPTY]).unwrap(), Vec::<String>::new());
}

/// The barometer probe: wrong address, driver or bus, missing, or extra.
#[test]
fn each_barometer_probe_is_judged() {
    let t = topology();
    for (from, to, want) in [
        ("0, 118)", "0, 119)", "WrongBaroProbe"),
        ("AP_Baro_BMP280", "AP_Baro_DPS280", "WrongBaroProbe"),
        (
            "#define HAL_BARO_PROBE_LIST HAL_BARO_PROBE1\n",
            "",
            "MissingBaroProbe",
        ),
    ] {
        let text = IDENTITY.replace(from, to);
        assert_ne!(text, IDENTITY);
        assert_eq!(kinds(&judge(&t, &text, &[EMPTY]).unwrap()), [want], "{to}");
    }
    // Only what the list names is compiled (`AP_Baro.cpp:666`, adversarial
    // A4): a probe it does not name is not judged, one it names is.
    let unlisted = IDENTITY.replace(
        "#define HAL_BARO_PROBE_LIST",
        "#define HAL_BARO_PROBE2 probe_i2c_dev(AP_Baro_BMP581::probe, 0, 70); RETURN_IF_NO_SPACE;\n#define HAL_BARO_PROBE_LIST",
    );
    assert_eq!(
        judge(&t, &unlisted, &[EMPTY]).unwrap(),
        Vec::<String>::new()
    );
    let listed = unlisted.replace(
        "HAL_BARO_PROBE_LIST HAL_BARO_PROBE1",
        "HAL_BARO_PROBE_LIST HAL_BARO_PROBE1;HAL_BARO_PROBE2",
    );
    assert_eq!(
        kinds(&judge(&t, &listed, &[EMPTY]).unwrap()),
        ["UnexpectedBaroProbe"]
    );
    // A list written straight out, with no numbered macro, is read too.
    let inline = IDENTITY.replace(
        "#define HAL_BARO_PROBE_LIST HAL_BARO_PROBE1",
        "#define HAL_BARO_PROBE_LIST HAL_BARO_PROBE1; probe_i2c_dev(AP_Baro_BMP280::probe, 0, 119); RETURN_IF_NO_SPACE",
    );
    assert_eq!(
        judge(&t, &inline, &[EMPTY]).unwrap(),
        ["UnexpectedBaroProbe HAL_BARO_PROBE_LIST [none | BMP280 on I2C1 at 0x77]"]
    );
}

/// §5: a topology with no barometer and one bus, and no SPI device, refuses
/// nothing when ArduPilot's tables hold none either.
#[test]
fn a_bare_topology_refuses_nothing_it_should_not() {
    let mut v: Value = serde_json::to_value(topology()).unwrap();
    v["i2c_devices"] = json!({});
    v["spi_devices"] = json!({});
    let t: SolvedTopology = serde_json::from_value(v).unwrap();
    let text = "#define MHZ (1000U*1000U)\n#define KHZ (1000U)\n";
    assert_eq!(
        judge(&t, text, &[EMPTY, EMPTY]).unwrap(),
        Vec::<String>::new()
    );
}

/// The serial order from the ELF's table: the console on SERIAL0 at its pins
/// and baud, nothing after it; anything else named.
#[test]
fn the_serial_order_and_bauds_are_judged() {
    let mut v: Value = serde_json::to_value(topology()).unwrap();
    for (net, pin, sig) in [("CONSOLE_TX", "PA9", "TX"), ("CONSOLE_RX", "PA10", "RX")] {
        v["assignments"][net] = json!({"pin": pin, "peripheral": "USART1", "signal": format!("USART1_{sig}"), "alternate_function": 7});
    }
    v["serial_ports"]["CONSOLE"] = json!({"index": 0, "peripheral": "USART1", "tx": "CONSOLE_TX", "rx": "CONSOLE_RX", "role": "console",
        "framing": {"baud": 115200, "data_bits": 8, "parity": "none", "stop_bits": 1, "inverted": false},
        "negotiated": null, "peer_deviation_ppm": 5000, "kernel_hz": 64000000, "brr": 556, "actual_millibaud": 115107913, "error_ppm": -799});
    let t: SolvedTopology = serde_json::from_value(v).unwrap();
    let usart1 = SerialEntry {
        instance: 1,
        is_usb: false,
        tx_line: 0x5802_0009,
        rx_line: 0x5802_000a,
        empty: false,
        driver_is_instance: true,
    };
    assert_eq!(
        judge(&t, IDENTITY, &[usart1, EMPTY]).unwrap(),
        Vec::<String>::new()
    );
    let usart3 = SerialEntry {
        instance: 3,
        tx_line: 0x5802_0c08,
        rx_line: 0x5802_0c09,
        ..usart1
    };
    let found = judge(&t, IDENTITY, &[usart1, usart3]).unwrap();
    assert_eq!(
        found,
        ["UnexpectedSerialPort SERIAL1 [none | U(S)ART3 TX PD8 RX PD9]"]
    );
    assert_eq!(
        kinds(&judge(&t, IDENTITY, &[usart3]).unwrap()),
        ["WrongSerialPort"]
    );
    assert_eq!(
        kinds(&judge(&t, IDENTITY, &[EMPTY]).unwrap()),
        ["MissingSerialPort"]
    );
    let moved = SerialEntry {
        tx_line: 0x5802_040e,
        ..usart1
    };
    assert_eq!(
        kinds(&judge(&t, IDENTITY, &[moved]).unwrap()),
        ["WrongSerialPort"]
    );
    let text = format!("{IDENTITY}#define DEFAULT_SERIAL0_BAUD 921600\n");
    assert_eq!(
        judge(&t, &text, &[usart1]).unwrap(),
        ["WrongSerialBaud SERIAL0 [115200 baud | 921600 baud, then 921600 baud]"]
    );
    // SERIAL0 opens at its define, then the GCS reopens it at the define /
    // 1000 mapped (adversarial A7): 14400 runs at 14000 after start-up.
    let mut v: Value = serde_json::to_value(&t).unwrap();
    v["serial_ports"]["CONSOLE"]["framing"]["baud"] = json!(14400);
    let slow: SolvedTopology = serde_json::from_value(v).unwrap();
    let text = format!("{IDENTITY}#define DEFAULT_SERIAL0_BAUD 14400\n");
    assert_eq!(
        judge(&slow, &text, &[usart1]).unwrap(),
        ["WrongSerialBaud SERIAL0 [14400 baud | 14400 baud, then 14000 baud]"]
    );
    // The entry's driver pointer is not `SD1` (adversarial A11).
    let other_driver = SerialEntry {
        driver_is_instance: false,
        ..usart1
    };
    assert_eq!(
        kinds(&judge(&t, IDENTITY, &[other_driver]).unwrap()),
        ["WrongSerialPort"]
    );
}

/// The adversarial pass's forms (FEEDBACK §4): each read as GCC reads it.
#[test]
fn each_fact_is_read_as_the_compiler_reads_it() {
    let t = topology();
    let one = |from: &str, to: &str| {
        let text = IDENTITY.replacen(from, to, 1);
        assert_ne!(text, IDENTITY, "{from}");
        judge(&t, &text, &[EMPTY])
    };
    // A1: any blanks GCC allows around `#` and `define`.
    for spaced in [
        "#define  HAL_I2C_MAX_CLOCK 1000000",
        "#define\tHAL_I2C_MAX_CLOCK 1000000",
        "  #  define HAL_I2C_MAX_CLOCK 1000000",
    ] {
        let found = one("#define HAL_I2C_MAX_CLOCK 400000", spaced).unwrap();
        assert_eq!(kinds(&found), ["I2cClockAboveDeclared"], "{spaced}");
    }
    // A3: a leading zero is octal; 020000*KHZ is 8.192 MHz.
    let found = one("16*MHZ,  16*MHZ", "16*MHZ,  020000*KHZ").unwrap();
    assert_eq!(
        found,
        [
            "WrongSpiSpeed SPIDEV dataflash high [within 1 kHz at or below 16000000 Hz (SPI1's achieved_hz) | 8192000 Hz]"
        ]
    );
    // A5: ArduPilot's I2C timings overridden.
    let found = one(
        "#define HAL_I2C_MAX_CLOCK 400000",
        "#define HAL_I2C_MAX_CLOCK 400000\n#define HAL_I2C_H7_400_TIMINGR 0x00707CBB",
    )
    .unwrap();
    assert_eq!(kinds(&found), ["I2cTimingOverridden"]);
    // A9: a speed past u64 is refused by name, one at its edge is a finding.
    assert!(matches!(
        one("16*MHZ,  16*MHZ", "16*MHZ,  18446744073709552*KHZ"),
        Err(Refusal::Header { name, .. }) if name == "HAL_SPI_DEVICE0"
    ));
    let found = one("16*MHZ,  16*MHZ", "16*MHZ,  18446744073709551615").unwrap();
    assert_eq!(kinds(&found), ["WrongSpiSpeed"]);
    // A10: a listed entry defined only in a conditional the reader does not
    // evaluate is refused, never skipped.
    let err = one(
        "#define HAL_BARO_PROBE_LIST HAL_BARO_PROBE1",
        "#ifdef X\n#define HAL_BARO_PROBE2 probe_i2c_dev(AP_Baro_BMP280::probe, 0, 119); RETURN_IF_NO_SPACE;\n#endif\n#define HAL_BARO_PROBE_LIST HAL_BARO_PROBE1;HAL_BARO_PROBE2",
    )
    .unwrap_err();
    assert!(matches!(err, Refusal::Conditional { .. }), "{err:?}");
    // A list naming a macro that is not defined is refused by name.
    let err = one(
        "#define HAL_BARO_PROBE_LIST HAL_BARO_PROBE1",
        "#define HAL_BARO_PROBE_LIST HAL_BARO_PROBE1;HAL_BARO_PROBE9",
    )
    .unwrap_err();
    assert!(
        matches!(&err, Refusal::Header { name, .. } if name == "HAL_BARO_PROBE_LIST"),
        "{err:?}"
    );
}

/// A table in a form the reader does not know is refused by name, never
/// read loosely (`INV-06`).
#[test]
fn an_unknown_table_form_is_refused_by_name() {
    for (from, to, name) in [
        (
            "SPIDEV_MODE0,  16*MHZ,  16*MHZ)",
            "SPIDEV_MODE0,  16*MHZ)",
            "HAL_SPI_DEVICE0",
        ),
        ("16*MHZ,  16*MHZ", "16*GHZ,  16*MHZ", "HAL_SPI_DEVICE0"),
        (
            "#define MHZ (1000U*1000U)",
            "#define MHZ (1000000U)",
            "HAL_SPI_DEVICE0",
        ),
        (
            "probe_i2c_dev(AP_Baro_BMP280::probe, 0, 118)",
            "probe_spi_dev(AP_Baro_BMP280::probe, \"baro\")",
            "HAL_BARO_PROBE1",
        ),
        (
            "HAL_I2C_MAX_CLOCK 400000",
            "HAL_I2C_MAX_CLOCK fast",
            "HAL_I2C_MAX_CLOCK",
        ),
        (
            "{ &I2CD1, 1, SHARED_DMA_NONE",
            "{ &I2CD2, 2, SHARED_DMA_NONE",
            "HAL_I2C1_CONFIG",
        ),
    ] {
        let text = IDENTITY.replace(from, to);
        assert_ne!(text, IDENTITY, "{from}");
        let err = judge(&topology(), &text, &[EMPTY]).unwrap_err();
        assert_eq!(
            (err.kind(), err.id().as_deref()),
            ("MalformedHeader", Some(name)),
            "{to}: {err}"
        );
    }
}

/// The ELF's serial table: each entry's driver pointer is held to the
/// `SD<instance>` symbol, and bytes in its padding are refused (adversarial
/// A11).
#[test]
fn the_serial_table_is_read_with_its_driver_and_padding() {
    let mut entry = [0u8; 44];
    entry[..4].copy_from_slice(&SD1.to_le_bytes());
    entry[4] = 1;
    entry[20..24].copy_from_slice(&0x5802_0009u32.to_le_bytes());
    entry[24..28].copy_from_slice(&0x5802_000au32.to_le_bytes());
    let read = |entries: Vec<[u8; 44]>| {
        let elf = Elf {
            serial: Some(entries),
            ..Elf::default()
        };
        aethelgard_pincheck::elf::serial_table(&elf.bytes())
    };
    let got = read(vec![entry, [0; 44]]).unwrap();
    assert!(got[0].driver_is_instance && !got[0].empty);
    assert_eq!((got[0].instance, got[0].tx_line), (1, 0x5802_0009));
    assert!(got[1].empty && !got[1].driver_is_instance);
    let mut other = entry;
    other[..4].copy_from_slice(&(SD1 + 0x64).to_le_bytes());
    assert!(!read(vec![other]).unwrap()[0].driver_is_instance);
    for at in [14, 15, 42, 43] {
        let mut padded = entry;
        padded[at] = 0xAA;
        let err = read(vec![padded]).unwrap_err();
        assert_eq!(err.kind(), "MalformedElf", "byte {at}: {err:?}");
    }
}
