//! Gate: I2C1 at 1 MHz, above sentinel-01's declared speed of 400 kHz
//! (`i2c_speeds`, AET-WO-1205).
use sentinel_01_board::Board;
use sentinel_01_board::i2c1_speed::FastPlus1M;
use sentinel_01_board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let b = rcc.enable_gpiob(&mut r, board.gpiob).split();
    let scl = b.pb6.into_i2c1_scl(&mut r);
    let sda = b.pb7.into_i2c1_sda(&mut r);
    let i2c = rcc.enable_i2c1(&mut r, board.i2c1);
    let _bus = i2c.init::<FastPlus1M, _>(&mut r, scl, sda);
}
