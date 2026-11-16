//! Gate (`AET-WO-4230` §1.1): a `SpiKernels` made by hand, so SPI1 would be
//! clocked with no kernel clock selected. Its field is private.
use sentinel_01_board::mmio::Mmio;
use sentinel_01_board::{Board, SpiKernels};

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let kernels = SpiKernels { _private: () };
    let _spi = rcc.enable_spi1(&mut r, board.spi1, &kernels);
}
