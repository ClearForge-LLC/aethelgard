//! Tier 1: the identity judge (`AET-WO-4400b2`) on the reference topology and
//! a `hwdef.h` holding ArduPilot's device tables as it writes them, with the
//! ELF's serial table given as decoded entries and its serial parameters
//! (`var_info`, `AET-WO-4405`) as rows. Every mismatch is named; the work
//! order's §5 cases are each here.

mod common;

use aethelgard_ir::SolvedTopology;
use aethelgard_pincheck::elf::{self, SerialEntry, SerialParam};
use aethelgard_pincheck::{Refusal, header, identity};
use common::{
    Elf, IDENTITY, INVENSENSEV3_PROBE, Port, SD1, SERIAL0_ONLY, VarRow, gps_var_info, group_end,
    sd_address, serial_bytes, var_info, var_row,
};
use serde_json::{Value, json};

/// The reference topology (`common::topology`, as `solve` gives it) with its
/// console taken out (`AET-WO-4410` R9): these tests declare every serial
/// port they judge, each against an ELF serial table they write, and the
/// reference's own console would be a port none of them declared.
fn topology() -> SolvedTopology {
    let mut t = common::topology();
    for (_, p) in std::mem::take(&mut t.serial_ports) {
        for net in [p.tx, p.rx] {
            t.assignments.remove(&net);
            t.pin_speeds.remove(&net);
        }
    }
    t
}

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
/// the ELF holding `serial`, SERIAL0's parameters alone, and Invensensev3's
/// probe.
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
    judge_with(t, text, serial, var_info(&SERIAL0_ONLY), functions)
}

/// [`judge_elf`], the ELF's `var_info` holding `rows`, and its
/// `AP_GPS::var_info` two receivers, as every measured board's.
fn judge_with(
    t: &SolvedTopology,
    text: &str,
    serial: &[SerialEntry],
    rows: Vec<VarRow>,
    functions: &[&str],
) -> Result<Vec<String>, Refusal> {
    judge_gps(t, text, serial, rows, Some(gps_var_info(2)), functions)
}

