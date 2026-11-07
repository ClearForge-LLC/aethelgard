//! Beyond the five: a configured pin forged without configuring it. This is
//! the first thing the adversarial pass will try (WO §5.1).
use aet_1400_s1_probe::AlternateFunction;
use aet_1400_s1_probe::good::PB8;
use core::marker::PhantomData;

fn main() {
    let _scl: PB8<AlternateFunction<4>> = PB8 { _mode: PhantomData };
}
