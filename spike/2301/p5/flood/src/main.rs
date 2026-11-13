//! AET-WO-2301 spike P5: the shape of AET-WO-2100 adversarial A9. Writes the boot word,
//! then writes GPIOC (0x5802_0800, never clocked: locked) at varying offsets and values
//! forever, so Renode's log lines vary and are not collapsed.
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
);

unsafe extern "C" fn fault() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    // SAFETY: the boot word, first in RAM.
    unsafe { (&raw mut AETHELGARD_BOOT).cast::<u32>().write_volatile(0x2100_b007) };
    let mut i: u32 = 0;
    loop {
        // GPIOC base 0x58020800; offset and value vary every write.
        let off = (i.wrapping_mul(4)) & 0x3fc;
        let p = (0x5802_0800u32 + off) as *mut u32;
        // SAFETY: a deliberate write to a peripheral that is not clocked.
        unsafe { p.write_volatile(i.wrapping_mul(2654435761)) };
        i = i.wrapping_add(1);
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
