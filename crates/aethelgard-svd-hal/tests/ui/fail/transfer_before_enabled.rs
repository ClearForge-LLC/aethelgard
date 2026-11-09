//! Gate (AET-WO-2202, ruling F1): a transfer on an I2C that has not been started.
use sentinel_01_board::mmio::Mmio;
use sentinel_01_board::Board;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let mut bus = rcc.enable_i2c1(&mut r, board.i2c1);
    let mut back = [0u8; 1];
    let _ = bus.transfer(&mut r, 0x76, &[0xd0], &mut back);
}
