//! The voltage-scaling rule (`AET-WO-4240v` §1.3; `RUL-14`, `RUL-20` (1)):
//! every bus clock, and the kernel clock of every peripheral a solved
//! topology enables, judged against the maximum DS12117 and RM0433 allow at
//! the core's voltage scale, at the clock's **fastest** frequency, never at
//! its nominal one (the ruling on #240, S14).
//!
//! **Independence.** This module shares no function, constant or table with
//! `solve.rs`, `motor.rs`, `serial.rs` or `aethelgard-svd-hal`. It computes
//! every clock itself from a [`SolvedClock`]'s fields, in exact integers
//! (`u128`, no floating point). The kernel clocks at reset are
//! [`crate::silicon`]'s reading of the selects, and the fastest frequency is
//! its HSI deviation ([`silicon::fast_hz`]): that module is the other
//! independent rule, never the solver.
//!
//! **The supply words.** [`cr3_supply_bits`] and [`D3CR_VOS_SCALE2`] are
//! written here from RM0433, so the Kani oracle, the twin's `PwrRule` and
//! the ledger take the `PWR` words from this rule, never from the generator,
//! which computes its own from ST's header (`INV-07`).
//!
//! **Revisions.** DS12117 Rev 10 gives the bus maxima (Table 111) for
//! revision V only: revision Y's operating conditions carry no frequency
//! rows. No silicon revision is declared (F-REV), so every verdict that
//! judges a bus clock carries the named condition [`BUS_LIMITS_REV_V`]. The
//! kernel maxima (RM0433 Table 60) hold on both revisions.

use serde::{Deserialize, Serialize};

use crate::{SolvedClock, SolvedTopology, silicon};

/// The core's voltage scale (RM0433 Rev 9 §6.6.2): `scale0` the fastest,
/// `scale3` the reset one ("After reset, the system starts on the lowest Run
/// mode voltage scaling (VOS3)", p. 280).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Vos {
    Scale0,
    Scale1,
    Scale2,
    Scale3,
}

impl Vos {
    /// The column of a limit table: 0 for `scale0` to 3 for `scale3`.
    fn column(self) -> usize {
        match self {
            Self::Scale0 => 0,
            Self::Scale1 => 1,
            Self::Scale2 => 2,
            Self::Scale3 => 3,
        }
    }

    /// The name, as the specification writes it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Scale0 => "scale0",
            Self::Scale1 => "scale1",
            Self::Scale2 => "scale2",
            Self::Scale3 => "scale3",
        }
    }
}

impl std::fmt::Display for Vos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// The named condition of a verdict that judged a bus clock: Table 111's
/// rows are revision V's, and no revision is declared.
pub const BUS_LIMITS_REV_V: &str = "bus_limits_rev_v";

/// The named condition of a foreign firmware's clock over its maximum at its
/// fastest frequency (`AET-WO-4240v` D-4): reported, never refused.
pub const VOS_LIMIT_AT_BOUNDARY: &str = "vos_limit_at_boundary";

/// A family of clocks that share one row of a limit table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// `fCPU`, `rcc_c_ck`.
    Cpu,
    /// `fHCLK`.
    Hclk,
    /// `fPCLK1` to `fPCLK4`.
    Pclk,
    /// The kernel clock of TIM1–8 and TIM12–17.
    Tim,
    Spi123,
    Spi45,
    Spi6,
    I2c123,
    I2c4,
    Usart16,
    Usart234578,
    Lpuart1,
}

const DS12117_TABLE_111: &str = "DS12117 Rev 10 Table 111 (continued), p. 210 (revision V)";
const RM0433_TABLE_60: &str = "RM0433 Rev 9 Table 60, pp. 355-356";

