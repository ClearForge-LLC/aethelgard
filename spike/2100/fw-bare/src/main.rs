//! AET-WO-2100 spike, S2 option 1: the smallest firmware on the generated
//! board crate, with a hand-written vector table and no runtime crate.
//!
//! It configures every solved pin and clock through the typestates, sends one
//! byte on SPI1 framed by `FLASH_CS`, then writes `BOOT_MAGIC` to the reserved
//! word and parks. Every `unsafe` here is counted in FEEDBACK S2.
#![no_std]
#![no_main]

use core::panic::PanicInfo;

use sentinel_01_board::Board;
use sentinel_01_board::i2c1_speed::Fast400k;
use sentinel_01_board::mmio::Mmio;

/// What the fixed point writes (S3). Not a reset value of RAM or of any
/// register the probe reads.
pub const BOOT_MAGIC: u32 = 0x2100_B007;

/// The observable word: the first word of RAM (`link.x`, `.aethelgard_boot`).
#[unsafe(link_section = ".aethelgard_boot")]
#[unsafe(no_mangle)]
static mut AETHELGARD_BOOT: u32 = 0;

unsafe extern "C" {
    static mut _sdata: u32;
    static mut _edata: u32;
    static _sidata: u32;
    static mut _sbss: u32;
    static mut _ebss: u32;
}

/// Reset, then the 14 core exceptions; the linker script puts the initial SP
/// before them.
#[unsafe(link_section = ".vector_table.exceptions")]
#[used]
static EXCEPTIONS: [unsafe extern "C" fn() -> !; 15] = [
    Reset, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault,
    fault, fault,
];

/// # Safety
/// Called once by the core out of reset, before anything reads a static.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Reset() -> ! {
    // SAFETY: the symbols bound word-aligned regions of RAM and flash that
    // the linker script defines; nothing else runs yet.
    unsafe {
        let mut src = &raw const _sidata;
        let mut dst = &raw mut _sdata;
        while dst < &raw mut _edata {
            dst.write_volatile(src.read());
            dst = dst.add(1);
            src = src.add(1);
        }
        let mut bss = &raw mut _sbss;
        while bss < &raw mut _ebss {
            bss.write_volatile(0);
            bss = bss.add(1);
        }
    }
    main()
}

/// Every other exception parks here, distinguishable from the fixed point
/// because the reserved word is never written.
unsafe extern "C" fn fault() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

fn main() -> ! {
    // SAFETY: this is the STM32H753 the crate was generated for, and this is
    // the only `Mmio`.
    let mut r = unsafe { Mmio::new() };
    let Some(Board {
        mut rcc,
        gpioa,
        gpiob,
        i2c1,
        spi1,
    }) = Board::take()
    else {
        loop {
            core::hint::spin_loop();
        }
    };
    let a = rcc.enable_gpioa(&mut r, gpioa).split();
    let b = rcc.enable_gpiob(&mut r, gpiob).split();
    let i2c1 = rcc.enable_i2c1(&mut r, i2c1);
    let spi1 = rcc.enable_spi1(&mut r, spi1);

    let mut cs = a.pa0.into_output(&mut r);
    let sck = a.pa5.into_spi1_sck(&mut r);
    let miso = a.pa6.into_spi1_miso(&mut r);
    let mosi = a.pa7.into_spi1_mosi(&mut r);
    let scl = b.pb6.into_i2c1_scl(&mut r);
    let sda = b.pb7.into_i2c1_sda(&mut r);

    let _i2c = i2c1.init::<Fast400k, _>(&mut r, scl, sda);
    let mut spi = spi1.init(&mut r, sck, mosi, miso);

    cs.set_low(&mut r);
    spi.write_byte(&mut r, 0x9f);
    cs.set_high(&mut r);

    #[cfg(feature = "uart")]
    uart::line(b"AET-2100 BOOT\n");

    // SAFETY: a word only this line writes.
    unsafe { (&raw mut AETHELGARD_BOOT).write_volatile(BOOT_MAGIC) };
    loop {
        core::hint::spin_loop();
    }
}

/// S3's UART candidate. USART3 is the Nucleo's virtual COM port; nothing in
/// the solved topology names it, so this goes around the board crate.
#[cfg(feature = "uart")]
mod uart {
    const RCC_APB1LENR: usize = 0x5802_4400 + 0x0e8; // RCC_BASE + APB1LENR
    const USART3: usize = 0x4000_4800; // USART3_BASE
    pub fn line(bytes: &[u8]) {
        // SAFETY: registers of this part; nothing else drives USART3.
        unsafe {
            let enr = RCC_APB1LENR as *mut u32;
            enr.write_volatile(enr.read_volatile() | 1 << 18); // USART3EN
            ((USART3) as *mut u32).write_volatile(1 << 3 | 1); // CR1: TE | UE
            for &b in bytes {
                while ((USART3 + 0x1c) as *const u32).read_volatile() & 1 << 7 == 0 {} // ISR.TXE
                ((USART3 + 0x28) as *mut u32).write_volatile(u32::from(b)); // TDR
            }
        }
    }
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
