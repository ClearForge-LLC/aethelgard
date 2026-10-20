#![forbid(unsafe_code)]

pub fn add(a: u8, b: u8) -> u8 {
    a + b
}

#[cfg(kani)]
mod proofs {
    #[kani::proof]
    fn add_in_range() {
        let a: u8 = kani::any();
        let b: u8 = kani::any();
        kani::assume(a <= 100 && b <= 100);
        assert_eq!(u16::from(super::add(a, b)), u16::from(a) + u16::from(b));
    }

    #[kani::proof]
    fn add_overflows() {
        let a: u8 = kani::any();
        let b: u8 = kani::any();
        super::add(a, b);
    }
}