/// Each family's maximum, MHz, at VOS0, VOS1, VOS2 and VOS3, with its
/// citation. `None` is a cell no document states: it judges nothing.
const LIMITS: [(Family, [Option<u64>; 4], &str); 12] = [
    (
        Family::Cpu,
        [Some(480), Some(400), Some(300), Some(200)],
        DS12117_TABLE_111,
    ),
    // Footnote 5: VOS0's 240 MHz holds only at TJmax 105 °C.
    (
        Family::Hclk,
        [Some(240), Some(200), Some(150), Some(100)],
        DS12117_TABLE_111,
    ),
    // Footnote 6: "Maximum APB clock frequency when at least one peripheral
    // is enabled".
    (
        Family::Pclk,
        [Some(120), Some(100), Some(75), Some(50)],
        DS12117_TABLE_111,
    ),
    (
        Family::Tim,
        [Some(240), Some(200), Some(150), Some(100)],
        RM0433_TABLE_60,
    ),
    (
        Family::Spi123,
        [Some(200), Some(200), Some(150), Some(100)],
        RM0433_TABLE_60,
    ),
    (
        Family::Spi45,
        [Some(125), Some(100), Some(75), Some(50)],
        RM0433_TABLE_60,
    ),
    (
        Family::Spi6,
        [Some(125), Some(100), Some(75), Some(50)],
        RM0433_TABLE_60,
    ),
    (
        Family::I2c123,
        [Some(125), Some(100), Some(75), Some(50)],
        RM0433_TABLE_60,
    ),
    (
        Family::I2c4,
        [Some(125), Some(100), Some(75), Some(50)],
        RM0433_TABLE_60,
    ),
    (
        Family::Usart16,
        [Some(125), Some(100), Some(75), Some(50)],
        RM0433_TABLE_60,
    ),
    (
        Family::Usart234578,
        [Some(125), Some(100), Some(75), Some(50)],
        RM0433_TABLE_60,
    ),
    (
        Family::Lpuart1,
        [Some(125), Some(100), Some(75), Some(50)],
        RM0433_TABLE_60,
    ),
];

/// The maximum of `family` at `vos`, Hz, with its citation; `None` where no
/// document states one.
pub fn limit_hz(family: Family, vos: Vos) -> Option<(u64, &'static str)> {
    LIMITS
        .iter()
        .find(|(f, _, _)| *f == family)
        .and_then(|(_, mhz, cite)| mhz[vos.column()].map(|m| (m * 1_000_000, *cite)))
}

/// One clock to judge: its name, its family and its nominal frequency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clock {
    pub name: String,
    pub family: Family,
    pub nominal_hz: u64,
}

/// One clock judged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Judged {
    pub clock: String,
    pub family: Family,
    pub nominal_hz: u64,
    /// `None` where the source's deviation is not known: not judged.
    pub fastest_hz: Option<u64>,
    /// `None` where no document states a maximum: not judged.
    pub limit_hz: Option<u64>,
    pub cite: Option<&'static str>,
    /// `Some(true)` within, `Some(false)` over, `None` not judged.
    pub holds: Option<bool>,
}

/// Judges each clock at `vos`: its fastest frequency, by `fastest`, against
/// its family's maximum. A clock passes only if its fastest is at or under
/// the maximum; one whose fastest or maximum is unknown is not judged.
pub fn judge(vos: Vos, clocks: &[Clock], fastest: impl Fn(u64) -> Option<u64>) -> Vec<Judged> {
    clocks
        .iter()
        .map(|c| {
            let fast = fastest(c.nominal_hz);
            let limit = limit_hz(c.family, vos);
            Judged {
                clock: c.name.clone(),
                family: c.family,
                nominal_hz: c.nominal_hz,
                fastest_hz: fast,
                limit_hz: limit.map(|l| l.0),
                cite: limit.map(|l| l.1),
                holds: fast.zip(limit).map(|(f, (l, _))| f <= l),
            }
        })
        .collect()
}

/// `n / d`, floored; `None` for a divisor of 0.
fn div(n: u64, d: u32) -> Option<u64> {
    (d > 0).then(|| n / u64::from(d))
}

