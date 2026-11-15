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
/// `s2-ardupilot.md` 1.2-1.3) and its probe and init for the BMP581
/// (`AP_Baro_BMP581` L81-133, 6.1), replayed: every read it gates on
/// answers what it needs, every checked register reads back, and no
/// register it touches is unmodelled.
#[test]
fn ardupilot_s_probe_and_init_pass_against_our_models() {
    let b = bench("ardupilot");
    let mut c = Vec::new();
    // ICM-42688-P: probe (IV3:1005), then start() (IV3:247-902).
    c.extend(spi(&b, &[0xF5], 1));
    for (r, v) in [
        (0x16, 0x80),
        (0x5F, 0x07),
        (0x4C, 0xC0),
        (0x4B, 0x02),
        (0x4E, 0x00),
        (0x4F, 0x06),
        (0x50, 0x06),
        (0x76, 0x01),
    ] {
        c.extend(spi(&b, &[r, v], 0));
    }
    c.extend(spi(&b, &[0x0B | 0x80], 1));
    c.extend(spi(&b, &[0x76, 0x00], 0));
    for (bank, r, v) in [
        (1, 0x0B, 0xA0),
        (1, 0x0C, 0x06),
        (1, 0x0D, 0x24),
        (1, 0x0E, 0xA0),
        (2, 0x03, 0x0A),
        (2, 0x04, 0x19),
        (2, 0x05, 0xA0),
    ] {
        c.extend(spi(&b, &[0x76, bank], 0));
        c.extend(spi(&b, &[r, v], 0));
        c.extend(spi(&b, &[0x76, 0x00], 0));
    }
    c.extend(spi(&b, &[0x4D | 0x80], 1));
    c.extend(spi(&b, &[0x4D, 0x51], 0));
    c.extend(spi(&b, &[0x4E, 0x0F], 0));
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
    let (text, lines) = run(&b, &b.models, &c);
    assert!(
        lines.iter().all(|l| l.starts_with("[NOISY]")),
        "no warning from either model: {text}"
    );
    let imu = |l: &str| format!("[NOISY] {}: {l}", b.imu);
    let baro = |l: &str| format!("[NOISY] {}: {l}", b.baro);
    for want in [
        imu("bank 0 read 0x75 -> 0x47"),
        imu("bank 1 write 0x0E <- 0xA0"),
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
