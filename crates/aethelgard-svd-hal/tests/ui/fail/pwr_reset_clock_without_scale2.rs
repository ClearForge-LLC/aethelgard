//! Gate (`AET-WO-4240v` §1.4): the reset clock read on the result of the
//! supply and voltage-scale bring-up, unchecked, in place of the token: the
//! core is at VOS Scale 2 only when the sequence returned `Ok`.
#![allow(dead_code)]

#[path = "../motors-pwm/board.rs"]
mod board;

use board::Board;
use board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let rcc = board.rcc;
    let unchecked = board.pwr.scale2(&mut r, 10);
    let _clock = rcc.reset_clock(&mut r, &unchecked);
}