/// The bus clocks of `clock` (RM0433 Rev 9 Figure 49, p. 351; §8.7.8–8.7.10):
/// `rcc_c_ck` is `sys_ck` / `D1CPRE`; `rcc_hclk` is that / `HPRE`;
/// `rcc_pclk3` is `rcc_hclk` / `D1PPRE`, `rcc_pclk1` / `D2PPRE1`,
/// `rcc_pclk2` / `D2PPRE2` and `rcc_pclk4` / `D3PPRE`. A divisor of 0 gives
/// no clock, and the clock is left out.
pub fn bus_clocks(clock: &SolvedClock) -> Vec<Clock> {
    let sys = u64::from(clock.sysclk_hz);
    let cpu = div(sys, clock.d1cpre);
    let hclk = cpu.and_then(|c| div(c, clock.hpre));
    let mut out = Vec::new();
    let mut push = |name: &str, family: Family, hz: Option<u64>| {
        if let Some(nominal_hz) = hz {
            out.push(Clock {
                name: name.to_owned(),
                family,
                nominal_hz,
            });
        }
    };
    push("fCPU", Family::Cpu, cpu);
    push("fHCLK", Family::Hclk, hclk);
    push(
        "fPCLK1",
        Family::Pclk,
        hclk.and_then(|h| div(h, clock.d2ppre1)),
    );
    push(
        "fPCLK2",
        Family::Pclk,
        hclk.and_then(|h| div(h, clock.d2ppre2)),
    );
    push(
        "fPCLK3",
        Family::Pclk,
        hclk.and_then(|h| div(h, clock.d1ppre)),
    );
    push(
        "fPCLK4",
        Family::Pclk,
        hclk.and_then(|h| div(h, clock.d3ppre)),
    );
    out
}

/// The timers on APB2 (RM0433 Rev 9 §2.3.2 Table 8, the memory map): TIM1,
/// TIM8, TIM15, TIM16 and TIM17; every other timer of `TIM1`–`TIM17` is on
/// APB1. This module's own list.
fn on_apb2(timer: u32) -> bool {
    matches!(timer, 1 | 8 | 15 | 16 | 17)
}

/// The kernel clock of `timer` (RM0433 Rev 9 Table 57, §8.5.6, pp.
/// 350–351, with `RCC_CFGR.TIMPRE`, §8.7.7 p. 392): `rcc_hclk1` while the
/// timer's `D2PPREx` divides by at most 2 (`TIMPRE` 0) or 4 (`TIMPRE` 1),
/// else 2 or 4 times its `rcc_pclkx`. Computed here again, on purpose.
pub fn timer_kernel(clock: &SolvedClock, timer: u32) -> Option<u64> {
    let hclk = div(u64::from(clock.sysclk_hz), clock.d1cpre).and_then(|c| div(c, clock.hpre))?;
    let ppre = if on_apb2(timer) {
        clock.d2ppre2
    } else {
        clock.d2ppre1
    };
    let factor: u32 = if clock.timpre { 4 } else { 2 };
    if ppre == 0 {
        None
    } else if ppre <= factor {
        Some(hclk)
    } else {
        Some(hclk / u64::from(ppre) * u64::from(factor))
    }
}

/// The family of a USART, UART or LPUART's kernel row in Table 60.
pub fn usart_family(peripheral: &str) -> Option<Family> {
    match peripheral {
        "USART1" | "USART6" => Some(Family::Usart16),
        "USART2" | "USART3" | "UART4" | "UART5" | "UART7" | "UART8" => Some(Family::Usart234578),
        "LPUART1" => Some(Family::Lpuart1),
        _ => None,
    }
}

