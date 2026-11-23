//! The kernel-clock switch rule (`AET-WO-4240e`; `RUL-20` (1), `INV-07`):
//! every enabled kernel-clocked peripheral has a running kernel clock when
//! it starts.
//!
//! **The p. 371 rule.** "Switching from the original to the new input can
//! only be performed if a clock is present on both inputs. If it [is] not the
//! case, no clock will be provided to the peripheral" (RM0433 Rev 9 §8.5.10,
//! "Clock switches and gating", p. 371; repeated p. 372). Each select
//! register resets to `0x0000 0000` (`RCC_D2CCIP1R` p. 413, `RCC_D2CCIP2R`
//! p. 416, `RCC_D3CCIPR` p. 418), so a mux's input at entry is its code
//! `0`, and a select that leaves it moves only while both inputs run.
//!
//! **Warm resets.** "After system reset, PLLs, HSE, CSI and HSI48 are
//! switched OFF. If one of these clocks is used as input clock through the
//! MUX, the switching does not take place" (p. 371), with p. 372's USART1
//! example: a mux left on `pll2_q_ck` stays there until `pll2_q_ck` runs. So
//! after a warm reset a mux may still sit on the input selected before it,
//! not on its reset input. This rule judges entry from the reset input, the
//! power-on state, and nothing else; entry from a selection made before a
//! warm reset is not judged here.
//!
//! **Independence.** The table, the decode of the PLL1 words and the set of
//! running sources are this module's own, read from RM0433 Rev 9 and
//! DS12117 Rev 10. It shares no function, constant or table with `solve.rs`,
//! `silicon.rs` or `aethelgard-svd-hal`, and reads the topology alone, never
//! the generator's sequence or the emitted firmware. It takes the HSI's band
//! from [`Budget::HSI`], the one budget every HSI verdict uses.
//!
//! **Premises the topology cannot show.** Stated in `AET-WO-4240e`: `HSION`
//! is never cleared and `CKPERSEL` is never written. This module's own:
//! `HSIDIV` is never written, and PLL2, PLL3, the CSI and the LSE are never
//! started (so each is judged not running).

use std::collections::BTreeSet;
use std::fmt;

use serde::Serialize;

use crate::clock::{Budget, SolvedKernelSwitch};
use crate::vos::Vos;
use crate::{SolvedTopology, SpiKernel};

/// The p. 371 rule's citation, carried by every refusal.
const RULE_PAGES: &str = "RM0433 Rev 9 §8.5.10, pp. 371–372";

// ---------------------------------------------------------------- the table

/// One kernel-clock mux: its select field and every defined code's input.
/// Each register resets to `0x0000 0000`, so the input at reset is code `0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Selector {
    pub name: &'static str,
    pub register: &'static str,
    pub bits: &'static str,
    /// Code → input; the codes not listed are "reserved, the kernel clock is
    /// disabled".
    pub codes: &'static [(u32, &'static str)],
    pub pages: &'static str,
}

impl Selector {
    /// The input at the register's reset value, code `0`.
    pub fn reset_input(&self) -> &'static str {
        self.input(0).expect("every row defines code 0")
    }

    pub fn input(&self, code: u32) -> Option<&'static str> {
        self.codes.iter().find(|(c, _)| *c == code).map(|(_, i)| *i)
    }
}

/// `RCC_D2CCIP1R.SPI123SEL` (§8.7.20, p. 414; Figure 51, p. 359).
pub const SPI123SEL: Selector = Selector {
    name: "SPI123SEL",
    register: "RCC_D2CCIP1R",
    bits: "14:12",
    codes: &[
        (0b000, "pll1_q_ck"),
        (0b001, "pll2_p_ck"),
        (0b010, "pll3_p_ck"),
        (0b011, "I2S_CKIN"),
        (0b100, "per_ck"),
    ],
    pages: "RM0433 Rev 9 §8.7.20, p. 414; Figure 51, p. 359",
};

/// `RCC_D2CCIP1R.SPI45SEL` (§8.7.20, p. 414). The register names code `000`
/// "APB clock"; Figure 51 (p. 359) names it `rcc_pclk2`.
pub const SPI45SEL: Selector = Selector {
    name: "SPI45SEL",
    register: "RCC_D2CCIP1R",
    bits: "18:16",
    codes: &[
        (0b000, "rcc_pclk2"),
        (0b001, "pll2_q_ck"),
        (0b010, "pll3_q_ck"),
        (0b011, "hsi_ker_ck"),
        (0b100, "csi_ker_ck"),
        (0b101, "hse_ck"),
    ],
    pages: "RM0433 Rev 9 §8.7.20, p. 414; Figure 51, p. 359",
};

