//! Schematic and bill-of-materials emission, and the files the DRC and ERC
//! gates write for themselves (`docs/architecture.md` §3, `RUL-03`); never
//! firmware.
//!
//! - [`schematic::emit`]: `Specification` + `SolvedTopology` → `.kicad_sch`
//!   and a bill of materials, byte-identical for the same inputs (`INV-01`).
//! - [`library`]: the vendored KiCad 10.0.7 symbols and footprints, each
//!   checked against its sha256 before use.
//! - [`rules`] and [`board`]: the design rules the gate judges by, and the
//!   board under judgement stripped of every rule it carries (`INV-07`).

#![forbid(unsafe_code)]

pub mod board;
pub mod bom;
pub mod library;
pub mod rules;
pub mod schematic;
pub mod sexpr;
