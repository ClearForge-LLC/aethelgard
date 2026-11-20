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

/// A PWM output's declared timing: its rate, the pulse range its commands
/// take, in microseconds, what it carries from `CEN` until the first command
/// (`idle`), and its timer tick (`resolution_ps`). `idle` and
/// `resolution_ps` are required on every PWM output from specification 22,
/// with no default (`AET-WO-1211a`, `1211` Q4 and Q5 as ruled): the range
/// bounds commands only, and `{"pulse_us": pulse_min_us}` with
/// `resolution_ps` 1,000,000 restates what the firmware wrote before.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct PwmTiming {
    pub rate_hz: u32,
    pub pulse_min_us: u32,
    pub pulse_max_us: u32,
    pub idle: Idle,
    /// Picoseconds per timer tick: exact only at the kernel clock it is
    /// judged at (`ResolutionNotDerivable`), so a change of clock re-judges
    /// it (`1211` Q5, carried to `AET-WO-4240b`). Whole nanoseconds cannot
    /// state the 64 MHz kernel's own tick, 15,625 ps.
    pub resolution_ps: u64,
}

/// A logic level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Low,
    High,
}

impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::High => "high",
        }
    }
}

/// What a PWM output carries from `CEN` until its first command
/// (`AET-WO-1211` §2.4): a held level, realised in PWM mode 1 by `CCR` 0
/// (low) or `ARR + 1` (high) (RM0433 Rev 9 p. 1558), or a pulse of
/// `pulse_us`, above 0 (`IdleSpelledTwice`: CCR 0 has one spelling,
/// `{"level": "low"}`). Exactly one key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Idle {
    Level(Level),
    PulseUs(u32),
}

/// Exactly one key, `level` or `pulse_us`: an empty object is refused as the
/// missing key, never read as a default, and a second key or an unknown one
/// is refused (`INV-06`), as [`Esc`] is.
impl<'de> Deserialize<'de> for Idle {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct IdleVisitor;
        impl<'de> Visitor<'de> for IdleVisitor {
            type Value = Idle;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an Idle object with one key, `level` or `pulse_us`")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Idle, A::Error> {
                let Some(key) = map.next_key::<String>()? else {
                    return Err(serde::de::Error::custom(
                        "missing field `level` / missing field `pulse_us`: an idle holds exactly one of them",
                    ));
                };
                let idle = match key.as_str() {
                    "level" => Idle::Level(map.next_value()?),
                    "pulse_us" => Idle::PulseUs(map.next_value()?),
                    other => {
                        return Err(serde::de::Error::unknown_variant(
                            other,
                            &["level", "pulse_us"],
                        ));
                    }
                };
                if map.next_key::<String>()?.is_some() {
                    return Err(serde::de::Error::custom(
                        "an Idle object holds exactly one key",
                    ));
                }
                Ok(idle)
            }
        }
        deserializer.deserialize_map(IdleVisitor)
    }
}

/// One declared motor output. Every key is required (`INV-06`): `pin` is
/// `null` for the solver to choose, `pwm` is `null` exactly when the
/// protocol is DShot, and `esc` is `null` when the output's ESC is
/// undeclared (specification 18, `AET-WO-2304b` §1.2).
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
    #[serde(deserialize_with = "Option::deserialize")]
    pub esc: Option<Esc>,
}

/// What an output's ESC is (`AET-WO-2304` D2, `2304b` §1.2): today only
/// `none`, an output that drives no ESC fed through the board, with its
/// source. `AET-WO-4500` adds a part beside it. A battery sag on a board
/// whose output declares neither halts by name (`EscUndeclared`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Esc {
    #[serde(rename = "none")]
    NoEsc(NoEsc),
}

/// Exactly one key, a variant's name: an empty object is refused as the
/// missing `none` (the one variant today), never read as a default, and a
/// second key or an unknown one is refused (`INV-06`).
impl<'de> Deserialize<'de> for Esc {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EscVisitor;
        impl<'de> Visitor<'de> for EscVisitor {
            type Value = Esc;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an Esc object with one key, `none`")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Esc, A::Error> {
                let Some(key) = map.next_key::<String>()? else {
                    return Err(serde::de::Error::missing_field("none"));
                };
                let esc = match key.as_str() {
                    "none" => Esc::NoEsc(map.next_value()?),
                    other => return Err(serde::de::Error::unknown_variant(other, &["none"])),
                };
                if map.next_key::<String>()?.is_some() {
                    return Err(serde::de::Error::custom(
                        "an Esc object holds exactly one key",
                    ));
                }
                Ok(esc)
            }
        }
        deserializer.deserialize_map(EscVisitor)
    }
}