/// `RCC_D3CCIPR.SPI6SEL` (§8.7.22, p. 418).
pub const SPI6SEL: Selector = Selector {
    name: "SPI6SEL",
    register: "RCC_D3CCIPR",
    bits: "30:28",
    codes: &[
        (0b000, "rcc_pclk4"),
        (0b001, "pll2_q_ck"),
        (0b010, "pll3_q_ck"),
        (0b011, "hsi_ker_ck"),
        (0b100, "csi_ker_ck"),
        (0b101, "hse_ck"),
    ],
    pages: "RM0433 Rev 9 §8.7.22, p. 418; Figure 51, p. 359",
};

/// `RCC_D2CCIP2R.I2C123SEL` (§8.7.21, p. 416).
pub const I2C123SEL: Selector = Selector {
    name: "I2C123SEL",
    register: "RCC_D2CCIP2R",
    bits: "13:12",
    codes: &[
        (0b00, "rcc_pclk1"),
        (0b01, "pll3_r_ck"),
        (0b10, "hsi_ker_ck"),
        (0b11, "csi_ker_ck"),
    ],
    pages: "RM0433 Rev 9 §8.7.21, p. 416; Figure 52, p. 360",
};

/// `RCC_D3CCIPR.I2C4SEL` (§8.7.22, p. 420).
pub const I2C4SEL: Selector = Selector {
    name: "I2C4SEL",
    register: "RCC_D3CCIPR",
    bits: "9:8",
    codes: &[
        (0b00, "rcc_pclk4"),
        (0b01, "pll3_r_ck"),
        (0b10, "hsi_ker_ck"),
        (0b11, "csi_ker_ck"),
    ],
    pages: "RM0433 Rev 9 §8.7.22, p. 420; Figure 52, p. 360",
};

/// `RCC_D2CCIP2R.USART16SEL` (§8.7.21, p. 417).
pub const USART16SEL: Selector = Selector {
    name: "USART16SEL",
    register: "RCC_D2CCIP2R",
    bits: "5:3",
    codes: &[
        (0b000, "rcc_pclk2"),
        (0b001, "pll2_q_ck"),
        (0b010, "pll3_q_ck"),
        (0b011, "hsi_ker_ck"),
        (0b100, "csi_ker_ck"),
        (0b101, "lse_ck"),
    ],
    pages: "RM0433 Rev 9 §8.7.21, p. 417; Figure 53, p. 360",
};

/// `RCC_D2CCIP2R.USART234578SEL` (§8.7.21, p. 417).
pub const USART234578SEL: Selector = Selector {
    name: "USART234578SEL",
    register: "RCC_D2CCIP2R",
    bits: "2:0",
    codes: &[
        (0b000, "rcc_pclk1"),
        (0b001, "pll2_q_ck"),
        (0b010, "pll3_q_ck"),
        (0b011, "hsi_ker_ck"),
        (0b100, "csi_ker_ck"),
        (0b101, "lse_ck"),
    ],
    pages: "RM0433 Rev 9 §8.7.21, p. 417; Figure 53, p. 360",
};

/// `RCC_D3CCIPR.ADCSEL` (§8.7.22, p. 419): `adc_ker_ck_input`, which an ADC
/// takes only with `ADCx_CCR.CKMODE` = `00` (§25.4.3, pp. 920–921).
pub const ADCSEL: Selector = Selector {
    name: "ADCSEL",
    register: "RCC_D3CCIPR",
    bits: "17:16",
    codes: &[(0b00, "pll2_p_ck"), (0b01, "pll3_r_ck"), (0b10, "per_ck")],
    pages: "RM0433 Rev 9 §8.7.22, p. 419; Figure 58, p. 364",
};

