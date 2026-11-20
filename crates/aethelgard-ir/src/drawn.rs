//! What a drawn board needs that `validate` can hold before it is drawn
//! (`AET-WO-1210`; `AET-WO-1209` §1.1 and §1.3, rules 1 and 3). A
//! specification that declares `board.drawn: false`, a twin stand-in, is held
//! to neither: no board is drawn from it, so `emit`, `extract` and `verify`
//! refuse it by name and `generate` writes no `kicad/` for it.

use crate::{IrValidationError as E, Specification, parts};

/// Step 23, rule 1: `extract`'s own precondition on a device's pin
/// (`aethelgard_eda::extract::lines` and `device_pin`), read at source:
///
/// - only SPI lines refuse; on an I2C line a pin counts at UM10204's `Ci`;
/// - each SPI bus in document order, its devices in document order, each on
///   the bus's SCK (then MOSI, which the same devices share), so the first
///   device refused is `extract`'s first;
/// - a device counts at its part's cited maximum, else at its declared
///   `pin_capacitance`, else it is refused;
/// - a bus a `power.crossings` entry carries is skipped: its device is drawn
///   on the translator's B side, never on the bus's own lines, and `extract`
///   refuses the translator there (`TranslatorPinUnbounded`).
pub(crate) fn pin_capacitance(spec: &Specification, table: &parts::PartsTable) -> Result<(), E> {
    if !spec.board.drawn {
        return Ok(());
    }
    let crossed = |bus: &str| {
        spec.power
            .iter()
            .flat_map(|p| &p.crossings)
            .any(|c| c.bus == bus)
    };
    for bus in spec.spi_buses.iter().filter(|b| !crossed(&b.id)) {
        for (i, d) in spec.spi_devices.iter().enumerate() {
            let unbounded = table
                .get(&d.part)
                .is_some_and(|p| p.pin_capacitance_femtofarads.is_none());
            if d.bus == bus.id && unbounded && d.pin_capacitance.is_none() {
                return Err(E::UnboundedPinCapacitance {
                    path: format!("spi_devices[{i}].pin_capacitance"),
                    reference: d.reference.clone(),
                    part: d.part.clone(),
                    line: format!("`{}` of SPI bus `{}`", bus.sck.net, bus.id),
                });
            }
        }
    }
    Ok(())
}

impl Specification {
    /// Rule 3, for the CLI's `validate`: on a drawn board, every device's
    /// part is one the emitter has an arm for. `arms` is the emitter's own
    /// set (`aethelgard_eda::schematic::KNOWN_PARTS`, which `AET-WO-1316`
    /// holds to the arms by test); this crate cannot read it, since the
    /// emitter depends on this crate. Devices in the emitter's draw order:
    /// I2C, then SPI, then the pin devices, each in document order.
    pub fn drawable_parts(&self, arms: &[&str]) -> Result<(), E> {
        if !self.board.drawn {
            return Ok(());
        }
        let i2c = self.i2c_devices.iter().enumerate();
        let devices = i2c
            .map(|(i, d)| (format!("i2c_devices[{i}].part"), &d.reference, &d.part))
            .chain(
                self.spi_devices
                    .iter()
                    .enumerate()
                    .map(|(i, d)| (format!("spi_devices[{i}].part"), &d.reference, &d.part)),
            )
            // A pin device (`AET-WO-1211a`): no emitter arm draws one until
            // `AET-WO-1211d`, so it is declared on a board that draws none.
            .chain(
                self.declared_pin_devices()
                    .iter()
                    .enumerate()
                    .map(|(i, d)| (format!("pin_devices[{i}].part"), &d.reference, &d.part)),
            );
        for (path, reference, part) in devices {
            if !arms.contains(&part.as_str()) {
                return Err(E::PartNotDrawable {
                    path,
                    reference: reference.clone(),
                    part: part.clone(),
                    arms: arms.join(", "),
                });
            }
        }
        Ok(())
    }
}
