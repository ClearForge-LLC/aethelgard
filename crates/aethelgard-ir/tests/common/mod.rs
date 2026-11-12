//! Test-only access to a specification's device lists. They are private to
//! `aethelgard-ir` (`RUL-16`, `AET-WO-1204`), so a test edits them the way a
//! person would: through the JSON the type reads and writes.

use aethelgard_ir::{I2cDevice, Specification, SpiDevice};

/// The device lists of a specification, copied out. Mutate `i2c` and `spi`;
/// dropping the guard writes both back (through JSON) into the specification.
pub struct Devices<'a> {
    spec: &'a mut Specification,
    pub i2c: Vec<I2cDevice>,
    pub spi: Vec<SpiDevice>,
}

pub fn devs(spec: &mut Specification) -> Devices<'_> {
    Devices {
        i2c: spec.declared_i2c_devices().to_vec(),
        spi: spec.declared_spi_devices().to_vec(),
        spec,
    }
}

impl Drop for Devices<'_> {
    fn drop(&mut self) {
        let mut json = serde_json::to_value(&*self.spec).expect("a specification serializes");
        json["i2c_devices"] = serde_json::to_value(&self.i2c).expect("devices serialize");
        json["spi_devices"] = serde_json::to_value(&self.spi).expect("devices serialize");
        *self.spec = serde_json::from_value(json).expect("the same shape reads back");
    }
}
