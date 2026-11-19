//! Probe host for libngspice 44.2 in-process lock-step (AET-WO-2101 S1c/S2c, reusable for S4).
//! Copied from AET-WO-2101's spike branch; its `pattern` and `sha256` modules are dropped (unused here).
//! `ffi` is the only module with `unsafe`; `Spice<H>` is the safe(ish) wrapper.
pub mod ffi;
pub use ffi::{Hooks, Spice};
