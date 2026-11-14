//! A seeded synthetic event stream, so `INV-04` can be exercised before the
//! simulators exist (`AET-WO-2100`). SplitMix64, no `rand`.

use sha2::{Digest, Sha256};

use crate::event::{Event, Header};

/// The seed the golden test pins.
pub const GOLDEN_SEED: u64 = 0x0400;
/// The golden stream's length.
pub const GOLDEN_EVENTS: usize = 10_000;

pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

const PINS: [&str; 6] = ["PA0", "PA1", "PB3", "PC13", "NRST", "I2C1_SDA"];
const NETS: [&str; 4] = ["VDD_3V3", "VBAT", "/SDA", "Net-(U1-Pad3)"];
const SCENARIOS: [&str; 3] = ["brownout", "sda-stuck-low", "clock-glitch"];
const DETAILS: [&str; 4] = [
    "",
    "duration 40 us",
    "rail sags to 2.1 V for 3 ms",
    "bit 7 flipped",
];

/// The header of the synthetic run for `seed`.
pub fn header(seed: u64) -> Header {
    let mut rng = SplitMix64(seed);
    let mut run_id = [0u8; 16];
    run_id[..8].copy_from_slice(&rng.next_u64().to_be_bytes());
    run_id[8..].copy_from_slice(&rng.next_u64().to_be_bytes());
    Header {
        run_id,
        artifacts: vec![
            (
                "firmware.elf".to_owned(),
                Sha256::digest(b"firmware").into(),
            ),
            ("spec.json".to_owned(), Sha256::digest(b"spec").into()),
        ],
        tools: Vec::new(),
    }
}

/// An endless-in-principle generator; take as many as you need.
pub struct Events {
    rng: SplitMix64,
    timestamp: u64,
}

pub fn events(seed: u64) -> Events {
    // A different stream from the header's, so the two do not correlate.
    Events {
        rng: SplitMix64(seed ^ 0xA5A5_A5A5_A5A5_A5A5),
        timestamp: 0,
    }
}

impl Iterator for Events {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        let rng = &mut self.rng;
        // 0 to 1,000 ns, so equal timestamps occur.
        self.timestamp += rng.next_u64() % 1001;
        let timestamp = self.timestamp;
        let pick = |rng: &mut SplitMix64, pool: &[&str]| {
            pool[(rng.next_u64() % pool.len() as u64) as usize].to_owned()
        };
        Some(match rng.next_u64() % 5 {
            0 => Event::RegisterWrite {
                timestamp,
                // 16 addresses, so writes overwrite one another.
                address: 0x4000_0000 + 4 * (rng.next_u64() % 16) as u32,
                value: rng.next_u64() as u32,
            },
            1 => Event::PinLevel {
                timestamp,
                pin: pick(rng, &PINS),
                level: (rng.next_u64() & 1) as u8,
            },
            2 => Event::Interrupt {
                timestamp,
                line: (rng.next_u64() % 1024) as u16,
            },
            3 => {
                let shift = rng.next_u64() % 48;
                Event::AnalogSample {
                    timestamp,
                    net: pick(rng, &NETS),
                    microvolts: (rng.next_u64().cast_signed()) >> shift,
                }
            }
            _ => Event::FaultInjected {
                timestamp,
                scenario: pick(rng, &SCENARIOS),
                detail: pick(rng, &DETAILS),
            },
        })
    }
}
