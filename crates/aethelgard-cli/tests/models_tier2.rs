//! `AET-WO-4220` S4, tier 2: the models of our own, driven from Renode's
//! monitor with no firmware (spike P4.1-P4.3, measured), in the pinned image.
//! Each check reads the model's own one-line witness of a transaction:
//! - every register a script or ArduPilot touches answers its datasheet's
//!   reset, each cited to its page;
//! - ArduPilot's probe and init (`Copter-4.7.2`, `fb773567`), replayed, gets
//!   every value it gates on;
//! - a stimulus becomes register counts rounded half away from zero;
//! - data-ready is the model's own act, from its own sampling (D8);
//! - a model that does not compile prints the line the CLI refuses on.

#![cfg(feature = "with-tools")]

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

mod common;

use common::verify::{generated, root};

/// The Sentinel-02 fixture's emitted platform and each model's Renode path.
struct Bench {
    dir: PathBuf,
    platform: PathBuf,
    models: Vec<PathBuf>,
    imu: String,
    baro: String,
}

fn bench(label: &str) -> Bench {
    let (solved, _) = generated(
        &root().join("crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json"),
        "own-board",
    );
    let dir = std::env::temp_dir().join(format!("aet-4220-models-{}-{label}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let platform = dir.join("platform.repl");
    fs::write(&platform, aethelgard_twin::platform::emit(&solved).unwrap()).unwrap();
    let mut models = Vec::new();
    for (m, file, _) in aethelgard_twin::model::OWN {
        let path = dir.join(file);
        fs::write(&path, m.source().unwrap()).unwrap();
        models.push(path);
    }
    let imu =
        aethelgard_twin::platform::spi_device_path(&solved.spi_devices["U5"].peripheral, "U5")
            .unwrap();
    let baro =
        aethelgard_twin::platform::device_path(&solved.i2c_devices["U7"].peripheral, "U7").unwrap();
    Bench {
        dir,
        platform,
        models,
        imu,
        baro,
    }
}

/// Renode 1.17.0's configuration as it writes it for itself, with
/// `use-synchronous-logging = true` (`twin_tier2.rs` pins the default).
const CONFIG: &str = "[general]
use-synchronous-logging = true
terminal = Termsharp
compiler-cache-enabled = False
serialization-mode = Generated
always-log-machine-name = False
collapse-repeated-log-entries = True
log-timestamps-type = Host
log-history-limit = 1000
history-path = $HOME/.config/renode/history
store-table-bits = 41
[monitor]
consume-exceptions-from-command = True
break-script-on-exception = True
number-format = Hexadecimal
[plugins]
enabled-plugins = 
[translation]
min-tb-size = 33554432
max-tb-size = 536870912
";

/// One Renode run: the models included, the platform loaded, both devices
/// at `Noisy`, then `commands`. Returns every log line of the two devices,
/// without its timestamp, in order: the witness lines the judge reads.
fn run(b: &Bench, models: &[PathBuf], commands: &[String]) -> (String, Vec<String>) {
    let mut c: Vec<String> = models
        .iter()
        .map(|m| format!("include @{}", m.display()))
        .collect();
    c.extend([
        "mach create".to_owned(),
        format!("machine LoadPlatformDescription @{}", b.platform.display()),
        format!("logLevel -1 sysbus.{}", b.imu),
        format!("logLevel -1 sysbus.{}", b.baro),
    ]);
    c.extend(commands.iter().cloned());
    c.push("quit".to_owned());
    // Synchronous logging, as the CLI runs Renode (`renode::config`): with
    // Renode's default, a log line written just before `quit` can be lost
    // (measured on CI: the last transaction's line was missing). Every other
    // key is Renode's own default, so it only reads the file.
    let home = b.dir.join("home");
    fs::create_dir_all(&home).unwrap();
    let config = b.dir.join("renode.config");
    fs::write(&config, CONFIG.replace("$HOME", home.to_str().unwrap())).unwrap();
    let out = Command::new("renode")
        .arg("--config")
        .arg(&config)
        .args(["--disable-gui", "--console", "--plain", "-e", &c.join("; ")])
        .env("DOTNET_EnableDiagnostics", "0")
        .env("HOME", &home)
        .current_dir(&b.dir)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).replace('\r', "");
    let lines = text
        .lines()
        .filter_map(|l| {
            let body = l.split_once("] ").map_or(l, |(_, r)| r);
            let level = l.find("[NOISY]").or_else(|| l.find("[WARNING]"))?;
            let tag = &l[level..];
            [&b.imu, &b.baro]
                .iter()
                .any(|d| body.contains(&format!("{d}: ")))
                .then(|| tag.to_owned())
        })
        .collect();
    (text, lines)
}

/// The monitor commands of one SPI transaction with the IMU: `out` sent,
/// then `reads` bytes clocked in, then the chip select released.
fn spi(b: &Bench, out: &[u8], reads: usize) -> Vec<String> {
    let mut c: Vec<String> = out
        .iter()
        .map(|x| format!("sysbus.{} Transmit {x:#04x}", b.imu))
        .collect();
    c.extend((0..reads).map(|_| format!("sysbus.{} Transmit 0x00", b.imu)));
    c.push(format!("sysbus.{} FinishTransmission", b.imu));
    c
}

/// One I2C transaction with the barometer: `write` sent, then `reads` read.
fn i2c(b: &Bench, write: &[u8], reads: usize) -> Vec<String> {
    let bytes: Vec<String> = write.iter().map(|x| format!("{x:#04x}")).collect();
    let mut c = vec![format!("sysbus.{} Write [{}]", b.baro, bytes.join(", "))];
    if reads > 0 {
        c.push(format!("sysbus.{} Read {reads}", b.baro));
    }
    c.push(format!("sysbus.{} FinishTransmission", b.baro));
    c
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("0x{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Every register the scripts or ArduPilot touch, read once from reset,
/// against its datasheet value: (register, bytes, page).
#[test]
fn our_models_answer_their_datasheets_reset_values() {
    let b = bench("resets");
    // DS-000347 1.9 (TDK), bank 0, each read alone.
    let imu: [(u8, &[u8], &str); 14] = [
        (0x75, &[0x47], "p. 88, 14.58 WHO_AM_I"),
        (0x11, &[0x00], "p. 64, 14.1 DEVICE_CONFIG"),
        (0x13, &[0x05], "p. 64, 14.2 DRIVE_CONFIG"),
        (0x14, &[0x00], "p. 65, 14.3 INT_CONFIG"),
        (0x2D, &[0x10], "p. 69, 14.21 INT_STATUS (RESET_DONE)"),
        (0x2D, &[0x00], "p. 69, 14.21 INT_STATUS, clear on read"),
        (0x4C, &[0x30], "p. 74, 14.34 INTF_CONFIG0"),
        (0x4D, &[0x91], "p. 75, 14.35 INTF_CONFIG1"),
        (0x4E, &[0x00], "p. 76, 14.36 PWR_MGMT0"),
        (0x4F, &[0x06], "p. 77, 14.37 GYRO_CONFIG0"),
        (0x50, &[0x06], "p. 78, 14.38 ACCEL_CONFIG0"),
        (0x51, &[0x16], "p. 79, 14.39 GYRO_CONFIG1"),
        (0x65, &[0x10], "p. 85, 14.51 INT_SOURCE0"),
        (
            0x1F,
            &[0x80, 0x00],
            "p. 66, 14.7 ACCEL_DATA_X1/X0, big-endian at reset",
        ),
    ];
    // BST-BMP581-DS004-13 (Bosch), read (rendered).
    let baro: [(u8, &[u8], &str); 11] = [
        (0x01, &[0x50], "p. 51, 7.1 CHIP_ID"),
        (0x02, &[0x32], "p. 51, 7.2 REV_ID"),
        (0x13, &[0x30], "p. 52, 7.4 DRIVE_CONFIG"),
        (0x14, &[0x35], "p. 52, 7.5 INT_CONFIG"),
        (0x28, &[0x02], "p. 58, 7.22 STATUS (status_nvm_rdy)"),
        (
            0x27,
            &[0x10],
            "p. 18, 4.3.9 INT_STATUS after power-up (por)",
        ),
        (0x27, &[0x00], "p. 58, 7.21 INT_STATUS, clear on read"),
        (0x36, &[0x00], "p. 64, 7.33 OSR_CONFIG"),
        (0x37, &[0x70], "p. 64, 7.34 ODR_CONFIG"),
        (0x30, &[0x03], "p. 61, 7.27 DSP_CONFIG"),
        (0x1D, &[0x7F; 6], "pp. 56-57, 7.11-7.16 TEMP and PRESS data"),
    ];
    // A finish with no transaction, as the multiplexer and the controller
    // make on other devices' edges, logs nothing (counted below).
    let mut c = vec![
        format!("sysbus.{} FinishTransmission", b.imu),
        format!("sysbus.{} FinishTransmission", b.baro),
    ];
    for (r, v, _) in imu {
        c.extend(spi(&b, &[r | 0x80], v.len()));
    }
    for (r, v, _) in baro {
        c.extend(i2c(&b, &[r], v.len()));
    }
    // `INT_STATUS` is read-only: a store warns and changes nothing.
    c.extend(i2c(&b, &[0x27, 0x10], 0));
    c.extend(i2c(&b, &[0x27], 1));
    let (text, lines) = run(&b, &b.models, &c);
    let mut want = Vec::new();
    for (r, v, page) in imu {
        want.push((
            format!("[NOISY] {}: bank 0 read 0x{r:02X} -> {}", b.imu, hex(v)),
            page,
        ));
    }
    for (r, v, page) in baro {
        want.push((
            format!("[NOISY] {}: read 0x{r:02X} -> {}", b.baro, hex(v)),
            page,
        ));
    }
    for (line, page) in [
        (
            format!(
                "[WARNING] {}: write to read-only register 0x27 ignored",
                b.baro
            ),
            "p. 58, 7.21 INT_STATUS is read-only",
        ),
        (
            format!("[NOISY] {}: write 0x27 <- 0x10", b.baro),
            "the store",
        ),
        (
            format!("[NOISY] {}: read 0x27 -> 0x00", b.baro),
            "unchanged",
        ),
    ] {
        want.push((line, page));
    }
    assert_eq!(lines.len(), want.len(), "{text}");
    for (line, (expected, page)) in lines.iter().zip(&want) {
        assert_eq!(line, expected, "{page}");
    }
}

/// The BMP581 out of reset is in DEEP STANDBY (DS p.16), so its first
/// measurement starts `t_startup_deep` (4 ms max, p.11) after the write that
/// leaves it, whatever that write sets `deep_dis` to, and ends after the
/// conversions at their maximum, 1.05 ms each at OSR 1x (Table 4, p.12):
/// 6.1 ms. A new read starts at the latest write's register (p.40), not
/// where the write's auto-increment left off (adversarial F3, F4).
#[test]
fn the_bmp581_measures_after_its_deep_start_up_and_reads_from_its_last_write() {
    let b = bench("bmp-start");
    let mut c = i2c(&b, &[0x36, 0x40, 0x81], 0);
    c.push("emulation RunFor \"0.006090000\"".to_owned());
    c.extend(i2c(&b, &[0x1D], 6));
    c.push("emulation RunFor \"0.000020000\"".to_owned());
    c.extend(i2c(&b, &[0x1D], 6));
    // The same configuration again changes nothing; a read alone after it
    // starts at its register, 0x36, not at 0x38 where the write left off.
    c.extend(i2c(&b, &[0x36, 0x40, 0x81], 0));
    c.push(format!("sysbus.{} Read 1", b.baro));
    c.push(format!("sysbus.{} FinishTransmission", b.baro));
    let (text, lines) = run(&b, &b.models, &c);
    let baro = |l: &str| format!("[NOISY] {}: {l}", b.baro);
    assert_eq!(
        lines,
        [
            baro("write 0x36 <- 0x40 0x81"),
            baro("read 0x1D -> 0x7F 0x7F 0x7F 0x7F 0x7F 0x7F"),
            baro("read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"),
            baro("write 0x36 <- 0x40 0x81"),
            baro("read 0x36 -> 0x40"),
        ],
        "{text}"
    );
}

/// What the datasheets forbid the firmware, the models flag: a register
/// write within 200 us of an ICM-42688-P sensor leaving OFF (DS-000347
/// p.76), an access within 1 ms of its soft reset (p.64), and an access
/// within 2 ms of the BMP581's (`t_soft_res`, p.11). Each is a warning, which
/// fails the firmware's step (adversarial F5).
#[test]
fn an_access_the_datasheets_forbid_warns() {
    let b = bench("timing");
    let mut c = Vec::new();
    c.extend(spi(&b, &[0x4E, 0x03], 0));
    c.extend(spi(&b, &[0x50, 0x66], 0));
    c.push("emulation RunFor \"0.000300000\"".to_owned());
    c.extend(spi(&b, &[0x50, 0x66], 0));
    c.extend(spi(&b, &[0x11, 0x01], 0));
    c.extend(spi(&b, &[0x75 | 0x80], 1));
    c.extend(i2c(&b, &[0x7E, 0xB6], 0));
    c.extend(i2c(&b, &[0x01], 1));
    c.push("emulation RunFor \"0.002100000\"".to_owned());
    c.extend(spi(&b, &[0x75 | 0x80], 1));
    c.extend(i2c(&b, &[0x01], 1));
    let (text, lines) = run(&b, &b.models, &c);
    let imu = |l: &str| format!("[NOISY] {}: {l}", b.imu);
    let baro = |l: &str| format!("[NOISY] {}: {l}", b.baro);
    let warn = |d: &str, l: &str| format!("[WARNING] {d}: {l}");
    assert_eq!(
        lines,
        [
            imu("bank 0 write 0x4E <- 0x03"),
            warn(
                &b.imu,
                "bank 0 write to 0x50 within 200 us of a sensor leaving OFF"
            ),
            imu("bank 0 write 0x50 <- 0x66"),
            imu("bank 0 write 0x50 <- 0x66"),
            imu("bank 0 write 0x11 <- 0x01"),
            warn(&b.imu, "bank 0 access to 0x75 within 1 ms of a soft reset"),
            imu("bank 0 read 0x75 -> 0x47"),
            baro("write 0x7E <- 0xB6"),
            warn(&b.baro, "access to 0x01 within 2 ms of a soft reset"),
            baro("read 0x01 -> 0x50"),
            imu("bank 0 read 0x75 -> 0x47"),
            baro("read 0x01 -> 0x50"),
        ],
        "{text}"
    );
}

/// ArduPilot's probe and `start()` for the ICM-42688-P (`Invensensev3`,
/// `s2-ardupilot.md` 1.2-1.3, at the H7 default of 2 kHz that `4400e`
/// measured: `0x4F` and `0x50` <- `0x05`, `IV3` L779-788, L844-901) and its
/// probe and init for the BMP581 (`AP_Baro_BMP581` L81-133, 6.1), replayed:
/// every read it gates on answers what it needs, every checked register
/// reads back, and no register it touches is unmodelled. Then its FIFO poll
/// (`IV3` L583-611, `AET-WO-4220c`): the count, and one packet.
#[test]
fn ardupilot_s_probe_and_init_pass_against_our_models() {
    let b = bench("ardupilot");
    let mut c = Vec::new();
    // ICM-42688-P: probe (IV3:1005), then start() (IV3:247-901).
    c.extend(spi(&b, &[0xF5], 1));
    c.extend(ardupilot_imu_start(&b, 0x05));
    // The checked register, read back (DEV:112-154).
    c.extend(spi(&b, &[0x4D | 0x80], 1));
    // BMP581 on I2C: probe gates, then the checked writes, read back.
    c.extend(i2c(&b, &[0x01], 1));
    c.extend(i2c(&b, &[0x28], 1));
    c.extend(i2c(&b, &[0x27], 1));
    c.extend(i2c(&b, &[0x37, 0x00], 0));
    c.extend(i2c(&b, &[0x36, 0x72], 0));
    c.extend(i2c(&b, &[0x37, 0x3D], 0));
    c.extend(i2c(&b, &[0x36], 1));
    c.extend(i2c(&b, &[0x37], 1));
    // The FIFO poll, 0.25 ms after the accelerometer's first sample at 10 ms.
    c.push("emulation RunFor \"0.010250000\"".to_owned());
    c.extend(spi(&b, &[0x2E | 0x80], 2));
    c.extend(spi(&b, &[0x30 | 0x80], 16));
    let (text, lines) = run(&b, &b.models, &c);
    assert!(
        lines.iter().all(|l| l.starts_with("[NOISY]")),
        "no warning from either model: {text}"
    );
    let packets = reads(&lines, &b.imu, 0x30);
    assert_eq!(packets.len(), 1, "{text}");
    assert_eq!(packets[0][0], 0x68, "packet 3's header, IV3 L458: {text}");
    let imu = |l: &str| format!("[NOISY] {}: {l}", b.imu);
    let baro = |l: &str| format!("[NOISY] {}: {l}", b.baro);
    for want in [
        imu("bank 0 read 0x75 -> 0x47"),
        imu("bank 1 write 0x0E <- 0x80"),
        imu("bank 0 read 0x2E -> 0x01 0x00"),
        imu("bank 2 write 0x05 <- 0xA0"),
        imu("bank 0 read 0x4D -> 0x91"),
        imu("bank 0 read 0x4D -> 0x51"),
        baro("read 0x01 -> 0x50"),
        baro("read 0x28 -> 0x02"),
        baro("read 0x27 -> 0x10"),
        baro("read 0x36 -> 0x72"),
        baro("read 0x37 -> 0x3D"),
    ] {
        assert!(lines.contains(&want), "`{want}` not in {lines:#?}");
    }
}

/// The ruling's amendment (`4210` D-A8): our models round as their
/// datasheets convert, half away from zero, at a fraction of one half or
/// more: the ICM-42688-P's 29 degrees C is 529.92 counts, read 530
/// (`0x0212`); the BMP581's 100,000.0078125 Pa is 6,400,000.5 counts, read
/// 6,400,001 (`0x61A801`), where truncation reads 6,400,000.
#[test]
fn our_models_round_half_away_from_zero() {
    let b = bench("rounding");
    let mut c = vec![
        format!("sysbus.{} Temperature 29", b.imu),
        format!("sysbus.{} Pressure 100000.0078125", b.baro),
    ];
    c.extend(spi(&b, &[0x4C, 0x30], 0));
    c.extend(spi(&b, &[0x4E, 0x03], 0));
    c.extend(i2c(&b, &[0x36, 0x40, 0x81], 0));
    c.push("emulation RunFor \"0.030000000\"".to_owned());
    c.extend(spi(&b, &[0x9D], 2));
    c.extend(i2c(&b, &[0x20], 3));
    let (text, lines) = run(&b, &b.models, &c);
    let imu = format!("[NOISY] {}: bank 0 read 0x1D -> 0x02 0x12", b.imu);
    let baro = format!("[NOISY] {}: read 0x20 -> 0x01 0xA8 0x61", b.baro);
    assert!(lines.contains(&imu), "{text}");
    assert!(lines.contains(&baro), "{text}");
}

/// D8: the models raise data-ready from their own sampling, never from a
/// store. The ICM-42688-P routed to `INT1` (`INT_SOURCE0.UI_DRDY_INT1_EN`)
/// pulses it once per sample after its accelerometer's start-up, and a store
/// to `INT_STATUS` changes nothing but logs its warning.
#[test]
fn data_ready_is_the_models_own_act() {
    let b = bench("drdy");
    let mut c = Vec::new();
    c.extend(spi(&b, &[0x2D | 0x80], 1));
    c.extend(spi(&b, &[0x2D, 0x08], 0));
    c.extend(spi(&b, &[0x2D | 0x80], 1));
    c.extend(spi(&b, &[0x65, 0x08], 0));
    c.extend(spi(&b, &[0x4E, 0x03], 0));
    c.push("emulation RunFor \"0.060000000\"".to_owned());
    c.extend(spi(&b, &[0x2D | 0x80], 1));
    let (text, lines) = run(&b, &b.models, &c);
    let imu = |l: &str| format!("[NOISY] {}: {l}", b.imu);
    assert_eq!(lines[0], imu("bank 0 read 0x2D -> 0x10"), "{text}");
    assert_eq!(
        lines[1],
        format!(
            "[WARNING] {}: bank 0 write to read-only register 0x2D ignored",
            b.imu
        ),
        "{text}"
    );
    assert_eq!(lines[2], imu("bank 0 write 0x2D <- 0x08"), "{text}");
    assert_eq!(lines[3], imu("bank 0 read 0x2D -> 0x00"), "{text}");
    let sets = lines.iter().filter(|l| **l == imu("INT1 set")).count();
    // 60 ms, the first sample at 10 ms, then one each millisecond.
    assert!((49..=52).contains(&sets), "{sets} pulses: {text}");
    assert_eq!(
        lines.last().unwrap(),
        &imu("bank 0 read 0x2D -> 0x08"),
        "{text}"
    );
}

/// F2, D6: a model that does not compile makes Renode print the line the
/// CLI refuses as `ModelLoadFailed`, stop the command line, and exit 0.
#[test]
fn a_model_that_does_not_compile_prints_the_load_error_and_exits_0() {
    let b = bench("broken");
    let broken = b.dir.join("Broken.cs");
    let good = fs::read_to_string(&b.models[0]).unwrap();
    fs::write(&broken, format!("#error AET-WO-4220 a broken copy\n{good}")).unwrap();
    let out = Command::new("renode")
        .args([
            "--disable-gui",
            "--console",
            "--plain",
            "-e",
            &format!(
                "include @{}; mach create; echo \"AFTER\"; quit",
                broken.display()
            ),
        ])
        .env("DOTNET_EnableDiagnostics", "0")
        .current_dir(&b.dir)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(
        text.contains("Errors during compilation or loading:"),
        "{text}"
    );
    assert!(text.contains("error CS"), "{text}");
    assert!(!text.lines().any(|l| l.trim() == "AFTER"), "{text}");
}

/// ArduPilot's `fifo_reset()` and `set_filter_and_scaling()` for the
/// ICM-42688-P at ODR code `odr` (`IV3` L247-252, L844-901; the bank values
/// for 2 kHz, `IV3` L813-818, or 1 kHz, L752-759), as `4400e` measured them
/// at 2 kHz (its FEEDBACK L154): the FIFO in STOP-on-FULL with accel, gyro
/// and temperature, the count in records and everything little-endian.
fn ardupilot_imu_start(b: &Bench, odr: u8) -> Vec<String> {
    let (delt, deltsqr, shift) = if odr == 0x05 {
        (12u8, 144u16, 8u8)
    } else {
        (6, 36, 10)
    };
    let mut c = Vec::new();
    for (r, v) in [
        (0x16, 0x80),
        (0x5F, 0x07),
        (0x4C, 0xC0),
        (0x4B, 0x02),
        (0x4E, 0x00),
        (0x4F, odr),
        (0x50, odr),
        (0x76, 0x01),
    ] {
        c.extend(spi(b, &[r, v], 0));
    }
    c.extend(spi(b, &[0x0B | 0x80], 1));
    c.extend(spi(b, &[0x76, 0x00], 0));
    let [lo, hi] = deltsqr.to_le_bytes();
    for (bank, r, v) in [
        (1, 0x0B, 0xA0),
        (1, 0x0C, delt),
        (1, 0x0D, lo),
        (1, 0x0E, (shift << 4) | (hi & 0x0F)),
        (2, 0x03, 5 << 1),
        (2, 0x04, 25),
        (2, 0x05, 10 << 4),
    ] {
        c.extend(spi(b, &[0x76, bank], 0));
        c.extend(spi(b, &[r, v], 0));
        c.extend(spi(b, &[0x76, 0x00], 0));
    }
    c.extend(spi(b, &[0x4D | 0x80], 1));
    c.extend(spi(b, &[0x4D, 0x51], 0));
    c.extend(spi(b, &[0x4E, 0x0F], 0));
    c
}

/// The bytes of every read of `reg` (bank 0) the IMU logged, in order.
fn reads(lines: &[String], imu: &str, reg: u8) -> Vec<Vec<u8>> {
    let head = format!("[NOISY] {imu}: bank 0 read 0x{reg:02X} -> ");
    lines
        .iter()
        .filter_map(|l| l.strip_prefix(&head))
        .map(|bytes| {
            bytes
                .split(' ')
                .map(|x| u8::from_str_radix(x.trim_start_matches("0x"), 16).unwrap())
                .collect()
        })
        .collect()
}

/// The twin's stimulus for the IMU, as the part table's data step and
/// `4400e`'s run set it: 29 degrees C and 1 g on Y.
fn stimulus(b: &Bench) -> Vec<String> {
    vec![
        format!("sysbus.{} Temperature 29", b.imu),
        format!("sysbus.{} AccelerationX 0", b.imu),
        format!("sysbus.{} AccelerationY 1", b.imu),
    ]
}

/// `AET-WO-4220c` §2: with ArduPilot's configuration at 2 kHz and the twin's
/// stimulus, the FIFO count grows at the ODR in virtual time from the
/// accelerometer's start-up (10 ms, Table 2 p.12), and each packet is the
/// datasheet's packet 3 byte for byte (FEEDBACK §5): header `0x68`; accel
/// (0, 2048, 0) and gyro (0, 0, 0), little-endian (B1); temperature
/// round(4 x 2.07) = 8 (p.25); the ODR timestamp, microseconds x 30/32 from
/// the model's reset (p.58, B6). Before the gyro's start-up (30 ms, Table 1
/// p.11) its slots hold 0 with FIFO_HOLD_LAST_DATA_EN set (B5). A read past
/// the count returns `0xFF` (B8), and a flush empties the FIFO (p.73).
#[test]
fn the_fifo_fills_at_the_odr_in_virtual_time_with_the_datasheets_packet() {
    let b = bench("fifo-2k");
    let mut c = stimulus(&b);
    c.extend(ardupilot_imu_start(&b, 0x05));
    for ms in ["0.009000000", "0.001250000", "0.030000000"] {
        c.push(format!("emulation RunFor \"{ms}\""));
        c.extend(spi(&b, &[0x2E | 0x80], 2));
    }
    // Two packets in one burst from FIFO_DATA, then the count again.
    c.extend(spi(&b, &[0x30 | 0x80], 32));
    c.extend(spi(&b, &[0x2E | 0x80], 2));
    // The data registers, as the generated firmware reads them.
    c.extend(spi(&b, &[0x1D | 0x80], 6));
    c.extend(spi(&b, &[0x4B, 0x02], 0));
    c.extend(spi(&b, &[0x2E | 0x80], 2));
    c.extend(spi(&b, &[0x30 | 0x80], 3));
    // Two more packets (40.5, 41.0 ms), then a soft reset at 41.25 ms (p.64):
    // it empties the FIFO and restarts the timestamp (B6). After 1.1 ms (no
    // access within 1 ms, p.64) the count is 0; configured again, the first
    // packet at 52.35 ms carries (52.35 - 41.25) ms x 30/32 = 10406 = 0x28A6.
    c.push("emulation RunFor \"0.001000000\"".to_owned());
    c.extend(spi(&b, &[0x11, 0x01], 0));
    c.push("emulation RunFor \"0.001100000\"".to_owned());
    c.extend(spi(&b, &[0x2E | 0x80], 2));
    c.extend(ardupilot_imu_start(&b, 0x05));
    c.push("emulation RunFor \"0.010250000\"".to_owned());
    c.extend(spi(&b, &[0x30 | 0x80], 16));
    let (text, lines) = run(&b, &b.models, &c);
    assert!(lines.iter().all(|l| l.starts_with("[NOISY]")), "{text}");
    let counts: Vec<u16> = reads(&lines, &b.imu, 0x2E)
        .iter()
        .map(|v| u16::from_le_bytes([v[0], v[1]]))
        .collect();
    // 9 ms: none; 10.25 ms: the 10.0 ms packet; 40.25 ms: 10.0 to 40.0 ms,
    // one each 0.5 ms (61); two read (59); flushed (0).
    assert_eq!(counts, [0, 1, 61, 59, 0, 0], "{text}");
    let data = reads(&lines, &b.imu, 0x30);
    #[rustfmt::skip]
    let want: [u8; 32] = [
        0x68, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x9F, 0x24,
        0x68, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x73, 0x26,
    ];
    assert_eq!(
        data[0], want,
        "10.0 ms: 9375 = 0x249F; 10.5 ms: 9843 = 0x2673: {text}"
    );
    assert_eq!(data[1], [0xFF, 0xFF, 0xFF], "{text}");
    assert_eq!(
        &data[2][14..16],
        [0xA6, 0x28],
        "after the soft reset: {text}"
    );
    // The data registers as before the FIFO (4220): 530 = 0x0212, 0, 2048.
    assert!(
        lines.contains(&format!(
            "[NOISY] {}: bank 0 read 0x1D -> 0x12 0x02 0x00 0x00 0x00 0x08",
            b.imu
        )),
        "{text}"
    );
}

/// p.58's own oracle at 1 kHz: with no RTC and `TMST_RES` 0 the timestamp in
/// the FIFO steps by 937 and 938; the gyro's slots carry its samples from its
/// start-up at 30 ms; and the temperature byte is p.25's 2.07 per degree.
#[test]
fn the_fifo_timestamp_steps_as_the_datasheet_gives_it_at_1_khz() {
    let b = bench("fifo-1k");
    let mut c = stimulus(&b);
    c.push(format!("sysbus.{} AngularRateZ 100", b.imu));
    c.push(format!("sysbus.{} Temperature 40", b.imu));
    c.extend(ardupilot_imu_start(&b, 0x06));
    c.push("emulation RunFor \"0.040500000\"".to_owned());
    c.extend(spi(&b, &[0x2E | 0x80], 2));
    c.extend(spi(&b, &[0x30 | 0x80], 31 * 16));
    // B10: -40 degrees C is -134.55 counts, saturated to -128 (0x80).
    c.push(format!("sysbus.{} Temperature -40", b.imu));
    c.push("emulation RunFor \"0.001000000\"".to_owned());
    c.extend(spi(&b, &[0x30 | 0x80], 16));
    // +100 degrees C is 155.25 counts, saturated to 127; the accelerometer to
    // 2 kHz under the gyro's 1 kHz (both re-phase at 41.5 ms): at 42.0 ms
    // the gyro has no new sample, and with FIFO_HOLD_LAST_DATA_EN set its
    // slot copies the last valid one (p.58), with HEADER_ODR_ACCEL (B7).
    c.push(format!("sysbus.{} Temperature 100", b.imu));
    c.extend(spi(&b, &[0x50, 0x05], 0));
    c.push("emulation RunFor \"0.001000000\"".to_owned());
    c.extend(spi(&b, &[0x30 | 0x80], 32));
    let (text, lines) = run(&b, &b.models, &c);
    assert!(lines.iter().all(|l| l.starts_with("[NOISY]")), "{text}");
    let more = reads(&lines, &b.imu, 0x30);
    assert_eq!(more[1][13], 0x80, "-40 degrees C: {text}");
    assert_eq!(more[2][0], 0x6A, "42.0 ms header: {text}");
    assert_eq!(more[2][13], 0x7F, "+100 degrees C: {text}");
    assert_eq!(
        i16::from_le_bytes([more[2][11], more[2][12]]),
        1640,
        "stale gyro: {text}"
    );
    assert_eq!(more[2][16], 0x68, "42.5 ms, both new: {text}");
    let count = reads(&lines, &b.imu, 0x2E)[0].clone();
    assert_eq!(count, [31, 0], "10 to 40 ms at 1 kHz: {text}");
    let data = &reads(&lines, &b.imu, 0x30)[0];
    let packets: Vec<&[u8]> = data.chunks(16).collect();
    let stamps: Vec<u16> = packets
        .iter()
        .map(|p| u16::from_le_bytes([p[14], p[15]]))
        .collect();
    let steps: Vec<u16> = stamps.windows(2).map(|w| w[1].wrapping_sub(w[0])).collect();
    assert!(steps.iter().all(|s| *s == 937 || *s == 938), "{steps:?}");
    assert!(steps.contains(&937) && steps.contains(&938), "{steps:?}");
    // Gyro Z: 0 before 30 ms (B5), then 100 dps x 16.4 = 1640 = 0x0668.
    let gz = |p: &[u8]| i16::from_le_bytes([p[11], p[12]]);
    assert_eq!(gz(packets[19]), 0, "29 ms");
    assert_eq!(gz(packets[20]), 1640, "30 ms");
    // 40 degrees C: round(15 x 2.07) = round(31.05) = 31 (p.25).
    assert!(packets.iter().all(|p| p[13] == 31), "temperature: {text}");
}

/// STOP-on-FULL at 2048 bytes less one packet (p.40, B9): 127 packets of 16
/// bytes, then nothing, and `FIFO_FULL_INT` (p.69); Bypass empties it. The count's byte order
/// follows `FIFO_COUNT_ENDIAN` (p.75; B2), and a read of `0x2E` latches both
/// bytes, so a later read of `0x2F` alone returns the latched value (p.69-70).
#[test]
fn the_fifo_stops_when_full_and_latches_its_count_in_either_order() {
    let b = bench("fifo-full");
    let mut c = stimulus(&b);
    c.extend(ardupilot_imu_start(&b, 0x05));
    c.extend(spi(&b, &[0x2D | 0x80], 1));
    c.push("emulation RunFor \"0.020250000\"".to_owned());
    // Big-endian count: 21 packets, 10.0 to 20.0 ms.
    c.extend(spi(&b, &[0x4C, 0xE0], 0));
    c.extend(spi(&b, &[0x2E | 0x80], 2));
    c.extend(spi(&b, &[0x2E | 0x80], 1));
    c.push("emulation RunFor \"0.005000000\"".to_owned());
    c.extend(spi(&b, &[0x2F | 0x80], 1));
    c.push("emulation RunFor \"0.100000000\"".to_owned());
    c.extend(spi(&b, &[0x4C, 0xC0], 0));
    c.extend(spi(&b, &[0x2E | 0x80], 2));
    c.extend(spi(&b, &[0x2D | 0x80], 1));
    // Bypass: no FIFO writes, so the count reads 0 (p.65), and stays 0.
    c.extend(spi(&b, &[0x16, 0x00], 0));
    c.push("emulation RunFor \"0.002000000\"".to_owned());
    c.extend(spi(&b, &[0x2E | 0x80], 2));
    let (text, lines) = run(&b, &b.models, &c);
    assert!(lines.iter().all(|l| l.starts_with("[NOISY]")), "{text}");
    let imu = |l: &str| format!("[NOISY] {}: {l}", b.imu);
    for want in [
        "bank 0 read 0x2E -> 0x00 0x15",
        "bank 0 read 0x2E -> 0x00",
        "bank 0 read 0x2F -> 0x15",
        "bank 0 read 0x2E -> 0x7F 0x00",
        "bank 0 read 0x2D -> 0x0A",
        "bank 0 read 0x2E -> 0x00 0x00",
    ] {
        assert!(lines.contains(&imu(want)), "`{want}`: {text}");
    }
}

/// Packets 1 and 2 (8 bytes, p.38, p.41), and every selection outside the
/// modelled set named by a warning (`AET-WO-4220c` §4.2), as is a read that
/// ends inside a packet with `FIFO_RESUME_PARTIAL_RD` 0 (B4).
#[test]
fn packets_1_and_2_and_each_selection_left_out_says_so() {
    let b = bench("fifo-sets");
    let mut c = stimulus(&b);
    c.extend(ardupilot_imu_start(&b, 0x05));
    c.extend(spi(&b, &[0x5F, 0x05], 0));
    c.push("emulation RunFor \"0.010250000\"".to_owned());
    c.extend(spi(&b, &[0x30 | 0x80], 8));
    c.extend(spi(&b, &[0x5F, 0x06], 0));
    c.push("emulation RunFor \"0.020000000\"".to_owned());
    c.extend(spi(&b, &[0x4B, 0x02], 0));
    c.push("emulation RunFor \"0.000500000\"".to_owned());
    c.extend(spi(&b, &[0x30 | 0x80], 3));
    for (r, v) in [
        (0x5F, 0x17),
        (0x5F, 0x0F),
        (0x5F, 0x03),
        (0x5F, 0x27),
        (0x54, 0x2B),
        (0x4E, 0x2F),
        (0x16, 0x40),
        (0x60, 0x01),
        (0x65, 0x04),
        (0x63, 0x0A),
    ] {
        c.extend(spi(&b, &[r, v], 0));
    }
    // Back in STOP-on-FULL but with the 20-bit packet selected: no packets.
    c.extend(spi(&b, &[0x16, 0x80], 0));
    c.extend(spi(&b, &[0x5F, 0x17], 0));
    c.extend(spi(&b, &[0x4B, 0x02], 0));
    c.push("emulation RunFor \"0.002000000\"".to_owned());
    c.extend(spi(&b, &[0x2E | 0x80], 2));
    c.extend(spi(&b, &[0x30 | 0x80], 1));
    let (text, lines) = run(&b, &b.models, &c);
    assert_eq!(reads(&lines, &b.imu, 0x2E), [vec![0x00, 0x00]], "{text}");
    assert_eq!(
        reads(&lines, &b.imu, 0x30).last().unwrap(),
        &[0xFF],
        "{text}"
    );
    let data = reads(&lines, &b.imu, 0x30);
    assert_eq!(
        data[0],
        [0x40, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x08],
        "packet 1: {text}"
    );
    assert_eq!(
        data[1],
        [0x20, 0x00, 0x00],
        "packet 2's first bytes: {text}"
    );
    let warn = |l: &str| format!("[WARNING] {}: {l}", b.imu);
    for want in [
        "a partial FIFO packet read with FIFO_RESUME_PARTIAL_RD = 0 is not modelled; the next read resumes at the next byte",
        "FIFO: FIFO_HIRES_EN (the 20-bit packet) is not modelled; no packets are written",
        "FIFO: FIFO_TMST_FSYNC_EN (FSYNC in the FIFO) is not modelled; no packets are written",
        "FIFO: FIFO_TEMP_EN = 0 with a sensor in the FIFO is not modelled; no packets are written",
        "FIFO: the watermark (FIFO_WM, FIFO_WM_GT_TH, FIFO_THS_INT) is not modelled",
        "FIFO: TMST_CONFIG other than TMST_EN 1, TMST_DELTA_EN 0, TMST_RES 0 (packet 3's timestamp) is not modelled; no packets are written",
        "FIFO: PWR_MGMT0.TEMP_DIS with the FIFO's temperature byte is not modelled; no packets are written",
        "FIFO: FIFO_MODE Stream-to-FIFO is not modelled; no packets are written",
        "FIFO: FIFO interrupts on a pin (INT_SOURCE 0x65 bits 2:1) are not modelled",
        "FIFO: INT_CONFIG0 FIFO interrupt clear on a FIFO data read is not modelled",
    ] {
        assert!(lines.contains(&warn(want)), "`{want}`: {text}");
    }
}

/// The adversarial pass (`AET-WO-4220c` §5): a packet in the datasheet's own
/// byte order, high byte first (`SENSOR_DATA_ENDIAN` 1, p.38), with the gyro
/// at 1 kHz under an accelerometer at 2 kHz. A packet with no new gyro sample
/// carries -32768 in its gyro slots with `FIFO_HOLD_LAST_DATA_EN` 0 (p.58,
/// p.75 (c)). The count in bytes (`FIFO_COUNT_REC` 0, p.75), and
/// `HEADER_ODR_ACCEL` on the first packet after the accelerometer's ODR
/// changes (p.39, B7).
#[test]
fn a_big_endian_packet_with_an_invalid_gyro_slot_follows_the_datasheet() {
    let b = bench("fifo-be");
    let mut c = stimulus(&b);
    c.push(format!("sysbus.{} AngularRateZ 100", b.imu));
    c.extend(ardupilot_imu_start(&b, 0x05));
    // No write within 200 us of a sensor leaving OFF (DS p.76).
    c.push("emulation RunFor \"0.000300000\"".to_owned());
    c.extend(spi(&b, &[0x4C, 0x50], 0));
    c.extend(spi(&b, &[0x4F, 0x06], 0));
    c.push("emulation RunFor \"0.030450000\"".to_owned());
    c.extend(spi(&b, &[0x4C, 0x10], 0));
    c.extend(spi(&b, &[0x2E | 0x80], 2));
    c.extend(spi(&b, &[0x30 | 0x80], 42 * 16));
    // An ODR write re-phases both sensors to now + their period (the model's
    // OnOutputDataRate): both sample next at 31.75 ms.
    c.extend(spi(&b, &[0x50, 0x06], 0));
    c.push("emulation RunFor \"0.001000000\"".to_owned());
    c.extend(spi(&b, &[0x30 | 0x80], 16));
    // The gyro to 2 kHz at 31.75 ms: its next sample at 32.25 ms, alone, with
    // HEADER_ODR_GYRO (B7).
    c.extend(spi(&b, &[0x4F, 0x05], 0));
    c.push("emulation RunFor \"0.000600000\"".to_owned());
    c.extend(spi(&b, &[0x30 | 0x80], 16));
    let (text, lines) = run(&b, &b.models, &c);
    assert!(lines.iter().all(|l| l.starts_with("[NOISY]")), "{text}");
    // 10.0 to 30.5 ms at 2 kHz: 42 packets of 16 bytes, 672 = 0x02A0.
    assert_eq!(reads(&lines, &b.imu, 0x2E)[0], [0xA0, 0x02], "{text}");
    let data = reads(&lines, &b.imu, 0x30);
    let packets: Vec<&[u8]> = data[0].chunks(16).collect();
    #[rustfmt::skip]
    let at_30_0: [u8; 16] = [
        0x68, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x06, 0x68, 0x08, 0x6D, 0xDD,
    ];
    #[rustfmt::skip]
    let at_30_5: [u8; 16] = [
        0x68, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
        0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x08, 0x6F, 0xB1,
    ];
    assert_eq!(packets[40], at_30_0, "30.0 ms, 28125 = 0x6DDD: {text}");
    assert_eq!(packets[41], at_30_5, "30.5 ms, no gyro sample: {text}");
    // Before the gyro's start-up its slots are invalid too, with the bit 0.
    assert_eq!(&packets[0][7..13], [0x80, 0x00, 0x80, 0x00, 0x80, 0x00]);
    // 31.75 ms: the first packet since the accelerometer's ODR became 1 kHz.
    assert_eq!(data[1][0], 0x6A, "HEADER_ODR_ACCEL: {text}");
    assert_eq!(data[2][0], 0x69, "HEADER_ODR_GYRO: {text}");
}
