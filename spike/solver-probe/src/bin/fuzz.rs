//! `fuzz <seed> <count>`: random specifications, solved under every pruning
//! setting with a node budget. Checks that every setting that finishes gives
//! the same bytes, and checks each solution independently of the search.
//! Prints the slowest case per setting; writes the slowest for fc+im+pm to
//! `$WORST` if set.

use std::collections::BTreeSet;
use std::time::Instant;

use aethelgard_ir::{
    I2cBus, Mcu, PinTable, Signal, Specification, SpiBus, SpiDevice, VoltageDomain,
};
use aethelgard_spike_solver::{Pruning, RESERVED, SolveError, SpikeTopology, Stats, solve};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*: fixed, seedable, no dependency.
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

fn signal(net: String, pin: Option<String>) -> Signal {
    Signal { net, pin }
}

/// A pin that carries `prefix<n>_<sig>` for some n, or any pin if none.
fn pin_for(rng: &mut Rng, table: &PinTable, prefix: &str, sig: &str) -> String {
    let pins: Vec<&String> = table
        .pins
        .iter()
        .filter(|(_, p)| {
            p.alternate_functions
                .keys()
                .any(|s| s.starts_with(prefix) && s.ends_with(&format!("_{sig}")))
        })
        .map(|(k, _)| k)
        .collect();
    pins[rng.below(pins.len())].clone()
}

fn spec(rng: &mut Rng, table: &PinTable, hand: u64) -> Specification {
    let any_pins: Vec<&String> = table.pins.keys().collect();
    let hand_pin = |rng: &mut Rng, prefix: &str, sig: &str| {
        rng.chance(hand).then(|| pin_for(rng, table, prefix, sig))
    };
    let i2c_buses: Vec<I2cBus> = (0..rng.below(6))
        .map(|i| I2cBus {
            id: format!("I2C_BUS_{i}"),
            peripheral: rng
                .chance(hand / 2)
                .then(|| format!("I2C{}", 1 + rng.below(4))),
            speed_hz: 400_000,
            pull_up_ohms: 2200,
            domain: "VDD_3V3".into(),
            sda: signal(format!("I2C{i}_SDA"), hand_pin(rng, "I2C", "SDA")),
            scl: signal(format!("I2C{i}_SCL"), hand_pin(rng, "I2C", "SCL")),
        })
        .collect();
    let spi_buses: Vec<SpiBus> = (0..rng.below(8))
        .map(|i| SpiBus {
            id: format!("SPI_BUS_{i}"),
            peripheral: rng
                .chance(hand / 2)
                .then(|| format!("SPI{}", 1 + rng.below(6))),
            speed_hz: 20_000_000,
            domain: "VDD_3V3".into(),
            sck: signal(format!("SPI{i}_SCK"), hand_pin(rng, "SPI", "SCK")),
            mosi: signal(format!("SPI{i}_MOSI"), hand_pin(rng, "SPI", "MOSI")),
            miso: signal(format!("SPI{i}_MISO"), hand_pin(rng, "SPI", "MISO")),
        })
        .collect();
    let mut spi_devices = Vec::new();
    for bus in &spi_buses {
        for _ in 0..rng.below(3) {
            let n = spi_devices.len();
            spi_devices.push(SpiDevice {
                reference: format!("U{}", 200 + n),
                part: "W25Q64JV".into(),
                supply: "VDD_3V3".into(),
                bus: bus.id.clone(),
                cs: signal(
                    format!("CS{n}"),
                    rng.chance(hand)
                        .then(|| any_pins[rng.below(any_pins.len())].clone()),
                ),
            });
        }
    }
    Specification {
        schema_version: 1,
        project_name: "fuzz".into(),
        mcu: Mcu {
            part: "STM32H753ZITx".into(),
            supply: "VDD_3V3".into(),
        },
        voltage_domains: vec![VoltageDomain {
            id: "VDD_3V3".into(),
            nominal_millivolts: 3300,
            tolerance_permille: 50,
        }],
        i2c_buses,
        spi_buses,
        i2c_devices: vec![],
        spi_devices,
        net_constraints: vec![],
    }
}

