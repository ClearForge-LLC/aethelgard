//! `AET-WO-2203` §1.4, the part the regeneration guard cannot see: the
//! committed board's devices (`U2`, `U3`) are already in reference order, so a
//! change that sorted devices by reference would pass it. This specification
//! lists `U10`, `U3`, `U2`, which sorts differently, and pins the sha256 of
//! every output that depends on device order. The digests were taken from the
//! code before the devices moved into `SolvedTopology` (`i2c_targets`).

use aethelgard_ir::Specification;
use aethelgard_svd_hal::board::generate_with;
use aethelgard_svd_hal::pin_table;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn digests() -> Vec<(&'static str, String)> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json");
    let mut doc: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let dev = |r: &str, p: &str, a: u8| serde_json::json!({"reference": r, "part": p, "supply": "VDD_3V3", "bus": "SENSORS", "address": a, "pin_capacitance": null});
    doc["i2c_devices"] = serde_json::json!([
        dev("U10", "BME280", 0x76),
        dev("U3", "TMP117", 0x48),
        dev("U2", "BME280", 0x77)
    ]);
    let bytes = serde_json::to_vec(&doc).unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let table = pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &hex(&bytes)).unwrap();
    let answers = |part: &str| matches!(aethelgard_twin::model::model(part), Ok(Some(_)));
    let files = generate_with("order-board", &solved, &table, &answers).unwrap();
    let off = BTreeSet::from(["SENSORS".to_owned()]);
    vec![
        (
            "firmware/src/main.rs",
            hex(files["firmware/src/main.rs"].as_bytes()),
        ),
        (
            "platform",
            hex(aethelgard_twin::platform::emit(&solved).unwrap().as_bytes()),
        ),
        (
            "platform faulted",
            hex(
                aethelgard_twin::platform::emit_faulted(&solved, &off, &BTreeSet::new())
                    .unwrap()
                    .as_bytes(),
            ),
        ),
        (
            "boot image",
            hex(format!("{:?}", aethelgard_twin::boot::expected(&solved).unwrap()).as_bytes()),
        ),
    ]
}

