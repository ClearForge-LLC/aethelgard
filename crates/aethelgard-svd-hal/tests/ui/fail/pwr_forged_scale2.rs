//! Gate (`AET-WO-4240v` §1.4): the `Scale2` token made by hand, without the
//! sequence that writes `PWR_CR3` and `PWR_D3CR` and waits for both bits.
#![allow(dead_code)]

#[path = "../motors-pwm/board.rs"]
mod board;

use board::mmio::Mmio;
use board::{Board, Scale2};

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let rcc = board.rcc;
    let forged = Scale2 { _private: () };
    let _clock = rcc.reset_clock(&mut r, &forged);
}
