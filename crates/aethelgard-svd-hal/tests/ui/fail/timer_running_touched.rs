//! Gate (`AET-WO-4300a`): a running timer started again. After `CEN` nothing
//! reads or writes the timer: its running state has no method.
#![allow(dead_code)]

#[path = "../motors-pwm/board.rs"]
mod board;

use board::Board;
use board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let a = rcc.enable_gpioa(&mut r, board.gpioa).split(&mut r);
    let ch1 = a.pa8.into_tim1_ch1(&mut r);
    let ch2 = a.pa9.into_tim1_ch2(&mut r);
    let ch3 = a.pa10.into_tim1_ch3(&mut r);
    let ch4 = a.pa11.into_tim1_ch4(&mut r);
    let tim1 = rcc.enable_tim1(&mut r, board.tim1);
    let Ok(clock) = rcc.reset_clock(&mut r) else {
        return;
    };
    let running = tim1.start_outputs(&mut r, &clock, ch1, ch2, ch3, ch4);
    let _again = running.start_outputs(&mut r, &clock, ch1, ch2, ch3, ch4);
}
