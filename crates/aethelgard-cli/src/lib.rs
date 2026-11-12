//! The one composition of the generators (`AET-WO-3301` §1.5): what
//! `aethelgard generate` writes, what `aethelgard verify` regenerates and
//! gates, and what `aethelgard-svd-hal`'s regeneration test compares with
//! `boards/`. Every caller goes through [`generate::tree`], so they cannot
//! drift. The binary is `src/main.rs`.

#![forbid(unsafe_code)]

pub mod generate;
