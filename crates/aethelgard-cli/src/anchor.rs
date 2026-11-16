//! The CLI's own watchpoint hooks and scheduled actions (`AET-WO-2302`,
//! ruled (a-i), D3, D4): the first monitor Python the CLI composes, and the
//! one strict reader of the lines it logs.
//!
//! Every board boots `MAX_VIRTUAL_NS`. Each witness's window is anchored
//! where its evidence starts, observed by these hooks:
//! - each motor timer's `CR1` write: logged with the word it held before
//!   (a hook runs before the write, measured), so the judge decides a `CEN`
//!   rise; on a DShot timer's rise the hook schedules the judge's stop at
//!   `DSHOT_WINDOW_NS` after it: the timer's solved registers read back,
//!   its observers silenced, `CEN` cleared;
//! - the boot word: on the write of `BOOT_MAGIC` over any other word, the
//!   first port's frame is scheduled a quantum after it, byte by byte a
//!   character apart rounded up to a quantum, the RX pin's mux read in the
//!   first byte's action;
//! - each port's `TDR` but the last: each byte written after this boot's
//!   boot word is counted, and the `FRAME_LEN`-th schedules the next port's
//!   frame a quantum after it (sequential, D4).
//!
//! Every action is checked stale first (F5): it acts only when the core's
//! executed instructions are exactly its anchor's plus its delay's, and
//! otherwise logs that it did nothing. A rendered reset restarts the count,
//! and an action from an earlier boot still fires in a later one.
//!
//! Measured in the pinned Renode 1.17.0 (`AET-WO-2302` build probes P1 to
//! P5): a `DoubleWord Write` hook runs before the write, so a read of its
//! address inside it returns the word before; a write by an action does not
//! fire a hook; a hook's Python globals and `sys` persist across calls and
//! across hooks; an action fires at exactly its anchor's count plus its
//! delay in instructions; a peripheral's hook survives `machine Reset`, but
//! a hook on RAM is not fired again after it (re-adding it fires both), so
//! the boot word's and the ports' hooks are added once, before the boot the
//! judge reads (the run's start, or the last release); and an observer an
//! earlier boot's stop silenced stays silenced across a reset, so each
//! reset sets the observers' level again.
//!
//! The lines are `[WARNING] sysbus: AETHELGARD-<TAG> <fields>`, each field
//! a decimal count or index, or a hexadecimal word as Python's `%x` writes
//! it. No specification text reaches the monitor: a timer is its number, a
//! port its place. The firmware cannot write a log line, but every line is
//! still read whole, strictly, after a validated timestamp.

use aethelgard_twin::boot::{BOOT_MAGIC, DSHOT_WINDOW_NS, MIPS, QUANTUM_NS};
use aethelgard_twin::serial::FRAME_LEN;

/// What every line of ours starts with, after the timestamp.
pub const PREFIX: &str = "[WARNING] sysbus: AETHELGARD-";

/// One line a hook or an action logged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hook {
    /// `CR1 <timer> <value> <before> <count>`: every `DoubleWord` write of a
    /// motor timer's `CR1`.
    Cr1 {
        timer: u32,
        value: u32,
        before: u32,
        count: u64,
    },
    /// `STOP <timer> <anchor> <count> <word>…`: the judge's stop, its timer's
    /// solved registers read back in the image's order, before `CEN` is
    /// cleared.
    Stop {
        timer: u32,
        anchor: u64,
        count: u64,
        words: Vec<u32>,
    },
    /// `CLEARED <timer> <anchor> <count> <cr1>`: `CR1` read after the stop
    /// cleared `CEN`.
    Cleared {
        timer: u32,
        anchor: u64,
        count: u64,
        cr1: u32,
    },
    /// `BOOTWORD <value> <before> <count>`: every `DoubleWord` write of the
    /// boot word.
    BootWord { value: u32, before: u32, count: u64 },
    /// `INJECT <port> <byte> <anchor> <count> [<moder> <afr>]`: a frame byte
    /// written into its port's USART, the RX pin's mux read with the first.
    Inject {
        port: usize,
        byte: usize,
        anchor: u64,
        count: u64,
        mux: Option<(u32, u32)>,
    },
    /// `ECHO <port> <n> <value> <count>`: the `n`-th `TDR` write of a port
    /// after this boot's boot word.
    Echo {
        port: usize,
        n: u64,
        value: u32,
        count: u64,
    },
    /// `STALE STOP <timer> <anchor> <count>` or `STALE INJECT <port> <byte>
    /// <anchor> <count>`: an action whose count was not its anchor's plus
    /// its delay's, which did nothing.
    Stale {
        what: Stale,
        anchor: u64,
        count: u64,
    },
}

