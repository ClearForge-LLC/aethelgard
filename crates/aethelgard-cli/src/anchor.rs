//! The CLI's own watchpoint hooks and scheduled actions (`AET-WO-2302`,
//! ruled (a-i), D3, D4): the first monitor Python the CLI composes, and the
//! one strict reader of the lines it logs.
//!
//! Every board boots `MAX_VIRTUAL_NS`. Each witness's window is anchored
//! where its evidence starts, observed by these hooks:
//! - each motor timer's `CR1` write: logged with the word it held before
//!   (a hook runs before the write, measured), so the judge decides a `CEN`
//!   rise; on a DShot timer's rise the hook schedules the judge's stop at
//!   `DSHOT_WINDOW_NS` after it: the timer's solved registers read back and
//!   `CEN` cleared. The observers are never silenced: a stopped timer makes
//!   no edge, so every edge after the window is the firmware's own (a
//!   restart, a pin driven by hand) and the judge names it;
//! - the boot word: on the write of `BOOT_MAGIC` over any other word, the
//!   first port's frame is scheduled a quantum after it, byte by byte a
//!   character apart rounded up to a quantum, the RX pin's mux read in the
//!   first byte's action;
//! - each port's `TDR` but the last: each byte written after this boot's
//!   boot word is logged, and, once the port's first frame byte went in,
//!   each that echoes its frame in order is counted: the `FRAME_LEN`-th
//!   schedules the next port's frame a quantum after it (sequential, D4).
//!   A byte that is not the frame's next (a pattern sent late, junk) is
//!   never counted (adversarial pass 1, L1); the judge recounts from the
//!   logged bytes and names a port injected before its predecessor's last
//!   echoed frame byte (L2).
//!
//! A port whose RX line the scenario holds open (D5, `serial_rx_open`) gets
//! no frame and no `TDR` hook: the chain skips it, so the port after it goes
//! in a quantum after the last echoed byte of the port before it that is
//! not open (or after the boot word, when none is). The firmware still
//! polls the open port for its echo while that frame waits in Renode's
//! queue, so the next port's pacing is not tested in such a run (F1's lost
//! test, never a false pass); its echo is still judged.
//!
//! A pin device (`AET-WO-1211c`) adds two hooks, each logging and acting on
//! nothing:
//! - each declared output's port's `BSRR` and `MODER`: every write logged
//!   with the word before it, so the judge decides the output's level write
//!   and its order before `MODER` makes it an output (`1211` §4 G1);
//! - each ADC input's `DR`: every read logged, so the judge sees the
//!   firmware read it. A `Read` hook's `value` is 0, never the word read
//!   (measured, `1211c` H-probe), so the code is read back after the run.
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
//! judge reads (the run's start, or the last release).
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
    /// `TDR <port> <value> <count>`: a `TDR` write of a port after this
    /// boot's boot word.
    Tdr { port: usize, value: u32, count: u64 },
    /// `STEP <check> <value> <count>`: every `DoubleWord` write of an I2C
    /// step check's status word (`AET-WO-3305`), by the check's index in the
    /// image: where the firmware ended that step in the log.
    Step {
        check: usize,
        value: u32,
        count: u64,
    },
    /// `GPIO <address> <value> <before> <count>`: every `DoubleWord` write
    /// of a declared output's port's `BSRR` or `MODER` (`AET-WO-1211c`).
    Gpio {
        address: u32,
        value: u32,
        before: u32,
        count: u64,
    },
    /// `DR <input> <count>`: every `DoubleWord` read of an ADC input's data
    /// register, by the input's index in the plan (`AET-WO-1211c`).
    Dr { input: usize, count: u64 },
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
    /// Whether a scheduled action logged the line (a stop, its `CEN`
    /// cleared, an injected byte, a stale action), rather than a hook on a
    /// write.
    pub fn scheduled(&self) -> bool {
        matches!(
            self,
            Self::Stop { .. } | Self::Cleared { .. } | Self::Inject { .. } | Self::Stale { .. }
        )
    }

    /// The core's executed instructions when the line was logged.
    pub fn count(&self) -> u64 {
        match self {
            Self::Cr1 { count, .. }
            | Self::Stop { count, .. }
            | Self::Cleared { count, .. }
            | Self::BootWord { count, .. }
            | Self::Inject { count, .. }
            | Self::Tdr { count, .. }
            | Self::Step { count, .. }
            | Self::Gpio { count, .. }
            | Self::Dr { count, .. }
            | Self::Stale { count, .. } => *count,
        }
    }
}