/// The clocks a generated firmware runs on `t`: every bus clock, and the
/// kernel clock of each timer, SPI, I2C and USART the topology enables, each
/// by this rule's own reading of its select. A kernel whose select the rule
/// cannot read is returned by name in the second vector, never judged as
/// passing.
pub fn generated_clocks(t: &SolvedTopology) -> (Vec<Clock>, Vec<String>) {
    let mut clocks = bus_clocks(&t.clock);
    let mut unread = Vec::new();
    let timers: std::collections::BTreeSet<u32> = t
        .assignments
        .values()
        .filter_map(|a| a.peripheral.strip_prefix("TIM")?.parse().ok())
        .collect();
    for n in timers {
        match timer_kernel(&t.clock, n) {
            Some(hz) => clocks.push(Clock {
                name: format!("TIM{n} kernel"),
                family: Family::Tim,
                nominal_hz: hz,
            }),
            None => unread.push(format!("TIM{n} kernel")),
        }
    }
    for (peripheral, c) in &t.spi_controllers {
        let family = match peripheral.as_str() {
            "SPI1" | "SPI2" | "SPI3" => Family::Spi123,
            "SPI4" | "SPI5" => Family::Spi45,
            _ => Family::Spi6,
        };
        match silicon::spi_kernel_hz(peripheral, c.kernel) {
            Some(hz) => clocks.push(Clock {
                name: format!("{peripheral} kernel"),
                family,
                nominal_hz: hz,
            }),
            None => unread.push(format!("{peripheral} kernel")),
        }
    }
    // I2C1–3 on `rcc_pclk1` at `I2C123SEL`'s reset `00`, I2C4 on
    // `rcc_pclk4` at `I2C4SEL`'s reset `00` (§8.7.21 p. 416, §8.7.22 p. 420).
    let bus = bus_clocks(&t.clock);
    let pclk = |name: &str| bus.iter().find(|c| c.name == name).map(|c| c.nominal_hz);
    for peripheral in t.i2c_timings.keys() {
        let (family, hz) = match peripheral.as_str() {
            "I2C4" => (Family::I2c4, pclk("fPCLK4")),
            _ => (Family::I2c123, pclk("fPCLK1")),
        };
        match hz {
            Some(hz) => clocks.push(Clock {
                name: format!("{peripheral} kernel"),
                family,
                nominal_hz: hz,
            }),
            None => unread.push(format!("{peripheral} kernel")),
        }
    }
    for port in t.serial_ports.values() {
        let p = &port.peripheral;
        match (usart_family(p), silicon::usart_kernel_hz(&t.clock, p)) {
            (Some(family), Some(hz)) => clocks.push(Clock {
                name: format!("{p} kernel"),
                family,
                nominal_hz: hz,
            }),
            _ => unread.push(format!("{p} kernel")),
        }
    }
    (clocks, unread)
}

/// The fastest a clock of the generated tree runs: every one is HSI-derived
/// (the reset tree), so HSI's worst fast deviation, by
/// [`silicon::fast_hz`], rounded up.
pub fn hsi_fastest(nominal_hz: u64) -> Option<u64> {
    Some(silicon::fast_hz(nominal_hz))
}

/// `FLASH_ACR` at reset, judged against RM0433 Rev 9 Table 17 (p. 160) at
/// the AXI clock's fastest (`AET-WO-4240v` D-7): nothing writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlashJudged {
    pub vos: Vos,
    pub hclk_fastest_hz: u64,
    /// `LATENCY` and `WRHIGHFREQ` at reset (`FLASH_ACR` reset `0x0000 0037`,
    /// RM0433 Rev 9 §4.9.1, p. 201).
    pub latency: u32,
    pub wrhighfreq: u32,
    /// What Table 17's row for `vos` and the clock needs; `None` past the
    /// table (no row).
    pub needed: Option<(u32, u32)>,
    pub holds: bool,
}

/// `FLASH_ACR`'s reset value (RM0433 Rev 9 §4.9.1, p. 201): `LATENCY[3:0]`
/// bits 3:0 = 7, `WRHIGHFREQ[1:0]` bits 5:4 = `11`.
const FLASH_ACR_RESET: u32 = 0x0000_0037;

