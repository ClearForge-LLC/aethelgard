//! Serial ports in the twin (`AET-WO-4300b` §1.5): what the judge expects a
//! port to carry, from the [`SolvedTopology`] and this module's constants
//! alone (`INV-07`), and the parsers for what Renode leaves.
//!
//! Renode 1.17.0's `UART.STM32F7_USART` passes no byte through a pin: a byte
//! reaches the port's file backend only from a `TDR` write with `TE` and `UE`
//! set (`STM32F7_USART.cs` L372–384), and `WriteChar` queues a byte when `RE`
//! and `UE` are set (`UARTBase.cs` L80–92), whatever the pins' mux. So the
//! capture witnesses what the USART sent, and the pins' `MODER`/`AFR`, read
//! by the composition, decide whether it reached a pin (FEEDBACK P5.1).
//!
//! A boot's capture of one port is its boots' patterns, then `F`:
//! - `P`, the pattern ([`pattern`]): the label's bytes and [`PATTERN_TAIL`],
//!   sent once by each boot that reached it, whole, or cut short (a proper
//!   prefix) by a rendered reset; the last one whole, and whole and cut
//!   together at most the releases plus one ([`parse`]);
//! - `F`, the frame ([`frame`]): bytes the composition injects at
//!   [`injection_ns`], which only a firmware that reads `RDR` sends back.
//!   They are derived from the firmware's own sha256, which no firmware can
//!   hold when it is built.

use aethelgard_ir::SolvedTopology;
use aethelgard_ir::serial::{Framing, Parity};
use aethelgard_svd_hal::cmsis::{self, RegisterError};
use sha2::{Digest, Sha256};

use crate::{Kind, TwinError, boot, kind, platform};

// belt-and-suspenders: the generator holds its own copies of these two
// constants (`aethelgard-svd-hal` `board/firmware.rs`), because the judge's
// expectations are never taken from the generator (`INV-07`); a test below
// proves the copies agree.
/// What each port sends after its label's bytes.
pub const PATTERN_TAIL: [u8; 5] = [0x00, 0xFE, 0x7F, 0x80, 0xFF];

/// The length of the injected frame, and the most bytes the firmware echoes.
pub const FRAME_LEN: usize = 8;

/// The domain the frame is hashed under.
pub const FRAME_DOMAIN: &[u8] = b"aethelgard-serial-frame";

/// Renode's name of a serial port's USART (`usart1`), from the stock table.
pub fn renode_name(peripheral: &str) -> Result<&'static str, TwinError> {
    if kind(peripheral)? != Kind::Serial {
        return Err(TwinError::UnsupportedPeripheral {
            peripheral: peripheral.to_owned(),
        });
    }
    platform::STOCK
        .iter()
        .find(|s| s.0 == peripheral)
        .map(|s| s.1)
        .ok_or_else(|| TwinError::NotInRenode {
            peripheral: peripheral.to_owned(),
            file: platform::STOCK_FILE,
        })
}

/// Renode's `lpuart1`, in the stock platform beside the USARTs and UARTs of
/// [`platform::STOCK`] (measured: a file backend on it opens, fix 1 P2); no
/// port may use it (`aethelgard_ir::serial::LPUART`).
pub const LPUART_RENODE: &str = "lpuart1";

/// Every stock serial peripheral's Renode name: the composition captures each
/// one, so a byte sent on one the topology does not name is seen
/// (`AET-WO-4300b` adversarial F12).
pub fn stock_uarts() -> Vec<&'static str> {
    platform::STOCK
        .iter()
        .filter(|s| kind(s.0).is_ok_and(|k| k == Kind::Serial))
        .map(|s| s.1)
        .chain([LPUART_RENODE])
        .collect()
}

/// The pattern a port sends at start: its label's UTF-8 bytes, then
/// [`PATTERN_TAIL`].
pub fn pattern(label: &str) -> Vec<u8> {
    let mut p = label.as_bytes().to_vec();
    p.extend_from_slice(&PATTERN_TAIL);
    p
}

/// The frame injected into the port labelled `label` of a firmware whose
/// sha256 is `firmware_sha256`: the first [`FRAME_LEN`] bytes of
/// `sha256(FRAME_DOMAIN ‖ firmware_sha256 ‖ label)`.
pub fn frame(firmware_sha256: &[u8; 32], label: &str) -> [u8; FRAME_LEN] {
    let mut h = Sha256::new();
    h.update(FRAME_DOMAIN);
    h.update(firmware_sha256);
    h.update(label.as_bytes());
    let digest = h.finalize();
    let mut out = [0u8; FRAME_LEN];
    out.copy_from_slice(&digest[..FRAME_LEN]);
    out
}

/// The 32 bytes of a lower-case hexadecimal sha256, or `None`.
pub fn sha256_bytes(hex: &str) -> Option<[u8; 32]> {
    let b = hex.as_bytes();
    if b.len() != 64 {
        return None;
    }
    let nibble = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let mut out = [0u8; 32];
    for (i, pair) in b.chunks(2).enumerate() {
        out[i] = nibble(pair[0])? << 4 | nibble(pair[1])?;
    }
    Some(out)
}

/// The instant the frame is injected, for a boot whose last start is
/// `boot_start_ns` (0, or a rendered release plus its nudge): one quantum
/// after it, when the firmware has had a whole quantum to configure its
/// ports (FEEDBACK P3.1).
pub fn injection_ns(boot_start_ns: u64) -> u64 {
    boot_start_ns.saturating_add(boot::QUANTUM_NS)
}

/// Whether a boot of `virtual_ns` holds the injection and one quantum after
/// it, in which the firmware echoes: a boot that cannot is refused by name,
/// never cut short (the ruling's amendment 3).
pub fn fits(injection_ns: u64, virtual_ns: u64) -> bool {
    injection_ns
        .checked_add(boot::QUANTUM_NS)
        .is_some_and(|end| end <= virtual_ns)
}

/// One character's time on the wire in ns, rounded up: its start bit, data
/// bits, parity bit if any and stop bits at the solved actual baud
/// (`actual_millibaud`, thousandths). `None` for a baud of 0 or an overflow.
pub fn char_ns(framing: &Framing, actual_millibaud: u64) -> Option<u64> {
    let bits = 1 + u64::from(framing.word_bits()) + u64::from(framing.stop_bits);
    let num = bits.checked_mul(1_000_000_000_000)?;
    (actual_millibaud > 0).then(|| num.div_ceil(actual_millibaud))
}

/// A port's wire-time budget in ns (the PM's ruling on adversarial F5): its
/// pattern, the frame, and one character more, since the firmware echoes
/// byte by byte, each character [`char_ns`]. The twin renders no wire time
/// (Renode queues the bytes): the CLI sums the ports' budgets in the
/// specification's order, since the firmware serves them one after another
/// (the ruling on pass 2's H1), and judges the sum statically, before the
/// boot, against the board's window.
pub fn wire_ns(label: &str, framing: &Framing, actual_millibaud: u64) -> Option<u64> {
    let chars = u64::try_from(pattern(label).len() + FRAME_LEN + 1).ok()?;
    char_ns(framing, actual_millibaud)?.checked_mul(chars)
}

/// What one port's capture holds, read against its pattern and frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    /// The whole patterns before the frame: one per boot that sent its
    /// pattern.
    pub patterns: usize,
    /// The patterns cut short among them: each a proper prefix of the
    /// pattern, which a boot sends when a rendered reset stops it mid-pattern
    /// (the ruling on C1's capture parser). The fewest that read the bytes.
    pub cut: usize,
    /// How many of the frame's bytes follow the last whole pattern, in
    /// order: [`FRAME_LEN`] for the whole frame, fewer for its first bytes
    /// only.
    pub echoed: usize,
    /// True when the bytes are not boots then the frame's first `echoed`
    /// bytes, exactly: anything else is in the capture.
    pub stray: bool,
    pub len: usize,
}

/// Reads `body` as the boots' patterns: whole patterns and patterns cut
/// short (each a non-empty proper prefix of `pattern`), in any order, the
/// last one whole; or nothing. `Some((whole, cut))` with the fewest
/// patterns in all that read it, `None` when no reading exists.
///
/// The fewest, by dynamic programming over the body: a greedy reading fails
/// where the pattern overlaps itself (`AB` then `ABAC…` read greedily takes
/// `ABA` and is left with `BAC…`). For a pattern [`pattern`] builds the
/// whole patterns are fixed anyway: `0xFF` ends [`PATTERN_TAIL`] and is in
/// no UTF-8 label, so each `0xFF` of the body ends a whole pattern, and a
/// cut one holds none.
fn boots(body: &[u8], pattern: &[u8]) -> Option<(usize, usize)> {
    if body.is_empty() {
        return Some((0, 0));
    }
    let (n, len) = (body.len(), pattern.len());
    if len == 0 {
        return None;
    }
    // `(patterns, cut)` to reach each position, the last pattern whole or cut.
    type Best = Option<(usize, usize)>;
    let better = |slot: &mut Best, cand: (usize, usize)| {
        if slot.is_none_or(|s| cand < s) {
            *slot = Some(cand);
        }
    };
    let mut whole: Vec<Best> = vec![None; n + 1];
    let mut cut: Vec<Best> = vec![None; n + 1];
    for i in 0..n {
        let here = if i == 0 {
            Some((0, 0))
        } else {
            match (whole[i], cut[i]) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            }
        };
        let Some((k, c)) = here else {
            continue;
        };
        let common = body[i..]
            .iter()
            .zip(pattern)
            .take_while(|(a, b)| a == b)
            .count();
        if common == len {
            better(&mut whole[i + len], (k + 1, c));
        }
        for l in 1..=common.min(len - 1) {
            better(&mut cut[i + l], (k + 1, c + 1));
        }
    }
    whole[n].map(|(k, c)| (k - c, c))
}

/// Reads `bytes` as boots then a prefix of `frame`, from the frame's end
/// (the boots read by [`boots`]: whole and cut patterns, the last whole):
/// - the whole frame at the end, the boots before it;
/// - else the boots alone (no echo; this wins where a partial echo would
///   also read, so a pattern's last bytes are never taken for the frame's
///   first);
/// - else the longest proper prefix of the frame at the end, the boots
///   before it;
/// - else the whole patterns at the start, and `stray` set.
///
/// The caller judges the count: at most one pattern, whole or cut, per boot
/// the run rendered, so a cut in a run with no reset is forged.
pub fn parse(bytes: &[u8], pattern: &[u8], frame: &[u8; FRAME_LEN]) -> Capture {
    let done = |(patterns, cut), echoed| Capture {
        patterns,
        cut,
        echoed,
        stray: false,
        len: bytes.len(),
    };
    if let Some(body) = bytes.strip_suffix(&frame[..])
        && let Some(b) = boots(body, pattern)
    {
        return done(b, FRAME_LEN);
    }
    if let Some(b) = boots(bytes, pattern) {
        return done(b, 0);
    }
    for n in (1..FRAME_LEN).rev() {
        if let Some(body) = bytes.strip_suffix(&frame[..n])
            && let Some(b) = boots(body, pattern)
        {
            return done(b, n);
        }
    }
    let patterns = if pattern.is_empty() {
        0
    } else {
        bytes
            .chunks(pattern.len())
            .take_while(|c| *c == pattern)
            .count()
    };
    Capture {
        patterns,
        cut: 0,
        echoed: 0,
        stray: true,
        len: bytes.len(),
    }
}

