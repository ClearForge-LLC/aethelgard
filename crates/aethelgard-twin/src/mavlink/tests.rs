//! `AET-WO-4400i`: the input's refusals and the witness, against frames
//! another encoder wrote: pymavlink 2.4.50 in the pinned ArduPilot image
//! (`~/.cache/aet-4400i/vec/gen.py`, kept with `spike/4400i`), and Copter
//! 4.7.2's own transmitted bytes from `spike/4400i`'s `early-fill` and
//! `reject` runs (`tests/data/4400i/`).

use super::*;

const HEARTBEAT_GCS: &str = "fd09000000ffbe0000000000000006080000035c2b";
const PARAM_SET_COMPASS_USE_0: &str = "fd17000000ffbe170000000000000101434f4d504153535f555345000000000009fc22";
const ARM: &str = "fd20000000ffbe4c00000000803f000000000000000000000000000000000000000000000000900101019e4e";
const ARM_FORCE_21196: &str = "fd20000000ffbe4c00000000803f0098a546000000000000000000000000000000000000000090010101abb2";
const ARM_FORCE_2989: &str = "fd20000000ffbe4c00000000803f00d03a45000000000000000000000000000000000000000090010101b8e6";
const ARM_INT_FORCE_21196: &str = "fd20000000ffbe4b00000000803f0098a546000000000000000000000000000000000000000090010101438a";
const SKIPCHK_1: &str = "fd17000000ffbe1700000000803f010141524d494e475f534b495043484b000006f5d4";
const SKIPCHK_0: &str = "fd17000000ffbe17000000000000010141524d494e475f534b495043484b000006234b";
const REQUIRE_0: &str = "fd17000000ffbe17000000000000010141524d494e475f52455155495245000002470c";
const REQUIRE_1: &str = "fd17000000ffbe1700000000803f010141524d494e475f524551554952450000029193";
const PARAM_VALUE_FROM_GCS: &str = "fd19000000ffbe1600000000803f010000004652414d455f434c415353000000000009e896";
const SYSTEM_TIME: &str = "fd01000000ffbe020000008edf";
const ATTITUDE_TARGET: &str = "fd27000000ffbe520000000000000000803f0000000000000000000000000000000000000000000000009a99193f010107fd48";
const UNKNOWN_PARAM: &str = "fd17000000ffbe1700000000803f01014145545f4e4f545f415f504152414d000948ef";
const SIGNED_HEARTBEAT: &str = "fd09010000ffbe000000000000000608000003bbd3000100000000003bc231d90cb8";
const V1_HEARTBEAT: &str = "fe0900ffbe000000000006080000032842";

/// The early-fill run's USART1 transmission: ArduPilot arms at 8.608 s.
const EARLY_FILL_TX: &[u8] = include_bytes!("../../tests/data/4400i/early-fill-tx.bin");
/// The reject run's: `PARAM_ERROR` for `AET_NOT_A_PARAM`.
const REJECT_TX: &[u8] = include_bytes!("../../tests/data/4400i/reject-tx.bin");

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn table() -> Vec<Message> {
    messages().unwrap()
}

fn one(s: &str, at_ns: u64) -> InputFrame {
    InputFrame {
        at_ns,
        bytes: hex(s),
        fill: 0,
    }
}

/// 115200 Bd 8N1, one character rounded up to the quantum.
const SPACING: u64 = 100_000;
const WINDOW: u64 = 5_000_000_000;

fn check(frames: &[InputFrame]) -> Result<Vec<Frame>, InputRefusal> {
    check_input(frames, SPACING, WINDOW, &table())
}

#[test]
fn the_table_parses_and_each_crc_extra_is_the_one_pymavlink_computes() {
    let t = table();
    assert_eq!(t.len(), 11);
    // pymavlink 2.4.50's crc_extra, read in the pinned ArduPilot image.
    for (name, id, extra) in [
        ("HEARTBEAT", 0, 50),
        ("PARAM_REQUEST_READ", 20, 214),
        ("PARAM_REQUEST_LIST", 21, 159),
        ("PARAM_VALUE", 22, 220),
        ("PARAM_SET", 23, 168),
        ("COMMAND_INT", 75, 158),
        ("COMMAND_LONG", 76, 152),
        ("COMMAND_ACK", 77, 143),
        ("SET_ATTITUDE_TARGET", 82, 49),
        ("STATUSTEXT", 253, 83),
        ("PARAM_ERROR", 345, 209),
    ] {
        let m = t.iter().find(|m| m.name == name).unwrap();
        assert_eq!((m.id, m.crc_extra), (id, extra), "{name}");
    }
}

