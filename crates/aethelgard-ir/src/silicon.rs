//! The independent silicon rules for the SPI and I2C controllers
//! (`AET-WO-4230` §1.1–§1.4, `RUL-20`): what RM0433 says each solved value
//! must be, judged by a formulation of its own.
//!
//! **Independence.** This module shares no function, constant or table with
//! `solve.rs`, with `aethelgard-svd-hal` or with `motor`/`serial`. Every
//! RM0433 and DS12117 fact it uses is written here again, on purpose, with
//! its citation; a fact both sides need is held twice so that an error in
//! one copy is a disagreement, not a shared blind spot. Where the solver
//! searches, this module checks: `MBR` by an inequality on the shifts and the
//! one below it, `TIMINGR` by decoding the word, testing each constraint as a
//! product inequality in exact integers, and testing that each field one
//! lower fails one (tightness) and that a smaller prescaler cannot fit.
//! It reads only the [`SolvedSpiController`] and [`SolvedI2cTiming`] types it
//! judges, and a USART's [`SolvedClock`] (`INV-01`); it never reads the
//! generated firmware (`INV-07`).
//!
//! belt-and-suspenders: every RM0433 bit position and reset value below
//! restates what `aethelgard-svd-hal` reads from the CMSIS header, written
//! from RM0433 so the generator's oracle takes its words from a second
//! source (contract D1, D4).
//!
//! **The worst case is judged** (`AET-WO-4230` §1.8, contract amendment 1).
//! Every kernel clock here is HSI-derived, and HSI strays by DS12117 Rev 10
//! Table 36 (rev Y) / Table 133 (rev V): ±0.3 MHz at 3.3 V 30 °C, −0.12/+0.03 %
//! over VDD, −2/+1 % over TJ −40 °C to TJmax, summed −25,894 / +14,984 ppm. A
//! minimum time and the SCL frequency are judged at the fastest kernel (the
//! ceiling of `nominal × (1 + 14,984 ppm)`), a maximum at the slowest (the
//! floor of `nominal × (1 − 25,894 ppm)`), so no bound is understated.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::clock::Budget;
use crate::{SolvedClock, SolvedI2cTiming, SolvedSpiController, SpiKernel};

/// The fastest a kernel clock of `nominal_hz` can run on the HSI, rounded up
/// ([`Budget::HSI`]): every SPI kernel this engine selects is `hsi_ker_ck`.
pub fn fast_hz(nominal_hz: u64) -> u64 {
    Budget::HSI.fastest_hz(nominal_hz)
}

/// The slowest a kernel clock of `nominal_hz` can run on the HSI, rounded
/// down.
pub fn slow_hz(nominal_hz: u64) -> u64 {
    Budget::HSI.slowest_hz(nominal_hz)
}

// ---------------------------------------------------------------- reset clock

/// HSI at reset: 64 MHz (DS12117 Rev 10 Table 36), divided by `HSIDIV`,
/// which resets to `00`, ÷1 (RM0433 Rev 9 §8.7.2, `RCC_CR`). So `hsi_ck` and
/// `hsi_ker_ck` (Figure 45, p. 338: `hsi_ker_ck` is `hsi_ck` gated by
/// `HSION`/`HSIKERON`) are 64 MHz; `sys_ck` is HSI (`RCC_CFGR.SW` resets to
/// HSI, §8.7.7) and every bus prescaler resets to ÷1 (§8.7.8–§8.7.10), so
/// `rcc_pclk1`, `rcc_pclk2` and `rcc_pclk4` (Figure 49, p. 351) are 64 MHz.
const RESET_HSI_KER_HZ: u64 = 64_000_000;
const RESET_PCLK_HZ: u64 = 64_000_000;

/// `RCC_D1CCIPR.CKPERSEL[1:0]` resets to `00`, `hsi_ker_ck` selected as
/// `per_ck` (RM0433 Rev 9 §8.7.19, p. 412; reset value `0x0000 0000`).
const RESET_CKPERSEL: u32 = 0b00;

/// `per_ck` at reset, from `CKPERSEL` (§8.7.19, p. 412): `00` `hsi_ker_ck`,
/// `01` `csi_ker_ck` (CSI is off at reset), `10` `hse_ck` (no crystal,
/// `mcu.hse_hz` 0), `11` disabled.
fn per_ck_at_reset() -> Option<u64> {
    (RESET_CKPERSEL == 0b00).then_some(RESET_HSI_KER_HZ)
}

/// A kernel-clock select field the SPI controllers use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiSelect {
    /// `RCC_D2CCIP1R.SPI123SEL[2:0]`, bits 14:12 (§8.7.20, p. 413–414).
    Spi123,
    /// `RCC_D2CCIP1R.SPI45SEL[2:0]`, bits 18:16 (§8.7.20, p. 413–414).
    Spi45,
    /// `RCC_D3CCIPR.SPI6SEL[2:0]`, bits 30:28 (§8.7.22, p. 418).
    Spi6,
}

/// `RCC_D2CCIP1R` at RCC offset 0x050 and `RCC_D3CCIPR` at 0x058, both
/// resetting to `0x0000 0000` (§8.7.20 p. 413, §8.7.22 p. 418).
pub const RCC_D2CCIP1R_OFFSET: u32 = 0x050;
pub const RCC_D3CCIPR_OFFSET: u32 = 0x058;

impl SpiSelect {
    /// The select field of `peripheral`, `None` for a name not `SPI1`–`SPI6`.
    pub fn of(peripheral: &str) -> Option<Self> {
        match peripheral {
            "SPI1" | "SPI2" | "SPI3" => Some(Self::Spi123),
            "SPI4" | "SPI5" => Some(Self::Spi45),
            "SPI6" => Some(Self::Spi6),
            _ => None,
        }
    }

    /// The register offset and the field's lowest bit.
    pub fn place(self) -> (u32, u32) {
        match self {
            Self::Spi123 => (RCC_D2CCIP1R_OFFSET, 12),
            Self::Spi45 => (RCC_D2CCIP1R_OFFSET, 16),
            Self::Spi6 => (RCC_D3CCIPR_OFFSET, 28),
        }
    }

    /// The code that selects `kernel` in this field, if it has one:
    /// `SPI123SEL` `100` = `per_ck`; `SPI45SEL` `011` = `hsi_ker_ck`;
    /// `SPI6SEL` `011` = `hsi_ker_ck`.
    pub fn code(self, kernel: SpiKernel) -> Option<u32> {
        match (self, kernel) {
            (Self::Spi123, SpiKernel::PerCk) => Some(0b100),
            (Self::Spi45 | Self::Spi6, SpiKernel::HsiKerCk) => Some(0b011),
            _ => None,
        }
    }
}

/// The clock a select field's `code` gives with every other RCC register at
/// its reset value, `None` where it gives none:
/// - `SPI123SEL` (p. 414): `000` `pll1_q_ck`, `001` `pll2_p_ck`, `010`
///   `pll3_p_ck` (every PLL is off at reset, `RCC_CR.PLLxON` 0, §8.7.2),
///   `011` `I2S_CKIN` (an external pin, never declared), `100` `per_ck`,
///   others reserved, the kernel clock disabled.
/// - `SPI45SEL` (p. 414) and `SPI6SEL` (p. 418): `000` the APB clock
///   (`rcc_pclk2`; `rcc_pclk4` for SPI6), `001`/`010` PLLs, `011`
///   `hsi_ker_ck`, `100` `csi_ker_ck` (CSI off), `101` `hse_ck` (no
///   crystal), others reserved.
pub fn kernel_at_reset(select: SpiSelect, code: u32) -> Option<u64> {
    match (select, code) {
        (SpiSelect::Spi123, 0b100) => per_ck_at_reset(),
        (SpiSelect::Spi45 | SpiSelect::Spi6, 0b000) => Some(RESET_PCLK_HZ),
        (SpiSelect::Spi45 | SpiSelect::Spi6, 0b011) => Some(RESET_HSI_KER_HZ),
        _ => None,
    }
}

/// The kernel clock `peripheral` has when its field selects `kernel` and the
/// rest of the clock tree is at reset; `None` if the field has no code for
/// `kernel` or the code gives no clock.
pub fn spi_kernel_hz(peripheral: &str, kernel: SpiKernel) -> Option<u64> {
    let select = SpiSelect::of(peripheral)?;
    kernel_at_reset(select, select.code(kernel)?)
}

