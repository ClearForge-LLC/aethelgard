//! The chip select driven before it is made an output.
use sentinel_01_board::Board;
use sentinel_01_board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let mut a = rcc.enable_gpioa(&mut r, board.gpioa).split();
    a.pa0.set_low(&mut r);
}