/// What a stale action would have done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stale {
    Stop { timer: u32 },
    Inject { port: usize, byte: usize },
}

impl Hook {
    /// The core's executed instructions when the line was logged.
    pub fn count(&self) -> u64 {
        match self {
            Self::Cr1 { count, .. }
            | Self::Stop { count, .. }
            | Self::Cleared { count, .. }
            | Self::BootWord { count, .. }
            | Self::Inject { count, .. }
            | Self::Echo { count, .. }
            | Self::Stale { count, .. } => *count,
        }
    }
}

/// A decimal field: digits, no leading zero but `0` itself.
fn dec(s: &str) -> Option<u64> {
    let ok = !s.is_empty()
        && s.len() <= 20
        && s.bytes().all(|b| b.is_ascii_digit())
        && (s == "0" || !s.starts_with('0'));
    ok.then(|| s.parse().ok()).flatten()
}

/// A hexadecimal word as Python's `%x` writes it: lower case, no prefix, no
/// leading zero but `0` itself.
fn hex(s: &str) -> Option<u32> {
    let ok = !s.is_empty()
        && s.len() <= 8
        && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && (s == "0" || !s.starts_with('0'));
    ok.then(|| u32::from_str_radix(s, 16).ok()).flatten()
}

fn index(s: &str) -> Option<usize> {
    dec(s).and_then(|n| usize::try_from(n).ok())
}

fn timer(s: &str) -> Option<u32> {
    dec(s).and_then(|n| u32::try_from(n).ok())
}

/// A line's text after its timestamp: `None` when it is not ours,
/// `Some(None)` when it starts as ours and is not exactly one of the shapes
/// above, which the judge refuses.
pub fn parse(body: &str) -> Option<Option<Hook>> {
    let rest = body.strip_prefix(PREFIX)?;
    let f: Vec<&str> = rest.split(' ').collect();
    Some(match f.as_slice() {
        ["CR1", t, v, b, c] => (|| {
            Some(Hook::Cr1 {
                timer: timer(t)?,
                value: hex(v)?,
                before: hex(b)?,
                count: dec(c)?,
            })
        })(),
        ["STOP", t, a, c, words @ ..] if !words.is_empty() => (|| {
            Some(Hook::Stop {
                timer: timer(t)?,
                anchor: dec(a)?,
                count: dec(c)?,
                words: words.iter().map(|w| hex(w)).collect::<Option<_>>()?,
            })
        })(),
        ["CLEARED", t, a, c, cr1] => (|| {
            Some(Hook::Cleared {
                timer: timer(t)?,
                anchor: dec(a)?,
                count: dec(c)?,
                cr1: hex(cr1)?,
            })
        })(),
        ["BOOTWORD", v, b, c] => (|| {
            Some(Hook::BootWord {
                value: hex(v)?,
                before: hex(b)?,
                count: dec(c)?,
            })
        })(),
        ["INJECT", p, k, a, c] => (|| {
            Some(Hook::Inject {
                port: index(p)?,
                byte: index(k)?,
                anchor: dec(a)?,
                count: dec(c)?,
                mux: None,
            })
        })(),
        ["INJECT", p, "0", a, c, m, af] => (|| {
            Some(Hook::Inject {
                port: index(p)?,
                byte: 0,
                anchor: dec(a)?,
                count: dec(c)?,
                mux: Some((hex(m)?, hex(af)?)),
            })
        })(),
        ["ECHO", p, n, v, c] => (|| {
            Some(Hook::Echo {
                port: index(p)?,
                n: dec(n)?,
                value: hex(v)?,
                count: dec(c)?,
            })
        })(),
        ["STALE", "STOP", t, a, c] => (|| {
            Some(Hook::Stale {
                what: Stale::Stop { timer: timer(t)? },
                anchor: dec(a)?,
                count: dec(c)?,
            })
        })(),
        ["STALE", "INJECT", p, k, a, c] => (|| {
            Some(Hook::Stale {
                what: Stale::Inject {
                    port: index(p)?,
                    byte: index(k)?,
                },
                anchor: dec(a)?,
                count: dec(c)?,
            })
        })(),
        _ => None,
    })
}