/// One access Renode's peripheral access log shows (`sysbus
/// LogPeripheralAccess`, enabled by the composition for each port's USART,
/// each of its pins' GPIO ports and the RCC): its source, whether it wrote,
/// its width in bits, the offset in the peripheral and the value written or
/// returned, and whether the model refused the write ([`refused_by`]).
///
/// Measured in Renode 1.17.0 (`AET-WO-4300b` fix 1, P1 and P2): one `INFO`
/// line per access, whatever the offset or width, a defined register or not,
/// and through `machine Reset`:
/// `[INFO] usart1: [cpu: 0x80009DE] WriteUInt32 to 0x28 (TransmitData), value 0x43`,
/// `… WriteUInt32 to 0x2C (TransmitData+0x4), value 0x1`,
/// `… WriteByte to 0x1 (ControlRegister1+0x1), value 0x10`,
/// `… ReadUInt32 from 0x1C (InterruptAndStatus), returned 0x206000C0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Access {
    pub source: String,
    pub write: bool,
    pub bits: u8,
    pub offset: u32,
    pub value: u64,
    /// The model logged that it ignored this write: a rebuild never applies
    /// it (`AET-WO-4300b` adversarial pass 2, L2).
    pub refused: bool,
}

/// `0x` then upper-case hexadecimal, as Renode writes `0x{:X}`.
fn upper_hex(s: &str) -> Option<u64> {
    let h = s.strip_prefix("0x")?;
    (!h.is_empty()
        && h.len() <= 16
        && h.bytes()
            .all(|b| b.is_ascii_digit() || (b'A'..=b'F').contains(&b)))
    .then(|| u64::from_str_radix(h, 16).ok())
    .flatten()
}

/// Parses a whole log line's text after its timestamp as one access, in
/// exactly the measured shape ([`Access`]); `None` for any other line.
pub fn access(body: &str) -> Option<Access> {
    let rest = body.strip_prefix("[INFO] ")?;
    let (source, rest) = rest.split_once(": [cpu: ")?;
    if source.is_empty() || !source.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    let (pc, rest) = rest.split_once("] ")?;
    upper_hex(pc)?;
    let (op, rest) = rest.split_once(' ')?;
    let (write, width) = if let Some(w) = op.strip_prefix("Write") {
        (true, w)
    } else {
        (false, op.strip_prefix("Read")?)
    };
    let bits = match width {
        "Byte" => 8,
        "UInt16" => 16,
        "UInt32" => 32,
        "UInt64" => 64,
        _ => return None,
    };
    let rest = rest.strip_prefix(if write { "to " } else { "from " })?;
    let (offset, rest) = rest.split_once(" (")?;
    let (name, value) = rest.split_once("), ")?;
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'_')
    {
        return None;
    }
    let value = value.strip_prefix(if write { "value " } else { "returned " })?;
    Some(Access {
        source: source.to_owned(),
        write,
        bits,
        offset: u32::try_from(upper_hex(offset)?).ok()?,
        value: upper_hex(value)?,
        refused: false,
    })
}

/// What Renode's GPIO model logs, right after the write's own access line,
/// when it ignores a write to a pin its `LCKR` locked (measured, adversarial
/// pass 2 q4): `[WARNING] gpioPortA: Ignoring attempt to change MODER
/// configuration of the locked pin #9`, and the same for `AFSEL`.
const LOCKED_PIN: &str = "Ignoring attempt to change ";

/// The source of a whole log line's text after its timestamp in which the
/// model says it ignored a write to a locked pin ([`LOCKED_PIN`]); `None` for
/// any other line.
pub fn refused_by(body: &str) -> Option<&str> {
    let (source, rest) = body.strip_prefix("[WARNING] ")?.split_once(": ")?;
    (!source.is_empty()
        && source.bytes().all(|b| b.is_ascii_alphanumeric())
        && rest.starts_with(LOCKED_PIN))
    .then_some(source)
}

/// True for a line of Renode's access log ([`access`]) the judge never reads:
/// a read (the RCC's included), or a write to a GPIO port's word other than
/// `MODER`, `OTYPER`, `OSPEEDR`, `PUPDR`, `AFRL` and `AFRH` (a sub-word write
/// into one of those is kept, and fails closed). The boot's log filter
/// discards these before its budget, uncounted: a firmware polling `ISR`
/// writes one line per poll (measured: 20 000 polls, 2.3 MB, per port that
/// waits out `ECHO_POLLS`; fix 1 P2), and one toggling a pin of a logged port
/// (`BSRR`, `ODR`) one per toggle.
pub fn is_unjudged_access(body: &str) -> bool {
    access(body).is_some_and(|a| {
        !a.write
            || (a.source.starts_with("gpioPort")
                && gpio_judged_offsets().is_some_and(|o| !o.contains(&(a.offset & !3))))
    })
}

/// The offsets in a GPIO port of the words the judge rebuilds (`MODER`,
/// `OTYPER`, `OSPEEDR`, `PUPDR`, `AFRL`, `AFRH`), from the header, read once;
/// `None` if the header cannot be read (then nothing is discarded).
fn gpio_judged_offsets() -> Option<[u32; 6]> {
    static OFFSETS: std::sync::OnceLock<Option<[u32; 6]>> = std::sync::OnceLock::new();
    *OFFSETS.get_or_init(|| {
        let f = cmsis::register_facts().ok()?;
        let g = f.layout("GPIO").ok()?;
        Some([
            g.offset("MODER", 0).ok()?,
            g.offset("OTYPER", 0).ok()?,
            g.offset("OSPEEDR", 0).ok()?,
            g.offset("PUPDR", 0).ok()?,
            g.offset("AFR", 0).ok()?,
            g.offset("AFR", 1).ok()?,
        ])
    })
}

/// True when `body` is a line of `source`'s own `INFO` log: from a logged
/// source, every such line must be a whole access, or the judge cannot
/// rebuild what was written ([`judge_writes`] then fails closed).
pub fn logged_by(body: &str, source: &str) -> bool {
    body.strip_prefix("[INFO] ")
        .and_then(|r| r.strip_prefix(source))
        .is_some_and(|r| r.starts_with(": "))
}

/// A tag the framing may set: the register's name and offset, the field's
/// name in ST's header, and its bit there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub register: &'static str,
    pub offset: u32,
    pub field: &'static str,
    pub mask: u32,
}

/// Every tag a framing may need, in register order (the order kind 28 lists
/// them in): `M0` for a 9-bit word and `M1` for a 7-bit one (`M[1:0]`,
/// RM0433 Rev 9 §48.8.1), `RXINV` and `TXINV` when inverted (§48.8.2).
/// Renode stores none of them (`STM32F7_USART.cs` L162, L167, L177–178):
/// each is seen in the values the access log shows written. Offsets and bits
/// from the header.
fn all_tags() -> Result<Vec<Tag>, TwinError> {
    let f = cmsis::register_facts()?;
    let layout = f.layout("USART")?;
    let (cr1, cr2) = (layout.offset("CR1", 0)?, layout.offset("CR2", 0)?);
    let tag = |register, offset, field: &'static str| -> Result<Tag, TwinError> {
        Ok(Tag {
            register,
            offset,
            field,
            mask: 1 << f.position(&format!("USART_{register}_{field}_Pos"))?,
        })
    };
    Ok(vec![
        tag("CR1", cr1, "M0")?,
        tag("CR1", cr1, "M1")?,
        tag("CR2", cr2, "RXINV")?,
        tag("CR2", cr2, "TXINV")?,
    ])
}

/// The tags `framing` needs set, of [`all_tags`], in its order.
pub fn needed_tags(framing: &Framing) -> Result<Vec<Tag>, TwinError> {
    let m = framing.m_bits().ok_or_else(|| {
        TwinError::Register(RegisterError::Undefined {
            name: format!("USART_CR1.M for a {}-bit word", framing.word_bits()),
        })
    })?;
    let need = [
        m & 0b01 != 0,
        m & 0b10 != 0,
        framing.inverted,
        framing.inverted,
    ];
    Ok(all_tags()?
        .into_iter()
        .zip(need)
        .filter_map(|(t, n)| n.then_some(t))
        .collect())
}

/// The bits of `CR1`, `CR2` and `CR3` Renode 1.17.0's `UART.STM32F7_USART`
/// stores (`STM32F7_USART.cs` L144–160, L175, L186–187, L317–318), from ST's
/// header: `CR1`'s `UE`, `RE`, `TE`, `RXNEIE`, `TCIE`, `TXEIE`, `PEIE`, `PS`,
/// `PCE`, `OVER8`, `RTOIE`; `CR2`'s `STOP`; `CR3`'s `DMAR`, `DMAT`. Every
/// other bit is a tag or reserved: it reads back 0, so a read-modify-write
/// in the twin clears what silicon keeps ([`judge_writes`]).
pub(crate) fn usart_stored(f: &cmsis::RegisterFacts) -> Result<[u32; 3], TwinError> {
    let bit = |name: &str| -> Result<u32, TwinError> { Ok(1 << f.position(name)?) };
    let cr1 = [
        "USART_CR1_UE_Pos",
        "USART_CR1_RE_Pos",
        "USART_CR1_TE_Pos",
        "USART_CR1_RXNEIE_RXFNEIE_Pos",
        "USART_CR1_TCIE_Pos",
        "USART_CR1_TXEIE_TXFNFIE_Pos",
        "USART_CR1_PEIE_Pos",
        "USART_CR1_PS_Pos",
        "USART_CR1_PCE_Pos",
        "USART_CR1_OVER8_Pos",
        "USART_CR1_RTOIE_Pos",
    ]
    .iter()
    .try_fold(0u32, |m, n| Ok::<_, TwinError>(m | bit(n)?))?;
    let (stop_pos, stop_width) = f.field("USART_CR2_STOP")?;
    Ok([
        cr1,
        ((1u32 << stop_width) - 1) << stop_pos,
        bit("USART_CR3_DMAR_Pos")? | bit("USART_CR3_DMAT_Pos")?,
    ])
}

