//! §3.1: every refused form is refused at the right offset, and an accepted
//! byte string is always the encoding of what it decodes to.

use aethelgard_chaos::cbor::{
    CborErrorKind as K, MAX_ARRAY, MAX_STRING, Value, decode_exact, decode_prefix, to_bytes,
};
use aethelgard_chaos::synthetic::{self, SplitMix64};

fn refused(bytes: &[u8]) -> (usize, K) {
    let e = decode_exact(bytes).unwrap_err();
    (e.offset, e.kind)
}

#[test]
fn major_types_5_6_7_are_refused() {
    assert_eq!(refused(&[0xa0]), (0, K::UnsupportedMajor(5)));
    assert_eq!(refused(&[0xc0, 0x00]), (0, K::UnsupportedMajor(6)));
    assert_eq!(refused(&[0xf4]), (0, K::UnsupportedMajor(7))); // false
    assert_eq!(refused(&[0xf5]), (0, K::UnsupportedMajor(7))); // true
    assert_eq!(refused(&[0xf6]), (0, K::UnsupportedMajor(7))); // null
    assert_eq!(
        refused(&[0xfb, 0, 0, 0, 0, 0, 0, 0, 0]),
        (0, K::UnsupportedMajor(7))
    ); // float
    // Inside an array the offset is the element's.
    assert_eq!(refused(&[0x82, 0x01, 0xf5]), (2, K::UnsupportedMajor(7)));
}

#[test]
fn indefinite_lengths_and_reserved_values_are_refused() {
    assert_eq!(refused(&[0x5f]), (0, K::IndefiniteLength));
    assert_eq!(refused(&[0x7f]), (0, K::IndefiniteLength));
    assert_eq!(refused(&[0x9f, 0xff]), (0, K::IndefiniteLength));
    assert_eq!(refused(&[0x1f]), (0, K::ReservedAdditionalInfo(31)));
    for info in 28..=30u8 {
        for major in [0u8, 1, 2, 3, 4] {
            assert_eq!(
                refused(&[major << 5 | info]),
                (0, K::ReservedAdditionalInfo(info))
            );
        }
    }
    assert_eq!(refused(&[0x81, 0x1c]), (1, K::ReservedAdditionalInfo(28)));
}

#[test]
fn arguments_not_in_shortest_form_are_refused() {
    assert_eq!(refused(&[0x18, 0x00]), (0, K::NotShortest));
    assert_eq!(refused(&[0x18, 0x17]), (0, K::NotShortest));
    assert_eq!(refused(&[0x19, 0x00, 0xff]), (0, K::NotShortest));
    assert_eq!(
        refused(&[0x1a, 0x00, 0x00, 0xff, 0xff]),
        (0, K::NotShortest)
    );
    assert_eq!(
        refused(&[0x1b, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff]),
        (0, K::NotShortest)
    );
    assert_eq!(refused(&[0x38, 0x05]), (0, K::NotShortest)); // negative
    assert_eq!(refused(&[0x58, 0x01, 0x00]), (0, K::NotShortest)); // byte string length
    assert_eq!(refused(&[0x98, 0x01, 0x00]), (0, K::NotShortest)); // array length
    assert_eq!(refused(&[0x82, 0x01, 0x18, 0x01]), (2, K::NotShortest));
    // The boundaries themselves are accepted.
    for bytes in [
        &[0x18u8, 0x18][..],
        &[0x19, 0x01, 0x00],
        &[0x1a, 0, 1, 0, 0],
        &[0x1b, 0, 0, 0, 1, 0, 0, 0, 0],
    ] {
        assert!(decode_exact(bytes).is_ok());
    }
}

#[test]
fn bounds_are_checked_before_allocation() {
    // A declared length of u64::MAX, for each length-bearing type.
    let max = [0xff; 8];
    for major in [2u8, 3, 4] {
        let mut bytes = vec![major << 5 | 27];
        bytes.extend(max);
        let (offset, kind) = refused(&bytes);
        assert_eq!(offset, 0);
        assert!(matches!(
            kind,
            K::StringTooLong(u64::MAX) | K::ArrayTooLong(u64::MAX)
        ));
    }
    // One past each bound.
    let mut s = vec![0x5a];
    s.extend((MAX_STRING as u32 + 1).to_be_bytes());
    assert_eq!(refused(&s), (0, K::StringTooLong(MAX_STRING + 1)));
    let mut a = vec![0x9a];
    a.extend((MAX_ARRAY as u32 + 1).to_be_bytes());
    assert_eq!(refused(&a), (0, K::ArrayTooLong(MAX_ARRAY + 1)));
    // Exactly at the bound is within it, and then runs out of input.
    let mut s = vec![0x5a];
    s.extend((MAX_STRING as u32).to_be_bytes());
    assert_eq!(refused(&s), (0, K::UnexpectedEnd));
    let mut a = vec![0x9a];
    a.extend((MAX_ARRAY as u32).to_be_bytes());
    assert_eq!(refused(&a), (0, K::UnexpectedEnd));
    // A full-size string and a full-size array are accepted.
    let mut s = vec![0x5a];
    s.extend((MAX_STRING as u32).to_be_bytes());
    s.extend(vec![7u8; MAX_STRING as usize]);
    assert!(decode_exact(&s).is_ok());
    let mut a = vec![0x9a];
    a.extend((MAX_ARRAY as u32).to_be_bytes());
    a.extend(vec![0u8; MAX_ARRAY as usize]);
    assert!(decode_exact(&a).is_ok());
}

