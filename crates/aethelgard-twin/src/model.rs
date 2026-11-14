//! What the twin can model on an I2C bus (`AET-WO-2202` §1.2): the parts with
//! a stock Renode 1.17.0 model. `architecture.md` §2.16 read the release at
//! source: it has a `BME280` model and none for the TMP117 or the TMP102. A
//! part not listed here is *unmodelled*, never silently passed.

/// A Renode model the twin places on a bus. Each variant is matched
/// exhaustively wherever the twin depends on what the model does, so a model
/// added here cannot reach the judge without a decision on its witness
/// (`AET-WO-2204` D2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    Bme280,
}

impl Model {
    /// The model as `platforms` and `peripherals` name it in Renode 1.17.0.
    pub fn renode(self) -> &'static str {
        match self {
            Model::Bme280 => "I2C.BME280",
        }
    }
}

/// (part, model).
pub const MODELS: [(&str, Model); 1] = [("BME280", Model::Bme280)];

/// The model for `part`, or `None` when the twin cannot model it.
pub fn kind(part: &str) -> Option<Model> {
    MODELS.iter().find(|m| m.0 == part).map(|m| m.1)
}

/// The Renode model for `part`, as Renode names it, or `None` when the twin
/// cannot model it.
pub fn model(part: &str) -> Option<&'static str> {
    kind(part).map(Model::renode)
}
