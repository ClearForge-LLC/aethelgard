//! What the twin can model (`AET-WO-2202` §1.2, `AET-WO-4200`): the parts the
//! parts table (`aethelgard_ir::parts`) gives a Renode 1.17.0 model. A part
//! with no model is *unmodelled*, never silently passed.
//!
//! The table names the model and holds each step's bytes; what the model
//! does is this module's (`AET-WO-2204` D2): the whole log lines it writes
//! for each step of a device's script (the witness, `AET-WO-4210` S2), the
//! warnings a correct step makes it write, each pinned to Renode's source
//! (D4), the `Noisy` lines it writes that are not a transfer, which the judge
//! skips and nothing else (`AET-WO-4200` D2), and the property each stimulus
//! is set through (D3). Each is an exhaustive `match` on [`Model`], so a
//! model added cannot reach the judge without a decision on each. A step the
//! twin knows no format for is `None`: a named failure, never a skip.

use aethelgard_ir::parts::{self, BusKind, Quantity, Step, StepKind};

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
    /// `Sensors.ICP_101xx`, on I2C: it logs each transaction at `Debug`
    /// only (`AET-WO-4210` D2).
    ///
    /// Model divergence (`AET-WO-4210` D-A8, ruled): the model truncates its
    /// raw temperature, `(ushort)((Temperature + 45) * (1 << 16) / 175)`
    /// (`ICP_101xx.cs` L374 at `066a7f13`), where the datasheet's conversion,
    /// and so the judge's expected value, rounds. They differ by one when the
    /// fraction is ½ or more (30 °C: 28086 against 28087), so a stimulus is
    /// chosen where both agree (25 °C: `0x6666`).
    Icp101xx,
    /// `Sensors.ICM42688P`, on SPI: this project's own (`AET-WO-4220`,
    /// `models/renode/ICM42688P.cs`).
    Icm42688p,
    /// `Sensors.BMP581`, on I2C: this project's own (`AET-WO-4220`,
    /// `models/renode/BMP581.cs`).
    Bmp581,
}

/// Every model, for the exhaustive tests.
pub const MODELS: [Model; 6] = [
    Model::Bme280,
    Model::GenericSpiFlash,
    Model::Icm20948,
    Model::Icp101xx,
    Model::Icm42688p,
    Model::Bmp581,
];

/// Each model of our own with its file in `aethelgard_renode_models::FILES`
/// and that file's sha256 (`AET-WO-4220` D6): the pin. The emitted platform
/// names it, and the CLI includes only a copy whose bytes hash to it. A
/// changed model fails `every_model_of_our_own_is_pinned_by_its_sha256`
/// until its pin moves with it.
pub const OWN: [(Model, &str, &str); 2] = [
    (
        Model::Bmp581,
        "BMP581.cs",
        "ffdcbb3169d2fa0ca1eaf1fc6de9c58cdd51fa61b525afba66e006e0e38663c4",
    ),
    (
        Model::Icm42688p,
        "ICM42688P.cs",
        "9c827af63d8bbbe57ca65d32b7a419ec0c5f521ec1ecd80be30c9aeaab3618c7",
    ),
];

/// Where a model of our own lives in the repository, as the platform's
/// header names it.
pub const OWN_DIR: &str = "models/renode";

/// The ICM-20948's bank-0 registers a script may name, as Renode's
/// `ICM20948_Bank0Registers.cs` names them: the model's lines carry the name.
const ICM_BANK0: [(u8, &str); 8] = [
    (0x00, "WhoAmI"),
    (0x06, "PowerManagement1"),
    (0x2D, "AccelerationXOutHigh"),
    (0x2E, "AccelerationXOutLow"),
    (0x2F, "AccelerationYOutHigh"),
    (0x30, "AccelerationYOutLow"),
    (0x31, "AccelerationZOutHigh"),
    (0x32, "AccelerationZOutLow"),
];

/// The warning each ICM-20948 register write the twin knows makes the model
/// write: the register framework's for a write into a tag
/// (`PeripheralRegister.cs`), measured (`AET-WO-4210` P1.3). A write not
/// listed has no known warnings, so it cannot be judged.
const ICM_WRITE_WARNINGS: [(u8, u8, &str); 1] = [(
    0x06,
    0x01,
    "Unhandled write to offset 0x6. Unhandled bits: [0] when writing value 0x1. Tags: CLKSEL (0x1).",
)];

