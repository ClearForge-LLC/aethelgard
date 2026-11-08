//! A pin put in a function the solver did not give it: PB6 carries I2C1_SCL,
//! so it has no SPI method at all.
use sentinel_01_board::Board;
use sentinel_01_board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let b = rcc.enable_gpiob(&mut r, board.gpiob).split();
    let _sck = b.pb6.into_spi1_sck(&mut r);
}
