//! Firmware reaching a register around the typestates through the real
//! `Mmio` (adversarial E3): it cannot name, or make, the token every
//! `Registers` method takes.
use sentinel_01_board::mmio::Mmio;
use sentinel_01_board::{Block, Registers};

fn main() {
    let mut hw = unsafe { Mmio::new() };
    hw.write(sentinel_01_board::sealed::Token(()), Block::Rcc, 0xe8, 0);
}
