//! The one valid driver: clock the port and the peripheral, configure both
//! pins to their solved alternate function, start the bus at a speed under
//! the ceiling, write a byte. trybuild runs a passing case, and on the host
//! these are not the STM32's registers, so `firmware` is compiled, not called.
use aet_1400_s1_probe::good::Board;
use aet_1400_s1_probe::mmio::Mmio;
use aet_1400_s1_probe::speed::Fast400k;

fn firmware() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let (pb8, pb9) = rcc.enable_gpiob(&mut r, board.gpiob).split();
    let scl = pb8.into_alternate::<4, _>(&mut r);
    let sda = pb9.into_alternate::<4, _>(&mut r);
    let i2c = rcc.enable_i2c1(&mut r, board.i2c1);
    let mut bus = i2c.init::<Fast400k, _>(&mut r, scl, sda);
    bus.write_byte(&mut r, 0x42);
}

fn main() {
    let _compiled: fn() = firmware;
}
