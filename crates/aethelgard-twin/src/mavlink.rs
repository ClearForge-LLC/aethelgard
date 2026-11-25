//! MAVLink 2 as the foreign judge reads it (`AET-WO-4400i`, ruled on PR #396):
//! a declared input's frames, checked and refused by name before any run,
//! and the witness read from what the firmware itself transmits, never from
//! the input (`INV-07`).
//!
//! - **The table** ([`MESSAGES`]) is closed: a frame whose message id is not
//!   in it cannot have its CRC checked, so an input carrying one is refused
//!   (`InputMessageUnlisted`), and the witness skips it.
//! - **An input frame** is exactly one unsigned MAVLink 2 frame
//!   ([`parse_exact`]); [`check_input`] applies the refusals of ruling D8 and
//!   the timing that lets the trace hold every byte's instant exactly.
//! - **The witness** ([`witness`]) scans the firmware's transmitted bytes for
//!   frames whose CRC holds ([`scan`]): the armed heartbeat, each
//!   `COMMAND_ACK`, each parameter's `PARAM_VALUE` or `PARAM_ERROR`, and the
//!   `STATUSTEXT`s.

use crate::TwinError;
use crate::boot::QUANTUM_NS;

/// The closed message table, with its citation (`data/mavlink-messages.tsv`).
pub const MESSAGES: &str = include_str!("../data/mavlink-messages.tsv");

/// MAVLink 2's start byte.
pub const STX: u8 = 0xFD;
/// The header before the payload: start, length, the two flag bytes, the
/// sequence, the system and component ids and the 3-byte message id.
const HEADER: usize = 10;
/// The incompatibility flag of a signed frame (MAVLink 2).
const SIGNED: u8 = 0x01;

/// `MAV_CMD_COMPONENT_ARM_DISARM` (`common.xml`).
pub const ARM_DISARM: u16 = 400;
/// The two `param2` values ArduPilot arms with and skips its checks
/// (`GCS_Common.cpp`:5247): MAVLink's force code, 21196 (`common.xml`:1920,
/// `GCS.h`:779), and ArduPilot's own, 2989 (`GCS.h`:778).
pub const FORCE_CODES: [f32; 2] = [21196.0, 2989.0];
/// `MAV_MODE_FLAG_SAFETY_ARMED` (`minimal.xml`), in a heartbeat's `base_mode`.
pub const SAFETY_ARMED: u8 = 0x80;
/// The parameters whose setting skips or removes ArduPilot's arming checks
/// (ruling D8): `ARMING_SKIPCHK` to anything but 0 (`AP_Arming.cpp`:195–201,
/// default 0), and `ARMING_REQUIRE` to 0, "no requirements (arm
/// immediately)" (`AP_Arming.cpp`:119–124). Each judged by the integer
/// ArduPilot would store ([`stored`]), so 0.5 is 0.
pub const SKIP_CHECKS: [(&str, Refused); 2] = [
    ("ARMING_SKIPCHK", Refused::NonZero),
    ("ARMING_REQUIRE", Refused::Zero),
];

/// The integer ArduPilot stores when an integer parameter is set to `value`:
/// 0.01 added away from zero, then truncated; NaN and infinities are not
/// stored at all (`AP_Param::set_float`, `AP_Param.cpp`:2234–2263).
pub fn stored(value: f32) -> Option<i64> {
    if !value.is_finite() {
        return None;
    }
    let add = if value < 0.0 { -0.01f32 } else { 0.01f32 };
    Some((value + add).trunc() as i64)
}

/// Which values of a [`SKIP_CHECKS`] parameter are refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    NonZero,
    Zero,
}

/// Whether an input may carry a message, or only the firmware sends it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    In,
    Out,
}

/// One row of [`MESSAGES`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub name: &'static str,
    pub id: u32,
    pub crc_extra: u8,
    pub base_len: usize,
    pub max_len: usize,
    pub used: Use,
}

/// The rows of [`MESSAGES`]. A row that is not six tab-separated fields of
/// the stated kinds, or whose lengths are not 1 ≤ base ≤ max ≤ 255, is
/// refused by name.
pub fn messages() -> Result<Vec<Message>, TwinError> {
    messages_in(MESSAGES)
}

