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
//! A boot's capture of one port is `P^k F`:
//! - `P`, the pattern ([`pattern`]): the label's bytes and [`PATTERN_TAIL`],
//!   sent once by each boot that reached it, so `k` is at least 1 and at most
//!   the releases plus one;
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

/// The file the composition captures `peripheral`'s bytes into, inside the
/// boot's fresh capture directory: `usart1.bin`.
pub fn capture_file(peripheral: &str) -> Result<String, TwinError> {
    Ok(format!("{}.bin", renode_name(peripheral)?))
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

/// What one port's capture holds, read against its pattern and frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    /// The whole patterns at the start, in a row.
    pub patterns: usize,
    /// How many of the frame's bytes follow them, in order: [`FRAME_LEN`]
    /// for the whole frame, fewer for its first bytes only.
    pub echoed: usize,
    /// True when the bytes are not `P^k` then the frame's first `echoed`
    /// bytes, exactly: anything else is in the capture.
    pub stray: bool,
    pub len: usize,
}

/// Reads `bytes` as `P^k` then a prefix of `frame`, from the frame's end:
/// - the whole frame at the end, then `P^k` before it;
/// - else `P^k` alone (no echo; this wins where a partial echo would also
///   read, so a pattern's last bytes are never taken for the frame's first);
/// - else the longest proper prefix of the frame at the end, `P^k` before it;
/// - else the whole patterns at the start, and `stray` set.
pub fn parse(bytes: &[u8], pattern: &[u8], frame: &[u8; FRAME_LEN]) -> Capture {
    let whole = |body: &[u8]| -> Option<usize> {
        if pattern.is_empty() || !body.len().is_multiple_of(pattern.len()) {
            return None;
        }
        body.chunks(pattern.len())
            .all(|c| c == pattern)
            .then(|| body.len() / pattern.len())
    };
    let done = |patterns, echoed| Capture {
        patterns,
        echoed,
        stray: false,
        len: bytes.len(),
    };
    if let Some(body) = bytes.strip_suffix(&frame[..])
        && let Some(k) = whole(body)
    {
        return done(k, FRAME_LEN);
    }
    if let Some(k) = whole(bytes) {
        return done(k, 0);
    }
    for n in (1..FRAME_LEN).rev() {
        if let Some(body) = bytes.strip_suffix(&frame[..n])
            && let Some(k) = whole(body)
        {
            return done(k, n);
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
        echoed: 0,
        stray: true,
        len: bytes.len(),
    }
}

/// One `Unhandled write` warning Renode logs for a write that sets a tag
/// (`PeripheralRegister.cs` L755–770, `TagLogger`): the source, the
/// register's offset, the whole value written, and each tag the write's
/// unhandled bits touch with its value in the write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagWarning {
    pub source: String,
    pub offset: u32,
    pub value: u32,
    pub tags: Vec<(String, u32)>,
}

/// Parses a whole log line's text after its timestamp:
/// `[WARNING] usart1: Unhandled write to offset 0x0. Unhandled bits: [12]
/// when writing value 0x100D. Tags: MO (0x1).` Renode writes the offset, the
/// value and each tag's value as `0x{:X}`, the bits as `GetSetBitsPretty`
/// (`BitHelper.cs` L413), and the tags joined by `, `. `None` for any other
/// line, a warning with no tag included.
pub fn tag_warning(body: &str) -> Option<TagWarning> {
    let hex = |s: &str| -> Option<u32> {
        let h = s.strip_prefix("0x")?;
        (!h.is_empty()
            && h.len() <= 8
            && h.bytes()
                .all(|b| b.is_ascii_digit() || b.is_ascii_uppercase()))
        .then(|| u32::from_str_radix(h, 16).ok())
        .flatten()
    };
    let rest = body.strip_prefix("[WARNING] ")?;
    let (source, rest) = rest.split_once(": Unhandled write to offset ")?;
    if source.is_empty() || source.contains(' ') {
        return None;
    }
    let (offset, rest) = rest.split_once(". Unhandled bits: [")?;
    let (bits, rest) = rest.split_once("] when writing value ")?;
    if bits.is_empty()
        || !bits
            .bytes()
            .all(|b| b.is_ascii_digit() || b == b'-' || b == b',' || b == b' ')
    {
        return None;
    }
    let (value, rest) = rest.split_once(". Tags: ")?;
    let tags = rest.strip_suffix('.')?;
    let mut parsed = Vec::new();
    for tag in tags.split(", ") {
        let (name, v) = tag.split_once(" (")?;
        let v = hex(v.strip_suffix(')')?)?;
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return None;
        }
        parsed.push((name.to_owned(), v));
    }
    Some(TagWarning {
        source: source.to_owned(),
        offset: hex(offset)?,
        value: hex(value)?,
        tags: parsed,
    })
}

