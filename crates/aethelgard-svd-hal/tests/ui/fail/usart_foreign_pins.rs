//! Gate (`AET-WO-4300b`): a USART configured on another USART's solved pins.
#![allow(dead_code)]

#[path = "../serial-four/board.rs"]
mod board;

use board::Board;
use board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let d = rcc.enable_gpiod(&mut r, board.gpiod).split();
    let tx3 = d.pd8.into_usart3_tx(&mut r);
    let rx3 = d.pd9.into_usart3_rx(&mut r);
    let usart1 = rcc.enable_usart1(&mut r, board.usart1);
    let Ok(clock) = rcc.reset_clock(&mut r) else {
        return;
    };
    let _usart1 = usart1.configure(&mut r, &clock, tx3, rx3);
}
