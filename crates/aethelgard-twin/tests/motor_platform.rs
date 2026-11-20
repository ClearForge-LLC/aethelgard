//! `AET-WO-4300a` §1.4: each solved motor output gets an observer on its
//! pin, with the pin's stock route kept; the model's timing is predicted from
//! the topology alone; and the expected image holds the timer's registers.

use std::fs;
use std::path::Path;

use aethelgard_ir::{SolvedTopology, Specification, solve};
use aethelgard_twin::boot::{self, Register};
use aethelgard_twin::{platform, timer};

fn topology(fixture: &str) -> SolvedTopology {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../aethelgard-cli/tests/fixtures")
        .join(format!("{fixture}.json"));
    let bytes = fs::read(path).unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    solve(&spec, &table, &"0".repeat(64)).unwrap()
}

#[test]
fn each_output_is_observed_at_the_model_s_timing() {
    let t = topology("motors-pwm");
    let obs = timer::observers(&t).unwrap();
    let got: Vec<(&str, &str, &str, u32, u32, u64, u64)> = obs
        .iter()
        .map(|o| {
            (
                o.label.as_str(),
                o.pin.as_str(),
                o.source.as_str(),
                o.timer,
                o.channel,
                o.period_ns,
                o.high_ns,
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            (
                "MOTOR1",
                "PA8",
                "gpioPortA.pa8obs",
                1,
                1,
                2_499_000,
                1_000_000
            ),
            (
                "MOTOR2",
                "PA9",
                "gpioPortA.pa9obs",
                1,
                2,
                2_499_000,
                1_000_000
            ),
            (
                "MOTOR3",
                "PA10",
                "gpioPortA.pa10obs",
                1,
                3,
                2_499_000,
                1_000_000
            ),
            (
                "MOTOR4",
                "PA11",
                "gpioPortA.pa11obs",
                1,
                4,
                2_499_000,
                1_000_000
            ),
        ]
    );
    // DShot600: ARR 106 and CCR 40 at PSC 0, a 15.625 ns tick at the 64 MHz
    // reset clock, the period realized in whole ns: 1,657, not 1,656.25
    // (`AET-WO-4400g`).
    let d = timer::observers(&topology("motors-dshot600")).unwrap();
    assert!(
        d.iter()
            .all(|o| (o.kernel_hz, o.period_ns, o.high_ns) == (64_000_000, 1_657, 625))
    );
}

#[test]
fn the_platform_adds_an_observer_and_keeps_each_pin_s_stock_route() {
    let text = platform::emit(&topology("motors-pwm")).unwrap();
    for (pin, n) in [("PA8", 8), ("PA9", 9), ("PA10", 10), ("PA11", 11)] {
        let name = format!("pa{n}obs");
        assert!(
            text.contains(&format!("{name}: Miscellaneous.LED @ gpioPortA {n}\n")),
            "{pin}"
        );
    }
    // Then the brown-out level's presentation (`AET-WO-4230` D10), the
    // clock's entries (`AET-WO-4400g`), the DWT's at the core clock last,
    // and last the PWR override (`AET-WO-4240v` D-3 (a)).
    let presentation = aethelgard_twin::boot::brown_out(&topology("motors-pwm"))
        .unwrap()
        .presentation();
    let text = text
        .strip_suffix(&platform::pwr_override().unwrap())
        .expect("the PWR override last");
    let (head, clock) = text
        .split_once("\n// The clock (AET-WO-4400g)")
        .expect("the clock's entries");
    assert!(
        head.ends_with(&format!(
            "\ngpioPortA:\n    8 -> timer1@00 | pa8obs@0\n    9 -> timer1@01 | pa9obs@0\n    10 -> timer1@02 | pa10obs@0\n    11 -> timer1@03 | pa11obs@0\n{presentation}"
        )),
        "{text}"
    );
    assert!(
        clock.contains("\ntimer1:\n    frequency: 64000000\n"),
        "{clock}"
    );
    assert!(
        clock.ends_with("\ndwt:\n    frequency: 64000000\n"),
        "{clock}"
    );
    let listed: Vec<(String, String)> = platform::instances(
        &topology("motors-pwm"),
        &Default::default(),
        &Default::default(),
    )
    .unwrap()
    .into_iter()
    .map(|i| (i.name, i.model))
    .collect();
    assert!(
        listed.contains(&("timer1".into(), "STM32_Timer".into())),
        "{listed:?}"
    );
}

#[test]
fn a_route_renode_lacks_or_inverts_is_refused_by_name() {
    let mut t = topology("motors-pwm");
    t.assignments.get_mut("MOTOR1").unwrap().alternate_function = 2;
    assert_eq!(timer::observers(&t).unwrap_err().kind(), "NoTimerRoute");
    // PB13 carries TIM1's channel 1 at AF1 as a complementary output, which
    // Renode's port inverts.
    let mut t = topology("motors-pwm");
    t.assignments.get_mut("MOTOR1").unwrap().pin = "PB13".into();
    assert_eq!(timer::observers(&t).unwrap_err().kind(), "InvertedRoute");
    assert_eq!(platform::emit(&t).unwrap_err().kind(), "InvertedRoute");
}

fn reg(name: &str, address: u32, mask: u32, value: u32) -> Register {
    Register {
        name: name.into(),
        address,
        mask,
        value,
    }
}

#[test]
fn the_expected_image_holds_the_timer_s_stored_fields_and_names_its_tags() {
    let image = boot::expected(&topology("motors-pwm")).unwrap();
    let tim1: Vec<&Register> = image
        .registers
        .iter()
        .filter(|r| r.name.starts_with("TIM1."))
        .collect();
    let want = [
        reg("TIM1.CR1", 0x4001_0000, 0xFF, 0x81),
        reg("TIM1.CR2", 0x4001_0004, 0x80, 0),
        reg("TIM1.SMCR", 0x4001_0008, 0x77, 0),
        reg("TIM1.DIER", 0x4001_000C, 0x5F, 0),
        reg("TIM1.CCMR1", 0x4001_0018, 0x7373, 0x6060),
        reg("TIM1.CCMR2", 0x4001_001C, 0x7373, 0x6060),
        reg("TIM1.CCER", 0x4001_0020, 0xBBBB, 0x1111),
        reg("TIM1.PSC", 0x4001_0028, u32::MAX, 63),
        reg("TIM1.ARR", 0x4001_002C, u32::MAX, 2499),
        reg("TIM1.RCR", 0x4001_0030, 0xFF, 0),
        reg("TIM1.CCR1", 0x4001_0034, u32::MAX, 1000),
        reg("TIM1.CCR2", 0x4001_0038, u32::MAX, 1000),
        reg("TIM1.CCR3", 0x4001_003C, u32::MAX, 1000),
        reg("TIM1.CCR4", 0x4001_0040, u32::MAX, 1000),
    ];
    assert_eq!(tim1, want.iter().collect::<Vec<_>>());
    // TIM1EN (bit 0) beside SPI1EN (bit 12) in APB2ENR.
    let apb2 = image
        .registers
        .iter()
        .find(|r| r.name == "RCC.APB2ENR")
        .unwrap();
    assert_eq!(apb2.value, 0x0000_1001);
    // PA8..PA11 in AF1.
    let afrh = image
        .registers
        .iter()
        .find(|r| r.name == "GPIOA.AFRH")
        .unwrap();
    assert_eq!((afrh.mask, afrh.value), (0x0000_FFFF, 0x0000_1111));
    for tag in [
        "TIM1.BDTR.MOE",
        "TIM1.BDTR.BKE",
        "TIM1.SMCR.ECE",
        "GPIOA.OTYPER",
        "TIM1.CCMR1.OC1PE",
        "TIM1.CCMR1.OC2PE",
        "TIM1.CCMR2.OC3PE",
        "TIM1.CCMR2.OC4PE",
    ] {
        assert!(image.unobservable.iter().any(|u| u == tag), "{tag}");
    }
}