/// A tag the framing needs set: the register's name and offset, the field's
/// name in ST's header, and its name as Renode's model logs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub register: &'static str,
    pub offset: u32,
    pub field: &'static str,
    /// `MO` for `M0` (`STM32F7_USART.cs` L162), else the header's name.
    pub renode: &'static str,
}

/// The tags `framing` needs set, in register order: `M0` for a 9-bit word
/// and `M1` for a 7-bit one (`M[1:0]`, RM0433 Rev 9 §48.8.1), `RXINV` and
/// `TXINV` when inverted (§48.8.2). Renode stores none of them
/// (`STM32F7_USART.cs` L162, L167, L177–178): each is seen only as the
/// warning a write that sets it logs. Offsets from the header's layout.
pub fn needed_tags(framing: &Framing) -> Result<Vec<Tag>, TwinError> {
    let layout = cmsis::register_facts()?.layout("USART")?;
    let (cr1, cr2) = (layout.offset("CR1", 0)?, layout.offset("CR2", 0)?);
    let m = framing.m_bits().ok_or_else(|| {
        TwinError::Register(RegisterError::Undefined {
            name: format!("USART_CR1.M for a {}-bit word", framing.word_bits()),
        })
    })?;
    let mut out = Vec::new();
    if m & 0b01 != 0 {
        out.push(Tag {
            register: "CR1",
            offset: cr1,
            field: "M0",
            renode: "MO",
        });
    }
    if m & 0b10 != 0 {
        out.push(Tag {
            register: "CR1",
            offset: cr1,
            field: "M1",
            renode: "M1",
        });
    }
    if framing.inverted {
        for f in ["RXINV", "TXINV"] {
            out.push(Tag {
                register: "CR2",
                offset: cr2,
                field: f,
                renode: f,
            });
        }
    }
    Ok(out)
}

/// What one port's tag warnings show against its framing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagJudgement {
    /// The warnings from the port's USART, in order.
    pub warnings: usize,
    /// Each needed tag no warning carried at 1, as `CR1.M0`.
    pub missing: Vec<String>,
    /// Each tag a warning carried that the framing does not need, or at
    /// another offset or value: `CR2.SWAP (0x1) at 0x4`.
    pub unexpected: Vec<String>,
}

impl TagJudgement {
    pub fn holds(&self) -> bool {
        self.missing.is_empty() && self.unexpected.is_empty()
    }
}

/// Judges the warnings `source` logged (`usart1`) against `needed`: every
/// needed tag carried at 1, at its register's offset, by at least one
/// warning; and no warning carrying any other tag, a needed one at another
/// offset, or any tag at another value.
pub fn judge_tags(source: &str, needed: &[Tag], warnings: &[TagWarning]) -> TagJudgement {
    let mut j = TagJudgement::default();
    let mine: Vec<&TagWarning> = warnings.iter().filter(|w| w.source == source).collect();
    j.warnings = mine.len();
    for t in needed {
        let carried = mine
            .iter()
            .any(|w| w.offset == t.offset && w.tags.iter().any(|(n, v)| n == t.renode && *v == 1));
        if !carried {
            j.missing.push(format!("{}.{}", t.register, t.field));
        }
    }
    for w in &mine {
        for (name, v) in &w.tags {
            let ok = *v == 1
                && needed
                    .iter()
                    .any(|t| t.renode == name && t.offset == w.offset);
            let shown = format!("{name} ({v:#X}) at {:#X}", w.offset);
            if !ok && !j.unexpected.contains(&shown) {
                j.unexpected.push(shown);
            }
        }
    }
    j
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
/// what the composition reads for a port's RX pin at the injection instant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinMux {
    pub pin: String,
    /// `GPIOA.MODER` and its address.
    pub moder: (String, u32),
    /// `GPIOA.AFRH` and its address.
    pub afr: (String, u32),
    /// The pin's index in its port.
    pub index: u32,
    pub alternate_function: u8,
}

/// `MODER` = `10`, alternate function (RM0433 Rev 9 §11.4.1).
const MODE_AF: u32 = 0b10;

impl PinMux {
    /// The pin's mux from the header's layout.
    pub fn of(pin: &str, alternate_function: u8) -> Result<Self, TwinError> {
        let (port, index) = crate::split_pin(pin)?;
        let f = cmsis::register_facts()?;
        let gpio = f.layout("GPIO")?;
        let base = f.base(&format!("GPIO{port}"))?;
        let word = index / 8;
        Ok(Self {
            pin: pin.to_owned(),
            moder: (format!("GPIO{port}.MODER"), base + gpio.offset("MODER", 0)?),
            afr: (
                format!("GPIO{port}.{}", ["AFRL", "AFRH"][word as usize]),
                base + gpio.offset("AFR", word)?,
            ),
            index,
            alternate_function,
        })
    }

    /// True when `moder` and `afr` put the pin in AF mode on its solved AF.
    pub fn routed(&self, moder: u32, afr: u32) -> bool {
        let mode = (moder >> (2 * self.index)) & 0b11;
        let af = (afr >> (4 * (self.index % 8))) & 0xF;
        mode == MODE_AF && af == u32::from(self.alternate_function)
    }

