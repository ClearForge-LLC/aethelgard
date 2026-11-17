//! AET-WO-4400e3 RETTOBASE fixture (tier 2): the regression witness of the twin's correction of
//! Renode 1.17.0's `ICSR.RETTOBASE`, with no ArduPilot.
//!
//! It nests two external interrupts at the priorities `AET-WO-4400e2` measured under ArduPilot:
//! TIM5 (IRQ 50) at `0x70` preempts DMA2 stream 1 (IRQ 57) at `0xA0`. It reads `ICSR` three times:
//! - in Thread mode, before either interrupt;
//! - in IRQ 57's handler, the only active exception;
//! - in IRQ 50's handler, which preempted IRQ 57's.
//!
//! The Armv7-M ARM (DDI 0403E.b §B3.2.4, p. B3-657) defines `RETTOBASE` in Handler mode as 0 when
//! an exception other than the one IPSR shows is active, so the third read must have bit 11 clear;
//! the second, bit 11 set. In Thread mode the bit is UNKNOWN.
//!
//! The record, first in RAM (`0x20000000`), never initialised by the reset handler:
//! - word 0: [`DONE`], written last;
//! - word 1: `ICSR` in Thread mode;
//! - word 2: `ICSR` in IRQ 57's handler;
//! - word 3: `ICSR` in IRQ 50's handler.
//!
//! Both interrupts are disabled again before word 0 is written. Every address is the Armv7-M
//! System Control Space's (`0xE000E000`), every IRQ number the STM32H753's vector table
//! (RM0433 Rev 9, Table 145).
#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

/// Word 0 once the program has finished.
const DONE: u32 = 0x4400_E3D0;

/// `AIRCR`: the key `0x05FA` and `PRIGROUP` 3, as ChibiOS writes it (`AET-WO-4400e2`: `0xFA052300`).
const AIRCR: u32 = 0xE000_ED0C;
const ICSR: u32 = 0xE000_ED04;
/// `NVIC_ISER1`, `NVIC_ICER1`, `NVIC_ISPR1`: IRQs 32 to 63.
const ISER1: u32 = 0xE000_E104;
const ICER1: u32 = 0xE000_E184;
const ISPR1: u32 = 0xE000_E204;
/// `NVIC_IPR`, one byte per IRQ.
const IPR: u32 = 0xE000_E400;
const OUTER: u32 = 57; // DMA2 stream 1
const INNER: u32 = 50; // TIM5
const OUTER_PRIORITY: u8 = 0xA0;
const INNER_PRIORITY: u8 = 0x70;

#[unsafe(link_section = ".aethelgard_boot")]
#[unsafe(no_mangle)]
static mut RECORD: [u32; 4] = [0; 4];

unsafe extern "C" {
    fn Reset() -> !;
}

#[unsafe(link_section = ".vector_table.exceptions")]
#[used]
static EXCEPTIONS: [unsafe extern "C" fn() -> !; 15] = [
    Reset, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault,
    fault, fault,
];

/// The STM32H753's 150 external interrupts: every one parks but the two this fixture nests.
#[unsafe(link_section = ".vector_table.interrupts")]
#[used]
static INTERRUPTS: [unsafe extern "C" fn(); 150] = {
    let mut t: [unsafe extern "C" fn(); 150] = [unexpected; 150];
    t[INNER as usize] = inner;
    t[OUTER as usize] = outer;
    t
};

global_asm!(
    ".section .text.Reset,\"ax\",%progbits",
    ".global Reset",
    ".type Reset,%function",
    ".thumb_func",
    "Reset:",
    "    ldr r0, =_sbss",
    "    ldr r1, =_ebss",
    "    movs r2, #0",
    "0:  cmp r0, r1",
    "    bhs 1f",
    "    stm r0!, {{r2}}",
    "    b 0b",
    "1:  bl main",
    "    udf #0",
    "    .pool",
    ".size Reset, . - Reset",
);

unsafe extern "C" fn fault() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

unsafe extern "C" fn unexpected() {
    loop {
        core::hint::spin_loop();
    }
}

fn w(addr: u32, value: u32) {
    // SAFETY: a fixture's write to a fixed System Control Space register of the Cortex-M7.
    unsafe { (addr as *mut u32).write_volatile(value) };
}

fn r(addr: u32) -> u32 {
    // SAFETY: a fixture's read of a fixed System Control Space register of the Cortex-M7.
    unsafe { (addr as *const u32).read_volatile() }
}

fn record(word: usize, value: u32) {
    // SAFETY: the record is this fixture's own static, written by one context at a time.
    unsafe { (&raw mut RECORD).cast::<u32>().add(word).write_volatile(value) };
}

fn recorded(word: usize) -> u32 {
    // SAFETY: as `record`.
    unsafe { (&raw const RECORD).cast::<u32>().add(word).read_volatile() }
}

fn barrier() {
    // SAFETY: data and instruction synchronisation barriers, so a pend takes effect before the
    // next instruction.
    unsafe { core::arch::asm!("dsb", "isb") };
}

/// IRQ 57's handler, the outer one: the only active exception when it reads `ICSR`. It then pends
/// IRQ 50, which preempts it at once.
unsafe extern "C" fn outer() {
    record(2, r(ICSR));
    w(ISPR1, 1 << (INNER - 32));
    barrier();
    // Bounded: IRQ 50 is taken at the latest at a translation block's end.
    let mut n = 0u32;
    while recorded(3) == 0 && n < 1_000 {
        n += 1;
        core::hint::spin_loop();
    }
}

/// IRQ 50's handler, the inner one: IRQ 57's handler is active beneath it.
unsafe extern "C" fn inner() {
    record(3, r(ICSR));
}

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    w(AIRCR, 0x05FA_0300);
    // SAFETY: byte writes to `NVIC_IPR`, one priority byte per IRQ.
    unsafe {
        ((IPR + OUTER) as *mut u8).write_volatile(OUTER_PRIORITY);
        ((IPR + INNER) as *mut u8).write_volatile(INNER_PRIORITY);
    }
    w(ISER1, (1 << (OUTER - 32)) | (1 << (INNER - 32)));
    record(1, r(ICSR));
    w(ISPR1, 1 << (OUTER - 32));
    barrier();
    let mut n = 0u32;
    while recorded(3) == 0 && n < 100_000 {
        n += 1;
        core::hint::spin_loop();
    }
    w(ICER1, (1 << (OUTER - 32)) | (1 << (INNER - 32)));
    record(0, DONE);
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