#[test]
fn every_input_frame_pymavlink_wrote_is_accepted() {
    for (i, s) in [
        HEARTBEAT_GCS,
        PARAM_SET_COMPASS_USE_0,
        ARM,
        SKIPCHK_0,
        REQUIRE_1,
        ATTITUDE_TARGET,
        UNKNOWN_PARAM,
    ]
    .into_iter()
    .enumerate()
    {
        let frames = [one(s, 1_000_000)];
        let got = check(&frames).unwrap_or_else(|e| panic!("vector {i}: {e}"));
        assert_eq!(got[0].wire, hex(s));
    }
    let d = parse_exact(&hex(PARAM_SET_COMPASS_USE_0), &table()).unwrap().decode();
    assert_eq!(
        d,
        Decoded::ParamSet {
            id: "COMPASS_USE".into(),
            value: 0.0
        }
    );
}

#[test]
fn a_forced_arm_is_refused_by_either_code_and_either_command() {
    for (s, code) in [
        (ARM_FORCE_21196, 21196.0),
        (ARM_FORCE_2989, 2989.0),
        (ARM_INT_FORCE_21196, 21196.0),
    ] {
        let e = check(&[one(HEARTBEAT_GCS, 0), one(s, 1_000_000)]).unwrap_err();
        assert_eq!(e, InputRefusal::ForcesArm { index: 1, code });
        assert_eq!(e.kind(), "InputForcesArm");
    }
}

#[test]
fn a_skipped_or_removed_check_is_refused_and_its_harmless_value_is_not() {
    let e = check(&[one(SKIPCHK_1, 0)]).unwrap_err();
    assert_eq!(e.kind(), "InputSkipsChecks");
    assert!(matches!(e, InputRefusal::SkipsChecks { param: "ARMING_SKIPCHK", .. }));
    let e = check(&[one(REQUIRE_0, 0)]).unwrap_err();
    assert!(matches!(e, InputRefusal::SkipsChecks { param: "ARMING_REQUIRE", .. }));
    assert!(check(&[one(SKIPCHK_0, 0), one(REQUIRE_1, 10_000_000)]).is_ok());
}

#[test]
fn bytes_that_are_not_one_mavlink_2_frame_are_refused_by_name() {
    // MAVLink 1, a corrupted CRC, a cut frame, bytes after a frame, text.
    let mut crc = hex(HEARTBEAT_GCS);
    *crc.last_mut().unwrap() ^= 1;
    let mut cut = hex(ARM);
    cut.pop();
    let mut trailing = hex(HEARTBEAT_GCS);
    trailing.push(0);
    for (bytes, want) in [
        (hex(V1_HEARTBEAT), Malformed::NoStart(0xFE)),
        (crc, Malformed::Crc { got: 0x2a5c, want: 0x2b5c }),
        (cut.clone(), Malformed::Short { have: cut.len(), need: cut.len() + 1 }),
        (trailing, Malformed::Trailing { extra: 1 }),
        (b"ARM NOW".to_vec(), Malformed::NoStart(b'A')),
    ] {
        let e = check(&[InputFrame { at_ns: 0, bytes, fill: 0 }]).unwrap_err();
        assert_eq!(e, InputRefusal::NotMavlink { index: 0, reason: want });
        assert_eq!(e.kind(), "InputNotMavlink");
    }
}

#[test]
fn an_unlisted_a_firmware_only_and_a_signed_message_are_refused() {
    let e = check(&[one(SYSTEM_TIME, 0)]).unwrap_err();
    assert_eq!(e, InputRefusal::MessageUnlisted { index: 0, msgid: 2 });
    let e = check(&[one(PARAM_VALUE_FROM_GCS, 0)]).unwrap_err();
    assert_eq!(e, InputRefusal::MessageNotInput { index: 0, name: "PARAM_VALUE" });
    assert_eq!(e.kind(), "InputMessageUnlisted");
    let e = check(&[one(SIGNED_HEARTBEAT, 0)]).unwrap_err();
    assert_eq!(e, InputRefusal::Signed { index: 0 });
}

