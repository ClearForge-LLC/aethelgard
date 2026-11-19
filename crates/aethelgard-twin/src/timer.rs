//! Motor outputs in the twin (`AET-WO-4300a`): the stock timer routes the
//! platform relies on, the observer each solved output gets, and the
//! model's conventions the witness is judged by (`4300` D3), each named with
//! its `STM32_Timer.cs` line (Renode 1.17.0, Infrastructure `066a7f13`).
//!
//! **The model's conventions**, never blended with silicon's (`4300` D3):
//! - every timer counts at the frequency the platform builds it at
//!   ([`platform::timer_hz`](crate::platform::timer_hz)): the solved kernel
//!   clock for the generated firmware, the declared one for a foreign
//!   firmware (`AET-WO-4400g`), never the stock file's 250 MHz;
//! - a period is realized in whole ns, rounded up
//!   ([`aethelgard_chaos::record::realized_ns`], `BaseClockSource.cs`
//!   L315–L325): at 64 MHz a DShot600 bit of 106 ticks runs 1,657 ns, where
//!   silicon runs 1,656.25. The silicon check judges the exact period, so
//!   this rounding never reaches a silicon verdict;
//! - a period is `ARR` ticks, where silicon counts `ARR + 1`
//!   (`STM32_Timer.cs` L62, `Limit = autoReloadValue`);
//! - each output rises at the update event (L50–78, PWM mode 1 `Set`) and
//!   falls `CCR` ticks later (L105–125, the channel's compare timer, `Unset`);
//! - the first rise is one period after `CEN` (L143–152: the counter starts
//!   at 0 and the first update is at the limit), so the judge anchors at the
//!   first rise and predicts every later edge from it;
//! - `MOE` (L385) and `OCxPE` (L437) are tags: they neither gate nor
//!   preload, so they cannot be read back.
//!
//! The tables below are read from [`STOCK_FILE`](crate::platform::STOCK_FILE) at the hash in
//! [`RENODE_FILES`](crate::platform::RENODE_FILES); the tier-1 test
//! `tests/timer_routes.rs` parses the vendored copy and asserts each equal.

use aethelgard_ir::SolvedTopology;
use aethelgard_ir::motor::{self, MotorProtocol};

use crate::{TwinError, split_pin};

/// The name of a solved output's observer, `pa8obs`; it logs under
/// `gpioPortA.pa8obs`. `aethelgard_ir::is_observer_name` refuses it as a
/// device reference, since the description is one namespace.
pub fn observer_name(port: char, index: u32) -> String {
    let name = format!("p{}{index}obs", port.to_ascii_lowercase());
    debug_assert!(aethelgard_ir::is_observer_name(&name));
    name
}

/// One solved motor output as the twin observes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observer {
    /// The output's label, its net.
    pub label: String,
    pub protocol: MotorProtocol,
    pub timer: u32,
    pub channel: u32,
    /// The pin, `PA8`, and its port and index.
    pub pin: String,
    pub port: char,
    pub index: u32,
    pub alternate_function: u8,
    /// The observer's name in the description, `pa8obs`, and its log
    /// source, `gpioPortA.pa8obs`.
    pub name: String,
    pub source: String,
    /// The frequency the platform builds the output's timer at, in Hz.
    pub kernel_hz: u64,
    /// The model's period, `ARR · (PSC + 1)` cycles of `kernel_hz`, as
    /// Renode realizes it, in ns.
    pub period_ns: u64,
    /// The model's high time, `CCR · (PSC + 1)` cycles, likewise.
    pub high_ns: u64,
}