/// An output with no ESC on the board's path, and why that is so.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct NoEsc {
    pub source: String,
}

/// The clock tree the generated firmware runs on, as a solved fact
/// (`AET-WO-4300a` §1.2, ruled at `4400` D1). The bring-up firmware never
/// touches `RCC_CR`, `RCC_CFGR` or the PLLs, so it runs on the reset clock:
/// HSI at 64 MHz (`RCC_CR` reset `0x0000 0001`, `HSIDIV` `00`, RM0433 Rev 9
/// §8.7.2), selected (`RCC_CFGR` reset 0, `SW` = `SWS` = HSI, `TIMPRE` 0,
/// §8.7.7), every domain prescaler at its reset value 1 (`RCC_D1CFGR`,
/// `RCC_D2CFGR` and `RCC_D3CFGR` reset 0, "not divided", §8.7.8–§8.7.10).
/// `hse_hz` is the HSE's frequency in whole Hz, 0 when `mcu.hse` is `null`
/// (an HSE tree is reachable only from a whole-Hz source, `AET-WO-4240a`).
/// `vos` is the core's voltage scale (`AET-WO-4240v`): the generated
/// firmware raises it from the reset `scale3` to `scale2` before any
/// peripheral runs, and [`crate::vos`] judges every clock at it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedClock {
    /// `HSI`, or `PLL1` from a declared HSE (`AET-WO-4240a`; generated from
    /// `4240b`).
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
    /// `PWR_D3CR.VOS`, the core's voltage scale.
    pub vos: crate::vos::Vos,
    /// The HSE tree (`AET-WO-4240a`): PLL1 from the declared HSE to 64 MHz,
    /// every prescaler 1, its words and its budget. Absent when the
    /// specification declares none, and not written back then, so a `null`
    /// board's topology is the one it was; an explicit `null` is refused.
    #[serde(
        default,
        deserialize_with = "crate::board::present",
        skip_serializing_if = "Option::is_none"
    )]
    pub hse: Option<crate::clock::SolvedHse>,
}

