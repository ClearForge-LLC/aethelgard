//! Gate (`AET-WO-4300b`): a USART configured without its RCC clock enable.
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
    let Ok(clock) = rcc.reset_clock(&mut r, &scale2) else {
        return;
    };
    let _usart1 = board.usart1.configure(&mut r, &clock, tx, rx);
}
