//! A configured pin forged with a struct literal instead of configured.
use core::marker::PhantomData;
use sentinel_01_board::AlternateFunction;
use sentinel_01_board::PB6;

fn main() {
    let _scl: PB6<AlternateFunction<4>> = PB6 { _mode: PhantomData };
}