/// The rows of a table in [`MESSAGES`]' format.
pub(crate) fn messages_in(text: &'static str) -> Result<Vec<Message>, TwinError> {
    text.lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let bad = || TwinError::MalformedMessageTable { row: l.to_owned() };
            let f: Vec<&'static str> = l.split('\t').collect();
            let [name, id, extra, base, max, used] = f.as_slice() else {
                return Err(bad());
            };
            let num = |s: &str| s.parse::<u32>().map_err(|_| bad());
            let (id, extra, base, max) = (num(id)?, num(extra)?, num(base)?, num(max)?);
            let used = match *used {
                "in" => Use::In,
                "out" => Use::Out,
                _ => return Err(bad()),
            };
            let Ok(crc_extra) = u8::try_from(extra) else {
                return Err(bad());
            };
            if base == 0 || base > max || max > 255 {
                return Err(bad());
            }
            Ok(Message {
                name,
                id,
                crc_extra,
                base_len: base as usize,
                max_len: max as usize,
                used,
            })
        })
        .collect()
}

/// The X.25 CRC (CRC-16/MCRF4XX) MAVLink accumulates, from `crc`.
pub fn x25(bytes: &[u8], mut crc: u16) -> u16 {
    for &b in bytes {
        let mut t = b ^ (crc & 0xFF) as u8;
        t ^= t << 4;
        crc = (crc >> 8) ^ (u16::from(t) << 8) ^ (u16::from(t) << 3) ^ (u16::from(t) >> 4);
    }
    crc
}

/// One frame, its payload zero-extended to the message's full length
/// (MAVLink 2 trims trailing zero bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub seq: u8,
    pub sysid: u8,
    pub compid: u8,
    pub msgid: u32,
    pub name: &'static str,
    pub used: Use,
    pub payload: Vec<u8>,
    /// The frame's bytes on the wire: header, payload as sent, and CRC.
    pub wire: Vec<u8>,
}

/// Why bytes are not one frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Malformed {
    NoStart(u8),
    Short { have: usize, need: usize },
    Signed,
    Unlisted(u32),
    TooLong { len: usize, max: usize },
    Crc { got: u16, want: u16 },
    Trailing { extra: usize },
}

impl std::fmt::Display for Malformed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoStart(b) => write!(f, "it starts with {b:#04x}, not MAVLink 2's 0xfd"),
            Self::Short { have, need } => write!(f, "it is {have} bytes, and needs {need}"),
            Self::Signed => write!(f, "it is signed (incompatibility flag 0x01)"),
            Self::Unlisted(id) => write!(f, "its message id {id} is not in the table"),
            Self::TooLong { len, max } => {
                write!(f, "its payload is {len} bytes, over the message's {max}")
            }
            Self::Crc { got, want } => write!(f, "its CRC is {got:#06x}, not {want:#06x}"),
            Self::Trailing { extra } => write!(f, "{extra} bytes follow the frame"),
        }
    }
}

/// The frame at the start of `bytes`, and how many bytes it takes. The
/// message id is looked up before the CRC, which needs its `CRC_EXTRA`.
pub fn parse(bytes: &[u8], table: &[Message]) -> Result<(Frame, usize), Malformed> {
    let first = *bytes.first().ok_or(Malformed::Short {
        have: 0,
        need: HEADER + 2,
    })?;
    if first != STX {
        return Err(Malformed::NoStart(first));
    }
    if bytes.len() < HEADER + 2 {
        return Err(Malformed::Short {
            have: bytes.len(),
            need: HEADER + 2,
        });
    }
    let len = usize::from(bytes[1]);
    if bytes[2] & SIGNED != 0 {
        return Err(Malformed::Signed);
    }
    let msgid = u32::from(bytes[7]) | u32::from(bytes[8]) << 8 | u32::from(bytes[9]) << 16;
    let m = table
        .iter()
        .find(|m| m.id == msgid)
        .ok_or(Malformed::Unlisted(msgid))?;
    if len > m.max_len {
        return Err(Malformed::TooLong {
            len,
            max: m.max_len,
        });
    }
    let need = HEADER + len + 2;
    if bytes.len() < need {
        return Err(Malformed::Short {
            have: bytes.len(),
            need,
        });
    }
    let want = x25(&[m.crc_extra], x25(&bytes[1..HEADER + len], 0xFFFF));
    let got = u16::from(bytes[HEADER + len]) | u16::from(bytes[HEADER + len + 1]) << 8;
    if got != want {
        return Err(Malformed::Crc { got, want });
    }
    let mut payload = bytes[HEADER..HEADER + len].to_vec();
    payload.resize(m.max_len, 0);
    Ok((
        Frame {
            seq: bytes[4],
            sysid: bytes[5],
            compid: bytes[6],
            msgid,
            name: m.name,
            used: m.used,
            payload,
            wire: bytes[..need].to_vec(),
        },
        need,
    ))
}

