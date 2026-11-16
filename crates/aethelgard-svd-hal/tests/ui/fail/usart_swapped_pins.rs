//! Gate (`AET-WO-4300b`): a USART's TX and RX pins passed in each other's
//! place.
#![allow(dead_code)]

#[path = "../serial-four/board.rs"]
mod board;

use board::Board;
use board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let a = rcc.enable_gpioa(&mut r, board.gpioa).split();
    let tx = a.pa9.into_usart1_tx(&mut r);
    let rx = a.pa10.into_usart1_rx(&mut r);
    let usart1 = rcc.enable_usart1(&mut r, board.usart1);
    let Ok(clock) = rcc.reset_clock(&mut r) else {
        return;
    };
    let _usart1 = usart1.configure(&mut r, &clock, rx, tx);
}