impl SolvedClock {
    /// The reset RCC tree at VOS Scale 2: the only clock this engine
    /// generates firmware for (`AET-WO-4240v` §1.3). Every RCC field is at
    /// its reset value; the core is at Scale 2, which the bring-up sets
    /// before any peripheral (the reset scale, Scale 3, holds `fPCLK` to 50
    /// MHz, DS12117 Rev 10 Table 111, and the tree runs it at 64).
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
            vos: crate::vos::Vos::Scale2,
            hse: None,
        }
    }

    /// The deviation of the source every clock of this tree derives from
    /// (`AET-WO-4240a` §1.4): the HSE's when one is declared, else the HSI's,
    /// [`crate::clock::Budget::HSI`].
    pub fn budget(&self) -> crate::clock::Budget {
        self.hse
            .as_ref()
            .map_or(crate::clock::Budget::HSI, crate::clock::SolvedHse::budget)
    }

    /// `rcc_hclk1`: `sys_ck` / `D1CPRE` / `HPRE` (RM0433 Rev 9 Figure 49).
    pub fn hclk_hz(&self) -> u64 {
        u64::from(self.sysclk_hz) / u64::from(self.d1cpre.max(1)) / u64::from(self.hpre.max(1))
    }

    /// `sys_d1cpre_ck`, the Cortex-M7's clock, which DWT's cycle counter
    /// counts: `sys_ck` / `D1CPRE` (RM0433 Rev 9 Figure 49, §8.7.8). 64 MHz
    /// on the reset clock.
    pub fn core_hz(&self) -> u64 {
        u64::from(self.sysclk_hz) / u64::from(self.d1cpre.max(1))
    }

    /// `rcc_pclk1`: `rcc_hclk1` / `D2PPRE1` (RM0433 Rev 9 Figure 49,
    /// §8.7.9). LPTIM1's kernel clock at `LPTIM1SEL`'s reset `000`
    /// (`RCC_D2CCIP2R` bits 30:28, §8.7.21). 64 MHz on the reset clock.
    pub fn pclk1_hz(&self) -> u64 {
        self.hclk_hz() / u64::from(self.d2ppre1.max(1))
    }

    /// `rcc_pclk4` (`rcc_pclk_d3`): `rcc_hclk4` / `D3PPRE` (RM0433 Rev 9
    /// §8.7.10). The kernel clock at reset of LPUART1 (`LPUART1SEL` `000`),
    /// LPTIM2 (`LPTIM2SEL` `000`) and LPTIM3–5 (`LPTIM345SEL` `000`), all in
    /// `RCC_D3CCIPR` (§8.7.22, p. 420). 64 MHz on the reset clock.
    pub fn pclk4_hz(&self) -> u64 {
        self.hclk_hz() / u64::from(self.d3ppre.max(1))
    }

    /// The kernel clock of the timers on `apb`, by RM0433 Rev 9 Table 57
    /// (§8.5.6, pp. 350–351, with `TIMPRE`, §8.7.7 p. 392): `rcc_hclk1`
    /// while `D2PPREx` divides by at most 2 (`TIMPRE` 0) or 4 (`TIMPRE` 1),
    /// else 2 or 4 times `rcc_pclkx`. The reset clock gives 64 MHz on both
    /// buses. [`SolvedTopology::validate`](crate::SolvedTopology::validate)
    /// refuses any solved clock but [`SolvedClock::reset`]; a foreign
    /// firmware's clock, decoded by [`SolvedClock::from_rcc`], may be any.
    pub fn timer_kernel_hz(&self, apb: Apb) -> u64 {
        let ppre = u64::from(match apb {
            Apb::Apb1 => self.d2ppre1,
            Apb::Apb2 => self.d2ppre2,
        })
        .max(1);
        let factor = if self.timpre { 4 } else { 2 };
        if ppre <= factor {
            self.hclk_hz()
        } else {
            self.hclk_hz() / ppre * factor
        }
    }

    /// The clock six RCC registers state, read back from the twin at a
    /// foreign run's end (`AET-WO-4400g` §1.2), decoded field by field by
    /// RM0433 Rev 9 ([`RccRegisters::OFFSETS`] cites each register). Only
    /// what a timer's kernel clock depends on is decoded: `D3PPRE` is not
    /// read and `d3ppre` is left 1. `hse_hz` is the HSE's frequency in whole
    /// Hz, 0 for none ([`SolvedClock::hse_hz`]).
    ///
    /// Refused by name: a reserved or unclocked selection, a divider RM0433
    /// calls not allowed or a wrong configuration, an HSE with no crystal
    /// declared, and a `D1CPRE` or `HPRE` of `11xx`. Renode 1.17.0's RCC
    /// divides those by 2^(n+1) (`STM32H7_RCC.cs` L242, L245) where RM0433
    /// divides by 64 to 512 (§8.7.8, p. 394), so the counters Renode clocks
    /// after an RCC write would disagree with this decode.
    pub fn from_rcc(
        r: &RccRegisters,
        hse_hz: u32,
        vos: crate::vos::Vos,
    ) -> Result<SolvedClock, RccDecodeError> {
        // §8.7.2, p. 385: HSIDIV[1:0] divides HSI's 64 MHz by 1, 2, 4 or 8.
        let hsi = 64_000_000u64 >> bits(r.cr, 3, 2);
        // §8.5.2: the CSI oscillator gives "about 4 MHz".
        let csi = 4_000_000u64;
        let hse = || {
            if hse_hz == 0 {
                Err(RccDecodeError::HseUndeclared)
            } else {
                Ok(u64::from(hse_hz))
            }
        };
        // §8.7.7, p. 393: SWS[2:0], the system clock in use. Renode's SWS
        // mirrors SW whether or not the source runs, so the source's ready
        // flag must be set too (§8.7.2: HSIRDY bit 2, CSIRDY bit 8, HSERDY
        // bit 17, PLL1RDY bit 25), as silicon switches only to a ready one.
        let sws = bits(r.cfgr, 3, 3);
        let (source, ready) = match sws {
            0 => ("HSI", 2),
            1 => ("CSI", 8),
            2 => ("HSE", 17),
            3 => ("PLL1", 25),
            _ => {
                return Err(RccDecodeError::Reserved {
                    field: "RCC_CFGR.SWS",
                    value: sws,
                });
            }
        };
        if bits(r.cr, ready, 1) == 0 {
            return Err(RccDecodeError::NotReady { clock: source });
        }
        let sys_ck = match sws {
            0 => hsi,
            1 => csi,
            2 => hse()?,
            _ => pll1_p(r, hsi, csi, hse)?,
        };
        // §8.7.8, p. 394: D1CPRE[3:0] and HPRE[3:0], 0xxx not divided, then
        // 2, 4, 8, 16, 64, 128, 256, 512; D1PPRE[2:0] (and §8.7.9, p. 396,
        // D2PPRE1[2:0] and D2PPRE2[2:0]), 0xx not divided, then 2 to 16.
        let ahb = |field: &'static str, v: u32| match v {
            0..=7 => Ok(1),
            8..=11 => Ok(2u32 << (v - 8)),
            _ => Err(RccDecodeError::Unmodelled { field, value: v }),
        };
        let apb = |v: u32| if v < 4 { 1 } else { 2u32 << (v - 4) };
        let sysclk_hz = u32::try_from(sys_ck).map_err(|_| RccDecodeError::Reserved {
            field: "sys_ck",
            value: u32::MAX,
        })?;
        Ok(SolvedClock {
            source: source.into(),
            sysclk_hz,
            hse_hz,
            d1cpre: ahb("RCC_D1CFGR.D1CPRE", bits(r.d1cfgr, 8, 4))?,
            hpre: ahb("RCC_D1CFGR.HPRE", bits(r.d1cfgr, 0, 4))?,
            d1ppre: apb(bits(r.d1cfgr, 4, 3)),
            d2ppre1: apb(bits(r.d2cfgr, 4, 3)),
            d2ppre2: apb(bits(r.d2cfgr, 8, 3)),
            d3ppre: 1,
            // §8.7.7, p. 392: TIMPRE, bit 15.
            timpre: bits(r.cfgr, 15, 1) == 1,
            // Not read: the specification declares it
            // (`mcu.foreign_clock.vos`, `AET-WO-4240v` D-4).
            vos,
            hse: None,
        })
    }
}