// ---------------------------------------------------------------- SPI

/// What [`check_spi`] found for one controller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpiFinding {
    pub peripheral: String,
    pub kernel: SpiKernel,
    /// The kernel clock this rule derives, 0 where it derives none.
    pub kernel_hz: u64,
    /// [`fast_hz`] of it.
    pub fast_kernel_hz: u64,
    pub declared_hz: u32,
    pub mbr: u8,
    pub achieved_hz: u64,
    pub achieved_max_hz: u64,
    /// The topology's kernel and kernel clock are what the select field
    /// gives at reset.
    pub kernel_ok: bool,
    /// `MBR` is at most 7, the fastest kernel divided by `2^(MBR+1)` is at
    /// most the declared speed, and with `MBR − 1` it would be above it.
    pub mbr_ok: bool,
    /// `achieved_hz` and `achieved_max_hz` are the nominal and fastest
    /// kernel divided by `2^(MBR+1)`, rounded down.
    pub achieved_ok: bool,
    pub holds: bool,
}

/// The rule for one SPI controller (`AET-WO-4230` §1.1, §1.2). `MBR[2:0]`
/// divides the kernel clock by `2^(MBR+1)`, `000` ÷2 to `111` ÷256 (RM0433
/// Rev 9 §50.11.3, p. 2218). Checked by multiplication, never by a shift
/// loop: `fast ≤ declared · 2^(MBR+1)` and, for `MBR` > 0,
/// `fast > declared · 2^MBR`.
pub fn check_spi(peripheral: &str, c: &SolvedSpiController) -> SpiFinding {
    let kernel = spi_kernel_hz(peripheral, c.kernel);
    let kernel_hz = kernel.unwrap_or(0);
    let fast = fast_hz(kernel_hz);
    let kernel_ok = kernel == Some(c.kernel_hz);
    let declared = u128::from(c.declared_hz);
    let mbr_ok = c.mbr <= 7 && {
        let div = 1u128 << (c.mbr + 1);
        u128::from(fast) <= declared * div
            && (c.mbr == 0 || u128::from(fast) > declared * (div / 2))
    };
    // `q = floor(n / d)` exactly when `q·d ≤ n < (q+1)·d`.
    let floor_of = |q: u64, n: u64| {
        c.mbr <= 7 && {
            let d = 1u128 << (c.mbr + 1);
            u128::from(q) * d <= u128::from(n) && u128::from(n) < (u128::from(q) + 1) * d
        }
    };
    let achieved_ok = floor_of(c.achieved_hz, kernel_hz) && floor_of(c.achieved_max_hz, fast);
    SpiFinding {
        peripheral: peripheral.to_owned(),
        kernel: c.kernel,
        kernel_hz,
        fast_kernel_hz: fast,
        declared_hz: c.declared_hz,
        mbr: c.mbr,
        achieved_hz: c.achieved_hz,
        achieved_max_hz: c.achieved_max_hz,
        kernel_ok,
        mbr_ok,
        achieved_ok,
        holds: kernel_ok && mbr_ok && achieved_ok,
    }
}

/// `SPI_CR1` bits (RM0433 Rev 9 §50.11.1, p. 2215–2216): `SPE` bit 0,
/// `CSTART` bit 9, `SSI` bit 12; reset `0x0000 0000`.
const CR1_SPE: u32 = 1 << 0;
const CR1_CSTART: u32 = 1 << 9;
const CR1_SSI: u32 = 1 << 12;
/// `SPI_CFG1` (§50.11.3, p. 2218–2219): `MBR[2:0]` bits 30:28, `CRCSIZE[4:0]`
/// bits 20:16 resetting to `00111`, `DSIZE[4:0]` bits 4:0 resetting to
/// `00111` (8 bits); reset `0x0007 0007`. "If CRC calculation is disabled by
/// CRCEN=0, the CRCSIZE field must be kept at its default setting" (p. 2219).
const CFG1_MBR_SHIFT: u32 = 28;
const CFG1_CRCSIZE_RESET: u32 = 0b00111 << 16;
const CFG1_DSIZE_8BIT: u32 = 0b00111;
/// `SPI_CFG2` (§50.11.4, p. 2221–2222): `AFCNTR` bit 31, `SSM` bit 26,
/// `MASTER` bit 22; reset `0x0000 0000` (`CPOL` = `CPHA` = 0, mode 0; `COMM`
/// `00` full duplex; `SP` `000` Motorola; `SSOE` 0). `AFCNTR` (p. 2221,
/// rendered): "This bit is taken into account when SPE=0 only … 1: the
/// peripheral keeps always control of all associated GPIOs", so SCK and MOSI
/// stay driven while the chip select is low and `SPE` is not yet set or
/// already cleared; "This bit must be never used at slave mode", and every
/// controller here is a master (the architect's ruling on PR #161, comment
/// 6911424608, Q-D).
const CFG2_AFCNTR: u32 = 1 << 31;
const CFG2_SSM: u32 = 1 << 26;
const CFG2_MASTER: u32 = 1 << 22;

/// The whole words the bring-up writes for one SPI controller (contract D4,
/// with `CFG1`'s `CRCSIZE` at its reset value, p. 2219, and `CFG2`'s
/// `AFCNTR` set, round 2 R4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpiWords {
    /// `SSI`: with `SSM` set, NSS reads high, so a master raises no mode
    /// fault (p. 2215, p. 2221).
    pub cr1_idle: u32,
    /// `SSI | SPE`.
    pub cr1_enable: u32,
    /// `SSI | SPE | CSTART`.
    pub cr1_start: u32,
    /// `MBR << 28 | CRCSIZE 8 bits (reset) | DSIZE 8 bits`.
    pub cfg1: u32,
    /// `MASTER | SSM | AFCNTR`.
    pub cfg2: u32,
    /// The RCC offset of the controller's select register and the whole word
    /// written there: every solved controller's field in that register set
    /// to its code, every other field at reset (0).
    pub kernel_select: (u32, u32),
}

/// [`SpiWords`] for `peripheral`, one of `controllers`' keys; `None` if it is
/// not, or any controller's kernel has no code in its field.
pub fn spi_words(
    controllers: &BTreeMap<String, SolvedSpiController>,
    peripheral: &str,
) -> Option<SpiWords> {
    let c = controllers.get(peripheral)?;
    let (offset, _) = SpiSelect::of(peripheral)?.place();
    let mut word = 0u32;
    for (p, other) in controllers {
        let select = SpiSelect::of(p)?;
        let (at, shift) = select.place();
        let code = select.code(other.kernel)?;
        if at == offset {
            word |= code << shift;
        }
    }
    if c.mbr > 7 {
        return None;
    }
    Some(SpiWords {
        cr1_idle: CR1_SSI,
        cr1_enable: CR1_SSI | CR1_SPE,
        cr1_start: CR1_SSI | CR1_SPE | CR1_CSTART,
        cfg1: u32::from(c.mbr) << CFG1_MBR_SHIFT | CFG1_CRCSIZE_RESET | CFG1_DSIZE_8BIT,
        cfg2: CFG2_MASTER | CFG2_SSM | CFG2_AFCNTR,
        kernel_select: (offset, word),
    })
}

// ---------------------------------------------------------------- I2C

/// The kernel clock of `peripheral` with its select field at reset:
/// `RCC_D2CCIP2R.I2C123SEL[1:0]` resets to `00`, `rcc_pclk1` (RM0433 Rev 9
/// §8.7.21, p. 416), and `RCC_D3CCIPR.I2C4SEL[1:0]` to `00`, `rcc_pclk4`
/// (§8.7.22, p. 420); both 64 MHz at reset. `None` for a name not
/// `I2C1`–`I2C4`.
pub fn i2c_kernel_hz(peripheral: &str) -> Option<u64> {
    matches!(peripheral, "I2C1" | "I2C2" | "I2C3" | "I2C4").then_some(RESET_PCLK_HZ)
}