/// The timers RM0433 Rev 9 Table 60 (§8.5.8, pp. 354–356; the row p. 355) lists with no kernel
/// mux ("TIM[8:1][17:12]", control bits `-`): their clock is the timer clock
/// of their APB (Table 57). GPIO ports are not in Table 60 at all: a GPIO
/// has a bus clock and no kernel clock, so `gpio` names no peripheral here.
const NO_MUX_TIMERS: [&str; 14] = [
    "TIM1", "TIM2", "TIM3", "TIM4", "TIM5", "TIM6", "TIM7", "TIM8", "TIM12", "TIM13", "TIM14",
    "TIM15", "TIM16", "TIM17",
];
const NO_MUX_PAGES: &str = "RM0433 Rev 9 §8.5.8, Table 60, pp. 354–356";

/// The mux a peripheral's kernel clock comes through, or `None` when the
/// table holds no row for it.
pub fn selector(peripheral: &str) -> Option<&'static Selector> {
    Some(match peripheral {
        "SPI1" | "SPI2" | "SPI3" => &SPI123SEL,
        "SPI4" | "SPI5" => &SPI45SEL,
        "SPI6" => &SPI6SEL,
        "I2C1" | "I2C2" | "I2C3" => &I2C123SEL,
        "I2C4" => &I2C4SEL,
        "USART1" | "USART6" => &USART16SEL,
        "USART2" | "USART3" | "UART4" | "UART5" | "UART7" | "UART8" => &USART234578SEL,
        "ADC1" | "ADC2" | "ADC3" => &ADCSEL,
        _ => return None,
    })
}

/// The code a solved SPI kernel writes on `sel`: `per_ck` is SPI123SEL's
/// `100`, `hsi_ker_ck` SPI45SEL's and SPI6SEL's `011`, `pll1_q_ck` and the
/// APB clock each select's reset `000` (pp. 414, 418). `None` when the kernel
/// is no input of that select.
fn spi_code(sel: &Selector, kernel: SpiKernel) -> Option<u32> {
    match (sel.name, kernel) {
        ("SPI123SEL", SpiKernel::Pll1Q) => Some(0b000),
        ("SPI123SEL", SpiKernel::PerCk) => Some(0b100),
        ("SPI45SEL" | "SPI6SEL", SpiKernel::Pclk) => Some(0b000),
        ("SPI45SEL" | "SPI6SEL", SpiKernel::HsiKerCk) => Some(0b011),
        _ => None,
    }
}

// ---------------------------------------------------------------- PLL1's words

/// The HSI's nominal `hsi_ck`: 64 MHz at `HSIDIV`'s reset `00` (§8.7.2,
/// p. 385), the PLLs' input at `PLLSRC` `00` (§8.7.11, p. 399).
const HSI_HZ: u64 = 64_000_000;

/// `PLL1RGE` (`RCC_PLLCFGR` bits 3:2, §8.7.12, p. 402): the reference range
/// each code selects, in Hz, bounds included.
const PLL1RGE_HZ: [(u64, u64); 4] = [
    (1_000_000, 2_000_000),
    (2_000_000, 4_000_000),
    (4_000_000, 8_000_000),
    (8_000_000, 16_000_000),
];

/// One VCO range (`PLL1VCOSEL`, bit 1, p. 402) as DS12117 Rev 10 states it:
/// `fPLL_IN`, `fVCO_OUT`, and `fPLL_Q_OUT`'s maximum at VOS1, VOS2, VOS3.
/// No table gives Q at VOS0: revision Y has no VOS0 row, and revision V's
/// Table 136 states `fPLL_P_OUT` alone.
struct VcoRange {
    name: &'static str,
    in_hz: (u64, u64),
    vco_hz: (u64, u64),
    q_max_hz: [u64; 3],
    pages: &'static str,
}

/// `PLL1VCOSEL` `0`, wide: Table 39 (rev Y, pp. 129–130). The VCO's maximum
/// is revision Y's 836 MHz, the stricter: revision V's Table 136 (p. 234)
/// and RM0433 (p. 402) give 960, and no silicon revision is declared.
const WIDE: VcoRange = VcoRange {
    name: "wide",
    in_hz: (2_000_000, 16_000_000),
    vco_hz: (192_000_000, 836_000_000),
    q_max_hz: [400_000_000, 300_000_000, 200_000_000],
    pages: "DS12117 Rev 10 Table 39 (rev Y), pp. 129–130; Table 136 (rev V), p. 234",
};

