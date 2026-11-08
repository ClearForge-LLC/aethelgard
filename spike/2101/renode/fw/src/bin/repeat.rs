//! AET-WO-2101 S3/S4 REPEAT probe: after init, loop forever: one BME280 ID read
//! (reg 0xD0 -> 0x60) on I2C1, then a ~100 us virtual-time spin delay (~10,000
//! instructions at Renode's 100 MIPS). Every wait is bounded (SPIN_MAX).
//! After a timed-out read the controller is reset (PE=0 then PE=1) unless built
//! with `--features no-reset`; STOPF/flags are cleared after every read.
//!
//! Result block, u32 words at 0x20000000 (judge reads with sysbus ReadDoubleWord):
//!  +0x00 magic 0x21015252     +0x04 reads attempted       +0x08 reads OK (status OK && byte==0x60)
//!  +0x0C reads wrong data (status OK, byte!=0x60)         +0x10 reads timed out
//!  +0x14 last status (0 OK, 1 TIMEOUT_TXIS, 2 TIMEOUT_TC, 3 TIMEOUT_RXNE, 4 TIMEOUT_STOPF)
//!  +0x18 last byte read       +0x1C ISR at end of last read
//!  +0x20 controller resets done (PE cycles after a timeout)  +0x24 loop-alive word (== attempted)
#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;
use core::ptr::{read_volatile as rd, write_volatile as wr};

const I2C1: u32 = 0x4000_5400;
const CR1: u32 = I2C1;
const CR2: u32 = I2C1 + 0x04;
const TIMINGR: u32 = I2C1 + 0x10;
const ISR: u32 = I2C1 + 0x18;
const ICR: u32 = I2C1 + 0x1C;
const RXDR: u32 = I2C1 + 0x24;
const TXDR: u32 = I2C1 + 0x28;

const ISR_TXIS: u32 = 1 << 1;
const ISR_RXNE: u32 = 1 << 2;
const ISR_STOPF: u32 = 1 << 5;
const ISR_TC: u32 = 1 << 6;

const SPIN_MAX: u32 = 2_000;
const ADDR7: u32 = 0x76;
const REG_ID: u32 = 0xD0;

const OK: u32 = 0;
const TIMEOUT_TXIS: u32 = 1;
const TIMEOUT_TC: u32 = 2;
const TIMEOUT_RXNE: u32 = 3;
const TIMEOUT_STOPF: u32 = 4;

#[unsafe(link_section = ".s3_result")]
#[unsafe(no_mangle)]
static mut RESULT: [u32; 16] = [0; 16];

unsafe extern "C" {
    fn Reset() -> !;
}

#[unsafe(link_section = ".vector_table.exceptions")]
#[used]
static EXCEPTIONS: [unsafe extern "C" fn() -> !; 15] = [
    Reset, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault,
    fault, fault,
];

global_asm!(
    ".section .text.Reset,\"ax\",%progbits",
    ".global Reset",
    ".type Reset,%function",
    ".thumb_func",
    "Reset:",
    "    bl main",
    "    udf #0",
    ".size Reset, . - Reset",
);

unsafe extern "C" fn fault() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}

unsafe fn reg(a: u32) -> u32 {
    unsafe { rd(a as *const u32) }
}
unsafe fn set(a: u32, v: u32) {
    unsafe { wr(a as *mut u32, v) }
}
fn res(i: usize, v: u32) {
    unsafe { wr((&raw mut RESULT as *mut u32).add(i), v) }
}

unsafe fn wait(mask: u32) -> bool {
    let mut n = 0;
    while n < SPIN_MAX {
        if unsafe { reg(ISR) } & mask != 0 {
            return true;
        }
        n += 1;
    }
    false
}

/// One ID read. Returns (status, byte).
unsafe fn read_id() -> (u32, u32) {
    unsafe {
        set(CR2, (ADDR7 << 1) | (1 << 16) | (1 << 13));
        if !wait(ISR_TXIS) {
            return (TIMEOUT_TXIS, 0);
        }
        set(TXDR, REG_ID);
        if !wait(ISR_TC) {
            return (TIMEOUT_TC, 0);
        }
        set(CR2, (ADDR7 << 1) | (1 << 10) | (1 << 16) | (1 << 13) | (1 << 25));
        if !wait(ISR_RXNE) {
            return (TIMEOUT_RXNE, 0);
        }
        let b = reg(RXDR);
        if !wait(ISR_STOPF) {
            return (TIMEOUT_STOPF, b);
        }
        (OK, b)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> ! {
    let (mut att, mut ok, mut bad, mut tmo, mut rst) = (0u32, 0u32, 0u32, 0u32, 0u32);
    unsafe {
        set(CR1, 0);
        set(TIMINGR, 0x10B0_1A2A);
        set(CR1, 1);
    }
    res(0, 0x2101_5252);
    loop {
        let (st, b) = unsafe { read_id() };
        att += 1;
        if st == OK {
            if b == 0x60 { ok += 1 } else { bad += 1 }
        } else {
            tmo += 1;
        }
        unsafe {
            res(7, reg(ISR));
            set(ICR, ISR_STOPF);
            #[cfg(not(feature = "no-reset"))]
            if st != OK {
                set(CR1, 0);
                set(CR1, 1);
                rst += 1;
            }
        }
        res(1, att);
        res(2, ok);
        res(3, bad);
        res(4, tmo);
        res(5, st);
        res(6, b);
        res(8, rst);
        res(9, att);
        // ~100 us: 2000 iterations x ~5 instructions (measured)
        let mut d = 0u32;
        while unsafe { rd(&raw const d) } < 2000 {
            unsafe { wr(&raw mut d, rd(&raw const d) + 1) };
        }
    }
}