/// The instructions a delay of `ns` takes at the core's `MIPS`.
pub fn instructions(ns: u64) -> u64 {
    ns * MIPS / 1000
}

/// The instant of instruction count `count` in a boot that started at
/// `start_ns`: 10 ns an instruction.
pub fn instant(start_ns: u64, count: u64) -> u64 {
    start_ns.saturating_add(count.saturating_mul(1000 / MIPS))
}

/// One motor timer as the composition hooks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerHook {
    /// `1` for `TIM1`.
    pub timer: u32,
    /// `CR1`'s address.
    pub cr1: u32,
    /// The timer's solved registers, in the image's order: what the stop
    /// reads back.
    pub registers: Vec<u32>,
    /// Each of its outputs' observers, `gpioPortA.pa8obs`.
    pub observers: Vec<String>,
    /// A DShot timer is stopped `DSHOT_WINDOW_NS` after its anchor; a PWM
    /// timer runs to the end of the boot.
    pub dshot: bool,
}

/// One serial port as the composition injects it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortHook {
    /// Renode's name of its USART, `usart1`.
    pub renode: &'static str,
    /// `TDR`'s address.
    pub tdr: u32,
    /// The RX pin's `MODER` and `AFR` addresses, read with the first byte.
    pub rx: (u32, u32),
    pub frame: [u8; FRAME_LEN],
    /// The bytes' spacing, a whole number of quanta.
    pub spacing_ns: u64,
}

/// The monitor's form of a watchpoint hook on writes of a word.
fn hook(address: u32, python: &str) -> String {
    debug_assert!(!python.contains('"') && !python.contains('\\'));
    format!("sysbus AddWatchpointHook {address:#010x} DoubleWord Write \"{python}\"")
}

/// A scheduled action, `delay_ns` after the hook that schedules it, that
/// does `act` (a Python expression) only when the count is its anchor's plus
/// the delay's, and otherwise logs `stale` (the fields after `STALE`) and
/// does nothing. The anchor is the hook's count, bound when it is scheduled.
fn action(delay_ns: u64, act: &str, stale: &str) -> String {
    debug_assert!(delay_ns.is_multiple_of(1000));
    format!(
        "self.Machine.ScheduleAction(TimeInterval.FromMicroseconds({}), lambda t, a=cpu.ExecutedInstructions: ({act}) if cpu.ExecutedInstructions == a + {} else self.Log(LogLevel.Warning, '{PREFIX_TAG}STALE {stale} %d %d' % (a, cpu.ExecutedInstructions)))",
        delay_ns / 1000,
        instructions(delay_ns)
    )
}

/// What Python's `self.Log` writes for `[WARNING] sysbus: ` (the prefix
/// without it).
const PREFIX_TAG: &str = "AETHELGARD-";

/// The hook on a motor timer's `CR1`: each write logged with the word
/// before it; on a DShot timer's `CEN` rise, the judge's stop scheduled.
pub fn timer_hook(t: &TimerHook) -> String {
    let n = t.timer;
    let cr1 = t.cr1;
    let mut py = format!(
        "from Antmicro.Renode.Time import TimeInterval; b = self.ReadDoubleWord({cr1:#x}); self.Log(LogLevel.Warning, '{PREFIX_TAG}CR1 {n} %x %x %d' % (value, b, cpu.ExecutedInstructions))"
    );
    if t.dshot {
        let words = vec!["%x"; t.registers.len()].join(" ");
        let reads: Vec<String> = t
            .registers
            .iter()
            .map(|r| format!("self.ReadDoubleWord({r:#x})"))
            .collect();
        let mut act = vec![format!(
            "self.Log(LogLevel.Warning, '{PREFIX_TAG}STOP {n} %d %d {words}' % (a, cpu.ExecutedInstructions, {}))",
            reads.join(", ")
        )];
        act.extend(
            t.observers
                .iter()
                .map(|o| format!("monitor.Parse('logLevel 3 sysbus.{o}')")),
        );
        act.push(format!(
            "self.WriteDoubleWord({cr1:#x}, self.ReadDoubleWord({cr1:#x}) & 0xfffffffe)"
        ));
        act.push(format!(
            "self.Log(LogLevel.Warning, '{PREFIX_TAG}CLEARED {n} %d %d %x' % (a, cpu.ExecutedInstructions, self.ReadDoubleWord({cr1:#x})))"
        ));
        py.push_str(&format!(
            "; (value & 1) and not (b & 1) and {}",
            action(DSHOT_WINDOW_NS, &act.join(", "), &format!("STOP {n}"))
        ));
    }
    hook(cr1, &py)
}