/// `PLL1VCOSEL` `1`, medium: Table 40 (rev Y, p. 130) = Table 137 (rev V,
/// p. 235); the outputs' maximum is 210, 210 and 200 MHz.
const MEDIUM: VcoRange = VcoRange {
    name: "medium",
    in_hz: (1_000_000, 2_000_000),
    vco_hz: (150_000_000, 420_000_000),
    q_max_hz: [210_000_000, 210_000_000, 200_000_000],
    pages: "DS12117 Rev 10 Table 40 (rev Y), p. 130; Table 137 (rev V), p. 235",
};

fn field(v: u32, at: u32, width: u32) -> u32 {
    (v >> at) & ((1 << width) - 1)
}

/// Why a `kernel_switch`'s words do not run PLL1: the first fault, in the
/// order the words are read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "fault", rename_all = "snake_case")]
pub enum WordFault {
    /// `PLLSRC` (bits 1:0) is not `00`, the HSI (p. 399).
    PllsrcNotHsi { pllsrc: u32 },
    /// `DIVM1` (bits 9:4) is `0`, "prescaler disabled" (p. 399).
    Divm1Zero,
    /// HSI/`DIVM1` across the HSI's band leaves the `PLL1RGE` range (p. 402).
    ReferenceOutsidePll1rge {
        rge: u32,
        slowest_hz: u64,
        fastest_hz: u64,
        min_hz: u64,
        max_hz: u64,
    },
    /// HSI/`DIVM1` across the band leaves the VCO range's `fPLL_IN`.
    ReferenceOutsideFpllIn {
        vco: &'static str,
        slowest_hz: u64,
        fastest_hz: u64,
        min_hz: u64,
        max_hz: u64,
        pages: &'static str,
    },
    /// The VCO, reference × (`DIVN1` + 1) (p. 404), across the band leaves
    /// `fVCO_OUT`.
    VcoOutOfRange {
        vco: &'static str,
        slowest_hz: u64,
        fastest_hz: u64,
        min_hz: u64,
        max_hz: u64,
        pages: &'static str,
    },
    /// No `fPLL_Q_OUT` maximum is stated at this scale.
    NoQMaximumAtScale { vos: Vos },
    /// `pll1_q_ck`, VCO / (`DIVQ1` + 1) (p. 404), at the band's top is over
    /// `fPLL_Q_OUT`'s maximum at the clock's scale.
    QOverMaximum {
        vos: Vos,
        fastest_hz: u64,
        max_hz: u64,
        pages: &'static str,
    },
    /// `DIVQ1EN` (bit 17) is clear: `pll1_q_ck` is disabled (p. 401).
    Divq1enClear,
}

impl fmt::Display for WordFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PllsrcNotHsi { pllsrc } => write!(
                f,
                "RCC_PLLCKSELR.PLLSRC is {pllsrc:02b}, not 00 (HSI) (RM0433 Rev 9 §8.7.11, p. 399)"
            ),
            Self::Divm1Zero => f.write_str(
                "RCC_PLLCKSELR.DIVM1 is 0, the prescaler disabled (RM0433 Rev 9 §8.7.11, p. 399)",
            ),
            Self::ReferenceOutsidePll1rge {
                rge,
                slowest_hz,
                fastest_hz,
                min_hz,
                max_hz,
            } => write!(
                f,
                "the reference runs {slowest_hz} to {fastest_hz} Hz across the HSI's band, outside PLL1RGE {rge:02b}'s {min_hz} to {max_hz} Hz (RM0433 Rev 9 §8.7.12, p. 402)"
            ),
            Self::ReferenceOutsideFpllIn {
                vco,
                slowest_hz,
                fastest_hz,
                min_hz,
                max_hz,
                pages,
            } => write!(
                f,
                "the reference runs {slowest_hz} to {fastest_hz} Hz across the HSI's band, outside the {vco} VCO's fPLL_IN of {min_hz} to {max_hz} Hz ({pages})"
            ),
            Self::VcoOutOfRange {
                vco,
                slowest_hz,
                fastest_hz,
                min_hz,
                max_hz,
                pages,
            } => write!(
                f,
                "the VCO runs {slowest_hz} to {fastest_hz} Hz across the HSI's band, outside the {vco} range's {min_hz} to {max_hz} Hz ({pages})"
            ),
            Self::NoQMaximumAtScale { vos } => write!(
                f,
                "no fPLL_Q_OUT maximum is stated at {vos} (DS12117 Rev 10 Tables 39/40 rev Y have no VOS0 row; Table 136 rev V states P alone)"
            ),
            Self::QOverMaximum {
                vos,
                fastest_hz,
                max_hz,
                pages,
            } => write!(
                f,
                "pll1_q_ck runs up to {fastest_hz} Hz, over fPLL_Q_OUT's {max_hz} Hz at {vos} ({pages})"
            ),
            Self::Divq1enClear => f.write_str(
                "RCC_PLLCFGR.DIVQ1EN is clear: pll1_q_ck is disabled (RM0433 Rev 9 §8.7.12, p. 401)",
            ),
        }
    }
}