/// RM0433 Rev 9 Table 17 (p. 160), "FLASH recommended number of wait
/// states and programming delay": per VOS, each row's upper `rcc_aclk`
/// bound in Hz (the lower bound is the row before's, exclusive), with
/// `LATENCY` and `WRHIGHFREQ`.
const TABLE_17: [(Vos, &[(u64, u32, u32)]); 4] = [
    (
        Vos::Scale0,
        &[
            (70_000_000, 0, 0b00),
            (140_000_000, 1, 0b01),
            (185_000_000, 2, 0b01),
            (210_000_000, 2, 0b10),
            (225_000_000, 3, 0b10),
            (240_000_000, 4, 0b10),
        ],
    ),
    (
        Vos::Scale1,
        &[
            (70_000_000, 0, 0b00),
            (140_000_000, 1, 0b01),
            (185_000_000, 2, 0b01),
            (210_000_000, 2, 0b10),
            (225_000_000, 3, 0b10),
        ],
    ),
    (
        Vos::Scale2,
        &[
            (55_000_000, 0, 0b00),
            (110_000_000, 1, 0b01),
            (165_000_000, 2, 0b01),
            (225_000_000, 3, 0b10),
        ],
    ),
    (
        Vos::Scale3,
        &[
            (45_000_000, 0, 0b00),
            (90_000_000, 1, 0b01),
            (135_000_000, 2, 0b01),
            (180_000_000, 3, 0b10),
            (225_000_000, 4, 0b10),
        ],
    ),
];

/// Judges `FLASH_ACR` at reset for `clock` at its `vos`: `LATENCY` must be at
/// least the row's ("After power-on, a default 7 wait-state latency is
/// specified ... with a safety margin", p. 160), and `WRHIGHFREQ` at least
/// the row's programming delay. A longer delay than the row's is taken as
/// within: the table states the delay programming needs, and this firmware
/// never programs the flash (inferred, `AET-WO-4240v` FEEDBACK).
pub fn flash_at_reset(clock: &SolvedClock) -> FlashJudged {
    let hclk = div(u64::from(clock.sysclk_hz), clock.d1cpre)
        .and_then(|c| div(c, clock.hpre))
        .unwrap_or(0);
    let hclk_fastest_hz = silicon::fast_hz(hclk);
    let latency = FLASH_ACR_RESET & 0xF;
    let wrhighfreq = (FLASH_ACR_RESET >> 4) & 0b11;
    let needed = TABLE_17
        .iter()
        .find(|(v, _)| *v == clock.vos)
        .and_then(|(_, rows)| rows.iter().find(|(max, _, _)| hclk_fastest_hz <= *max))
        .map(|(_, ws, wr)| (*ws, *wr));
    FlashJudged {
        vos: clock.vos,
        hclk_fastest_hz,
        latency,
        wrhighfreq,
        needed,
        holds: needed.is_some_and(|(ws, wr)| latency >= ws && wrhighfreq >= wr),
    }
}

/// What the rule found for a generated topology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Verdict {
    pub vos: Vos,
    pub clocks: Vec<Judged>,
    /// Kernels whose select the rule cannot read, by name.
    pub unread: Vec<String>,
    pub flash: FlashJudged,
    /// [`BUS_LIMITS_REV_V`] when a bus clock was judged.
    pub conditions: Vec<&'static str>,
}

impl Verdict {
    /// Every clock judged within its maximum, every kernel read, and
    /// `FLASH_ACR` at reset within Table 17.
    pub fn holds(&self) -> bool {
        self.unread.is_empty()
            && self.flash.holds
            && self.clocks.iter().all(|c| c.holds == Some(true))
    }
}