/// Taken from the code before the move (`i2c_targets` in the caller). The
/// "platform faulted" digest was re-taken in `AET-WO-1900` (2300 F13), which
/// reworded the faulted platform's provenance comment and nothing else: the old
/// digest was `35509f30…1073`. The "boot image" digest was re-taken in
/// `AET-WO-2204`, which replaced the bus's `CR2`/`TXDR` witness with each
/// identity read's device path and witness lines (the `Image` the digest
/// prints); the other three are unchanged. The old digest was `704f44d1…8e7c`.
/// The "firmware/src/main.rs" digest was re-taken in `AET-WO-4200`, whose
/// firmware hands the flash's chip select to `SPI1` and reads the flash's
/// identity after the I2C reads (record 2 here); the I2C reads' text and order
/// are unchanged. The old digest was `bd4e06af…c5f1`. The "platform", "platform
/// faulted" and "boot image" digests were re-taken in `AET-WO-4200` too: the
/// platform gains SPI1's multiplexer and the flash after the I2C devices, whose
/// lines and order are unchanged, and the `Image` gains U4's SPI check. The old
/// digests were `0b39d4aa…56d2`, `2ccc0e8d…c75b` and `06c8463c…5f5e`. The
/// firmware and both platform digests were re-taken again at the merge of
/// `AET-WO-4100`, whose specification version 2 moved them on `main` (to
/// `7079aa52…`, `b777f4e5…` and `82de0037…` there). They were re-taken once more
/// in `AET-WO-4300c` (specification version 3, so the specification's sha256;
/// the flash is `W25Q64JVxIQ`; and the firmware's SPI output speeds): the
/// firmware was `c468b3cb…604f`, the platform `9fd77dad…53aa` and the
/// faulted platform `c80833a9…8bec`. The boot image's digest is unchanged.
/// Re-taken in `AET-WO-4300f` (specification version 4, `connectors`), which
/// moved only the specification's sha256: with the old hash put back, each
/// output's digest is the old one exactly. The three were `ecd58f5f…9b7b`,
/// `bc1517cd…7522` and `f684f871…3dba`.
/// Re-taken in `AET-WO-1308` (version 5) the same way: with `main`'s hash
/// passed to `solve`, each was `f3e10417…2985`, `9c02c1e2…12bf4` and
/// `cc3a3e65…2dff9` exactly. Re-taken in `AET-WO-4300a`: specification version 6
/// (`motor_outputs`, `mcu.hse_hz`) moved the specification's sha256 in the
/// three (`44f22188…e3af`, `bd957c1d…d4b0`, `b962492f…ee15`); every enable
/// register cold and judged in full (2100 F24) moved the boot image
/// (`747bdc04…fbb3`). Re-taken in `AET-WO-4300d` (version 7, `power` and
/// `mcu.bor_level`): with `main`'s hash `ff1786a9…5756` passed to `solve`, all
/// four were `main`'s digests exactly (`dac44166…2e7e`, `693c78ad…dab0`,
/// `2fe09c95…f5b0`, the boot image unchanged). Re-taken in `AET-WO-4210` (each
/// device's script and its records; specification version 8,
/// `accept_lifecycle`; the image's SysTick and NVIC words), merging after
/// `4300d`: the four were `a5a8e53a…5e46`, `b3bf305f…a745`, `ea29d696…07f0` and
/// `893fb13d…c4a6` there. The boot image was re-taken in `AET-WO-4220`, whose
/// `StepCheck` names each step (`label`): with its three `label: "identity"`
/// fields removed, the image's text hashes to the old `0e4d72ee…c8a2` exactly.
/// Re-taken in `AET-WO-4300b`, merging after `4300d` and `4210`
/// (specification version 9, solved version 7, `serial_ports`): the
/// reference gains its console (USART1 on PA9/PA10). With the console and
/// its connector removed and `main`'s hash passed to `solve`, all four were
/// `main`'s digests exactly (`d114ae78…c455`, `849eb6bc…4397`,
/// `b24f53eb…9e35`, then `0e4d72ee…c8a2`), so the console alone moved them.
/// The boot image again at `4300b`'s merge after `4220`: it was
/// `80c36d58…ce16`, and with the `label: "identity"` fields removed it still
/// is exactly; with the console removed and `main`'s hash passed, all four are
/// `main`'s now (the boot image `a03116d6…6b64`). The boot image again with
/// `CR1.FIFOEN` (the PM's ruling on adversarial pass 2, H1): it was
/// `224dfd8a…f501`, and with `USART1.CR1.FIFOEN` dropped from its
/// unobservable list it still is exactly. Re-taken for the port's required
/// `negotiated` key (ruled on PR #152, §9 Q1): with the key dropped from the
/// hashed document, the three moved digests were `72acc1b5…bbc1`,
/// `6b61b761…b403` and `2716e538…4530` exactly (the specification's sha256
/// alone moved them); the boot image did not move. The firmware's digest was
/// re-taken in `AET-WO-4230`, whose bring-up reads the reset clock first and
/// hands it to I2C1 and SPI1, passes the registers to each port's `split`,
/// and names the SPI `EOT` timeout; it was `1564c394…3474`. The device reads'
/// text and order are unchanged, and the platform and boot image did not move.
/// Re-taken in `AET-WO-4230` slice A2a (the
/// console's `peer_deviation_ppm`, which moved the hashed document, so the
/// specification's sha256 in the firmware and the two platforms; the boot
/// image did not move; `dca32616…7ec2d`, `0a8e3b2b…9734`, `a93b7c45…1fd4`
/// before).
/// Moved once more by the merge of `AET-WO-4300d3` (specification version 10;
/// `4230` renumbered to 11), which changes no generator: the three moved with
/// the specification's sha256 alone (`4fe0b873…27c4`, `efe40970…ff4b`,
/// `f2de05af…b9ea` before); the boot image did not move.
/// Moved again by `AET-WO-4230` round 2 (`acknowledge_unwitnessed` and each
/// port's `peer_deviation_source` in the hashed document), which changes no
/// generator: the three moved with the specification's sha256 alone
/// (`2f7d00e8…df67`, `87f7aa19…ed24`, `61e7224b…bb06` before); the boot image
/// did not move. Moved once more by the reference's acknowledgements
/// (`AET-WO-4230` R1, slice R2-cli), which change no generator: the
/// specification's sha256 alone (`0ae37256…7df4`, `ee09ef30…29f1`,
/// `d0e7995a…cda3` before); the boot image did not move. Moved once more by
/// the merge of `AET-WO-4300g` (specification version 11, `mcu.supply_source`
/// and the debug and supply connectors; `4230` renumbered to 12), which changes
/// no generator of these (the blessed reference tree moved in its sha256 line
/// alone): `59c7e081…ccdb`, `a9b14d56…3246`, `892382d5…71cf` before; the boot
/// image did not move. Moved once more by `AET-WO-4211` (the flash is `nrnd`
/// and acknowledged), which changes no generator: with each text's
/// `LIFECYCLE NRND` line removed and the old specification's sha256 put back,
/// the three are `c5003f4f…620b`, `875c5440…fbe8` and `63b028f0…bbf8` exactly;
/// the boot image did not move.
/// Moved once more by `AET-WO-4220b` (specification version 13, each device's
/// `pin_capacitance`), which changes no generator of these: with the old
/// specification's sha256 (`281a522a…`) put back for the new (`12ea89f8…`),
/// the three texts are `main`'s byte for byte (`b738cc97…912f`,
/// `b27e162f…31cd`, `8e390d00…c21e` before); the boot image did not move.
/// Moved once more by `AET-WO-1312` (specification version 14), which
/// changes no generator of these: with the old specification's sha256
/// (`12ea89f8…9a2e`) put back in each text, the three are `b4c0b9a0…8ec2`,
/// `f0b5a670…d67f` and `ba58ff9e…cdad` exactly; the boot image did not move.
/// Moved once more by `AET-WO-1313` (specification version 15), which
/// changes no generator of these: with the old specification's sha256
/// (`d38a84b3…`) put back for the new (`68466678…`) in each text, the three
/// are `a60bdcac…a048`, `a81a65b3…5fdc` and `292791aa…d388` exactly; the
/// boot image did not move.
/// Moved once more by `AET-WO-4400c2` (spi1's receive DMA request, ruled on
/// PR #204): with its two edits undone (the `DMARecieve -> dmamux1@37` line
/// and the comment's clause), both platform texts are `244791ab…6b45` and
/// `6cad35bc…e52a` exactly (measured, after `1313` merged); `main.rs` and the boot image did not move.
/// Moved again by `AET-WO-4400d` (specification 16), which changes no
/// generator of these: with the version-15 specification's sha256 put back
/// for the new in each text, `main.rs` is `d7b69000…e51b` and the two
/// platforms are `4400c2`'s `b2bc0dcf…d676` and `bfa37e13…ae21` exactly
/// (measured on the merge); the boot image did not move.
/// Moved again by `AET-WO-4400g` (specification 17, and the clock's update
/// entries at the end of each platform): with the version-16
/// specification's sha256 (`e70351f0…`) put back for the new (`6deadfe4…`)
/// and the platform's clock block removed, `main.rs` and the two platforms
/// are `126afcb9…8d60`, `6f7b2db1…dd35` and `fc22c2c0…ccfa` exactly
/// (measured); the boot image did not move.
/// Moved again by `AET-WO-2304b` (specification 18, which adds no key this
/// document's devices or platform read): `main.rs` and the two platforms,
/// which carry the specification's sha256; the boot image did not move
/// (measured).
/// Moved again by `AET-WO-4240v`'s declaration (specification 18, now 19,
/// `mcu.core_supply`), which changes no generator of these: with the
/// version-17 specification's sha256 (`6deadfe4…e9c9`) put back for the new
/// (`ab8ca1d8…c110`) in each text, `main.rs` and the two platforms are
/// `5c770e1c…4722`, `88ab17b6…ce6d` and `2a8151a8…de84` exactly (measured);
/// the boot image did not move. Then `main.rs` moved again by
/// `AET-WO-4240v` §1.4, on purpose: the supply and VOS Scale 2 first, its
/// three fault words and `PWR_POLLS`, `pwr` taken from the board and
/// `reset_clock` given the token (`16a2f0ba…bae1` before); the two
/// platforms and the boot image did not move. Then each moved once more, at
/// the merge of `AET-WO-4240v`'s twin slice and its acknowledgements (§1.7),
/// measured on the minimal specification as it was before them: `main.rs` is
/// `f30204e8…de95` exactly there, so it moved by the specification's sha256
/// only; each platform, cut before its `// The PWR (AET-WO-4240v` block (the
/// `pwr` override, D-3 (a)), is `55d10bbd…1f0d` and `2a4eaea9…e4a5` exactly;
/// and the boot image is the new digest there too, so it moved only by
/// `Image::unobservable`'s `PWR.CSR1.ACTVOSRDY` and `PWR.D3CR.VOSRDY`.
/// Re-taken at the merge of `main` after `2304b` (`AET-WO-4240v`,
/// specification 19, which adds `2304b`'s keys and renumbers): with the
/// document's sha256 before the merge (`45b7fbfb…`) put back for the new
/// (`ffd83362…`) in each text, `main.rs` and the two platforms are
/// `427baa54…7b38`, `81a4ecac…fb2d` and `7b36e730…d61b` exactly, the digests
/// before the merge (measured); the boot image did not move.
/// Re-taken for specification 20 (`AET-WO-4240a`, version line only): with
/// the document's version-19 sha256 put back for its version-20 one in each
/// text, `main.rs` and the two platforms are `f443af76…2c3a`,
/// `b57442b4…e5e9` and `455de730…7a7c` exactly, the digests before
/// (measured); the boot image did not move.
/// Then moved again by `AET-WO-4241` (the architect's D3), on purpose: `main.rs`
/// gains U3's identity read (the TMP117, which the twin has no model for)
/// after every modelled read and before the boot word, and the boot image
/// gains U3's record, placed after every check's; the two platforms did not
/// move (measured).
/// Moved by `AET-WO-1210` (specification 21, renumbered after `4240a`), on
/// top of `4241`'s move: with the version-20 document's sha256
/// (`ccc80f11…e8b2`) put back for the new (`3dc38592…f2d1`) in each text,
/// `main.rs` and the two platforms are `a95a1bb6…f918`, `bb101271…ebdb` and
/// `26e266be…4fca` exactly, the digests before (measured); the boot image
/// did not move.
const BEFORE: [(&str, &str); 4] = [
    (
        "firmware/src/main.rs",
        "9e852499d8dcbe28a4a68d3a732a7f5916c3c96d2bf747cb274e479b42c88a06",
    ),
    (
        "platform",
        "10302bcebc154e72f134348be4e42cbc4d2043ef1ac67503b574c4966b92d47d",
    ),
    (
        "platform faulted",
        "380b9dcf1aa4d1725727d950830de2b4cadcceac0678a1b65a563d21501b4fca",
    ),
    (
        "boot image",
        "221f6b5975901f30fb5dd12e0ccdad3ad7fc4d8626745923b33e356fb2976d70",
    ),
];

#[test]
fn out_of_order_devices_keep_the_specification_order() {
    let now = digests();
    assert_eq!(now.len(), BEFORE.len());
    for ((name, digest), (want_name, want)) in now.iter().zip(BEFORE) {
        assert_eq!(*name, want_name);
        assert_eq!(digest, want, "{name} changed");
    }
}
