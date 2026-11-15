//! The valid motor-output driver (`AET-WO-4300a`), on the generated
//! `motors-pwm` board, included by path (`tests/motor_outputs.rs` guards its
//! bytes): clock port A and TIM1, put PA8 to PA11 in their TIM1 functions,
//! read the reset clock, and start the outputs. Compiled, not called, as in
//! `valid_driver.rs`.
#![allow(dead_code)]

#[path = "../motors-pwm/board.rs"]
mod board;

use board::Board;
use board::mmio::Mmio;

fn firmware() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let a = rcc.enable_gpioa(&mut r, board.gpioa).split();
    let ch1 = a.pa8.into_tim1_ch1(&mut r);
    let ch2 = a.pa9.into_tim1_ch2(&mut r);
    let ch3 = a.pa10.into_tim1_ch3(&mut r);
    let ch4 = a.pa11.into_tim1_ch4(&mut r);
    let tim1 = rcc.enable_tim1(&mut r, board.tim1);
    let Ok(clock) = rcc.reset_clock(&mut r) else {
        return;
    };
    let _running = tim1.start_outputs(&mut r, &clock, ch1, ch2, ch3, ch4);
}

fn main() {
    let _compiled: fn() = firmware;
}
