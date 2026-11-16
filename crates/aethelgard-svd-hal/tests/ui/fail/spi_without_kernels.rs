//! Gate (`AET-WO-4230` §1.1): SPI1 clocked before its kernel clock is
//! selected. `enable_spi1` takes the `SpiKernels` that only
//! `select_spi_kernels` makes, so the enable without it does not compile.
use sentinel_01_board::Board;
use sentinel_01_board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let _clock = rcc.reset_clock(&mut r).unwrap();
    let _spi = rcc.enable_spi1(&mut r, board.spi1);
}