/// Independent of the search: every rule of S1 checked on the output.
fn verify(spec: &Specification, table: &PinTable, topo: &SpikeTopology) -> Result<(), String> {
    let mut pins = BTreeSet::new();
    let mut instances = BTreeSet::new();
    let mut check_bus = |kind: &str,
                         id: &str,
                         peripheral: &Option<String>,
                         sigs: &[(&str, &Signal)]|
     -> Result<(), String> {
        let mut inst = None;
        for (sig, s) in sigs {
            let a = topo
                .assignments
                .get(&s.net)
                .ok_or(format!("{} unassigned", s.net))?;
            if !a.peripheral.starts_with(kind) {
                return Err(format!("{id}: {} is not {kind}", a.peripheral));
            }
            if *inst.get_or_insert(a.peripheral.clone()) != a.peripheral {
                return Err(format!("{id}: two instances"));
            }
            if a.signal != format!("{}_{sig}", a.peripheral) {
                return Err(format!("{}: signal {}", s.net, a.signal));
            }
            let af = table
                .pins
                .get(&a.pin)
                .and_then(|p| p.alternate_functions.get(&a.signal));
            if af != Some(&a.alternate_function) {
                return Err(format!(
                    "{}: {} AF{} not in table",
                    s.net, a.pin, a.alternate_function
                ));
            }
            if s.pin.as_ref().is_some_and(|h| *h != a.pin) {
                return Err(format!("{}: hand pin moved", s.net));
            }
            if RESERVED.contains(&a.pin.as_str()) || !pins.insert(a.pin.clone()) {
                return Err(format!("{}: pin {} reserved or reused", s.net, a.pin));
            }
        }
        let inst = inst.expect("a bus has signals");
        if peripheral.as_ref().is_some_and(|p| *p != inst) {
            return Err(format!("{id}: hand peripheral moved"));
        }
        if !instances.insert(inst.clone()) {
            return Err(format!("{id}: instance {inst} reused"));
        }
        Ok(())
    };
    for b in &spec.i2c_buses {
        check_bus(
            "I2C",
            &b.id,
            &b.peripheral,
            &[("SDA", &b.sda), ("SCL", &b.scl)],
        )?;
    }
    for b in &spec.spi_buses {
        let sigs = [("SCK", &b.sck), ("MOSI", &b.mosi), ("MISO", &b.miso)];
        check_bus("SPI", &b.id, &b.peripheral, &sigs)?;
    }
    for d in &spec.spi_devices {
        let pin = topo
            .gpio
            .get(&d.cs.net)
            .ok_or(format!("{} unassigned", d.cs.net))?;
        if !table.pins.contains_key(pin) || d.cs.pin.as_ref().is_some_and(|h| h != pin) {
            return Err(format!("{}: cs pin {pin}", d.cs.net));
        }
        if RESERVED.contains(&pin.as_str()) || !pins.insert(pin.clone()) {
            return Err(format!("{}: cs pin {pin} reserved or reused", d.cs.net));
        }
    }
    let nets = spec.i2c_buses.len() * 2 + spec.spi_buses.len() * 3;
    if topo.assignments.len() != nets || topo.gpio.len() != spec.spi_devices.len() {
        return Err("extra assignments".into());
    }
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args.next().and_then(|n| n.parse().ok()).expect("seed");
    let count: usize = args.next().and_then(|n| n.parse().ok()).expect("count");
    let limit = 2_000_000;
    let settings: Vec<Pruning> = [(false, false), (true, false), (false, true), (true, true)]
        .into_iter()
        .map(|(im, pm)| Pruning {
            instance_matching: im,
            pin_matching: pm,
            memo: false,
            node_limit: limit,
        })
        .collect();
    let table = aethelgard_svd_hal::pin_table("STM32H753ZITx").expect("table");
    let mut rng = Rng(seed.max(1));
    let mut worst: Vec<(u64, f64, String)> = vec![(0, 0.0, String::new()); settings.len()];
    let (mut ok, mut infeasible, mut pre, mut budget, mut bad) = (0, 0, 0, 0, 0);
    for case in 0..count {
        let hand = [0, 15, 30, 50][case % 4];
        let s = spec(&mut rng, &table, hand);
        if s.validate().is_err() {
            continue;
        }
        let bytes = serde_json::to_vec_pretty(&s).expect("serialize");
        let mut answers = Vec::new();
        for (i, &p) in settings.iter().enumerate() {
            let mut stats = Stats::default();
            let t = Instant::now();
            let r = solve(&bytes, &table, p, &mut stats);
            let secs = t.elapsed().as_secs_f64();
            if stats.nodes > worst[i].0 {
                worst[i] = (
                    stats.nodes,
                    secs,
                    String::from_utf8(bytes.clone()).expect("utf8"),
                );
            }
            if let Ok(topo) = &r
                && let Err(e) = verify(&s, &table, topo)
            {
                bad += 1;
                println!("VERIFY FAILED case {case} {}: {e}", p.label());
            }
            answers.push(r);
        }
        let done: Vec<&Result<SpikeTopology, SolveError>> = answers
            .iter()
            .filter(|r| !matches!(r, Err(SolveError::BudgetExceeded { .. })))
            .collect();
        if done.windows(2).any(|w| w[0] != w[1]) {
            bad += 1;
            println!("SETTINGS DISAGREE case {case}");
        }
        match &answers[3] {
            Ok(_) => ok += 1,
            Err(SolveError::Infeasible { .. }) => infeasible += 1,
            Err(SolveError::BudgetExceeded { .. }) => budget += 1,
            Err(_) => pre += 1,
        }
    }
    println!(
        "seed={seed} count={count} limit={limit}: fc+im+pm solved={ok} infeasible={infeasible} prechecked={pre} budget={budget}; disagreements_or_verify_failures={bad}"
    );
    for (p, (nodes, secs, _)) in settings.iter().zip(&worst) {
        println!("  worst {}: nodes={nodes} secs={secs:.4}", p.label());
    }
    if let Ok(path) = std::env::var("WORST") {
        std::fs::write(path, &worst[3].2).expect("write worst");
    }
}
