//! A speed the generator did not admit, declared by firmware: the trait is
//! sealed.
use sentinel_01_board::i2c1_speed::AtMostCeiling;

struct Turbo;

impl AtMostCeiling for Turbo {
    const TIMINGR: u32 = 0;
}

fn main() {}