/// The ICP-101xx commands a script may send, as Renode's `ICP_101xx.cs`
/// `Command` enum names them.
const ICP_COMMANDS: [(u16, &str); 3] = [
    (0xEFC8, "Id"),
    (0x805D, "SoftReset"),
    (0x7866, "UltraLowNoiseTemperaturePressure"),
];

/// The ICM-42688-P's `REG_BANK_SEL` (DS-000347 14.59).
const ICM_REG_BANK_SEL: u8 = 0x76;

/// Bytes as our models write them: `0x47 0x00`, or `(none)`.
fn listed(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "(none)".to_owned();
    }
    let each: Vec<String> = bytes.iter().map(|b| format!("0x{b:02X}")).collect();
    each.join(" ")
}

fn icm_register(r: u8) -> Option<&'static str> {
    ICM_BANK0.iter().find(|x| x.0 == r).map(|x| x.1)
}

/// Bytes as Renode's `Misc.PrettyPrintCollectionHex` writes them: `[0x0, 0x8]`.
fn pretty(bytes: &[u8]) -> String {
    let each: Vec<String> = bytes.iter().map(|b| format!("0x{b:X}")).collect();
    format!("[{}]", each.join(", "))
}

/// A step's expected bytes, the first on the wire first.
fn read_bytes(step: &Step) -> Vec<u8> {
    (0..u32::from(step.width))
        .rev()
        .map(|i| u8::try_from((step.expected >> (8 * i)) & 0xFF).unwrap_or(0))
        .collect()
}

