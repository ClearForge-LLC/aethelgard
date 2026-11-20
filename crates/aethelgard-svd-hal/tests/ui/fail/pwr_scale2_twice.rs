//! Gate (`AET-WO-4240v` §1.4): the sequence run twice. `PWR_CR3`'s supply
//! configuration is written once after power-on (RM0433 Rev 9 §6.8.4,
//! p. 308), so `scale2` consumes the one `Pwr`.
#![allow(dead_code)]

#[path = "../motors-pwm/board.rs"]
mod board;

use board::Board;
use board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let pwr = board.pwr;
    let _first = pwr.scale2(&mut r, 10);
    let _second = pwr.scale2(&mut r, 10);
}