/// `bytes` as exactly one frame, and nothing after it.
pub fn parse_exact(bytes: &[u8], table: &[Message]) -> Result<Frame, Malformed> {
    let (frame, used) = parse(bytes, table)?;
    match bytes.len() - used {
        0 => Ok(frame),
        extra => Err(Malformed::Trailing { extra }),
    }
}

/// Every frame in a transmitted stream whose CRC holds, with its offset.
/// At each start byte a frame is tried; one that parses is stepped over,
/// anything else moves on by one byte, so a start byte inside other text or
/// a frame of an unlisted message never hides a frame after it.
pub fn scan(stream: &[u8], table: &[Message]) -> Vec<(usize, Frame)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < stream.len() {
        if stream[at] == STX
            && let Ok((frame, used)) = parse(&stream[at..], table)
        {
            out.push((at, frame));
            at += used;
            continue;
        }
        at += 1;
    }
    out
}

impl Frame {
    fn u16_at(&self, o: usize) -> u16 {
        u16::from_le_bytes([self.payload[o], self.payload[o + 1]])
    }

    fn f32_at(&self, o: usize) -> f32 {
        f32::from_le_bytes([
            self.payload[o],
            self.payload[o + 1],
            self.payload[o + 2],
            self.payload[o + 3],
        ])
    }

    /// A `char[n]` field: up to its first NUL, each byte as a char.
    fn text_at(&self, o: usize, n: usize) -> String {
        self.payload[o..o + n]
            .iter()
            .take_while(|&&b| b != 0)
            .map(|&b| char::from(b))
            .collect()
    }

    /// What the judge reads of the frame, by message (offsets from the
    /// table's definitions, base fields sorted by size).
    pub fn decode(&self) -> Decoded {
        match self.name {
            "HEARTBEAT" => Decoded::Heartbeat {
                base_mode: self.payload[6],
            },
            "COMMAND_LONG" | "COMMAND_INT" => Decoded::Command {
                command: self.u16_at(28),
                param1: self.f32_at(0),
                param2: self.f32_at(4),
            },
            "PARAM_SET" => Decoded::ParamSet {
                id: self.text_at(6, 16),
                value: self.f32_at(0),
            },
            "PARAM_VALUE" => Decoded::ParamValue {
                id: self.text_at(8, 16),
                value: self.f32_at(0),
            },
            "PARAM_ERROR" => Decoded::ParamError {
                id: self.text_at(4, 16),
                error: self.payload[20],
            },
            "COMMAND_ACK" => Decoded::CommandAck {
                command: self.u16_at(0),
                result: self.payload[2],
            },
            "STATUSTEXT" => Decoded::Text {
                text: self.text_at(1, 50),
            },
            _ => Decoded::Other,
        }
    }
}

/// The fields the judge reads.
#[derive(Clone, Debug, PartialEq)]
pub enum Decoded {
    Heartbeat {
        base_mode: u8,
    },
    Command {
        command: u16,
        param1: f32,
        param2: f32,
    },
    ParamSet {
        id: String,
        value: f32,
    },
    ParamValue {
        id: String,
        value: f32,
    },
    ParamError {
        id: String,
        error: u8,
    },
    CommandAck {
        command: u16,
        result: u8,
    },
    Text {
        text: String,
    },
    Other,
}

/// One frame of a declared input: its first byte's instant, its bytes, and
/// the zero bytes of fill sent after it (ruling D3: declared, never added).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputFrame {
    pub at_ns: u64,
    pub bytes: Vec<u8>,
    pub fill: u32,
}

