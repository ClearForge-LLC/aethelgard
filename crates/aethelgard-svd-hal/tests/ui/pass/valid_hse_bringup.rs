//! `AET-WO-4240b`, run on the host: the generated `Hse::start` of the 16 MHz
//! crystal board on a model of RCC and the flash controller written from the
//! PM's contract and RM0433 Rev 9 (§8.7.2, §8.7.7, §4.9.1), never from the
//! generator. The case sets whether `HSERDY`, `PLL1RDY` and `SWS` ever rise
//! and whether `FLASH_ACR` takes its word. Each case checks the outcome (each
//! timeout its own `ClockFault`, which `main` maps to its own boot word) and
//! every write, in order: nothing is written after a fault. Kani proves the
//! sequence on any reads (`hse_start_is_in_order_bounded_and_named`).
//! `Board::take` hands out the one `Hse` once per program, so each case runs
//! in a child process of its own.
#![allow(dead_code)]

#[path = "../hse-crystal/board.rs"]
mod board;

use board::{Block, Board, ClockFault, Registers, sealed};

const CR: usize = 0x000;
const CFGR: usize = 0x010;
const HSERDY: u32 = 1 << 17;
const PLL1RDY: u32 = 1 << 25;
const POLLS: u32 = 3;
/// The contract's words for the 16 MHz crystal (`4240` FEEDBACK §4.5).
const WORDS: [(Block, usize, u32); 8] = [
    (Block::Rcc, 0x000, 0x0001_0001),
    (Block::Rcc, 0x000, 0x0009_0001),
    (Block::Rcc, 0x028, 0x0000_0052),
    (Block::Rcc, 0x02C, 0x0003_0004),
    (Block::Rcc, 0x030, 0x0103_064F),
    (Block::Rcc, 0x000, 0x0109_0001),
    (Block::Flash, 0x000, 0x0000_0011),
    (Block::Rcc, 0x010, 0x0000_0003),
];

struct Model {
    rcc: [u32; 64],
    acr: u32,
    hse_rises: bool,
    pll_locks: bool,
    acr_takes: bool,
    sw_takes: bool,
    writes: Vec<(Block, usize, u32)>,
    /// Reads of CR while HSEON is set (HSERDY's wait), and of CFGR.
    cr_reads: u32,
    cfgr_reads: u32,
}

impl sealed::Sealed for Model {}

impl Registers for Model {
    fn read(&mut self, _: sealed::Token, block: Block, offset: usize) -> u32 {
        match block {
            Block::Pwr => {
                if offset == 0x0C {
                    0x42
                } else {
                    !0
                }
            }
            Block::Flash => {
                assert_eq!(offset, 0x000);
                self.acr
            }
            Block::Rcc if offset == CR => {
                self.cr_reads += 1;
                let v = self.rcc[0];
                let mut r = v;
                if v & (1 << 16) != 0 && self.hse_rises {
                    r |= HSERDY;
                }
                if v & (1 << 24) != 0 && self.pll_locks {
                    r |= PLL1RDY;
                }
                r
            }
            Block::Rcc if offset == CFGR => {
                self.cfgr_reads += 1;
                let v = self.rcc[CFGR / 4];
                if self.sw_takes { v | ((v & 0b111) << 3) } else { v }
            }
            Block::Rcc => self.rcc[offset / 4],
            _ => panic!("read {block:?} {offset:#x}"),
        }
    }
    fn write(&mut self, _: sealed::Token, block: Block, offset: usize, value: u32) {
        match block {
            Block::Pwr => {}
            Block::Flash => {
                self.writes.push((block, offset, value));
                if self.acr_takes {
                    self.acr = value;
                }
            }
            Block::Rcc => {
                self.writes.push((block, offset, value));
                self.rcc[offset / 4] = value;
            }
            _ => panic!("wrote {block:?} {offset:#x}"),
        }
    }
    fn read8(&mut self, _: sealed::Token, _: Block, _: usize) -> u8 {
        panic!("byte read")
    }
    fn write8(&mut self, _: sealed::Token, _: Block, _: usize, _: u8) {
        panic!("byte write")
    }
}

fn run(hse_rises: bool, pll_locks: bool, acr_takes: bool, sw_takes: bool) -> (Result<(), ClockFault>, Model) {
    let mut m = Model {
        rcc: [0; 64],
        acr: 0x37,
        hse_rises,
        pll_locks,
        acr_takes,
        sw_takes,
        writes: Vec::new(),
        cr_reads: 0,
        cfgr_reads: 0,
    };
    let board = Board::take().unwrap();
    let scale2 = board.pwr.scale2(&mut m, POLLS).unwrap();
    let reset = board.rcc.reset_clock(&mut m, &scale2).unwrap();
    let result = board.hse.start(&mut m, reset, POLLS, POLLS, POLLS).map(|_| ());
    (result, m)
}

fn case(name: &str) {
    match name {
        // Every bit rises: the contract's eight words, in order, and the token.
        "ok" => {
            let (r, m) = run(true, true, true, true);
            assert_eq!(r, Ok(()));
            assert_eq!(m.writes, WORDS);
            assert_eq!(m.acr, 0x11);
        }
        // HSERDY never rises (BOOT_HSE_NOT_READY): HSEON, then the bound's
        // reads, then nothing.
        "hse" => {
            let (r, m) = run(false, true, true, true);
            assert_eq!(r, Err(ClockFault::HseNotReady));
            assert_eq!(m.writes, WORDS[..1]);
            assert_eq!(m.cr_reads, 1 + POLLS, "the reset clock's read, then the bound's");
        }
        // PLL1RDY never rises (BOOT_PLL1_NOT_LOCKED): up to PLL1ON, then nothing.
        "pll" => {
            let (r, m) = run(true, false, true, true);
            assert_eq!(r, Err(ClockFault::Pll1NotLocked));
            assert_eq!(m.writes, WORDS[..6]);
        }
        // FLASH_ACR keeps its reset word (BOOT_FLASH_ACR_NOT_TAKEN): no switch.
        "acr" => {
            let (r, m) = run(true, true, false, true);
            assert_eq!(r, Err(ClockFault::FlashAcrNotTaken));
            assert_eq!(m.writes, WORDS[..7]);
        }
        // SWS never reads PLL1 (BOOT_SW_NOT_TAKEN): every word, the bound's reads.
        "sws" => {
            let (r, m) = run(true, true, true, false);
            assert_eq!(r, Err(ClockFault::SwNotTaken));
            assert_eq!(m.writes, WORDS);
            assert_eq!(m.cfgr_reads, 1 + POLLS, "the reset clock's read, then the bound's");
        }
        _ => panic!("no case {name}"),
    }
}

const CASES: [&str; 5] = ["ok", "hse", "pll", "acr", "sws"];

fn main() {
    match std::env::args().nth(1) {
        Some(name) => case(&name),
        None => {
            let me = std::env::current_exe().unwrap();
            for name in CASES {
                let ok = std::process::Command::new(&me)
                    .arg(name)
                    .status()
                    .unwrap()
                    .success();
                assert!(ok, "case {name} failed");
            }
        }
    }
}
