//! `AET-WO-4300a`, tier 1: the stock timer tables of `aethelgard_twin::timer`
//! are exactly what Renode 1.17.0's `stm32h7.repl` says. The file is read
//! from the vendored copy at run time, checked against its sha256 in
//! `RENODE_FILES`, and parsed for the four shapes the tables hold.

use std::fs;
use std::path::Path;

use aethelgard_twin::platform::{RENODE_FILES, STOCK_FILE};
use aethelgard_twin::timer::{
    GPIO_TIMER_INPUTS, INVERTED_AF, STOCK_TIMERS, TIMER_HZ, TIMER_ROUTES,
};
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
                .any(|l| l.trim() == format!("frequency: {TIMER_HZ}")),
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
