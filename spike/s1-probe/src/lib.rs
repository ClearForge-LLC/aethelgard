//! AET-WO-1400 spike, S1: a hand-written stand-in for what the board-crate
//! generator will emit, for one solved topology:
//!
//! - PB8 solved to `I2C1_SCL` and PB9 to `I2C1_SDA`, both alternate function 4
//!   (ST's `GPIO-STM32H747_gpio_v1_0_Modes.xml`, `GPIO_AF4_I2C1`);
//! - I2C1, whose kernel clock is gated by `RCC_APB1LENR.I2C1EN` (bit 21), on
//!   port B, gated by `RCC_AHB4ENR.GPIOBEN` (bit 1) (RM0433 §8.7);
//! - a bus whose solved rise-time ceiling admits 100 kHz and 400 kHz, not 1 MHz.
//!
//! Every register access goes through [`Registers`]. The one implementation at
//! real addresses is [`mmio`], the only `unsafe` in the crate. The Kani harness
//! in `proofs` implements the trait over a register image it owns.
//!
//! The board API is written once, in `board!`, and instantiated four times: the
//! correct one, and three with one seeded generator bug each. The bug is a
//! one-argument difference at the instantiation, so the four modules differ in
//! exactly the line that names it.
#![no_std]

use core::marker::PhantomData;

/// The register blocks the probe touches.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Block {
    Rcc,
    GpioB,
    I2c1,
}

/// The one door to the hardware. Offsets are byte offsets into the block.
pub trait Registers {
    fn read(&mut self, block: Block, offset: usize) -> u32;
    fn write(&mut self, block: Block, offset: usize, value: u32);
    fn modify(&mut self, block: Block, offset: usize, f: impl FnOnce(u32) -> u32) {
        let value = self.read(block, offset);
        self.write(block, offset, f(value));
    }
}

/// Volatile access at the STM32H753's real base addresses (RM0433 §2.3.2).
/// Never exercised by the proof: Kani cannot own this memory
/// (`architecture.md` §2.2).
pub mod mmio {
    use super::{Block, Registers};

    pub struct Mmio {
        _private: (),
    }

    impl Mmio {
        /// # Safety
        /// The caller is on an STM32H753, and no other `Mmio` exists.
        pub unsafe fn new() -> Self {
            Mmio { _private: () }
        }
    }

    const fn base(block: Block) -> usize {
        match block {
            Block::Rcc => 0x5802_4400,
            Block::GpioB => 0x5802_0400,
            Block::I2c1 => 0x4000_5400,
        }
    }

    impl Registers for Mmio {
        fn read(&mut self, block: Block, offset: usize) -> u32 {
            // SAFETY: a register of a peripheral that exists on this part.
            unsafe { core::ptr::read_volatile((base(block) + offset) as *const u32) }
        }
        fn write(&mut self, block: Block, offset: usize, value: u32) {
            // SAFETY: as for `read`.
            unsafe { core::ptr::write_volatile((base(block) + offset) as *mut u32, value) }
        }
    }
}

/// Pin modes.
pub struct Unconfigured;
pub struct AlternateFunction<const AF: u8>;

/// Clock and peripheral states.
pub struct Gated;
pub struct Clocked;
pub struct Enabled<S>(PhantomData<S>);

/// Bus speeds. Only those at or under the solved rise-time ceiling implement
/// [`speed::AtMostCeiling`], and the trait is sealed, so firmware cannot add one.
pub mod speed {
    mod sealed {
        pub trait Sealed {}
    }

    pub trait AtMostCeiling: sealed::Sealed {
        /// `I2C_TIMINGR` for this speed. Placeholder values for a 64 MHz
        /// kernel clock: clock-tree solving is out of scope (WO §4).
        const TIMINGR: u32;
    }

    pub struct Standard100k;
    pub struct Fast400k;
    pub struct FastPlus1M;

    impl sealed::Sealed for Standard100k {}
    impl AtMostCeiling for Standard100k {
        const TIMINGR: u32 = 0x1070_7DBC;
    }
    impl sealed::Sealed for Fast400k {}
    impl AtMostCeiling for Fast400k {
        const TIMINGR: u32 = 0x0060_2173;
    }
}

/// The alternate functions the solver assigned to a pin. A pin implements only
/// the ones it was solved to, so any other `into_alternate::<N>` fails to compile.
pub trait SolvedAf<const AF: u8> {}

