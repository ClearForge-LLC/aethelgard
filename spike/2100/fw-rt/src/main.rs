//! AET-WO-2100 spike, S2 option 2: the smallest firmware on the generated
//! board crate, with a cortex_m_rt runtime (`#[entry]`).
//!
//! It configures every solved pin and clock through the typestates, sends one
//! byte on SPI1 framed by `FLASH_CS`, then writes `BOOT_MAGIC` to the reserved
//! word and parks. Every `unsafe` here is counted in FEEDBACK S2.
#![no_std]
#![no_main]

use core::panic::PanicInfo;

use cortex_m_rt::entry;
use sentinel_01_board::Board;
use sentinel_01_board::i2c1_speed::Fast400k;
use sentinel_01_board::mmio::Mmio;

/// What the fixed point writes (S3). Not a reset value of RAM or of any
/// register the probe reads.
pub const BOOT_MAGIC: u32 = 0x2100_B007;

/// The observable word: the first word of RAM (`memory.x`, `.aethelgard_boot`).
#[unsafe(link_section = ".aethelgard_boot")]
#[unsafe(no_mangle)]
static mut AETHELGARD_BOOT: u32 = 0;

#[entry]
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

    // SAFETY: a word only this line writes.
    unsafe { (&raw mut AETHELGARD_BOOT).write_volatile(BOOT_MAGIC) };
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
