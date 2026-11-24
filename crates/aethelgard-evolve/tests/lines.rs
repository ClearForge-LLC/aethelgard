//! `AET-WO-3309`: the readers of a device's log lines and the placement of
//! its transactions among its steps (`aethelgard_evolve::lines`).

use aethelgard_evolve::lines::{Form, form, place, transactions};

const DEVICE: &str = "i2c1.u6";

fn icp(rest: &str) -> String {
    format!("[DEBUG] {DEVICE}: {rest}")
}

/// The ICP-101xx's lines as its model writes them (`twin` `pretty`): a
/// command, then a read of its data; each byte unpadded upper-case hex.
#[test]
fn the_icp_101xx_s_lines_are_read_exactly_as_its_model_writes_them() {
    let command = icp("Received command: [0xEF, 0xC8]");
    let read =
        icp("Reading data with CRC: [0x0, 0x8], current command: 0xEFC8 (Id), requested: 2 bytes");
    let script = [command.clone(), read.clone()];
    assert_eq!(form(&script, DEVICE), Some(Form::Icp));
    let (ts, none) = transactions(&script, DEVICE, Form::Icp);
    assert!(none.is_empty());
    assert_eq!(ts.len(), 1);
    assert_eq!(ts[0].written, [0xEF, 0xC8]);
    assert_eq!(ts[0].read.as_deref(), Some(&[0x00, 0x08][..]));
    // None of these is a line of the model's.
    for line in [
        icp("Received command: [0xef, 0xC8]"),
        icp("Received command: [0x0EF, 0xC8]"),
        icp("Received command: [0x08, 0xC8]"),
        icp("Received command: [0xEF,0xC8]"),
        icp("Received command: [EF, C8]"),
        icp("Received command: [0xEF, 0xC8] "),
        icp("Received command: []"),
        icp("Reading data with CRC: [0x0, 0x8], current command: 0xEFC8 (Id)"),
        icp("Reading data with CRC: [0x0, 0x8]"),
        icp("Reading data: [0x0, 0x8], current command: 0xEFC8 (Id), requested: 2 bytes"),
        format!("[NOISY] {DEVICE}: Received command: [0xEF, 0xC8]"),
        format!("[DEBUG] i2c1.u66: Received command: [0xEF, 0xC8]"),
    ] {
        let (ts, none) = transactions(std::slice::from_ref(&line), DEVICE, Form::Icp);
        assert!(ts.is_empty() && none == [0], "{line}");
    }
}

/// A transaction's key is its pointer and its direction; the ICP-101xx's
/// pointer is its whole two-byte command.
#[test]
fn a_transaction_is_placed_by_its_whole_key() {
    // Two ICP read steps whose commands share a first byte: each its own.
    let steps: [(&[u8], u8); 2] = [(&[0x78, 0x66], 2), (&[0x78, 0x99], 2)];
    let line = |c: &str| {
        [
            icp(&format!("Received command: [{c}]")),
            icp("Reading data with CRC: [0x1, 0x2], current command: 0x0 (X), requested: 2 bytes"),
        ]
    };
    let log = [line("0x78, 0x99"), line("0x78, 0x66")].concat();
    assert_eq!(
        place(&steps, &log, DEVICE, Form::Icp),
        [Some(1), Some(1), Some(0), Some(0)]
    );
    // The same pointer, written alone, is not the read step's: it goes to
    // the device's write step.
    let d = "i2c1.u7";
    let steps: [(&[u8], u8); 2] = [(&[0x01], 1), (&[0x36, 0x40, 0x81], 0)];
    let log = [
        format!("[NOISY] {d}: write 0x01 <- (none)"),
        format!("[NOISY] {d}: read 0x01 -> 0x50"),
    ];
    assert_eq!(place(&steps, &log, d, Form::Bmp581), [Some(1), Some(0)]);
}
