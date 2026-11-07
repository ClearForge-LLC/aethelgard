//! Spike's choice 1: a pin put in an alternate function the solver did not
//! assign it (PB8 carries I2C4_SCL on AF6 in ST's table, but was solved to AF4).
use aet_1400_s1_probe::good::Board;
use aet_1400_s1_probe::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let (pb8, _pb9) = rcc.enable_gpiob(&mut r, board.gpiob).split();
    let _scl = pb8.into_alternate::<6, _>(&mut r);
}
