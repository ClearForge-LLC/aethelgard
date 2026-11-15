//! What the twin can model (`AET-WO-2202` §1.2, `AET-WO-4200`): the parts the
//! parts table (`aethelgard_ir::parts`) gives a Renode 1.17.0 model. A part
//! with no model is *unmodelled*, never silently passed.
//!
//! The table names the model; what the model does is this module's: the
//! whole log lines it writes for an identity read (the witness, `AET-WO-2204`
//! D2), and the `Noisy` lines it writes that are not a transfer, which the
//! judge skips and nothing else (`AET-WO-4200` D2). Both are an exhaustive
//! `match` on [`Model`], so a model added cannot reach the judge without a
//! decision on either.

use aethelgard_ir::parts::{self, BusKind};

use crate::TwinError;

/// A Renode model the twin places on a bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    /// `I2C.BME280`.
    Bme280,
    /// `SPI.GenericSpiFlash`: a JEDEC NOR flash whose identity bytes the
    /// platform sets.
    GenericSpiFlash,
    /// `Sensors.ICM20948`, on SPI.
    Icm20948,
}

impl Model {
    /// The model as `platforms` and `peripherals` name it in Renode 1.17.0.
    pub fn renode(self) -> &'static str {
        match self {
            Model::Bme280 => "I2C.BME280",
            Model::GenericSpiFlash => "SPI.GenericSpiFlash",
            Model::Icm20948 => "Sensors.ICM20948",
        }
    }

    /// The model the parts table's name stands for. Every name in
    /// [`parts::RENODE_MODELS`] has one (a tier-1 test).
    pub fn from_renode(name: &str) -> Option<Model> {
        [Model::Bme280, Model::GenericSpiFlash, Model::Icm20948]
            .into_iter()
            .find(|m| m.renode() == name)
    }

    /// The bus the twin places the model on. The ICM-20948 has an I2C side
    /// too; the twin wires only its SPI side.
    pub fn bus(self) -> BusKind {
        match self {
            Model::Bme280 => BusKind::I2c,
            Model::GenericSpiFlash | Model::Icm20948 => BusKind::Spi,
        }
    }

    /// The whole log lines (after the host timestamp) `device` writes, in
    /// order, for an identity read that writes `write` then reads `width`
    /// bytes answering `value`; `None` when the twin knows no format for that
    /// read, a named failure, never a skip (`AET-WO-2204` D2). Bytes are
    /// upper-case hexadecimal without padding, as Renode's `{0:X}` writes them.
    pub fn witness(self, device: &str, write: u8, width: u8, value: u32) -> Option<Vec<String>> {
        let bytes: Vec<u32> = (0..width)
            .rev()
            .map(|i| (value >> (8 * u32::from(i))) & 0xFF)
            .collect();
        match self {
            // `BME280.cs` L38 and L75: `Write {bytes}`, `Read {bytes}`.
            Model::Bme280 => {
                let read: Vec<String> = bytes.iter().map(|b| format!("{b:X}")).collect();
                Some(vec![
                    format!("[NOISY] {device}: Write {write:X}"),
                    format!("[NOISY] {device}: Read {}", read.join(" ")),
                ])
            }
            // `GenericSpiFlash.cs` L129 (every byte), L465 (the decoded
            // command) and L635 (every byte answered). Only Read JEDEC ID
            // (0x9F) has a known format; the firmware clocks 0x00 out while
            // it reads.
            Model::GenericSpiFlash => {
                if write != 0x9F {
                    return None;
                }
                let mut lines = vec![
                    format!(
                        "[NOISY] {device}: Transmitting data 0x9F, current state: RecognizeOperation"
                    ),
                    format!(
                        "[NOISY] {device}: Decoded operation: Operation: ReadID, write enabled False"
                    ),
                ];
                for b in &bytes {
                    lines.push(format!(
                        "[NOISY] {device}: Transmitting data 0x0, current state: HandleCommand"
                    ));
                    lines.push(format!(
                        "[NOISY] {device}: Handled command: Operation: ReadID, returning 0x{b:X}"
                    ));
                }
                Some(lines)
            }
            // `ICM20948.cs` L131 (a read's first byte), L142 (each byte read)
            // and L156 (every byte). Only `WhoAmI` (bank 0, register 0, read
            // bit set) for one byte has a known format: the register name in
            // the line moves with the register.
            Model::Icm20948 => {
                if write != 0x80 || width != 1 {
                    return None;
                }
                Some(vec![
                    format!("[NOISY] {device}: Preparing for read at bank #0 from 0x0 (WhoAmI)"),
                    format!("[NOISY] {device}: Received byte 0x80, returning 0x0"),
                    format!(
                        "[NOISY] {device}: Read at bank #0 from 0x0 (WhoAmI), returned 0x{:X}",
                        bytes[0]
                    ),
                    format!(
                        "[NOISY] {device}: Received byte 0x0, returning 0x{:X}",
                        bytes[0]
                    ),
                ])
            }
        }
    }

    /// The `Noisy` lines `device` writes that are not a transfer, exactly,
    /// which the judge skips; any other line from the device fails it
    /// (`AET-WO-4200` D2). Measured with the multiplexer wiring the emitter
    /// uses, where the flash is never sent a GPIO and so never logs `Chip
    /// Select is deasserted.` (`GenericSpiFlash.cs` L83, a direct wire only).
    pub fn skipped(self, device: &str) -> Vec<String> {
        match self {
            Model::Bme280 | Model::GenericSpiFlash => Vec::new(),
            // `ICM20948.cs` L162, on every chip-select edge the mux routes to it.
            Model::Icm20948 => vec![format!(
                "[NOISY] {device}: Finishing transmission, going idle"
            )],
        }
    }
}

