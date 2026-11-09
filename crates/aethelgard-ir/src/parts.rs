//! What a part says about itself on the wire (`AET-WO-2202` §1.1): the
//! identity register a firmware can read to know the part is present. The
//! generator and the judge read it from here (`INV-01`); neither takes an
//! expected value from the other (`INV-07`). The devices themselves are in
//! `SolvedTopology::i2c_devices` (`AET-WO-2203`).

/// One identity read: `width` bytes (1 to 4) from `register`, which a present
/// part answers with `value`, first byte on the wire as the most significant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identity {
    pub register: u8,
    pub width: u8,
    pub value: u32,
    /// The datasheet section the value was read from.
    pub source: &'static str,
}

/// The identity of `part`, or `None`: a part with no entry has no identity read.
pub fn identity(part: &str) -> Option<Identity> {
    match part {
        "BME280" => Some(Identity {
            register: 0xD0,
            width: 1,
            value: 0x60,
            source: "Bosch BST-BME280-DS002, section 5.4.1",
        }),
        "TMP117" => Some(Identity {
            register: 0x0F,
            width: 2,
            value: 0x0117,
            source: "TI SNOSD82D, section 7.6.1.8",
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_parts_have_an_identity_and_the_rest_have_none() {
        assert_eq!(identity("BME280").unwrap().value, 0x60);
        assert_eq!(identity("TMP117").unwrap().width, 2);
        assert_eq!(identity("bme280"), None);
        assert_eq!(identity("W25Q128"), None);
        for part in ["BME280", "TMP117"] {
            let i = identity(part).unwrap();
            assert!((1..=4).contains(&i.width));
            assert!(i.width == 4 || i.value >> (8 * u32::from(i.width)) == 0);
        }
    }
}
