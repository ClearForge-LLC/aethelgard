//! DShot frames as the foreign judge reads them (`AET-WO-4412` D4): from the
//! DMA model's line for each completed paced transfer
//! (`models/renode/AethelgardSTM32DMA.cs`), never from the firmware's
//! memory.
//!
//! A DShot frame is 16 bits, most significant first: an 11-bit throttle, one
//! telemetry-request bit, and a 4-bit checksum, `(v ^ v >> 4 ^ v >> 8) & 0xF`
//! over the 12 bits before it. A bit is one timer period whose high time is
//! 37.5 % of it for a 0 and 75 % for a 1, so a compare value over half the
//! period (`ARR + 1` ticks) is a 1. The timer's update request moves one burst
//! of `DCR.DBL + 1` words into `CCRx` per bit (RM0433 Rev 9 pp. 1586,
//! 1615–1617), so word `k` of burst `b` is burst position `k`'s compare value
//! for bit slot `b`; slots holding 0 are no pulse.

/// One completed paced transfer, as the DMA model's line states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transfer {
    pub stream: u32,
    pub request: u32,
    pub enabled_ns: u64,
    pub end_ns: u64,
    pub requests: u32,
    pub words: Vec<u32>,
    /// `Some(k)` when the stream was disabled before its end, `k` items left
    /// in NDTR (RM0433 Rev 9 §15.3.16, p. 654): its frames were cut off.
    pub disabled_ndtr: Option<u32>,
}

/// The line's body after the DMA's source (`dma2: `), or `None` for any other
/// line: `stream S req R NDTR N EN a end b reqs k: w w …`, with ` (disabled,
/// NDTR k)` before the colon for a stream disabled before its end.
pub fn transfer(body: &str) -> Option<Transfer> {
    let (head, words) = body.split_once(':')?;
    let (head, disabled_ndtr) = match head.split_once(" (disabled, NDTR ") {
        Some((h, left)) => (h, Some(left.strip_suffix(')')?.parse().ok()?)),
        None => (head, None),
    };
    let f: Vec<&str> = head.split_whitespace().collect();
    if f.len() != 12
        || f[0] != "stream"
        || f[2] != "req"
        || f[4] != "NDTR"
        || f[6] != "EN"
        || f[8] != "end"
        || f[10] != "reqs"
    {
        return None;
    }
    Some(Transfer {
        stream: f[1].parse().ok()?,
        request: f[3].parse().ok()?,
        enabled_ns: f[7].parse().ok()?,
        end_ns: f[9].parse().ok()?,
        requests: f[11].parse().ok()?,
        words: words
            .split_whitespace()
            .map(|w| u32::from_str_radix(w, 16).ok())
            .collect::<Option<Vec<_>>>()?,
        disabled_ndtr,
    })
}

/// One burst position's frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Frame {
    /// A well-formed frame: throttle, telemetry request.
    Packet { throttle: u16, telemetry: bool },
    /// Sixteen pulses whose checksum does not hold.
    BadChecksum { bits: u16 },
    /// Not sixteen pulses.
    Pulses { count: usize },
    /// The stream was disabled before its end, `ndtr_left` items unsent: a
    /// frame cut off, whatever its words decode to.
    Cut { ndtr_left: u32 },
}

/// The checksum over a frame's first 12 bits.
pub fn checksum(v: u16) -> u16 {
    (v ^ (v >> 4) ^ (v >> 8)) & 0xF
}