/// What a port's bring-up writes, from the solved topology and ST's header
/// alone (`INV-07`): the word each of `CR1`, `CR2`, `CR3` and `BRR` holds at
/// the write that first sets `CR1.UE` (`CR1` with `UE`; `CR3` its reset 0,
/// never written), and `TDR`'s offset. The firmware writes `BRR`, `CR2`,
/// `CR1`, `CR1 | UE`, then `TDR` only (the contract's order). `CR1` holds
/// `FIFOEN` (RM0433 Rev 9 §48.8.1, bit 29, written only with `UE` clear;
/// the PM's ruling on adversarial pass 2, H1): Renode stores none of bits
/// 29–31 (measured, fix 2 f1: `Unhandled bits: [29] … Tags: RESERVED`, and
/// `CR1` reads back without it), so it is seen only in the access log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bringup {
    /// `(name, offset, word at UE)` for `CR1`, `CR2`, `CR3`, `BRR`.
    pub words: [(&'static str, u32, u32); 4],
    pub tdr: u32,
    pub ue: u32,
    pub tags: Vec<Tag>,
    pub needed: Vec<Tag>,
    /// For each of [`Self::words`], the bits Renode stores nowhere
    /// ([`usart_stored`]; none of `BRR`'s are judged here).
    pub unstored: [u32; 4],
}

impl Bringup {
    pub fn of(framing: &Framing, brr: u32) -> Result<Self, TwinError> {
        let f = cmsis::register_facts()?;
        let layout = f.layout("USART")?;
        let bit = |name: &str| -> Result<u32, TwinError> { Ok(1 << f.position(name)?) };
        let needed = needed_tags(framing)?;
        let set = |reg: &str| {
            needed
                .iter()
                .filter(|t| t.register == reg)
                .fold(0, |m, t| m | t.mask)
        };
        let (pce, ps) = parity_bits(framing);
        let ue = bit("USART_CR1_UE_Pos")?;
        let mut cr1 = ue
            | bit("USART_CR1_RE_Pos")?
            | bit("USART_CR1_TE_Pos")?
            | bit("USART_CR1_FIFOEN_Pos")?
            | set("CR1");
        if pce {
            cr1 |= bit("USART_CR1_PCE_Pos")?;
        }
        if ps {
            cr1 |= bit("USART_CR1_PS_Pos")?;
        }
        let (stop_pos, _) = f.field("USART_CR2_STOP")?;
        let stop = framing.stop_field().ok_or_else(|| {
            TwinError::Register(RegisterError::Undefined {
                name: format!("USART_CR2.STOP for {} stop bits", framing.stop_bits),
            })
        })?;
        let cr2 = (u32::from(stop) << stop_pos) | set("CR2");
        Ok(Self {
            words: [
                ("CR1", layout.offset("CR1", 0)?, cr1),
                ("CR2", layout.offset("CR2", 0)?, cr2),
                ("CR3", layout.offset("CR3", 0)?, 0),
                ("BRR", layout.offset("BRR", 0)?, brr),
            ],
            tdr: layout.offset("TDR", 0)?,
            ue,
            tags: all_tags()?,
            needed,
            unstored: {
                let [a, b, c] = usart_stored(&f)?;
                [!a, !b, !c, 0]
            },
        })
    }
}

/// What a port's writes, rebuilt from the access log, show against its
/// bring-up ([`judge_writes`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagJudgement {
    /// The USART's writes in the last boot.
    pub writes: usize,
    /// Each needed tag not set in its register's word at `UE` in some boot,
    /// as `CR1.M0`, in [`all_tags`]' order.
    pub missing: Vec<String>,
    /// Everything else the writes did that the bring-up does not, each once,
    /// in the order first seen: a tag set at `UE` the framing does not need
    /// (`CR1.M1 set at UE`), another bit of a word at `UE` (`CR1 0x40D at UE,
    /// not 0x140D`), a word changed after `UE` (`CR2 0x32000 after UE`), a
    /// write the bring-up never makes (`0x1 to 0x2C`, `WriteByte to 0x1`,
    /// `TDR before UE`), and a last boot that never set `UE`; then what
    /// [`judge_rcc`] and [`judge_pins`] add.
    pub unexpected: Vec<String>,
}

impl TagJudgement {
    pub fn holds(&self) -> bool {
        self.missing.is_empty() && self.unexpected.is_empty()
    }

    /// Adds `x` to [`Self::unexpected`] unless it is there already.
    pub fn note(&mut self, x: String) {
        if !self.unexpected.contains(&x) {
            self.unexpected.push(x);
        }
    }
}

/// Judges the writes `source` (the port's USART, `usart1`) made in each boot,
/// in log order (`boots`, the first boot then one per rendered release;
/// every boot starts from reset, `0` in each word), against `b`: in each
/// boot, every write a word write to `CR1`, `CR2`, `BRR` or `TDR`; no write
/// sets a bit Renode does not store ([`Bringup::unstored`]) that `b`'s word
/// leaves clear; at the write that first sets `UE`, the four words exactly
/// `b`'s, the tags judged apart; after it, no word changed and before it no
/// `TDR` write; and the last boot sets `UE`. A boot cut by a reset before
/// `UE` is judged as far as it went.
pub fn judge_writes(b: &Bringup, source: &str, boots: &[Vec<Access>]) -> TagJudgement {
    let mut j = TagJudgement::default();
    let mut missing = vec![false; b.tags.len()];
    let note = |j: &mut TagJudgement, x: String| j.note(x);
    for (k, boot) in boots.iter().enumerate() {
        let mut word = [0u32; 4];
        let mut enabled = false;
        let mine = boot.iter().filter(|a| a.write && a.source == source);
        if k + 1 == boots.len() {
            j.writes = mine.clone().count();
        }
        for a in mine {
            let slot = b.words.iter().position(|w| w.1 == a.offset);
            let value = u32::try_from(a.value).unwrap_or(u32::MAX);
            if a.bits != 32 {
                note(
                    &mut j,
                    format!("Write{} to {:#X}", width_name(a.bits), a.offset),
                );
                continue;
            }
            if a.offset == b.tdr {
                if !enabled {
                    note(&mut j, "TDR before UE".to_owned());
                }
                continue;
            }
            let Some(i) = slot.filter(|&i| b.words[i].0 != "CR3") else {
                note(&mut j, format!("{value:#X} to {:#X}", a.offset));
                continue;
            };
            // A bit Renode stores nowhere reads back 0, so a later
            // read-modify-write clears it in the log and keeps it on silicon:
            // set once where the bring-up leaves it clear, it fails (the PM's
            // ruling on fix 2's A2 hole).
            let unstored = value & b.unstored[i] & !b.words[i].2;
            if unstored != 0 {
                note(
                    &mut j,
                    format!(
                        "{} {unstored:#X} set, a bit Renode does not store",
                        b.words[i].0
                    ),
                );
            }
            if enabled {
                if word[i] != value {
                    note(&mut j, format!("{} {value:#X} after UE", b.words[i].0));
                }
                word[i] = value;
                continue;
            }
            word[i] = value;
            if b.words[i].0 != "CR1" || value & b.ue == 0 {
                continue;
            }
            enabled = true;
            for (tag, gone) in b.tags.iter().zip(missing.iter_mut()) {
                let at = b
                    .words
                    .iter()
                    .position(|w| w.0 == tag.register)
                    .unwrap_or(0);
                let set = word[at] & tag.mask != 0;
                let needed = b.needed.contains(tag);
                if needed && !set {
                    *gone = true;
                } else if set && !needed {
                    note(&mut j, format!("{}.{} set at UE", tag.register, tag.field));
                }
            }
            for (n, (name, _, want)) in b.words.iter().enumerate() {
                let tag_bits = b
                    .tags
                    .iter()
                    .filter(|t| t.register == *name)
                    .fold(0, |m, t| m | t.mask);
                if (word[n] ^ want) & !tag_bits != 0 {
                    note(
                        &mut j,
                        format!(
                            "{name} {:#X} at UE, not {:#X}",
                            word[n] & !tag_bits,
                            want & !tag_bits
                        ),
                    );
                }
            }
        }
        if k + 1 == boots.len() && !enabled {
            note(&mut j, "the last boot never set CR1.UE".to_owned());
        }
    }
    j.missing = b
        .tags
        .iter()
        .zip(&missing)
        .filter(|(_, m)| **m)
        .map(|(t, _)| format!("{}.{}", t.register, t.field))
        .collect();
    j
}

/// The TX pin's `MODER` and `AFR` words at the `TDR` writes of each boot
/// that wrote one, rebuilt in log order from the words the access log shows
/// the firmware wrote to the pin's GPIO port in that boot (a register never
/// written reads `0` here: never routed): the first `TDR` write's at which
/// the pin was not routed, else the first's; `None` for a boot in which a
/// non-word write reached either register before a `TDR` write. The pattern
/// counts only if every boot's is routed: the pin routed at every `TDR`
/// write (`AET-WO-4300b` adversarial F4 and the PM's ruling on its
/// residual: route, send one, unroute, send the rest).
pub fn tx_at_send(
    tx: &PinMux,
    usart: &str,
    tdr: u32,
    boots: &[Vec<Access>],
) -> Vec<Option<(u32, u32)>> {
    let mut out = Vec::new();
    for boot in boots {
        let mut words = PinWords::reset(tx);
        let mut first: Option<Option<(u32, u32)>> = None;
        let mut bad: Option<Option<(u32, u32)>> = None;
        for a in boot.iter().filter(|a| a.write) {
            if a.source == usart && a.offset == tdr {
                let now = words.moder.zip(words.afr);
                first.get_or_insert(now);
                if bad.is_none() && !now.is_some_and(|(m, f)| tx.routed(m, f)) {
                    bad = Some(now);
                }
                continue;
            }
            words.apply(tx, a);
        }
        if let Some(m) = bad.or(first) {
            out.push(m);
        }
    }
    out
}

/// A pin's words in its GPIO port, rebuilt in log order from the words the
/// access log shows written: `MODER` and its `AFR` word from `0` (never
/// routed), `OTYPER`, `OSPEEDR` and `PUPDR` from their reset values
/// ([`PinMux::config_reset`]); a word becomes `None` once a write the rebuild
/// cannot apply reaches it (not a word write at its offset), and a write the
/// model refused ([`Access::refused`]) is not applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PinWords {
    moder: Option<u32>,
    afr: Option<u32>,
    /// `OTYPER`, `OSPEEDR`, `PUPDR`.
    config: [Option<u32>; 3],
}

impl PinWords {
    fn reset(pin: &PinMux) -> Self {
        Self {
            moder: Some(0),
            afr: Some(0),
            config: pin.config_reset.map(Some),
        }
    }