/// The model for `part`, or `None` when the twin cannot model it.
pub fn kind(part: &str) -> Result<Option<Model>, TwinError> {
    let table = parts::table().map_err(TwinError::InvalidTopology)?;
    let Some(name) = table.get(part).and_then(|p| p.renode.as_deref()) else {
        return Ok(None);
    };
    Model::from_renode(name)
        .map(Some)
        .ok_or_else(|| TwinError::UnknownModel {
            part: part.to_owned(),
            model: name.to_owned(),
        })
}

/// The Renode model for `part`, as Renode names it, or `None`.
pub fn model(part: &str) -> Result<Option<&'static str>, TwinError> {
    Ok(kind(part)?.map(Model::renode))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each line the judge skips, beside the Renode source it is written by,
    /// as read at `renode-infrastructure` `066a7f13` (`AET-WO-4200` D2).
    #[test]
    fn every_skipped_line_is_its_renode_source_line() {
        let source: [(Model, &str, u32, &str); 1] = [(
            Model::Icm20948,
            "src/Emulator/Peripherals/Peripherals/Sensors/ICM20948/ICM20948.cs",
            162,
            r#"this.NoisyLog("Finishing transmission, going idle");"#,
        )];
        for m in [Model::Bme280, Model::GenericSpiFlash, Model::Icm20948] {
            let pinned: Vec<String> = source
                .iter()
                .filter(|s| s.0 == m)
                .map(|s| {
                    let literal = s.3.split('"').nth(1).unwrap();
                    format!("[NOISY] x.y: {literal}")
                })
                .collect();
            assert_eq!(m.skipped("x.y"), pinned, "{m:?}");
        }
    }

    #[test]
    fn every_model_the_table_may_name_is_one_of_ours_on_its_bus() {
        for (name, buses) in parts::RENODE_MODELS {
            let m = Model::from_renode(name).unwrap();
            assert!(buses.contains(&m.bus()), "{name}");
        }
        assert_eq!(Model::from_renode("I2C.BMP180"), None);
    }

    #[test]
    fn the_witness_formats_are_the_measured_lines() {
        let flash = Model::GenericSpiFlash
            .witness("spi1.spi1mux.u4", 0x9F, 3, 0xEF_4017)
            .unwrap();
        assert_eq!(flash.len(), 8);
        assert_eq!(
            flash[7],
            "[NOISY] spi1.spi1mux.u4: Handled command: Operation: ReadID, returning 0x17"
        );
        assert_eq!(Model::GenericSpiFlash.witness("d", 0x90, 3, 0), None);
        let imu = Model::Icm20948
            .witness("spi1.spi1mux.u5", 0x80, 1, 0xEA)
            .unwrap();
        assert_eq!(
            imu[2],
            "[NOISY] spi1.spi1mux.u5: Read at bank #0 from 0x0 (WhoAmI), returned 0xEA"
        );
        assert_eq!(Model::Icm20948.witness("d", 0x80, 2, 0), None);
        assert_eq!(Model::Icm20948.witness("d", 0x81, 1, 0), None);
    }
}