/// Generator knobs: the correct emission, and the seeded bugs.
pub mod knobs {
    /// AF field value written for alternate function `af`.
    pub const fn af_exact(af: u8) -> u32 {
        af as u32
    }
    /// Seeded bug 1: off by one.
    pub const fn af_off_by_one(af: u8) -> u32 {
        af as u32 + 1
    }
    /// Byte offset of the AFR register holding `pin`: AFRL (0x20) or AFRH (0x24).
    pub const fn afr_offset(pin: u8) -> usize {
        0x20 + 4 * (pin as usize / 8)
    }
    /// Seeded bug 3: indexes by pin, not by pin/8; pin 8 lands at 0x40, past
    /// the block's last register (AFRH, 0x24).
    pub const fn afr_offset_by_pin(pin: u8) -> usize {
        0x20 + 4 * pin as usize
    }
}

macro_rules! board {
    ($name:ident, af_value = $af_value:path, afr_offset = $afr_offset:path, enable_i2c1_clock = $clock:expr) => {
        pub mod $name {
            use super::speed::AtMostCeiling;
            use super::*;
            use core::sync::atomic::{AtomicBool, Ordering};

            const RCC_AHB4ENR: usize = 0x0E0;
            const RCC_APB1LENR: usize = 0x0E8;
            const GPIOBEN: u32 = 1 << 1;
            const I2C1EN: u32 = 1 << 21;
            const GPIO_MODER: usize = 0x00;
            const I2C_CR1: usize = 0x00;
            const I2C_TIMINGR: usize = 0x10;
            const I2C_TXDR: usize = 0x28;
            const PE: u32 = 1;

            pub struct Pin<const N: u8, M> {
                _mode: PhantomData<M>,
            }
            pub type PB8<M> = Pin<8, M>;
            pub type PB9<M> = Pin<9, M>;
            impl<M> SolvedAf<4> for Pin<8, M> {}
            impl<M> SolvedAf<4> for Pin<9, M> {}

            impl<const N: u8> Pin<N, Unconfigured> {
                pub fn into_alternate<const AF: u8, R: Registers>(
                    self,
                    r: &mut R,
                ) -> Pin<N, AlternateFunction<AF>>
                where
                    Self: SolvedAf<AF>,
                {
                    let afr = $afr_offset(N);
                    let shift = 4 * (N as u32 % 8);
                    let field = $af_value(AF) & 0xF;
                    r.modify(Block::GpioB, afr, |v| {
                        (v & !(0xF << shift)) | (field << shift)
                    });
                    let shift = 2 * N as u32;
                    r.modify(Block::GpioB, GPIO_MODER, |v| {
                        (v & !(0b11 << shift)) | (0b10 << shift)
                    });
                    Pin { _mode: PhantomData }
                }
            }

            pub struct GpioB<S> {
                _state: PhantomData<S>,
            }
            impl GpioB<Clocked> {
                pub fn split(self) -> (PB8<Unconfigured>, PB9<Unconfigured>) {
                    (Pin { _mode: PhantomData }, Pin { _mode: PhantomData })
                }
            }

            pub struct I2c1<S> {
                _state: PhantomData<S>,
            }
            impl I2c1<Clocked> {
                pub fn init<S: AtMostCeiling, R: Registers>(
                    self,
                    r: &mut R,
                    _scl: PB8<AlternateFunction<4>>,
                    _sda: PB9<AlternateFunction<4>>,
                ) -> I2c1<Enabled<S>> {
                    r.modify(Block::I2c1, I2C_CR1, |v| v & !PE);
                    r.write(Block::I2c1, I2C_TIMINGR, S::TIMINGR);
                    r.modify(Block::I2c1, I2C_CR1, |v| v | PE);
                    I2c1 {
                        _state: PhantomData,
                    }
                }
            }
            impl<S: AtMostCeiling> I2c1<Enabled<S>> {
                pub fn write_byte<R: Registers>(&mut self, r: &mut R, byte: u8) {
                    r.write(Block::I2c1, I2C_TXDR, u32::from(byte));
                }
            }

            pub struct Rcc {
                _private: (),
            }
            impl Rcc {
                pub fn enable_gpiob<R: Registers>(
                    &mut self,
                    r: &mut R,
                    _port: GpioB<Gated>,
                ) -> GpioB<Clocked> {
                    r.modify(Block::Rcc, RCC_AHB4ENR, |v| v | GPIOBEN);
                    GpioB {
                        _state: PhantomData,
                    }
                }
                pub fn enable_i2c1<R: Registers>(
                    &mut self,
                    r: &mut R,
                    _i2c: I2c1<Gated>,
                ) -> I2c1<Clocked> {
                    if $clock {
                        r.modify(Block::Rcc, RCC_APB1LENR, |v| v | I2C1EN);
                    }
                    I2c1 {
                        _state: PhantomData,
                    }
                }
            }

            pub struct Board {
                pub rcc: Rcc,
                pub gpiob: GpioB<Gated>,
                pub i2c1: I2c1<Gated>,
            }

            static TAKEN: AtomicBool = AtomicBool::new(false);

            impl Board {
                /// The board's peripherals, once per program.
                pub fn take() -> Option<Board> {
                    if TAKEN.swap(true, Ordering::AcqRel) {
                        return None;
                    }
                    Some(Board {
                        rcc: Rcc { _private: () },
                        gpiob: GpioB {
                            _state: PhantomData,
                        },
                        i2c1: I2c1 {
                            _state: PhantomData,
                        },
                    })
                }
            }
        }
    };
}

