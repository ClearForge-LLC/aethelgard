//! Motor outputs (`AET-WO-4300a`): what a specification declares, the clock
//! the generated firmware runs on, the timer values the solver derives, and
//! the static check that judges those values for silicon (`4300` D3).
//!
//! A motor output is one timer channel on an alternate-function pin. Its net
//! is its label, and its pin, timer and channel are an entry of
//! `SolvedTopology::assignments` (`TIMn` / `TIMn_CHk`), so every generator
//! that draws or configures solved pins sees it. [`SolvedMotorOutput`] adds
//! the timing.
//!
//! Two clocks are never blended (`4300` D3). [`check_silicon`] judges `PSC`,
//! `ARR` and `CCR` against the declared intent on the clock the topology
//! states, with silicon's `ARR + 1` (RM0433 Rev 9 §38.3.2, "the counter
//! counts from 0 to the auto-reload value"). The twin's conventions (`ARR`
//! ticks, a fixed 250 MHz) are judged in `aethelgard-twin`, never here.

use std::fmt;

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The protocol a motor output speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MotorProtocol {
    /// Servo-style PWM at a declared rate and pulse range.
    Pwm,
    /// DShot at 300,000 bit/s.
    Dshot300,
    /// DShot at 600,000 bit/s.
    Dshot600,
}

impl MotorProtocol {
    /// The name the specification and the trace use.
    pub fn name(self) -> &'static str {
        match self {
            Self::Pwm => "pwm",
            Self::Dshot300 => "dshot300",
            Self::Dshot600 => "dshot600",
        }
    }

    /// A DShot protocol's bit rate, bit/s: ArduPilot `Copter-4.7.2`
    /// (`fb773567`), `libraries/AP_HAL_ChibiOS/RCOutput.cpp` L2521–2524.
    /// `None` for PWM.
    pub fn bit_rate(self) -> Option<u32> {
        match self {
            Self::Pwm => None,
            Self::Dshot300 => Some(300_000),
            Self::Dshot600 => Some(600_000),
        }
    }
}

/// A DShot bit is 8 ticks and a `0` bit is high for 3 of them (ArduPilot
/// `Copter-4.7.2`, `libraries/AP_HAL/RCOutput.h` L406–407,
/// `DSHOT_BIT_WIDTH_TICKS_DEFAULT`, `DSHOT_BIT_0_TICKS_DEFAULT`). The
/// bring-up firmware sends only `0` bits: a zero throttle (`AET-WO-4300a`
/// §1.3).
pub const DSHOT_BIT_TICKS: u64 = 8;
/// The high ticks of a DShot `0` bit, of [`DSHOT_BIT_TICKS`].
pub const DSHOT_ZERO_TICKS: u64 = 3;

/// A PWM output's declared timing: its rate, and the pulse range an ESC
/// reads, in microseconds. The firmware starts the output at the range's
/// minimum, its idle pulse (`AET-WO-4300a` §1.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct PwmTiming {
    pub rate_hz: u32,
    pub pulse_min_us: u32,
    pub pulse_max_us: u32,
}

/// One declared motor output. Every key is required (`INV-06`): `pin` is
/// `null` for the solver to choose, and `pwm` is `null` exactly when the
/// protocol is DShot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct MotorOutput {
    /// The output's name and its net's.
    pub label: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub pin: Option<String>,
    pub protocol: MotorProtocol,
    #[serde(deserialize_with = "Option::deserialize")]
    pub pwm: Option<PwmTiming>,
}

/// The clock tree the generated firmware runs on, as a solved fact
/// (`AET-WO-4300a` §1.2, ruled at `4400` D1). The bring-up firmware never
/// touches `RCC_CR`, `RCC_CFGR` or the PLLs, so it runs on the reset clock:
/// HSI at 64 MHz (`RCC_CR` reset `0x0000 0001`, `HSIDIV` `00`, RM0433 Rev 9
/// §8.7.2), selected (`RCC_CFGR` reset 0, `SW` = `SWS` = HSI, `TIMPRE` 0,
/// §8.7.7), every domain prescaler at its reset value 1 (`RCC_D1CFGR`,
/// `RCC_D2CFGR` and `RCC_D3CFGR` reset 0, "not divided", §8.7.8–§8.7.10).
/// `hse_hz` is the specification's `mcu.hse_hz`, which is 0 (no crystal).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedClock {
    /// `HSI`, the only source this engine builds.
    pub source: String,
    pub sysclk_hz: u32,
    pub hse_hz: u32,
    /// Each prescaler as its divisor (1 = not divided).
    pub d1cpre: u32,
    pub hpre: u32,
    pub d1ppre: u32,
    pub d2ppre1: u32,
    pub d2ppre2: u32,
    pub d3ppre: u32,
    /// `RCC_CFGR.TIMPRE`.
    pub timpre: bool,
}