/// `I2C_CR1` as the bring-up writes it, `PE` clear (RM0433 Rev 9 §47.9.1,
/// p. 2001–2002; reset `0x0000 0000`): every interrupt enable (bits 7:1) 0;
/// `DNF[3:0]` (bits 11:8) `0000`, no digital filter; `ANFOFF` (bit 12) 0,
/// the analog filter on; `TXDMAEN`, `RXDMAEN` (14, 15) 0; `SBC` (16) 0;
/// `NOSTRETCH` (17) 0, which "must be kept cleared in controller mode";
/// `WUPEN`, `GCEN`, `SMBHEN`, `SMBDEN`, `ALERTEN`, `PECEN` (18–23) 0.
pub fn i2c_cr1() -> u32 {
    let pe = 0u32;
    let interrupts = 0u32 << 1;
    let dnf = 0u32 << 8;
    let anfoff = 0u32 << 12;
    let dma = 0u32 << 14;
    let nostretch = 0u32 << 17;
    pe | interrupts | dnf | anfoff | dma | nostretch
}

/// One I2C mode's limits, nanoseconds and hertz: RM0433 Rev 9 Table 390
/// (p. 1973, UM10204's clock timings) and Table 388 (p. 1958, data setup
/// and hold), and DS12117 Rev 10 Table 93 (rev Y, p. 188) / Table 191 (rev V,
/// p. 298), the least `i2c_ker_ck` with the analog filter on and `DNF` 0.
/// `tBUF` and `tSU;STA` equal `tLOW` in every mode, and `tHD;STA` and
/// `tSU;STO` equal `tHIGH`, so `SCLL` and `SCLH` meet them by meeting
/// `tLOW` and `tHIGH` (Table 390's note: `SCLL` sets `tBUF`, `tSU;STA`;
/// `SCLH` sets `tHD;STA`, `tSU;STO`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct I2cLimits {
    pub mode: &'static str,
    pub scl_hz: u64,
    pub t_low_ns: u64,
    pub t_high_ns: u64,
    pub tr_max_ns: u64,
    pub tf_max_ns: u64,
    pub t_su_dat_ns: u64,
    pub t_hd_dat_min_ns: u64,
    pub t_vd_dat_max_ns: u64,
    pub kernel_min_hz: u64,
}

/// The three modes, as data.
pub const I2C_LIMITS: [I2cLimits; 3] = [
    I2cLimits {
        mode: "Standard100k",
        scl_hz: 100_000,
        t_low_ns: 4_700,
        t_high_ns: 4_000,
        tr_max_ns: 1_000,
        tf_max_ns: 300,
        t_su_dat_ns: 250,
        t_hd_dat_min_ns: 0,
        t_vd_dat_max_ns: 3_450,
        kernel_min_hz: 2_000_000,
    },
    I2cLimits {
        mode: "Fast400k",
        scl_hz: 400_000,
        t_low_ns: 1_300,
        t_high_ns: 600,
        tr_max_ns: 300,
        tf_max_ns: 300,
        t_su_dat_ns: 100,
        t_hd_dat_min_ns: 0,
        t_vd_dat_max_ns: 900,
        kernel_min_hz: 8_000_000,
    },
    I2cLimits {
        mode: "FastPlus1M",
        scl_hz: 1_000_000,
        t_low_ns: 500,
        t_high_ns: 260,
        tr_max_ns: 120,
        tf_max_ns: 120,
        t_su_dat_ns: 50,
        t_hd_dat_min_ns: 0,
        t_vd_dat_max_ns: 450,
        kernel_min_hz: 17_000_000,
    },
];

/// The UM10204 mode a bus whose specification declares `declared_hz` starts
/// at (the adversarial pass's F3): the fastest of [`I2C_LIMITS`] whose SCL
/// frequency is at most the declared speed, so that no SCL the bus runs is
/// above what its devices declared (RM0433 Rev 9 Table 390 gives each mode's
/// timings at its own SCL frequency); `None` below Standard-mode's 100 kHz.
/// Chosen by the largest frequency that fits, never by a list's order.
pub fn i2c_start_mode(declared_hz: u64) -> Option<&'static str> {
    I2C_LIMITS
        .iter()
        .filter(|l| l.scl_hz <= declared_hz)
        .max_by_key(|l| l.scl_hz)
        .map(|l| l.mode)
}

/// `tAF`, the analog filter's delay: DS12117 Rev 10 Table 94 (rev Y,
/// p. 188) gives 50 to 260 ns, Table 192 (rev V, p. 298) 50 to 80 ns; the
/// revision is not declared, so the widest, 50 to 260.
const T_AF_MIN_NS: u64 = 50;
const T_AF_MAX_NS: u64 = 260;

/// The fewest `i2c_ker_ck` periods each of `tSYNC1` and `tSYNC2` lasts:
/// "two to three" (RM0433 Rev 9 §47.4.9, p. 1971). The slopes and the
/// filters are not credited: RM0433 Tables 391–393 notes 2–4 (p. 1983–1984)
/// give `tSYNC1 + tSYNC2`'s minimum as `4 × tI2CCLK`.
const SYNC_MIN_CYCLES: u128 = 2;

const NS_PER_S: u128 = 1_000_000_000;
const PS_PER_S: u128 = 1_000_000_000_000;

/// `I2C_TIMINGR` decoded (RM0433 Rev 9 §47.9.5, p. 2007): `PRESC[3:0]` bits
/// 31:28, bits 27:24 reserved, `SCLDEL[3:0]` 23:20, `SDADEL[3:0]` 19:16,
/// `SCLH[7:0]` 15:8, `SCLL[7:0]` 7:0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Timingr {
    pub presc: u32,
    pub scldel: u32,
    pub sdadel: u32,
    pub sclh: u32,
    pub scll: u32,
    /// Bits 27:24, which must be 0.
    pub reserved: u32,
}

impl Timingr {
    pub fn decode(word: u32) -> Self {
        Self {
            presc: word >> 28 & 0xF,
            reserved: word >> 24 & 0xF,
            scldel: word >> 20 & 0xF,
            sdadel: word >> 16 & 0xF,
            sclh: word >> 8 & 0xFF,
            scll: word & 0xFF,
        }
    }
}

/// Each constraint of one word, at the kernel bounds given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct I2cConstraints {
    /// `tSYNC1(min) + (SCLL+1)·tPRESC ≥ tLOW` at the fastest kernel.
    pub t_low: bool,
    /// `tSYNC2(min) + (SCLH+1)·tPRESC ≥ tHIGH` at the fastest kernel.
    pub t_high: bool,
    /// The shortest period, `tSYNC1 + tSYNC2 (min) + [(SCLH+1)+(SCLL+1)]·
    /// tPRESC` (p. 1971), at the fastest kernel, is at least `1 / f_mode`.
    pub scl_frequency: bool,
    /// `{[tr(max) + tSU;DAT(min)] / tPRESC} − 1 ≤ SCLDEL` (p. 1958).
    pub data_setup: bool,
    /// `{tf(max) + tHD;DAT(min) − tAF(min) − [(DNF+3)·tI2CCLK]} / tPRESC ≤
    /// SDADEL` (p. 1957), `DNF` 0.
    pub data_hold: bool,
    /// The slowest kernel is at least DS12117 Table 93's minimum.
    pub kernel_minimum: bool,
}

impl I2cConstraints {
    fn all(&self) -> bool {
        self.t_low
            && self.t_high
            && self.scl_frequency
            && self.data_setup
            && self.data_hold
            && self.kernel_minimum
    }
}

/// What [`check_timingr`] found for one word.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TimingFinding {
    pub mode: String,
    pub timingr: u32,
    pub fields: Timingr,
    pub fast_kernel_hz: u64,
    pub slow_kernel_hz: u64,
    /// `tLOW` and `tHIGH` at the fastest kernel and shortest `tSYNC`,
    /// picoseconds rounded down; the mode's minima beside them.
    pub t_low_ps: u64,
    pub t_low_min_ps: u64,
    pub t_high_ps: u64,
    pub t_high_min_ps: u64,
    /// The SCL frequency at the fastest kernel and shortest `tSYNC`, Hz
    /// rounded up; the mode's maximum beside it.
    pub scl_max_hz: u64,
    pub scl_mode_hz: u64,
    pub constraints: I2cConstraints,
    /// `SDADEL ≤ {tVD;DAT(max) − tr(max) − tAF(max) − [(DNF+4)·tI2CCLK]} /
    /// tPRESC` (p. 1958) at the slowest kernel. Reported, but it does not
    /// gate `holds`: RM0433 p. 1958, "This condition can be violated when
    /// NOSTRETCH = 0, because the device stretches SCL low to guarantee the
    /// set-up time", and [`i2c_cr1`] has `NOSTRETCH` 0. In Fast-mode Plus
    /// with `tAF(max)` 260 ns no `SDADEL` meets both it and `data_hold`.
    pub data_valid: bool,
    /// The word meets every constraint, and every field above 0, one lower,
    /// fails one.
    pub tight: bool,
    /// `PRESC` is 0, or one less cannot fit some constraint in its field.
    pub presc_least: bool,
    /// Bits 27:24 are 0.
    pub reserved_zero: bool,
    pub holds: bool,
}

