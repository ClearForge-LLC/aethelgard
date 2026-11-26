//! `AET-WO-4412` D4 (ruled 2026-11-26): DShot on a timer with a DMA request.
//!
//! DShot moves its bits by its timer's update DMA request, a `DCR`/`DMAR`
//! burst into `CCRx` per bit (ArduPilot sets DMAMUX1 to TIM1_UP, request 15,
//! measured in `spike/4412`). TIM12, TIM13 and TIM14 raise no DMA request
//! (RM0433 Rev 9 Table 124, p. 697), so the solver offers them to PWM only,
//! and a DShot output that only they could serve is `NoDshotDmaRequest`.
//! Against the real STM32H753ZITx pin table, through `aethelgard solve`.

mod common;

use std::path::Path;
use std::process::Command;

use serde_json::Value;

fn solve(dir: &Path, name: &str, v: &Value) -> (Option<i32>, Value) {
    let p = dir.join(name);
    std::fs::write(&p, serde_json::to_vec_pretty(v).unwrap()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["solve", "--file", p.to_str().unwrap()])
        .env("TMPDIR", dir)
        .current_dir(dir)
        .output()
        .unwrap();
    (
        out.status.code(),
        serde_json::from_slice(&out.stdout).unwrap_or(Value::Null),
    )
}

fn motors_dshot600() -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/motors-dshot600.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

/// `PF8`'s only timer channel is `TIM13_CH1` (its `TIM16_CH1N` is
/// complementary, not a channel this engine drives): a DShot output held
/// there is refused by name, naming the output, the protocol, the timer and
/// the table; the same pin solves for PWM, which needs no request; and the
/// fixture's DShot outputs, placed by the solver, each land on a timer with
/// an update request.
#[test]
fn a_dshot_output_on_a_timer_with_no_dma_request_is_refused_by_name() {
    let dir = common::private_tmp("dshot-dma");
    let mut v = motors_dshot600();
    v["motor_outputs"][0]["pin"] = "PF8".into();
    let (code, r) = solve(&dir.0, "pf8-dshot.json", &v);
    assert_eq!(code, Some(2), "{r}");
    assert_eq!(r["error"]["kind"], "NoDshotDmaRequest", "{r}");
    let m = r["error"]["message"].as_str().unwrap();
    assert!(
        m.contains("`MOTOR1`")
            && m.contains("dshot600")
            && m.contains("TIM13")
            && m.contains("Table 124, p. 697"),
        "{m}"
    );

    for motor in v["motor_outputs"].as_array_mut().unwrap() {
        motor["protocol"] = "pwm".into();
        motor["pwm"] = serde_json::json!({"rate_hz": 400, "pulse_min_us": 1000,
            "pulse_max_us": 2000, "idle": {"pulse_us": 1000}, "resolution_ps": 1_000_000});
    }
    let (code, t) = solve(&dir.0, "pf8-pwm.json", &v);
    assert_eq!(code, Some(0), "{t}");
    assert_eq!(t["assignments"]["MOTOR1"]["signal"], "TIM13_CH1", "{t}");

    let (code, t) = solve(&dir.0, "placed.json", &motors_dshot600());
    assert_eq!(code, Some(0), "{t}");
    for m in ["MOTOR1", "MOTOR2", "MOTOR3", "MOTOR4"] {
        let signal = t["assignments"][m]["signal"].as_str().unwrap();
        let (n, _) = aethelgard_ir::motor::timer_channel(signal).unwrap();
        assert!(
            aethelgard_ir::motor::up_request(n).is_some(),
            "{m} on {signal}"
        );
    }
    for timer in [12, 13, 14] {
        assert_eq!(aethelgard_ir::motor::up_request(timer), None, "TIM{timer}");
    }
}

/// `serial-dshot-9600`, the CI's DShot board, with its console.
fn serial_dshot() -> Value {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/serial-dshot-9600.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

/// `AET-WO-4412` (3): ArduPilot's `MOT_PWM_TYPE` is one parameter for every
/// motor, so a board whose outputs mix protocols (two DShot600 outputs on
/// TIM1, two PWM outputs the solver places on another timer) is refused by
/// `generate hwdef` by name, and writes neither file; the all-DShot board
/// writes `defaults.parm` beside `hwdef.dat`.
#[test]
fn a_board_mixing_motor_protocols_has_no_mot_pwm_type_and_is_refused_by_name() {
    let dir = common::private_tmp("mot-pwm-type");
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
            .args(args)
            .env("TMPDIR", &dir.0)
            .current_dir(&dir.0)
            .output()
            .unwrap();
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let hwdef = |name: &str, v: &Value| {
        let (code, topology) = solve(&dir.0, &format!("{name}.json"), v);
        assert_eq!(code, Some(0), "{topology}");
        let t = dir.0.join(format!("{name}.topology.json"));
        std::fs::write(&t, serde_json::to_vec(&topology).unwrap()).unwrap();
        let out = dir.0.join(format!("{name}.hwdef.dat"));
        let r = run(&[
            "generate",
            "hwdef",
            "--topology",
            t.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ]);
        (r, out)
    };
    let ((code, r), out) = hwdef("all-dshot", &serial_dshot());
    assert_eq!(code, Some(0), "{r}");
    let defaults = std::fs::read_to_string(dir.0.join("all-dshot.defaults.parm")).unwrap();
    assert!(defaults.ends_with("\nMOT_PWM_TYPE 6\n"), "{defaults}");
    assert!(out.exists());
    let mut v = serial_dshot();
    for m in ["MOTOR3", "MOTOR4"] {
        let motor = v["motor_outputs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|o| o["label"] == m)
            .unwrap();
        motor["protocol"] = "pwm".into();
        motor["pwm"] = serde_json::json!({"rate_hz": 400, "pulse_min_us": 1000,
            "pulse_max_us": 2000, "idle": {"pulse_us": 1000}, "resolution_ps": 1_000_000});
    }
    let ((code, r), out) = hwdef("mixed", &v);
    assert_eq!(code, Some(2), "{r}");
    assert!(r.contains("\"MotorProtocolsMixed\""), "{r}");
    assert!(!out.exists() && !dir.0.join("mixed.defaults.parm").exists());
}
