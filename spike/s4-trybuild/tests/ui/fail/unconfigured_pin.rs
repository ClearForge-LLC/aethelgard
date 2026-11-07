//! Gate: a pin still unconfigured is handed to the peripheral.
use aet_1400_s1_probe::good::Board;
use aet_1400_s1_probe::mmio::Mmio;
use aet_1400_s1_probe::speed::Fast400k;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let (pb8, pb9) = rcc.enable_gpiob(&mut r, board.gpiob).split();
    let sda = pb9.into_alternate::<4, _>(&mut r);
    let i2c = rcc.enable_i2c1(&mut r, board.i2c1);
    let _bus = i2c.init::<Fast400k, _>(&mut r, pb8, sda);
}