/// Each burst position's frame in `t`, `width` words per burst (`DBL + 1`),
/// the timer's period `period` ticks (`ARR + 1`).
pub fn frames(t: &Transfer, width: usize, period: u32) -> Vec<Frame> {
    if let Some(ndtr_left) = t.disabled_ndtr {
        return vec![Frame::Cut { ndtr_left }; width.max(1)];
    }
    (0..width)
        .map(|k| {
            let pulses: Vec<u32> = t
                .words
                .iter()
                .skip(k)
                .step_by(width.max(1))
                .copied()
                .filter(|&w| w != 0)
                .collect();
            if pulses.len() != 16 {
                return Frame::Pulses {
                    count: pulses.len(),
                };
            }
            let bits = pulses
                .iter()
                .fold(0u16, |acc, &w| (acc << 1) | u16::from(2 * w > period));
            let v = bits >> 4;
            if checksum(v) != bits & 0xF {
                return Frame::BadChecksum { bits };
            }
            Frame::Packet {
                throttle: v >> 1,
                telemetry: v & 1 == 1,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The words a frame of `throttle` and `telemetry` puts in one burst
    /// position: ArduPilot's layout on Sentinel-03 (measured, `spike/4412`):
    /// a leading empty burst, 16 bits at 3 (0) or 6 (1) of a period of 8,
    /// two trailing empty bursts.
    fn burst_words(throttle: u16, telemetry: bool) -> Vec<u32> {
        let v = (throttle << 1) | u16::from(telemetry);
        let bits = (v << 4) | checksum(v);
        let mut w = vec![0];
        w.extend(
            (0..16)
                .rev()
                .map(|i| if bits >> i & 1 == 1 { 6 } else { 3 }),
        );
        w.extend([0, 0]);
        w
    }

    fn interleave(channels: &[Vec<u32>]) -> Vec<u32> {
        (0..channels[0].len())
            .flat_map(|b| channels.iter().map(move |c| c[b]))
            .collect()
    }

    #[test]
    fn a_measured_line_parses_and_its_frames_decode() {
        let line = "stream 1 req 15 NDTR 76 EN 6180670750 end 6180697224 reqs 19: 0 0 0 0 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 3 0 0 0 0 0 0 0 0";
        let t = transfer(line).unwrap();
        assert_eq!(
            (t.stream, t.request, t.requests, t.words.len()),
            (1, 15, 19, 76)
        );
        assert_eq!(t.end_ns - t.enabled_ns, 26_474);
        assert_eq!(
            frames(&t, 4, 8),
            vec![
                Frame::Packet {
                    throttle: 0,
                    telemetry: false
                };
                4
            ]
        );
        assert_eq!(transfer("stream 1 req 15: 3"), None);
    }

    /// A stream disabled before its end (the model's ` (disabled, NDTR k)`)
    /// parses, and every position's frame is cut off, whatever its words.
    #[test]
    fn a_disabled_transfer_parses_and_is_cut_off() {
        let t = transfer(
            "stream 1 req 15 NDTR 76 EN 1 end 2 reqs 10 (disabled, NDTR 36): 0 0 0 0 3 3 3 3",
        )
        .unwrap();
        assert_eq!(
            (t.requests, t.disabled_ndtr, t.words.len()),
            (10, Some(36), 8)
        );
        assert_eq!(frames(&t, 4, 8), vec![Frame::Cut { ndtr_left: 36 }; 4]);
        assert_eq!(
            transfer("stream 1 req 15 NDTR 76 EN 1 end 2 reqs 10 (disabled, NDTR x): 3"),
            None
        );
    }

    #[test]
    fn throttle_telemetry_and_a_bad_checksum_or_pulse_count_are_told_apart() {
        let good = [
            burst_words(248, false),
            burst_words(54, true),
            burst_words(2047, false),
            burst_words(1, true),
        ];
        let mut t = Transfer {
            stream: 1,
            request: 15,
            enabled_ns: 0,
            end_ns: 1,
            requests: 19,
            words: interleave(&good),
            disabled_ndtr: None,
        };
        assert_eq!(
            frames(&t, 4, 8),
            [
                Frame::Packet {
                    throttle: 248,
                    telemetry: false
                },
                Frame::Packet {
                    throttle: 54,
                    telemetry: true
                },
                Frame::Packet {
                    throttle: 2047,
                    telemetry: false
                },
                Frame::Packet {
                    throttle: 1,
                    telemetry: true
                },
            ]
        );
        // The last bit of position 0 flipped: its checksum fails.
        let mut bad = good.clone();
        bad[0][16] = if bad[0][16] == 6 { 3 } else { 6 };
        t.words = interleave(&bad);
        assert!(matches!(frames(&t, 4, 8)[0], Frame::BadChecksum { .. }));
        // A pulse lost: 15, not 16.
        let mut short = good.clone();
        short[1][5] = 0;
        t.words = interleave(&short);
        assert_eq!(frames(&t, 4, 8)[1], Frame::Pulses { count: 15 });
    }
}