impl SolvedClock {
    /// The reset clock: the only one this engine generates firmware for.
    pub fn reset() -> Self {
        Self {
            source: "HSI".into(),
            sysclk_hz: 64_000_000,
            hse_hz: 0,
            d1cpre: 1,
            hpre: 1,
            d1ppre: 1,
            d2ppre1: 1,
            d2ppre2: 1,
            d3ppre: 1,
            timpre: false,
        }
    }

    /// `rcc_hclk1`: `sys_ck` / `D1CPRE` / `HPRE` (RM0433 Rev 9 Figure 49).
    pub fn hclk_hz(&self) -> u64 {
        u64::from(self.sysclk_hz) / u64::from(self.d1cpre.max(1)) / u64::from(self.hpre.max(1))
    }

    /// The kernel clock of the timers on `apb`, by RM0433 Rev 9 Table 57:
    /// with `TIMPRE` 0, the bus clock when its prescaler is 1, else twice it.
    /// The reset clock gives 64 MHz on both buses. Only `TIMPRE` = 0 is
    /// modelled; [`SolvedTopology::validate`](crate::SolvedTopology::validate)
    /// refuses any clock but [`SolvedClock::reset`].
    pub fn timer_kernel_hz(&self, apb: Apb) -> u64 {
        let ppre = u64::from(match apb {
            Apb::Apb1 => self.d2ppre1,
            Apb::Apb2 => self.d2ppre2,
        });
        let pclk = self.hclk_hz() / ppre.max(1);
        if ppre <= 1 { pclk } else { 2 * pclk }
    }
}

/// The two APB buses of the D2 domain that carry timers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Apb {
    Apb1,
    Apb2,
}

/// The timers with capture/compare channels on the STM32H753, by number
/// (RM0433 Rev 9 §2.3.2 Table 8): TIM6 and TIM7 are basic timers with no
/// channel.
pub const TIMERS: [u32; 12] = [1, 2, 3, 4, 5, 8, 12, 13, 14, 15, 16, 17];

/// The bus a timer sits on (RM0433 Rev 9 §2.3.2 Table 8): TIM1, TIM8 and
/// TIM15–TIM17 on APB2, the rest on APB1.
pub fn apb(timer: u32) -> Apb {
    if matches!(timer, 1 | 8 | 15 | 16 | 17) {
        Apb::Apb2
    } else {
        Apb::Apb1
    }
}

/// Whether a timer has `BDTR` and its main output enable (RM0433 Rev 9
/// §38.4.20 for TIM1/TIM8, §41.5.16 for TIM15, §41.6.14 for TIM16/TIM17):
/// without `MOE` set, silicon drives none of its outputs.
pub fn has_main_output_enable(timer: u32) -> bool {
    matches!(timer, 1 | 8 | 15 | 16 | 17)
}

/// `TIMn_CHk` as `(n, k)`, with `n` in [`TIMERS`] and `k` 1 to 4; `None` for
/// any other signal, a complementary `TIMn_CHkN` included.
pub fn timer_channel(signal: &str) -> Option<(u32, u32)> {
    let rest = signal.strip_prefix("TIM")?;
    let (n, k) = rest.split_once("_CH")?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(n) || !digits(k) || n.starts_with('0') {
        return None;
    }
    let (n, k) = (n.parse().ok()?, k.parse().ok()?);
    (TIMERS.contains(&n) && (1..=4).contains(&k)).then_some((n, k))
}

/// One motor output as the generators and the judges need it: its place in
/// the specification, its declared protocol and PWM timing (copied by
/// `solve`, `RUL-16`), and the timer values solved for the topology's clock.
/// Its timer, channel, pin and alternate function are its net's entry of
/// `assignments`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedMotorOutput {
    /// Position in the specification's `motor_outputs`.
    pub index: u32,
    pub protocol: MotorProtocol,
    #[serde(deserialize_with = "Option::deserialize")]
    pub pwm: Option<PwmTiming>,
    /// `TIMx_PSC`, `TIMx_ARR` and `TIMx_CCRk` as the firmware writes them.
    pub psc: u32,
    pub arr: u32,
    pub ccr: u32,
}