impl Model {
    /// The model as `platforms` and `peripherals` name it in Renode 1.17.0.
    pub fn renode(self) -> &'static str {
        match self {
            Model::Bme280 => "I2C.BME280",
            Model::GenericSpiFlash => "SPI.GenericSpiFlash",
            Model::Icm20948 => "Sensors.ICM20948",
            Model::Icp101xx => "Sensors.ICP_101xx",
            Model::Icm42688p => "Sensors.ICM42688P",
            Model::Bmp581 => "Sensors.BMP581",
        }
    }

    /// A model of our own: its file name and pinned sha256 (`OWN`); `None`
    /// for one of Renode's.
    pub fn own(self) -> Option<(&'static str, &'static str)> {
        OWN.iter().find(|o| o.0 == self).map(|o| (o.1, o.2))
    }

    /// The text of a model of our own, from `aethelgard_renode_models`.
    pub fn source(self) -> Option<&'static str> {
        let (file, _) = self.own()?;
        aethelgard_renode_models::FILES
            .iter()
            .find(|f| f.0 == file)
            .map(|f| f.1)
    }

    /// The model the parts table's name stands for. Every name in
    /// [`parts::RENODE_MODELS`] has one (a tier-1 test).
    pub fn from_renode(name: &str) -> Option<Model> {
        MODELS.into_iter().find(|m| m.renode() == name)
    }

    /// The bus the twin places the model on. The ICM-20948 has an I2C side
    /// too; the twin wires only its SPI side.
    pub fn bus(self) -> BusKind {
        match self {
            Model::Bme280 | Model::Icp101xx | Model::Bmp581 => BusKind::I2c,
            Model::GenericSpiFlash | Model::Icm20948 | Model::Icm42688p => BusKind::Spi,
        }
    }

    /// The level the judge sets the device's log to, as Renode's `logLevel`
    /// takes it, and the prefix its witness lines carry: `Noisy` (-1), or
    /// `Debug` (0) for the ICP-101xx, whose transactions log nothing at
    /// `Noisy` but its pressure arithmetic (`AET-WO-4210` P6.2, D2).
    pub fn level(self) -> (i8, &'static str) {
        match self {
            Model::Bme280
            | Model::GenericSpiFlash
            | Model::Icm20948
            | Model::Icm42688p
            | Model::Bmp581 => (-1, "[NOISY] "),
            Model::Icp101xx => (0, "[DEBUG] "),
        }
    }

    /// The whole log lines (after the host timestamp) `device` writes, in
    /// order, for `step`, answering its expected bytes, with the warnings a
    /// correct step makes it write where it writes them; `None` when the twin
    /// knows no format for that step, a named failure, never a skip
    /// (`AET-WO-2204` D2). Bytes are upper-case hexadecimal without padding,
    /// as Renode's `{0:X}` writes them.
    pub fn witness(self, device: &str, step: &Step) -> Option<Vec<String>> {
        let bytes = read_bytes(step);
        let w = step.write.as_slice();
        match self {
            // `BME280.cs` L38 and L75: `Write {bytes}`, `Read {bytes}`. Only
            // an identity read has a known format.
            Model::Bme280 => {
                let [write] = w else { return None };
                if step.kind != StepKind::Identity {
                    return None;
                }
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
                if w != [parts::JEDEC_READ] || step.kind != StepKind::Identity {
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
            // `ICM20948.cs` `Transmit`: the first byte selects a bank-0
            // register (bits 6:0) and the direction (bit 7), and logs
            // `Preparing for read` (L131) or `Preparing for write` (L136);
            // every byte logs `Received byte` (L156); each byte read after it
            // logs `Read at` (L142) and moves to the next register. A write's
            // value is in its `Received byte` line; its own line is `Debug`
            // (L147), which the witness does not read. Only the registers in
            // `ICM_BANK0` have a known name, and only bank 0 is selected
            // from reset.
            Model::Icm20948 => {
                let (&first, values) = w.split_first()?;
                let r = first & 0x7F;
                let name = icm_register(r)?;
                if first & 0x80 != 0 {
                    if !values.is_empty() || step.width == 0 {
                        return None;
                    }
                    let mut lines = vec![
                        format!(
                            "[NOISY] {device}: Preparing for read at bank #0 from 0x{r:X} ({name})"
                        ),
                        format!("[NOISY] {device}: Received byte 0x{first:X}, returning 0x0"),
                    ];
                    for (i, b) in bytes.iter().enumerate() {
                        let at = r.checked_add(u8::try_from(i).ok()?)?;
                        let name = icm_register(at)?;
                        lines.push(format!(
                            "[NOISY] {device}: Read at bank #0 from 0x{at:X} ({name}), returned 0x{b:X}"
                        ));
                        lines.push(format!(
                            "[NOISY] {device}: Received byte 0x0, returning 0x{b:X}"
                        ));
                    }
                    Some(lines)
                } else {
                    if values.len() != 1 || step.width != 0 {
                        return None;
                    }
                    // The value is written when its byte arrives, before that
                    // byte's `Received byte` line: so is any warning the
                    // write makes (measured, `AET-WO-4210` P1.2, P1.3).
                    let mut lines = vec![
                        format!(
                            "[NOISY] {device}: Preparing for write at bank #0 to 0x{r:X} ({name})"
                        ),
                        format!("[NOISY] {device}: Received byte 0x{first:X}, returning 0x0"),
                    ];
                    lines.extend(self.warnings(device, step)?);
                    lines.push(format!(
                        "[NOISY] {device}: Received byte 0x{:X}, returning 0x0",
                        values[0]
                    ));
                    Some(lines)
                }
            }
            // `ICP_101xx.cs` `Write` (L61): one `Received command` line per
            // write of a 16-bit command; `Read` (L75): one `Reading data with
            // CRC` line per read. The model puts a CRC byte after every two
            // data bytes, so only a read of at most two bytes answers the
            // step's bytes alone.
            Model::Icp101xx => {
                let &[hi, lo] = w else { return None };
                let command = u16::from_be_bytes([hi, lo]);
                let name = ICP_COMMANDS.iter().find(|c| c.0 == command)?.1;
                if step.width > 2 {
                    return None;
                }
                let mut lines = vec![format!("[DEBUG] {device}: Received command: {}", pretty(w))];
                if step.width > 0 {
                    lines.push(format!(
                        "[DEBUG] {device}: Reading data with CRC: {}, current command: 0x{command:X} ({name}), requested: {} bytes",
                        pretty(&bytes),
                        step.width
                    ));
                }
                Some(lines)
            }
            // Ours (`AET-WO-4220`): one line per transaction, every byte in
            // it (`ICM42688P.cs` `FinishTransmission`). The bank is
            // `REG_BANK_SEL`'s, 0 from reset; a step that selects another
            // bank has no witness, since the next step's bank would depend
            // on it.
            Model::Icm42688p => {
                let (&first, values) = w.split_first()?;
                let r = first & 0x7F;
                if first & 0x80 != 0 {
                    if !values.is_empty() {
                        return None;
                    }
                    Some(vec![format!(
                        "[NOISY] {device}: bank 0 read 0x{r:02X} -> {}",
                        listed(&bytes)
                    )])
                } else {
                    if step.width != 0 || r == ICM_REG_BANK_SEL {
                        return None;
                    }
                    Some(vec![format!(
                        "[NOISY] {device}: bank 0 write 0x{r:02X} <- {}",
                        listed(values)
                    )])
                }
            }
            // `BMP581.cs` `FinishTransmission`: the first byte written is
            // the register pointer; a transaction that writes after it and
            // also reads would log two lines, which no step makes.
            Model::Bmp581 => {
                let (&r, values) = w.split_first()?;
                match (values.is_empty(), step.width) {
                    (true, 0) | (false, 0) => Some(vec![format!(
                        "[NOISY] {device}: write 0x{r:02X} <- {}",
                        listed(values)
                    )]),
                    (true, _) => Some(vec![format!(
                        "[NOISY] {device}: read 0x{r:02X} -> {}",
                        listed(&bytes)
                    )]),
                    (false, _) => None,
                }
            }
        }
    }

    /// The `[WARNING]` lines a correct `step` makes the model write, exactly
    /// and in order, each pinned to Renode's source (`AET-WO-4210` D4). The
    /// witness holds them where the model writes them, so any other warning
    /// from the device in the step, or a declared one missing, fails it.
    /// `None` when the twin does not know what the step makes the model
    /// warn, so the step has no witness: a named failure.
    fn warnings(self, device: &str, step: &Step) -> Option<Vec<String>> {
        match self {
            // Measured: none of these warns in a correct identity read, or
            // in the ICP-101xx's script (`AET-WO-4210` P6.2).
            // Ours write a warning only for a store to a read-only or
            // unmodelled register, which no correct step makes.
            Model::Bme280
            | Model::GenericSpiFlash
            | Model::Icp101xx
            | Model::Icm42688p
            | Model::Bmp581 => Some(Vec::new()),
            Model::Icm20948 => match step.write.as_slice() {
                [r, v] if r & 0x80 == 0 => ICM_WRITE_WARNINGS
                    .iter()
                    .find(|x| x.0 == *r && x.1 == *v)
                    .map(|x| vec![format!("[WARNING] {device}: {}", x.2)]),
                [r] if r & 0x80 != 0 => Some(Vec::new()),
                _ => None,
            },
        }
    }

    /// The warning the model writes when the multiplexer finishes it while
    /// it is idle, which it does on every chip-select edge of its controller
    /// (`SPIMultiplexer.cs` `UpdateChipSelectState`; `AET-WO-4210` F11): an
    /// edge artefact, allowed only on the line directly after the
    /// multiplexer's `Finisihing transmission on device` line for the
    /// device's own address (D4).
    pub fn idle_warning(self, device: &str) -> Option<String> {
        match self {
            // `GenericSpiFlash.cs` L95, measured (`AET-WO-4210` P1.6).
            Model::GenericSpiFlash => Some(format!(
                "[WARNING] {device}: Transmission finished in the unexpected state: RecognizeOperation"
            )),
            // Ours log nothing on an empty finish (`AET-WO-4220`).
            Model::Bme280
            | Model::Icm20948
            | Model::Icp101xx
            | Model::Icm42688p
            | Model::Bmp581 => None,
        }
    }

    /// The `Noisy` lines `device` writes that are not a transfer, exactly,
    /// which the judge skips; any other line from the device fails it
    /// (`AET-WO-4200` D2). Measured with the multiplexer wiring the emitter
    /// uses, where the flash is never sent a GPIO and so never logs `Chip
    /// Select is deasserted.` (`GenericSpiFlash.cs` L83, a direct wire only).
    pub fn skipped(self, device: &str) -> Vec<String> {
        match self {
            // The ICP-101xx is read at `Debug`, which does not print its
            // `Noisy` arithmetic.
            // Ours log nothing that is not a transaction, but an interrupt
            // pin's change, which no step's firmware enables (D8).
            Model::Bme280
            | Model::GenericSpiFlash
            | Model::Icp101xx
            | Model::Icm42688p
            | Model::Bmp581 => Vec::new(),
            // `ICM20948.cs` L162, on every chip-select edge the mux routes to it.
            Model::Icm20948 => vec![format!(
                "[NOISY] {device}: Finishing transmission, going idle"
            )],
        }
    }

    /// The model's property the twin sets `quantity` through, before the
    /// run (`AET-WO-4210` D3); `None` when the model has no such property.
    pub fn stimulus(self, quantity: Quantity) -> Option<&'static str> {
        match (self, quantity) {
            // `ICM20948_Accelerometer.cs`: in g.
            (Model::Icm20948, Quantity::AccelerationX) => Some("DefaultAccelerationX"),
            (Model::Icm20948, Quantity::AccelerationY) => Some("DefaultAccelerationY"),
            (Model::Icm20948, Quantity::AccelerationZ) => Some("DefaultAccelerationZ"),
            // `ICP_101xx.cs`: in degrees Celsius.
            (Model::Icp101xx, Quantity::Temperature) => Some("DefaultTemperature"),
            // Ours: in g, degrees Celsius and pascals (`AET-WO-4220`).
            (Model::Icm42688p, Quantity::AccelerationX) => Some("AccelerationX"),
            (Model::Icm42688p, Quantity::AccelerationY) => Some("AccelerationY"),
            (Model::Icm42688p, Quantity::AccelerationZ) => Some("AccelerationZ"),
            (Model::Icm42688p | Model::Bmp581, Quantity::Temperature) => Some("Temperature"),
            (Model::Bmp581, Quantity::Pressure) => Some("Pressure"),
            (Model::Bme280 | Model::GenericSpiFlash, _)
            | (Model::Icm20948, Quantity::Temperature | Quantity::Pressure)
            | (Model::Icp101xx, _)
            | (Model::Icm42688p, Quantity::Pressure)
            | (
                Model::Bmp581,
                Quantity::AccelerationX | Quantity::AccelerationY | Quantity::AccelerationZ,
            ) => None,
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
        for m in MODELS {
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

    fn step(kind: StepKind, write: &[u8], width: u8, expected: u64) -> Step {
        Step {
            kind,
            label: kind.name(),
            write: write.to_vec(),
            width,
            expected,
            mask: u64::MAX,
            settle_us: 0,
        }
    }

    /// The C# format string's literal text, with each `{n}` or `{n:X}` a
    /// wildcard: whether `line` is what the format writes.
    fn matches_format(format: &str, line: &str) -> bool {
        let mut rest = line;
        let mut parts = format.split('{').peekable();
        let first = parts.next().unwrap();
        let Some(r) = rest.strip_prefix(first) else {
            return false;
        };
        rest = r;
        while let Some(p) = parts.next() {
            let literal = p.split_once('}').map_or("", |(_, l)| l);
            if literal.is_empty() {
                if parts.peek().is_none() {
                    return !rest.is_empty();
                }
                continue;
            }
            let last = parts.peek().is_none();
            match if last {
                rest.rfind(literal)
            } else {
                rest.find(literal)
            } {
                Some(at) if at > 0 => rest = &rest[at + literal.len()..],
                _ => return false,
            }
        }
        rest.is_empty()
    }

    /// Each format the witness, the warnings and the stimulus use, beside
    /// the Renode source line that writes it, as read at
    /// `renode-infrastructure` `066a7f13` (`AET-WO-4210` S1).
    #[test]
    fn every_witness_line_is_its_renode_source_line() {
        let icm = "src/Emulator/Peripherals/Peripherals/Sensors/ICM20948/ICM20948.cs";
        let icp = "src/Emulator/Peripherals/Peripherals/Sensors/ICP_101xx.cs";
        let pinned: [(&str, u32, &str); 7] = [
            (
                icm,
                131,
                "Preparing for read at bank #{0} from 0x{1:X} ({2})",
            ),
            (
                icm,
                136,
                "Preparing for write at bank #{0} to 0x{1:X} ({2})",
            ),
            (
                icm,
                142,
                "Read at bank #{0} from 0x{1:X} ({2}), returned 0x{3:X}",
            ),
            (icm, 156, "Received byte 0x{0:X}, returning 0x{1:X}"),
            (icp, 61, "Received command: {0}"),
            (
                icp,
                75,
                "Reading data with CRC: {0}, current command: {1}, requested: {2} bytes",
            ),
            (
                "src/Emulator/Main/Core/Structure/Registers/PeripheralRegister.cs",
                755,
                "Unhandled write to offset 0x{2:X}. Unhandled bits: [{1}] when writing value 0x{3:X}.{0}",
            ),
        ];
        let imu = [
            step(StepKind::Identity, &[0x80], 1, 0xEA),
            step(StepKind::Configure, &[0x06, 0x01], 0, 0),
            step(StepKind::Data, &[0xAD], 6, 0x4000),
        ];
        let baro = [
            step(StepKind::Identity, &[0xEF, 0xC8], 2, 0x0008),
            step(StepKind::Configure, &[0x80, 0x5D], 0, 0),
            step(StepKind::Data, &[0x78, 0x66], 2, 0x6666),
        ];
        let mut lines: Vec<String> = Vec::new();
        for s in &imu {
            lines.extend(Model::Icm20948.witness("d", s).unwrap());
        }
        for s in &baro {
            lines.extend(Model::Icp101xx.witness("d", s).unwrap());
        }
        for line in &lines {
            let body = line.split_once(": ").unwrap().1;
            let by: Vec<&(&str, u32, &str)> = pinned
                .iter()
                .filter(|p| matches_format(p.2, body))
                .collect();
            assert_eq!(by.len(), 1, "`{line}` is written by {by:?}");
        }
        // Every pinned format is used.
        for p in &pinned {
            assert!(
                lines
                    .iter()
                    .any(|l| matches_format(p.2, l.split_once(": ").unwrap().1)),
                "{p:?}"
            );
        }
        for (m, name) in [
            (Model::Icm20948, "ICM20948_Bank0Registers.cs"),
            (Model::Icp101xx, "ICP_101xx.cs"),
        ] {
            assert!(!name.is_empty() && m.level().1.ends_with("] "));
        }
    }

    /// The register and command names the model's lines carry, beside their
    /// enum members in Renode's source (`066a7f13`).
    #[test]
    fn every_name_is_its_renode_enum_member() {
        let bank0: [(u32, &str); 8] = [
            (274, "WhoAmI = 0x0,"),
            (277, "PowerManagement1 = 0x6,"),
            (291, "AccelerationXOutHigh = 0x2D,"),
            (292, "AccelerationXOutLow = 0x2E,"),
            (293, "AccelerationYOutHigh = 0x2F,"),
            (294, "AccelerationYOutLow = 0x30,"),
            (295, "AccelerationZOutHigh = 0x31,"),
            (296, "AccelerationZOutLow = 0x32,"),
        ];
        for ((r, name), (_, member)) in ICM_BANK0.iter().zip(bank0) {
            assert_eq!(format!("{name} = 0x{r:X},"), member);
        }
        let commands: [(u32, &str); 3] = [
            (503, "Id = 0xEFC8,"),
            (502, "SoftReset = 0x805D,"),
            (500, "UltraLowNoiseTemperaturePressure = 0x7866,"),
        ];
        for ((c, name), (_, member)) in ICP_COMMANDS.iter().zip(commands) {
            assert_eq!(format!("{name} = 0x{c:X},"), member);
        }
    }

    /// The lines the spike measured in the pinned image (`AET-WO-4210` P1.1,
    /// P1.2, P1.4, P1.5, P6.2), and every step the twin knows no format for.
    #[test]
    fn the_witness_formats_are_the_measured_lines() {
        let flash = Model::GenericSpiFlash
            .witness(
                "spi1.spi1mux.u4",
                &step(StepKind::Identity, &[0x9F], 3, 0xEF_4017),
            )
            .unwrap();
        assert_eq!(flash.len(), 8);
        assert_eq!(
            flash[7],
            "[NOISY] spi1.spi1mux.u4: Handled command: Operation: ReadID, returning 0x17"
        );
        let d = "spi1.spi1mux.u5";
        let imu = |s: Step| Model::Icm20948.witness(d, &s).unwrap();
        assert_eq!(
            imu(step(StepKind::Identity, &[0x80], 1, 0xEA))[2],
            "[NOISY] spi1.spi1mux.u5: Read at bank #0 from 0x0 (WhoAmI), returned 0xEA"
        );
        assert_eq!(
            imu(step(StepKind::Configure, &[0x06, 0x01], 0, 0)),
            [
                "[NOISY] spi1.spi1mux.u5: Preparing for write at bank #0 to 0x6 (PowerManagement1)",
                "[NOISY] spi1.spi1mux.u5: Received byte 0x6, returning 0x0",
                "[WARNING] spi1.spi1mux.u5: Unhandled write to offset 0x6. Unhandled bits: [0] when writing value 0x1. Tags: CLKSEL (0x1).",
                "[NOISY] spi1.spi1mux.u5: Received byte 0x1, returning 0x0",
            ]
        );
        let data = imu(step(StepKind::Data, &[0xAD], 6, 0x4000));
        assert_eq!(data.len(), 14);
        assert_eq!(
            data[0],
            "[NOISY] spi1.spi1mux.u5: Preparing for read at bank #0 from 0x2D (AccelerationXOutHigh)"
        );
        assert_eq!(
            data[10],
            "[NOISY] spi1.spi1mux.u5: Read at bank #0 from 0x31 (AccelerationZOutHigh), returned 0x40"
        );
        assert_eq!(
            data[13],
            "[NOISY] spi1.spi1mux.u5: Received byte 0x0, returning 0x0"
        );
        let baro = Model::Icp101xx
            .witness(
                "i2c1.u6",
                &step(StepKind::Identity, &[0xEF, 0xC8], 2, 0x0008),
            )
            .unwrap();
        assert_eq!(
            baro,
            [
                "[DEBUG] i2c1.u6: Received command: [0xEF, 0xC8]",
                "[DEBUG] i2c1.u6: Reading data with CRC: [0x0, 0x8], current command: 0xEFC8 (Id), requested: 2 bytes",
            ]
        );
        assert_eq!(
            Model::Icp101xx
                .witness("i2c1.u6", &step(StepKind::Configure, &[0x80, 0x5D], 0, 0))
                .unwrap(),
            ["[DEBUG] i2c1.u6: Received command: [0x80, 0x5D]"]
        );
        // No known format: a named failure, never a skip.
        let none = [
            (
                Model::GenericSpiFlash,
                step(StepKind::Identity, &[0x90], 3, 0),
            ),
            (Model::Icm20948, step(StepKind::Identity, &[0x80], 2, 0)),
            (Model::Icm20948, step(StepKind::Identity, &[0x81], 1, 0)),
            (Model::Icm20948, step(StepKind::Configure, &[0x06], 0, 0)),
            (
                Model::Icm20948,
                step(StepKind::Configure, &[0x06, 0x01, 0x02], 0, 0),
            ),
            (Model::Icp101xx, step(StepKind::Data, &[0x78, 0x66], 3, 0)),
            (Model::Icp101xx, step(StepKind::Data, &[0x48, 0xA3], 2, 0)),
            (Model::Bme280, step(StepKind::Data, &[0xF7], 1, 0)),
        ];
        for (m, s) in none {
            assert_eq!(m.witness("d", &s), None, "{m:?} {s:?}");
        }
        // A write whose warnings the twin does not know has no witness.
        assert_eq!(
            Model::Icm20948.witness("d", &step(StepKind::Configure, &[0x06, 0x41], 0, 0)),
            None
        );
    }

    /// `AET-WO-4220` D6: each model of our own is pinned by the sha256 of its
    /// text as `aethelgard_renode_models` carries it, and every file there is
    /// pinned. A changed model fails here until its pin moves with it.
    #[test]
    fn every_model_of_our_own_is_pinned_by_its_sha256() {
        use sha2::{Digest, Sha256};
        for (file, text) in aethelgard_renode_models::FILES {
            let sha: String = Sha256::digest(text.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            let pinned = OWN.iter().find(|o| o.1 == file).map(|o| o.2);
            assert_eq!(pinned, Some(sha.as_str()), "{file}");
        }
        assert_eq!(OWN.len(), aethelgard_renode_models::FILES.len());
        for m in MODELS {
            assert_eq!(m.own().is_some(), m.source().is_some(), "{m:?}");
            assert_eq!(
                m.own().is_some(),
                matches!(m, Model::Icm42688p | Model::Bmp581),
                "{m:?}"
            );
        }
    }

    /// `AET-WO-4220`: each line the witness expects of a model of our own,
    /// beside the C# format string that writes it, read from the pinned
    /// source: a format changed on one side alone fails here.
    #[test]
    fn our_witness_lines_are_our_models_format_strings() {
        let icm = Model::Icm42688p.source().unwrap();
        let bmp = Model::Bmp581.source().unwrap();
        for (source, format) in [
            (icm, r#""bank {0} read 0x{1:X2} -> {2}""#),
            (icm, r#""bank {0} write 0x{1:X2} <- {2}""#),
            (bmp, r#""read 0x{0:X2} -> {1}""#),
            (bmp, r#""write 0x{0:X2} <- {1}""#),
            (bmp, r#""write 0x{0:X2} <- (none)""#),
        ] {
            assert!(source.contains(format), "{format}");
        }
        for source in [icm, bmp] {
            assert!(source.contains("(none)"));
            assert!(source.contains(r#".Append("0x").Append(b.ToString("X2"))"#));
            assert!(source.contains("namespace Antmicro.Renode.Peripherals.Sensors"));
        }
        let d = "spi1.spi1mux.u5";
        let imu = |s: Step| Model::Icm42688p.witness(d, &s).unwrap();
        assert_eq!(
            imu(step(StepKind::Identity, &[0xF5], 1, 0x47)),
            ["[NOISY] spi1.spi1mux.u5: bank 0 read 0x75 -> 0x47"]
        );
        assert_eq!(
            imu(step(StepKind::Configure, &[0x4E, 0x03], 0, 0)),
            ["[NOISY] spi1.spi1mux.u5: bank 0 write 0x4E <- 0x03"]
        );
        assert_eq!(
            imu(step(StepKind::Data, &[0x9D], 6, 0x1202_0000_0040)),
            ["[NOISY] spi1.spi1mux.u5: bank 0 read 0x1D -> 0x12 0x02 0x00 0x00 0x00 0x40"]
        );
        let baro = |s: Step| Model::Bmp581.witness("i2c1.u8", &s).unwrap();
        assert_eq!(
            baro(step(StepKind::Configure, &[0x36, 0x40, 0x81], 0, 0)),
            ["[NOISY] i2c1.u8: write 0x36 <- 0x40 0x81"]
        );
        assert_eq!(
            baro(step(StepKind::Data, &[0x1D], 6, 0x0000_1940_F362)),
            ["[NOISY] i2c1.u8: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62"]
        );
        // No witness: a bank change, a write that also reads, a read with
        // data bytes sent after its address.
        assert_eq!(
            Model::Icm42688p.witness(d, &step(StepKind::Configure, &[0x76, 0x01], 0, 0)),
            None
        );
        assert_eq!(
            Model::Bmp581.witness("i2c1.u8", &step(StepKind::Data, &[0x1D, 0x00], 1, 0)),
            None
        );
        assert_eq!(
            Model::Icm42688p.witness(d, &step(StepKind::Data, &[0x9D, 0x00], 1, 0)),
            None
        );
        for m in [Model::Icm42688p, Model::Bmp581] {
            assert_eq!(m.level(), (-1, "[NOISY] "));
            assert_eq!(m.skipped("x.y"), Vec::<String>::new());
            assert_eq!(m.idle_warning("x.y"), None);
        }
        // Every stimulus property is a public property of the model.
        for (m, q) in [
            (Model::Icm42688p, Quantity::AccelerationX),
            (Model::Icm42688p, Quantity::AccelerationY),
            (Model::Icm42688p, Quantity::AccelerationZ),
            (Model::Icm42688p, Quantity::Temperature),
            (Model::Bmp581, Quantity::Temperature),
            (Model::Bmp581, Quantity::Pressure),
        ] {
            let prop = m.stimulus(q).unwrap();
            assert!(
                m.source()
                    .unwrap()
                    .contains(&format!("public double {prop} ")),
                "{m:?} {prop}"
            );
        }
    }

    #[test]
    fn every_stimulus_is_a_property_of_its_model() {
        assert_eq!(
            Model::Icm20948.stimulus(Quantity::AccelerationZ),
            Some("DefaultAccelerationZ")
        );
        assert_eq!(
            Model::Icp101xx.stimulus(Quantity::Temperature),
            Some("DefaultTemperature")
        );
        for m in [Model::Bme280, Model::GenericSpiFlash] {
            for (q, _, _) in parts::QUANTITIES {
                assert_eq!(m.stimulus(q), None);
            }
        }
    }
}