/// `width` bits of `v` from bit `at`.
fn bits(v: u32, at: u32, width: u32) -> u32 {
    (v >> at) & ((1 << width) - 1)
}

/// `pll1_p_ck`, rounded down: the reference `PLLSRC` selects, divided by
/// `DIVM1`, multiplied by `DIVN1`, divided by `DIVP1`. The fractional part
/// is not read: Renode 1.17.0 tags `PLL1FRACEN` and `FRACN1`
/// (`STM32H7_RCC.cs` L291, L370), so a caller states that as a condition.
fn pll1_p(
    r: &RccRegisters,
    hsi: u64,
    csi: u64,
    hse: impl Fn() -> Result<u64, RccDecodeError>,
) -> Result<u64, RccDecodeError> {
    // §8.7.11, p. 399: PLLSRC[1:0] HSI, CSI, HSE or no clock; DIVM1[5:0], 0
    // the prescaler disabled, else the divisor.
    let reference = match bits(r.pllckselr, 0, 2) {
        0 => hsi,
        1 => csi,
        2 => hse()?,
        v => {
            return Err(RccDecodeError::Reserved {
                field: "RCC_PLLCKSELR.PLLSRC",
                value: v,
            });
        }
    };
    let divm1 = bits(r.pllckselr, 4, 6);
    if divm1 == 0 {
        return Err(RccDecodeError::Reserved {
            field: "RCC_PLLCKSELR.DIVM1",
            value: 0,
        });
    }
    // DIVM1's reset value: Renode 1.17.0's RCC starts its PLL1 prescaler at 1
    // (`STM32H7_RCC.cs` L78) and changes it only when a write changes the
    // field (L276), so with 32 read back Renode may be dividing by 1.
    if divm1 == 32 {
        return Err(RccDecodeError::Unmodelled {
            field: "RCC_PLLCKSELR.DIVM1",
            value: 32,
        });
    }
    // §8.7.13, pp. 403–404: DIVN1[8:0] from 0x003 multiplies by the field
    // plus 1 ("others: wrong configurations"); DIVP1[6:0] divides by the
    // field plus 1, which is 1 or even ("odd division factors are not
    // allowed").
    let divn1 = bits(r.pll1divr, 0, 9);
    if divn1 < 3 {
        return Err(RccDecodeError::Reserved {
            field: "RCC_PLL1DIVR.DIVN1",
            value: divn1,
        });
    }
    let divp1 = bits(r.pll1divr, 9, 7);
    let p = u64::from(divp1) + 1;
    if p != 1 && p % 2 != 0 {
        return Err(RccDecodeError::Reserved {
            field: "RCC_PLL1DIVR.DIVP1",
            value: divp1,
        });
    }
    // §8.7.13, p. 404 (DIVN1's caution): the reference 1 to 16 MHz, and the
    // VCO 192 to 960 MHz (`PLL1VCOSEL` 0) or 150 to 420 MHz (1). `RCC_PLLCFGR`
    // is not read, so the wider bound of the two holds: 150 to 960 MHz.
    let ref1 = reference / u64::from(divm1);
    let vco = reference * (u64::from(divn1) + 1) / u64::from(divm1);
    if !(1_000_000..=16_000_000).contains(&ref1) {
        return Err(RccDecodeError::OutOfRange {
            quantity: "ref1_ck",
            hz: ref1,
        });
    }
    if !(150_000_000..=960_000_000).contains(&vco) {
        return Err(RccDecodeError::OutOfRange {
            quantity: "vco1_ck",
            hz: vco,
        });
    }
    Ok(vco / p)
}

