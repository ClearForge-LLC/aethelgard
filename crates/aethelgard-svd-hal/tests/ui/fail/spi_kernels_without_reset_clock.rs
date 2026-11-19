//! Gate (`AET-WO-4230` §1.1, D6): the kernel clocks selected on the result of
//! the reset-clock check, unchecked: the selection was solved for the reset
//! clock, and the check must be read before it, once.
use sentinel_01_board::Board;
use sentinel_01_board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let scale2 = board.pwr.scale2(&mut r, 10).unwrap();
    let mut rcc = board.rcc;
    let unchecked = rcc.reset_clock(&mut r, &scale2);
    let _kernels = rcc.select_spi_kernels(&mut r, &unchecked);
}