fn nested(depth: usize) -> Vec<u8> {
    let mut bytes = vec![0x81; depth - 1];
    bytes.push(0x80);
    bytes
}

#[test]
fn nesting_is_at_most_eight_deep() {
    assert!(decode_exact(&nested(8)).is_ok());
    // The ninth array is the one refused, at its own offset.
    assert_eq!(refused(&nested(9)), (8, K::TooDeep));
    assert_eq!(refused(&nested(40)), (8, K::TooDeep));
}

#[test]
fn invalid_utf8_and_truncation_are_refused() {
    assert_eq!(refused(&[0x62, 0xc3, 0x28]), (0, K::InvalidUtf8));
    assert_eq!(refused(&[0x61, 0xff]), (0, K::InvalidUtf8));
    assert_eq!(refused(&[0x63, 0xed, 0xa0, 0x80]), (0, K::InvalidUtf8)); // a surrogate
    assert_eq!(refused(&[]), (0, K::UnexpectedEnd));
    assert_eq!(refused(&[0x19, 0x01]), (0, K::UnexpectedEnd));
    assert_eq!(refused(&[0x43, 1, 2]), (0, K::UnexpectedEnd));
    assert_eq!(refused(&[0x83, 1, 2]), (0, K::UnexpectedEnd));
}

#[test]
fn trailing_bytes_are_refused_at_the_first_extra_byte() {
    assert_eq!(refused(&[0x01, 0x00]), (1, K::TrailingBytes));
    assert_eq!(refused(&[0x80, 0x00, 0x00]), (1, K::TrailingBytes));
    // Padding is trailing bytes too.
    assert_eq!(refused(&[0x82, 0x01, 0x02, 0x00]), (3, K::TrailingBytes));
}

fn edge_cases() -> Vec<Value> {
    let mut v = vec![
        Value::Text(String::new()),
        Value::Bytes(vec![]),
        Value::Array(vec![]),
    ];
    for n in [
        0u64,
        1,
        23,
        24,
        255,
        256,
        65_535,
        65_536,
        u64::from(u32::MAX),
        1 << 32,
        u64::MAX,
    ] {
        v.push(Value::Uint(n));
        v.push(Value::Neg(n));
    }
    for n in [0i64, -1, 23, -24, -25, i64::MAX, i64::MIN] {
        v.push(Value::from_i64(n));
    }
    v.push(Value::Text("héllo ✓".to_owned()));
    v.push(Value::Array(vec![
        Value::Array(vec![Value::Bytes(vec![1, 2, 3])]),
        Value::Text("x".into()),
    ]));
    v
}

#[test]
fn edge_values_round_trip() {
    for value in edge_cases() {
        let bytes = to_bytes(&value);
        assert_eq!(decode_exact(&bytes).unwrap(), value);
        assert_eq!(to_bytes(&decode_exact(&bytes).unwrap()), bytes);
    }
    assert_eq!(Value::from_i64(i64::MIN).as_i64(), Some(i64::MIN));
    assert_eq!(Value::from_i64(i64::MAX).as_i64(), Some(i64::MAX));
    assert_eq!(Value::Uint(1 << 63).as_i64(), None);
    assert_eq!(Value::Neg(1 << 63).as_i64(), None);
}

#[test]
fn every_generated_value_round_trips() {
    for event in synthetic::events(0x0400).take(10_000) {
        let value = event.to_value();
        let bytes = to_bytes(&value);
        assert_eq!(decode_exact(&bytes).unwrap(), value);
        assert_eq!(to_bytes(&decode_exact(&bytes).unwrap()), bytes);
    }
    let header = synthetic::header(0x0400).to_value();
    assert_eq!(
        to_bytes(&decode_exact(&to_bytes(&header)).unwrap()),
        to_bytes(&header)
    );
}

/// The defining property, tried directly on bytes the decoder is not shown in
/// advance: mutate valid encodings and splice random bytes; whatever the
/// decoder accepts must re-encode to exactly the bytes it consumed.
#[test]
fn whatever_the_decoder_accepts_re_encodes_to_the_same_bytes() {
    let mut rng = SplitMix64(0xC0FFEE);
    let seeds: Vec<Vec<u8>> = edge_cases()
        .iter()
        .chain(
            &synthetic::events(1)
                .take(50)
                .map(|e| e.to_value())
                .collect::<Vec<_>>(),
        )
        .map(to_bytes)
        .collect();
    let (mut accepted, mut refused_count) = (0u32, 0u32);
    for round in 0..200_000u32 {
        let mut bytes = seeds[(rng.next_u64() % seeds.len() as u64) as usize].clone();
        for _ in 0..(1 + rng.next_u64() % 3) {
            let at = (rng.next_u64() % (bytes.len() as u64 + 1)) as usize;
            match rng.next_u64() % 3 {
                0 if at < bytes.len() => bytes[at] = rng.next_u64() as u8,
                1 => bytes.insert(at, rng.next_u64() as u8),
                _ if at < bytes.len() => {
                    bytes.remove(at);
                }
                _ => {}
            }
        }
        if round % 4 == 0 {
            bytes = (0..rng.next_u64() % 12)
                .map(|_| rng.next_u64() as u8)
                .collect();
        }
        match decode_prefix(&bytes, 0) {
            Ok((value, end)) => {
                accepted += 1;
                assert_eq!(to_bytes(&value), &bytes[..end], "{bytes:02x?}");
            }
            Err(_) => refused_count += 1,
        }
    }
    assert!(
        accepted > 10_000 && refused_count > 10_000,
        "{accepted} {refused_count}"
    );
}