/// Whether PLL1 runs from the HSI on a `kernel_switch`'s words at `vos`,
/// across the HSI's whole band ([`Budget::HSI`]), with `pll1_q_ck` enabled.
/// Every comparison is a cross-multiplication in `u128`, bounds included.
fn judge_words(ks: &SolvedKernelSwitch, vos: Vos) -> Result<(), WordFault> {
    let pllsrc = field(ks.pllckselr, 0, 2);
    if pllsrc != 0b00 {
        return Err(WordFault::PllsrcNotHsi { pllsrc });
    }
    let m = u128::from(field(ks.pllckselr, 4, 6));
    if m == 0 {
        return Err(WordFault::Divm1Zero);
    }
    let slow = u128::from(Budget::HSI.slowest_hz(HSI_HZ));
    let fast = u128::from(Budget::HSI.fastest_hz(HSI_HZ));
    let hz = |x: u128, den: u128| u64::try_from(x / den).unwrap_or(u64::MAX);
    let inside = |lo: u64, hi: u64, scale: u128| {
        slow * scale >= u128::from(lo) * m && fast * scale <= u128::from(hi) * m
    };
    let rge = field(ks.pllcfgr, 2, 2);
    let (min_hz, max_hz) = PLL1RGE_HZ[rge as usize];
    if !inside(min_hz, max_hz, 1) {
        return Err(WordFault::ReferenceOutsidePll1rge {
            rge,
            slowest_hz: hz(slow, m),
            fastest_hz: hz(fast, m),
            min_hz,
            max_hz,
        });
    }
    let range = if field(ks.pllcfgr, 1, 1) == 0 {
        &WIDE
    } else {
        &MEDIUM
    };
    let (min_hz, max_hz) = range.in_hz;
    if !inside(min_hz, max_hz, 1) {
        return Err(WordFault::ReferenceOutsideFpllIn {
            vco: range.name,
            slowest_hz: hz(slow, m),
            fastest_hz: hz(fast, m),
            min_hz,
            max_hz,
            pages: range.pages,
        });
    }
    let n = u128::from(field(ks.pll1divr, 0, 9)) + 1;
    let (min_hz, max_hz) = range.vco_hz;
    if !inside(min_hz, max_hz, n) {
        return Err(WordFault::VcoOutOfRange {
            vco: range.name,
            slowest_hz: hz(slow * n, m),
            fastest_hz: hz(fast * n, m),
            min_hz,
            max_hz,
            pages: range.pages,
        });
    }
    let max_hz = match vos {
        Vos::Scale0 => return Err(WordFault::NoQMaximumAtScale { vos }),
        Vos::Scale1 => range.q_max_hz[0],
        Vos::Scale2 => range.q_max_hz[1],
        Vos::Scale3 => range.q_max_hz[2],
    };
    let q = u128::from(field(ks.pll1divr, 16, 7)) + 1;
    if fast * n > u128::from(max_hz) * m * q {
        return Err(WordFault::QOverMaximum {
            vos,
            fastest_hz: hz(fast * n, m * q),
            max_hz,
            pages: range.pages,
        });
    }
    if field(ks.pllcfgr, 17, 1) == 0 {
        return Err(WordFault::Divq1enClear);
    }
    Ok(())
}

// ---------------------------------------------------------------- what runs

/// When an input is asked about: during the select writes, or when the
/// peripheral starts, after them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Instant {
    Window,
    Start,
}