board!(
    good,
    af_value = knobs::af_exact,
    afr_offset = knobs::afr_offset,
    enable_i2c1_clock = true
);
board!(
    bug_wrong_af,
    af_value = knobs::af_off_by_one,
    afr_offset = knobs::afr_offset,
    enable_i2c1_clock = true
);
board!(
    bug_no_clock,
    af_value = knobs::af_exact,
    afr_offset = knobs::afr_offset,
    enable_i2c1_clock = false
);
board!(
    bug_past_block,
    af_value = knobs::af_exact,
    afr_offset = knobs::afr_offset_by_pin,
    enable_i2c1_clock = true
);

/// The proof. Everything the specification says is written here again, from
/// RM0433 and the solved topology, and nothing is imported from the board
/// modules but their public API: a wrong constant in a board module is not
/// mirrored in its oracle. (In the build, the generator writes this module from
/// the specification side, `INV-07`; in this spike it is hand-written beside
/// the crate under proof.)
#[cfg(kani)]
mod proofs {
    use super::{Block, Registers};

    const AHB4ENR: usize = 0x0E0;
    const APB1LENR: usize = 0x0E8;
    const GPIOBEN: u32 = 1 << 1;
    const I2C1EN: u32 = 1 << 21;
    const MODER: usize = 0x00;
    const AFRL: usize = 0x20;
    const AFRH: usize = 0x24;
    const CR1: usize = 0x00;
    const TIMINGR: usize = 0x10;
    const PE: u32 = 1;
    /// Register words in each block, as RM0433 lays them out: RCC fills its
    /// 1 KiB window; GPIO ends at AFRH (0x24); I2C ends at TXDR (0x28).
    const RCC_WORDS: usize = 256;
    const GPIO_WORDS: usize = 10;
    const I2C_WORDS: usize = 11;

    /// The harness-owned register image, and the oracle for the ordering rules.
    #[derive(Clone, Copy)]
    struct Image {
        rcc: [u32; RCC_WORDS],
        gpiob: [u32; GPIO_WORDS],
        i2c1: [u32; I2C_WORDS],
    }

    impl Image {
        fn slot(&mut self, block: Block, offset: usize) -> &mut u32 {
            let words: &mut [u32] = match block {
                Block::Rcc => &mut self.rcc,
                Block::GpioB => &mut self.gpiob,
                Block::I2c1 => &mut self.i2c1,
            };
            kani::assert(
                offset % 4 == 0 && offset / 4 < words.len(),
                "register access past the end of its block",
            );
            &mut words[offset / 4]
        }
    }

    impl Registers for Image {
        fn read(&mut self, block: Block, offset: usize) -> u32 {
            *self.slot(block, offset)
        }
        fn write(&mut self, block: Block, offset: usize, value: u32) {
            match block {
                Block::GpioB => kani::assert(
                    self.rcc[AHB4ENR / 4] & GPIOBEN != 0,
                    "GPIOB written before RCC_AHB4ENR.GPIOBEN is set",
                ),
                Block::I2c1 => {
                    kani::assert(
                        self.rcc[APB1LENR / 4] & I2C1EN != 0,
                        "I2C1 written before RCC_APB1LENR.I2C1EN is set",
                    );
                    if offset == TIMINGR {
                        kani::assert(
                            self.i2c1[CR1 / 4] & PE == 0,
                            "I2C1 TIMINGR written while PE is set",
                        );
                    }
                }
                Block::Rcc => {}
            }
            *self.slot(block, offset) = value;
        }
    }