/// The observer of every motor output of `topology`, in the specification's
/// order. A solved channel and pin with no stock route, or one the port
/// inverts, is refused by name: its output would never reach the pin, or
/// reach it inverted, and the witness would fail for a reason that is the
/// platform's, not the firmware's.
pub fn observers(topology: &SolvedTopology) -> Result<Vec<Observer>, TwinError> {
    topology.validate().map_err(TwinError::InvalidTopology)?;
    let mut out = Vec::new();
    for (label, m, a) in topology.motor_outputs_in_order() {
        let (timer, channel) =
            motor::timer_channel(&a.signal).ok_or_else(|| TwinError::UnsupportedPeripheral {
                peripheral: a.peripheral.clone(),
            })?;
        let (port, index) = split_pin(&a.pin)?;
        let af = a.alternate_function;
        if !TIMER_ROUTES.contains(&(timer, channel, port, index, af)) {
            return Err(TwinError::NoTimerRoute {
                label: label.to_owned(),
                signal: a.signal.clone(),
                pin: a.pin.clone(),
                alternate_function: af,
            });
        }
        if INVERTED_AF.contains(&(port, index, af)) {
            return Err(TwinError::InvertedRoute {
                label: label.to_owned(),
                pin: a.pin.clone(),
                alternate_function: af,
            });
        }
        let cycles = u64::from(m.psc) + 1;
        let kernel_hz = crate::platform::timer_hz(topology, timer);
        // `validate` above refuses a zero clock, and 16-bit `ARR`, `CCR` and
        // `PSC` keep every period far inside `u64` ns.
        let realized = |ticks: u32| {
            aethelgard_chaos::record::realized_ns(u64::from(ticks) * cycles, kernel_hz)
                .expect("a validated clock realizes every 16-bit period")
        };
        let name = observer_name(port, index);
        out.push(Observer {
            label: label.to_owned(),
            protocol: m.protocol,
            timer,
            channel,
            pin: a.pin.clone(),
            port,
            index,
            alternate_function: af,
            source: format!("gpioPort{port}.{name}"),
            name,
            kernel_hz,
            period_ns: realized(m.arr),
            high_ns: realized(m.ccr),
        });
    }
    Ok(out)
}

/// The stock input route of `port` pin `index`, whole (`timer1@00`).
pub fn stock_input(port: char, index: u32) -> Option<&'static str> {
    GPIO_TIMER_INPUTS
        .iter()
        .find(|(p, n, _)| *p == port && *n == index)
        .map(|(_, _, route)| *route)
}