/// Why an input does not run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum NotRunning {
    /// Off after every system reset (PLLs, HSE, CSI, p. 371), or outside
    /// the chip (`I2S_CKIN`, the LSE), and started by nothing the topology
    /// states.
    NeverStarted,
    /// A PLL1 output on a board with no HSE tree and no `kernel_switch`:
    /// PLL1 is off after reset (p. 371) and nothing starts it.
    Pll1NotStarted,
    /// The `kernel_switch`'s words do not run PLL1.
    Pll1WordsInvalid { fault: WordFault },
    /// PLL1 from the HSI runs across the select writes alone, and is
    /// stopped after them (`clock.kernel_switch`).
    Pll1StoppedAfterWindow,
    /// PLL1 runs from the HSE tree with this output's enable clear
    /// (`RCC_PLLCFGR`, p. 401).
    OutputDisabled { field: &'static str },
}

impl fmt::Display for NotRunning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NeverStarted => f.write_str(
                "it is off after reset and the topology starts nothing that gives it (RM0433 Rev 9 p. 371)",
            ),
            Self::Pll1NotStarted => f.write_str(
                "PLL1 is off after reset, and the topology has no HSE tree and no kernel_switch to start it (RM0433 Rev 9 p. 371)",
            ),
            Self::Pll1WordsInvalid { fault } => {
                write!(f, "clock.kernel_switch does not run PLL1: {fault}")
            }
            Self::Pll1StoppedAfterWindow => f.write_str(
                "a PLL stopped while a peripheral uses it: PLL1 runs from the HSI across the select writes alone (clock.kernel_switch)",
            ),
            Self::OutputDisabled { field } => write!(
                f,
                "RCC_PLLCFGR.{field} is clear in clock.hse.pllcfgr (RM0433 Rev 9 §8.7.12, p. 401)"
            ),
        }
    }
}

/// The mux inputs that run from reset on every board: `hsi_ker_ck` (the
/// HSI: `HSION` resets 1, `RCC_CR` `0x0000 0001`, §8.7.2 pp. 383, 386);
/// `per_ck`, `hsi_ker_ck` at `CKPERSEL`'s reset `00` (§8.7.19, p. 412); and
/// the APB clocks. An ADC on `CKMODE` ≠ `00` takes no mux input: its
/// synchronous clock (`adc_sclk` on revision V, the AHB clock on revision Y;
/// §25.4.3, pp. 920–921) runs from reset too, and is not asked about.
const FROM_RESET: [&str; 5] = [
    "hsi_ker_ck",
    "per_ck",
    "rcc_pclk1",
    "rcc_pclk2",
    "rcc_pclk4",
];

/// `None` when `input` runs at `at`, else why not. Only `pll1_q_ck` among
/// the inputs off at reset is ever started (by an HSE tree or a
/// `kernel_switch`); no select this rule reads reaches `hse_ck`, the CSI,
/// the LSE, PLL2, PLL3 or `I2S_CKIN`, and each would be judged not running.
fn not_running(t: &SolvedTopology, input: &str, at: Instant) -> Option<NotRunning> {
    if FROM_RESET.contains(&input) {
        return None;
    }
    let c = &t.clock;
    match input {
        "pll1_q_ck" => {
            if let Some(h) = &c.hse {
                // PLL1 runs from the HSE tree from before any select to the
                // end; its words cannot change while it runs (pp. 399–404).
                return (field(h.pllcfgr, 17, 1) == 0)
                    .then_some(NotRunning::OutputDisabled { field: "DIVQ1EN" });
            }
            let Some(ks) = &c.kernel_switch else {
                return Some(NotRunning::Pll1NotStarted);
            };
            if let Err(fault) = judge_words(ks, c.vos) {
                return Some(NotRunning::Pll1WordsInvalid { fault });
            }
            (at == Instant::Start).then_some(NotRunning::Pll1StoppedAfterWindow)
        }
        _ => Some(NotRunning::NeverStarted),
    }
}

// ---------------------------------------------------------------- refusals

/// One input that does not run across the select writes, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeadInput {
    pub input: &'static str,
    pub why: NotRunning,
}