/// The constraints of `f` for `limits` with the fastest kernel `fast`.
fn constraints(limits: &I2cLimits, fast: u64, kernel_slow: u64, f: &Timingr) -> I2cConstraints {
    let fast = u128::from(fast);
    let m = u128::from(f.presc) + 1;
    let (scll, sclh) = (u128::from(f.scll), u128::from(f.sclh));
    let ns = |t: u64| u128::from(t) * fast;
    // `cycles / fast ≥ t ns` as `cycles · 10⁹ ≥ t · fast`.
    let at_least = |cycles: u128, t_ns: u64| cycles * NS_PER_S >= ns(t_ns);
    let hold_ns = (limits.tf_max_ns + limits.t_hd_dat_min_ns).saturating_sub(T_AF_MIN_NS);
    I2cConstraints {
        t_low: at_least(SYNC_MIN_CYCLES + (scll + 1) * m, limits.t_low_ns),
        t_high: at_least(SYNC_MIN_CYCLES + (sclh + 1) * m, limits.t_high_ns),
        // `cycles / fast ≥ 1 / f_mode` as `cycles · f_mode ≥ fast`.
        scl_frequency: (2 * SYNC_MIN_CYCLES + (scll + sclh + 2) * m) * u128::from(limits.scl_hz)
            >= fast,
        data_setup: at_least(
            (u128::from(f.scldel) + 1) * m,
            limits.tr_max_ns + limits.t_su_dat_ns,
        ),
        // `SDADEL·m ≥ hold·fast/10⁹ − 3` as `(SDADEL·m + 3)·10⁹ ≥ hold·fast`.
        data_hold: at_least(u128::from(f.sdadel) * m + 3, hold_ns),
        kernel_minimum: kernel_slow >= limits.kernel_min_hz,
    }
}

/// `SDADEL·m ≤ valid·slow/10⁹ − 4` as `(SDADEL·m + 4)·10⁹ ≤ valid·slow`.
fn data_valid(limits: &I2cLimits, slow: u64, f: &Timingr) -> bool {
    let m = u128::from(f.presc) + 1;
    let valid = (limits.t_vd_dat_max_ns).checked_sub(limits.tr_max_ns + T_AF_MAX_NS);
    valid.is_some_and(|v| {
        (u128::from(f.sdadel) * m + 4) * NS_PER_S <= u128::from(v) * u128::from(slow)
    })
}

/// Whether any word at prescaler multiplier `m` (`PRESC + 1`) can meet the
/// lower bounds: each field at its largest fails some bound, else it fits.
/// Every constraint but `data_valid` only gains from a larger field.
fn fits(limits: &I2cLimits, fast: u64, slow: u64, m: u32) -> bool {
    let widest = Timingr {
        presc: m - 1,
        reserved: 0,
        scldel: 0xF,
        sdadel: 0xF,
        sclh: 0xFF,
        scll: 0xFF,
    };
    constraints(limits, fast, slow, &widest).all()
}

/// The limits of `mode`, by its name.
pub fn i2c_limits(mode: &str) -> Option<&'static I2cLimits> {
    I2C_LIMITS.iter().find(|l| l.mode == mode)
}

/// The rule for one `TIMINGR` word (`AET-WO-4230` §1.4, contract D3 and
/// amendment 1): for `mode` on a nominal kernel clock `kernel_hz`, judged
/// between [`fast_hz`] and [`slow_hz`] of it, the HSI's. `None` for an
/// unknown mode.
pub fn check_timingr(kernel_hz: u64, mode: &str, timingr: u32) -> Option<TimingFinding> {
    check_timingr_on(Budget::HSI, kernel_hz, mode, timingr)
}

/// [`check_timingr`] with the kernel clock's source deviating by `budget`
/// (`AET-WO-4240a` §1.4).
pub fn check_timingr_on(
    budget: Budget,
    kernel_hz: u64,
    mode: &str,
    timingr: u32,
) -> Option<TimingFinding> {
    judge_timingr(
        budget.fastest_hz(kernel_hz),
        budget.slowest_hz(kernel_hz),
        mode,
        timingr,
    )
}

/// [`check_timingr`] with the kernel bounds given: a test can judge RM0433's
/// worked examples (Tables 391–393) on their exact kernel clocks.
pub fn judge_timingr(fast: u64, slow: u64, mode: &str, timingr: u32) -> Option<TimingFinding> {
    let limits = i2c_limits(mode)?;
    let f = Timingr::decode(timingr);
    let c = constraints(limits, fast, slow, &f);
    let lower = |field: fn(&mut Timingr) -> &mut u32| {
        let mut g = f;
        let v = field(&mut g);
        if *v == 0 {
            return true;
        }
        *v -= 1;
        !constraints(limits, fast, slow, &g).all()
    };
    let tight = c.all()
        && lower(|t| &mut t.scll)
        && lower(|t| &mut t.sclh)
        && lower(|t| &mut t.scldel)
        && lower(|t| &mut t.sdadel);
    let presc_least = f.presc == 0 || !fits(limits, fast, slow, f.presc);
    let m = u128::from(f.presc) + 1;
    let ps = |cycles: u128| {
        u64::try_from(cycles * PS_PER_S / u128::from(fast.max(1))).unwrap_or(u64::MAX)
    };
    let period = 2 * SYNC_MIN_CYCLES + (u128::from(f.scll) + u128::from(f.sclh) + 2) * m;
    let reserved_zero = f.reserved == 0;
    Some(TimingFinding {
        mode: limits.mode.to_owned(),
        timingr,
        fields: f,
        fast_kernel_hz: fast,
        slow_kernel_hz: slow,
        t_low_ps: ps(SYNC_MIN_CYCLES + (u128::from(f.scll) + 1) * m),
        t_low_min_ps: limits.t_low_ns * 1_000,
        t_high_ps: ps(SYNC_MIN_CYCLES + (u128::from(f.sclh) + 1) * m),
        t_high_min_ps: limits.t_high_ns * 1_000,
        scl_max_hz: u64::try_from(u128::from(fast).div_ceil(period)).unwrap_or(u64::MAX),
        scl_mode_hz: limits.scl_hz,
        constraints: c,
        data_valid: data_valid(limits, slow, &f),
        tight,
        presc_least,
        reserved_zero,
        holds: c.all() && tight && presc_least && reserved_zero,
    })
}

/// What [`check_i2c`] found for one controller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct I2cFinding {
    pub peripheral: String,
    /// The kernel clock is [`i2c_kernel_hz`].
    pub kernel_ok: bool,
    /// `CR1` is [`i2c_cr1`].
    pub cr1_ok: bool,
    /// The modes are exactly the three of [`I2C_LIMITS`].
    pub modes_ok: bool,
    /// Each word, in mode-name order.
    pub timingr: Vec<TimingFinding>,
    pub holds: bool,
}

/// The rule for one I2C controller: its kernel clock, its `CR1` and every
/// `TIMINGR` word, on the HSI. A caller holding the topology judges it on its
/// own clock with [`check_i2c_on`]; the generator, which builds the HSI's
/// tree only until `AET-WO-4240b`, calls this.
pub fn check_i2c(peripheral: &str, t: &SolvedI2cTiming) -> I2cFinding {
    check_i2c_on(Budget::HSI, peripheral, t)
}