    fn apply(&mut self, pin: &PinMux, a: &Access) {
        if !a.write || a.refused || a.source != pin.source {
            return;
        }
        let [ot, sp, pu] = &mut self.config;
        for (at, word) in [
            (pin.offsets.0, &mut self.moder),
            (pin.offsets.1, &mut self.afr),
            (pin.config_offsets[0], ot),
            (pin.config_offsets[1], sp),
            (pin.config_offsets[2], pu),
        ] {
            if a.offset & !3 == at {
                *word = match (*word, a.bits == 32 && a.offset == at) {
                    (Some(_), true) => u32::try_from(a.value).ok(),
                    _ => None,
                };
            }
        }
    }

    /// `None` when the pin is push-pull, low speed and pulled neither way;
    /// else what it is, and `when`, for a finding.
    fn misconfigured(&self, pin: &PinMux, when: &str) -> Option<String> {
        match self.config {
            [Some(ot), Some(sp), Some(pu)] => {
                let (ot, sp, pu) = pin.config_fields([ot, sp, pu]);
                ((ot, sp, pu) != (0, 0, 0)).then(|| {
                    format!(
                        "{} OTYPER {ot}, OSPEEDR {sp:#04b}, PUPDR {pu:#04b} {when}",
                        pin.pin
                    )
                })
            }
            _ => Some(format!(
                "{} OTYPER/OSPEEDR/PUPDR not rebuilt {when}",
                pin.pin
            )),
        }
    }
}

/// What the access log shows of a port's two pins beyond their mux
/// (`AET-WO-4300b` adversarial pass 2, A2 and L2), each failure once, in the
/// order first seen:
/// - each pin push-pull, low speed (`00`, the contract's) and pulled neither
///   way, as the bring-up leaves it (an open-drain TX with a pull-down never
///   drives the line high): the TX pin at every `TDR` write of each boot, the
///   RX pin at the injection (`inject_at`, how many of the last boot's
///   accesses precede it; `None` when no frame went in), both at the end of
///   the last boot (`PA9 OTYPER 1, OSPEEDR 0b00, PUPDR 0b10 at a TDR write`);
/// - no write to either pin's GPIO port refused by the model (a pin its
///   `LCKR` locked; the rebuild does not apply it): `gpioPortA refused a
///   write to 0x0 (a locked pin)`.
pub fn judge_pins(
    tx: &PinMux,
    rx: &PinMux,
    usart: &str,
    tdr: u32,
    inject_at: Option<usize>,
    boots: &[Vec<Access>],
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut note = |x: Option<String>| {
        if let Some(x) = x.filter(|x| !out.contains(x)) {
            out.push(x);
        }
    };
    for (k, boot) in boots.iter().enumerate() {
        let last = k + 1 == boots.len();
        let (mut t, mut r) = (PinWords::reset(tx), PinWords::reset(rx));
        for (i, a) in boot.iter().enumerate() {
            if last && inject_at == Some(i) {
                note(r.misconfigured(rx, "at the injection"));
            }
            if a.write && a.source == usart && a.offset == tdr {
                note(t.misconfigured(tx, "at a TDR write"));
            }
            // `OT` is a tag in Renode's GPIO model: it reads back 0, so a
            // read-modify-write clears it in the log and keeps it on silicon.
            // A serial pin's solved `OT` is 0: a write that sets it fails.
            for pin in [tx, rx] {
                if a.write
                    && a.source == pin.source
                    && a.offset == pin.config_offsets[0]
                    && a.value & (1 << pin.index) != 0
                {
                    note(Some(format!(
                        "{} OT set in OTYPER, a bit Renode does not store",
                        pin.pin
                    )));
                }
            }
            if a.refused && (a.source == tx.source || a.source == rx.source) {
                note(Some(format!(
                    "{} refused a write to {:#X} (a locked pin)",
                    a.source, a.offset
                )));
            }
            t.apply(tx, a);
            r.apply(rx, a);
        }
        if last {
            if inject_at == Some(boot.len()) {
                note(r.misconfigured(rx, "at the injection"));
            }
            note(t.misconfigured(tx, "at the end"));
            note(r.misconfigured(rx, "at the end"));
        }
    }
    out
}

/// Renode's name of the RCC in the stock platform (`rcc:
/// Miscellaneous.STM32H7_RCC @ sysbus 0x58024400`, [`platform::STOCK_FILE`]):
/// the composition logs its accesses beside the ports' (adversarial pass 2,
/// A1).
pub const RCC_RENODE: &str = "rcc";

/// A clock gate the bring-up sets: its peripheral, its `RCC_*ENR` register's
/// name and offset in the RCC, and the bit, from the header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    pub peripheral: String,
    pub register: String,
    pub offset: u32,
    pub mask: u32,
}

/// What a port allows the RCC's writes to be (`AET-WO-4300b` adversarial
/// pass 3, ADV3-1, the PM's ruling): the bring-up writes only the run-mode
/// clock enable registers (`AHB3ENR` at 0xD4 … `APB4ENR` at 0xF4, every register of the
/// header's `RCC_TypeDef` named `*ENR` but not `*LPENR`), and only sets bits
/// in them (measured, fix 3 m1: the reference, the four-port fixture and
/// both motor fixtures write `AHB4ENR`, `APB1LENR` and `APB2ENR`, nothing
/// else). Every RCC register is named for a finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RccRule {
    /// Every register of the RCC's layout: its name and offset.
    pub registers: Vec<(String, u32)>,
    /// The offsets of the enable registers.
    pub enables: Vec<u32>,
    /// The gates of the port's USART and of each of its pins' GPIO ports,
    /// once each.
    pub gates: Vec<Gate>,
}

/// The RCC rule of a port on `peripheral` (its USART) with `pins`.
pub fn rcc_rule(peripheral: &str, pins: &[&str]) -> Result<RccRule, TwinError> {
    let f = cmsis::register_facts()?;
    let rcc = f.layout("RCC")?;
    let registers: Vec<(String, u32)> = rcc
        .fields
        .iter()
        .flat_map(|r| (0..r.words).map(move |k| (r.name.clone(), r.offset + 4 * k)))
        .collect();
    let enables = rcc
        .fields
        .iter()
        .filter(|r| r.name.ends_with("ENR") && !r.name.ends_with("LPENR"))
        .map(|r| r.offset)
        .collect();
    let mut names = vec![peripheral.to_owned()];
    for pin in pins {
        let port = format!("GPIO{}", crate::split_pin(pin)?.0);
        if !names.contains(&port) {
            names.push(port);
        }
    }
    let gates = names
        .into_iter()
        .map(|peripheral| {
            let (register, bit) = f.enable_bit(&peripheral)?;
            Ok(Gate {
                offset: rcc.offset(&register, 0)?,
                mask: 1 << bit,
                register,
                peripheral,
            })
        })
        .collect::<Result<_, TwinError>>()?;
    Ok(RccRule {
        registers,
        enables,
        gates,
    })
}

/// The RCC's writes in `boots` the bring-up never makes, each once, in the
/// order first seen (`AET-WO-4300b` adversarial pass 3, ADV3-1 and 1b, the
/// PM's ruling; it subsumes pass 2's reset bits, A1):
/// - a write that is not a word write at a word's offset, which the judge
///   cannot rebuild (`WriteByte to RCC 0xF0`);
/// - a word write to any register but an enable register: a prescaler, a
///   kernel clock selection, an oscillator or a reset (`RCC.D2CFGR 0x400
///   written, not an enable register`), which Renode may store or not, and
///   which changes or stops a port's clock on silicon;
/// - in a boot, an enable write that clears the gate of the port's USART or
///   of its pins' GPIO ports once a write set it (`RCC.APB2ENR.USART1EN
///   cleared after it was set`).
pub fn judge_rcc(rule: &RccRule, boots: &[Vec<Access>]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for boot in boots {
        let mut set = vec![false; rule.gates.len()];
        for a in boot.iter().filter(|a| a.write && a.source == RCC_RENODE) {
            let value = u32::try_from(a.value).unwrap_or(u32::MAX);
            let x = if a.bits != 32 || a.offset % 4 != 0 {
                Some(format!(
                    "Write{} to RCC {:#X}",
                    width_name(a.bits),
                    a.offset
                ))
            } else if !rule.enables.contains(&a.offset) {
                let name = rule
                    .registers
                    .iter()
                    .find(|r| r.1 == a.offset && !r.0.starts_with("RESERVED"))
                    .map_or_else(|| format!("{:#X}", a.offset), |r| r.0.clone());
                Some(format!(
                    "RCC.{name} {value:#X} written, not an enable register"
                ))
            } else {
                None
            };
            if let Some(x) = x {
                if !out.contains(&x) {
                    out.push(x);
                }
                continue;
            }
            for (g, on) in rule.gates.iter().zip(set.iter_mut()) {
                if g.offset != a.offset {
                    continue;
                }
                if value & g.mask != 0 {
                    *on = true;
                } else if *on {
                    let x = format!(
                        "RCC.{}.{}EN cleared after it was set",
                        g.register, g.peripheral
                    );
                    if !out.contains(&x) {
                        out.push(x);
                    }
                }
            }
        }
    }
    out
}

/// How Renode names an access of `bits` bits: `Byte`, `UInt16`, `UInt32`,
/// `UInt64`.
fn width_name(bits: u8) -> &'static str {
    match bits {
        8 => "Byte",
        16 => "UInt16",
        32 => "UInt32",
        _ => "UInt64",
    }
}

/// One serial port as the judge needs it, in the specification's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    pub label: String,
    pub peripheral: String,
    /// Renode's name, `usart1`: the log source and the monitor's target.
    pub renode: &'static str,
    pub tx: String,
    pub rx: String,
    pub tx_af: u8,
    pub rx_af: u8,
    pub framing: Framing,
    pub tags: Vec<Tag>,
}

/// Every serial port of `topology`, in the specification's order.
pub fn ports(topology: &SolvedTopology) -> Result<Vec<Port>, TwinError> {
    topology
        .serial_ports_in_order()
        .into_iter()
        .map(|(label, p, tx, rx)| {
            Ok(Port {
                label: label.to_owned(),
                peripheral: p.peripheral.clone(),
                renode: renode_name(&p.peripheral)?,
                tx: tx.pin.clone(),
                rx: rx.pin.clone(),
                tx_af: tx.alternate_function,
                rx_af: rx.alternate_function,
                framing: p.framing.clone(),
                tags: needed_tags(&p.framing)?,
            })
        })
        .collect()
}

/// A pin's mode and alternate function registers, and where its fields sit:
/// what the composition reads for a port's RX pin at the injection instant,
/// and what the judge rebuilds for its TX pin from the access log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinMux {
    pub pin: String,
    /// Renode's name of the pin's GPIO port, `gpioPortA`: the access log's
    /// source.
    pub source: &'static str,
    /// `MODER`'s and the `AFR` word's offsets in the port.
    pub offsets: (u32, u32),
    /// `GPIOA.MODER` and its address.
    pub moder: (String, u32),
    /// `GPIOA.AFRH` and its address.
    pub afr: (String, u32),
    /// The pin's index in its port.
    pub index: u32,
    pub alternate_function: u8,
    /// `OTYPER`'s, `OSPEEDR`'s and `PUPDR`'s offsets in the port, and their
    /// words at reset.
    pub config_offsets: [u32; 3],
    pub config_reset: [u32; 3],
}

