//! The valid driver: clock both ports and both peripherals, put every pin in
//! its solved function, start I2C1 at a speed under its ceiling and SPI1 as a
//! master with its chip select, and read the flash's identity. trybuild runs a
//! passing case, and on the host these are not the part's registers, so
//! `firmware` is compiled and not called.
use sentinel_01_board::i2c1_speed::Fast400k;
use sentinel_01_board::mmio::Mmio;
use sentinel_01_board::{Board, Spi1Device, Spi1Selects, SpiError};

fn firmware() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let a = rcc.enable_gpioa(&mut r, board.gpioa).split();
    let b = rcc.enable_gpiob(&mut r, board.gpiob).split();
    let scl = b.pb6.into_i2c1_scl(&mut r);
    let sda = b.pb7.into_i2c1_sda(&mut r);
    let sck = a.pa5.into_spi1_sck(&mut r);
    let mosi = a.pa7.into_spi1_mosi(&mut r);
    let miso = a.pa6.into_spi1_miso(&mut r);
    let mut cs = a.pa0.into_output(&mut r);
    cs.set_high(&mut r);
    let mut i2c = rcc.enable_i2c1(&mut r, board.i2c1).init::<Fast400k, _>(&mut r, scl, sda);
    let mut spi = rcc
        .enable_spi1(&mut r, board.spi1)
        .init(&mut r, sck, mosi, miso, Spi1Selects { pa0: cs });
    i2c.write_byte(&mut r, 0x76);
    let mut id = [0u8; 1];
    let _read: Result<usize, sentinel_01_board::I2cError> =
        i2c.transfer(&mut r, 0x76, &[0xd0], &mut id);
    let mut jedec = [0u8; 3];
    let _read: Result<usize, SpiError> =
        spi.transfer(&mut r, Spi1Device::U4, &[0x9f], &mut jedec);
}

fn main() {
    let _compiled: fn() = firmware;
}