    /// The state the specification allows, from any starting image.
    fn check_final(before: &Image, after: &Image) {
        // Pins: PB8 and PB9 in alternate-function mode (0b10), AF4, and no
        // other pin's MODER or AFR field touched.
        let moder_mask = 0xF << 16;
        kani::assert(
            after.gpiob[MODER / 4] & moder_mask == 0b1010 << 16,
            "PB8/PB9 not in AF mode",
        );
        kani::assert(
            after.gpiob[MODER / 4] & !moder_mask == before.gpiob[MODER / 4] & !moder_mask,
            "another pin's MODER changed",
        );
        let afrh_mask = 0xFF;
        kani::assert(
            after.gpiob[AFRH / 4] & afrh_mask == 0x44,
            "PB8/PB9 AFRH field is not AF4",
        );
        kani::assert(
            after.gpiob[AFRH / 4] & !afrh_mask == before.gpiob[AFRH / 4] & !afrh_mask,
            "another pin's AFRH field changed",
        );
        kani::assert(
            after.gpiob[AFRL / 4] == before.gpiob[AFRL / 4],
            "AFRL changed",
        );
        for i in 1..GPIO_WORDS {
            if i != AFRL / 4 && i != AFRH / 4 {
                kani::assert(
                    after.gpiob[i] == before.gpiob[i],
                    "a GPIOB register other than MODER/AFR changed",
                );
            }
        }
        // Clocks: exactly the two enables added.
        let mut rcc = before.rcc;
        rcc[AHB4ENR / 4] |= GPIOBEN;
        rcc[APB1LENR / 4] |= I2C1EN;
        // Word by word: `==` on the arrays lowers to a 1 KiB `memcmp` loop.
        for i in 0..RCC_WORDS {
            kani::assert(
                after.rcc[i] == rcc[i],
                "RCC differs from before plus GPIOBEN and I2C1EN",
            );
        }
        // The peripheral: enabled.
        kani::assert(after.i2c1[CR1 / 4] & PE != 0, "I2C1 not enabled");
    }

    macro_rules! harness {
        ($board:ident) => {
            pub mod $board {
                use super::*;
                use crate::speed::{AtMostCeiling, Fast400k, Standard100k};
                use crate::$board::{Board, I2c1, PB8, PB9};
                use crate::{AlternateFunction, Clocked};

                fn drive<S: AtMostCeiling>(
                    img: &mut Image,
                    i2c: I2c1<Clocked>,
                    pb8: PB8<AlternateFunction<4>>,
                    pb9: PB9<AlternateFunction<4>>,
                ) {
                    let mut bus = i2c.init::<S, _>(img, pb8, pb9);
                    let writes: u8 = kani::any();
                    kani::assume(writes <= 2);
                    for _ in 0..writes {
                        bus.write_byte(img, kani::any());
                    }
                }

                /// Every call sequence the typestate API admits for this
                /// topology, from an arbitrary starting image: either clock
                /// order, either legal speed, zero to two bytes written.
                #[kani::proof]
                #[kani::unwind(257)]
                fn every_valid_sequence() {
                    let mut img = Image {
                        rcc: kani::any(),
                        gpiob: kani::any(),
                        i2c1: kani::any(),
                    };
                    let before = img;
                    let board = Board::take().unwrap();
                    let mut rcc = board.rcc;
                    let (i2c, pb8, pb9);
                    if kani::any() {
                        let (p8, p9) = rcc.enable_gpiob(&mut img, board.gpiob).split();
                        pb8 = p8.into_alternate::<4, _>(&mut img);
                        pb9 = p9.into_alternate::<4, _>(&mut img);
                        i2c = rcc.enable_i2c1(&mut img, board.i2c1);
                    } else {
                        i2c = rcc.enable_i2c1(&mut img, board.i2c1);
                        let (p8, p9) = rcc.enable_gpiob(&mut img, board.gpiob).split();
                        pb8 = p8.into_alternate::<4, _>(&mut img);
                        pb9 = p9.into_alternate::<4, _>(&mut img);
                    }
                    if kani::any() {
                        drive::<Standard100k>(&mut img, i2c, pb8, pb9);
                    } else {
                        drive::<Fast400k>(&mut img, i2c, pb8, pb9);
                    }
                    check_final(&before, &img);
                    kani::cover!(true, "the end of a valid sequence is reachable");
                }
            }
        };
    }

    harness!(good);
    harness!(bug_wrong_af);
    harness!(bug_no_clock);
    harness!(bug_past_block);
}
