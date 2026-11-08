//! Gate, pending: I2C1 at 1 MHz over sentinel-01's 400 kHz ceiling. It moves
//! to `ui/fail/` when the solver's `i2c_ceilings` (AET-WO-1202) reaches the
//! generator; until then it compiles, and FEEDBACK records the gap.
use sentinel_01_board::Board;
use sentinel_01_board::i2c1_speed::FastPlus1M;
use sentinel_01_board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let b = rcc.enable_gpiob(&mut r, board.gpiob).split();
    let scl = b.pb6.into_i2c1_scl(&mut r);
    let sda = b.pb7.into_i2c1_sda(&mut r);
    let i2c = rcc.enable_i2c1(&mut r, board.i2c1);
    let _bus = i2c.init::<FastPlus1M, _>(&mut r, scl, sda);
}