/// Every `Timers.STM32_Timer` of [`STOCK_FILE`](crate::platform::STOCK_FILE), as (number, `sysbus` address).
pub const STOCK_TIMERS: [(u32, u32); 14] = [
    (1, 0x4001_0000),
    (2, 0x4000_0000),
    (3, 0x4000_0400),
    (4, 0x4000_0800),
    (5, 0x4000_0c00),
    (6, 0x4000_1000),
    (7, 0x4000_1400),
    (8, 0x4001_0400),
    (12, 0x4000_1800),
    (13, 0x4000_1c00),
    (14, 0x4000_2000),
    (15, 0x4001_4000),
    (16, 0x4001_4400),
    (17, 0x4001_4800),
];
/// Every timer channel's output route of [`STOCK_FILE`](crate::platform::STOCK_FILE) (`timerN: k -> gpioPortX#pin@af`),
/// as (timer, channel from 1, port, pin, alternate function). Renode's
/// channel `k` drives `TIMn_CH(k+1)` and `TIMn_CH(k+1)N` pins alike.
pub const TIMER_ROUTES: [(u32, u32, char, u32, u8); 102] = [
    (1, 1, 'A', 7, 1),
    (1, 1, 'A', 8, 1),
    (1, 1, 'B', 13, 1),
    (1, 1, 'E', 8, 1),
    (1, 1, 'E', 9, 1),
    (1, 1, 'K', 0, 1),
    (1, 1, 'K', 1, 1),
    (1, 2, 'A', 9, 1),
    (1, 2, 'B', 0, 1),
    (1, 2, 'B', 14, 1),
    (1, 2, 'E', 10, 1),
    (1, 2, 'E', 11, 1),
    (1, 2, 'J', 10, 1),
    (1, 2, 'J', 11, 1),
    (1, 3, 'A', 10, 1),
    (1, 3, 'B', 1, 1),
    (1, 3, 'B', 15, 1),
    (1, 3, 'E', 12, 1),
    (1, 3, 'E', 13, 1),
    (1, 3, 'J', 8, 1),
    (1, 3, 'J', 9, 1),
    (1, 4, 'A', 11, 1),
    (1, 4, 'E', 14, 1),
    (2, 1, 'A', 0, 1),
    (2, 1, 'A', 5, 1),
    (2, 1, 'A', 15, 1),
    (2, 2, 'A', 1, 1),
    (2, 2, 'B', 3, 1),
    (2, 3, 'A', 2, 1),
    (2, 3, 'B', 10, 1),
    (2, 4, 'A', 3, 1),
    (2, 4, 'B', 11, 1),
    (3, 1, 'A', 6, 2),
    (3, 1, 'B', 4, 2),
    (3, 1, 'C', 6, 2),
    (3, 2, 'A', 7, 2),
    (3, 2, 'B', 5, 2),
    (3, 2, 'C', 7, 2),
    (3, 3, 'B', 0, 2),
    (3, 3, 'C', 8, 2),
    (3, 4, 'B', 1, 2),
    (3, 4, 'C', 9, 2),
    (4, 1, 'B', 6, 2),
    (4, 1, 'D', 12, 2),
    (4, 2, 'B', 7, 2),
    (4, 2, 'D', 13, 2),
    (4, 3, 'B', 8, 2),
    (4, 3, 'D', 14, 2),
    (4, 4, 'B', 9, 2),
    (4, 4, 'D', 15, 2),
    (5, 1, 'A', 0, 2),
    (5, 1, 'H', 10, 2),
    (5, 2, 'A', 1, 2),
    (5, 2, 'H', 11, 2),
    (5, 3, 'A', 2, 2),
    (5, 3, 'H', 12, 2),
    (5, 4, 'A', 3, 2),
    (5, 4, 'I', 0, 2),
    (8, 1, 'A', 5, 3),
    (8, 1, 'A', 7, 3),
    (8, 1, 'C', 6, 3),
    (8, 1, 'I', 5, 3),
    (8, 1, 'J', 8, 3),
    (8, 1, 'J', 9, 3),
    (8, 2, 'B', 0, 3),
    (8, 2, 'B', 14, 3),
    (8, 2, 'C', 7, 3),
    (8, 2, 'I', 6, 3),
    (8, 2, 'J', 6, 3),
    (8, 2, 'J', 7, 3),
    (8, 2, 'J', 10, 3),
    (8, 2, 'J', 11, 3),
    (8, 3, 'B', 1, 3),
    (8, 3, 'B', 15, 3),
    (8, 3, 'C', 8, 3),
    (8, 3, 'I', 7, 3),
    (8, 3, 'K', 0, 3),
    (8, 3, 'K', 1, 3),
    (8, 4, 'C', 9, 3),
    (8, 4, 'I', 2, 3),
    (12, 1, 'B', 14, 2),
    (12, 1, 'H', 6, 2),
    (12, 2, 'B', 15, 2),
    (12, 2, 'H', 9, 2),
    (13, 1, 'A', 6, 9),
    (13, 1, 'F', 8, 9),
    (14, 1, 'A', 7, 9),
    (14, 1, 'F', 9, 9),
    (15, 1, 'A', 1, 4),
    (15, 1, 'A', 2, 4),
    (15, 1, 'E', 4, 4),
    (15, 1, 'E', 5, 4),
    (15, 2, 'A', 3, 4),
    (15, 2, 'E', 6, 4),
    (16, 1, 'B', 6, 1),
    (16, 1, 'B', 8, 1),
    (16, 1, 'F', 6, 1),
    (16, 1, 'F', 8, 1),
    (17, 1, 'B', 7, 1),
    (17, 1, 'B', 9, 1),
    (17, 1, 'F', 7, 1),
    (17, 1, 'F', 9, 1),
];
/// Each pin's stock input route to the timers (`gpioPortX: pin -> timer…`),
/// whole: the emitter restates it with the observer added, because a later
/// route of the same pin replaces an earlier one (`4300` spike: the `|` form
/// keeps `8 -> timer1@00`). Every pin of [`TIMER_ROUTES`] has one.
pub const GPIO_TIMER_INPUTS: [(char, u32, &str); 66] = [
    ('A', 0, "timer2@00 | timer5@00"),
    ('A', 1, "timer2@01 | timer5@01 | timer15@00"),
    ('A', 2, "timer2@02 | timer5@02 | timer15@00"),
    ('A', 3, "timer2@03 | timer5@03 | timer15@01"),
    ('A', 5, "timer2@00 | timer8@00"),
    ('A', 6, "timer3@00 | timer13@00"),
    ('A', 7, "timer1@00 | timer3@01 | timer8@00 | timer14@00"),
    ('A', 8, "timer1@00"),
    ('A', 9, "timer1@01"),
    ('A', 10, "timer1@02"),
    ('A', 11, "timer1@03"),
    ('A', 15, "timer2@00"),
    ('B', 0, "timer1@01 | timer3@02 | timer8@01"),
    ('B', 1, "timer1@02 | timer3@03 | timer8@02"),
    ('B', 3, "timer2@01"),
    ('B', 4, "timer3@00"),
    ('B', 5, "timer3@01"),
    ('B', 6, "timer4@00 | timer16@00"),
    ('B', 7, "timer4@01 | timer17@00"),
    ('B', 8, "timer4@02 | timer16@00"),
    ('B', 9, "timer4@03 | timer17@00"),
    ('B', 10, "timer2@02"),
    ('B', 11, "timer2@03"),
    ('B', 13, "timer1@00"),
    ('B', 14, "timer1@01 | timer8@01 | timer12@00"),
    ('B', 15, "timer1@02 | timer8@02 | timer12@01"),
    ('C', 6, "timer3@00 | timer8@00"),
    ('C', 7, "timer3@01 | timer8@01"),
    ('C', 8, "timer3@02 | timer8@02"),
    ('C', 9, "timer3@03 | timer8@03"),
    ('D', 12, "timer4@00"),
    ('D', 13, "timer4@01"),
    ('D', 14, "timer4@02"),
    ('D', 15, "timer4@03"),
    ('E', 4, "timer15@00"),
    ('E', 5, "timer15@00"),
    ('E', 6, "timer15@01"),
    ('E', 8, "timer1@00"),
    ('E', 9, "timer1@00"),
    ('E', 10, "timer1@01"),
    ('E', 11, "timer1@01"),
    ('E', 12, "timer1@02"),
    ('E', 13, "timer1@02"),
    ('E', 14, "timer1@03"),
    ('F', 6, "timer16@00"),
    ('F', 7, "timer17@00"),
    ('F', 8, "timer13@00 | timer16@00"),
    ('F', 9, "timer14@00 | timer17@00"),
    ('H', 6, "timer12@00"),
    ('H', 9, "timer12@01"),
    ('H', 10, "timer5@00"),
    ('H', 11, "timer5@01"),
    ('H', 12, "timer5@02"),
    ('I', 0, "timer5@03"),
    ('I', 2, "timer8@03"),
    ('I', 5, "timer8@00"),
    ('I', 6, "timer8@01"),
    ('I', 7, "timer8@02"),
    ('J', 6, "timer8@01"),
    ('J', 7, "timer8@01"),
    ('J', 8, "timer1@02 | timer8@00"),
    ('J', 9, "timer1@02 | timer8@00"),
    ('J', 10, "timer1@01 | timer8@01"),
    ('J', 11, "timer1@01 | timer8@01"),
    ('K', 0, "timer1@00 | timer8@02"),
    ('K', 1, "timer1@00 | timer8@02"),
];
/// Each `invertedAFPins` entry of [`STOCK_FILE`](crate::platform::STOCK_FILE) as (port, pin, alternate
/// function): the port inverts what that function drives (a `CHkN` route).
pub const INVERTED_AF: [(char, u32, u8); 30] = [
    ('A', 5, 3),
    ('A', 7, 1),
    ('A', 7, 3),
    ('B', 0, 1),
    ('B', 0, 3),
    ('B', 1, 1),
    ('B', 1, 3),
    ('B', 6, 1),
    ('B', 7, 1),
    ('B', 13, 1),
    ('B', 14, 1),
    ('B', 14, 3),
    ('B', 15, 1),
    ('B', 15, 3),
    ('E', 4, 4),
    ('E', 8, 1),
    ('E', 10, 1),
    ('E', 12, 1),
    ('F', 8, 1),
    ('F', 9, 1),
    ('H', 13, 3),
    ('H', 14, 3),
    ('H', 15, 3),
    ('J', 7, 3),
    ('J', 8, 1),
    ('J', 9, 3),
    ('J', 10, 1),
    ('J', 11, 3),
    ('K', 0, 1),
    ('K', 1, 3),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_observer_name_is_the_pin_s() {
        assert_eq!(observer_name('A', 8), "pa8obs");
        assert_eq!(observer_name('K', 15), "pk15obs");
        assert_eq!(stock_input('A', 8), Some("timer1@00"));
        assert_eq!(
            stock_input('A', 7),
            Some("timer1@00 | timer3@01 | timer8@00 | timer14@00")
        );
        assert_eq!(stock_input('A', 4), None);
    }

    /// At the 64 MHz reset clock (`AET-WO-4400g`): the probes' `pwm400`
    /// (`PSC` 63, `ARR` 2499, `CCR` 1000) is a 1 µs tick, exact; DShot600
    /// (`PSC` 0, `ARR` 106, `CCR` 40) is 1,657 ns realized, not the
    /// nominal 1,656.25, and its high time 625 ns, exact.
    #[test]
    fn the_model_s_periods_are_realized_at_64_mhz() {
        let r = |ticks| aethelgard_chaos::record::realized_ns(ticks, 64_000_000);
        assert_eq!(r(2499 * 64), Some(2_499_000));
        assert_eq!(r(1000 * 64), Some(1_000_000));
        assert_eq!(r(106), Some(1_657));
        assert_eq!(r(40), Some(625));
        assert_eq!(aethelgard_chaos::record::realized_ns(1, 0), None);
    }
}