/// `MODER` = `10`, alternate function (RM0433 Rev 9 §11.4.1).
const MODE_AF: u32 = 0b10;

/// A port's `OTYPER`, `OSPEEDR` and `PUPDR` at reset (RM0433 Rev 9
/// §11.4.2–11.4.4, pp. 541–542): `OTYPER` 0 in every port; `OSPEEDR`
/// `0x0C00_0000` in port A and `0x0000_00C0` in port B, `PUPDR` `0x6400_0000`
/// and `0x0000_0100`, 0 in every other port (the debug pins). A word the
/// firmware never writes holds its reset value on silicon.
fn config_reset(port: char) -> [u32; 3] {
    match port {
        'A' => [0, 0x0C00_0000, 0x6400_0000],
        'B' => [0, 0x0000_00C0, 0x0000_0100],
        _ => [0, 0, 0],
    }
}

impl PinMux {
    /// The pin's mux from the header's layout.
    pub fn of(pin: &str, alternate_function: u8) -> Result<Self, TwinError> {
        let (port, index) = crate::split_pin(pin)?;
        let f = cmsis::register_facts()?;
        let gpio = f.layout("GPIO")?;
        let base = f.base(&format!("GPIO{port}"))?;
        let word = index / 8;
        let source = platform::STOCK
            .iter()
            .find(|s| s.0 == format!("GPIO{port}"))
            .map(|s| s.1)
            .ok_or_else(|| TwinError::InvalidPin {
                pin: pin.to_owned(),
            })?;
        Ok(Self {
            pin: pin.to_owned(),
            source,
            offsets: (gpio.offset("MODER", 0)?, gpio.offset("AFR", word)?),
            moder: (format!("GPIO{port}.MODER"), base + gpio.offset("MODER", 0)?),
            afr: (
                format!("GPIO{port}.{}", ["AFRL", "AFRH"][word as usize]),
                base + gpio.offset("AFR", word)?,
            ),
            index,
            alternate_function,
            config_offsets: [
                gpio.offset("OTYPER", 0)?,
                gpio.offset("OSPEEDR", 0)?,
                gpio.offset("PUPDR", 0)?,
            ],
            config_reset: config_reset(port),
        })
    }

    /// The pin's `OT`, `OSPEEDR` and `PUPDR` fields in `words` (`OTYPER`,
    /// `OSPEEDR`, `PUPDR`).
    pub fn config_fields(&self, words: [u32; 3]) -> (u8, u8, u8) {
        let i = self.index;
        (
            ((words[0] >> i) & 1) as u8,
            ((words[1] >> (2 * i)) & 0b11) as u8,
            ((words[2] >> (2 * i)) & 0b11) as u8,
        )
    }

    /// The pin's `MODER` and `AFSEL` fields in the registers `moder` and
    /// `afr`.
    pub fn fields(&self, moder: u32, afr: u32) -> (u8, u8) {
        let mode = (moder >> (2 * self.index)) & 0b11;
        let af = (afr >> (4 * (self.index % 8))) & 0xF;
        (mode as u8, af as u8)
    }

    /// True when `moder` and `afr` put the pin in AF mode on its solved AF.
    pub fn routed(&self, moder: u32, afr: u32) -> bool {
        let (mode, af) = self.fields(moder, afr);
        u32::from(mode) == MODE_AF && af == self.alternate_function
    }

    /// `PA10 MODER 0b10, AF7`: what the reads showed, for a finding.
    pub fn describe(&self, moder: u32, afr: u32) -> String {
        let (mode, af) = self.fields(moder, afr);
        format!("{} MODER {mode:#04b}, AF{af}", self.pin)
    }
}

/// `8N1`, with `inverted` when it is: how the verdict names a framing.
pub fn describe(framing: &Framing) -> String {
    format!(
        "{} {}{}",
        framing.baud,
        framing.short(),
        if framing.inverted { " inverted" } else { "" }
    )
}