/// The RCC registers [`SolvedClock::from_rcc`] decodes, each as read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RccRegisters {
    pub cr: u32,
    pub cfgr: u32,
    pub d1cfgr: u32,
    pub d2cfgr: u32,
    pub pllckselr: u32,
    pub pll1divr: u32,
}

impl RccRegisters {
    /// Each register's name and offset from RCC's base, in the order of
    /// [`RccRegisters::from_values`]: RM0433 Rev 9 §8.7.2 (`RCC_CR`,
    /// 0x000), §8.7.7 (`RCC_CFGR`, 0x010), §8.7.8 (`RCC_D1CFGR`, 0x018),
    /// §8.7.9 (`RCC_D2CFGR`, 0x01C), §8.7.11 (`RCC_PLLCKSELR`, 0x028) and
    /// §8.7.13 (`RCC_PLL1DIVR`, 0x030).
    pub const OFFSETS: [(&'static str, u32); 6] = [
        ("RCC_CR", 0x000),
        ("RCC_CFGR", 0x010),
        ("RCC_D1CFGR", 0x018),
        ("RCC_D2CFGR", 0x01C),
        ("RCC_PLLCKSELR", 0x028),
        ("RCC_PLL1DIVR", 0x030),
    ];

    pub fn from_values(v: [u32; 6]) -> Self {
        Self {
            cr: v[0],
            cfgr: v[1],
            d1cfgr: v[2],
            d2cfgr: v[3],
            pllckselr: v[4],
            pll1divr: v[5],
        }
    }
}

/// Why six RCC registers do not decode to a clock.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RccDecodeError {
    #[error(
        "{field} is {value}: RM0433 Rev 9 calls it reserved, not allowed or a wrong configuration, or it clocks nothing"
    )]
    Reserved { field: &'static str, value: u32 },
    #[error(
        "HSE clocks the system or PLL1, and the specification declares no HSE (`mcu.hse` null)"
    )]
    HseUndeclared,
    #[error(
        "RCC_CFGR.SWS names {clock} as the system clock, but its ready flag in RCC_CR is clear (RM0433 Rev 9 §8.7.2): silicon switches only to a ready source"
    )]
    NotReady { clock: &'static str },
    #[error(
        "{quantity} would be {hz} Hz, outside RM0433 Rev 9 §8.7.13's range (a 1 to 16 MHz reference, a 150 to 960 MHz VCO)"
    )]
    OutOfRange { quantity: &'static str, hz: u64 },
    #[error(
        "{field} is {value}: Renode 1.17.0's RCC divides by another figure than RM0433 Rev 9 there, so the counters it clocks would disagree with the decode"
    )]
    Unmodelled { field: &'static str, value: u32 },
}

/// A foreign firmware's kernel clocks, as its specification declares them
/// (`AET-WO-4400g` §1.2, Q1 ruled on PR #229): the timers of each APB bus,
/// and each USART the platform carries, keyed by peripheral (each of
/// [`crate::serial::PERIPHERALS`], no other). `simulate --foreign` builds
/// the twin's timers and USARTs at these and checks the timers against the
/// RCC registers at the run's end. A USART's kernel clock cannot be read
/// back (Renode 1.17.0 drops `RCC_D2CCIP2R`), so it is declared, never
/// checked, and the verdict says so.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct ForeignClock {
    pub apb1_timers_hz: crate::board::Sourced,
    pub apb2_timers_hz: crate::board::Sourced,
    #[serde(deserialize_with = "crate::unique_keys")]
    pub usarts: std::collections::BTreeMap<String, crate::board::Sourced>,
    /// The voltage scale the firmware runs its core at (`AET-WO-4240v`
    /// D-4): required, with its source. The foreign judge applies
    /// [`crate::vos`] at it, and a clock over its maximum is reported as
    /// [`crate::vos::VOS_LIMIT_AT_BOUNDARY`], never refused.
    pub vos: SourcedVos,
}

/// A declared voltage scale and where it was read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SourcedVos {
    pub value: crate::vos::Vos,
    pub source: String,
}

/// [`ForeignClock`]'s values, copied by `solve`, so the twin builds the
/// platform from the topology alone (`RUL-16`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct SolvedForeignClock {
    pub apb1_timers_hz: u64,
    pub apb2_timers_hz: u64,
    #[serde(deserialize_with = "crate::unique_keys")]
    pub usarts: std::collections::BTreeMap<String, u64>,
    pub vos: crate::vos::Vos,
}