#[test]
fn timing_the_trace_cannot_hold_exactly_is_refused() {
    // Off the quantum.
    let e = check(&[one(ARM, 1_000_050)]).unwrap_err();
    assert_eq!(e, InputRefusal::TimeOffQuantum { index: 0, at_ns: 1_000_050 });
    // A heartbeat is 21 bytes: it ends 2.1 ms on, and the line is free a
    // character later, at 2.2 ms. 2.1 ms overlaps; 2.2 ms does not.
    let e = check(&[one(HEARTBEAT_GCS, 0), one(ARM, 2_100_000)]).unwrap_err();
    assert_eq!(
        e,
        InputRefusal::FramesOverlap { index: 1, at_ns: 2_100_000, free_ns: 2_200_000 }
    );
    assert!(check(&[one(HEARTBEAT_GCS, 0), one(ARM, 2_200_000)]).is_ok());
    // Fill counts: 21 + 43 bytes end at 6.4 ms.
    let filled = InputFrame { fill: 43, ..one(HEARTBEAT_GCS, 0) };
    let e = check(&[filled.clone(), one(ARM, 6_400_000)]).unwrap_err();
    assert!(matches!(e, InputRefusal::FramesOverlap { free_ns: 6_500_000, .. }));
    // The window: a frame must end inside it.
    let late = one(HEARTBEAT_GCS, WINDOW - 2_000_000);
    let e = check(&[late]).unwrap_err();
    assert_eq!(
        e,
        InputRefusal::PastWindow { index: 0, end_ns: WINDOW + 100_000, window_ns: WINDOW }
    );
    assert!(check(&[one(HEARTBEAT_GCS, WINDOW - 2_100_000)]).is_ok());
}

#[test]
fn the_schedule_spaces_every_byte_and_its_fill() {
    let f = InputFrame { fill: 2, ..one(HEARTBEAT_GCS, 1_000_000) };
    let s = schedule(&[f], SPACING);
    assert_eq!(s.len(), 23);
    assert_eq!(s[0], (1_000_000, 0xFD));
    assert_eq!(s[22], (1_000_000 + 22 * SPACING, 0));
}

#[test]
fn the_witness_reads_the_arm_from_ardupilots_own_heartbeat() {
    let input = [parse_exact(&hex(HEARTBEAT_GCS), &table()).unwrap()];
    let w = witness(EARLY_FILL_TX, &input, &table());
    // pymavlink finds the first armed heartbeat at byte 11339 (base_mode 209).
    assert_eq!(w.armed, Some((11339, 1, 1, 209)));
    assert!(w.acks.contains(&(400, 0)), "an ARM accepted");
    assert!(w.acks.contains(&(400, 4)), "an ARM refused");
    assert!(w.texts.iter().any(|t| t == "Arm: AHRS: EKF3 still initialising"));
    assert!(w.values.iter().any(|(id, v)| id == "FRAME_CLASS" && *v == 1.0));
    assert!(w.errors.is_empty());
    // Cut before the armed heartbeat: no arm.
    assert_eq!(witness(&EARLY_FILL_TX[..11339], &input, &table()).armed, None);
}

#[test]
fn a_rejected_parameter_is_read_from_ardupilots_param_error() {
    let w = witness(REJECT_TX, &[], &table());
    assert_eq!(w.errors, vec![("AET_NOT_A_PARAM".to_owned(), 1)]);
    assert!(w.values.iter().any(|(id, v)| id == "FRAME_CLASS" && *v == 99.0));
}

#[test]
fn a_heartbeat_the_input_carried_never_witnesses_the_arm() {
    // An armed heartbeat in the input, echoed back: same bytes. And one
    // from the input's source: same ids. Neither counts.
    let armed = &EARLY_FILL_TX[11339..11339 + 21];
    let frame = parse_exact(armed, &table()).unwrap();
    assert_eq!(frame.decode(), Decoded::Heartbeat { base_mode: 209 });
    let w = witness(armed, std::slice::from_ref(&frame), &table());
    assert_eq!((w.armed, w.heartbeats), (None, 0));
    let mut other = frame.clone();
    other.wire = Vec::new();
    let w = witness(armed, &[other], &table());
    assert_eq!((w.armed, w.heartbeats), (None, 0), "same source");
    let w = witness(armed, &[], &table());
    assert_eq!(w.armed, Some((0, 1, 1, 209)));
}

#[test]
fn a_frame_hidden_by_a_start_byte_in_text_is_still_found() {
    let hb = &EARLY_FILL_TX[11339..11339 + 21];
    let mut stream = b"Init \xfd text".to_vec();
    stream.push(0xFD);
    stream.extend_from_slice(hb);
    assert_eq!(scan(&stream, &table()).len(), 1);
}

#[test]
fn a_payload_longer_than_its_message_is_refused() {
    // The heartbeat's 9 bytes declared 10, one more byte, the CRC recomputed:
    // well formed but for its length.
    let mut b = hex(HEARTBEAT_GCS);
    b.truncate(b.len() - 2);
    b[1] = 10;
    b.push(0x55);
    let crc = x25(&[50], x25(&b[1..], 0xFFFF));
    b.extend_from_slice(&crc.to_le_bytes());
    let e = check(&[InputFrame { at_ns: 0, bytes: b, fill: 0 }]).unwrap_err();
    assert_eq!(
        e,
        InputRefusal::NotMavlink { index: 0, reason: Malformed::TooLong { len: 10, max: 9 } }
    );
}
