//! Clause 5 of the P0 gate (`INV-04`): a 10,000-event trace replays to a state
//! sequence whose hash equals the original's, and the bytes and the hash are
//! the same on every machine.

mod common;

use aethelgard_chaos::{Event, replay_dir};
use sha2::{Digest, Sha256};

use common::*;

/// sha256 of the golden `trace.cbor`. Changing it requires bumping the format
/// version and saying why in FEEDBACK. It was `51c6193f…a896d` (trace
/// version 1) until `AET-WO-2300` added the campaign's kinds, then
/// `e1d39213…c86e8` (version 2) until `AET-WO-3100` added kind 15, then
/// `a2d551d1…4d166` (version 3) until `AET-WO-2301` added kinds 16 to 20, then
/// `e9b23970…146ca` (version 4) until `AET-WO-2204` changed kinds 13 and 19; each
/// time only the header's version byte changed, and setting it back gives the
/// old hash again. Then `080a804e…21371` (version 5) until `AET-WO-2205` added
/// the header's tools: the version byte, the header's length and its empty
/// tools array changed, and undoing the three gives version 5's hash again.
/// `AET-WO-4200` (`dd0dd2db…c7ff`, version 7), `AET-WO-4300a` (`43cbe654…0ea3`,
/// version 8), `AET-WO-4210` (`d107fffb…3e75`, version 9), `AET-WO-4220`
/// (`8271f786…1ab4`, version 10), `AET-WO-4300b` (`1db8a16f…12d9`,
/// version 11), `AET-WO-4230` (`27753b78…c311`, version 12),
/// `AET-WO-2302` (`25392c25…8747`, version 13), `AET-WO-4400g` (`bcd082c2…8383`,
/// version 14), `AET-WO-2304b` (`10d566b5…fe1a`, version 15) and
/// `AET-WO-3305` (`7eccc893…5d31`, version 16), `AET-WO-2305` (`3ac943f6…673c`,
/// version 17), `AET-WO-1211c` (`aec9e66e…9c59`, version 18), `AET-WO-2310`
/// (`c91d20e6…5541`, version 19; the synthetic header names no tool, so the
/// tool entries' third item, a hash or nothing, is absent from it) and
/// `AET-WO-4400i` (version 20) each changed the version byte alone.
const GOLDEN_TRACE_SHA256: &str =
    "78853df76f3429ce206f268c4c117ff572127e9921a5dc29120959b737798eb6";
/// The golden `state_hash`. Same rule. It was `535db225…adac4` (manifest
/// version 1) until `AET-WO-0400a` made the chain hash each event's delta,
/// then `af2d46bb…b4939` until `AET-WO-2300` changed the header it starts
/// from, then `eb6408ee…93edc` until `AET-WO-3100` changed it again, then
/// `0db4b4f9…10cea` until `AET-WO-2301` did, then `e5dcbe7c…37925` until
/// `AET-WO-2204` did, then `28d3096c…1f82d` until `AET-WO-2205` did, then
/// `6868123f…ec11b` until `AET-WO-4200` did, then `abf1ccd2…62524` until
/// `AET-WO-4300a` did, then `35971fa6…2b99b` until `AET-WO-4210` did, then
/// `8b4dc285…def7cf` until `AET-WO-4220` did, then `955d28e0…bbdbbd` until
/// `AET-WO-4300b` did, then `d9b03ded…07124f` until `AET-WO-4230` did, then
/// `1236d811…e1dbd4` until `AET-WO-2302` did, then `64a3c723…5d03e0` until
/// `AET-WO-4400g` did, then `ada1b631…fdd8f7` until `AET-WO-2304b` did, then
/// `4b06f855…c3cbf2` until `AET-WO-3305` did, then `f4c133fa…34c693` until
/// `AET-WO-2305` did, then `707d5af0…970773` until `AET-WO-1211c` did, then
/// `fb34fc99…4226d1` until `AET-WO-2310` did, then `04b7c603…fa9b68` until
/// `AET-WO-4400i` did.
const GOLDEN_STATE_HASH: &str = "c357d1b6a4968bb0faf77e532e3d4df72ed2122072c1724a30bc6150e918ed75";