impl InputFrame {
    /// The bytes on the line: the frame, then its fill.
    pub fn sent(&self) -> Vec<u8> {
        let mut b = self.bytes.clone();
        b.resize(self.bytes.len() + self.fill as usize, 0);
        b
    }
}

/// Every way a declared input is refused (ruling D8, `INV-06`), each with
/// the index of the frame that fails.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum InputRefusal {
    #[error("frame {index} is not one MAVLink 2 frame: {reason}")]
    NotMavlink { index: usize, reason: Malformed },
    #[error(
        "frame {index} carries message id {msgid}, which is not in the input's table (`data/mavlink-messages.tsv`), so its CRC cannot be checked"
    )]
    MessageUnlisted { index: usize, msgid: u32 },
    #[error(
        "frame {index} carries {name}, which only the firmware sends: an input may not carry it"
    )]
    MessageNotInput { index: usize, name: &'static str },
    #[error("frame {index} is signed: an input is unsigned MAVLink 2")]
    Signed { index: usize },
    #[error(
        "frame {index} arms with force code {code} (param2), which skips the arming checks: the judge refuses a forced arm"
    )]
    ForcesArm { index: usize, code: f32 },
    #[error(
        "frame {index} sets {param} to {value}, which skips or removes the arming checks: the judge refuses it"
    )]
    SkipsChecks {
        index: usize,
        param: &'static str,
        value: f32,
    },
    #[error(
        "frame {index} starts at {at_ns} ns, which is not a whole number of the {QUANTUM_NS} ns quantum: the trace could not hold it exactly"
    )]
    TimeOffQuantum { index: usize, at_ns: u64 },
    #[error(
        "frame {index} starts at {at_ns} ns, before {free_ns} ns: the frame before it, and one idle character, end there"
    )]
    FramesOverlap {
        index: usize,
        at_ns: u64,
        free_ns: u64,
    },
    #[error("frame {index} ends at {end_ns} ns, past the run's {window_ns} ns")]
    PastWindow {
        index: usize,
        end_ns: u64,
        window_ns: u64,
    },
}

impl InputRefusal {
    /// The refusal's name, as the CLI reports it.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::NotMavlink { .. } => "InputNotMavlink",
            Self::MessageUnlisted { .. } | Self::MessageNotInput { .. } => "InputMessageUnlisted",
            Self::Signed { .. } => "InputSigned",
            Self::ForcesArm { .. } => "InputForcesArm",
            Self::SkipsChecks { .. } => "InputSkipsChecks",
            Self::TimeOffQuantum { .. } => "InputTimeOffQuantum",
            Self::FramesOverlap { .. } => "InputFramesOverlap",
            Self::PastWindow { .. } => "InputPastWindow",
        }
    }
}

/// `force`: whether `param2` is one of [`FORCE_CODES`]. ArduPilot compares
/// within `FLT_EPSILON`; within 0.5 refuses more, never less.
fn force(param2: f32) -> Option<f32> {
    FORCE_CODES.into_iter().find(|c| (param2 - c).abs() < 0.5)
}