/// Why a peripheral's kernel clock cannot be judged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "why", rename_all = "snake_case")]
pub enum Unknown {
    /// The table holds no row for this enabled peripheral (`INV-06`).
    NoRow,
    /// An SPI peripheral with pins or devices and no `spi_controllers`
    /// entry, so no kernel is stated.
    NoSpiController,
    /// A solved SPI kernel that is no input of the peripheral's select.
    SpiKernelNotOnSelect {
        kernel: SpiKernel,
        selector: &'static str,
    },
    /// An ADC is enabled and `clock.adc` is absent.
    AdcClockAbsent,
    /// `clock.adc.ckmode` is not a 2-bit `CKMODE` code (p. 1039).
    CkmodeNotACode { ckmode: u32 },
    /// `clock.kernel_switch` beside `clock.hse`: PLL1 already runs from the
    /// HSE, and its words cannot be written while it runs (pp. 399–404).
    KernelSwitchBesideHse,
}

impl fmt::Display for Unknown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRow => f.write_str("the kernel-clock table holds no row for it"),
            Self::NoSpiController => {
                f.write_str("it is enabled and spi_controllers states no kernel for it")
            }
            Self::SpiKernelNotOnSelect { kernel, selector } => {
                write!(f, "its kernel {kernel:?} is no input of {selector}")
            }
            Self::AdcClockAbsent => f.write_str(
                "an ADC is enabled and clock.adc is absent (RM0433 Rev 9 §25.4.3, p. 920)",
            ),
            Self::CkmodeNotACode { ckmode } => write!(
                f,
                "clock.adc.ckmode {ckmode} is not a 2-bit CKMODE code (RM0433 Rev 9 §25.7.2, p. 1039)"
            ),
            Self::KernelSwitchBesideHse => f.write_str(
                "clock.kernel_switch is stated beside clock.hse: PLL1 runs from the HSE tree, and its words cannot be written while it runs (RM0433 Rev 9 §8.7.11–§8.7.13, pp. 399–404)",
            ),
        }
    }
}

fn dead_list(dead: &[DeadInput]) -> String {
    dead.iter()
        .map(|d| format!("{} ({})", d.input, d.why))
        .collect::<Vec<_>>()
        .join("; ")
}

/// A peripheral with no running kernel clock at its start, by the p. 371
/// rule. Each is its own kind ([`KernelClockRefusal::kind`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(tag = "kind")]
pub enum KernelClockRefusal {
    /// The select leaves its reset input while one of the two does not run,
    /// so no clock reaches the peripheral.
    #[error(
        "{peripheral}: {selector} cannot move from {from} to {to}, since {} does not run across the select writes, so no kernel clock reaches {peripheral} ({pages})",
        dead_list(dead)
    )]
    KernelClockDeadSwitch {
        peripheral: String,
        selector: &'static str,
        from: &'static str,
        to: &'static str,
        dead: Vec<DeadInput>,
        pages: String,
    },
    /// The input the peripheral starts on does not run then.
    #[error(
        "{peripheral}: {selector} gives it {input}, which does not run when {peripheral} starts: {why} ({pages})"
    )]
    KernelClockNotRunning {
        peripheral: String,
        selector: &'static str,
        input: &'static str,
        why: NotRunning,
        pages: String,
    },
    /// The peripheral's kernel clock cannot be judged.
    #[error("{peripheral}: its kernel clock cannot be judged: {why} ({pages})")]
    KernelClockUnknown {
        peripheral: String,
        why: Unknown,
        pages: String,
    },
}

impl KernelClockRefusal {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::KernelClockDeadSwitch { .. } => "KernelClockDeadSwitch",
            Self::KernelClockNotRunning { .. } => "KernelClockNotRunning",
            Self::KernelClockUnknown { .. } => "KernelClockUnknown",
        }
    }

    /// The peripheral it names (`PLL1` for [`Unknown::KernelSwitchBesideHse`]).
    pub fn peripheral(&self) -> &str {
        match self {
            Self::KernelClockDeadSwitch { peripheral, .. }
            | Self::KernelClockNotRunning { peripheral, .. }
            | Self::KernelClockUnknown { peripheral, .. } => peripheral,
        }
    }
}

fn unknown(peripheral: &str, why: Unknown, pages: &str) -> KernelClockRefusal {
    KernelClockRefusal::KernelClockUnknown {
        peripheral: peripheral.to_owned(),
        why,
        pages: pages.to_owned(),
    }
}

// ---------------------------------------------------------------- the rule

