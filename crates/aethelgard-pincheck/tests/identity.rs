//! Tier 1: the identity judge (`AET-WO-4400b2`) on the reference topology and
//! a `hwdef.h` holding ArduPilot's device tables as it writes them, with the
//! ELF's serial table given as decoded entries. Every mismatch is named; the
//! work order's §5 cases are each here.

mod common;

use aethelgard_ir::SolvedTopology;
use aethelgard_pincheck::elf::SerialEntry;
use aethelgard_pincheck::{Refusal, header, identity};
use common::{Elf, IDENTITY, INVENSENSEV3_PROBE, SD1, sd_address, serial_bytes, topology};
use serde_json::{Value, json};

/// The IMU probe ArduPilot writes for `IMU Invensensev3 SPI:icm42688
/// ROTATION_ROLL_180_YAW_90`, as the generated sentinel-02's `hwdef.h` holds
/// it (measured, `hwdef.h` lines 140 and 144).
const IMU_PROBE: &str = "#define HAL_INS_PROBE1  ADD_BACKEND(AP_InertialSensor_Invensensev3::probe(*this,hal.spi->get_device(\"icm42688\"),ROTATION_ROLL_180_YAW_90))\n#define HAL_INS_PROBE_LIST HAL_INS_PROBE1";

const EMPTY: SerialEntry = SerialEntry {
    instance: 0,
    is_usb: false,
    tx_line: 0,
    rx_line: 0,
    empty: true,
    driver_is_instance: false,
};

/// The findings as `kind at [expected | decoded]`, or the refusal's kind,
/// the ELF holding `serial` and Invensensev3's probe.
fn judge(t: &SolvedTopology, text: &str, serial: &[SerialEntry]) -> Result<Vec<String>, Refusal> {
    judge_elf(t, text, serial, &[INVENSENSEV3_PROBE])
}

