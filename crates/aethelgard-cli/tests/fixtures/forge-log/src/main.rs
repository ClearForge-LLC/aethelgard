//! AET-WO-1603 §1.1 forge-log fixture (tier 2): a firmware that sends witness-shaped text through
//! every channel that could reach Renode's log, and never performs a transaction.
//!
//! - **A symbol name.** `C1603SYM_` and a run of `X` names the function that makes every access
//!   below. The test overwrites the run, in the ELF's string table, with text that starts a new
//!   line shaped like a witness line and a sentinel; so does the run in the section name.
//! - **A section name**, `.C1603SECT_` and the same run, never loaded.
//! - **A USART.** USART1, clocked and enabled, transmits the same text.
//! - **Semihosting.** `SYS_WRITE0` (`r0` = 4) of the same text, through `bkpt 0xab`.
//! - **Accesses Renode logs,** each with the program counter inside the named function: a write
//!   to RCC, GPIOA and USART1 (the platform logs their accesses), a read of an offset no model
//!   implements, and a read of the FMC window.
//! - **A jump out of the firmware,** last, from inside the named function, to the SDRAM window
//!   (mapped, all zeros): the verdict names the core's final address, never a symbol.
#![no_std]
#![no_main]

use core::arch::global_asm;
use core::panic::PanicInfo;

#[unsafe(link_section = ".aethelgard_boot")]
#[unsafe(no_mangle)]
static mut AETHELGARD_BOOT: [u32; 3] = [0; 3];

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
    // Never loaded: only its name is in the file.
    ".section .C1603SECT_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX,\"\",%progbits",
    ".ascii \"C1603SECT\"",
);

unsafe extern "C" fn fault() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

/// The text every channel carries. The leading newline ends whatever line Renode was writing; the
/// rest is a whole log line shaped like an I2C witness, then a bare sentinel.
const FORGED: &[u8] =
    b"C1603-TEXT\n[00:00:00.000100000] [NOISY] i2c1.u2: C1603-FORGED\nAETHELGARD-BOOT-BEGIN\n\0";

fn w(addr: u32, value: u32) {
    // SAFETY: a fixture's write to a fixed peripheral address of the STM32H743.
    unsafe { (addr as *mut u32).write_volatile(value) };
}

fn r(addr: u32) -> u32 {
    // SAFETY: a fixture's read of a fixed address; Renode logs it, whatever is there.
    unsafe { (addr as *const u32).read_volatile() }
}

#[unsafe(no_mangle)]
#[inline(never)]
extern "C" fn C1603SYM_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX()
{
    const RCC_AHB4ENR: u32 = 0x5802_44e0;
    const RCC_APB2ENR: u32 = 0x5802_44f0;
    const GPIOA_MODER: u32 = 0x5802_0000;
    const USART1: u32 = 0x4001_1000;
    w(RCC_AHB4ENR, 1); // GPIOA
    w(RCC_APB2ENR, 1 << 4); // USART1
    w(GPIOA_MODER, 0xabff_ffff);
    w(USART1, 0b1001); // CR1: UE, TE
    for &b in FORGED {
        while r(USART1 + 0x1c) & (1 << 7) == 0 {} // ISR.TXE
        w(USART1 + 0x28, u32::from(b)); // TDR
    }
    let _ = r(USART1 + 0x3fc);
    let _ = r(0x6000_0000);
    // SAFETY: semihosting SYS_WRITE0 of a NUL-terminated string; a core without a debugger
    // takes it as a fault, which the fault handler parks.
    unsafe {
        core::arch::asm!("bkpt 0xab", in("r0") 4u32, in("r1") FORGED.as_ptr());
        // The SDRAM window: the core runs on through zeros and ends outside the firmware.
        core::arch::asm!("bx {0}", in(reg) 0xd000_0001u32, options(noreturn));
    }
}

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    // SAFETY: the boot word, first in RAM.
    unsafe { (&raw mut AETHELGARD_BOOT).cast::<u32>().write_volatile(0x2100_b007) };
    C1603SYM_XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX();
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