crate::object_only!(PwmTiming, MotorOutput, SolvedClock, SolvedMotorOutput);

/// Why the solver could not derive a timer's values for an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingError {
    /// The kernel clock is not a whole number of MHz, so no prescaler gives
    /// a 1 µs tick.
    NoMicrosecondTick,
    /// The period does not fit a 16-bit `ARR` at the tick.
    RateOutOfRange,
    /// The pulse range's maximum does not fit inside the period.
    PulseExceedsPeriod,
}

/// `PSC`, `ARR` and `CCR` for one output on a kernel clock of `kernel_hz`.
///
/// - **PWM:** a 1 µs tick (`PSC = kernel / 1 MHz − 1`), the period the
///   nearest whole tick count to `1 s / rate` (half up), so `ARR` is that
///   less 1 (silicon counts `ARR + 1`), and `CCR` the idle pulse, the range's
///   minimum, in ticks.
/// - **DShot:** `PSC` 0, the bit the nearest whole tick count to
///   `kernel / bit rate`, and a `0` bit's high time the nearest whole count
///   to 3/8 of it.
pub fn solve_timing(
    protocol: MotorProtocol,
    pwm: Option<&PwmTiming>,
    kernel_hz: u64,
) -> Result<(u32, u32, u32), TimingError> {
    let half_up = |num: u64, den: u64| (2 * num + den) / (2 * den);
    let (psc, ticks, ccr) = match (protocol.bit_rate(), pwm) {
        (Some(bit_rate), _) => {
            let ticks = half_up(kernel_hz, u64::from(bit_rate));
            let ccr = half_up(ticks * DSHOT_ZERO_TICKS, DSHOT_BIT_TICKS);
            (0, ticks, ccr)
        }
        (None, Some(p)) => {
            if !kernel_hz.is_multiple_of(1_000_000) || kernel_hz == 0 {
                return Err(TimingError::NoMicrosecondTick);
            }
            let ticks = half_up(1_000_000, u64::from(p.rate_hz.max(1)));
            if u64::from(p.pulse_max_us) >= ticks {
                return Err(TimingError::PulseExceedsPeriod);
            }
            (kernel_hz / 1_000_000 - 1, ticks, u64::from(p.pulse_min_us))
        }
        (None, None) => return Err(TimingError::RateOutOfRange),
    };
    if !(2..=0x1_0000).contains(&ticks) || psc > 0xFFFF || ccr == 0 || ccr >= ticks {
        return Err(TimingError::RateOutOfRange);
    }
    let narrow = |v: u64| u32::try_from(v).expect("at most 0xFFFF");
    Ok((narrow(psc), narrow(ticks - 1), narrow(ccr)))
}

/// What [`check_silicon`] found for one output, in picoseconds where a time
/// is named (`period_ps` and `high_ps` are floors; the verdict is exact).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiliconTiming {
    pub kernel_hz: u64,
    /// `(PSC + 1)` kernel cycles.
    pub tick_ps: u64,
    /// `(ARR + 1)` ticks: what silicon counts.
    pub period_ps: u64,
    /// `CCR` ticks: the high time of PWM mode 1.
    pub high_ps: u64,
    /// The declared period: `1 s / rate` for PWM, `1 s / bit rate` for DShot.
    pub nominal_period_ps: u64,
    /// The declared high time: the idle pulse for PWM, 3/8 of a bit for
    /// DShot's `0`.
    pub nominal_high_ps: u64,
    /// Every rule held. The rules: the period within half a tick of the
    /// nominal; for PWM the high time exactly the idle pulse and the range's
    /// maximum strictly inside the period; for DShot the high time within
    /// half a tick of 3/8 of the nominal bit.
    pub holds: bool,
}

