//! ArduPilot's STM32H7 clock tree (`AET-WO-4412` D1 (a)): the timer and
//! USART kernel clocks unmodified ArduPilot runs at, for each oscillator its
//! `stm32h7_mcuconf.h` supports, read from the pinned image and held as
//! cited data (`data/ardupilot-h7-clock.tsv`, whose header names the file,
//! its sha256 and every line).
//!
//! A specification's `mcu.foreign_clock` is the foreign firmware's clock,
//! declared; this table is what it must equal for ArduPilot on the declared
//! oscillator, so the declaration is derived from the specification's crystal
//! and cannot drift from the firmware it describes. The arithmetic is
//! RM0433 Rev 9's: `PLLx = osc / DIVMx × DIVNx / (DIVPx | DIVQx)`; HCLK is
//! `PLL1_P / D1CPRE / D1HPRE`; with `TIMPRE` 1 and `D2PPREx` /2 each APB
//! timer clock is HCLK (Table 57, pp. 351–352); the USARTs run on `PLL2_Q`.

use crate::vos::Vos;

/// The held table, verbatim.
pub const TABLE_TSV: &str = include_str!("../data/ardupilot-h7-clock.tsv");

/// The HSI's frequency with `STM32_HSIDIV_DIV1` (L104): 64 MHz.
pub const HSI_HZ: u64 = 64_000_000;
/// `STM32_D1CPRE_DIV1` (L289) and `STM32_D1HPRE_DIV2` (L290).
const D1CPRE: u64 = 1;
const D1HPRE: u64 = 2;

/// One row's clocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tree {
    /// `OSCILLATOR_HZ`; 0 is no crystal (the HSI).
    pub oscillator_hz: u64,
    /// Each APB timer kernel clock: HCLK (`TIMPRE` 1, `D2PPRE` /2).
    pub timers_hz: u64,
    /// Every USART's kernel clock: `PLL2_Q` (L343–344).
    pub usarts_hz: u64,
    /// `STM32_VOS_SCALE1` (L109).
    pub vos: Vos,
    /// The rows' lines in the held file.
    pub lines: &'static str,
}

fn rows() -> impl Iterator<Item = (u64, [u64; 6], &'static str)> {
    TABLE_TSV
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            let n = |i: usize| f[i].parse::<u64>().expect("a held integer");
            (n(0), [n(1), n(2), n(3), n(4), n(5), n(6)], f[7])
        })
}

/// ArduPilot's tree on `oscillator_hz`, or `None` where its header has no
/// row (`#error "Unsupported HSE clock"`, L164–165).
pub fn tree(oscillator_hz: u64) -> Option<Tree> {
    let (_, [m1, n1, p1, m2, n2, q2], lines) = rows().find(|(osc, _, _)| *osc == oscillator_hz)?;
    let src = if oscillator_hz == 0 {
        HSI_HZ
    } else {
        oscillator_hz
    };
    let hclk = src * n1 / (m1 * p1 * D1CPRE * D1HPRE);
    Some(Tree {
        oscillator_hz,
        timers_hz: hclk,
        usarts_hz: src * n2 / (m2 * q2),
        vos: Vos::Scale1,
        lines,
    })
}

/// A foreign firmware that is ArduPilot runs this tree (`AET-WO-4412` D1 (b),
/// ruled 2026-11-26T11:26Z): `simulate --foreign` holds a declared
/// `mcu.foreign_clock` to it when the ELF names `AP_HAL::panic`. Every timer
/// and USART kernel clock and the scale equal [`tree`]'s row for `hse`'s
/// frequency, or for no oscillator (the HSI) when none is declared; an
/// oscillator with no row is refused too. `validate` holds a declaration to
/// its own consistency only, so a firmware of the user's own declares its own.
pub fn hold(
    c: &crate::motor::SolvedForeignClock,
    hse: Option<&crate::clock::Hse>,
) -> Result<(), crate::IrValidationError> {
    use crate::IrValidationError as E;

    let oscillator_hz = match hse {
        None => 0,
        Some(h) => match crate::clock::source_hz(h)? {
            (hz, 1) => hz,
            (num, den) => {
                return Err(E::ForeignClockOscillatorUnsupported {
                    oscillator: format!("{num}/{den} Hz"),
                    held: oscillators(),
                });
            }
        },
    };
    let Some(t) = tree(oscillator_hz) else {
        return Err(E::ForeignClockOscillatorUnsupported {
            oscillator: format!("{oscillator_hz} Hz"),
            held: oscillators(),
        });
    };
    let wrong = |field: String, declared: String, expected: String| E::ForeignClockNotArduPilots {
        field,
        declared,
        expected,
        oscillator_hz,
        lines: t.lines.to_owned(),
    };
    for (field, hz) in [
        ("apb1_timers_hz".to_owned(), c.apb1_timers_hz),
        ("apb2_timers_hz".to_owned(), c.apb2_timers_hz),
    ]
    .into_iter()
    .chain(c.usarts.iter().map(|(k, v)| (format!("usarts.{k}"), *v)))
    {
        let want = if field.starts_with("usarts.") {
            t.usarts_hz
        } else {
            t.timers_hz
        };
        if hz != want {
            return Err(wrong(field, format!("{hz} Hz"), format!("{want} Hz")));
        }
    }
    if c.vos != t.vos {
        return Err(wrong(
            "vos".to_owned(),
            c.vos.to_string(),
            t.vos.to_string(),
        ));
    }
    Ok(())
}

/// Every oscillator the table holds, in order.
pub fn oscillators() -> Vec<u64> {
    rows().map(|(osc, _, _)| osc).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_divides_exactly_and_gives_ardupilots_400_mhz_core() {
        for (osc, [m1, n1, p1, m2, n2, q2], _) in rows() {
            let src = if osc == 0 { HSI_HZ } else { osc };
            assert_eq!(src * n1 % (m1 * p1), 0, "{osc}");
            assert_eq!(src * n2 % (m2 * q2), 0, "{osc}");
            assert_eq!(src * n1 / (m1 * p1), 400_000_000, "{osc}: sys_ck");
        }
    }

    #[test]
    fn the_hsi_row_is_sentinel_02_owns_and_every_crystal_runs_usarts_at_100_mhz() {
        let hsi = tree(0).expect("the HSI row");
        assert_eq!((hsi.timers_hz, hsi.usarts_hz), (200_000_000, 72_000_000));
        for osc in [8_000_000, 16_000_000, 24_000_000, 25_000_000, 32_000_000] {
            let t = tree(osc).expect("a crystal row");
            assert_eq!(
                (t.timers_hz, t.usarts_hz),
                (200_000_000, 100_000_000),
                "{osc}"
            );
        }
        assert_eq!(tree(12_000_000), None);
        assert_eq!(oscillators().len(), 6);
    }
}
