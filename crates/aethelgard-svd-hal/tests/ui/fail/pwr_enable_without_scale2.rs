//! Gate (`AET-WO-4240v` F2, ruled for `AET-WO-4240b`): a clock enable on the
//! result of the supply and voltage-scale bring-up, unchecked, in place of the
//! token. DS12117 Rev 10 p. 210 note 6 makes "at least one peripheral
//! enabled" the condition of VOS Scale 3's 50 MHz limit, so no peripheral is
//! clocked before the core is at Scale 2: every `enable_*` takes the `Scale2`
//! that only a successful `Pwr::scale2` makes.
#![allow(dead_code)]

#[path = "../motors-pwm/board.rs"]
mod board;

use board::Board;
use board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let unchecked = board.pwr.scale2(&mut r, 10);
    let _a = rcc.enable_gpioa(&mut r, board.gpioa, unchecked);
}