impl SolvedForeignClock {
    /// The declared kernel clock of the timers on `apb`.
    pub fn timer_hz(&self, apb: Apb) -> u64 {
        match apb {
            Apb::Apb1 => self.apb1_timers_hz,
            Apb::Apb2 => self.apb2_timers_hz,
        }
    }
}

impl From<&ForeignClock> for SolvedForeignClock {
    fn from(c: &ForeignClock) -> Self {
        Self {
            apb1_timers_hz: c.apb1_timers_hz.value.into(),
            apb2_timers_hz: c.apb2_timers_hz.value.into(),
            usarts: c
                .usarts
                .iter()
                .map(|(k, v)| (k.clone(), v.value.into()))
                .collect(),
            vos: c.vos.value,
        }
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

crate::object_only!(
    PwmTiming,
    MotorOutput,
    NoEsc,
    SolvedClock,
    SolvedMotorOutput,
    ForeignClock,
    SolvedForeignClock,
    SourcedVos
);

/// Why the solver could not derive a timer's values for an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingError {
    /// `kernel × resolution_ps / 1e12` is not a whole number of kernel
    /// cycles from 1 to 65,536, so no prescaler gives the declared tick.
    ResolutionNotDerivable,
    /// The period is not 3 to 65,536 ticks, or the shortest commanded pulse
    /// is not at least 1 tick and 2 ticks short of the period.
    RateOutOfRange,
    /// The pulse range's maximum does not fit inside the period.
    PulseExceedsPeriod,
    /// `pulse_min_us` or `pulse_max_us` is not a whole number of ticks.
    PulseNotOnTick,
    /// The idle pulse is not a whole number of ticks.
    IdleNotOnTick,
    /// The idle pulse is longer than the period.
    IdleExceedsPeriod,
}

/// A PWM output's idle or tick refused (`AET-WO-1211a`, `1211` §2.4), each
/// its own kind ([`PwmRefusal::kind`]), naming the output.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PwmRefusal {
    #[error(
        "motor output `{output}`: idle `{{\"pulse_us\": 0}}` is CCR 0, which has one spelling, `{{\"level\": \"low\"}}` (`1211` Q4)"
    )]
    IdleSpelledTwice { output: String },
    #[error(
        "motor output `{output}`: resolution_ps {resolution_ps} is not a whole number of cycles from 1 to 65,536 of its {kernel_hz} Hz timer clock, so no prescaler gives it"
    )]
    ResolutionNotDerivable {
        output: String,
        resolution_ps: u64,
        kernel_hz: u64,
    },
    #[error(
        "motor output `{output}`: its pulse range, {min_us} to {max_us} us, is not a whole number of its {resolution_ps} ps ticks"
    )]
    PulseNotOnTick {
        output: String,
        min_us: u32,
        max_us: u32,
        resolution_ps: u64,
    },
    #[error(
        "motor output `{output}`: its idle pulse, {pulse_us} us, is not a whole number of its {resolution_ps} ps ticks"
    )]
    IdleNotOnTick {
        output: String,
        pulse_us: u32,
        resolution_ps: u64,
    },
    #[error(
        "motor output `{output}`: its idle pulse, {pulse_us} us, is longer than its period at {rate_hz} Hz"
    )]
    IdleExceedsPeriod {
        output: String,
        pulse_us: u32,
        rate_hz: u32,
    },
}

impl PwmRefusal {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::IdleSpelledTwice { .. } => "IdleSpelledTwice",
            Self::ResolutionNotDerivable { .. } => "ResolutionNotDerivable",
            Self::PulseNotOnTick { .. } => "PulseNotOnTick",
            Self::IdleNotOnTick { .. } => "IdleNotOnTick",
            Self::IdleExceedsPeriod { .. } => "IdleExceedsPeriod",
        }
    }

    /// The output it names.
    pub fn output(&self) -> &str {
        match self {
            Self::IdleSpelledTwice { output }
            | Self::ResolutionNotDerivable { output, .. }
            | Self::PulseNotOnTick { output, .. }
            | Self::IdleNotOnTick { output, .. }
            | Self::IdleExceedsPeriod { output, .. } => output,
        }
    }
}

/// The picoseconds in one second.
const PS_PER_S: u128 = 1_000_000_000_000;