/// Port `k`'s frame, scheduled from the hook that anchors it: byte `j` a
/// quantum and `j` spacings later, the RX pin's mux read with the first.
fn frame_actions(k: usize, p: &PortHook) -> String {
    let actions: Vec<String> = p
        .frame
        .iter()
        .enumerate()
        .map(|(j, b)| {
            let log = if j == 0 {
                format!(
                    "self.Log(LogLevel.Warning, '{PREFIX_TAG}INJECT {k} 0 %d %d %x %x' % (a, cpu.ExecutedInstructions, self.ReadDoubleWord({:#x}), self.ReadDoubleWord({:#x})))",
                    p.rx.0, p.rx.1
                )
            } else {
                format!(
                    "self.Log(LogLevel.Warning, '{PREFIX_TAG}INJECT {k} {j} %d %d' % (a, cpu.ExecutedInstructions))"
                )
            };
            let write = format!("monitor.Parse('sysbus.{} WriteChar {b:#04x}')", p.renode);
            action(
                QUANTUM_NS + j as u64 * p.spacing_ns,
                &format!("{log}, {write}"),
                &format!("INJECT {k} {j}"),
            )
        })
        .collect();
    format!("({})", actions.join(", "))
}

/// The hook on the boot word: each write logged with the word before it;
/// on `BOOT_MAGIC` written over any other word, this boot's echo counts
/// reset and the first port's frame scheduled.
pub fn boot_word_hook(boot_word: u32, ports: &[PortHook]) -> String {
    let mut py = format!(
        "from Antmicro.Renode.Time import TimeInterval; import sys; b = self.ReadDoubleWord({boot_word:#x}); self.Log(LogLevel.Warning, '{PREFIX_TAG}BOOTWORD %x %x %d' % (value, b, cpu.ExecutedInstructions)); go = value == {BOOT_MAGIC:#x} and b != {BOOT_MAGIC:#x}"
    );
    if let Some(first) = ports.first() {
        py.push_str(&format!(
            "; go and setattr(sys, 'aethelgard_echo', [0] * {}); go and {}",
            ports.len(),
            frame_actions(0, first)
        ));
    }
    hook(boot_word, &py)
}

