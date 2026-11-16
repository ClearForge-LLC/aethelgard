//! Gate: a port's pins taken before the port's clock is enabled.
use sentinel_01_board::Board;
use sentinel_01_board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let _pins = board.gpiob.split(&mut r);
}
