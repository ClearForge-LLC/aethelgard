//! Gate: a peripheral started without its RCC clock enable.
use sentinel_01_board::Board;
use sentinel_01_board::i2c1_speed::Fast400k;
use sentinel_01_board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let b = rcc.enable_gpiob(&mut r, board.gpiob).split();
    let scl = b.pb6.into_i2c1_scl(&mut r);
    let sda = b.pb7.into_i2c1_sda(&mut r);
    let _bus = board.i2c1.init::<Fast400k, _>(&mut r, scl, sda);
}
