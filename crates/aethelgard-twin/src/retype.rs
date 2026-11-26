//! A DShot output's timer and the DMA that feeds it, as Aethelgard's own
//! models (`AET-WO-4412` D4, ruled 2026-11-26T14:09Z), on a foreign
//! platform only.
//!
//! Renode 1.17.0's `STM32_Timer` fires its limit handler and re-arms four
//! channel timers at every update (≈25 s of wall per virtual second at
//! DShot600's bit rate), and its `STM32DMA` moves a memory-to-peripheral
//! stream whole at `EN`, into a `DMAR` the timer does not model, so no DShot
//! bit ever left the twin (`spike/4412/RESULTS.md` §1). The models pace the
//! stream by the timer's update request (RM0433 Rev 9 Table 124, p. 697) and
//! burst it into `CCRx` by `DCR`/`DMAR` (pp. 1586, 1615–1617).
//!
//! **Where:** only in a foreign platform (a topology with `foreign_clock`),
//! and only for the timers of its DShot outputs: the generated firmware
//! never enables timer DMA, and a generated platform's bytes, and so every
//! generated verdict and package, stay as they were (INV-08).
//!
//! **How:** cancel-and-restate (Tviburi's `RUL-07` form, its `RUL-09` (b)
//! trap held): each stock entry is cancelled (`timerN: @none`) and the model
//! declared at its address under its name (`as "timerN"`), with the stock
//! entry's registration, limit, interrupt lines and alternate-function
//! routes restated on the model's own variable, every GPIO route into the
//! timer and its frequency too, and `dmamux1`'s outputs repointed onto the
//! DMA models: no untyped update is left on a cancelled name (a tier-1 test).

use std::collections::BTreeSet;
use std::fmt::Write as _;

use aethelgard_ir::SolvedTopology;

use crate::TwinError;

/// The timer model's Renode type (`models/renode/AethelgardSTM32Timer.cs`).
pub const TIMER_MODEL: &str = "Timers.AethelgardSTM32Timer";
/// The DMA model's (`models/renode/AethelgardSTM32DMA.cs`).
pub const DMA_MODEL: &str = "DMA.AethelgardSTM32DMA";

/// Each timer with an update DMA request, as Renode 1.17.0's
/// `platforms/cpus/stm32h7.repl` declares it: its `initialLimit` and its
/// interrupt lines, word for word.
pub const STOCK_DECLARATIONS: [(u32, &str, &str); 9] = [
    (
        1,
        "0xffff",
        "BreakInterrupt -> nvic@24\n    UpdateInterrupt -> nvic@25\n    [TriggerInterrupt, CommutationInterrupt] -> nvic@[26, 26]\n    CaptureCompareInterrupt -> nvic@27",
    ),
    (2, "0xffffffff", "-> nvic@28"),
    (3, "0xffff", "-> nvic@29"),
    (4, "0xffff", "-> nvic@30"),
    (5, "0xffffffff", "-> nvic@50"),
    (
        8,
        "0xffff",
        "BreakInterrupt -> nvic@43\n    UpdateInterrupt -> nvic@44\n    [TriggerInterrupt, CommutationInterrupt] -> nvic@[45, 45]\n    CaptureCompareInterrupt -> nvic@46",
    ),
    (15, "0xffff", "-> nvic@116"),
    (16, "0xffff", "-> nvic@117"),
    (17, "0xffff", "-> nvic@118"),
];

/// The stock DMA entries, word for word: (name, address, alias, interrupts,
/// the first DMAMUX1 channel it serves).
pub const STOCK_DMAS: [(&str, u32, &str, &str, u32); 2] = [
    (
        "dma2",
        0x4002_0400,
        "DMA2",
        "[0-7] -> nvic@[56-60, 68-70]",
        8,
    ),
    ("dma1", 0x4002_0000, "DMA1", "[0-7] -> nvic@[11-17, 47]", 0),
];

/// The model's variable for timer `n`: `aethelgardTimer1`.
pub fn timer_var(n: u32) -> String {
    format!("aethelgardTimer{n}")
}

/// The timers the foreign platform of `topology` takes as models: those of
/// its DShot outputs, in order; none on a generated platform.
pub fn retyped(topology: &SolvedTopology) -> Vec<u32> {
    if topology.foreign_clock.is_none() {
        return Vec::new();
    }
    let set: BTreeSet<u32> = topology
        .motor_outputs_in_order()
        .into_iter()
        .filter(|(_, m, _)| m.protocol.bit_rate().is_some())
        .filter_map(|(_, _, a)| aethelgard_ir::motor::timer_channel(&a.signal))
        .map(|(n, _)| n)
        .collect();
    set.into_iter().collect()
}

