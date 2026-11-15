//! Gate (`AET-WO-4300a`): a timer's outputs started with one channel pin
//! never put in its timer function.
#![allow(dead_code)]

#[path = "../motors-pwm/board.rs"]
mod board;

use board::Board;
use board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let a = rcc.enable_gpioa(&mut r, board.gpioa).split();
    let ch2 = a.pa9.into_tim1_ch2(&mut r);
    let ch3 = a.pa10.into_tim1_ch3(&mut r);
    let ch4 = a.pa11.into_tim1_ch4(&mut r);
    let tim1 = rcc.enable_tim1(&mut r, board.tim1);
    let Ok(clock) = rcc.reset_clock(&mut r) else {
        return;
    };
    let _running = tim1.start_outputs(&mut r, &clock, a.pa8, ch2, ch3, ch4);
}
