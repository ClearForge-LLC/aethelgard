//! The valid serial driver (`AET-WO-4300b`), on the generated `serial-four`
//! board, included by path (`tests/serial_ports.rs` guards its bytes): clock
//! port A and USART1, put PA9 and PA10 in their USART1 functions, read the
//! reset clock, configure, send, and echo what arrives. Compiled, not called,
//! as in `valid_driver.rs`.
#![allow(dead_code)]

#[path = "../serial-four/board.rs"]
mod board;

use board::Board;
use board::mmio::Mmio;

fn firmware() {
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
    let mut usart1 = usart1.configure(&mut r, &clock, tx, rx);
    for &byte in b"mavlink" {
        if usart1.send(&mut r, byte).is_err() {
            break;
        }
    }
    if let Some(byte) = usart1.receive(&mut r, 100) {
        let _ = usart1.send(&mut r, byte);
    }
}

fn main() {
    let _compiled: fn() = firmware;
}