/// Every peripheral the topology enables that could take a kernel clock:
/// each SPI and I2C controller, each serial port's peripheral, each ADC
/// input's ADC but a `power.sense` net's, and every peripheral a pin or a
/// device is placed on. Sorted by name.
fn enabled(t: &SolvedTopology) -> BTreeSet<&str> {
    let sense = t.power.as_ref().map(|p| &p.sense);
    let adcs = t
        .adc_inputs
        .iter()
        .filter(|(net, _)| !sense.is_some_and(|s| s.contains_key(*net)))
        .map(|(_, a)| a.peripheral.as_str());
    t.spi_controllers
        .keys()
        .chain(t.i2c_timings.keys())
        .map(String::as_str)
        .chain(t.serial_ports.values().map(|p| p.peripheral.as_str()))
        .chain(adcs)
        .chain(t.assignments.values().map(|a| a.peripheral.as_str()))
        .chain(t.spi_devices.values().map(|d| d.peripheral.as_str()))
        .chain(t.i2c_devices.values().map(|d| d.peripheral.as_str()))
        .collect()
}

/// The rule (`AET-WO-4240e`): every enabled kernel-clocked peripheral of `t`
/// has a running kernel clock at its start. Empty when every one does;
/// otherwise one refusal per peripheral, sorted by peripheral, after a
/// `kernel_switch` stated beside an HSE tree, which is refused first.
///
/// A peripheral's mux enters at its reset input (code `0`). An input other
/// than that is reached only if both run across the select writes: with an
/// HSE tree, after it runs; with a `kernel_switch`, inside its window; else
/// on the reset sources alone. Then the input must run at the peripheral's
/// start, after the window, when a `kernel_switch`'s PLL1 is stopped.
pub fn check(t: &SolvedTopology) -> Vec<KernelClockRefusal> {
    let mut out = Vec::new();
    if t.clock.hse.is_some() && t.clock.kernel_switch.is_some() {
        out.push(unknown(
            "PLL1",
            Unknown::KernelSwitchBesideHse,
            "RM0433 Rev 9 §8.7.11–§8.7.13, pp. 399–404",
        ));
    }
    for p in enabled(t) {
        if NO_MUX_TIMERS.contains(&p) {
            continue;
        }
        let Some(sel) = selector(p) else {
            out.push(unknown(p, Unknown::NoRow, NO_MUX_PAGES));
            continue;
        };
        let code = if sel.name == ADCSEL.name {
            let Some(adc) = &t.clock.adc else {
                out.push(unknown(p, Unknown::AdcClockAbsent, sel.pages));
                continue;
            };
            match adc.ckmode {
                // `CKMODE` ≠ `00`: the synchronous clock, never the mux
                // (§25.4.3, pp. 920–921), and it runs from reset.
                1..=3 => continue,
                0 => 0b00,
                ckmode => {
                    out.push(unknown(p, Unknown::CkmodeNotACode { ckmode }, sel.pages));
                    continue;
                }
            }
        } else if sel.name.starts_with("SPI") {
            let Some(c) = t.spi_controllers.get(p) else {
                out.push(unknown(p, Unknown::NoSpiController, sel.pages));
                continue;
            };
            let Some(code) = spi_code(sel, c.kernel) else {
                let why = Unknown::SpiKernelNotOnSelect {
                    kernel: c.kernel,
                    selector: sel.name,
                };
                out.push(unknown(p, why, sel.pages));
                continue;
            };
            code
        } else {
            // I2C and USART selects are never written: their reset input.
            0
        };
        let from = sel.reset_input();
        let to = sel.input(code).expect("every code chosen is a row's");
        let pages = format!("{RULE_PAGES}; {}", sel.pages);
        if to != from {
            let dead: Vec<DeadInput> = [from, to]
                .into_iter()
                .filter_map(|input| {
                    not_running(t, input, Instant::Window).map(|why| DeadInput { input, why })
                })
                .collect();
            if !dead.is_empty() {
                out.push(KernelClockRefusal::KernelClockDeadSwitch {
                    peripheral: p.to_owned(),
                    selector: sel.name,
                    from,
                    to,
                    dead,
                    pages,
                });
                continue;
            }
        }
        if let Some(why) = not_running(t, to, Instant::Start) {
            out.push(KernelClockRefusal::KernelClockNotRunning {
                peripheral: p.to_owned(),
                selector: sel.name,
                input: to,
                why,
                pages,
            });
        }
    }
    out
}