    /// `PA10 MODER 0b10, AF7`: what the reads showed, for a finding.
    pub fn describe(&self, moder: u32, afr: u32) -> String {
        format!(
            "{} MODER {:#04b}, AF{}",
            self.pin,
            (moder >> (2 * self.index)) & 0b11,
            (afr >> (4 * (self.index % 8))) & 0xF
        )
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
        assert_eq!(capture_file("UART7").unwrap(), "uart7.bin");
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
        // The frame without a pattern, a byte after the frame, a cut pattern,
        // and the pattern of another port are all stray or patternless.
        assert_eq!(c(&f).patterns, 0);
        assert!(c(&cat(&[&p, &f, &[0]])).stray);
        assert!(c(&cat(&[&p[..4], &p, &f])).stray);
        let other = c(&cat(&[&pattern("RC"), &f]));
        assert!(other.stray && other.patterns == 0);
        assert_eq!(c(&[]), parse(&[], &p, &f));
        assert_eq!(c(&[]).patterns, 0);
        // A frame whose first byte is the pattern's last: no echo wins.
        let g = [0xFF, 9, 9, 9, 9, 9, 9, 9];
        let amb = parse(&p, &p, &g);
        assert_eq!((amb.patterns, amb.echoed), (1, 0));
    }

    #[test]
    fn a_tag_warning_parses_only_in_renode_s_shape() {
        let w = tag_warning(
            "[WARNING] usart1: Unhandled write to offset 0x0. Unhandled bits: [12] when writing value 0x100D. Tags: MO (0x1).",
        )
        .unwrap();
        assert_eq!(
            w,
            TagWarning {
                source: "usart1".into(),
                offset: 0,
                value: 0x100D,
                tags: vec![("MO".into(), 1)]
            }
        );
        let two = tag_warning(
            "[WARNING] uart4: Unhandled write to offset 0x4. Unhandled bits: [16-17] when writing value 0x32000. Tags: RXINV (0x1), TXINV (0x1).",
        )
        .unwrap();
        assert_eq!(two.offset, 4);
        assert_eq!(two.tags.len(), 2);
        for bad in [
            "[INFO] usart1: Unhandled write to offset 0x0. Unhandled bits: [12] when writing value 0x100D. Tags: MO (0x1).",
            "[WARNING] usart1: Unhandled write to offset 0x0. Unhandled bits: [12] when writing value 0x100d. Tags: MO (0x1).",
            "[WARNING] usart1: Unhandled write to offset 0x0. Unhandled bits: [12] when writing value 0x100D.",
            "[WARNING] usart1: Unhandled write to offset 0x0. Unhandled bits: [12] when writing value 0x100D. Tags: MO (0x1)",
            "[WARNING] usart1: Unhandled write to offset 0x0. Unhandled bits: [x] when writing value 0x100D. Tags: MO (0x1).",
            "[WARNING] a b: Unhandled write to offset 0x0. Unhandled bits: [12] when writing value 0x100D. Tags: MO (0x1).",
        ] {
            assert_eq!(tag_warning(bad), None, "{bad}");
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
        let names: Vec<&str> = sbus.iter().map(|t| t.renode).collect();
        assert_eq!(names, ["MO", "RXINV", "TXINV"]);
        assert_eq!((sbus[0].offset, sbus[1].offset), (0, 4));
        assert!(
            needed_tags(&framing(8, Parity::None, 1, false))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            needed_tags(&framing(7, Parity::None, 1, false)).unwrap()[0].field,
            "M1"
        );
        let warn = |source: &str, offset, tags: &[(&str, u32)]| TagWarning {
            source: source.into(),
            offset,
            value: 0,
            tags: tags.iter().map(|(n, v)| ((*n).to_owned(), *v)).collect(),
        };
        let ok = [
            warn("uart4", 0, &[("MO", 1)]),
            warn("uart4", 0, &[("MO", 1)]),
            warn("uart4", 4, &[("RXINV", 1), ("TXINV", 1)]),
            warn("usart1", 0, &[("SWAP", 1)]),
        ];
        let j = judge_tags("uart4", &sbus, &ok);
        assert!(j.holds(), "{j:?}");
        assert_eq!(j.warnings, 3);
        // One missing, one at the wrong offset, one the framing does not need.
        let j = judge_tags("uart4", &sbus, &ok[..2]);
        assert_eq!(j.missing, ["CR2.RXINV", "CR2.TXINV"]);
        let j = judge_tags("uart4", &sbus, &[warn("uart4", 4, &[("MO", 1)])]);
        assert!(j.missing.contains(&"CR1.M0".to_owned()));
        assert_eq!(j.unexpected, ["MO (0x1) at 0x4"]);
        let j = judge_tags("usart1", &[], &ok);
        assert_eq!(j.unexpected, ["SWAP (0x1) at 0x0"]);
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