/// The static check for silicon (`4300` D3): `PSC`, `ARR` and `CCR` of
/// `output`, a channel of `TIM{timer}`, against its declared timing, on the
/// timer kernel clock `clock` gives, with silicon's `ARR + 1`. Exact integer
/// arithmetic; it never reads the generated firmware or board (`INV-07`).
pub fn check_silicon(clock: &SolvedClock, timer: u32, output: &SolvedMotorOutput) -> SiliconTiming {
    let kernel = u128::from(clock.timer_kernel_hz(apb(timer)));
    let (psc1, arr1, ccr) = (
        u128::from(output.psc) + 1,
        u128::from(output.arr) + 1,
        u128::from(output.ccr),
    );
    let ps = |cycles: u128| {
        u64::try_from(cycles * 1_000_000_000_000 / kernel.max(1)).unwrap_or(u64::MAX)
    };
    let tick_ps = ps(psc1);
    let period_ps = ps(arr1 * psc1);
    let high_ps = ps(ccr * psc1);
    let (holds, nominal_period_ps, nominal_high_ps) =
        match (output.protocol.bit_rate(), &output.pwm) {
            (Some(bit_rate), _) => {
                let rate = u128::from(bit_rate);
                // |period − 1/rate| ≤ tick/2, times 2·kernel·rate.
                let period_ok = (2 * arr1 * psc1 * rate).abs_diff(2 * kernel) <= psc1 * rate;
                // |high − 3/8 bit| ≤ tick/2, times 8·kernel·rate.
                let high_ok = (8 * ccr * psc1 * rate)
                    .abs_diff(u128::from(DSHOT_ZERO_TICKS) * kernel)
                    <= 4 * psc1 * rate;
                let nominal = 1_000_000_000_000 / rate;
                (
                    period_ok && high_ok,
                    u64::try_from(nominal).unwrap_or(u64::MAX),
                    u64::try_from(
                        nominal * u128::from(DSHOT_ZERO_TICKS) / u128::from(DSHOT_BIT_TICKS),
                    )
                    .unwrap_or(u64::MAX),
                )
            }
            (None, Some(p)) => {
                let rate = u128::from(p.rate_hz);
                let period_ok =
                    rate > 0 && (2 * arr1 * psc1 * rate).abs_diff(2 * kernel) <= psc1 * rate;
                // The idle pulse exactly: CCR·(PSC+1)/kernel = pulse_min µs.
                let high_ok = ccr * psc1 * 1_000_000 == u128::from(p.pulse_min_us) * kernel;
                // The longest pulse fits: pulse_max µs < (ARR+1)(PSC+1)/kernel.
                let max_ok = u128::from(p.pulse_max_us) * kernel < arr1 * psc1 * 1_000_000;
                (
                    period_ok && high_ok && max_ok,
                    u64::try_from(1_000_000_000_000 / rate.max(1)).unwrap_or(u64::MAX),
                    u64::from(p.pulse_min_us) * 1_000_000,
                )
            }
            (None, None) => (false, 0, 0),
        };
    SiliconTiming {
        kernel_hz: u64::try_from(kernel).unwrap_or(u64::MAX),
        tick_ps,
        period_ps,
        high_ps,
        nominal_period_ps,
        nominal_high_ps,
        holds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pwm(rate_hz: u32, lo: u32, hi: u32) -> PwmTiming {
        PwmTiming {
            rate_hz,
            pulse_min_us: lo,
            pulse_max_us: hi,
        }
    }

    fn solved(
        protocol: MotorProtocol,
        pwm: Option<PwmTiming>,
        (psc, arr, ccr): (u32, u32, u32),
    ) -> SolvedMotorOutput {
        SolvedMotorOutput {
            index: 0,
            protocol,
            pwm,
            psc,
            arr,
            ccr,
        }
    }

    #[test]
    fn the_reset_clock_gives_every_timer_64_mhz() {
        let c = SolvedClock::reset();
        assert_eq!(c.timer_kernel_hz(Apb::Apb1), 64_000_000);
        assert_eq!(c.timer_kernel_hz(Apb::Apb2), 64_000_000);
        // Table 57: a bus prescaler above 1 doubles the bus clock.
        let halved = SolvedClock {
            d2ppre2: 2,
            ..SolvedClock::reset()
        };
        assert_eq!(halved.timer_kernel_hz(Apb::Apb2), 64_000_000);
        assert_eq!(halved.timer_kernel_hz(Apb::Apb1), 64_000_000);
        let quartered = SolvedClock {
            d2ppre1: 4,
            ..SolvedClock::reset()
        };
        assert_eq!(quartered.timer_kernel_hz(Apb::Apb1), 32_000_000);
    }

    #[test]
    fn a_channel_signal_is_parsed_and_a_complementary_one_is_not() {
        assert_eq!(timer_channel("TIM1_CH1"), Some((1, 1)));
        assert_eq!(timer_channel("TIM17_CH1"), Some((17, 1)));
        for not in [
            "TIM1_CH1N",
            "TIM1_CH5",
            "TIM6_CH1",
            "TIM01_CH1",
            "HRTIM_CHA1",
            "LPTIM1_OUT",
            "TIM1_ETR",
            "TIM1_CH",
        ] {
            assert_eq!(timer_channel(not), None, "{not}");
        }
    }

    #[test]
    fn pwm_and_dshot_solve_to_the_values_the_probes_ran() {
        let k = 64_000_000;
        assert_eq!(
            solve_timing(MotorProtocol::Pwm, Some(&pwm(400, 1000, 2000)), k),
            Ok((63, 2499, 1000))
        );
        assert_eq!(
            solve_timing(MotorProtocol::Pwm, Some(&pwm(50, 1000, 2000)), k),
            Ok((63, 19999, 1000))
        );
        // 490 Hz is 2040.8 µs: the nearest whole tick count is 2041.
        assert_eq!(
            solve_timing(MotorProtocol::Pwm, Some(&pwm(490, 1000, 2000)), k),
            Ok((63, 2040, 1000))
        );
        assert_eq!(
            solve_timing(MotorProtocol::Dshot600, None, k),
            Ok((0, 106, 40))
        );
        assert_eq!(
            solve_timing(MotorProtocol::Dshot300, None, k),
            Ok((0, 212, 80))
        );
        // Below 16 Hz the period passes a 16-bit ARR at 1 µs.
        assert_eq!(
            solve_timing(MotorProtocol::Pwm, Some(&pwm(15, 1000, 2000)), k),
            Err(TimingError::RateOutOfRange)
        );
        assert_eq!(
            solve_timing(MotorProtocol::Pwm, Some(&pwm(16, 1000, 2000)), k),
            Ok((63, 62499, 1000))
        );
        // 490 Hz rounds a 2040.3 µs period down to 2040 ticks, which a
        // 2040 µs maximum would fill: refused, never a 100 % duty.
        assert_eq!(
            solve_timing(MotorProtocol::Pwm, Some(&pwm(490, 1000, 2041)), k),
            Err(TimingError::PulseExceedsPeriod)
        );
        assert_eq!(
            solve_timing(MotorProtocol::Pwm, Some(&pwm(400, 1000, 2000)), 64_500_000),
            Err(TimingError::NoMicrosecondTick)
        );
    }

    #[test]
    fn the_silicon_check_holds_for_solved_values_and_fails_one_tick_off() {
        let c = SolvedClock::reset();
        let p = pwm(400, 1000, 2000);
        let good = solved(MotorProtocol::Pwm, Some(p.clone()), (63, 2499, 1000));
        let t = check_silicon(&c, 1, &good);
        assert!(t.holds, "{t:?}");
        assert_eq!(
            (t.tick_ps, t.period_ps, t.high_ps),
            (1_000_000, 2_500_000_000, 1_000_000_000)
        );
        // The model's convention (ARR ticks) is not silicon's: ARR 2500 is a
        // 2501 µs period, more than half a tick off 2500 µs.
        assert!(
            !check_silicon(
                &c,
                1,
                &solved(MotorProtocol::Pwm, Some(p.clone()), (63, 2500, 1000))
            )
            .holds
        );
        // A prescaler one off moves the tick, and with it the idle pulse.
        assert!(
            !check_silicon(
                &c,
                1,
                &solved(MotorProtocol::Pwm, Some(p.clone()), (64, 2499, 1000))
            )
            .holds
        );
        assert!(
            !check_silicon(
                &c,
                1,
                &solved(MotorProtocol::Pwm, Some(p), (63, 2499, 1001))
            )
            .holds
        );
        // 490 Hz: 2041 µs against 2040.82 µs, inside half a tick.
        assert!(
            check_silicon(
                &c,
                3,
                &solved(
                    MotorProtocol::Pwm,
                    Some(pwm(490, 1000, 2000)),
                    (63, 2040, 1000)
                )
            )
            .holds
        );
        // DShot600: a 1671.875 ns bit against 1666.67 ns, 5.2 ns off, and
        // half a tick is 7.8 ns; one tick more is 20.8 ns off.
        let d = solved(MotorProtocol::Dshot600, None, (0, 106, 40));
        let t = check_silicon(&c, 1, &d);
        assert!(t.holds, "{t:?}");
        assert_eq!((t.period_ps, t.high_ps), (1_671_875, 625_000));
        assert!(!check_silicon(&c, 1, &solved(MotorProtocol::Dshot600, None, (0, 107, 40))).holds);
        assert!(!check_silicon(&c, 1, &solved(MotorProtocol::Dshot600, None, (0, 106, 41))).holds);
        assert!(check_silicon(&c, 2, &solved(MotorProtocol::Dshot300, None, (0, 212, 80))).holds);
    }
}