/// [`judge`], the ELF defining exactly `functions`.
fn judge_elf(
    t: &SolvedTopology,
    text: &str,
    serial: &[SerialEntry],
    functions: &[&str],
) -> Result<Vec<String>, Refusal> {
    let parts = aethelgard_ir::parts::table().unwrap();
    let pre = header::preprocess(text)?;
    let elf = Elf {
        serial: Some(serial.iter().map(serial_bytes).collect()),
        objects: serial
            .iter()
            .filter(|e| e.driver_is_instance && e.instance != 1)
            .map(|e| (format!("SD{}", e.instance), sd_address(e.instance)))
            .collect(),
        functions: functions.iter().map(|f| (*f).to_owned()).collect(),
        ..Elf::default()
    };
    Ok(identity::judge(t, parts, &pre, &elf.bytes())?
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
    v["spi_devices"]["U5"] = json!({"index": 1, "part": "ICM42688P", "bus": "FLASH", "peripheral": "SPI1", "cs": "IMU_CS",
        "orientation": {"x": "right", "z": "up"}});
    let t: SolvedTopology = serde_json::from_value(v).unwrap();
    // Its IMU line as ArduPilot writes it, right, so only the SPI facts move.
    let text = format!("{IDENTITY}\n{IMU_PROBE}\n");
    let text = text
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

/// The reference topology with U5, an ICM-42688-P on SPI1 at `PA1`, declared
/// X right and Z up (`sentinel-02-own`'s D5 declaration), and its header with
/// U5's `SPIDesc` and IMU probe as ArduPilot writes them.
fn with_imu(orientation: Option<Value>) -> (SolvedTopology, String) {
    let mut v: Value = serde_json::to_value(topology()).unwrap();
    v["gpio"]["IMU_CS"] = "PA1".into();
    v["pin_speeds"]["IMU_CS"] = 0.into();
    v["spi_devices"]["U5"] = json!({"index": 1, "part": "ICM42688P", "bus": "FLASH", "peripheral": "SPI1", "cs": "IMU_CS"});
    if let Some(o) = orientation {
        v["spi_devices"]["U5"]["orientation"] = o;
    }
    let t: SolvedTopology = serde_json::from_value(v).unwrap();
    let text = IDENTITY
        .replace(
            "#define HAL_SPI_DEVICE_LIST",
            "#define HAL_SPI_DEVICE1  SPIDesc(\"icm42688\", 0, 2, PAL_LINE(GPIOA,1U) , SPIDEV_MODE0, 16*MHZ, 16*MHZ)\n#define HAL_SPI_DEVICE_LIST",
        )
        .replace("   HAL_SPI_DEVICE0", "   HAL_SPI_DEVICE0,HAL_SPI_DEVICE1");
    (t, format!("{text}\n{IMU_PROBE}\n"))
}

fn declared() -> (SolvedTopology, String) {
    with_imu(Some(json!({"x": "right", "z": "up"})))
}

/// `AET-WO-4400d` §2: the declared IMU's line passes; each mutation is
/// named: the line removed, the wrong rotation, the wrong SPI device, the
/// wrong driver, and an IMU with no declared orientation.
#[test]
fn the_imu_probe_is_judged_against_the_declared_orientation() {
    let (t, text) = declared();
    assert_eq!(judge(&t, &text, &[EMPTY]).unwrap(), Vec::<String>::new());
    let probe = "ROTATION_ROLL_180_YAW_90))";
    let cases: [(&str, &str, Vec<&str>); 6] = [
        // Removed: the list is empty.
        (
            "#define HAL_INS_PROBE_LIST HAL_INS_PROBE1",
            "",
            vec!["MissingImuProbe"],
        ),
        // ROTATION_ROLL_180 is forward = top with X forward: a quarter turn.
        (probe, "ROTATION_ROLL_180))", vec!["WrongImuRotation"]),
        (probe, "ROTATION_NONE))", vec!["WrongImuRotation"]),
        // Ruled D4: the alias of 10 is the same rotation.
        (probe, "ROTATION_PITCH_180_YAW_270))", vec![]),
        (
            "get_device(\"icm42688\")",
            "get_device(\"dataflash\")",
            vec!["WrongImuProbe"],
        ),
        (
            "AP_InertialSensor_Invensensev3::probe",
            "AP_InertialSensor_Invensense::probe",
            vec!["WrongImuProbe", "ImuDriverNotLinked"],
        ),
    ];
    for (from, to, want) in cases {
        let changed = text.replacen(from, to, 1);
        assert_ne!(changed, text, "{from}");
        let found = judge(&t, &changed, &[EMPTY]).unwrap();
        assert_eq!(kinds(&found), want, "{to}: {found:?}");
    }
    // Declared otherwise: X forward is ROTATION_ROLL_180, so the line's
    // ROLL_180_YAW_90 is a quarter turn wrong.
    let (t, text) = with_imu(Some(json!({"x": "forward", "z": "up"})));
    let found = judge(&t, &text, &[EMPTY]).unwrap();
    assert_eq!(kinds(&found), ["WrongImuRotation"], "{found:?}");
    assert!(
        found[0].contains("[ROTATION_ROLL_180 | ROTATION_ROLL_180_YAW_90]"),
        "{found:?}"
    );
    // No declared orientation: no rotation is right (INV-06).
    let (t, text) = with_imu(None);
    let found = judge(&t, &text, &[EMPTY]).unwrap();
    assert_eq!(kinds(&found), ["WrongImuRotation"], "{found:?}");
    assert!(found[0].contains("an undeclared orientation"), "{found:?}");
    // A probe the topology has no IMU for.
    let text = format!("{IDENTITY}\n{IMU_PROBE}\n");
    let found = judge(&topology(), &text, &[EMPTY]).unwrap();
    assert_eq!(kinds(&found), ["UnexpectedImuProbe"], "{found:?}");
}

/// The ELF must define the driver's probe under its mangled name: a
/// `hwdef.h` with an IMU line paired with an ELF built without it is named.
#[test]
fn the_imu_driver_must_be_linked_into_the_elf() {
    let (t, text) = declared();
    let found = judge_elf(&t, &text, &[EMPTY], &[]).unwrap();
    assert_eq!(kinds(&found), ["ImuDriverNotLinked"], "{found:?}");
    let other = INVENSENSEV3_PROBE.replace("8Rotation", "8Rotationx");
    let found = judge_elf(&t, &text, &[EMPTY], &[&other]).unwrap();
    assert_eq!(kinds(&found), ["ImuDriverNotLinked"], "{found:?}");
}

/// The list is read as the compiler expands it: a numbered probe the list
/// does not name is not read, and one it names through another macro is
/// (adversarial A4's class).
#[test]
fn the_imu_list_is_read_as_the_compiler_expands_it() {
    let (t, text) = declared();
    let stray = text.replace(
        "#define HAL_INS_PROBE_LIST",
        "#define HAL_INS_PROBE2  ADD_BACKEND(AP_InertialSensor_Invensensev3::probe(*this,hal.spi->get_device(\"icm42688\"),ROTATION_NONE))\n#define HAL_INS_PROBE_LIST",
    );
    assert_eq!(judge(&t, &stray, &[EMPTY]).unwrap(), Vec::<String>::new());
    let named = stray.replace(
        "#define HAL_INS_PROBE_LIST HAL_INS_PROBE1",
        "#define MY_PROBES HAL_INS_PROBE2\n#define HAL_INS_PROBE_LIST MY_PROBES",
    );
    let found = judge(&t, &named, &[EMPTY]).unwrap();
    assert_eq!(kinds(&found), ["WrongImuRotation"], "{found:?}");
    let twice = text.replace(
        "#define HAL_INS_PROBE_LIST HAL_INS_PROBE1",
        "#define HAL_INS_PROBE_LIST HAL_INS_PROBE1;HAL_INS_PROBE1",
    );
    let found = judge(&t, &twice, &[EMPTY]).unwrap();
    assert_eq!(kinds(&found), ["UnexpectedImuProbe"], "{found:?}");
}

/// A probe in a form this reader does not know, a rotation that is not a
/// right-angle enumerator, or a rotation redefined as a macro, is refused
/// by name, never read as something else.
#[test]
fn an_unknown_probe_form_or_rotation_is_refused() {
    let (t, text) = declared();
    for (from, to) in [
        ("ROTATION_ROLL_180_YAW_90", "ROTATION_YAW_45"),
        ("ROTATION_ROLL_180_YAW_90", "ROTATION_CUSTOM_1"),
        ("ROTATION_ROLL_180_YAW_90", "10"),
        ("ROTATION_ROLL_180_YAW_90", "(Rotation)10"),
        ("ADD_BACKEND(", "ADD_BACKEND_INSTANCE("),
        (
            "hal.spi->get_device(\"icm42688\")",
            "GET_I2C_DEVICE(0,0x68)",
        ),
        ("*this,", "*that,"),
    ] {
        let changed = text.replacen(from, to, 1);
        assert_ne!(changed, text, "{from}");
        let err = judge(&t, &changed, &[EMPTY]).unwrap_err();
        assert_eq!(err.kind(), "MalformedHeader", "{to}: {err:?}");
    }
    let redefined = format!("{text}#define ROTATION_ROLL_180_YAW_90 ROTATION_NONE\n");
    let err = judge(&t, &redefined, &[EMPTY]).unwrap_err();
    assert_eq!(err.kind(), "MalformedHeader", "{err:?}");
}

/// The probe must be a function the ELF defines: a data object of that name
/// is not the driver (adversarial: a symbol of the right name, the wrong kind).
#[test]
fn an_object_named_like_the_probe_is_not_the_driver() {
    let (t, text) = declared();
    let parts = aethelgard_ir::parts::table().unwrap();
    let pre = header::preprocess(&text).unwrap();
    let elf = Elf {
        serial: Some(vec![serial_bytes(&EMPTY)]),
        objects: vec![(INVENSENSEV3_PROBE.to_owned(), 0x0800_2000)],
        ..Elf::default()
    };
    let found = identity::judge(&t, parts, &pre, &elf.bytes()).unwrap();
    let kinds: Vec<&str> = found.iter().map(|f| f.kind).collect();
    assert_eq!(kinds, ["ImuDriverNotLinked"], "{found:?}");
}

/// The adversarial pass's break (`AET-WO-4400d`): a probe redefined with the
/// digraph `%:` for `#`, after an `#undef`, compiled as `ROTATION_ROLL_180`
/// while the reader saw the first definition. `%:define` is now read as
/// `#define` (a second definition is refused), and any compiled `#undef` or
/// `#pragma` naming a name the judge reads is refused: it could stand in
/// another definition. A header written with `%:` alone reads as with `#`.
#[test]
fn a_digraph_undef_or_pragma_cannot_hide_a_definition() {
    let (t, text) = declared();
    let redefine = "%:define HAL_INS_PROBE1 ADD_BACKEND(AP_InertialSensor_Invensensev3::probe(*this,hal.spi->get_device(\"icm42688\"),ROTATION_ROLL_180))";
    for extra in [
        format!("#undef HAL_INS_PROBE1\n{redefine}"),
        redefine.to_owned(),
        " %: undef HAL_INS_PROBE_LIST".to_owned(),
        "#pragma push_macro(\"HAL_INS_PROBE1\")".to_owned(),
        "#undef HAL_SPI_DEVICE_LIST".to_owned(),
    ] {
        let err = judge(&t, &format!("{text}{extra}\n"), &[EMPTY]).unwrap_err();
        assert_eq!(err.kind(), "MalformedHeader", "{extra}: {err:?}");
    }
    let digraph = text.replace("#define HAL_INS_PROBE1 ", "%:  define HAL_INS_PROBE1 ");
    assert_ne!(digraph, text);
    assert_eq!(judge(&t, &digraph, &[EMPTY]).unwrap(), Vec::<String>::new());
    let wrong = digraph.replace("ROTATION_ROLL_180_YAW_90", "ROTATION_ROLL_180");
    assert_eq!(
        kinds(&judge(&t, &wrong, &[EMPTY]).unwrap()),
        ["WrongImuRotation"]
    );
}

/// `HAL_SITL_INVENSENSEV3` compiles a probe the list does not name
/// (`AP_InertialSensor.cpp` L1223-1225); it is named, not passed.
#[test]
fn the_sitl_probe_define_is_named() {
    let (t, text) = declared();
    let found = judge(
        &t,
        &format!("{text}#define HAL_SITL_INVENSENSEV3 1\n"),
        &[EMPTY],
    )
    .unwrap();
    assert_eq!(kinds(&found), ["UnexpectedImuProbe"], "{found:?}");
}

/// `AET-WO-4404`: a topology with one port per `(role, inverted)`, in
/// order, the first the console; port `i` on U(S)ART `i + 1`, its TX and RX
/// on port C onwards; with the ELF's serial table to match, and a `hwdef.h`
/// holding each non-RC port's baud (115200, SERIAL0's default) and nothing
/// else of the serial ports.
fn with_ports(ports: &[(&str, bool)]) -> (SolvedTopology, Vec<SerialEntry>, String) {
    let mut v: Value = serde_json::to_value(topology()).unwrap();
    let mut entries = Vec::new();
    let mut text = IDENTITY.to_owned();
    for (i, (role, inverted)) in ports.iter().enumerate() {
        let instance = u8::try_from(i + 1).unwrap();
        let peripheral = format!("USART{instance}");
        let (port, pin) = (2 + i / 8, (2 * i) % 16);
        let letter = char::from(b'A' + u8::try_from(port).unwrap());
        let label = format!("P{i}");
        for (sig, p) in [("TX", pin), ("RX", pin + 1)] {
            v["assignments"][format!("{label}_{sig}")] = json!({"pin": format!("P{letter}{p}"),
                "peripheral": peripheral, "signal": format!("{peripheral}_{sig}"), "alternate_function": 7});
        }
        let (baud, parity, stop) = match *role {
            "rc_sbus" => (100_000, "even", 2),
            "rc_crsf" => (420_000, "none", 1),
            _ => (115_200, "none", 1),
        };
        v["serial_ports"][&label] = json!({"index": i, "peripheral": peripheral,
            "tx": format!("{label}_TX"), "rx": format!("{label}_RX"), "role": role,
            "framing": {"baud": baud, "data_bits": 8, "parity": parity, "stop_bits": stop, "inverted": inverted},
            "negotiated": null, "peer_deviation_ppm": 5000, "kernel_hz": 64000000, "brr": 556,
            "actual_millibaud": 115107913, "error_ppm": -799});
        let line = 0x5802_0000 + 0x400 * u32::try_from(port).unwrap() + u32::try_from(pin).unwrap();
        entries.push(SerialEntry {
            instance,
            is_usb: false,
            tx_line: line,
            rx_line: line + 1,
            empty: false,
            driver_is_instance: true,
        });
        if i > 0 && !role.starts_with("rc_") {
            text.push_str(&format!("#define DEFAULT_SERIAL{i}_BAUD 115200\n"));
        }
    }
    (serde_json::from_value(v).unwrap(), entries, text)
}

/// fc-base's ports (FEEDBACK §2.3): the console, GPS, CRSF, and GPS.
const FC_BASE: [(&str, bool); 4] = [
    ("console", false),
    ("gps", false),
    ("rc_crsf", false),
    ("gps", false),
];

/// fc-base's serial defines as ArduPilot writes them to `hwdef.h`
/// (measured, FEEDBACK §2.3).
const FC_BASE_DEFINES: &str = "\
#define DEFAULT_SERIAL1_PROTOCOL SerialProtocol_GPS
#define DEFAULT_SERIAL1_OPTIONS 0
#define DEFAULT_SERIAL2_PROTOCOL SerialProtocol_RCIN
#define DEFAULT_SERIAL3_PROTOCOL SerialProtocol_GPS
#define DEFAULT_SERIAL3_OPTIONS 0
#define HAL_HAVE_SERIAL0_PARAMS 1
#define HAL_HAVE_SERIAL1_PARAMS 1
#define HAL_HAVE_SERIAL2_PARAMS 1
#define HAL_HAVE_SERIAL3_PARAMS 1
";

/// The serial findings for `ports` with `defines` added to the header.
fn serial(ports: &[(&str, bool)], defines: &str) -> Result<Vec<String>, Refusal> {
    let (t, entries, text) = with_ports(ports);
    judge(&t, &format!("{text}{defines}"), &entries)
}

/// fc-base passes; each §1.1 (c) mutation is named, `adv-proto-swap` twice
/// (FEEDBACK §2.3, measured on the built boards).
#[test]
fn each_serial_port_s_protocol_and_options_are_judged() {
    assert_eq!(
        serial(&FC_BASE, FC_BASE_DEFINES).unwrap(),
        Vec::<String>::new()
    );
    let swap = FC_BASE_DEFINES
        .replace(
            "SERIAL1_PROTOCOL SerialProtocol_GPS",
            "SERIAL1_PROTOCOL SerialProtocol_RCIN",
        )
        .replace(
            "SERIAL2_PROTOCOL SerialProtocol_RCIN",
            "SERIAL2_PROTOCOL SerialProtocol_GPS",
        );
    assert_eq!(
        serial(&FC_BASE, &swap).unwrap(),
        [
            "WrongSerialProtocol SERIAL1 [SerialProtocol_GPS (5), for a gps port | SerialProtocol_RCIN (23)]",
            "WrongSerialProtocol SERIAL2 [SerialProtocol_RCIN (23), for a rc_crsf port | SerialProtocol_GPS (5)]"
        ]
    );
    for (options, shown) in [("8", "0x8"), ("3", "0x3")] {
        let text =
            FC_BASE_DEFINES.replace("SERIAL1_OPTIONS 0", &format!("SERIAL1_OPTIONS {options}"));
        assert_eq!(
            serial(&FC_BASE, &text).unwrap(),
            [format!("WrongSerialOptions SERIAL1 [none | {shown}]")]
        );
    }
    // A deleted define runs the tag's default for its n: MAVLink 2 on
    // SERIAL1; GPS on SERIAL3, where a MAVLink port stands.
    let text = FC_BASE_DEFINES.replace("#define DEFAULT_SERIAL1_PROTOCOL SerialProtocol_GPS\n", "");
    assert_eq!(
        kinds(&serial(&FC_BASE, &text).unwrap()),
        ["WrongSerialProtocol"]
    );
    let mav3 = [FC_BASE[0], FC_BASE[1], FC_BASE[2], ("mavlink", false)];
    let text = FC_BASE_DEFINES.replace(
        "SERIAL3_PROTOCOL SerialProtocol_GPS",
        "SERIAL3_PROTOCOL SerialProtocol_MAVLink2",
    );
    assert_eq!(serial(&mav3, &text).unwrap(), Vec::<String>::new());
    let text = FC_BASE_DEFINES.replace("#define DEFAULT_SERIAL3_PROTOCOL SerialProtocol_GPS\n", "");
    assert_eq!(
        serial(&mav3, &text).unwrap(),
        [
            "WrongSerialProtocol SERIAL3 [SerialProtocol_MAVLink2 (2), for a mavlink port | SerialProtocol_GPS (5)]"
        ]
    );
}

/// A protocol as GCC reads it: an enumerator by name, or a C integer
/// constant in any radix; a name the enum lacks, a qualified name, a
/// negative, a macro standing in for the enumerator, or an `#undef` are
/// refused by name; a define under `#if 0` is not compiled.
#[test]
fn each_serial_protocol_form_reads_as_the_compiler_reads_it() {
    let one = |v: &str| {
        serial(
            &FC_BASE,
            &FC_BASE_DEFINES.replace(
                "SERIAL1_PROTOCOL SerialProtocol_GPS",
                &format!("SERIAL1_PROTOCOL {v}"),
            ),
        )
    };
    for v in ["SerialProtocol_GPS", "5", "05", "0x5", "5U", " 5 "] {
        assert_eq!(one(v).unwrap(), Vec::<String>::new(), "{v}");
    }
    for v in [
        "6",
        "SerialProtocol_GPS2",
        "SerialProtocol_MAVLink",
        "SerialProtocol_None",
        "1",
    ] {
        assert_eq!(kinds(&one(v).unwrap()), ["WrongSerialProtocol"], "{v}");
    }
    for v in [
        "SerialProtocol_Nope",
        "AP_SerialManager::SerialProtocol_GPS",
        "-1",
        "(5)",
        "09",
        "GPS_PROTO",
    ] {
        match one(v) {
            Err(Refusal::Header { name, .. }) => {
                assert_eq!(name, "DEFAULT_SERIAL1_PROTOCOL", "{v}")
            }
            other => panic!("{v}: {other:?}"),
        }
    }
    let defines = format!("{FC_BASE_DEFINES}#define SerialProtocol_GPS 23\n");
    match serial(&FC_BASE, &defines) {
        Err(Refusal::Header { name, .. }) => assert_eq!(name, "SerialProtocol_GPS"),
        other => panic!("{other:?}"),
    }
    let undef = format!("{FC_BASE_DEFINES}#undef DEFAULT_SERIAL1_PROTOCOL\n");
    assert!(matches!(
        serial(&FC_BASE, &undef),
        Err(Refusal::Header { .. })
    ));
    let dead = FC_BASE_DEFINES.replace(
        "#define DEFAULT_SERIAL1_PROTOCOL SerialProtocol_GPS\n",
        "#if 0\n#define DEFAULT_SERIAL1_PROTOCOL SerialProtocol_GPS\n#endif\n",
    );
    assert_eq!(
        kinds(&serial(&FC_BASE, &dead).unwrap()),
        ["WrongSerialProtocol"]
    );
}

/// An absent define runs the tag's `#ifndef` default for its n
/// (`AP_SerialManager.cpp:47-152`): MAVLink 2 on 1 and 2, GPS on 3 and 4,
/// none on 5 to 9. A port past SERIAL9 has no parameters at all.
#[test]
fn an_absent_protocol_runs_the_default_for_its_n() {
    let at = |found: Vec<String>| -> Vec<String> {
        found
            .iter()
            .map(|f| f.split(' ').take(2).collect::<Vec<_>>().join(" "))
            .collect()
    };
    let mut ports = vec![("console", false)];
    ports.extend([("gps", false); 9]);
    assert_eq!(
        at(serial(&ports, "").unwrap()),
        [
            "SERIAL1", "SERIAL2", "SERIAL5", "SERIAL6", "SERIAL7", "SERIAL8", "SERIAL9"
        ]
        .map(|n| format!("WrongSerialProtocol {n}"))
    );
    let mut ports = vec![("console", false)];
    ports.extend([("mavlink", false); 9]);
    assert_eq!(
        at(serial(&ports, "").unwrap()),
        [
            "SERIAL3", "SERIAL4", "SERIAL5", "SERIAL6", "SERIAL7", "SERIAL8", "SERIAL9"
        ]
        .map(|n| format!("WrongSerialProtocol {n}"))
    );
    let found = serial(&ports, "").unwrap();
    assert!(found.contains(&"WrongSerialProtocol SERIAL5 [SerialProtocol_MAVLink2 (2), for a mavlink port | SerialProtocol_None (-1)]".to_owned()), "{found:?}");
    // The ELF's table holds ten entries at most (`elf.rs`), so the eleventh
    // port is also missing from it.
    ports.push(("mavlink", false));
    let (t, mut entries, text) = with_ports(&ports);
    entries.truncate(10);
    let found = judge(&t, &text, &entries).unwrap();
    assert!(kinds(&found).contains(&"MissingSerialPort"), "{found:?}");
    assert_eq!(
        found.last().unwrap(),
        "MissingSerialParameters SERIAL10 [SERIAL10_PROTOCOL and _OPTIONS | none: ArduPilot's parameter table stops at SERIAL9 (AP_SerialManager.cpp:379-395)]"
    );
}

/// SERIAL7 and 8 read a literal 0 for their options (`AP_SerialManager.cpp`
/// `:357`, `:376`): a define there does nothing and is not even read.
/// SERIAL9's is read.
#[test]
fn serial7_and_8_options_are_never_read() {
    let mut ports = vec![("console", false)];
    ports.extend([("gps", false); 9]);
    let protocols: String = (1..=9)
        .map(|n| format!("#define DEFAULT_SERIAL{n}_PROTOCOL SerialProtocol_GPS\n"))
        .collect();
    assert_eq!(serial(&ports, &protocols).unwrap(), Vec::<String>::new());
    let ignored = format!(
        "{protocols}#define DEFAULT_SERIAL7_OPTIONS 8\n#define DEFAULT_SERIAL8_OPTIONS (not a number)\n"
    );
    assert_eq!(serial(&ports, &ignored).unwrap(), Vec::<String>::new());
    let read = format!("{protocols}#define DEFAULT_SERIAL9_OPTIONS 8\n");
    assert_eq!(
        serial(&ports, &read).unwrap(),
        ["WrongSerialOptions SERIAL9 [none | 0x8]"]
    );
    let malformed = format!("{protocols}#define DEFAULT_SERIAL6_OPTIONS 8u8\n");
    assert!(matches!(
        serial(&ports, &malformed),
        Err(Refusal::Header { .. })
    ));
}

/// Every option bit that moves a pin or a wire, on a port that is not RC
/// input and on one that is: only RXINV on RC input passes; an inverted
/// port needs RXINV and TXINV both.
#[test]
fn each_option_bit_is_judged() {
    let crsf = [("console", false), ("rc_crsf", false)];
    let sbus = [("console", false), ("rc_sbus", true)];
    let gps = [("console", false), ("gps", false)];
    let inverted = [("console", false), ("gps", true)];
    let with = |ports: &[(&str, bool)], protocol: &str, options: u32| {
        serial(
            ports,
            &format!("#define DEFAULT_SERIAL1_PROTOCOL {protocol}\n#define DEFAULT_SERIAL1_OPTIONS {options}\n"),
        )
        .unwrap()
    };
    for ports in [&crsf[..], &sbus[..]] {
        assert_eq!(with(ports, "SerialProtocol_RCIN", 0), Vec::<String>::new());
        assert_eq!(with(ports, "SerialProtocol_RCIN", 1), Vec::<String>::new());
        for bit in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] {
            assert_eq!(
                kinds(&with(ports, "SerialProtocol_RCIN", 1 << bit)),
                ["WrongSerialOptions"],
                "{ports:?} bit {bit}"
            );
        }
    }
    assert_eq!(
        with(&sbus, "SerialProtocol_RCIN", 2),
        [
            "WrongSerialOptions SERIAL1 [no bit but RXINV, which RC input sets (AP_RCProtocol.cpp:326-336) | 0x2]"
        ]
    );
    for bit in 0..=12 {
        assert_eq!(
            kinds(&with(&gps, "SerialProtocol_GPS", 1 << bit)),
            ["WrongSerialOptions"],
            "bit {bit}"
        );
    }
    assert_eq!(
        with(&inverted, "SerialProtocol_GPS", 3),
        Vec::<String>::new()
    );
    for options in [0, 1, 2, 7] {
        assert_eq!(
            kinds(&with(&inverted, "SerialProtocol_GPS", options)),
            ["WrongSerialOptions"],
            "{options}"
        );
    }
}

/// ArduPilot attaches one RC input port and drops the next
/// (`AP_SerialManager.cpp:536-541`), even where the topology asks for both.
#[test]
fn a_second_rc_input_port_is_named() {
    let two = [("console", false), ("rc_crsf", false), ("rc_sbus", true)];
    let defines = "#define DEFAULT_SERIAL1_PROTOCOL SerialProtocol_RCIN\n#define DEFAULT_SERIAL2_PROTOCOL 23\n";
    assert_eq!(
        serial(&two, defines).unwrap(),
        [
            "RcInputTwice SERIAL2 [one RCIN port, SERIAL1 (AP_SerialManager.cpp:536-541) | SerialProtocol_RCIN (23)]"
        ]
    );
    // By define: a GPS port compiled as RC input beside the CRSF port.
    let swap = FC_BASE_DEFINES.replace(
        "SERIAL1_PROTOCOL SerialProtocol_GPS",
        "SERIAL1_PROTOCOL SerialProtocol_RCIN",
    );
    assert_eq!(
        kinds(&serial(&FC_BASE, &swap).unwrap()),
        ["WrongSerialProtocol", "RcInputTwice"]
    );
}

/// SERIAL0 has no options parameter, so a console declared inverted runs
/// not inverted; its protocol is never read (`AP_SerialManager.cpp:180`).
#[test]
fn the_console_s_protocol_is_not_read_and_its_inversion_is_named() {
    let found = serial(&[("console", true)], "").unwrap();
    assert_eq!(
        found,
        [
            "WrongSerialOptions SERIAL0 [RXINV and TXINV (declared inverted) | none: SERIAL0 has no options parameter (AP_SerialManager.cpp:165-181)]"
        ]
    );
    let ignored =
        "#define DEFAULT_SERIAL0_PROTOCOL SerialProtocol_GPS\n#define DEFAULT_SERIAL0_OPTIONS 3\n";
    assert_eq!(
        serial(&[("console", false)], ignored).unwrap(),
        Vec::<String>::new()
    );
}

/// A port's parameters compile only where `HAL_HAVE_SERIALn_PARAMS` does
/// (`AP_SerialManager.cpp:183` … `:379`): any value but 1, or a second
/// definition, is refused by name.
#[test]
fn a_port_whose_parameters_are_not_compiled_is_refused() {
    let off = FC_BASE_DEFINES.replace("HAL_HAVE_SERIAL2_PARAMS 1", "HAL_HAVE_SERIAL2_PARAMS 0");
    match serial(&FC_BASE, &off) {
        Err(Refusal::Header { name, .. }) => assert_eq!(name, "HAL_HAVE_SERIAL2_PARAMS"),
        other => panic!("{other:?}"),
    }
    let twice = format!("{FC_BASE_DEFINES}#define HAL_HAVE_SERIAL1_PARAMS 0\n");
    match serial(&FC_BASE, &twice) {
        Err(Refusal::Header { name, .. }) => assert_eq!(name, "HAL_HAVE_SERIAL1_PARAMS"),
        other => panic!("{other:?}"),
    }
}
