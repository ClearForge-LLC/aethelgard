//! `AET-WO-4300a`, tier 1: the stock timer tables of `aethelgard_twin::timer`
//! are exactly what Renode 1.17.0's `stm32h7.repl` says. The file is read
//! from the vendored copy at run time, checked against its sha256 in
//! `RENODE_FILES`, and parsed for the four shapes the tables hold.

use std::fs;
use std::path::Path;

use aethelgard_twin::platform::{RENODE_BUILD, RENODE_FILES, STOCK_FILE};
use aethelgard_twin::timer::{GPIO_TIMER_INPUTS, INVERTED_AF, STOCK_TIMERS, TIMER_ROUTES};

/// The stock file's `frequency` for every timer: no firmware's clock, which
/// the platform's clock entries override (`AET-WO-4400g`).
const STOCK_TIMER_HZ: u64 = 250_000_000;
use sha2::{Digest, Sha256};

fn stock() -> String {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/renode-1.17.0")
        .join(STOCK_FILE);
    let bytes = fs::read(&file).unwrap();
    let (_, want) = RENODE_FILES.iter().find(|f| f.0 == STOCK_FILE).unwrap();
    let got: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(got, *want);
    String::from_utf8(bytes).unwrap()
}

/// `name:` alone on a line opens a block of routes until a blank line.
fn blocks<'a>(text: &'a str, prefix: &str) -> Vec<(&'a str, Vec<&'a str>)> {
    let mut out = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let Some(name) = line.strip_suffix(':') else {
            continue;
        };
        if !name.starts_with(prefix) || name.contains(' ') {
            continue;
        }
        let mut routes = Vec::new();
        while let Some(next) = lines.peek() {
            if next.trim().is_empty() {
                break;
            }
            routes.push(next.trim());
            lines.next();
        }
        out.push((name, routes));
    }
    out
}

#[test]
fn the_timer_instances_and_their_frequency_are_the_files() {
    let text = stock();
    let mut found = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("timer") else {
            continue;
        };
        let Some((n, model)) = rest.split_once(": ") else {
            continue;
        };
        if !model.starts_with("Timers.STM32_Timer @") {
            continue;
        }
        let address = lines[i + 1]
            .trim()
            .trim_start_matches("sysbus ")
            .trim_end_matches(';');
        let address = u32::from_str_radix(address.trim_start_matches("0x"), 16).unwrap();
        let block: Vec<&str> = lines[i..]
            .iter()
            .take_while(|l| !l.is_empty())
            .copied()
            .collect();
        assert!(
            block
                .iter()
                .any(|l| l.trim() == format!("frequency: {STOCK_TIMER_HZ}")),
            "timer{n}"
        );
        found.push((n.parse::<u32>().unwrap(), address));
    }
    found.sort_unstable();
    assert_eq!(found, STOCK_TIMERS);
}

#[test]
fn the_output_routes_are_the_files() {
    let text = stock();
    let mut found = Vec::new();
    for (name, routes) in blocks(&text, "timer") {
        let timer: u32 = name["timer".len()..].parse().unwrap();
        for route in routes {
            let (channel, to) = route.split_once(" -> ").unwrap();
            for d in to.split(" | ") {
                let (port_pin, af) = d.trim().split_once('@').unwrap();
                let (port, pin) = port_pin
                    .strip_prefix("gpioPort")
                    .unwrap()
                    .split_once('#')
                    .unwrap();
                found.push((
                    timer,
                    channel.parse::<u32>().unwrap() + 1,
                    port.chars().next().unwrap(),
                    pin.parse::<u32>().unwrap(),
                    af.parse::<u8>().unwrap(),
                ));
            }
        }
    }
    found.sort_unstable();
    assert_eq!(found, TIMER_ROUTES);
}

#[test]
fn the_input_routes_are_the_files_and_every_routed_pin_has_one() {
    let text = stock();
    let mut found = Vec::new();
    for (name, routes) in blocks(&text, "gpioPort") {
        let port = name.chars().last().unwrap();
        for route in routes {
            let (pin, to) = route.split_once(" -> ").unwrap();
            found.push((port, pin.parse::<u32>().unwrap(), to.trim().to_owned()));
        }
    }
    found.sort();
    let want: Vec<(char, u32, String)> = GPIO_TIMER_INPUTS
        .iter()
        .map(|(p, n, r)| (*p, *n, (*r).to_owned()))
        .collect();
    assert_eq!(found, want);
    for (_, _, port, pin, _) in TIMER_ROUTES {
        assert!(
            GPIO_TIMER_INPUTS
                .iter()
                .any(|(p, n, _)| (*p, *n) == (port, pin)),
            "P{port}{pin}"
        );
    }
}

