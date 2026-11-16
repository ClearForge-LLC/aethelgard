//! Gate (`AET-WO-4230` D10): `FLASH_OPTSR_CUR` is read once. `option_status`
//! takes the `Flash` by value, so a second read does not compile.
use sentinel_01_board::Board;
use sentinel_01_board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let flash = board.flash;
    let _first = flash.option_status(&mut r);
    let _second = flash.option_status(&mut r);
}
