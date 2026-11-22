//! Gate (AET-WO-4200, F3): the flash's chip select handed to SPI1 before
//! `into_output` drove it, while it still floats.
use sentinel_01_board::mmio::Mmio;
use sentinel_01_board::{Board, Spi1Selects};

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let scale2 = board.pwr.scale2(&mut r, 10).unwrap();
    let mut rcc = board.rcc;
    let clock = rcc.reset_clock(&mut r, &scale2).unwrap();
    let kernels = rcc.select_spi_kernels(&mut r, &clock);
    let a = rcc.enable_gpioa(&mut r, board.gpioa, scale2).split(&mut r);
    let sck = a.pa5.into_spi1_sck(&mut r);
    let mosi = a.pa7.into_spi1_mosi(&mut r);
    let miso = a.pa6.into_spi1_miso(&mut r);
    let _spi = rcc
        .enable_spi1(&mut r, board.spi1, scale2, &kernels)
        .init(&mut r, &clock, sck, mosi, miso, Spi1Selects { pa0: a.pa0 });
}
