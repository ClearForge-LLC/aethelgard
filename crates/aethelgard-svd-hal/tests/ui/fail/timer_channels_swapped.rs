//! Gate (`AET-WO-4300a`): CH1's and CH2's pins swapped, both configured into
//! their own timer functions.
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
    let a = rcc.enable_gpioa(&mut r, board.gpioa).split(&mut r);
    let ch1 = a.pa8.into_tim1_ch1(&mut r);
    let ch2 = a.pa9.into_tim1_ch2(&mut r);
    let ch3 = a.pa10.into_tim1_ch3(&mut r);
    let ch4 = a.pa11.into_tim1_ch4(&mut r);
    let tim1 = rcc.enable_tim1(&mut r, board.tim1);
    let Ok(clock) = rcc.reset_clock(&mut r, &scale2) else {
        return;
    };
    let _running = tim1.start_outputs(&mut r, &clock, ch2, ch1, ch3, ch4);
}
