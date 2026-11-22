//! Gate (`AET-WO-4300a`): a timer's outputs started on the result of the
//! reset-clock check, unchecked, in place of the token.
#![allow(dead_code)]

#[path = "../motors-pwm/board.rs"]
mod board;

use board::Board;
use board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let scale2 = board.pwr.scale2(&mut r, 10).unwrap();
    let mut rcc = board.rcc;
    let a = rcc.enable_gpioa(&mut r, board.gpioa, scale2).split(&mut r);
    let ch1 = a.pa8.into_tim1_ch1(&mut r);
    let ch2 = a.pa9.into_tim1_ch2(&mut r);
    let ch3 = a.pa10.into_tim1_ch3(&mut r);
    let ch4 = a.pa11.into_tim1_ch4(&mut r);
    let tim1 = rcc.enable_tim1(&mut r, board.tim1, scale2);
    let unchecked = rcc.reset_clock(&mut r, &scale2);
    let _running = tim1.start_outputs(&mut r, &unchecked, ch1, ch2, ch3, ch4);
}