/// A GPIO route (`timer1@00 | timer3@01`) with every retyped timer's target
/// on its model's variable.
pub fn route(route: &str, retyped: &[u32]) -> String {
    route
        .split(" | ")
        .map(|t| {
            let renamed = t.split_once('@').and_then(|(name, at)| {
                let n: u32 = name.strip_prefix("timer")?.parse().ok()?;
                retyped
                    .contains(&n)
                    .then(|| format!("{}@{at}", timer_var(n)))
            });
            renamed.unwrap_or_else(|| t.to_owned())
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

/// The block that declares the models for `retyped` at `hz` (the declared
/// clock of each timer's bus), skipping the GPIO input routes of `observed`
/// pins, which the observers restate themselves.
pub fn emit(
    out: &mut String,
    topology: &SolvedTopology,
    retyped: &[u32],
    observed: &BTreeSet<(char, u32)>,
) -> Result<(), TwinError> {
    if retyped.is_empty() {
        return Ok(());
    }
    let facts = aethelgard_svd_hal::cmsis::register_facts()?;
    let mut requests = Vec::new();
    let _ = write!(
        out,
        "\n// AET-WO-4412 D4: the DShot outputs' timers and the DMA that feeds them are Aethelgard's own models,\n\
         // by cancel-and-restate: each stock entry cancelled, the model declared at its address under its name.\n"
    );
    for &n in retyped {
        let request = aethelgard_ir::motor::up_request(n).ok_or_else(|| {
            TwinError::UnsupportedPeripheral {
                peripheral: format!("TIM{n} (no update DMA request, RM0433 Rev 9 Table 124)"),
            }
        })?;
        let (_, limit, irqs) = STOCK_DECLARATIONS
            .iter()
            .find(|d| d.0 == n)
            .ok_or_else(|| TwinError::UnsupportedPeripheral {
                peripheral: format!("TIM{n}"),
            })?;
        requests.push(request);
        let base = facts.base(&format!("TIM{n}"))?;
        let hz = crate::platform::timer_hz(topology, n);
        let _ = write!(
            out,
            "timer{n}: @none\n\
             {var}: {TIMER_MODEL} @ {{\n        sysbus 0x{base:08X};\n        rcc RegistrationPeripheralAlias.TIM{n}\n    }} as \"timer{n}\"\n    \
             UpdateDmaRequest -> dmamux1@{request}\n    initialLimit: {limit}\n    frequency: {hz}\n    {irqs}\n",
            var = timer_var(n),
        );
        // The stock alternate-function mapping (`timerN: k -> gpioPortX#p@af`).
        let _ = writeln!(out, "{}:", timer_var(n));
        for channel in 1..=4u32 {
            let pins: Vec<String> = crate::timer::TIMER_ROUTES
                .iter()
                .filter(|r| r.0 == n && r.1 == channel)
                .map(|r| format!("gpioPort{}#{:02}@{:02}", r.2, r.3, r.4))
                .collect();
            if !pins.is_empty() {
                let _ = writeln!(out, "    {} -> {}", channel - 1, pins.join(" | "));
            }
        }
    }
    let paced = requests
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    for (name, base, alias, irqs, first) in STOCK_DMAS {
        let _ = write!(
            out,
            "{name}: @none\n\
             aethelgard{cap}: {DMA_MODEL} @ {{\n        sysbus 0x{base:08X};\n        rcc RegistrationPeripheralAlias.{alias}\n    }} as \"{name}\"\n    \
             dmamux: dmamux1\n    firstMuxChannel: {first}\n    pacedRequests: \"{paced}\"\n    {irqs}\n",
            cap = format!("Dma{}", &name[3..]),
        );
    }
    out.push_str(
        "dmamux1:\n    [0-7] -> aethelgardDma1@[0-7]\n    [8-15] -> aethelgardDma2@[0-7]\n",
    );
    // Every other stock GPIO route into a retyped timer, restated whole.
    let mut by_port: std::collections::BTreeMap<char, Vec<String>> = Default::default();
    for (port, pin, stock) in crate::timer::GPIO_TIMER_INPUTS {
        let names_one = stock.split(" | ").any(|t| {
            t.split_once('@')
                .and_then(|(name, _)| name.strip_prefix("timer")?.parse::<u32>().ok())
                .is_some_and(|n| retyped.contains(&n))
        });
        if names_one && !observed.contains(&(port, pin)) {
            by_port
                .entry(port)
                .or_default()
                .push(format!("    {pin} -> {}", route(stock, retyped)));
        }
    }
    for (port, lines) in by_port {
        let _ = writeln!(out, "gpioPort{port}:\n{}", lines.join("\n"));
    }
    Ok(())
}
