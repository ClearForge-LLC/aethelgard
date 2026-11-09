//! The valid driver: clock both ports and both peripherals, put every pin in
//! its solved function, start I2C1 at a speed under its ceiling and SPI1 as a
//! master, and drive the flash chip select. trybuild runs a passing case, and
//! on the host these are not the part's registers, so `firmware` is compiled
//! and not called.
use sentinel_01_board::Board;
use sentinel_01_board::i2c1_speed::Fast400k;
use sentinel_01_board::mmio::Mmio;

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
    let mut i2c = rcc.enable_i2c1(&mut r, board.i2c1).init::<Fast400k, _>(&mut r, scl, sda);
    let mut spi = rcc.enable_spi1(&mut r, board.spi1).init(&mut r, sck, mosi, miso);
    i2c.write_byte(&mut r, 0x76);
    let mut id = [0u8; 1];
    let _read: Result<usize, sentinel_01_board::I2cError> =
        i2c.transfer(&mut r, 0x76, &[0xd0], &mut id);
    cs.set_low(&mut r);
    spi.write_byte(&mut r, 0x9f);
    cs.set_high(&mut r);
}

fn main() {
    let _compiled: fn() = firmware;
}