/// [`check_i2c`] with the kernel clock's source deviating by `budget`
/// (`AET-WO-4240a` §1.4): [`SolvedClock::budget`] of the topology's clock.
pub fn check_i2c_on(budget: Budget, peripheral: &str, t: &SolvedI2cTiming) -> I2cFinding {
    let kernel_ok = i2c_kernel_hz(peripheral) == Some(t.kernel_hz);
    let cr1_ok = t.cr1 == i2c_cr1();
    let modes_ok = t.timingr.len() == I2C_LIMITS.len()
        && I2C_LIMITS.iter().all(|l| t.timingr.contains_key(l.mode));
    let timingr: Vec<TimingFinding> = t
        .timingr
        .iter()
        .filter_map(|(mode, word)| check_timingr_on(budget, t.kernel_hz, mode, *word))
        .collect();
    let words_ok = timingr.len() == t.timingr.len() && timingr.iter().all(|f| f.holds);
    I2cFinding {
        peripheral: peripheral.to_owned(),
        kernel_ok,
        cr1_ok,
        modes_ok,
        timingr,
        holds: kernel_ok && cr1_ok && modes_ok && words_ok,
    }
}

// ---------------------------------------------------------------- USART

/// The kernel clock of a USART or UART with its select field at reset, from
/// `clock` (the adversarial pass's F1: this module's own, never the
/// solver's `serial::kernel_hz`). `RCC_D2CCIP2R` resets to `0x0000 0000`
/// (RM0433 Rev 9 §8.7.21, p. 416, rendered): `USART16SEL[2:0]` (bits 5:3)
/// `000` selects `rcc_pclk2` for USART1 and USART6, and `USART234578SEL[2:0]`
/// (bits 2:0) `000` selects `rcc_pclk1` for USART2, USART3, UART4, UART5,
/// UART7 and UART8 (p. 417, rendered). `rcc_pclk1` and `rcc_pclk2` are
/// `sys_ck` divided by `D1CPRE`, then `HPRE`, then `D2PPRE1` or `D2PPRE2`
/// (Figure 49, p. 351): here one division of `sys_ck` by the three
/// divisors' product, which floors to the same whole hertz. A divisor of 0
/// gives no clock; so does a peripheral not named above (LPUART1's select is
/// `RCC_D3CCIPR`'s).
pub fn usart_kernel_hz(clock: &SolvedClock, peripheral: &str) -> Option<u64> {
    let apb = match peripheral {
        "USART1" | "USART6" => clock.d2ppre2,
        "USART2" | "USART3" | "UART4" | "UART5" | "UART7" | "UART8" => clock.d2ppre1,
        _ => return None,
    };
    let product = u128::from(clock.d1cpre) * u128::from(clock.hpre) * u128::from(apb);
    (product > 0).then(|| u64::try_from(u128::from(clock.sysclk_hz) / product).unwrap_or(0))
}

/// `USART_BRR.BRR[15:0]`, a 16-bit field (RM0433 Rev 9 §48.8.5, p. 2082),
/// holding `USARTDIV`: "In case of oversampling by 16 and 8, USARTDIV must
/// be greater than or equal to 16" (§48.5.7, the note on p. 2035).
const BRR_MIN: u32 = 16;
const BRR_MAX: u32 = 0xFFFF;

/// What [`check_brr`] found for one USART: exact integers, nothing rounded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BrrFinding {
    /// The kernel clock this rule derives ([`usart_kernel_hz`]), 0 where it
    /// derives none; `BRR` is judged against it.
    pub kernel_hz: u64,
    /// The topology's kernel clock for the port.
    pub topology_kernel_hz: u64,
    /// The topology's kernel clock is the rule's.
    pub kernel_ok: bool,
    pub baud: u32,
    pub brr: u32,
    /// `|BRR · baud − kernel_hz|`: `baud` times how far `BRR` lies from
    /// `USARTDIV`, the exact `kernel_hz / baud`.
    pub residue: u64,
    /// `baud > 0` and `2 · residue ≤ baud`: `BRR` is within one half of
    /// `USARTDIV`, so it is a nearest whole `USARTDIV`.
    pub nearest: bool,
    /// `16 ≤ BRR ≤ 0xFFFF`.
    pub in_range: bool,
    pub holds: bool,
}

/// The rule for a USART's `BRR` (`AET-WO-4230` R3, the architect's ruling on
/// PR #161, comment 6911424608, Q-H). With oversampling by 16, `BRR` is
/// `USARTDIV` (RM0433 Rev 9 §48.5.7, p. 2034–2035: "In case of oversampling
/// by 16, … Tx/Rx baud = usart_ker_ck_pres / USARTDIV", "When OVER8 = 0,
/// BRR = USARTDIV"), so the word a kernel clock needs for `baud` is the
/// whole number nearest `kernel / baud`. The kernel is the rule's own,
/// [`usart_kernel_hz`] of `peripheral` on `clock`, and the topology's
/// `kernel_hz` must equal it (the adversarial pass's F1: a solver that
/// divided a wrong kernel consistently would otherwise hold). Checked by an
/// inequality, never by dividing: `2 · |BRR · baud − kernel| ≤ baud`, with
/// `BRR` in its field's range. It shares nothing with the solver's
/// `serial::solve_brr`, which rounds a quotient; at an exact half both
/// neighbours hold.
pub fn check_brr(
    peripheral: &str,
    clock: &SolvedClock,
    kernel_hz: u64,
    baud: u32,
    brr: u32,
) -> BrrFinding {
    let rule = usart_kernel_hz(clock, peripheral);
    let mut f = divider(rule.unwrap_or(0), baud, brr);
    f.topology_kernel_hz = kernel_hz;
    f.kernel_ok = rule == Some(kernel_hz);
    f.holds &= f.kernel_ok;
    f
}

/// [`check_brr`]'s inequality on a kernel of `kernel_hz`, taken as the
/// topology's.
fn divider(kernel_hz: u64, baud: u32, brr: u32) -> BrrFinding {
    let residue = (u128::from(brr) * u128::from(baud)).abs_diff(u128::from(kernel_hz));
    let nearest = baud > 0 && 2 * residue <= u128::from(baud);
    let in_range = (BRR_MIN..=BRR_MAX).contains(&brr);
    BrrFinding {
        kernel_hz,
        topology_kernel_hz: kernel_hz,
        kernel_ok: true,
        baud,
        brr,
        residue: u64::try_from(residue).unwrap_or(u64::MAX),
        nearest,
        in_range,
        holds: nearest && in_range,
    }
}

// ---------------------------------------------------------------- GPIO

/// `GPIOx_PUPDR.PUPDy[1:0]` (RM0433 Rev 9 §11.4.4, p. 542): `00` no
/// pull-up or pull-down, `01` pull-up, `10` pull-down, `11` reserved.
const PUPDR_PULL_DOWN: u32 = 0b10;

/// The `PUPDR` field of every motor-output pin: pull-down, written in the
/// port's first GPIO init as part of the whole word, so that a motor line is
/// held low from then until `start_outputs` (the architect's ruling on
/// PR #161, comment 6911424608, Q-G: "PUPDR pull-down (`10`) on every
/// motor-output pin in the first GPIO init, whole word").
pub fn motor_output_pupd() -> u32 {
    PUPDR_PULL_DOWN
}

/// The `PUPDR` field the solved topology implies for the pin of `net`
/// (`AET-WO-4400b` D3): pull-down on a motor output
/// ([`motor_output_pupd`]), and no internal pull on every other pin. The
/// specification declares no internal pull, and an I2C line's pull-up is
/// external (`analog_lines[..].pull_up_ohms`). A pull a board needs is a
/// declared fact, never a generator's default. The pinout check reads this;
/// no generator calls it (`RUL-20`).
pub fn implied_pupd(t: &crate::SolvedTopology, net: &str) -> u32 {
    if t.motor_outputs.contains_key(net) {
        motor_output_pupd()
    } else {
        0
    }
}

