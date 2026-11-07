//! Gate: the peripheral is started without its RCC clock enable.
use aet_1400_s1_probe::good::Board;
use aet_1400_s1_probe::mmio::Mmio;
use aet_1400_s1_probe::speed::Fast400k;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let (pb8, pb9) = rcc.enable_gpiob(&mut r, board.gpiob).split();
    let scl = pb8.into_alternate::<4, _>(&mut r);
    let sda = pb9.into_alternate::<4, _>(&mut r);
    let _bus = board.i2c1.init::<Fast400k, _>(&mut r, scl, sda);
}