/// [`judge_with`], the ELF's `AP_GPS::var_info` holding `gps` (`None`: no
/// such symbol).
fn judge_gps(
    t: &SolvedTopology,
    text: &str,
    serial: &[SerialEntry],
    rows: Vec<VarRow>,
    gps: Option<Vec<VarRow>>,
    functions: &[&str],
) -> Result<Vec<String>, Refusal> {
    let parts = aethelgard_ir::parts::table().unwrap();
    let pre = header::preprocess(text)?;
    let elf = Elf {
        var_info: Some(rows),
        gps,
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
    v["gpio"]["IMU_CS"] = json!({"pin": "PA1", "direction": "output", "initial": "high"});
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
        v["motor_outputs"][&net] = json!({"index": i, "protocol": "dshot600", "pwm": null, "psc": 0, "arr": 107, "ccr": 40, "pull": {"fitment": "fitted", "resistor": format!("R{}", 10 + i), "ohms": 442_000, "link": format!("R{}", 20 + i), "floor_ohms": 422_561, "ceiling_ohms": 895_714}});
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
    // SERIAL0 runs at `0_BAUD`, the define / 1000 as var_info holds it,
    // mapped; the define itself in hwdef.h is not read (FEEDBACK D1).
    let console = |define: &str, row: i32| {
        judge_with(
            &t,
            &format!("{IDENTITY}#define DEFAULT_SERIAL0_BAUD {define}\n"),
            &[usart1],
            var_info(&[(2, row, None)]),
            &[INVENSENSEV3_PROBE],
        )
        .unwrap()
    };
    assert_eq!(
        console("921600", 921),
        ["WrongSerialBaud SERIAL0 [115200 baud | 921600 baud]"]
    );
    // The row is judged, whatever the header says: a header of 921600 beside
    // a row of 115 passes, a header of 115200 beside a row of 921 does not.
    assert_eq!(console("921600", 115), Vec::<String>::new());
    assert_eq!(
        console("115200", 921),
        ["WrongSerialBaud SERIAL0 [115200 baud | 921600 baud]"]
    );
    // The residual (D1): 115999 / 1000 is 115, the row of 115200, so the
    // boot open at 115999 (`AP_SerialManager.cpp:419`) is not seen.
    assert_eq!(console("115999", 115), Vec::<String>::new());
    let mut v: Value = serde_json::to_value(&t).unwrap();
    v["serial_ports"]["CONSOLE"]["framing"]["baud"] = json!(14400);
    let slow: SolvedTopology = serde_json::from_value(v).unwrap();
    assert_eq!(
        judge_with(
            &slow,
            &format!("{IDENTITY}#define DEFAULT_SERIAL0_BAUD 14400\n"),
            &[usart1],
            var_info(&[(2, 14, None)]),
            &[INVENSENSEV3_PROBE],
        )
        .unwrap(),
        ["WrongSerialBaud SERIAL0 [14400 baud | 14000 baud]"]
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
    v["gpio"]["IMU_CS"] = json!({"pin": "PA1", "direction": "output", "initial": "high"});
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
        var_info: Some(var_info(&SERIAL0_ONLY)),
        gps: Some(gps_var_info(2)),
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

/// `AET-WO-4410` (5), review 3 A15: a function-like `#define` of a name the
/// judge reads is refused by name, wherever the judge reads it: a single
/// value (`HAL_SITL_INVENSENSEV3`, `Preprocessed::value`), a bus entry
/// (`HAL_SPI1_CONFIG`, `candidates`). Read as undefined, the first would
/// pass a SITL probe and the second refuse for the wrong reason. The
/// adversarial spellings are read as GCC reads them: a blank or a comment
/// before `(` is object-like; `%:` and a continuation are not escapes.
#[test]
fn a_function_like_define_of_a_name_the_judge_reads_is_refused() {
    let (t, text) = declared();
    let refused = |extra: &str| {
        let err = judge(&t, &format!("{text}{extra}"), &[EMPTY]).unwrap_err();
        (err.kind(), err.id(), err.to_string())
    };
    for extra in [
        "#define HAL_SITL_INVENSENSEV3(x) 1\n",
        "%:define HAL_SITL_INVENSENSEV3(x) 1\n",
        "#define HAL_SITL_INVENSENSEV3\\\n(x) 1\n",
    ] {
        let (kind, id, message) = refused(extra);
        assert_eq!(kind, "MalformedHeader", "{extra}");
        assert_eq!(id.as_deref(), Some("HAL_SITL_INVENSENSEV3"), "{extra}");
        assert!(message.contains("function-like"), "{extra}: {message}");
    }
    // Object-like, as GCC reads them: the probe is defined, and named.
    for extra in [
        "#define HAL_SITL_INVENSENSEV3 (x)\n",
        "#define HAL_SITL_INVENSENSEV3/**/(x)\n",
    ] {
        let found = judge(&t, &format!("{text}{extra}"), &[EMPTY]).unwrap();
        assert_eq!(kinds(&found), ["UnexpectedImuProbe"], "{extra}: {found:?}");
    }
    // A bus entry read through `candidates`.
    let bus = text.replace("#define HAL_SPI1_CONFIG {", "#define HAL_SPI1_CONFIG(x) {");
    assert_ne!(bus, text);
    let err = judge(&t, &bus, &[EMPTY]).unwrap_err();
    assert_eq!(
        (err.kind(), err.id().as_deref()),
        ("MalformedHeader", Some("HAL_SPI1_CONFIG"))
    );
}

/// The reference topology with a battery sense on PC0, ADC1 INP10 (or the
/// given ADC input), and the divider `top` over `bottom`.
fn with_battery(top: u32, bottom: u32, peripheral: &str) -> SolvedTopology {
    let mut v: Value = serde_json::to_value(topology()).unwrap();
    v["power"] = json!({
        "domains": {}, "input_steady_milliamps": 0, "input_fault_milliamps": 0,
        "conditions": [], "sense": {"VBAT_SENSE": {"pin": "PC0", "position": 26,
            "channels": ["ADC1_INP10", "ADC2_INP10", "ADC3_INP10"],
            "top_ohms": top, "bottom_ohms": bottom}}
    });
    v["adc_inputs"] =
        json!({"VBAT_SENSE": {"pin": "PC0", "peripheral": peripheral, "channel": "INP10"}});
    serde_json::from_value(v).unwrap()
}

/// ArduPilot's battery lines in `hwdef.h` for PC0 on ADC1 INP10, as the
/// `s03adc1` build wrote them (`spike/4410`), with `scale` as given.
fn battery_header(scale: &str) -> String {
    format!(
        "{IDENTITY}#define HAL_BATT_VOLT_PIN 10\n#define HAL_BATT_VOLT_SCALE {scale}\n#define HAL_BATT_MONITOR_DEFAULT 3\n#define HAL_ANALOG_PINS \\\n{{ 10, 10,    3.30/4096 }}, /* PC0 BATT_VOLTAGE_SENS */ \\\n\n"
    )
}

/// `AET-WO-4410` (1): the battery monitor is judged against the solved sense
/// by the check's own arithmetic. The generated quotient passes, and so does
/// any literal of exactly that value; each wrong fact is named.
#[test]
fn the_battery_monitor_is_judged_against_the_solved_sense() {
    let t = with_battery(100_000, 10_000, "ADC1");
    for scale in [
        "(110000.0f/10000.0f)",
        "11.0",
        "11",
        "(11.0f/1.0f)",
        "11.000f",
    ] {
        assert_eq!(
            judge(&t, &battery_header(scale), &[EMPTY]).unwrap(),
            Vec::<String>::new(),
            "{scale}"
        );
    }
    let good = battery_header("(110000.0f/10000.0f)");
    let cases: [(&str, &str, &str); 8] = [
        (
            "#define HAL_BATT_VOLT_PIN 10\n",
            "#define HAL_BATT_VOLT_PIN 11\n",
            "WrongBattVoltPin",
        ),
        ("#define HAL_BATT_VOLT_PIN 10\n", "", "WrongBattVoltPin"),
        ("(110000.0f/10000.0f)", "11.01", "WrongBattVoltScale"),
        (
            "(110000.0f/10000.0f)",
            "(110000.0f/1000.0f)",
            "WrongBattVoltScale",
        ),
        (
            "#define HAL_BATT_MONITOR_DEFAULT 3\n",
            "#define HAL_BATT_MONITOR_DEFAULT 4\n",
            "WrongBattMonitorType",
        ),
        (
            "{ 10, 10,    3.30/4096 }",
            "{ 10, 10, 11*3.30/4096 }",
            "WrongAnalogPin",
        ),
        (
            "{ 10, 10,    3.30/4096 }",
            "{ 10, 11,    3.30/4096 }",
            "WrongAnalogPin",
        ),
        (
            "#define HAL_BATT_VOLT_PIN 10\n",
            "#define HAL_BATT_VOLT_PIN 10\n#define HAL_USE_ADC FALSE\n",
            "AdcOff",
        ),
    ];
    for (from, to, kind) in cases {
        let text = good.replacen(from, to, 1);
        assert_ne!(text, good, "{kind}: {from}");
        assert_eq!(kinds(&judge(&t, &text, &[EMPTY]).unwrap()), [kind], "{to}");
    }
}

/// §5: 100 k over 9.09 k is 12.0011001100…, which no short decimal states.
/// The quotient passes, a reduced quotient of the same value passes, and
/// ArduPilot's customary decimal rounding is named.
#[test]
fn a_ratio_no_decimal_states_is_judged_exactly() {
    let t = with_battery(100_000, 9_090, "ADC1");
    for scale in ["(109090.0f/9090.0f)", "(10909.0f/909.0f)"] {
        assert_eq!(
            judge(&t, &battery_header(scale), &[EMPTY]).unwrap(),
            Vec::<String>::new(),
            "{scale}"
        );
    }
    for scale in ["12.0011", "12.00110011", "12.0"] {
        assert_eq!(
            kinds(&judge(&t, &battery_header(scale), &[EMPTY]).unwrap()),
            ["WrongBattVoltScale"],
            "{scale}"
        );
    }
}

/// `AET-WO-4410` §3: the scale reader's edges. A quotient over zero is no
/// fraction, even `0/0`; a literal too long for exact arithmetic is named,
/// never overflowed; and a pin written in hex is read as C reads it.
#[test]
fn the_battery_monitor_s_literals_are_read_exactly_at_their_edges() {
    let t = with_battery(100_000, 10_000, "ADC1");
    for scale in [
        "(110000.0f/0.0f)",
        "(0.0f/0.0f)",
        // 39 decimals: the digits parse, and 10^39 is past u128.
        "0.000000000000000000000000000000000000011",
    ] {
        assert_eq!(
            kinds(&judge(&t, &battery_header(scale), &[EMPTY]).unwrap()),
            ["WrongBattVoltScale"],
            "{scale}"
        );
    }
    let hex = battery_header("11.0").replace("HAL_BATT_VOLT_PIN 10", "HAL_BATT_VOLT_PIN 0xA");
    assert_ne!(hex, battery_header("11.0"));
    assert_eq!(judge(&t, &hex, &[EMPTY]).unwrap(), Vec::<String>::new());
}

/// A battery monitor with no solved sense, and a sense on ADC3, are each named.
#[test]
fn a_battery_monitor_with_no_adc1_sense_is_named() {
    let text = battery_header("(110000.0f/10000.0f)");
    assert_eq!(
        kinds(&judge(&topology(), &text, &[EMPTY]).unwrap()),
        ["UnexpectedBattMonitor"]
    );
    let t = with_battery(100_000, 10_000, "ADC3");
    assert_eq!(
        kinds(&judge(&t, &text, &[EMPTY]).unwrap()),
        ["SenseNotOnAdc1"]
    );
}

/// `AET-WO-4404`: a topology with one port per `(role, inverted)`, in
/// order, the first the console; port `i` on U(S)ART `i + 1`, its TX and RX
/// on port C onwards; with the ELF's serial table to match. Since
/// `AET-WO-4405` the protocols, options and bauds come from `var_info`
/// ([`compiled`]); the `hwdef.h` holds nothing of the serial ports.
fn with_ports(ports: &[(&str, bool)]) -> (SolvedTopology, Vec<SerialEntry>) {
    let mut v: Value = serde_json::to_value(topology()).unwrap();
    let mut entries = Vec::new();
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
    }
    (serde_json::from_value(v).unwrap(), entries)
}

/// The `var_info` a right build of `ports` compiles: SERIAL0 MAVLink 2 at
/// `115200/1000` with no options row; each later port its role's protocol
/// (MAVLink 2, GPS 5, RCIN 23), 115200 (RC input: the tag's default `57`,
/// which ArduPilot's RC input replaces), and RXINV with TXINV exactly when an
/// inverted port is not RC input.
fn compiled(ports: &[(&str, bool)]) -> Vec<Port> {
    ports
        .iter()
        .enumerate()
        .map(|(n, (role, inverted))| {
            let rc = role.starts_with("rc_");
            match (n, *role) {
                (0, _) => (2, 115, None),
                (_, "gps") => (5, 115_200, Some(if *inverted { 3 } else { 0 })),
                _ if rc => (23, 57, Some(0)),
                _ => (2, 115_200, Some(if *inverted { 3 } else { 0 })),
            }
        })
        .collect()
}

/// fc-base's ports (`AET-WO-4404` FEEDBACK §2.3): the console, GPS, CRSF,
/// and GPS.
const FC_BASE: [(&str, bool); 4] = [
    ("console", false),
    ("gps", false),
    ("rc_crsf", false),
    ("gps", false),
];

/// fc-base's serial defines as ArduPilot writes them to `hwdef.h`
/// (measured, `AET-WO-4404` FEEDBACK §2.3): what today's ELF reading no
/// longer looks at.
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

/// The serial findings for `ports`, the ELF's `var_info` holding `params`
/// and the `hwdef.h` holding `defines`.
fn serial(ports: &[(&str, bool)], params: &[Port], defines: &str) -> Result<Vec<String>, Refusal> {
    let (t, entries) = with_ports(ports);
    judge_with(
        &t,
        &format!("{IDENTITY}{defines}"),
        &entries,
        var_info(params),
        &[INVENSENSEV3_PROBE],
    )
}

/// `params` with port `n` replaced.
fn with(params: &[Port], n: usize, port: Port) -> Vec<Port> {
    let mut p = params.to_vec();
    p[n] = port;
    p
}

/// fc-base passes; each `4404` §1.1 (c) mutation is named from `var_info`
/// as the spike decoded it (`spike/4405`), while `hwdef.h` holds fc-base's
/// right defines; and the same mutations in `hwdef.h`, with `var_info`
/// right, pass: the ELF's value is the one judged.
#[test]
fn each_serial_port_s_protocol_and_options_are_judged_from_the_elf() {
    let base = compiled(&FC_BASE);
    let judged = |params: &[Port]| serial(&FC_BASE, params, FC_BASE_DEFINES).unwrap();
    assert_eq!(judged(&base), Vec::<String>::new());
    let swap = with(&with(&base, 1, (23, 115_200, Some(0))), 2, (5, 57, Some(0)));
    assert_eq!(
        judged(&swap),
        [
            "WrongSerialProtocol SERIAL1 [SerialProtocol_GPS (5), for a gps port | SerialProtocol_RCIN (23)]",
            "WrongSerialProtocol SERIAL2 [SerialProtocol_RCIN (23), for a rc_crsf port | SerialProtocol_GPS (5)]"
        ]
    );
    for (options, shown) in [(8, "0x8"), (3, "0x3")] {
        assert_eq!(
            judged(&with(&base, 1, (5, 115_200, Some(options)))),
            [format!("WrongSerialOptions SERIAL1 [none | {shown}]")]
        );
    }
    // adv-proto-del1: the define deleted, SERIAL1 runs its default, MAVLink 2.
    assert_eq!(
        judged(&with(&base, 1, (2, 115_200, Some(0)))),
        [
            "WrongSerialProtocol SERIAL1 [SerialProtocol_GPS (5), for a gps port | SerialProtocol_MAVLink2 (2)]"
        ]
    );
    // fc-mav3 and adv-proto-del3: a MAVLink port at SERIAL3, then its define
    // deleted, so it runs SERIAL3's default, GPS.
    let mav3 = [FC_BASE[0], FC_BASE[1], FC_BASE[2], ("mavlink", false)];
    let mav3_rows = compiled(&mav3);
    assert_eq!(serial(&mav3, &mav3_rows, "").unwrap(), Vec::<String>::new());
    assert_eq!(
        serial(&mav3, &with(&mav3_rows, 3, (5, 115_200, Some(0))), "").unwrap(),
        [
            "WrongSerialProtocol SERIAL3 [SerialProtocol_MAVLink2 (2), for a mavlink port | SerialProtocol_GPS (5)]"
        ]
    );
    // The mutations in hwdef.h alone: var_info says what compiled, and passes.
    for defines in [
        FC_BASE_DEFINES
            .replace(
                "SERIAL1_PROTOCOL SerialProtocol_GPS",
                "SERIAL1_PROTOCOL SerialProtocol_RCIN",
            )
            .replace(
                "SERIAL2_PROTOCOL SerialProtocol_RCIN",
                "SERIAL2_PROTOCOL SerialProtocol_GPS",
            ),
        FC_BASE_DEFINES.replace("SERIAL1_OPTIONS 0", "SERIAL1_OPTIONS 8"),
        FC_BASE_DEFINES.replace("#define DEFAULT_SERIAL1_PROTOCOL SerialProtocol_GPS\n", ""),
    ] {
        assert_eq!(
            serial(&FC_BASE, &base, &defines).unwrap(),
            Vec::<String>::new(),
            "{defines}"
        );
    }
}

/// Each `hwdef.h` form `AET-WO-4404` refused, to keep its reading of the
/// serial defines honest, no longer reaches the judge: the ELF says what the
/// compiler did, and the verdict follows it (FEEDBACK §3). A protocol
/// spelled any way, a macro standing in for the enumerator, a parameter
/// guard other than 1, a malformed options or baud value: fc-base's
/// `var_info` passes beside each, and a mutated one fails beside each.
#[test]
fn the_header_s_serial_defines_no_longer_reach_the_judge() {
    let base = compiled(&FC_BASE);
    let wrong = with(&base, 1, (23, 115_200, Some(0)));
    for define in [
        "#define DEFAULT_SERIAL1_PROTOCOL_X 0\n",
        "#define SerialProtocol_GPS 23\n",
        "#define DEFAULT_SERIAL5_PROTOCOL SerialProtocol_Nope\n",
        "#define DEFAULT_SERIAL5_PROTOCOL AP_SerialManager::SerialProtocol_GPS\n",
        "#define DEFAULT_SERIAL5_PROTOCOL -1\n",
        "#define DEFAULT_SERIAL5_PROTOCOL (5)\n",
        "#define DEFAULT_SERIAL5_PROTOCOL 09\n",
        "#define DEFAULT_SERIAL5_PROTOCOL GPS_PROTO\n",
        "#define HAL_HAVE_SERIAL5_PARAMS 0\n",
        "#define HAL_HAVE_SERIAL5_PARAMS (1)\n",
        "#define DEFAULT_SERIAL6_OPTIONS 8u8\n",
        "#define DEFAULT_SERIAL1_BAUD fast\n",
    ] {
        let defines = format!("{FC_BASE_DEFINES}{define}");
        assert_eq!(
            serial(&FC_BASE, &base, &defines).unwrap(),
            Vec::<String>::new(),
            "{define}"
        );
        assert_eq!(
            kinds(&serial(&FC_BASE, &wrong, &defines).unwrap()),
            ["WrongSerialProtocol", "RcInputTwice"],
            "{define}"
        );
    }
}

/// A declared port `var_info` does not compile is named, whatever stops it
/// (`HAL_HAVE_SERIALn_PARAMS`, or n past 9, where the table ends): its
/// protocol, options and baud are then not judged.
#[test]
fn a_declared_port_var_info_does_not_compile_is_named() {
    let mut rows = var_info(&compiled(&FC_BASE));
    rows.retain(|r| !r.name.starts_with("2_"));
    let (t, entries) = with_ports(&FC_BASE);
    assert_eq!(
        judge_with(&t, IDENTITY, &entries, rows, &[INVENSENSEV3_PROBE]).unwrap(),
        [
            "MissingSerialParameters SERIAL2 [SERIAL2's parameters in var_info | none: var_info compiles no SERIAL2 row (AP_SerialManager.cpp:164-400)]"
        ]
    );
    // Eleven ports: the ELF's tables hold ten at most (`elf.rs`).
    let mut ports = vec![("console", false)];
    ports.extend([("mavlink", false); 10]);
    let (t, mut entries) = with_ports(&ports);
    entries.truncate(10);
    let found = judge_with(
        &t,
        IDENTITY,
        &entries,
        var_info(&compiled(&ports[..10])),
        &[INVENSENSEV3_PROBE],
    )
    .unwrap();
    assert_eq!(
        kinds(&found),
        ["MissingSerialPort", "MissingSerialParameters"],
        "{found:?}"
    );
    assert!(
        found[1].starts_with("MissingSerialParameters SERIAL10 "),
        "{found:?}"
    );
}

/// SERIAL7 and 8 compile a literal 0 for their options
/// (`AP_SerialManager.cpp:357`, `:376`), so a define there does nothing: the
/// eight-port board with `DEFAULT_SERIAL7_OPTIONS 8` (`eight-opt7`) passes,
/// because `var_info` says 0. A `7_OPTIONS` of 8 in `var_info` is named, as
/// `eight-opt6`'s 6_OPTIONS 8 is; SERIAL9's is judged too.
#[test]
fn serial7_options_are_judged_as_var_info_holds_them() {
    let mut ports = vec![("console", false)];
    ports.extend([("mavlink", false); 9]);
    let rows = compiled(&ports);
    let opt7 = "#define DEFAULT_SERIAL7_OPTIONS 8\n";
    assert_eq!(serial(&ports, &rows, opt7).unwrap(), Vec::<String>::new());
    for n in [6, 7, 9] {
        assert_eq!(
            serial(&ports, &with(&rows, n, (2, 115_200, Some(8))), "").unwrap(),
            [format!("WrongSerialOptions SERIAL{n} [none | 0x8]")]
        );
    }
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
    let at = |ports: &[(&str, bool)], protocol: i8, options: i32| {
        let rows = with(&compiled(ports), 1, (protocol, 115_200, Some(options)));
        serial(ports, &rows, "").unwrap()
    };
    for ports in [&crsf[..], &sbus[..]] {
        assert_eq!(at(ports, 23, 0), Vec::<String>::new());
        assert_eq!(at(ports, 23, 1), Vec::<String>::new());
        for bit in 1..=12 {
            assert_eq!(
                kinds(&at(ports, 23, 1 << bit)),
                ["WrongSerialOptions"],
                "{ports:?} bit {bit}"
            );
        }
    }
    assert_eq!(
        at(&sbus, 23, 2),
        [
            "WrongSerialOptions SERIAL1 [no bit but RXINV, which RC input sets (AP_RCProtocol.cpp:326-336) | 0x2]"
        ]
    );
    for bit in 0..=12 {
        assert_eq!(
            kinds(&at(&gps, 5, 1 << bit)),
            ["WrongSerialOptions"],
            "bit {bit}"
        );
    }
    assert_eq!(at(&inverted, 5, 3), Vec::<String>::new());
    for options in [0, 1, 2, 7] {
        assert_eq!(
            kinds(&at(&inverted, 5, options)),
            ["WrongSerialOptions"],
            "{options}"
        );
    }
}

/// ArduPilot attaches one RC input port and drops the next
/// (`AP_SerialManager.cpp:536-541`), even where the topology asks for both,
/// and counts every compiled port, declared or not.
#[test]
fn a_second_rc_input_port_is_named() {
    let two = [("console", false), ("rc_crsf", false), ("rc_sbus", true)];
    assert_eq!(
        serial(&two, &compiled(&two), "").unwrap(),
        [
            "RcInputTwice SERIAL2 [one RCIN port, SERIAL1 (AP_SerialManager.cpp:536-541) | SerialProtocol_RCIN (23)]"
        ]
    );
    // A GPS port compiled as RC input beside the CRSF port.
    let swap = with(&compiled(&FC_BASE), 1, (23, 115_200, Some(0)));
    assert_eq!(
        kinds(&serial(&FC_BASE, &swap, "").unwrap()),
        ["WrongSerialProtocol", "RcInputTwice"]
    );
    // An undeclared compiled port takes RC input too.
    let one = [("console", false), ("rc_crsf", false)];
    let mut rows = compiled(&one);
    rows.push((23, 57, Some(0)));
    assert_eq!(
        serial(&one, &rows, "").unwrap(),
        [
            "RcInputTwice SERIAL2 [one RCIN port, SERIAL1 (AP_SerialManager.cpp:536-541) | SerialProtocol_RCIN (23)]"
        ]
    );
}

/// `GPS_MAX_RECEIVERS` is 2 (`AP_GPS_config.h:16`): `AP_GPS::init` binds the
/// first two GPS ports (`AP_GPS.cpp:345-349`), so a third, and every one
/// after, is named. The eight-port board's five GPS ports (`4404` A-D2)
/// leave SERIAL4, 5 and 6 unbound. A GPS2 port is found as GPS
/// (`protocol_match`, `AP_SerialManager.cpp:786-790`), and an undeclared
/// compiled GPS port takes a receiver too.
#[test]
fn a_third_gps_port_is_named() {
    let two = [("console", false), ("gps", false), ("gps", false)];
    assert_eq!(
        serial(&two, &compiled(&two), "").unwrap(),
        Vec::<String>::new()
    );
    let eight = [
        ("console", false),
        ("gps", false),
        ("rc_crsf", false),
        ("gps", false),
        ("gps", false),
        ("gps", false),
        ("gps", false),
        ("mavlink", false),
    ];
    let found = serial(&eight, &compiled(&eight), "").unwrap();
    assert_eq!(
        found,
        [4, 5, 6].map(|n| format!(
            "GpsPortUnbound SERIAL{n} [at most 2 GPS ports, SERIAL1 and SERIAL3 (AP_GPS::var_info compiles 2 receivers; AP_GPS.cpp:345-349) | SerialProtocol_GPS (5)]"
        ))
    );
    let three = [
        ("console", false),
        ("gps", false),
        ("gps", false),
        ("mavlink", false),
    ];
    let gps2 = with(&compiled(&three), 3, (6, 115_200, Some(0)));
    assert_eq!(
        kinds(&serial(&three, &gps2, "").unwrap()),
        ["WrongSerialProtocol", "GpsPortUnbound"]
    );
    let mut undeclared = compiled(&two);
    undeclared.push((5, 230, Some(0)));
    assert_eq!(
        serial(&two, &undeclared, "").unwrap(),
        [
            "GpsPortUnbound SERIAL3 [at most 2 GPS ports, SERIAL1 and SERIAL2 (AP_GPS::var_info compiles 2 receivers; AP_GPS.cpp:345-349) | SerialProtocol_GPS (5)]"
        ]
    );
}

/// The GPS ports `ports` would leave unbound, the ELF compiling `params`
/// and an `AP_GPS::var_info` of `gps`.
fn serial_gps(ports: &[(&str, bool)], params: &[Port], gps: Option<Vec<VarRow>>) -> Vec<String> {
    let (t, entries) = with_ports(ports);
    judge_gps(
        &t,
        IDENTITY,
        &entries,
        var_info(params),
        gps,
        &[INVENSENSEV3_PROBE],
    )
    .unwrap()
}

/// The cap is read, not assumed (`AET-WO-4406` §1.3): it is the number of
/// receiver rows `AP_GPS::var_info` compiles. `advgps1`'s one receiver
/// (`4405` A4, `GPS_MAX_RECEIVERS 1`, measured in `spike/4406` §2.2) leaves
/// fc-base's second GPS port unbound, which two receivers bind; three would
/// bind the eight-port board's SERIAL4, which two do not.
#[test]
fn the_gps_cap_is_read_from_ap_gps_var_info() {
    let unbound = |cap: u8, bound: &str, n: u8| {
        let s = if cap == 1 { "" } else { "s" };
        format!(
            "GpsPortUnbound SERIAL{n} [at most {cap} GPS port{s}, {bound} (AP_GPS::var_info compiles {cap} receiver{s}; AP_GPS.cpp:345-349) | SerialProtocol_GPS (5)]"
        )
    };
    let fc_base = compiled(&FC_BASE);
    assert_eq!(
        serial_gps(&FC_BASE, &fc_base, Some(gps_var_info(2))),
        Vec::<String>::new()
    );
    assert_eq!(
        serial_gps(&FC_BASE, &fc_base, Some(gps_var_info(1))),
        [unbound(1, "SERIAL1", 3)]
    );
    assert_eq!(
        serial_gps(&FC_BASE, &fc_base, Some(gps_var_info(0))),
        [unbound(0, "none", 1), unbound(0, "none", 3)]
    );
    let eight = [
        ("console", false),
        ("gps", false),
        ("rc_crsf", false),
        ("gps", false),
        ("gps", false),
        ("gps", false),
        ("gps", false),
        ("mavlink", false),
    ];
    let mut three = gps_var_info(2);
    three.insert(three.len() - 1, common::gps_receiver("3_", 34));
    assert_eq!(
        serial_gps(&eight, &compiled(&eight), Some(three)),
        [5, 6].map(|n| unbound(3, "SERIAL1 and SERIAL3 and SERIAL4", n))
    );
}

/// With AP_GPS compiled out (no `AP_GPS::var_info`), each port the topology
/// declares a GPS has no consumer and is named (`GpsNotCompiled`); with no
/// GPS port declared nothing is, even where a compiled port's protocol is
/// GPS. At this tag such a Copter does not build (`spike/4406`
/// `out/advnogps-build-errors.txt`), so this is the reader's rule, not a
/// measured board.
#[test]
fn a_declared_gps_port_with_no_ap_gps_is_named() {
    assert_eq!(
        serial_gps(&FC_BASE, &compiled(&FC_BASE), None),
        [1, 3].map(|n| format!(
            "GpsNotCompiled SERIAL{n} [AP_GPS, to bind this GPS port | none: the ELF has no AP_GPS::var_info]"
        ))
    );
    let mavlink = [("console", false), ("mavlink", false)];
    let mut stray = compiled(&mavlink);
    stray.push((5, 230, Some(0)));
    assert_eq!(serial_gps(&mavlink, &stray, None), Vec::<String>::new());
}

/// `AP_GPS::var_info` is read as `serial_params` reads `var_info`, and a
/// group row that is not the next receiver's, or carries other flags, is
/// refused by name: the count would not be the receivers.
#[test]
fn every_malformed_gps_var_info_is_refused_by_name() {
    let kind = |rows: Vec<VarRow>| {
        elf::gps_receivers(
            &Elf {
                gps: Some(rows),
                ..Elf::default()
            }
            .bytes(),
        )
        .map(|n| format!("{n:?}"))
        .unwrap_or_else(|e| e.kind().to_owned())
    };
    let edit = |f: &dyn Fn(&mut Vec<VarRow>)| {
        let mut rows = gps_var_info(2);
        f(&mut rows);
        rows
    };
    assert_eq!(kind(gps_var_info(2)), "Some(2)");
    assert_eq!(kind(gps_var_info(1)), "Some(1)");
    assert_eq!(elf::gps_receivers(&Elf::default().bytes()), Ok(None));
    let end = |r: &mut Vec<VarRow>| r.len() - 1;
    for (why, rows) in [
        (
            "no 1_",
            edit(&|r| {
                r.remove(12);
            }),
        ),
        ("2_ named 3_", edit(&|r| r[13].name = "3_".into())),
        ("1_ twice", edit(&|r| r[13].name = "1_".into())),
        ("a group not a receiver", edit(&|r| r[0].kind = 6)),
        ("flags 0", edit(&|r| r[12].flags = 0)),
        ("AP_PARAM_FLAG_POINTER", edit(&|r| r[13].flags = 2)),
    ] {
        assert_eq!(kind(rows), "MalformedGpsVarInfo", "{why}");
    }
    assert_eq!(
        kind(edit(&|r| {
            let at = end(r);
            r.remove(at);
        })),
        "VarInfoUnterminated"
    );
    assert_eq!(
        kind(edit(&|r| {
            let at = end(r);
            r[at].idx = 0;
        })),
        "MalformedVarInfo"
    );
    // A scalar row named like a receiver is not one.
    assert_eq!(kind(edit(&|r| r[0].name = "3_".into())), "Some(2)");
}

/// SERIAL0's protocol is read from `var_info`, where the tag compiles a
/// literal MAVLink 2 (`AP_SerialManager.cpp:180`); a define for it does
/// nothing. SERIAL0 has no options row, so a console declared inverted runs
/// not inverted. With no console, the first port stands at SERIAL0 and runs
/// MAVLink 2 whatever its role (`AET-WO-4404` D4).
#[test]
fn the_console_s_protocol_is_read_and_its_inversion_is_named() {
    let console = [("console", false)];
    assert_eq!(
        serial(&[("console", true)], &compiled(&console), "").unwrap(),
        [
            "WrongSerialOptions SERIAL0 [RXINV and TXINV (declared inverted) | none: SERIAL0 has no options parameter (AP_SerialManager.cpp:165-181)]"
        ]
    );
    let ignored =
        "#define DEFAULT_SERIAL0_PROTOCOL SerialProtocol_GPS\n#define DEFAULT_SERIAL0_OPTIONS 3\n";
    assert_eq!(
        serial(&console, &compiled(&console), ignored).unwrap(),
        Vec::<String>::new()
    );
    assert_eq!(
        serial(&console, &[(5, 115, None)], "").unwrap(),
        [
            "WrongSerialProtocol SERIAL0 [SerialProtocol_MAVLink2 (2), for a console port | SerialProtocol_GPS (5)]"
        ]
    );
    let no_console = [("gps", false), ("rc_crsf", false)];
    assert_eq!(
        serial(&no_console, &compiled(&no_console), "").unwrap(),
        [
            "WrongSerialProtocol SERIAL0 [SerialProtocol_GPS (5), for a gps port | SerialProtocol_MAVLink2 (2)]"
        ]
    );
    let mavlink = [("mavlink", false)];
    assert_eq!(
        serial(&mavlink, &compiled(&mavlink), "").unwrap(),
        Vec::<String>::new()
    );
    let sbus = [("rc_sbus", true)];
    assert_eq!(
        kinds(&serial(&sbus, &compiled(&sbus), "").unwrap()),
        ["WrongSerialProtocol", "WrongSerialOptions"]
    );
}

/// Each port's baud is its `n_BAUD` mapped (`map_baudrate`,
/// `AP_SerialManager.cpp:737-770`), never the stored value: SERIAL0's `115`
/// is 115200, and a port with no baud define runs its `#ifndef` default,
/// `57` (57600). No `DEFAULT_SERIALn_BAUD` in `hwdef.h` is read. RC
/// input's baud is ArduPilot's own (`AET-WO-4400b` F3).
#[test]
fn each_baud_is_the_parameter_mapped() {
    let ports = [("console", false), ("mavlink", false), ("rc_crsf", false)];
    let rows = compiled(&ports);
    assert_eq!(serial(&ports, &rows, "").unwrap(), Vec::<String>::new());
    for (stored, runs) in [
        (57, 57_600),
        (0, 57_600),
        (-1, 57_600),
        (9600, 9600),
        (2, 2400),
    ] {
        assert_eq!(
            serial(&ports, &with(&rows, 1, (2, stored, Some(0))), "").unwrap(),
            [format!(
                "WrongSerialBaud SERIAL1 [115200 baud | {runs} baud]"
            )],
            "{stored}"
        );
    }
    for stored in [115, 115_200] {
        assert_eq!(
            serial(
                &ports,
                &with(&rows, 1, (2, stored, Some(0))),
                "#define DEFAULT_SERIAL1_BAUD 9600\n"
            )
            .unwrap(),
            Vec::<String>::new(),
            "{stored}"
        );
    }
    assert_eq!(
        serial(&ports, &with(&rows, 2, (23, 9, Some(0))), "").unwrap(),
        Vec::<String>::new()
    );
    assert_eq!(
        serial(&ports, &with(&rows, 0, (2, 115_200, None)), "").unwrap(),
        Vec::<String>::new(),
        "a stored 115200 also maps to 115200"
    );
    assert_eq!(
        serial(&ports, &with(&rows, 0, (2, 57, None)), "").unwrap(),
        ["WrongSerialBaud SERIAL0 [115200 baud | 57600 baud]"]
    );
}

/// `serial_params` reads fc-base's table as the spike decoded it.
#[test]
fn var_info_reads_as_the_spike_decoded_it() {
    let rows = var_info(&[
        (2, 57, None),
        (5, 115_200, Some(0)),
        (23, 57, Some(0)),
        (5, 115_200, Some(0)),
    ]);
    let params = elf::serial_params(
        &Elf {
            var_info: Some(rows),
            ..Elf::default()
        }
        .bytes(),
    )
    .unwrap();
    let p = |port, protocol, baud, options| SerialParam {
        port,
        protocol,
        baud,
        options,
    };
    assert_eq!(
        params,
        [
            p(0, 2, 57, None),
            p(1, 5, 115_200, Some(0)),
            p(2, 23, 57, Some(0)),
            p(3, 5, 115_200, Some(0))
        ]
    );
    assert_eq!(elf::map_baudrate(params[0].baud), 57_600);
}

/// Every malformed `var_info` is refused by name (`INV-06`), never read
/// loosely: no symbol, a partial entry, no `AP_GROUPEND`, anything after it
/// or an `AP_GROUPEND` that is not exactly one, a name outside flash or too
/// long, a row named twice or not known, flags (a default that is not a
/// float), a type other than its member's (a nested group among them), a
/// default its type does not hold exactly, and a port missing a row.
#[test]
fn every_malformed_var_info_is_refused_by_name() {
    let good = var_info(&compiled(&FC_BASE));
    let kind = |rows: Vec<VarRow>| {
        elf::serial_params(
            &Elf {
                var_info: Some(rows),
                ..Elf::default()
            }
            .bytes(),
        )
        .map(|_| "read")
        .unwrap_or_else(|e| e.kind())
    };
    let message = |rows: Vec<VarRow>| {
        elf::serial_params(
            &Elf {
                var_info: Some(rows),
                ..Elf::default()
            }
            .bytes(),
        )
        .unwrap_err()
        .to_string()
    };
    let edit = |f: &dyn Fn(&mut Vec<VarRow>)| {
        let mut rows = good.clone();
        f(&mut rows);
        rows
    };
    let named =
        |rows: &mut Vec<VarRow>, name: &str| rows.iter().position(|r| r.name == name).unwrap();
    assert_eq!(kind(good.clone()), "read");
    assert_eq!(
        elf::serial_params(&Elf::default().bytes())
            .unwrap_err()
            .kind(),
        "SymbolMissing"
    );
    let rows = good.len() as u32;
    let partial = Elf {
        var_info: Some(good.clone()),
        var_info_size: Some(16 * rows - 8),
        ..Elf::default()
    };
    assert_eq!(
        elf::serial_params(&partial.bytes()).unwrap_err().kind(),
        "SymbolSize"
    );
    assert_eq!(
        kind(edit(&|r| {
            r.pop();
        })),
        "VarInfoUnterminated"
    );
    // ArduPilot stops at AP_GROUPEND: a row after it is not part of the
    // table, and this reader refuses the table rather than read on.
    assert_eq!(
        kind(edit(&|r| r.push(var_row("4_PROTOCOL", 1, 5.0)))),
        "MalformedVarInfo"
    );
    let early = edit(&|r| {
        let at = named(r, "1_OPTIONS");
        r.insert(at, group_end());
    });
    assert_eq!(kind(early), "MalformedVarInfo");
    for bad_end in [
        VarRow {
            idx: 0,
            ..group_end()
        },
        VarRow {
            name: "x".into(),
            ..group_end()
        },
        VarRow {
            offset: 4,
            ..group_end()
        },
        VarRow {
            value: 1,
            ..group_end()
        },
        VarRow {
            flags: 1,
            ..group_end()
        },
    ] {
        assert_eq!(
            kind(edit(&|r| *r.last_mut().unwrap() = bad_end.clone())),
            "MalformedVarInfo",
            "{bad_end:?}"
        );
    }
    // A name read where it is flashed: nowhere in the image, in the symbol
    // table's bytes (not loaded), or in RAM that no PT_LOAD flashes.
    for at in [0x2000_0000, 0] {
        assert_eq!(
            kind(edit(&|r| r[0].name_at = Some(at))),
            "SymbolNotLoaded",
            "{at:#x}"
        );
    }
    let in_ram = Elf {
        var_info: Some(edit(&|r| r[0].name_at = Some(common::RAM))),
        ram: Some(b"0_BAUD\0".to_vec()),
        ..Elf::default()
    };
    assert_eq!(
        elf::serial_params(&in_ram.bytes()).unwrap_err().kind(),
        "SymbolNotLoaded"
    );
    // A name whose NUL lies past the flashed image: the last bytes flashed
    // are `_PASS`, with no NUL (the GPIO image, then the rows, their names,
    // then the tail).
    let names: usize = good.iter().map(|r| r.name.len() + 1).sum();
    let tail_at = common::ADDR + 308 + 16 * good.len() as u32 + names as u32;
    let off_the_end = Elf {
        var_info: Some(edit(&|r| r[0].name_at = Some(tail_at))),
        tail: b"_PASS".to_vec(),
        ..Elf::default()
    };
    assert_eq!(
        elf::serial_params(&off_the_end.bytes()).unwrap_err().kind(),
        "SymbolNotLoaded"
    );
    assert_eq!(
        kind(edit(&|r| r[0].name = "0_BAUD_AND_SEVENTEEN".into())),
        "VarInfoNameTooLong"
    );
    assert_eq!(
        kind(edit(&|r| {
            let at = named(r, "1_PROTOCOL");
            r.insert(at, var_row("1_PROTOCOL", 1, 5.0));
        })),
        "VarInfoRowTwice"
    );
    for unknown in [
        "1_PROTOKOL",
        "0_OPTIONS",
        "10_PROTOCOL",
        "1_protocol",
        "_PASS3",
        "",
    ] {
        let rows = edit(&|r| {
            let at = named(r, "_PASS1");
            r.insert(at, var_row(unknown, 3, 0.0));
        });
        assert_eq!(kind(rows), "UnknownVarInfoRow", "{unknown}");
    }
    // AP_PARAM_FLAG_DEFAULT_POINTER: the union is an offset, not a float.
    assert_eq!(
        kind(edit(&|r| {
            let at = named(r, "1_BAUD");
            r[at].flags = 1 << 7;
        })),
        "MalformedVarInfo"
    );
    for wrong_type in [2, 4, 6] {
        let rows = edit(&|r| {
            let at = named(r, "1_BAUD");
            r[at].kind = wrong_type;
        });
        assert_eq!(kind(rows), "MalformedVarInfo", "type {wrong_type}");
    }
    assert!(
        message(edit(&|r| {
            let at = named(r, "1_PROTOCOL");
            r[at].kind = 6;
        }))
        .contains("`1_PROTOCOL`) has type 6, not 1")
    );
    for (name, value) in [
        ("1_PROTOCOL", 5.5),
        ("1_PROTOCOL", 128.0),
        ("1_PROTOCOL", -129.0),
        ("1_PROTOCOL", f32::NAN),
        ("1_BAUD", 3.0e9),
        ("1_BAUD", -3.0e9),
        ("1_OPTIONS", f32::INFINITY),
        ("0_BAUD", -0.5),
    ] {
        let rows = edit(&|r| {
            let at = named(r, name);
            r[at].value = value.to_bits();
        });
        assert_eq!(kind(rows), "MalformedVarInfo", "{name} {value}");
    }
    // ArduPilot applies a default by offset (`AP_Param.cpp:1490-1497`): a
    // row whose name and offset disagree is refused (`AET-WO-4405` A5, its
    // offset swap of 1_PROTOCOL and 2_PROTOCOL; `AET-WO-4406` §1.4), and the
    // pass-through rows' offsets are not judged.
    let swapped = edit(&|r| {
        let (one, two) = (named(r, "1_PROTOCOL"), named(r, "2_PROTOCOL"));
        let o = r[one].offset;
        r[one].offset = r[two].offset;
        r[two].offset = o;
    });
    assert_eq!(kind(swapped.clone()), "VarInfoRowOffset");
    assert!(message(swapped).contains("(`1_PROTOCOL`) sets the member at offset 56, not 32"),);
    for (name, delta) in [("0_BAUD", 8), ("0_PROTOCOL", 1), ("3_OPTIONS", 24)] {
        let rows = edit(&|r| {
            let at = named(r, name);
            r[at].offset += delta;
        });
        assert_eq!(kind(rows), "VarInfoRowOffset", "{name} +{delta}");
    }
    assert_eq!(
        kind(edit(&|r| {
            let at = named(r, "_PASS2");
            r[at].offset = 1234;
        })),
        "read"
    );
    for dropped in ["1_BAUD", "2_OPTIONS", "0_PROTOCOL", "3_PROTOCOL"] {
        let rows = edit(&|r| {
            let at = named(r, dropped);
            r.remove(at);
        });
        assert_eq!(kind(rows), "VarInfoPartialPort", "{dropped}");
    }
}
