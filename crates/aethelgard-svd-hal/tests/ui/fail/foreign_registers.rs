//! Firmware implementing `Registers` itself, to advance the typestates on a
//! stand-in while the real part stays unconfigured (adversarial E2): the trait
//! is sealed.
use sentinel_01_board::{Block, Registers};

struct Fake;

impl Registers for Fake {
    fn read(&mut self, _: Token, _: Block, _: usize) -> u32 {
        0
    }
    fn write(&mut self, _: Token, _: Block, _: usize, _: u32) {}
}

fn main() {}
