//! SCL and SDA swapped, both correctly configured.
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
    let i2c = rcc.enable_i2c1(&mut r, board.i2c1);
    let _bus = i2c.init::<Fast400k, _>(&mut r, sda, scl);
}
