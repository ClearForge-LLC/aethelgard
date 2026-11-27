//! Aethelgard's own Renode 1.17.0 peripheral models, as text (`AET-WO-4220`).
//!
//! `RUL-12` as amended: a Renode model is C#, the language Renode compiles at
//! load, kept under `models/renode/` and never holding engine logic. Data only:
//! no hashes, no parsing. `aethelgard-twin` pins each file's sha256 and the CLI
//! refuses a model that is missing, unpinned or altered (ruling D6).

#![no_std]
#![forbid(unsafe_code)]

/// Every model: its file name under `models/renode/` and its text.
pub const FILES: [(&str, &str); 4] = [
    (
        "AethelgardSTM32DMA.cs",
        include_str!("../AethelgardSTM32DMA.cs"),
    ),
    (
        "AethelgardSTM32Timer.cs",
        include_str!("../AethelgardSTM32Timer.cs"),
    ),
    ("BMP581.cs", include_str!("../BMP581.cs")),
    ("ICM42688P.cs", include_str!("../ICM42688P.cs")),
];
