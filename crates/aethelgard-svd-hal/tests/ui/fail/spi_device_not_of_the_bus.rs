//! Gate (AET-WO-4200): a transfer on SPI1 names its device only by SPI1's own
//! device enum. The reference board has one SPI bus, so another bus's enum
//! does not exist here; an I2C address, as `I2c1::transfer` takes, stands in
//! for it. The generator's own test `each_bus_transfers_only_to_its_own_devices`
//! shows that each bus's transfer takes its own enum.
use sentinel_01_board::mmio::Mmio;
use sentinel_01_board::{Board, Spi1Selects};

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let a = rcc.enable_gpioa(&mut r, board.gpioa).split();
    let sck = a.pa5.into_spi1_sck(&mut r);
    let mosi = a.pa7.into_spi1_mosi(&mut r);
    let miso = a.pa6.into_spi1_miso(&mut r);
    let pa0 = a.pa0.into_output(&mut r);
    let mut spi = rcc
        .enable_spi1(&mut r, board.spi1)
        .init(&mut r, sck, mosi, miso, Spi1Selects { pa0 });
    let mut back = [0u8; 3];
    let _ = spi.transfer(&mut r, 0x76, &[0x9f], &mut back);
}