/// A decimal field: digits, no leading zero but `0` itself, and in a `u64`
/// (`parse` refuses the rest).
fn dec(s: &str) -> Option<u64> {
    let ok =
        !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s == "0" || !s.starts_with('0'));
    ok.then(|| s.parse().ok()).flatten()
}

/// A hexadecimal word as Python's `%x` writes it: lower case, no prefix, no
/// leading zero but `0` itself, and in a `u32` (`from_str_radix` refuses the
/// rest).
fn hex(s: &str) -> Option<u32> {
    let ok = !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
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
        ["INJECT", p, k, a, c] if *k != "0" => (|| {
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
        ["TDR", p, v, c] => (|| {
            Some(Hook::Tdr {
                port: index(p)?,
                value: hex(v)?,
                count: dec(c)?,
            })
        })(),
        ["STEP", k, v, c] => (|| {
            Some(Hook::Step {
                check: index(k)?,
                value: hex(v)?,
                count: dec(c)?,
            })
        })(),
        ["GPIO", at, v, b, c] => (|| {
            Some(Hook::Gpio {
                address: hex(at)?,
                value: hex(v)?,
                before: hex(b)?,
                count: dec(c)?,
            })
        })(),
        ["DR", k, c] => (|| {
            Some(Hook::Dr {
                input: index(k)?,
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
    /// The scenario holds its RX line open: nothing goes in (D5).
    pub open: bool,
}

/// The first port at or after `k` that is not held open: the next in the
/// sequential chain.
fn next_injected(ports: &[PortHook], k: usize) -> Option<usize> {
    (k..ports.len()).find(|&j| !ports[j].open)
}

/// The monitor's form of a watchpoint hook on writes of a word.
fn hook(address: u32, python: &str) -> String {
    hook_of(address, "DoubleWord", python)
}

/// The monitor's form of a watchpoint hook on writes `width` wide
/// (`Byte`, `Word` or `DoubleWord`).
fn hook_of(address: u32, width: &str, python: &str) -> String {
    debug_assert!(!python.contains('"') && !python.contains('\\'));
    format!("sysbus AddWatchpointHook {address:#010x} {width} Write \"{python}\"")
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

/// The hooks on a motor timer's `CR1`, one per store width (`DoubleWord`,
/// `Word`, `Byte`; adversarial pass 1, M2): each write logged with the word
/// before it; on a DShot timer's `CEN` rise, the judge's stop scheduled.
/// Each width fires only on its own stores (measured, fix round M2), and a
/// store narrower than a word at `CR1`'s address holds `CEN` in its bit 0
/// as a word does, so one composition serves all three. A store at
/// `CR1 + 1` to `+ 3` cannot touch `CEN` and is not hooked; a DMA write
/// fires no hook at all, and its edges fail closed as
/// `motor_anchor_unobserved`.
pub fn timer_hooks(t: &TimerHook) -> Vec<String> {
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
    ["DoubleWord", "Word", "Byte"]
        .iter()
        .map(|w| hook_of(cr1, w, &py))
        .collect()
}

/// Port `k`'s frame, scheduled from the hook that anchors it: byte `j` a
/// quantum and `j` spacings later, the RX pin's mux read with the first,
/// which also starts the port's echo count (0, from the boot word's -1).
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
            let mut act = format!(
                "{log}, monitor.Parse('sysbus.{} WriteChar {b:#04x}')",
                p.renode
            );
            if j == 0 {
                act.push_str(&format!(
                    ", __import__('sys').aethelgard_echo.__setitem__({k}, 0)"
                ));
            }
            action(
                QUANTUM_NS + j as u64 * p.spacing_ns,
                &act,
                &format!("INJECT {k} {j}"),
            )
        })
        .collect();
    format!("({})", actions.join(", "))
}

/// The hook on an I2C step check's status word (`AET-WO-3305`): each write
/// logged with its check's index, so the judge can attribute an address the
/// controller found no device at to the step under way.
pub fn step_hook(check: usize, status_at: u32) -> String {
    hook(
        status_at,
        &format!(
            "self.Log(LogLevel.Warning, '{PREFIX_TAG}STEP {check} %x %d' % (value, cpu.ExecutedInstructions))"
        ),
    )
}

/// The hook on a declared output's port register, `BSRR` or `MODER`
/// (`AET-WO-1211c`): each `DoubleWord` write logged with the word before it.
/// A narrower store, or one at another offset, fires no hook, and the
/// output fails closed as unwitnessed.
pub fn gpio_hook(address: u32) -> String {
    hook(
        address,
        &format!(
            "b = self.ReadDoubleWord({address:#x}); self.Log(LogLevel.Warning, '{PREFIX_TAG}GPIO %x %x %x %d' % ({address:#x}, value, b, cpu.ExecutedInstructions))"
        ),
    )
}

/// The hook on an ADC input's data register (`AET-WO-1211c`): each
/// `DoubleWord` read logged by the input's index.
pub fn dr_hook(input: usize, address: u32) -> String {
    format!(
        "sysbus AddWatchpointHook {address:#010x} DoubleWord Read \"self.Log(LogLevel.Warning, '{PREFIX_TAG}DR {input} %d' % (cpu.ExecutedInstructions))\""
    )
}

/// The hook on the boot word: each write logged with the word before it;
/// on `BOOT_MAGIC` written over any other word, this boot's echo counts
/// reset (-1: no frame in yet) and the first port not held open has its
/// frame scheduled.
pub fn boot_word_hook(boot_word: u32, ports: &[PortHook]) -> String {
    let mut py = format!(
        "from Antmicro.Renode.Time import TimeInterval; import sys; b = self.ReadDoubleWord({boot_word:#x}); self.Log(LogLevel.Warning, '{PREFIX_TAG}BOOTWORD %x %x %d' % (value, b, cpu.ExecutedInstructions)); go = value == {BOOT_MAGIC:#x} and b != {BOOT_MAGIC:#x}"
    );
    if let Some(first) = next_injected(ports, 0) {
        py.push_str(&format!(
            "; go and setattr(sys, 'aethelgard_echo', [-1] * {}); go and {}",
            ports.len(),
            frame_actions(first, &ports[first])
        ));
    }
    hook(boot_word, &py)
}

/// The hook on port `k`'s `TDR` (`k` neither held open nor the last port
/// that is not): each write after this boot's boot word logged; once port
/// `k`'s first frame byte went in, a write of the frame's next byte is
/// counted (adversarial pass 1, L1: never a pattern byte or junk), and the
/// `FRAME_LEN`-th schedules the frame of the next port not held open.
pub fn echo_hook(k: usize, ports: &[PortHook], boot_word: u32) -> Option<String> {
    let p = ports.get(k).filter(|p| !p.open)?;
    let next = next_injected(ports, k + 1)?;
    let frame: Vec<String> = p.frame.iter().map(|b| format!("{b:#x}")).collect();
    let py = format!(
        "from Antmicro.Renode.Time import TimeInterval; import sys; e = getattr(sys, 'aethelgard_echo', None); ok = e is not None and self.ReadDoubleWord({boot_word:#x}) == {BOOT_MAGIC:#x}; ok and self.Log(LogLevel.Warning, '{PREFIX_TAG}TDR {k} %x %d' % (value, cpu.ExecutedInstructions)); f = [{}]; m = ok and 0 <= e[{k}] < {FRAME_LEN} and value == f[e[{k}]]; m and e.__setitem__({k}, e[{k}] + 1); m and e[{k}] == {FRAME_LEN} and {}",
        frame.join(", "),
        frame_actions(next, &ports[next])
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
            open: false,
        }
    }

    /// The composed text is the form measured in the pinned Renode (build
    /// probe P5): one monitor command, the Python in double quotes with
    /// single-quoted strings inside, no backslash.
    #[test]
    fn a_dshot_timer_s_hook_logs_every_write_and_schedules_the_stop_on_a_rise() {
        let hooks = timer_hooks(&dshot());
        // One per store width, the same Python in each (adversarial pass
        // 1, M2: a narrow store of `CEN` anchors as a word's does).
        let widths: Vec<&str> = hooks.iter().map(|h| h.split(' ').nth(3).unwrap()).collect();
        assert_eq!(widths, ["DoubleWord", "Word", "Byte"]);
        let python = |h: &str| h.split_once(" Write ").unwrap().1.to_owned();
        assert!(hooks.iter().all(|h| python(h) == python(&hooks[0])));
        let h = &hooks[0];
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
        // The read-back, then CEN cleared and read again, in that order;
        // the observers are never silenced (adversarial pass 1, H1).
        let stop = h.find("AETHELGARD-STOP 1 %d %d %x %x' % (a, cpu.ExecutedInstructions, self.ReadDoubleWord(0x40010000), self.ReadDoubleWord(0x4001002c))").unwrap();
        let clear = h
            .find("self.WriteDoubleWord(0x40010000, self.ReadDoubleWord(0x40010000) & 0xfffffffe)")
            .unwrap();
        let cleared = h.find("'AETHELGARD-CLEARED 1 %d %d %x' % (a, cpu.ExecutedInstructions, self.ReadDoubleWord(0x40010000))").unwrap();
        assert!(stop < clear && clear < cleared);
        assert!(!h.contains("logLevel"), "{h}");
    }

    #[test]
    fn a_pwm_timer_s_hook_only_logs() {
        let hooks = timer_hooks(&TimerHook {
            dshot: false,
            ..dshot()
        });
        assert_eq!(hooks.len(), 3);
        for h in hooks {
            assert!(h.contains("'AETHELGARD-CR1 1 %x %x %d'"));
            assert!(!h.contains("ScheduleAction"), "{h}");
            assert!(!h.contains("WriteDoubleWord"), "{h}");
        }
    }

    #[test]
    fn the_boot_word_schedules_the_first_port_s_frame_byte_by_byte() {
        let ports = [port("usart1", 0x4001_1028), port("usart2", 0x4000_4428)];
        let h = boot_word_hook(0x2000_0000, &ports);
        assert!(h.starts_with("sysbus AddWatchpointHook 0x20000000 DoubleWord Write \""));
        assert_eq!(h.matches('"').count(), 2, "{h}");
        assert!(h.contains("go = value == 0x2100b007 and b != 0x2100b007; go and setattr(sys, 'aethelgard_echo', [-1] * 2); go and ("));
        // Byte j a quantum and j spacings after the boot word, each stale-checked.
        for (j, b) in ports[0].frame.iter().enumerate() {
            let us = 100 + 1100 * j;
            let due = 100 * us;
            assert!(
                h.contains(&format!(
                    "FromMicroseconds({us}), lambda t, a=cpu.ExecutedInstructions: ("
                )),
                "byte {j}"
            );
            let start = if j == 0 {
                ", __import__('sys').aethelgard_echo.__setitem__(0, 0)"
            } else {
                ""
            };
            assert!(h.contains(&format!("monitor.Parse('sysbus.usart1 WriteChar {b:#04x}'){start}) if cpu.ExecutedInstructions == a + {due} else self.Log(LogLevel.Warning, 'AETHELGARD-STALE INJECT 0 {j} %d %d'")), "byte {j}");
        }
        // The RX pin's mux is read with the first byte, before it goes in;
        // then the port's echo count starts.
        assert!(h.contains("'AETHELGARD-INJECT 0 0 %d %d %x %x' % (a, cpu.ExecutedInstructions, self.ReadDoubleWord(0x58020400), self.ReadDoubleWord(0x58020424))), monitor.Parse('sysbus.usart1 WriteChar 0xba'), __import__('sys').aethelgard_echo.__setitem__(0, 0)) if"));
        assert_eq!(h.matches("__setitem__").count(), 1, "{h}");
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
        // Logged only after this boot's boot word, which the reset re-arms;
        // counted only when it is the frame's next byte, once the port's
        // first byte went in (adversarial pass 1, L1).
        assert!(h.contains("ok = e is not None and self.ReadDoubleWord(0x20000000) == 0x2100b007; ok and self.Log(LogLevel.Warning, 'AETHELGARD-TDR 0 %x %d' % (value, cpu.ExecutedInstructions)); f = [0xba, 0x17, 0xfc, 0xb3, 0x40, 0x9d, 0x9, 0x7a]; m = ok and 0 <= e[0] < 8 and value == f[e[0]]; m and e.__setitem__(0, e[0] + 1); m and e[0] == 8 and (self.Machine.ScheduleAction(TimeInterval.FromMicroseconds(100), lambda t, a=cpu.ExecutedInstructions: (self.Log(LogLevel.Warning, 'AETHELGARD-INJECT 1 0 %d %d %x %x'"), "{h}");
        assert!(h.contains("monitor.Parse('sysbus.usart2 WriteChar 0xba')"));
        // The last port schedules nothing.
        assert!(echo_hook(1, &ports, 0x2000_0000).is_none());
    }

    /// `AET-WO-2302` D5: a port held open gets no frame and no `TDR` hook;
    /// the chain skips it, and every line keeps the port's own index.
    #[test]
    fn the_chain_skips_a_port_held_open() {
        let mut ports = [
            port("usart1", 0x4001_1028),
            port("usart2", 0x4000_4428),
            port("uart4", 0x4000_4c28),
        ];
        ports[1].open = true;
        let bw = boot_word_hook(0x2000_0000, &ports);
        assert!(bw.contains("[-1] * 3"), "{bw}");
        assert!(bw.contains("'AETHELGARD-INJECT 0 0 %d %d %x %x'"));
        assert!(!bw.contains("usart2") && !bw.contains("uart4"), "{bw}");
        // Port 0's last echoed byte schedules port 2's frame, not port 1's.
        let e0 = echo_hook(0, &ports, 0x2000_0000).unwrap();
        assert!(e0.contains("m and e[0] == 8 and (self.Machine.ScheduleAction(TimeInterval.FromMicroseconds(100), lambda t, a=cpu.ExecutedInstructions: (self.Log(LogLevel.Warning, 'AETHELGARD-INJECT 2 0 %d %d %x %x'"), "{e0}");
        assert!(e0.contains("monitor.Parse('sysbus.uart4 WriteChar 0xba')"));
        assert!(!e0.contains("usart2"), "{e0}");
        // The open port has no hook, nor does the last port.
        assert!(echo_hook(1, &ports, 0x2000_0000).is_none());
        assert!(echo_hook(2, &ports, 0x2000_0000).is_none());
        // The first port held open: the boot word schedules the next.
        ports[1].open = false;
        ports[0].open = true;
        let bw = boot_word_hook(0x2000_0000, &ports);
        assert!(bw.contains("'AETHELGARD-INJECT 1 0 %d %d %x %x'"), "{bw}");
        assert!(bw.contains("monitor.Parse('sysbus.usart2 WriteChar 0xba')"));
        assert!(!bw.contains("usart1"), "{bw}");
        assert!(echo_hook(0, &ports, 0x2000_0000).is_none());
        assert!(
            echo_hook(1, &ports, 0x2000_0000)
                .unwrap()
                .contains("INJECT 2 0")
        );
        // Every port held open: the boot word only logs.
        for p in &mut ports {
            p.open = true;
        }
        let bw = boot_word_hook(0x2000_0000, &ports);
        assert!(!bw.contains("ScheduleAction") && !bw.contains("aethelgard_echo"));
        assert!((0..3).all(|k| echo_hook(k, &ports, 0x2000_0000).is_none()));
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
            p("[WARNING] sysbus: AETHELGARD-TDR 0 ba 13125"),
            Some(Some(Hook::Tdr {
                port: 0,
                value: 0xba,
                count: 13_125
            }))
        );
        assert_eq!(
            p("[WARNING] sysbus: AETHELGARD-STEP 1 22020001 721093"),
            Some(Some(Hook::Step {
                check: 1,
                value: 0x2202_0001,
                count: 721_093
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
        assert_eq!(
            p("[NOISY] gpioPortA.pa8obs: LED state changed to True"),
            None
        );
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
            "[WARNING] sysbus: AETHELGARD-INJECT 0 0 3075 13075",
            "[WARNING] sysbus: AETHELGARD-STOP 1 3067 203067",
            "[WARNING] sysbus: AETHELGARD-STALE NOW 1 2 3",
            "[WARNING] sysbus: AETHELGARD-TDR 0 ba",
            "[WARNING] sysbus: AETHELGARD-ECHO 0 1 ba 13125",
            // `AET-WO-3305`: a step line is read as strictly as the rest.
            "[WARNING] sysbus: AETHELGARD-STEP 1 22020001",
            "[WARNING] sysbus: AETHELGARD-STEP 1 22020001 721093 0",
            "[WARNING] sysbus: AETHELGARD-STEP 01 22020001 721093",
            "[WARNING] sysbus: AETHELGARD-STEP 1 0x22020001 721093",
            "[WARNING] sysbus: AETHELGARD-STEP 1 22020001 -1",
            "[WARNING] sysbus: AETHELGARD-STEP x 22020001 721093",
            "[WARNING] sysbus: AETHELGARD-STEP 1 122020001 721093",
            "[WARNING] sysbus: AETHELGARD-",
        ] {
            assert_eq!(p(bad), Some(None), "{bad}");
        }
    }
}
