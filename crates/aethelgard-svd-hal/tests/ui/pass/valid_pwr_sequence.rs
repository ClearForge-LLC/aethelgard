//! `AET-WO-4240v` §1.4, run on the host: the generated `Pwr::scale2` of the
//! `motors-pwm` board on a model of the power controller written from RM0433
//! Rev 9 (§6.8.2, §6.8.4, §6.8.6), never from the generator. `PWR_CR3` takes
//! its first write after power-on and ignores every later one (p. 308); the
//! case sets what `ACTVOSRDY` and `VOSRDY` read. Each case checks the outcome
//! and every access, in order. Kani proves the sequence on any reads
//! (`pwr_scale2_is_once_in_order_and_bounded`); these are named cases, the
//! adversarial pass's warm resets among them. `Board::take` hands out the one
//! `Pwr` once per program, so each case runs in a child process of its own.
#![allow(dead_code)]

#[path = "../motors-pwm/board.rs"]
mod board;

use board::{Block, Board, PwrFault, Registers, sealed};

const CSR1: usize = 0x04;
const CR3: usize = 0x0C;
const D3CR: usize = 0x18;
const READY: u32 = 1 << 13;
const POLLS: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Access {
    Read(usize, u32),
    Write(usize, u32),
}

struct Pwr {
    cr3: u32,
    /// `CR3` took its one write since power-on.
    locked: bool,
    actvosrdy: bool,
    vosrdy: bool,
    d3cr: u32,
    log: Vec<Access>,
}

impl sealed::Sealed for Pwr {}

impl Registers for Pwr {
    fn read(&mut self, _: sealed::Token, block: Block, offset: usize) -> u32 {
        assert_eq!(block, Block::Pwr);
        let v = match offset {
            CSR1 => 0x4000 | if self.actvosrdy { READY } else { 0 },
            CR3 => self.cr3,
            D3CR => (self.d3cr & !READY) | if self.vosrdy { READY } else { 0 },
            _ => panic!("read {offset:#x}"),
        };
        self.log.push(Access::Read(offset, v));
        v
    }
    fn write(&mut self, _: sealed::Token, block: Block, offset: usize, value: u32) {
        assert_eq!(block, Block::Pwr);
        self.log.push(Access::Write(offset, value));
        match offset {
            CR3 if !self.locked => {
                self.cr3 = value;
                self.locked = true;
            }
            CR3 => {}
            D3CR => self.d3cr = value,
            _ => panic!("wrote {offset:#x}"),
        }
    }
    fn read8(&mut self, _: sealed::Token, _: Block, _: usize) -> u8 {
        panic!("byte read")
    }
    fn write8(&mut self, _: sealed::Token, _: Block, _: usize, _: u8) {
        panic!("byte write")
    }
}

/// After power-on (`locked` false) or a warm reset (true), `CR3` holding
/// `cr3`.
fn pwr(cr3: u32, locked: bool, actvosrdy: bool, vosrdy: bool) -> Pwr {
    Pwr {
        cr3,
        locked,
        actvosrdy,
        vosrdy,
        d3cr: 0x4000,
        log: Vec::new(),
    }
}

fn run(p: &mut Pwr) -> Result<(), PwrFault> {
    Board::take().unwrap().pwr.scale2(p, POLLS).map(|_| ())
}

fn case(name: &str) {
    use Access::{Read, Write};
    let polls = |offset: usize, v: u32| vec![Read(offset, v); POLLS as usize];
    match name {
        // Power-on, revision V's reset (0x46): the supply bits written, bit 6
        // kept as read, read back as declared, then Scale 2.
        "por" => {
            let mut p = pwr(0x46, false, true, true);
            assert_eq!(run(&mut p), Ok(()));
            assert_eq!(
                p.log,
                [
                    Read(CR3, 0x46),
                    Write(CR3, 0x42),
                    Read(CR3, 0x42),
                    Read(CSR1, 0x6000),
                    Write(D3CR, 0x8000),
                    Read(D3CR, 0xA000),
                ]
            );
            assert_eq!((p.cr3, p.d3cr), (0x42, 0x8000));
        }
        // A warm reset after this firmware: the write is ignored, the read-back
        // is declared, and the sequence goes on.
        "warm_ours" => {
            let mut p = pwr(0x42, true, true, true);
            assert_eq!(run(&mut p), Ok(()));
            assert_eq!(p.log[..3], [Read(CR3, 0x42), Write(CR3, 0x42), Read(CR3, 0x42)]);
            assert_eq!(p.log.iter().filter(|a| matches!(a, Write(..))).count(), 2);
        }
        // A warm reset after ArduPilot's `LDOEN | USB33DEN` (4400c F30): the
        // supply bits are the declared ones, so it goes on; no second CR3 write.
        "warm_ardupilot" => {
            let mut p = pwr(0x0100_0002, true, true, true);
            assert_eq!(run(&mut p), Ok(()));
            assert_eq!(p.cr3, 0x0100_0002);
            assert_eq!(p.log[1], Write(CR3, 0x0100_0002));
            assert_eq!(p.log.iter().filter(|a| **a == Write(CR3, 0x0100_0002)).count(), 1);
        }
        // A warm reset after a writer that chose bypass: refused by name,
        // nothing written after the read-back.
        "warm_bypass" => {
            let mut p = pwr(0x01, true, true, true);
            assert_eq!(run(&mut p), Err(PwrFault::SupplyNotDeclared));
            assert_eq!(p.log, [Read(CR3, 0x01), Write(CR3, 0x02), Read(CR3, 0x01)]);
        }
        // `ACTVOSRDY` never rises: the bound's reads of CSR1, then nothing.
        "actvosrdy" => {
            let mut p = pwr(0x46, false, false, true);
            assert_eq!(run(&mut p), Err(PwrFault::ActvosrdyTimeout));
            let mut want = vec![Read(CR3, 0x46), Write(CR3, 0x42), Read(CR3, 0x42)];
            want.extend(polls(CSR1, 0x4000));
            assert_eq!(p.log, want);
            assert_eq!(p.d3cr, 0x4000);
        }
        // `VOSRDY` never rises: D3CR written once, then the bound's reads.
        "vosrdy" => {
            let mut p = pwr(0x46, false, true, false);
            assert_eq!(run(&mut p), Err(PwrFault::VosrdyTimeout));
            let mut want = vec![
                Read(CR3, 0x46),
                Write(CR3, 0x42),
                Read(CR3, 0x42),
                Read(CSR1, 0x6000),
                Write(D3CR, 0x8000),
            ];
            want.extend(polls(D3CR, 0x8000));
            assert_eq!(p.log, want);
        }
        _ => panic!("no case {name}"),
    }
}

const CASES: [&str; 6] = [
    "por",
    "warm_ours",
    "warm_ardupilot",
    "warm_bypass",
    "actvosrdy",
    "vosrdy",
];

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
