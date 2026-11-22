//! Gate (`AET-WO-4300b`): a USART configured on the result of the
//! reset-clock check, unchecked, in place of the token: its `BRR` was solved
//! for the reset clock.
#![allow(dead_code)]

#[path = "../serial-four/board.rs"]
mod board;

use board::Board;
use board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let scale2 = board.pwr.scale2(&mut r, 10).unwrap();
    let mut rcc = board.rcc;
    let a = rcc.enable_gpioa(&mut r, board.gpioa, scale2).split(&mut r);
    let tx = a.pa9.into_usart1_tx(&mut r);
    let rx = a.pa10.into_usart1_rx(&mut r);
    let usart1 = rcc.enable_usart1(&mut r, board.usart1, scale2);
    let unchecked = rcc.reset_clock(&mut r, &scale2);
    let _usart1 = usart1.configure(&mut r, &unchecked, tx, rx);
}