/// `PSC`, `ARR` and `CCR` for one output on a kernel clock of `kernel_hz`.
///
/// - **PWM:** the declared tick (`PSC = kernel × resolution_ps / 1e12 − 1`,
///   exact or refused), the period the nearest whole tick count to
///   `1 s / rate` (half up), so `ARR` is that less 1 (silicon counts
///   `ARR + 1`), and `CCR` the idle word: 0 for `low`, `ARR + 1` for `high`
///   (RM0433 Rev 9 p. 1558: a compare value above the auto-reload value
///   holds `OCxREF` at 1), or the idle pulse in ticks. The commanded range
///   keeps its rule (at least 1 tick, and 2 ticks short of the period), and
///   the idle word alone is free of it (`AET-WO-1211` §2.4).
/// - **DShot:** `PSC` 0, the bit the nearest whole tick count to
///   `kernel / bit rate`, and a `0` bit's high time the nearest whole count
///   to 3/8 of it.
pub fn solve_timing(
    protocol: MotorProtocol,
    pwm: Option<&PwmTiming>,
    kernel_hz: u64,
) -> Result<(u32, u32, u32), TimingError> {
    let half_up = |num: u128, den: u128| (2 * num + den) / (2 * den);
    let (psc, ticks, ccr) = match (protocol.bit_rate(), pwm) {
        (Some(bit_rate), _) => {
            let ticks = half_up(u128::from(kernel_hz), u128::from(bit_rate));
            let ccr = half_up(
                ticks * u128::from(DSHOT_ZERO_TICKS),
                u128::from(DSHOT_BIT_TICKS),
            );
            if !(3..=0x1_0000).contains(&ticks) || ccr == 0 || ccr + 1 >= ticks {
                return Err(TimingError::RateOutOfRange);
            }
            (0, ticks, ccr)
        }
        (None, Some(p)) => {
            let res = u128::from(p.resolution_ps);
            let cycles = u128::from(kernel_hz) * res;
            if res == 0
                || !cycles.is_multiple_of(PS_PER_S)
                || !(1..=0x1_0000).contains(&(cycles / PS_PER_S))
            {
                return Err(TimingError::ResolutionNotDerivable);
            }
            let ticks = half_up(PS_PER_S, u128::from(p.rate_hz.max(1)) * res);
            if !(3..=0x1_0000).contains(&ticks) {
                return Err(TimingError::RateOutOfRange);
            }
            let on_tick = |us: u32, e: TimingError| {
                let ps = u128::from(us) * 1_000_000;
                if ps.is_multiple_of(res) {
                    Ok(ps / res)
                } else {
                    Err(e)
                }
            };
            let min = on_tick(p.pulse_min_us, TimingError::PulseNotOnTick)?;
            let max = on_tick(p.pulse_max_us, TimingError::PulseNotOnTick)?;
            if max >= ticks {
                return Err(TimingError::PulseExceedsPeriod);
            }
            // The commanded range keeps today's clause; the idle word is free of it.
            if min == 0 || min + 1 >= ticks {
                return Err(TimingError::RateOutOfRange);
            }
            let ccr = match p.idle {
                Idle::Level(Level::Low) => 0,
                Idle::Level(Level::High) => ticks,
                Idle::PulseUs(us) => {
                    let t = on_tick(us, TimingError::IdleNotOnTick)?;
                    if t > ticks {
                        return Err(TimingError::IdleExceedsPeriod);
                    }
                    t
                }
            };
            (cycles / PS_PER_S - 1, ticks, ccr)
        }
        (None, None) => return Err(TimingError::RateOutOfRange),
    };
    let narrow = |v: u128| u32::try_from(v).expect("at most 0x10000");
    Ok((narrow(psc), narrow(ticks - 1), narrow(ccr)))
}

