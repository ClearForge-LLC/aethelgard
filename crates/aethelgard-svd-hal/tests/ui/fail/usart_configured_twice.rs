//! Gate (`AET-WO-4300b`): a configured USART configured again. Its
//! configuration writes `BRR`, `CR2` and `CR1` once, before `UE`.
#![allow(dead_code)]

#[path = "../serial-four/board.rs"]
mod board;

use board::Board;
use board::mmio::Mmio;

fn main() {
    let mut r = unsafe { Mmio::new() };
    let board = Board::take().unwrap();
    let mut rcc = board.rcc;
    let a = rcc.enable_gpioa(&mut r, board.gpioa).split(&mut r);
    let tx = a.pa9.into_usart1_tx(&mut r);
    let rx = a.pa10.into_usart1_rx(&mut r);
    let usart1 = rcc.enable_usart1(&mut r, board.usart1);
    let Ok(clock) = rcc.reset_clock(&mut r) else {
        return;
    };
    let usart1 = usart1.configure(&mut r, &clock, tx, rx);
    let _again = usart1.configure(&mut r, &clock, tx, rx);
}
