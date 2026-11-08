//! AET-WO-2101 S3 probe: one I2C read of the BME280 ID register (0xD0 -> 0x60)
//! on I2C1 (0x40005400, STM32 I2Cv2 sequence), every wait a bounded spin.
//!
//! Control block (judge writes, 0x20000100): +0 go (seq), +4 nbytes_read (0 -> 1).
//! Result block (0x20000000), u32 words:
//!  0 magic  1 status  2 byte0  3 byte1  4 ISR(end)  5 spins_txis  6 spins_tc
//!  7 spins_rxne  8 spins_stopf  9 nbytes  10 seq  11 done (== seq when finished)
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

const SPIN_MAX: u32 = 20_000;
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

#[unsafe(link_section = ".s3_ctl")]
#[unsafe(no_mangle)]
static mut CTL: [u32; 4] = [0; 4];

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
fn ctl(i: usize) -> u32 {
    unsafe { rd((&raw const CTL as *const u32).add(i)) }
}

/// Spin until `ISR & mask`, bounded. Returns spins used, or None on expiry.
unsafe fn wait(mask: u32, spins_slot: usize) -> bool {
    let mut n = 0;
    while n < SPIN_MAX {
        if unsafe { reg(ISR) } & mask != 0 {
            res(spins_slot, n);
            return true;
        }
        n += 1;
    }
    res(spins_slot, n);
    false
}

unsafe fn txn(nbytes: u32, seq: u32) {
    for i in 0..16 {
        res(i, 0);
    }
    res(0, 0x2101_53A5);
    res(9, nbytes);
    res(10, seq);
    unsafe {
        // Reset flags from any previous transaction: PE low then high.
        set(CR1, 0);
        set(TIMINGR, 0x10B0_1A2A);
        set(CR1, 1);
        // Write phase: SADD, NBYTES=1, START.
        set(CR2, (ADDR7 << 1) | (1 << 16) | (1 << 13));
        let st;
        'run: {
            if !wait(ISR_TXIS, 5) {
                st = TIMEOUT_TXIS;
                break 'run;
            }
            set(TXDR, REG_ID);
            if !wait(ISR_TC, 6) {
                st = TIMEOUT_TC;
                break 'run;
            }
            // Read phase: RD_WRN, NBYTES, START, AUTOEND.
            set(CR2, (ADDR7 << 1) | (1 << 10) | (nbytes << 16) | (1 << 13) | (1 << 25));
            let mut s = OK;
            let mut i = 0;
            while i < nbytes {
                if !wait(ISR_RXNE, 7) {
                    s = TIMEOUT_RXNE;
                    break;
                }
                res(2 + i as usize, reg(RXDR));
                i += 1;
            }
            if s == OK && !wait(ISR_STOPF, 8) {
                s = TIMEOUT_STOPF;
            }
            st = s;
        }
        res(1, st);
        res(4, reg(ISR));
        set(ICR, ISR_STOPF);
    }
    res(11, seq);
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> ! {
    let mut done = 0;
    loop {
        let go = ctl(0);
        if go != done && go != 0 {
            let n = match ctl(1) {
                0 => 1,
                n => n,
            };
            unsafe { txn(n, go) };
            done = go;
        }
    }
}
