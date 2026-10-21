// Calibration input for `aethelgard prove`: a crate that does not compile.
pub fn add(a: u8, b: u8) -> u8 {
    a + b + "not a number"
}

#[cfg(kani)]
mod proofs {
    #[kani::proof]
    fn add_in_range() {
        let a: u8 = kani::any();
        let _ = super::add(a, 1);
    }
}