/// Whether the framing's parity is any (`PCE`) and odd (`PS`).
pub(crate) fn parity_bits(framing: &Framing) -> (bool, bool) {
    (
        framing.parity != Parity::None,
        framing.parity == Parity::Odd,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_twin_s_copies_are_the_generator_s() {
        assert_eq!(PATTERN_TAIL, aethelgard_svd_hal::board::PATTERN_TAIL);
        assert_eq!(
            u32::try_from(FRAME_LEN).unwrap(),
            aethelgard_svd_hal::board::FRAME_LEN
        );
    }

    #[test]
    fn every_serial_peripheral_has_its_stock_name_and_nothing_else_does() {
        for p in aethelgard_ir::serial::PERIPHERALS {
            assert_eq!(renode_name(p).unwrap(), p.to_ascii_lowercase(), "{p}");
        }
        for p in ["LPUART1", "SPI1", "I2C1", "USART9"] {
            assert!(renode_name(p).is_err(), "{p}");
        }
    }

    #[test]
    fn the_frame_is_the_domain_s_hash_and_moves_with_firmware_and_label() {
        let fw = [0x11u8; 32];
        let f = frame(&fw, "CONSOLE");
        let mut h = Sha256::new();
        h.update(b"aethelgard-serial-frame");
        h.update([0x11u8; 32]);
        h.update(b"CONSOLE");
        assert_eq!(f[..], h.finalize()[..8]);
        assert_ne!(frame(&[0x12u8; 32], "CONSOLE"), f);
        assert_ne!(frame(&fw, "GPS"), f);
        let hex = "00ff".repeat(16);
        assert_eq!(sha256_bytes(&hex).unwrap()[..2], [0x00, 0xFF]);
        assert_eq!(sha256_bytes(&hex.to_uppercase()), None);
        assert_eq!(sha256_bytes("00"), None);
    }

    #[test]
    fn the_injection_is_a_quantum_after_the_last_start_and_must_fit() {
        assert_eq!(injection_ns(0), 100_000);
        assert_eq!(injection_ns(5_000_010), 5_100_010);
        assert!(fits(100_000, 200_000));
        assert!(!fits(100_000, 199_999));
        assert!(!fits(u64::MAX, u64::MAX));
    }

    #[test]
    fn a_capture_reads_as_patterns_then_the_frame_or_is_stray() {
        let p = pattern("GPS");
        assert_eq!(p, [b'G', b'P', b'S', 0x00, 0xFE, 0x7F, 0x80, 0xFF]);
        let f = [1, 2, 3, 4, 5, 6, 7, 8];
        let cat = |parts: &[&[u8]]| parts.concat();
        let c = |b: &[u8]| parse(b, &p, &f);
        assert_eq!(
            c(&cat(&[&p, &f])),
            Capture {
                patterns: 1,
                cut: 0,
                echoed: 8,
                stray: false,
                len: 16
            }
        );
        assert_eq!(c(&cat(&[&p, &p, &f])).patterns, 2);
        let none = c(&p);
        assert_eq!((none.patterns, none.echoed, none.stray), (1, 0, false));
        let partial = c(&cat(&[&p, &f[..3]]));
        assert_eq!(
            (partial.patterns, partial.echoed, partial.stray),
            (1, 3, false)
        );
        // The frame without a pattern, a byte after the frame, and the
        // pattern of another port are all stray or patternless.
        assert_eq!(c(&f).patterns, 0);
        assert!(c(&cat(&[&p, &f, &[0]])).stray);
        let other = c(&cat(&[&pattern("RC"), &f]));
        assert!(other.stray && other.patterns == 0);
        assert_eq!(c(&[]), parse(&[], &p, &f));
        assert_eq!(c(&[]).patterns, 0);
        // A frame whose first byte is the pattern's last: no echo wins.
        let g = [0xFF, 9, 9, 9, 9, 9, 9, 9];
        let amb = parse(&p, &p, &g);
        assert_eq!((amb.patterns, amb.echoed), (1, 0));
    }

    /// The ruling on C1's parser: a pattern a reset cut short reads as a
    /// cut, never as stray, before the last whole pattern; the caller bounds
    /// cut and whole together by the boots.
    #[test]
    fn a_pattern_cut_by_a_reset_reads_as_a_cut_before_the_last_whole_one() {
        let p = pattern("GPS");
        let f = [1, 2, 3, 4, 5, 6, 7, 8];
        let cat = |parts: &[&[u8]]| parts.concat();
        let c = |b: &[u8]| parse(b, &p, &f);
        let one = c(&cat(&[&p[..4], &p, &f]));
        assert_eq!(
            (one.patterns, one.cut, one.echoed, one.stray),
            (1, 1, 8, false)
        );
        // Cuts and whole patterns interleave, each cut any proper prefix.
        let many = c(&cat(&[&p[..1], &p, &p[..7], &p[..3], &p, &f[..2]]));
        assert_eq!(
            (many.patterns, many.cut, many.echoed, many.stray),
            (2, 3, 2, false)
        );
        // A cut after the last whole pattern, a cut alone, a cut before the
        // frame, and a cut that is not the pattern's prefix are stray.
        for bad in [
            cat(&[&p, &p[..4]]),
            cat(&[&p, &p[..4], &f]),
            p[..4].to_vec(),
            cat(&[&p[..4], &f]),
            cat(&[&p[1..4], &p, &f]),
            cat(&[&p, &[0x00], &p, &f]),
        ] {
            assert!(c(&bad).stray, "{bad:02X?}");
        }
    }

    /// A label that overlaps itself: a greedy reading of `AB` then `ABAC…`
    /// takes `ABA` and fails; the fewest-patterns reading finds the cut.
    #[test]
    fn a_self_overlapping_pattern_still_reads_its_cut() {
        let p = pattern("ABAC");
        let f = [9; FRAME_LEN];
        let cap = parse(&[&p[..2], &p[..], &f[..]].concat(), &p, &f);
        assert_eq!((cap.patterns, cap.cut, cap.stray), (1, 1, false));
        let cap = parse(&[&p[..3], &p[..3], &p[..], &f[..]].concat(), &p, &f);
        assert_eq!((cap.patterns, cap.cut, cap.stray), (1, 2, false));
        // The fewest: `ABA` + `ABAC…` is one cut, never `AB` + `A` + ….
        let cap = parse(&[&p[..3], &p[..]].concat(), &p, &f);
        assert_eq!((cap.patterns, cap.cut), (1, 1));
    }

    /// Lines verbatim from Renode 1.17.0's access log (fix 1, P2's `v.out`).
    const WRITE_TDR: &str =
        "[INFO] usart1: [cpu: 0x80009DE] WriteUInt32 to 0x28 (TransmitData), value 0x43";
    const WRITE_PRESC: &str =
        "[INFO] usart1: [cpu: 0x800070A] WriteUInt32 to 0x2C (TransmitData+0x4), value 0x1";
    const WRITE_BYTE: &str =
        "[INFO] usart1: [cpu: 0x8000710] WriteByte to 0x1 (ControlRegister1+0x1), value 0x10";
    const READ_ISR: &str = "[INFO] usart1: [cpu: 0x80009B2] ReadUInt32 from 0x1C (InterruptAndStatus), returned 0x206000C0";
    const GPIO_HALF: &str =
        "[INFO] gpioPortA: [cpu: 0x8000728] WriteUInt16 to 0x0 (Mode), value 0xA801";

    #[test]
    fn an_access_parses_only_in_renode_s_measured_shape() {
        let a = |source: &str, write, bits, offset, value| Access {
            source: source.into(),
            write,
            bits,
            offset,
            value,
            refused: false,
        };
        assert_eq!(access(WRITE_TDR), Some(a("usart1", true, 32, 0x28, 0x43)));
        assert_eq!(access(WRITE_PRESC), Some(a("usart1", true, 32, 0x2C, 1)));
        assert_eq!(access(WRITE_BYTE), Some(a("usart1", true, 8, 1, 0x10)));
        assert_eq!(
            access(READ_ISR),
            Some(a("usart1", false, 32, 0x1C, 0x2060_00C0))
        );
        assert_eq!(access(GPIO_HALF), Some(a("gpioPortA", true, 16, 0, 0xA801)));
        assert!(is_unjudged_access(READ_ISR) && !is_unjudged_access(WRITE_TDR));
        // A GPIO port's writes are judged only in MODER, OTYPER, OSPEEDR,
        // PUPDR and AFR, a sub-word write into one included; the RCC's reads
        // are discarded, its writes kept.
        for (line, unjudged) in [
            (GPIO_HALF, false),
            (
                "[INFO] gpioPortA: [cpu: 0x8000A56] WriteUInt32 to 0x4 (OutputType), value 0x200",
                false,
            ),
            (
                "[INFO] gpioPortA: [cpu: 0x8000A56] WriteUInt32 to 0x8 (OutputSpeed), value 0x400",
                false,
            ),
            (
                "[INFO] gpioPortA: [cpu: 0x8000A56] WriteUInt32 to 0xC (PullUpPullDown), value 0x80000",
                false,
            ),
            (
                "[INFO] gpioPortA: [cpu: 0x8000A56] WriteUInt32 to 0x1C (ConfigurationLock), value 0x10200",
                true,
            ),
            (
                "[INFO] rcc: [cpu: 0x8000710] WriteUInt32 to 0x88 (AHB4Reset), value 0x1",
                false,
            ),
            (
                "[INFO] rcc: [cpu: 0x8000A2A] ReadUInt32 from 0xE0 (AHB4Clock), returned 0x1",
                true,
            ),
            (
                "[INFO] gpioPortA: [cpu: 0x8000728] WriteUInt32 to 0x18 (BitSet), value 0x1",
                true,
            ),
            (
                "[INFO] gpioPortA: [cpu: 0x8000728] WriteUInt32 to 0x14 (Output), value 0x1",
                true,
            ),
            (
                "[INFO] gpioPortA: [cpu: 0x8000728] WriteByte to 0x25 (AlternateFunctionHigh+0x1), value 0x7",
                false,
            ),
            (
                "[INFO] gpioPortA: [cpu: 0x80009BC] WriteUInt32 to 0x20 (AlternateFunctionLow), value 0x5500000",
                false,
            ),
            (
                "[INFO] usart1: [cpu: 0x8000728] WriteUInt32 to 0x18 (Request), value 0x1",
                false,
            ),
        ] {
            assert_eq!(is_unjudged_access(line), unjudged, "{line}");
        }
        assert!(logged_by(WRITE_TDR, "usart1") && !logged_by(WRITE_TDR, "usart"));
        for bad in [
            "[WARNING] usart1: [cpu: 0x80009DE] WriteUInt32 to 0x28 (TransmitData), value 0x43",
            "[INFO] usart1: [cpu: 0x80009de] WriteUInt32 to 0x28 (TransmitData), value 0x43",
            "[INFO] usart1: [cpu: 0x80009DE] WriteUInt32 to 0x28 (TransmitData), value 0x43 (2)",
            "[INFO] usart1: [cpu: 0x80009DE] WriteUInt32 from 0x28 (TransmitData), value 0x43",
            "[INFO] usart1: [cpu: 0x80009DE] WriteUInt8 to 0x28 (TransmitData), value 0x43",
            "[INFO] usart1: [cpu: 0x80009DE] ReadUInt32 from 0x1C (InterruptAndStatus), value 0x0",
            "[INFO] usart1: WriteUInt32 to 0x28 (TransmitData), value 0x43",
            "[INFO] usart 1: [cpu: 0x80009DE] WriteUInt32 to 0x28 (TransmitData), value 0x43",
            "[WARNING] usart1: Unhandled write to offset 0x2C, value 0x1.",
        ] {
            assert_eq!(access(bad), None, "{bad}");
        }
        // A locked pin's write refused, as Renode logs it (adversarial pass 2,
        // q4), and nothing else.
        for (line, source) in [
            (
                "[WARNING] gpioPortA: Ignoring attempt to change MODER configuration of the locked pin #9",
                Some("gpioPortA"),
            ),
            (
                "[WARNING] gpioPortB: Ignoring attempt to change AFSEL configuration of the locked pin #6",
                Some("gpioPortB"),
            ),
            (
                "[INFO] gpioPortA: Ignoring attempt to change MODER configuration of the locked pin #9",
                None,
            ),
            (
                "[WARNING] gpioPortB: Unhandled write to offset 0x4. Unhandled bits: [6] when writing value 0x40. Tags: OT6 (0x1).",
                None,
            ),
        ] {
            assert_eq!(refused_by(line), source, "{line}");
        }
    }

    fn framing(data_bits: u8, parity: Parity, stop_bits: u8, inverted: bool) -> Framing {
        Framing {
            baud: 100_000,
            data_bits,
            parity,
            stop_bits,
            inverted,
        }
    }

    #[test]
    fn the_framing_needs_its_tags_and_no_other() {
        let sbus = needed_tags(&framing(8, Parity::Even, 2, true)).unwrap();
        let names: Vec<&str> = sbus.iter().map(|t| t.field).collect();
        assert_eq!(names, ["M0", "RXINV", "TXINV"]);
        assert_eq!((sbus[0].offset, sbus[1].offset), (0, 4));
        assert_eq!(
            (sbus[0].mask, sbus[1].mask, sbus[2].mask),
            (1 << 12, 1 << 16, 1 << 17)
        );
        assert!(
            needed_tags(&framing(8, Parity::None, 1, false))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            needed_tags(&framing(7, Parity::None, 1, false)).unwrap()[0].field,
            "M1"
        );
    }

    fn w(source: &str, offset: u32, value: u32) -> Access {
        Access {
            source: source.into(),
            write: true,
            bits: 32,
            offset,
            value: value.into(),
            refused: false,
        }
    }

    /// The four-port fixture's SBUS on UART4: BRR, CR2, CR1, CR1 | UE, then a
    /// byte; as the generated board writes it, in FIFO mode.
    fn sbus_boot() -> Vec<Access> {
        vec![
            w("uart4", 0x0c, 640),
            w("uart4", 0x04, 0x0003_2000),
            w("uart4", 0x00, 0x2000_140c),
            w("uart4", 0x00, 0x2000_140d),
            w("uart4", 0x28, 0x53),
        ]
    }

    #[test]
    fn the_bring_up_s_words_come_from_the_framing_and_the_header() {
        let b = Bringup::of(&framing(8, Parity::Even, 2, true), 640).unwrap();
        assert_eq!(
            b.words,
            [
                ("CR1", 0, 0x2000_140d),
                ("CR2", 4, 0x3_2000),
                ("CR3", 8, 0),
                ("BRR", 0xc, 640)
            ]
        );
        assert_eq!((b.tdr, b.ue), (0x28, 1));
        let console = Bringup::of(&framing(8, Parity::None, 1, false), 556).unwrap();
        assert_eq!(console.words[0].2, 0x2000_000d);
        assert_eq!(console.words[1].2, 0);
    }

    /// Adversarial F1, F2, F3: the words in force at the first write that
    /// sets `UE` are judged, from the values written, not from which writes
    /// set a tag; nothing changes after it; and a write the bring-up never
    /// makes is seen whatever its offset or width.
    #[test]
    fn adversarial_f1_f2_f3_the_writes_are_judged_at_ue_and_after() {
        let b = Bringup::of(&framing(8, Parity::Even, 2, true), 640).unwrap();
        let j = judge_writes(&b, "uart4", &[sbus_boot()]);
        assert!(j.holds(), "{j:?}");
        assert_eq!(j.writes, 5);
        let with = |edit: &dyn Fn(&mut Vec<Access>)| {
            let mut boot = sbus_boot();
            edit(&mut boot);
            judge_writes(&b, "uart4", &[boot])
        };
        // F1 (ADV-3): M0 set, then cleared by a second CR1 write before UE.
        let j = with(&|v| {
            v[2] = w("uart4", 0, 0x2000_040c);
            v[3] = w("uart4", 0, 0x2000_040d);
        });
        assert_eq!(j.missing, ["CR1.M0"]);
        assert!(j.unexpected.is_empty(), "{j:?}");
        // F2 (ADV-4): CR2 without inversion, inverted only after UE.
        let j = with(&|v| {
            v[1] = w("uart4", 4, 0x2000);
            v.insert(4, w("uart4", 4, 0x3_2000));
        });
        assert_eq!(j.missing, ["CR2.RXINV", "CR2.TXINV"]);
        assert_eq!(j.unexpected, ["CR2 0x32000 after UE"]);
        // F3 (ADV-2): PRESC, GTPR, RTOR, RQR, CR3: never written by the bring-up.
        for at in [0x2c, 0x10, 0x14, 0x18, 0x08] {
            let j = with(&|v| v.insert(2, w("uart4", at, 1)));
            assert_eq!(j.unexpected, [format!("0x1 to {at:#X}")], "{at:#x}");
        }
        // ADV-2b: UE cleared, PRESC, UE set again, after the bring-up.
        let j = with(&|v| {
            v.extend([
                w("uart4", 0, 0x2000_140c),
                w("uart4", 0x2c, 1),
                w("uart4", 0, 0x2000_140d),
            ])
        });
        assert_eq!(
            j.unexpected,
            [
                "CR1 0x2000140C after UE",
                "0x1 to 0x2C",
                "CR1 0x2000140D after UE"
            ]
        );
        // A byte write that sets M0 is no word write (measured, P2).
        let j = with(&|v| {
            v.insert(
                2,
                Access {
                    bits: 8,
                    ..w("uart4", 1, 0x10)
                },
            )
        });
        assert_eq!(j.unexpected, ["WriteByte to 0x1"]);
        // A tag set that the framing does not need, a stored bit off, a
        // byte before UE, and UE never set.
        let j = with(&|v| v[3] = w("uart4", 0, 0x3000_140d));
        assert_eq!(
            j.unexpected,
            [
                "CR1 0x10000000 set, a bit Renode does not store",
                "CR1.M1 set at UE"
            ]
        );
        // The PM's ruling on fix 2's A2 hole: an unstored bit the bring-up
        // leaves clear, set and then cleared (a read-modify-write reads it as
        // 0), fails wherever it is set: M1 before UE, CR2's ADDM7 after it.
        let j = with(&|v| v.insert(2, w("uart4", 0, 0x3000_140c)));
        assert_eq!(
            j.unexpected,
            ["CR1 0x10000000 set, a bit Renode does not store"]
        );
        let j = with(&|v| v.extend([w("uart4", 4, 0x3_2010), w("uart4", 4, 0x3_2000)]));
        assert_eq!(
            j.unexpected,
            [
                "CR2 0x10 set, a bit Renode does not store",
                "CR2 0x32010 after UE",
                "CR2 0x32000 after UE"
            ]
        );
        // A stored bit, or one the bring-up sets (M0, the inversion), is not.
        assert_eq!(b.unstored[0] & 0x8d, 0);
        assert_eq!(b.unstored[0] & 0x2000_1000, 0x2000_1000);
        let j = with(&|v| v[3] = w("uart4", 0, 0x2000_340d));
        assert_eq!(
            j.unexpected,
            [
                "CR1 0x2000 set, a bit Renode does not store",
                "CR1 0x2000240D at UE, not 0x2000040D"
            ]
        );
        let j = with(&|v| v[3] = w("uart4", 0, 0x2000_140f));
        assert_eq!(
            j.unexpected,
            [
                "CR1 0x2 set, a bit Renode does not store",
                "CR1 0x2000040F at UE, not 0x2000040D"
            ]
        );
        // The PM's ruling on pass 2's H1: FIFO mode is on at UE, which
        // Renode stores nowhere; written with UE clear, then cleared with it.
        let j = with(&|v| v[3] = w("uart4", 0, 0x140d));
        assert_eq!(j.unexpected, ["CR1 0x40D at UE, not 0x2000040D"]);
        let j = with(&|v| v.insert(0, w("uart4", 0x28, 0)));
        assert_eq!(j.unexpected, ["TDR before UE"]);
        let j = with(&|v| v.truncate(3));
        assert_eq!(j.unexpected, ["the last boot never set CR1.UE"]);
        // Another USART's writes are not this port's.
        let mut boot = sbus_boot();
        boot.push(w("usart1", 0x2c, 1));
        assert!(judge_writes(&b, "uart4", &[boot]).holds());
        // A boot cut by a reset before UE is judged as far as it went; the
        // last boot is judged whole, and a boot's words start from reset.
        let cut = sbus_boot()[..2].to_vec();
        assert!(judge_writes(&b, "uart4", &[cut.clone(), sbus_boot()]).holds());
        assert!(!judge_writes(&b, "uart4", &[sbus_boot(), cut]).holds());
        let rest = sbus_boot()[2..].to_vec();
        let j = judge_writes(&b, "uart4", &[sbus_boot(), rest]);
        assert_eq!(j.missing, ["CR2.RXINV", "CR2.TXINV"]);
    }

    /// Adversarial F4: the TX pin counts only if it was routed at the first
    /// `TDR` write of each boot, rebuilt from the words written before it.
    #[test]
    fn adversarial_f4_the_tx_pin_is_judged_at_the_first_tdr_write() {
        let tx = PinMux::of("PA9", 7).unwrap();
        assert_eq!((tx.source, tx.offsets), ("gpioPortA", (0, 0x24)));
        let routed = [w("gpioPortA", 0x24, 0x70), w("gpioPortA", 0, 0x8_0000)];
        let send = w("usart1", 0x28, 0x43);
        let boot = |before: &[Access], after: &[Access]| {
            let mut b = before.to_vec();
            b.push(send.clone());
            b.extend_from_slice(after);
            b
        };
        let got = tx_at_send(&tx, "usart1", 0x28, &[boot(&routed, &[])]);
        assert_eq!(got, [Some((0x8_0000, 0x70))]);
        assert!(got[0].is_some_and(|(m, a)| tx.routed(m, a)));
        // ADV-1: PA9 an input while the pattern goes out, routed afterwards,
        // before the pattern's next byte: the first TDR write is the one.
        let mut after = routed[1..].to_vec();
        after.push(send.clone());
        let got = tx_at_send(&tx, "usart1", 0x28, &[boot(&routed[..1], &after)]);
        assert_eq!(got, [Some((0, 0x70))]);
        assert!(!got[0].is_some_and(|(m, a)| tx.routed(m, a)));
        // A half-word write to MODER cannot be rebuilt: not routed.
        let half = Access {
            bits: 16,
            ..w("gpioPortA", 0, 0x8_0000)
        };
        let got = tx_at_send(
            &tx,
            "usart1",
            0x28,
            &[boot(&[routed[0].clone(), routed[1].clone(), half], &[])],
        );
        assert_eq!(got, [None]);
        // Each boot from reset: a boot after a release that sends before routing.
        let got = tx_at_send(
            &tx,
            "usart1",
            0x28,
            &[boot(&routed, &[]), boot(&[], &routed)],
        );
        assert_eq!(got, [Some((0x8_0000, 0x70)), Some((0, 0))]);
        // The PM's ruling on F4's residual: routed, one byte, unrouted, the
        // rest, routed again before the end: the unrouted writes count.
        let mut residual = routed.to_vec();
        residual.push(send.clone());
        residual.push(w("gpioPortA", 0, 0));
        residual.extend(std::iter::repeat_n(send.clone(), 11));
        residual.push(routed[1].clone());
        let got = tx_at_send(&tx, "usart1", 0x28, &[residual]);
        assert_eq!(got, [Some((0, 0x70))]);
        // Another port's GPIO words, and a boot with no TDR write.
        let other = [w("gpioPortB", 0x24, 0x70), w("gpioPortB", 0, 0x8_0000)];
        assert_eq!(
            tx_at_send(&tx, "usart1", 0x28, &[boot(&other, &[])]),
            [Some((0, 0))]
        );
        assert!(tx_at_send(&tx, "usart1", 0x28, &[routed.to_vec()]).is_empty());
    }

    /// Adversarial pass 3, ADV3-1 and 1b (the PM's ruling; it subsumes pass
    /// 2's A1, ADV2-1): the RCC's writes are the enable registers' alone, as
    /// words, and none clears the gate of the port's USART or of its pins'
    /// GPIO ports once set in that boot.
    #[test]
    fn adversarial_3_adv3_1_the_rcc_writes_only_enable_registers() {
        let rule = rcc_rule("USART1", &["PA9", "PA10"]).unwrap();
        let gates: Vec<(&str, &str, u32, u32)> = rule
            .gates
            .iter()
            .map(|g| (g.peripheral.as_str(), g.register.as_str(), g.offset, g.mask))
            .collect();
        assert_eq!(
            gates,
            [
                ("USART1", "APB2ENR", 0xF0, 1 << 4),
                ("GPIOA", "AHB4ENR", 0xE0, 1)
            ]
        );
        // AHB3ENR at 0xD4 through APB4ENR at 0xF4, no sleep enable.
        assert_eq!(
            rule.enables,
            [0xD4, 0xD8, 0xDC, 0xE0, 0xE4, 0xE8, 0xEC, 0xF0, 0xF4]
        );
        // The reference's RCC writes (measured, fix 3 m1).
        let enable = [
            w("rcc", 0xE0, 1),
            w("rcc", 0xE0, 3),
            w("rcc", 0xE8, 0x20_0000),
            w("rcc", 0xF0, 0x1000),
            w("rcc", 0xF0, 0x1010),
        ];
        assert!(judge_rcc(&rule, &[enable.to_vec()]).is_empty());
        let with = |extra: &[Access]| {
            let mut boot = enable.to_vec();
            boot.extend_from_slice(extra);
            judge_rcc(&rule, &[boot])
        };
        // ADV3-1: D2PPRE2 /2 after the bring-up; ADV3-1b: USART16SEL CSI.
        assert_eq!(
            with(&[w("rcc", 0x1C, 0x400)]),
            ["RCC.D2CFGR 0x400 written, not an enable register"]
        );
        assert_eq!(
            with(&[w("rcc", 0x54, 0x20)]),
            ["RCC.D2CCIP2R 0x20 written, not an enable register"]
        );
        // The clock's other words, a sleep enable, a reserved word and one
        // past the layout, each named; written to their reset value too.
        for (at, name) in [
            (0x00, "CR"),
            (0x10, "CFGR"),
            (0x18, "D1CFGR"),
            (0x20, "D3CFGR"),
            (0x118, "APB2LPENR"),
            (0x14, "0x14"),
            (0x130, "0x130"),
        ] {
            assert_eq!(
                with(&[w("rcc", at, 0)]),
                [format!("RCC.{name} 0x0 written, not an enable register")],
                "{at:#x}"
            );
        }
        // Pass 2's A1 (ADV2-1): GPIOARST pulsed; A1b: USART1RST. Another
        // peripheral's reset is no longer allowed either.
        assert_eq!(
            with(&[w("rcc", 0x88, 1), w("rcc", 0x88, 0)]),
            [
                "RCC.AHB4RSTR 0x1 written, not an enable register",
                "RCC.AHB4RSTR 0x0 written, not an enable register"
            ]
        );
        assert_eq!(
            with(&[w("rcc", 0x98, 0x1011)]),
            ["RCC.APB2RSTR 0x1011 written, not an enable register"]
        );
        // A sub-word or unaligned write anywhere in the RCC, an enable
        // register's included, cannot be rebuilt.
        let byte = Access {
            bits: 8,
            ..w("rcc", 0xF0, 0)
        };
        assert_eq!(with(&[byte]), ["WriteByte to RCC 0xF0"]);
        assert_eq!(with(&[w("rcc", 0xF2, 0)]), ["WriteUInt32 to RCC 0xF2"]);
        // The gate of the port's USART, or of its pins' port, cleared once
        // set; another peripheral's gate, or the gate written clear before it
        // is set, is not.
        assert_eq!(
            with(&[w("rcc", 0xF0, 0x1000)]),
            ["RCC.APB2ENR.USART1EN cleared after it was set"]
        );
        assert_eq!(
            with(&[w("rcc", 0xE0, 2), w("rcc", 0xE0, 3)]),
            ["RCC.AHB4ENR.GPIOAEN cleared after it was set"]
        );
        assert!(with(&[w("rcc", 0xE0, 1), w("rcc", 0xF0, 0x10)]).is_empty());
        let before = [w("rcc", 0xF0, 0), w("rcc", 0xF0, 0x10)];
        assert!(judge_rcc(&rule, &[before.to_vec()]).is_empty());
        // Each boot from reset: a gate set in one boot and clear in the next
        // before it is set again.
        assert!(judge_rcc(&rule, &[enable.to_vec(), before.to_vec()]).is_empty());
        // Another source's write at the same offset is not the RCC's.
        assert!(with(&[w("gpioPortA", 0x1C, 1)]).is_empty());
        // A UART8 port's pins on port E: APB1LENR's 31 and AHB4ENR's 4.
        let uart8 = rcc_rule("UART8", &["PE1", "PE0"]).unwrap();
        let g: Vec<(u32, u32)> = uart8.gates.iter().map(|g| (g.offset, g.mask)).collect();
        assert_eq!(g, [(0xE8, 1 << 31), (0xE0, 1 << 4)]);
    }

    /// The console's TX and RX as the generated board leaves them: each word
    /// written whole, push-pull, low speed, no pull, then routed.
    fn console_pins() -> Vec<Access> {
        vec![
            w("gpioPortA", 0x4, 0),
            w("gpioPortA", 0x8, 0),
            w("gpioPortA", 0xC, 0),
            w("gpioPortA", 0x24, 0x770),
            w("gpioPortA", 0, 0x28_0000),
        ]
    }

    /// Adversarial pass 2, A2 (ADV2-2): each pin must be push-pull, low
    /// speed and pulled neither way, rebuilt from the words written: the TX
    /// pin at every TDR write, the RX pin at the injection, both at the end.
    #[test]
    fn adversarial_2_a2_the_pins_are_push_pull_low_speed_and_unpulled() {
        let (tx, rx) = (
            PinMux::of("PA9", 7).unwrap(),
            PinMux::of("PA10", 7).unwrap(),
        );
        assert_eq!(tx.config_offsets, [4, 8, 0xC]);
        let send = w("usart1", 0x28, 0x43);
        let judge = |boot: &[Access], inject_at| {
            judge_pins(&tx, &rx, "usart1", 0x28, inject_at, &[boot.to_vec()])
        };
        let mut good = console_pins();
        good.push(send.clone());
        assert!(judge(&good, Some(good.len())).is_empty());
        assert!(judge(&good, None).is_empty());
        // ADV2-2: PA9 open-drain with a pull-down after it is routed.
        let mut bad = console_pins();
        bad.extend([w("gpioPortA", 0x4, 1 << 9), w("gpioPortA", 0xC, 0b10 << 18)]);
        bad.push(send.clone());
        assert_eq!(
            judge(&bad, None),
            [
                "PA9 OT set in OTYPER, a bit Renode does not store",
                "PA9 OTYPER 1, OSPEEDR 0b00, PUPDR 0b10 at a TDR write",
                "PA9 OTYPER 1, OSPEEDR 0b00, PUPDR 0b10 at the end"
            ]
        );
        // Open-drain for the pattern only, push-pull again before the end.
        let mut brief = bad.clone();
        brief.extend([w("gpioPortA", 0x4, 0), w("gpioPortA", 0xC, 0)]);
        assert_eq!(
            judge(&brief, None),
            [
                "PA9 OT set in OTYPER, a bit Renode does not store",
                "PA9 OTYPER 1, OSPEEDR 0b00, PUPDR 0b10 at a TDR write"
            ]
        );
        // The PM's ruling on fix 2's A2 hole (ADV2-6): OT9 set, then OTYPER
        // read-modify-written; Renode reads OT back as 0, so the log shows it
        // cleared before the pattern, which silicon would not.
        let mut rmw = console_pins();
        rmw.extend([w("gpioPortA", 0x4, 1 << 9), w("gpioPortA", 0x4, 0)]);
        rmw.push(send.clone());
        assert_eq!(
            judge(&rmw, None),
            ["PA9 OT set in OTYPER, a bit Renode does not store"]
        );
        // Another pin's OT on the port is not a serial pin's.
        let mut other = good.clone();
        other.insert(0, w("gpioPortA", 0x4, 1 << 8));
        assert!(judge(&other, None).is_empty());
        // PA10 pulled up at the injection, released after it; a speed set.
        let mut rx_bad = good.clone();
        rx_bad.push(w("gpioPortA", 0xC, 0b01 << 20));
        let at = rx_bad.len();
        rx_bad.push(w("gpioPortA", 0xC, 0));
        assert_eq!(
            judge(&rx_bad, Some(at)),
            ["PA10 OTYPER 0, OSPEEDR 0b00, PUPDR 0b01 at the injection"]
        );
        rx_bad.push(w("gpioPortA", 0x8, 0b11 << 20));
        assert_eq!(
            judge(&rx_bad, None),
            ["PA10 OTYPER 0, OSPEEDR 0b11, PUPDR 0b00 at the end"]
        );
        // A half-word write into OSPEEDR cannot be rebuilt.
        let mut half = good.clone();
        half.insert(
            0,
            Access {
                bits: 16,
                ..w("gpioPortA", 0xA, 0)
            },
        );
        // Both pins' word.
        assert_eq!(
            judge(&half, None),
            [
                "PA9 OTYPER/OSPEEDR/PUPDR not rebuilt at a TDR write",
                "PA9 OTYPER/OSPEEDR/PUPDR not rebuilt at the end",
                "PA10 OTYPER/OSPEEDR/PUPDR not rebuilt at the end"
            ]
        );
        // A word never written holds its reset value: PB4's pull-up and
        // PB3's very high speed (RM0433 Rev 9 §11.4.3, §11.4.4).
        let (pb4, pb3) = (
            PinMux::of("PB4", 11).unwrap(),
            PinMux::of("PB3", 11).unwrap(),
        );
        assert_eq!(pb4.config_reset, [0, 0xC0, 0x100]);
        let tdr = [w("uart7", 0x28, 0x43)];
        assert_eq!(
            judge_pins(&pb4, &pb3, "uart7", 0x28, None, &[tdr.to_vec()]),
            [
                "PB4 OTYPER 0, OSPEEDR 0b00, PUPDR 0b01 at a TDR write",
                "PB4 OTYPER 0, OSPEEDR 0b00, PUPDR 0b01 at the end",
                "PB3 OTYPER 0, OSPEEDR 0b11, PUPDR 0b00 at the end"
            ]
        );
        // Each boot from reset: a boot after a release judged on its own.
        let rest = vec![send.clone()];
        let got = judge_pins(&tx, &rx, "usart1", 0x28, None, &[good.clone(), rest]);
        assert!(got.is_empty(), "{got:?}");
        assert!(PinMux::of("PA9", 7).unwrap().config_reset[2] & (0b11 << 18) == 0);
    }

    /// Adversarial pass 2, L2 (ADV2-5): a write the model refused (a pin
    /// `LCKR` locked) fails by name, and the rebuild does not apply it.
    #[test]
    fn adversarial_2_l2_a_refused_write_is_named_and_not_applied() {
        let (tx, rx) = (
            PinMux::of("PA9", 7).unwrap(),
            PinMux::of("PA10", 7).unwrap(),
        );
        let mut boot = console_pins();
        boot[4].refused = true;
        boot.push(w("usart1", 0x28, 0x43));
        assert_eq!(
            judge_pins(&tx, &rx, "usart1", 0x28, None, &[boot.clone()]),
            ["gpioPortA refused a write to 0x0 (a locked pin)"]
        );
        assert_eq!(tx_at_send(&tx, "usart1", 0x28, &[boot]), [Some((0, 0x770))]);
        // Another port's refusal is not this port's.
        let mut other = console_pins();
        other.push(Access {
            refused: true,
            ..w("gpioPortB", 0, 0)
        });
        assert!(judge_pins(&tx, &rx, "usart1", 0x28, None, &[other]).is_empty());
    }

    /// The PM's ruling on F5: one character at the solved actual baud, in ns
    /// rounded up; the budget is the pattern, the frame and one character.
    /// 115200 8N1 (BRR 556, 115 107.913 baud): 10 bits, 86 876 ns, 21
    /// characters for `CONSOLE`, 1 824 396 ns. 9600 8N1 (BRR 6667, 9 599.520
    /// baud): 1 041 719 ns, 21 876 099 ns. SBUS 100000 8E2: 12 bits, 120 000
    /// ns.
    #[test]
    fn a_port_s_wire_time_is_its_characters_at_the_solved_baud() {
        let f = |baud, data_bits, parity, stop_bits| Framing {
            baud,
            data_bits,
            parity,
            stop_bits,
            inverted: false,
        };
        let console = f(115_200, 8, Parity::None, 1);
        assert_eq!(char_ns(&console, 115_107_913), Some(86_876));
        assert_eq!(wire_ns("CONSOLE", &console, 115_107_913), Some(1_824_396));
        let slow = f(9_600, 8, Parity::None, 1);
        assert_eq!(char_ns(&slow, 9_599_520), Some(1_041_719));
        assert_eq!(wire_ns("CONSOLE", &slow, 9_599_520), Some(21_876_099));
        assert_eq!(
            char_ns(&f(100_000, 8, Parity::Even, 2), 100_000_000),
            Some(120_000)
        );
        // Exact: 10 bits at 10 000 baud is 1 ms, not rounded up past it.
        assert_eq!(char_ns(&slow, 10_000_000), Some(1_000_000));
        assert_eq!(char_ns(&slow, 0), None);
    }

    #[test]
    fn every_stock_uart_is_captured() {
        assert_eq!(
            stock_uarts(),
            [
                "usart1", "usart2", "usart3", "uart4", "uart5", "usart6", "uart7", "uart8",
                "lpuart1"
            ]
        );
    }

    #[test]
    fn a_pin_is_routed_only_in_af_mode_on_its_af() {
        let m = PinMux::of("PA10", 7).unwrap();
        assert_eq!(m.moder.0, "GPIOA.MODER");
        assert_eq!(m.afr.0, "GPIOA.AFRH");
        assert_eq!(m.afr.1 - m.moder.1, 0x24);
        let af = 0b10 << 20;
        assert!(m.routed(af, 7 << 8));
        assert!(!m.routed(af, 8 << 8));
        assert!(!m.routed(0b11 << 20, 7 << 8));
        assert_eq!(m.describe(0b11 << 20, 8 << 8), "PA10 MODER 0b11, AF8");
        assert_eq!(PinMux::of("PB3", 7).unwrap().afr.0, "GPIOB.AFRL");
    }
}
