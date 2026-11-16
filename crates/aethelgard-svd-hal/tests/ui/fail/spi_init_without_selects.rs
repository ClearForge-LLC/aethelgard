//! Gate (AET-WO-4200, F3): SPI1 started without its chip selects, so a device
//! whose chip select was never driven could be selected by default.
use sentinel_01_board::Board;
use sentinel_01_board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let clock = rcc.reset_clock(&mut r).unwrap();
    let a = rcc.enable_gpioa(&mut r, board.gpioa).split(&mut r);
    let sck = a.pa5.into_spi1_sck(&mut r);
    let mosi = a.pa7.into_spi1_mosi(&mut r);
    let miso = a.pa6.into_spi1_miso(&mut r);
    let _spi = rcc.enable_spi1(&mut r, board.spi1).init(&mut r, &clock, sck, mosi, miso);
}
