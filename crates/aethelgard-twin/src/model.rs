//! What the twin can model on an I2C bus (`AET-WO-2202` §1.2): the parts with
//! a stock Renode 1.17.0 model. `architecture.md` §2.16 read the release at
//! source: it has a `BME280` model and none for the TMP117 or the TMP102. A
//! part not listed here is *unmodelled*, never silently passed.

/// (part, Renode model), the model as `platforms` and `peripherals` name it
/// in Renode 1.17.0.
pub const MODELS: [(&str, &str); 1] = [("BME280", "I2C.BME280")];

/// The Renode model for `part`, or `None` when the twin cannot model it.
pub fn model(part: &str) -> Option<&'static str> {
    MODELS.iter().find(|m| m.0 == part).map(|m| m.1)
}

/// `I2C.BME280` is listed as `BME280`.
pub fn listed(model: &str) -> &str {
    model.rsplit('.').next().unwrap_or(model)
}

/// The devices of `devices` the twin can answer for: the ones whose part has
/// a model. The generated firmware reads exactly these (`AET-WO-2202`), and
/// the platform declares exactly these; the rest are listed as unmodelled.
pub fn answerable(
    devices: &[aethelgard_ir::parts::I2cTarget],
) -> Vec<aethelgard_ir::parts::I2cTarget> {
    devices
        .iter()
        .filter(|d| model(&d.part).is_some())
        .cloned()
        .collect()
}