/// What [`check_silicon`] found for one output, in picoseconds where a time
/// is named (`period_ps` and `high_ps` are floors; the verdict is exact).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SiliconTiming {
    pub kernel_hz: u64,
    /// `(PSC + 1)` kernel cycles; not in the verdict.
    #[serde(skip)]
    pub tick_ps: u64,
    /// `(ARR + 1)` ticks: what silicon counts.
    pub period_ps: u64,
    /// `CCR` ticks: the high time of PWM mode 1.
    pub high_ps: u64,
    /// The period and the high time at the clock source's slowest and
    /// fastest ([`SolvedClock::budget`]): the same cycle counts on a
    /// kernel clock of `kernel × (1 − slow)` (floored) and
    /// `kernel × (1 + fast)` (ceiled), floors in picoseconds.
    pub period_slowest_ps: u64,
    pub period_fastest_ps: u64,
    pub high_slowest_ps: u64,
    pub high_fastest_ps: u64,
    /// The declared period: `1 s / rate` for PWM, `1 s / bit rate` for DShot.
    pub nominal_period_ps: u64,
    /// The declared high time: the idle for PWM (0 for `low`, the period
    /// for `high`, else the idle pulse), 3/8 of a bit for DShot's `0`.
    pub nominal_high_ps: u64,
    /// Every rule held, at the nominal clock only: the worst-case figures
    /// above do not enter `holds`. No source this repository holds gives a
    /// DShot or PWM receiver's timing tolerance, so whether a flight
    /// controller accepts these figures at HSI's extremes is the
    /// architect's question (`AET-WO-4230` §1.8), not a judgement made here. The rules: the period within half a tick of the
    /// nominal; for PWM the tick exactly `resolution_ps`, the high time exactly the
    /// declared idle and the range's maximum strictly inside the period; for DShot the high time within
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
    let budget = clock.budget();
    let slow = kernel * u128::from(1_000_000u32.saturating_sub(budget.slow_ppm)) / 1_000_000;
    let fast = (kernel * u128::from(1_000_000 + budget.fast_ppm)).div_ceil(1_000_000);
    let ps_at = |cycles: u128, k: u128| {
        u64::try_from(cycles * 1_000_000_000_000 / k.max(1)).unwrap_or(u64::MAX)
    };
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
                // The declared tick exactly: (PSC+1)/kernel = resolution_ps.
                let tick_ok = psc1 * PS_PER_S == u128::from(p.resolution_ps) * kernel;
                // The declared idle exactly (`AET-WO-1211a`, `1211` §2.4): CCR 0
                // for low, ARR + 1 for high (RM0433 Rev 9 p. 1558), else
                // CCR·(PSC+1)/kernel = the idle pulse.
                let (high_ok, nominal_high) = match p.idle {
                    Idle::Level(Level::Low) => (ccr == 0, 0),
                    Idle::Level(Level::High) => (ccr == arr1, PS_PER_S / rate.max(1)),
                    Idle::PulseUs(us) => (
                        ccr * psc1 * 1_000_000 == u128::from(us) * kernel,
                        u128::from(us) * 1_000_000,
                    ),
                };
                // The longest pulse fits: pulse_max µs < (ARR+1)(PSC+1)/kernel.
                let max_ok = u128::from(p.pulse_max_us) * kernel < arr1 * psc1 * 1_000_000;
                (
                    period_ok && tick_ok && high_ok && max_ok,
                    u64::try_from(PS_PER_S / rate.max(1)).unwrap_or(u64::MAX),
                    u64::try_from(nominal_high).unwrap_or(u64::MAX),
                )
            }
            (None, None) => (false, 0, 0),
        };
    SiliconTiming {
        kernel_hz: u64::try_from(kernel).unwrap_or(u64::MAX),
        tick_ps,
        period_ps,
        high_ps,
        period_slowest_ps: ps_at(arr1 * psc1, slow),
        period_fastest_ps: ps_at(arr1 * psc1, fast),
        high_slowest_ps: ps_at(ccr * psc1, slow),
        high_fastest_ps: ps_at(ccr * psc1, fast),
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
            idle: Idle::PulseUs(lo),
            resolution_ps: 1_000_000,
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
            Err(TimingError::ResolutionNotDerivable)
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

    /// `AET-WO-4230` §1.8: the worst-case period and high time at HSI's two
    /// extremes (62,342,784 Hz slow, 64,958,976 Hz fast on 64 MHz), floors in
    /// picoseconds, and `holds` unchanged by them.
    #[test]
    fn the_worst_case_period_and_high_time_at_hsi_s_extremes() {
        let c = SolvedClock::reset();
        let t = check_silicon(
            &c,
            1,
            &solved(
                MotorProtocol::Pwm,
                Some(pwm(400, 1000, 2000)),
                (63, 2499, 1000),
            ),
        );
        assert!(t.holds);
        assert_eq!((t.period_ps, t.high_ps), (2_500_000_000, 1_000_000_000));
        assert_eq!(
            (t.period_slowest_ps, t.high_slowest_ps),
            (2_566_455_806, 1_026_582_322)
        );
        assert_eq!(
            (t.period_fastest_ps, t.high_fastest_ps),
            (2_463_093_014, 985_237_205)
        );
        let d = check_silicon(&c, 1, &solved(MotorProtocol::Dshot600, None, (0, 106, 40)));
        assert!(d.holds);
        assert_eq!(
            (d.period_slowest_ps, d.high_slowest_ps),
            (1_716_317, 641_613)
        );
        assert_eq!(
            (d.period_fastest_ps, d.high_fastest_ps),
            (1_647_193, 615_773)
        );
    }
}