/// The rule on a generated topology: its tree at its `vos`, every clock at
/// its fastest.
pub fn judge_generated(t: &SolvedTopology) -> Verdict {
    let (clocks, unread) = generated_clocks(t);
    let clocks = judge(t.clock.vos, &clocks, hsi_fastest);
    let conditions = if clocks
        .iter()
        .any(|c| matches!(c.family, Family::Cpu | Family::Hclk | Family::Pclk))
    {
        vec![BUS_LIMITS_REV_V]
    } else {
        Vec::new()
    };
    Verdict {
        vos: t.clock.vos,
        clocks,
        unread,
        flash: flash_at_reset(&t.clock),
        conditions,
    }
}

// ---------------------------------------------------------------- foreign

/// What the rule found for a foreign firmware's clock (`AET-WO-4240v` D-4):
/// reported, never refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ForeignVerdict {
    /// The declared scale (`mcu.foreign_clock.vos`).
    pub vos: Vos,
    pub clocks: Vec<Judged>,
    /// Each clock the decode does not reach, with why, by name.
    pub not_judged: Vec<String>,
    /// [`VOS_LIMIT_AT_BOUNDARY`] once per clock over its maximum at its
    /// fastest, then [`BUS_LIMITS_REV_V`] when a bus clock was judged.
    pub conditions: Vec<String>,
}

/// The rule at a foreign run's end (`AET-WO-4240v` D-4): `decoded` is the
/// tree [`SolvedClock::from_rcc`](crate::SolvedClock::from_rcc) decoded from
/// `registers`, at the declared `vos`; `usarts` the declared USART kernel
/// clocks. Every clock is judged at the HSI's fastest when the tree is HSI's
/// (`RCC_CFGR.SWS` HSI, or PLL1 with `RCC_PLLCKSELR.PLLSRC` `00`, HSI, RM0433
/// Rev 9 §8.7.11 p. 399; one source feeds every PLL); on any other source no
/// deviation is declared and nothing is judged. `fPCLK4` is not judged:
/// `D3PPRE` is not read. The USART kernels are declared, never read back
/// (Renode 1.17.0 drops `RCC_D2CCIP2R`), and are judged as HSI-derived when
/// the tree is.
pub fn judge_foreign(
    registers: &crate::motor::RccRegisters,
    decoded: &SolvedClock,
    usarts: &std::collections::BTreeMap<String, u64>,
) -> ForeignVerdict {
    let pllsrc = registers.pllckselr & 0b11;
    let hsi = decoded.source == "HSI" || (decoded.source == "PLL1" && pllsrc == 0b00);
    let mut clocks: Vec<Clock> = bus_clocks(decoded)
        .into_iter()
        .filter(|c| c.name != "fPCLK4")
        .collect();
    let mut not_judged = vec!["fPCLK4: D3PPRE is not read".to_owned()];
    for (name, timer) in [("APB1 timers kernel", 2), ("APB2 timers kernel", 1)] {
        match timer_kernel(decoded, timer) {
            Some(hz) => clocks.push(Clock {
                name: name.to_owned(),
                family: Family::Tim,
                nominal_hz: hz,
            }),
            None => not_judged.push(format!("{name}: no clock decoded")),
        }
    }
    for (usart, hz) in usarts {
        match usart_family(usart) {
            Some(family) => clocks.push(Clock {
                name: format!("{usart} kernel"),
                family,
                nominal_hz: *hz,
            }),
            None => not_judged.push(format!("{usart} kernel: no Table 60 row")),
        }
    }
    if !hsi {
        not_judged.extend(clocks.drain(..).map(|c| {
            format!(
                "{}: the tree is {} (PLLSRC {pllsrc:02b}), whose deviation is not declared",
                c.name, decoded.source
            )
        }));
    }
    let judged = judge(decoded.vos, &clocks, hsi_fastest);
    let mut conditions: Vec<String> = judged
        .iter()
        .filter(|c| c.holds == Some(false))
        .map(|c| {
            format!(
                "{VOS_LIMIT_AT_BOUNDARY}: {} at {} Hz at its fastest, over its {} Hz maximum at {} ({})",
                c.clock,
                c.fastest_hz.unwrap_or(0),
                c.limit_hz.unwrap_or(0),
                decoded.vos,
                c.cite.unwrap_or("")
            )
        })
        .collect();
    if judged
        .iter()
        .any(|c| matches!(c.family, Family::Cpu | Family::Hclk | Family::Pclk))
    {
        conditions.push(BUS_LIMITS_REV_V.to_owned());
    }
    ForeignVerdict {
        vos: decoded.vos,
        clocks: judged,
        not_judged,
        conditions,
    }
}