#[test]
fn golden_trace_and_state_hash_are_pinned() {
    let events = golden_events();
    let (dir, manifest) = write_run("golden", 0x0400, &events);
    let bytes = std::fs::read(dir.join("trace.cbor")).unwrap();
    let trace_sha = hex(&Sha256::digest(&bytes));
    let state_hash = hex(&manifest.state_hash);
    eprintln!(
        "GOLDEN trace_sha256={trace_sha} state_hash={state_hash} bytes={}",
        bytes.len()
    );
    assert_eq!(trace_sha, GOLDEN_TRACE_SHA256);
    assert_eq!(state_hash, GOLDEN_STATE_HASH);
    // AET-WO-4400g, AET-WO-2302, AET-WO-4230, AET-WO-4300b, AET-WO-4220, AET-WO-4210,
    // AET-WO-4300a and AET-WO-4200 each changed the version byte alone: set
    // back to 12, the bytes are version 12's golden trace (`27753b78…c311`,
    // AET-WO-4230's); set back to 11, version 11's (`1db8a16f…12d9`,
    // AET-WO-4300b's);
    // set back to 10, the bytes are version
    // 10's golden trace (`8271f786…1ab4`, AET-WO-4220's); set back to 9,
    // version 9's (`d107fffb…3e75`, AET-WO-4210's); set back to 8, version 8's
    // (`43cbe654…0ea3`, AET-WO-4300a's); set back to 7, version 7's
    // (`dd0dd2db…c7ff`, AET-WO-4200's); set back to 6, version 6's
    // (AET-WO-2205's).
    let at = 1 + 1 + "aethelgard-trace".len();
    assert_eq!(u64::from(bytes[at]), aethelgard_chaos::event::TRACE_VERSION);
    let mut bytes = bytes.clone();
    bytes[at] = 19;
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "c91d20e6ae46e219bbbc7dca4e00ad667963fb1ede93812844393c44a6b05541"
    );
    bytes[at] = 18;
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "aec9e66ede9cc6094b016690f5fa40072b9a7f140421d5a4915a182a137f9c59"
    );
    bytes[at] = 17;
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "3ac943f66440d5f3cbf4615ca1ab89e034ba4eaa605900a475074b27663c673c"
    );
    bytes[at] = 16;
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "7eccc89360bb332e5f9768c101f5702fa97cfbe566e04a394cc4246896c25d31"
    );
    bytes[at] = 14;
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "bcd082c29bd33708606ddbef7f10b66888de0b501c5c024c4030a56b0ad48383"
    );
    bytes[at] = 13;
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "25392c25bb5eff96fef99dd47dfe8260e475cd7ba9ba11ff67f86abf91888747"
    );
    bytes[at] = 12;
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "27753b78a16997117c2474bf10c0eac8561766bcf3fc239fc74fcd11635fc311"
    );
    bytes[at] = 11;
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "1db8a16f4e7ee03751e15c068dbeb5bf537f69749a81603a94def5f0485b12d9"
    );
    bytes[at] = 10;
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "8271f78613d04bc41684aad3112a5b85813a951e969bf1cec32ee3557f721ab4"
    );
    bytes[at] = 9;
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "d107fffb7dd3ec316224b798f3d877630deb57e0158482ac6655a71ee4a63e75"
    );
    bytes[at] = 8;
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "43cbe654d5d57b2fabcad9ab5c83655d65c3ebbdf6d27b3e225787dde6a50ea3"
    );
    bytes[at] = 7;
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "dd0dd2dbdf44f6a8f04ad9ad9ea80144b37ce38a12ec11bc08e471dd0b20c7ff"
    );
    bytes[at] = 6;
    assert_eq!(
        hex(&Sha256::digest(&bytes)),
        "1f9442fe2ca781f1c870093c114ba11d0ad6787daac27cd49a175a2fbfbade6b"
    );
    // AET-WO-2205 changed the header alone: a sixth item, the empty tools
    // array last, and the version. Undone, the bytes are version 5's golden
    // trace again; with the version set back to 4, version 4's.
    let (_, header_end) = aethelgard_chaos::cbor::decode_prefix(&bytes, 0).unwrap();
    assert_eq!((bytes[0], bytes[header_end - 1]), (0x86, 0x80));
    assert_eq!(bytes[at], 6);
    let mut v5 = vec![0x85];
    v5.extend(&bytes[1..header_end - 1]);
    v5.extend(&bytes[header_end..]);
    v5[at] = 5;
    assert_eq!(
        hex(&Sha256::digest(&v5)),
        "080a804ef992452a2f879b0c798603b5f1a1b40d0ba18e170b2a57975f221371"
    );
    let mut v4 = v5;
    v4[at] = 4;
    assert_eq!(
        hex(&Sha256::digest(&v4)),
        "e9b23970e9a5e874c86b6f4246482558a3b2d8844a9e9837f2dfc6ea219146ca"
    );
}

#[test]
fn the_stream_covers_every_kind_and_repeats_timestamps() {
    let events = golden_events();
    assert_eq!(events.len(), 10_000);
    let kinds = |f: fn(&Event) -> bool| events.iter().any(f);
    assert!(kinds(|e| matches!(e, Event::RegisterWrite { .. })));
    assert!(kinds(|e| matches!(e, Event::PinLevel { .. })));
    assert!(kinds(|e| matches!(e, Event::Interrupt { .. })));
    assert!(kinds(|e| matches!(e, Event::AnalogSample { .. })));
    assert!(kinds(|e| matches!(e, Event::FaultInjected { .. })));
    assert!(
        events
            .windows(2)
            .any(|w| w[0].timestamp() == w[1].timestamp())
    );
}

#[test]
fn replaying_twice_gives_the_same_state_hash() {
    let (dir, manifest) = write_run("twice", 0x0400, &golden_events());
    let first = replay_dir(&dir).unwrap();
    let second = replay_dir(&dir).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.manifest, manifest);
    assert_eq!(first.manifest.event_count, 10_000);
}

#[test]
fn two_writes_of_the_same_stream_are_byte_identical() {
    let (a, _) = write_run("same-a", 0x0400, &golden_events());
    let (b, _) = write_run("same-b", 0x0400, &golden_events());
    for name in ["trace.cbor", "manifest.cbor"] {
        assert_eq!(
            std::fs::read(a.join(name)).unwrap(),
            std::fs::read(b.join(name)).unwrap()
        );
    }
}
