//! Gate (`AET-WO-4230` D6): SPI1 started on the result of the reset-clock
//! check, unchecked, in place of the token: its kernel clock is the reset
//! clock's only when the check passed.
use sentinel_01_board::mmio::Mmio;
use sentinel_01_board::{Board, Spi1Selects};

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let unchecked = rcc.reset_clock(&mut r);
    let kernels = rcc.select_spi_kernels(&mut r, unchecked.as_ref().unwrap());
    let a = rcc.enable_gpioa(&mut r, board.gpioa).split(&mut r);
    let sck = a.pa5.into_spi1_sck(&mut r);
    let mosi = a.pa7.into_spi1_mosi(&mut r);
    let miso = a.pa6.into_spi1_miso(&mut r);
    let pa0 = a.pa0.into_output(&mut r);
    let spi = rcc.enable_spi1(&mut r, board.spi1, &kernels);
    let _bus = spi.init(&mut r, &unchecked, sck, mosi, miso, Spi1Selects { pa0 });
}