// ---------------------------------------------------------------- PWR words

/// `PWR_CR3`'s supply bits (RM0433 Rev 9 §6.8.4, p. 308): `SCUEN` bit 2,
/// `LDOEN` bit 1, `BYPASS` bit 0, each rw. "The lower byte of this register
/// is written once after POR and shall be written before changing VOS level
/// or ck_sys clock frequency"; bits 7:3 are "Reserved, must be kept at reset
/// value" (bit 6 resets to 1 on revision V, `0x0000 0046`, and to 0 on
/// revision Y, `0x0000 0006`, p. 307), so the bring-up writes every bit but
/// these three as it read them.
pub const CR3_SUPPLY_MASK: u32 = 0b111;

/// `LDOEN` and `BYPASS`, the supply configuration RM0433 Rev 9 Table 34
/// (p. 261) names: the read-back judges these two. `SCUEN` is not a supply
/// configuration (Table 34 does not name it), and what it reads after the
/// single write is not stated (p. 308).
pub const CR3_SUPPLY_CONFIG_MASK: u32 = 0b011;

/// `PWR_D3CR.VOS[1:0]`, bits 15:14 (RM0433 Rev 9 §6.8.6, p. 310): `00`
/// "Reserved (Scale 3 selected)", `01` Scale 3 (default), `10` Scale 2, `11`
/// Scale 1. Scale 0 has no code here: it is VOS1 with `SYSCFG_PWRCR.ODEN`.
pub const D3CR_VOS_MASK: u32 = 0b11 << 14;

/// `PWR_D3CR` as the bring-up writes it, whole: `VOS` = `10`, Scale 2; bit 13
/// (`VOSRDY`) is read-only, and bits 31:16 and 12:0 are "Reserved, must be
/// kept at reset value", which is 0 (reset `0x0000 4000`, p. 310).
pub const D3CR_VOS_SCALE2: u32 = 0b10 << 14;

/// The supply configuration a board declares (`mcu.core_supply.mode`): the
/// two legal rows of RM0433 Rev 9 Table 34 (p. 261); `LDOEN` = `BYPASS` is
/// "Illegal combination, the default configuration is kept", and the
/// write-once mechanism then locks the register until a power cycle (p. 308).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CoreSupplyMode {
    /// "LDO supply": `LDOEN` 1, `BYPASS` 0. VCORE from the internal LDO.
    Ldo,
    /// "LDO Bypass": `LDOEN` 0, `BYPASS` 1. "VCORE supplied from external
    /// source".
    Bypass,
}

impl CoreSupplyMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Ldo => "ldo",
            Self::Bypass => "bypass",
        }
    }
}

/// The `PWR_CR3` supply bits `mode` writes, under [`CR3_SUPPLY_MASK`]:
/// `LDOEN` and `BYPASS` by Table 34's row, and `SCUEN` 0, "Supply
/// configuration update locked" (p. 308): the bring-up's write is the single
/// write the bit enables, and the word states the locked state that follows
/// it (`AET-WO-4240v` FEEDBACK, the `SCUEN` decision).
pub fn cr3_supply_bits(mode: CoreSupplyMode) -> u32 {
    let scuen = 0u32 << 2;
    let (ldoen, bypass) = match mode {
        CoreSupplyMode::Ldo => (1u32, 0u32),
        CoreSupplyMode::Bypass => (0, 1),
    };
    scuen | (ldoen << 1) | bypass
}
