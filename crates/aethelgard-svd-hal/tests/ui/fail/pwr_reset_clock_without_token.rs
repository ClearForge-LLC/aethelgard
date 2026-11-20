//! Gate (`AET-WO-4240v` §1.4): the reset clock read with no `Scale2` at all,
//! before the supply and voltage-scale bring-up ran. Every I2C, SPI, USART
//! and timer start and the kernel selection take a `ResetClock`, so none of
//! them can precede the sequence either.
#![allow(dead_code)]

#[path = "../motors-pwm/board.rs"]
mod board;

use board::Board;
use board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let rcc = board.rcc;
    let _clock = rcc.reset_clock(&mut r);
}
