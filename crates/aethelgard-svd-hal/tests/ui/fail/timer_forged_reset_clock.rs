//! Gate (`AET-WO-4300a`): the reset-clock token made by hand, without the
//! check that reads the clock tree.
#![allow(dead_code)]

#[path = "../motors-pwm/board.rs"]
mod board;

use board::mmio::Mmio;
use board::{Board, ResetClock};

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let a = rcc.enable_gpioa(&mut r, board.gpioa).split();
    let ch1 = a.pa8.into_tim1_ch1(&mut r);
    let ch2 = a.pa9.into_tim1_ch2(&mut r);
    let ch3 = a.pa10.into_tim1_ch3(&mut r);
    let ch4 = a.pa11.into_tim1_ch4(&mut r);
    let tim1 = rcc.enable_tim1(&mut r, board.tim1);
    let forged = ResetClock { _private: () };
    let _running = tim1.start_outputs(&mut r, &forged, ch1, ch2, ch3, ch4);
}