/// Whether the pin of `net` is open-drain (`GPIOx_OTYPER.OTy` = 1, RM0433
/// Rev 9 §11.4.2): exactly the solved I2C lines, the keys of `analog_lines`
/// (`AET-WO-4400b`, `4400a` A11). The pinout check reads this; no generator
/// calls it (`RUL-20`).
pub fn implied_open_drain(t: &crate::SolvedTopology, net: &str) -> bool {
    t.analog_lines.contains_key(net)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spi(kernel: SpiKernel, declared_hz: u32, mbr: u8) -> SolvedSpiController {
        // Hand values: 64 MHz nominal, 64,958,976 Hz fastest.
        SolvedSpiController {
            kernel,
            kernel_hz: 64_000_000,
            declared_hz,
            mbr,
            achieved_hz: 64_000_000 >> (mbr + 1),
            achieved_max_hz: 64_958_976 >> (mbr + 1),
        }
    }

    #[test]
    fn the_hsi_bounds_are_the_contract_s() {
        // 64 MHz × 1.014984 = 64,958,976 exactly; × 0.974106 = 62,342,784.
        assert_eq!(fast_hz(64_000_000), 64_958_976);
        assert_eq!(slow_hz(64_000_000), 62_342_784);
        // Rounded so the bound is never understated.
        assert_eq!(fast_hz(1), 2);
        assert_eq!(slow_hz(1), 0);
    }

    #[test]
    fn the_kernel_at_reset_is_hsi_through_per_ck_and_nothing_else() {
        assert_eq!(kernel_at_reset(SpiSelect::Spi123, 0b100), Some(64_000_000));
        for dead in [0b000, 0b001, 0b010, 0b011, 0b101, 0b110, 0b111] {
            assert_eq!(kernel_at_reset(SpiSelect::Spi123, dead), None, "{dead:03b}");
        }
        assert_eq!(kernel_at_reset(SpiSelect::Spi45, 0b011), Some(64_000_000));
        assert_eq!(kernel_at_reset(SpiSelect::Spi6, 0b011), Some(64_000_000));
        // The APB clock, `000`, runs at reset too (rcc_pclk2, rcc_pclk4).
        assert_eq!(kernel_at_reset(SpiSelect::Spi45, 0b000), Some(64_000_000));
        assert_eq!(kernel_at_reset(SpiSelect::Spi6, 0b000), Some(64_000_000));
        for dead in [0b001, 0b010, 0b100, 0b101, 0b110, 0b111] {
            assert_eq!(kernel_at_reset(SpiSelect::Spi45, dead), None, "{dead:03b}");
        }
        assert_eq!(spi_kernel_hz("SPI1", SpiKernel::PerCk), Some(64_000_000));
        assert_eq!(spi_kernel_hz("SPI1", SpiKernel::HsiKerCk), None);
        assert_eq!(spi_kernel_hz("SPI5", SpiKernel::HsiKerCk), Some(64_000_000));
        assert_eq!(spi_kernel_hz("SPI6", SpiKernel::PerCk), None);
        assert_eq!(spi_kernel_hz("SPI7", SpiKernel::PerCk), None);
    }

    #[test]
    fn mbr_is_the_fastest_at_or_below_the_declared_speed_at_the_fastest_kernel() {
        // (declared, MBR): 64,958,976 / 2^(MBR+1) ≤ declared, and / 2^MBR not.
        for (declared, mbr) in [
            (32_479_488, 0),
            (32_479_487, 1),
            (20_000_000, 1),
            (16_239_744, 1),
            (16_239_743, 2),
            (7_000_000, 3),
            (1_000_000, 6),
            (1_014_984, 5),
            (253_746, 7),
        ] {
            let c = spi(SpiKernel::PerCk, declared, mbr);
            let f = check_spi("SPI1", &c);
            assert!(f.holds, "{declared} {mbr}: {f:?}");
            for wrong in [mbr.wrapping_sub(1), mbr + 1] {
                if wrong <= 7 {
                    assert!(
                        !check_spi("SPI1", &spi(SpiKernel::PerCk, declared, wrong)).mbr_ok,
                        "{declared} {wrong}"
                    );
                }
            }
        }
        // 20 MHz: MBR 1, 16 MHz nominal, 16,239,744 Hz at the fastest.
        let f = check_spi("SPI1", &spi(SpiKernel::PerCk, 20_000_000, 1));
        assert_eq!((f.achieved_hz, f.achieved_max_hz), (16_000_000, 16_239_744));
        // 7 MHz: MBR 3, 4 MHz.
        assert_eq!(
            check_spi("SPI1", &spi(SpiKernel::PerCk, 7_000_000, 3)).achieved_hz,
            4_000_000
        );
        // Below 253,746 Hz no MBR reaches: 250 kHz and 253,745 Hz fail at 7.
        assert!(!check_spi("SPI1", &spi(SpiKernel::PerCk, 253_745, 7)).holds);
        assert!(!check_spi("SPI1", &spi(SpiKernel::PerCk, 250_000, 7)).holds);
        // MBR past 3 bits.
        assert!(!check_spi("SPI1", &spi(SpiKernel::PerCk, 1, 8)).holds);
    }

    #[test]
    fn the_kernel_and_achieved_rates_are_held() {
        let good = spi(SpiKernel::PerCk, 20_000_000, 1);
        assert!(check_spi("SPI1", &good).holds);
        // The wrong kernel for the field.
        let f = check_spi("SPI1", &spi(SpiKernel::HsiKerCk, 20_000_000, 1));
        assert!(!f.kernel_ok && !f.holds);
        assert!(check_spi("SPI4", &spi(SpiKernel::HsiKerCk, 20_000_000, 1)).holds);
        assert!(!check_spi("SPI4", &spi(SpiKernel::PerCk, 20_000_000, 1)).holds);
        let off_by_one = SolvedSpiController {
            achieved_hz: 16_000_001,
            ..good.clone()
        };
        assert!(!check_spi("SPI1", &off_by_one).achieved_ok);
        let nominal_as_max = SolvedSpiController {
            achieved_max_hz: 16_000_000,
            ..good.clone()
        };
        assert!(!check_spi("SPI1", &nominal_as_max).achieved_ok);
        let wrong_hz = SolvedSpiController {
            kernel_hz: 32_000_000,
            ..good
        };
        assert!(!check_spi("SPI1", &wrong_hz).kernel_ok);
    }

    #[test]
    fn the_spi_words_are_rm0433_s() {
        let mut all = BTreeMap::new();
        all.insert("SPI1".to_owned(), spi(SpiKernel::PerCk, 20_000_000, 1));
        let w = spi_words(&all, "SPI1").unwrap();
        assert_eq!(w.cr1_idle, 0x0000_1000);
        assert_eq!(w.cr1_enable, 0x0000_1001);
        assert_eq!(w.cr1_start, 0x0000_1201);
        assert_eq!(w.cfg1, 0x1007_0007);
        // MASTER (22), SSM (26) and AFCNTR (31), every other field 0.
        assert_eq!(w.cfg2, 0x8440_0000);
        assert_eq!(w.cfg2 >> 31, 1, "AFCNTR");
        // SPI123SEL = 100 at bits 14:12 of RCC_D2CCIP1R (0x050).
        assert_eq!(w.kernel_select, (0x050, 0x0000_4000));
        // SPI4 shares the register: SPI45SEL = 011 at bits 18:16, and SPI1's
        // field is kept in both words.
        all.insert("SPI4".to_owned(), spi(SpiKernel::HsiKerCk, 7_000_000, 3));
        assert_eq!(
            spi_words(&all, "SPI1").unwrap().kernel_select,
            (0x050, 0x0003_4000)
        );
        let w4 = spi_words(&all, "SPI4").unwrap();
        assert_eq!(w4.kernel_select, (0x050, 0x0003_4000));
        assert_eq!(w4.cfg1, 0x3007_0007);
        // SPI6: SPI6SEL = 011 at bits 30:28 of RCC_D3CCIPR (0x058).
        all.insert("SPI6".to_owned(), spi(SpiKernel::HsiKerCk, 20_000_000, 1));
        assert_eq!(
            spi_words(&all, "SPI6").unwrap().kernel_select,
            (0x058, 0x3000_0000)
        );
        assert_eq!(spi_words(&all, "SPI2"), None);
    }

    #[test]
    fn i2c_runs_on_the_reset_pclk_with_the_analog_filter_on() {
        for p in ["I2C1", "I2C2", "I2C3", "I2C4"] {
            assert_eq!(i2c_kernel_hz(p), Some(64_000_000), "{p}");
        }
        assert_eq!(i2c_kernel_hz("I2C5"), None);
        assert_eq!(i2c_cr1(), 0);
    }

    #[test]
    fn timingr_decodes_by_rm0433_s_bit_map() {
        let f = Timingr::decode(0x50D3_2A40);
        assert_eq!(
            (f.presc, f.reserved, f.scldel, f.sdadel, f.sclh, f.scll),
            (5, 0, 13, 3, 42, 64)
        );
        assert_eq!(Timingr::decode(0x0F00_0000).reserved, 0xF);
    }

    /// Hand-computed at 64 MHz, fastest 64,958,976 Hz and slowest 62,342,784
    /// Hz, with `tSYNC1`, `tSYNC2` two kernel cycles each and `tAF` 50 to
    /// 260 ns. Standard-mode: `SCLDEL` needs `(SCLDEL+1)(PRESC+1) ≥ 1250 ns ×
    /// 64.958976 MHz = 81.2`, so `PRESC` 5 (at 4 the most is 80); `SCLH`:
    /// `2 + 6(SCLH+1) ≥ 259.8`, 42; `SCLL`: `4 + 6(SCLL+SCLH+2) ≥ 649.6`
    /// (100 kHz), 64; `SDADEL`: `6·SDADEL + 3 ≥ 250 ns × f = 16.2`, 3.
    const HAND: [(&str, u32); 3] = [
        ("Standard100k", 0x50D3_2A40),
        ("Fast400k", 0x10C7_123C),
        ("FastPlus1M", 0x00B2_0E2D),
    ];

    #[test]
    fn the_hand_computed_words_hold_and_are_tight() {
        for (mode, word) in HAND {
            let f = check_timingr(64_000_000, mode, word).unwrap();
            assert!(f.holds, "{mode}: {f:#?}");
            assert!(f.scl_max_hz <= f.scl_mode_hz && f.t_low_ps >= f.t_low_min_ps);
        }
        let sm = check_timingr(64_000_000, "Standard100k", 0x50D3_2A40).unwrap();
        // tLOW = (2 + 65·6) / 64,958,976 Hz = 6.03458 µs; tHIGH 4.0025 µs;
        // the SCL at most 64,958,976 / 652 = 99,630.3 Hz.
        assert_eq!(
            (sm.t_low_ps, sm.t_high_ps, sm.scl_max_hz),
            (6_034_577, 4_002_526, 99_631)
        );
        assert!(sm.data_valid);
        // Fast-mode Plus: the data-valid maximum cannot be met beside the
        // hold minimum with tAF(max) 260 ns (reported, not gating).
        let fp = check_timingr(64_000_000, "FastPlus1M", 0x00B2_0E2D).unwrap();
        assert!(!fp.data_valid && fp.holds);
    }

    #[test]
    fn each_field_off_by_one_either_way_fails() {
        for (mode, word) in HAND {
            for (shift, mask, name) in [
                (0, 0xFF, "SCLL"),
                (8, 0xFF, "SCLH"),
                (16, 0xF, "SDADEL"),
                (20, 0xF, "SCLDEL"),
                (28, 0xF, "PRESC"),
            ] {
                let f = check_timingr(64_000_000, mode, word + (1 << shift)).unwrap();
                assert!(!f.holds, "{mode} {name}+1: {f:#?}");
                if word >> shift & mask != 0 {
                    let f = check_timingr(64_000_000, mode, word - (1 << shift)).unwrap();
                    assert!(!f.holds, "{mode} {name}-1: {f:#?}");
                }
            }
        }
    }

    #[test]
    fn a_larger_prescaler_resynthesized_tight_is_refused_as_not_least() {
        // Fast-mode at PRESC 2 (multiplier 3), each field the least: SCLDEL
        // 3·(SCLDEL+1) ≥ 25.98 → 8; SDADEL 3·S + 3 ≥ 16.24 → 5; SCLH
        // 2 + 3(SCLH+1) ≥ 38.98 → 12; SCLL: 4 + 3(SCLL+SCLH+2) ≥ 162.4 → 39.
        let w = 2 << 28 | 8 << 20 | 5 << 16 | 12 << 8 | 39;
        let f = check_timingr(64_000_000, "Fast400k", w).unwrap();
        assert!(f.constraints.t_low && f.tight, "{f:#?}");
        assert!(!f.presc_least && !f.holds);
        // Standard-mode: PRESC 4 fits at the nominal kernel (5 × 16 = 80
        // cycles is 1250 ns at 64 MHz) but not at the fastest.
        assert!(!fits(
            i2c_limits("Standard100k").unwrap(),
            64_958_976,
            62_342_784,
            5
        ));
        assert!(fits(
            i2c_limits("Standard100k").unwrap(),
            64_000_000,
            64_000_000,
            5
        ));
        assert!(fits(
            i2c_limits("Standard100k").unwrap(),
            64_958_976,
            62_342_784,
            6
        ));
    }

    #[test]
    fn rm0433_s_worked_examples_decode_and_are_judged() {
        // Table 392 (p. 1983), f_I2CCLK 16 MHz, exact (no HSI spread): Fm+
        // 1000 kHz, PRESC 0, SCLL 0x4, SCLH 0x2, SDADEL 0, SCLDEL 0x2.
        let w = 0x0020_0204;
        let f = judge_timingr(16_000_000, 16_000_000, "FastPlus1M", w).unwrap();
        assert_eq!(
            (
                f.fields.presc,
                f.fields.scll,
                f.fields.sclh,
                f.fields.sdadel,
                f.fields.scldel
            ),
            (0, 4, 2, 0, 2)
        );
        // tLOW: (2 + 5) / 16 MHz = 437.5 ns < 500; tHIGH (2 + 3) = 312.5 ns ≥
        // 260; the SCL with the shortest tSYNC (4 cycles, note 4) is 16 MHz /
        // (4 + 5 + 3) = 1.3333 MHz > 1 MHz: the example's 1.0 µs assumes
        // 500 ns of tSYNC. Setup: 3 × 62.5 = 187.5 ns ≥ 170. Hold: (0 + 3) × 62.5 =
        // 187.5 ns ≥ 70.
        assert_eq!(
            (f.t_low_ps, f.t_high_ps, f.scl_max_hz),
            (437_500, 312_500, 1_333_334)
        );
        assert!(!f.constraints.t_low && f.constraints.t_high && !f.constraints.scl_frequency);
        assert!(f.constraints.data_setup && f.constraints.data_hold && !f.holds);
        // Table 391 (p. 1983), 8 MHz, Sm 100 kHz: PRESC 1, SCLL 0x13, SCLH
        // 0xF, SDADEL 2, SCLDEL 4. tLOW (2 + 40) × 125 = 5.25 µs; tHIGH (2 +
        // 32) × 125 = 4.25 µs; period (4 + 72) × 125 = 9.5 µs, 105.3 kHz.
        let w = 0x1042_0F13;
        let f = judge_timingr(8_000_000, 8_000_000, "Standard100k", w).unwrap();
        assert_eq!(
            (f.t_low_ps, f.t_high_ps, f.scl_max_hz),
            (5_250_000, 4_250_000, 105_264)
        );
        assert!(f.constraints.t_low && f.constraints.t_high && !f.constraints.scl_frequency);
        // SCLDEL: (4 + 1) × 2 × 125 = 1250 ns, exactly tr + tSU;DAT.
        assert!(f.constraints.data_setup && f.constraints.data_hold && f.data_valid);
        assert!(!f.tight && !f.holds);
        // Table 393 (p. 1984), 48 MHz, Fm 400 kHz: PRESC 5, SCLL 9, SCLH 3,
        // SDADEL 3, SCLDEL 3. tLOW (2 + 60)/48 MHz = 1291.7 ns < 1300.
        let w = 0x5033_0309;
        let f = judge_timingr(48_000_000, 48_000_000, "Fast400k", w).unwrap();
        assert_eq!((f.t_low_ps, f.t_high_ps), (1_291_666, 541_666));
        assert!(!f.constraints.t_low && !f.constraints.t_high && !f.holds);
    }

    #[test]
    fn an_unknown_mode_and_a_reserved_bit_are_refused() {
        assert!(check_timingr(64_000_000, "HighSpeed3M4", 0x50D3_2A40).is_none());
        let f = check_timingr(64_000_000, "Standard100k", 0x50D3_2A40 | 1 << 24).unwrap();
        assert!(!f.reserved_zero && !f.holds);
        // A kernel below DS12117 Table 93's 17 MHz fails Fm+ whatever the word.
        let f = check_timingr(16_000_000, "FastPlus1M", 0x0020_0204).unwrap();
        assert!(!f.constraints.kernel_minimum);
    }

    #[test]
    fn an_i2c_controller_is_held_whole() {
        let t = SolvedI2cTiming {
            kernel_hz: 64_000_000,
            cr1: 0,
            timingr: HAND.iter().map(|(m, w)| ((*m).to_owned(), *w)).collect(),
        };
        assert!(check_i2c("I2C1", &t).holds);
        let mut missing = t.clone();
        missing.timingr.remove("FastPlus1M");
        assert!(!check_i2c("I2C1", &missing).modes_ok);
        let analog_off = SolvedI2cTiming {
            cr1: 1 << 12,
            ..t.clone()
        };
        assert!(!check_i2c("I2C1", &analog_off).holds);
        assert!(!check_i2c("I2C5", &t).holds);
    }

    #[test]
    fn brr_is_the_nearest_whole_usartdiv_by_the_inequality() {
        // 64 MHz / 115200 = 555.56: 556 · 115200 = 64,051,200, 51,200 over,
        // 2 · 51,200 = 102,400 ≤ 115,200. 555 is 64,000 under (128,000 >
        // 115,200) and 557 166,400 over: both fail.
        let f = divider(64_000_000, 115_200, 556);
        assert_eq!(
            (f.residue, f.nearest, f.in_range, f.holds),
            (51_200, true, true, true)
        );
        for wrong in [555, 557] {
            assert!(!divider(64_000_000, 115_200, wrong).holds, "{wrong}");
        }
        assert_eq!(divider(64_000_000, 115_200, 555).residue, 64_000);
        // 420000: 152 · 420000 = 63,840,000, 160,000 under, 320,000 ≤
        // 420,000; 151 is 580,000 under and 153 260,000 over (520,000).
        let f = divider(64_000_000, 420_000, 152);
        assert_eq!((f.residue, f.holds), (160_000, true));
        assert_eq!(divider(64_000_000, 420_000, 153).residue, 260_000);
        for wrong in [151, 153] {
            assert!(!divider(64_000_000, 420_000, wrong).holds, "{wrong}");
        }
        // 100000: 640 exactly; 639 and 641 are 100,000 off, 200,000 > 100,000.
        assert_eq!(divider(64_000_000, 100_000, 640).residue, 0);
        assert!(!divider(64_000_000, 100_000, 639).holds);
        assert!(!divider(64_000_000, 100_000, 641).holds);
    }

    #[test]
    fn at_an_exact_half_both_neighbours_hold() {
        // 1,024,000 baud: USARTDIV = 62.5. 62 · 1,024,000 = 63,488,000 and
        // 63 · 1,024,000 = 64,512,000, each 512,000 away: 2 · 512,000 =
        // 1,024,000, the baud, so `≤` holds for both and `<` for neither.
        for brr in [62, 63] {
            let f = divider(64_000_000, 1_024_000, brr);
            assert_eq!((f.residue, f.holds), (512_000, true), "{brr}");
        }
        assert!(!divider(64_000_000, 1_024_000, 61).holds);
        assert!(!divider(64_000_000, 1_024_000, 64).holds);
    }

    #[test]
    fn brr_is_16_to_0xffff() {
        // 4 Mbaud: 16 exactly, the least BRR the field takes.
        let f = divider(64_000_000, 4_000_000, 16);
        assert_eq!((f.residue, f.in_range, f.holds), (0, true, true));
        // 4,266,667 baud: 15 · 4,266,667 = 64,000,005, 5 over, nearest, but
        // below 16.
        let f = divider(64_000_000, 4_266_667, 15);
        assert_eq!(
            (f.residue, f.nearest, f.in_range, f.holds),
            (5, true, false, false)
        );
        // 0xFFFF holds; 0x10000 is nearest (1000 · 65,536 = 65,536,000) but
        // past the 16-bit field.
        assert!(divider(65_535_000, 1_000, 0xFFFF).holds);
        let f = divider(65_536_000, 1_000, 0x1_0000);
        assert_eq!((f.residue, f.nearest, f.in_range), (0, true, false));
        // A zero baud has no USARTDIV, even on a zero kernel.
        assert!(!divider(0, 0, 16).holds);
        assert!(!divider(64_000_000, 0, 16).holds);
        // No overflow at the extremes.
        let f = divider(u64::MAX, u32::MAX, u32::MAX);
        assert!(!f.holds);
    }

    #[test]
    fn a_bus_starts_at_the_fastest_mode_at_or_under_its_declared_speed() {
        for (hz, mode) in [
            (99_999, None),
            (100_000, Some("Standard100k")),
            (399_999, Some("Standard100k")),
            (400_000, Some("Fast400k")),
            (999_999, Some("Fast400k")),
            (1_000_000, Some("FastPlus1M")),
            (3_400_000, Some("FastPlus1M")),
        ] {
            assert_eq!(i2c_start_mode(hz), mode, "{hz}");
        }
    }

    #[test]
    fn the_usart_kernel_is_the_apb_clock_its_select_resets_to() {
        // RM0433 Rev 9 §8.7.21: USART1 and USART6 on rcc_pclk2, the rest on
        // rcc_pclk1; 64 MHz each at the reset clock.
        let reset = SolvedClock::reset();
        for p in [
            "USART1", "USART2", "USART3", "UART4", "UART5", "USART6", "UART7", "UART8",
        ] {
            assert_eq!(usart_kernel_hz(&reset, p), Some(64_000_000), "{p}");
        }
        assert_eq!(usart_kernel_hz(&reset, "LPUART1"), None);
        assert_eq!(usart_kernel_hz(&reset, "USART9"), None);
        // Each APB prescaler reaches only its own USARTs.
        let pclk2_half = SolvedClock {
            d2ppre2: 2,
            ..SolvedClock::reset()
        };
        assert_eq!(usart_kernel_hz(&pclk2_half, "USART1"), Some(32_000_000));
        assert_eq!(usart_kernel_hz(&pclk2_half, "USART6"), Some(32_000_000));
        assert_eq!(usart_kernel_hz(&pclk2_half, "USART2"), Some(64_000_000));
        let pclk1_quarter = SolvedClock {
            d2ppre1: 4,
            ..SolvedClock::reset()
        };
        assert_eq!(usart_kernel_hz(&pclk1_quarter, "UART8"), Some(16_000_000));
        assert_eq!(usart_kernel_hz(&pclk1_quarter, "USART1"), Some(64_000_000));
        let zero = SolvedClock {
            hpre: 0,
            ..SolvedClock::reset()
        };
        assert_eq!(usart_kernel_hz(&zero, "USART1"), None);
        // No overflow at the extremes: the divisors' product exceeds u64.
        let huge = SolvedClock {
            d1cpre: u32::MAX,
            hpre: u32::MAX,
            d2ppre2: u32::MAX,
            ..SolvedClock::reset()
        };
        assert_eq!(usart_kernel_hz(&huge, "USART1"), Some(0));
    }

    #[test]
    fn brr_is_judged_on_the_rule_s_kernel_and_the_topology_s_must_be_it() {
        let reset = SolvedClock::reset();
        let f = check_brr("USART1", &reset, 64_000_000, 115_200, 556);
        assert_eq!(
            (f.kernel_hz, f.topology_kernel_hz, f.kernel_ok, f.holds),
            (64_000_000, 64_000_000, true, true)
        );
        // The adversarial pass's M1: the kernel halved and divided
        // consistently, 278 the nearest divider of 32 MHz at 115,200 baud,
        // which is 230,400 baud on the 64 MHz silicon runs.
        let f = check_brr("USART1", &reset, 32_000_000, 115_200, 278);
        assert!(!f.kernel_ok && !f.nearest && !f.holds, "{f:?}");
        assert_eq!(
            (f.kernel_hz, f.topology_kernel_hz),
            (64_000_000, 32_000_000)
        );
        // A peripheral with no kernel at reset holds nothing.
        let f = check_brr("LPUART1", &reset, 64_000_000, 115_200, 556);
        assert!(!f.kernel_ok && !f.holds && f.kernel_hz == 0, "{f:?}");
    }

    #[test]
    fn a_motor_output_pin_is_pulled_down() {
        assert_eq!(motor_output_pupd(), 0b10);
    }
}
