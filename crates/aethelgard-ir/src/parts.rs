//! What a part says about itself on the wire (`AET-WO-2202` §1.1): the
//! identity register a firmware can read to know the part is present, and the
//! I2C devices of a specification placed on the peripheral that serves their
//! bus. Both the generator and the judge read these facts from here
//! (`INV-01`); neither takes an expected value from the other (`INV-07`).

use crate::{SolvedTopology, Specification};

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

/// An I2C device of the specification, on the peripheral its bus was solved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct I2cTarget {
    pub reference: String,
    pub part: String,
    /// ST's name of the controller, such as `I2C1`.
    pub peripheral: String,
    pub address: u8,
}

/// The specification names a device on a bus the solution does not place.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TargetError {
    #[error("device `{reference}` is on bus `{bus}`, which the specification does not declare")]
    UnknownBus { reference: String, bus: String },
    #[error(
        "device `{reference}` is on bus `{bus}`, whose line `{net}` the solved topology does not place"
    )]
    UnplacedBus {
        reference: String,
        bus: String,
        net: String,
    },
}

/// Every I2C device of `spec`, in the order the specification lists them, on
/// the peripheral the solution gave its bus (the one that carries its `sda`
/// net).
pub fn i2c_targets(
    spec: &Specification,
    solved: &SolvedTopology,
) -> Result<Vec<I2cTarget>, TargetError> {
    spec.i2c_devices
        .iter()
        .map(|d| {
            let bus = spec
                .i2c_buses
                .iter()
                .find(|b| b.id == d.bus)
                .ok_or_else(|| TargetError::UnknownBus {
                    reference: d.reference.clone(),
                    bus: d.bus.clone(),
                })?;
            let placed =
                solved
                    .assignments
                    .get(&bus.sda.net)
                    .ok_or_else(|| TargetError::UnplacedBus {
                        reference: d.reference.clone(),
                        bus: bus.id.clone(),
                        net: bus.sda.net.clone(),
                    })?;
            Ok(I2cTarget {
                reference: d.reference.clone(),
                part: d.part.clone(),
                peripheral: placed.peripheral.clone(),
                address: d.address,
            })
        })
        .collect()
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