/// The hook on port `k`'s `TDR` (`k` not the last): each write after this
/// boot's boot word counted and logged, and the `FRAME_LEN`-th schedules
/// port `k + 1`'s frame.
pub fn echo_hook(k: usize, ports: &[PortHook], boot_word: u32) -> Option<String> {
    let (p, next) = (ports.get(k)?, ports.get(k + 1)?);
    let py = format!(
        "from Antmicro.Renode.Time import TimeInterval; import sys; e = getattr(sys, 'aethelgard_echo', None); ok = e is not None and self.ReadDoubleWord({boot_word:#x}) == {BOOT_MAGIC:#x}; ok and e.__setitem__({k}, e[{k}] + 1); ok and self.Log(LogLevel.Warning, '{PREFIX_TAG}ECHO {k} %d %x %d' % (e[{k}], value, cpu.ExecutedInstructions)); ok and e[{k}] == {FRAME_LEN} and {}",
        frame_actions(k + 1, next)
    );
    Some(hook(p.tdr, &py))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dshot() -> TimerHook {
        TimerHook {
            timer: 1,
            cr1: 0x4001_0000,
            registers: vec![0x4001_0000, 0x4001_002c],
            observers: vec!["gpioPortA.pa8obs".into(), "gpioPortA.pa9obs".into()],
            dshot: true,
        }
    }

    fn port(renode: &'static str, tdr: u32) -> PortHook {
        PortHook {
            renode,
            tdr,
            rx: (0x5802_0400, 0x5802_0424),
            frame: [0xba, 0x17, 0xfc, 0xb3, 0x40, 0x9d, 0x09, 0x7a],
            spacing_ns: 1_100_000,
        }
    }

    /// The composed text is the form measured in the pinned Renode (build
    /// probe P5): one monitor command, the Python in double quotes with
    /// single-quoted strings inside, no backslash.
    #[test]
    fn a_dshot_timer_s_hook_logs_every_write_and_schedules_the_stop_on_a_rise() {
        let h = timer_hook(&dshot());
        assert!(h.starts_with("sysbus AddWatchpointHook 0x40010000 DoubleWord Write \""));
        assert!(h.ends_with('"'));
        assert_eq!(h.matches('"').count(), 2, "{h}");
        assert!(!h.contains('\\'));
        assert!(h.contains("b = self.ReadDoubleWord(0x40010000); self.Log(LogLevel.Warning, 'AETHELGARD-CR1 1 %x %x %d' % (value, b, cpu.ExecutedInstructions))"));
        // The rise, decided before the write: CEN in the word written and
        // not in the word before.
        assert!(h.contains("(value & 1) and not (b & 1) and self.Machine.ScheduleAction(TimeInterval.FromMicroseconds(2000), lambda t, a=cpu.ExecutedInstructions: ("));
        // Stale first: the stop acts at exactly its anchor plus 2 ms of
        // instructions, and otherwise logs that it did nothing.
        assert!(h.contains(") if cpu.ExecutedInstructions == a + 200000 else self.Log(LogLevel.Warning, 'AETHELGARD-STALE STOP 1 %d %d' % (a, cpu.ExecutedInstructions)))"));
        // The read-back, then the observers silenced, then CEN cleared and
        // read again, in that order.
        let stop = h.find("AETHELGARD-STOP 1 %d %d %x %x' % (a, cpu.ExecutedInstructions, self.ReadDoubleWord(0x40010000), self.ReadDoubleWord(0x4001002c))").unwrap();
        let silence = h.find("monitor.Parse('logLevel 3 sysbus.gpioPortA.pa8obs'), monitor.Parse('logLevel 3 sysbus.gpioPortA.pa9obs')").unwrap();
        let clear = h.find("self.WriteDoubleWord(0x40010000, self.ReadDoubleWord(0x40010000) & 0xfffffffe)").unwrap();
        let cleared = h.find("'AETHELGARD-CLEARED 1 %d %d %x' % (a, cpu.ExecutedInstructions, self.ReadDoubleWord(0x40010000))").unwrap();
        assert!(stop < silence && silence < clear && clear < cleared);
    }

    #[test]
    fn a_pwm_timer_s_hook_only_logs() {
        let h = timer_hook(&TimerHook {
            dshot: false,
            ..dshot()
        });
        assert!(h.contains("'AETHELGARD-CR1 1 %x %x %d'"));
        assert!(!h.contains("ScheduleAction"), "{h}");
        assert!(!h.contains("WriteDoubleWord"), "{h}");
    }

    #[test]
    fn the_boot_word_schedules_the_first_port_s_frame_byte_by_byte() {
        let ports = [port("usart1", 0x4001_1028), port("usart2", 0x4000_4428)];
        let h = boot_word_hook(0x2000_0000, &ports);
        assert!(h.starts_with("sysbus AddWatchpointHook 0x20000000 DoubleWord Write \""));
        assert_eq!(h.matches('"').count(), 2, "{h}");
        assert!(h.contains("go = value == 0x2100b007 and b != 0x2100b007; go and setattr(sys, 'aethelgard_echo', [0] * 2); go and ("));
        // Byte j a quantum and j spacings after the boot word, each stale-checked.
        for (j, b) in ports[0].frame.iter().enumerate() {
            let us = 100 + 1100 * j;
            let due = 100 * us;
            assert!(
                h.contains(&format!("FromMicroseconds({us}), lambda t, a=cpu.ExecutedInstructions: (")),
                "byte {j}"
            );
            assert!(h.contains(&format!("monitor.Parse('sysbus.usart1 WriteChar {b:#04x}')) if cpu.ExecutedInstructions == a + {due} else self.Log(LogLevel.Warning, 'AETHELGARD-STALE INJECT 0 {j} %d %d'")), "byte {j}");
        }
        // The RX pin's mux is read with the first byte, before it goes in.
        assert!(h.contains("'AETHELGARD-INJECT 0 0 %d %d %x %x' % (a, cpu.ExecutedInstructions, self.ReadDoubleWord(0x58020400), self.ReadDoubleWord(0x58020424))), monitor.Parse('sysbus.usart1 WriteChar 0xba')"));
        // Only the first port goes in from the boot word (D4).
        assert!(!h.contains("usart2"), "{h}");
        // Without a port, the hook only logs.
        let bare = boot_word_hook(0x2000_0000, &[]);
        assert!(!bare.contains("ScheduleAction") && !bare.contains("aethelgard_echo"));
    }

    #[test]
    fn a_port_s_last_echoed_byte_schedules_the_next_port() {
        let ports = [port("usart1", 0x4001_1028), port("usart2", 0x4000_4428)];
        let h = echo_hook(0, &ports, 0x2000_0000).unwrap();
        assert!(h.starts_with("sysbus AddWatchpointHook 0x40011028 DoubleWord Write \""));
        // Counted only after this boot's boot word, which the reset re-arms.
        assert!(h.contains("ok = e is not None and self.ReadDoubleWord(0x20000000) == 0x2100b007; ok and e.__setitem__(0, e[0] + 1)"));
        assert!(h.contains("ok and e[0] == 8 and (self.Machine.ScheduleAction(TimeInterval.FromMicroseconds(100), lambda t, a=cpu.ExecutedInstructions: (self.Log(LogLevel.Warning, 'AETHELGARD-INJECT 1 0 %d %d %x %x'"));
        assert!(h.contains("monitor.Parse('sysbus.usart2 WriteChar 0xba')"));
        // The last port schedules nothing.
        assert!(echo_hook(1, &ports, 0x2000_0000).is_none());
    }

    #[test]
    fn every_line_is_read_whole_and_strictly() {
        let p = |s: &str| parse(s);
        assert_eq!(
            p("[WARNING] sysbus: AETHELGARD-CR1 1 81 0 3067"),
            Some(Some(Hook::Cr1 {
                timer: 1,
                value: 0x81,
                before: 0,
                count: 3067
            }))
        );
        assert_eq!(
            p("[WARNING] sysbus: AETHELGARD-STOP 1 3067 203067 81 0 6a"),
            Some(Some(Hook::Stop {
                timer: 1,
                anchor: 3067,
                count: 203_067,
                words: vec![0x81, 0, 0x6a]
            }))
        );
        assert_eq!(
            p("[WARNING] sysbus: AETHELGARD-INJECT 0 0 3075 13075 a000a000 44000000"),
            Some(Some(Hook::Inject {
                port: 0,
                byte: 0,
                anchor: 3075,
                count: 13075,
                mux: Some((0xa000_a000, 0x4400_0000))
            }))
        );
        assert_eq!(
            p("[WARNING] sysbus: AETHELGARD-STALE STOP 1 2167 168997"),
            Some(Some(Hook::Stale {
                what: Stale::Stop { timer: 1 },
                anchor: 2167,
                count: 168_997
            }))
        );
        // Not ours.
        assert_eq!(p("[WARNING] sysbus: AETCEN 2167"), None);
        assert_eq!(p("[NOISY] gpioPortA.pa8obs: LED state changed to True"), None);
        // Ours, but not exactly a shape: refused, never read past.
        for bad in [
            "[WARNING] sysbus: AETHELGARD-CR1 1 81 0",
            "[WARNING] sysbus: AETHELGARD-CR1 1 0x81 0 3067",
            "[WARNING] sysbus: AETHELGARD-CR1 1 81 0 3067 ",
            "[WARNING] sysbus: AETHELGARD-CR1 1 81  0 3067",
            "[WARNING] sysbus: AETHELGARD-CR1 1 81 00 3067",
            "[WARNING] sysbus: AETHELGARD-CR1 1 81 0 03067",
            "[WARNING] sysbus: AETHELGARD-CR1 1 8A 0 3067",
            "[WARNING] sysbus: AETHELGARD-CR1 1 100000000 0 3067",
            "[WARNING] sysbus: AETHELGARD-INJECT 0 1 3075 13075 a000a000 44000000",
            "[WARNING] sysbus: AETHELGARD-STOP 1 3067 203067",
            "[WARNING] sysbus: AETHELGARD-STALE NOW 1 2 3",
            "[WARNING] sysbus: AETHELGARD-",
        ] {
            assert_eq!(p(bad), Some(None), "{bad}");
        }
    }
}
