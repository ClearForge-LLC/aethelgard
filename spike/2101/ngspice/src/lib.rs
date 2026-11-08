//! Probe host for libngspice 44.2 in-process lock-step (AET-WO-2101 S1c/S2c, reusable for S4).
//! `ffi` is the only module with `unsafe`; `Spice<H>` is the safe(ish) wrapper.
pub mod ffi;
pub mod pattern;
pub mod sha256;
pub use ffi::{Hooks, Spice};
