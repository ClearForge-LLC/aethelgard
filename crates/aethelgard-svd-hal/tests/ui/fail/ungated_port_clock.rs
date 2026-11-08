//! Gate: a port's pins taken before the port's clock is enabled.
use sentinel_01_board::Board;

fn main() {
    let board = Board::take().unwrap();
    let _pins = board.gpiob.split();
}