#[test]
fn the_inverted_pins_are_the_files() {
    let text = stock();
    let mut found = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let Some(port) = line
            .strip_prefix("gpioPort")
            .filter(|r| r.contains(": GPIOPort.STM32_GPIOPort"))
            .and_then(|r| r.chars().next())
        else {
            continue;
        };
        for l in lines[i..].iter().take_while(|l| !l.is_empty()) {
            let Some(list) = l.trim().strip_prefix("invertedAFPins: ") else {
                continue;
            };
            for entry in list
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split("], [")
            {
                let nums: Vec<u32> = entry.split(", ").map(|v| v.parse().unwrap()).collect();
                for &af in &nums[1..] {
                    found.push((port, nums[0], u8::try_from(af).unwrap()));
                }
            }
        }
    }
    found.sort_unstable();
    assert_eq!(found, INVERTED_AF);
}

fn topology(fixture: &str) -> aethelgard_ir::SolvedTopology {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../aethelgard-cli/tests/fixtures")
        .join(format!("{fixture}.json"));
    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap()
}

/// Every instance of the stock file with a `frequency:`, by name: each is a
/// timer (LPTIM1–5 too), a USART (LPUART1 too) or the DWT, which a clock
/// sets, or the independent watchdog, which counts LSI, outside the solved
/// clock tree, and is held apart by name.
fn stock_clocked() -> Vec<String> {
    let text = stock();
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some((name, model)) = line.split_once(": ") else {
            continue;
        };
        let clocked = lines[i..]
            .iter()
            .take_while(|l| !l.is_empty())
            .any(|l| l.trim().starts_with("frequency: "));
        if name.contains(' ') || !clocked {
            continue;
        }
        if name == "watchdog" {
            assert!(
                model.starts_with("Timers.STM32_IndependentWatchdog @"),
                "{model}"
            );
            continue;
        }
        assert!(
            [
                "Timers.STM32_Timer @",
                "Timers.STM32L0_LpTimer @",
                "UART.STM32F7_USART @",
                "Miscellaneous.DWT @",
            ]
            .iter()
            .any(|m| model.starts_with(m)),
            "{name}: {model} has a frequency no clock sets"
        );
        out.push(name.to_owned());
    }
    out.sort();
    out
}

/// `AET-WO-4400g` §1.1: under the solved clock, every timer, USART and the
/// DWT the stock file carries gets an entry, none keeps a stock figure, and
/// each is the reset clock's 64 MHz (RM0433 Rev 9 §8.7.2, Table 57,
/// §8.7.21–§8.7.22). §1.2: under a declared clock, every one but the DWT and
/// LPUART1, which no declaration sets and the platform names as unclocked;
/// the timers at the declared 200 MHz, the USARTs at 72 MHz.
#[test]
fn every_timer_usart_and_the_dwt_of_the_stock_file_is_clocked() {
    let want = stock_clocked();
    assert_eq!(want.len(), 14 + 5 + 9 + 1, "{want:?}");
    let names = |c: &[(String, u64)]| {
        let mut n: Vec<String> = c.iter().map(|(n, _)| n.clone()).collect();
        n.sort();
        n
    };
    let solved = topology("motors-pwm");
    let a = aethelgard_twin::platform::clocked(&solved);
    assert_eq!(names(&a), want);
    assert!(a.iter().all(|(_, hz)| *hz == 64_000_000), "{a:?}");
    assert!(aethelgard_twin::platform::unclocked(&solved).is_empty());
    let declared = topology("sentinel-02-own");
    let c = aethelgard_twin::platform::clocked(&declared);
    let unclocked = aethelgard_twin::platform::unclocked(&declared);
    assert_eq!(
        unclocked,
        [
            "dwt", "lptimer1", "lptimer2", "lptimer3", "lptimer4", "lptimer5", "lpuart1"
        ]
    );
    let mut all = names(&c);
    all.extend(unclocked.iter().map(|s| (*s).to_owned()));
    all.sort();
    assert_eq!(all, want);
    for (name, hz) in &c {
        let declared_hz = if name.starts_with("timer") {
            200_000_000
        } else {
            72_000_000
        };
        assert_eq!(*hz, declared_hz, "{name}");
    }
    // Each APB bus's timers take that bus's declaration (adversarial pass: a
    // swap of the buses survived while both were declared alike).
    let mut split = declared.clone();
    split.foreign_clock.as_mut().unwrap().apb2_timers_hz = 100_000_000;
    let c = aethelgard_twin::platform::clocked(&split);
    let hz = |n: &str| c.iter().find(|(name, _)| name == n).unwrap().1;
    for (timer, want) in [
        ("timer1", 100),
        ("timer2", 200),
        ("timer8", 100),
        ("timer15", 100),
        ("timer5", 200),
    ] {
        assert_eq!(hz(timer), want * 1_000_000, "{timer}");
    }
}

/// `AET-WO-4400g` §1.5: the pinned build is typed once, here, and is the
/// tools image's: `container/smoke.sh` holds the image to that `build:` line.
#[test]
fn the_renode_build_is_the_images_pin() {
    let smoke =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../container/smoke.sh"))
            .unwrap();
    let pinned: Vec<&str> = smoke
        .lines()
        .filter_map(|l| l.strip_prefix("is_line renode.version \"  build: "))
        .map(|l| l.trim_end_matches('"'))
        .collect();
    assert_eq!(pinned, [RENODE_BUILD]);
}