/// A declared input's frames checked in order, each refused by name; on
/// success, each frame parsed. Each byte goes `spacing_ns` after the one
/// before it (the port's character time, a whole number of quanta); a frame
/// must start on a quantum, after the frame before it and one idle
/// character, and end inside `window_ns`.
pub fn check_input(
    frames: &[InputFrame],
    spacing_ns: u64,
    window_ns: u64,
    table: &[Message],
) -> Result<Vec<Frame>, InputRefusal> {
    let mut out = Vec::with_capacity(frames.len());
    let mut free_ns = 0u64;
    for (index, f) in frames.iter().enumerate() {
        let frame = match parse_exact(&f.bytes, table) {
            Ok(frame) => frame,
            Err(Malformed::Unlisted(msgid)) => {
                return Err(InputRefusal::MessageUnlisted { index, msgid });
            }
            Err(Malformed::Signed) => return Err(InputRefusal::Signed { index }),
            Err(reason) => return Err(InputRefusal::NotMavlink { index, reason }),
        };
        if frame.used == Use::Out {
            return Err(InputRefusal::MessageNotInput {
                index,
                name: frame.name,
            });
        }
        match frame.decode() {
            Decoded::Command {
                command: ARM_DISARM,
                param2,
                ..
            } => {
                if let Some(code) = force(param2) {
                    return Err(InputRefusal::ForcesArm { index, code });
                }
            }
            Decoded::ParamSet { id, value } => {
                for (param, refused) in SKIP_CHECKS {
                    let hit = match refused {
                        Refused::NonZero => stored(value).is_some_and(|v| v != 0),
                        Refused::Zero => stored(value) == Some(0),
                    };
                    if id == param && hit {
                        return Err(InputRefusal::SkipsChecks {
                            index,
                            param,
                            value,
                        });
                    }
                }
            }
            _ => {}
        }
        if f.at_ns % QUANTUM_NS != 0 {
            return Err(InputRefusal::TimeOffQuantum {
                index,
                at_ns: f.at_ns,
            });
        }
        if f.at_ns < free_ns {
            return Err(InputRefusal::FramesOverlap {
                index,
                at_ns: f.at_ns,
                free_ns,
            });
        }
        let n = (f.bytes.len() as u64).saturating_add(u64::from(f.fill));
        let end_ns = f.at_ns.saturating_add(n.saturating_mul(spacing_ns));
        if end_ns > window_ns {
            return Err(InputRefusal::PastWindow {
                index,
                end_ns,
                window_ns,
            });
        }
        free_ns = end_ns.saturating_add(spacing_ns);
        out.push(frame);
    }
    Ok(out)
}

/// Each byte an input sends and its instant: frame by frame, each byte
/// `spacing_ns` after the last, from the frame's `at_ns`.
pub fn schedule(frames: &[InputFrame], spacing_ns: u64) -> Vec<(u64, u8)> {
    frames
        .iter()
        .flat_map(|f| {
            f.sent()
                .into_iter()
                .enumerate()
                .map(move |(k, b)| (f.at_ns + k as u64 * spacing_ns, b))
        })
        .collect()
}

/// What the firmware's own transmitted bytes witness (`INV-07`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Witness {
    /// The first heartbeat that counts as armed ([`witness`]), as
    /// (its offset in the stream, system id, component id, `base_mode`).
    pub armed: Option<(usize, u8, u8, u8)>,
    /// Heartbeats that count, armed or not.
    pub heartbeats: u64,
    /// Each `COMMAND_ACK`, in order: (command, result).
    pub acks: Vec<(u16, u8)>,
    /// Each `PARAM_VALUE`, in order: (id, value).
    pub values: Vec<(String, f32)>,
    /// Each `PARAM_ERROR`, in order: (id, error).
    pub errors: Vec<(String, u8)>,
    /// Each `STATUSTEXT`, in order.
    pub texts: Vec<String>,
}

/// The witness in `captured`, the firmware's transmitted bytes. A frame
/// counts only if its CRC holds; a heartbeat counts only if its source (its
/// system and component ids) is none of the input's frames' sources, so a
/// heartbeat the input carried, echoed or forwarded, is never the
/// firmware's; it witnesses the arm when `base_mode` has [`SAFETY_ARMED`].
pub fn witness(captured: &[u8], input: &[Frame], table: &[Message]) -> Witness {
    let mut w = Witness::default();
    for (at, frame) in scan(captured, table) {
        let foreign_source = input
            .iter()
            .any(|i| (i.sysid, i.compid) == (frame.sysid, frame.compid));
        match frame.decode() {
            Decoded::Heartbeat { base_mode } if !foreign_source => {
                w.heartbeats += 1;
                if base_mode & SAFETY_ARMED != 0 && w.armed.is_none() {
                    w.armed = Some((at, frame.sysid, frame.compid, base_mode));
                }
            }
            Decoded::CommandAck { command, result } => w.acks.push((command, result)),
            Decoded::ParamValue { id, value } => w.values.push((id, value)),
            Decoded::ParamError { id, error } => w.errors.push((id, error)),
            Decoded::Text { text } => w.texts.push(text),
            _ => {}
        }
    }
    w
}

#[cfg(test)]
mod tests;
