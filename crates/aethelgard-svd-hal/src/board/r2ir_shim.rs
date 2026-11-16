//! R2-ir shim (`AET-WO-4230` round 2): what slice R2-ir adds to
//! `aethelgard_ir::silicon` (contract R3, R4, R5), by the contract's names,
//! so this crate builds and proves before that slice merges. Everything else
//! is `aethelgard_ir::silicon`'s own. Removed at merge (the PM): delete this
//! file and its `mod`, and `use aethelgard_ir::silicon;` again in `board.rs`.

use std::collections::BTreeMap;

use aethelgard_ir::SolvedSpiController;
pub use aethelgard_ir::silicon::*;

/// R2-ir shim: R4, `CFG2.AFCNTR` (bit 31, RM0433 Rev 9 §50.11.4, p. 2221)
/// in the rule's `CFG2`. The assertion trips once R2-ir's word carries it,
/// so a shim left behind cannot hide a rule mutant.
pub fn spi_words(
    controllers: &BTreeMap<String, SolvedSpiController>,
    peripheral: &str,
) -> Option<SpiWords> {
    let mut w = aethelgard_ir::silicon::spi_words(controllers, peripheral)?;
    debug_assert_eq!(w.cfg2 & (1 << 31), 0, "R2-ir has merged: remove r2ir_shim");
    w.cfg2 |= 1 << 31;
    Some(w)
}

/// R2-ir shim: R3's finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrrFinding {
    pub holds: bool,
}

/// R2-ir shim: R3, `2·|brr·baud − kernel| ≤ baud` and `brr ≥ 16`, exact.
pub fn check_brr(kernel_hz: u64, baud: u32, brr: u32) -> BrrFinding {
    let off = (u128::from(brr) * u128::from(baud)).abs_diff(u128::from(kernel_hz));
    BrrFinding {
        holds: 2 * off <= u128::from(baud) && brr >= 16,
    }
}

/// R2-ir shim: R5, `PUPDR` `10`, pull-down, on every motor-output pin (the
/// architect's ruling on Q-G; RM0433 Rev 9 §11.4.4, p. 542).
pub fn motor_output_pupd() -> u32 {
    0b10
}
