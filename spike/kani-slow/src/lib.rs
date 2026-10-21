// S4 probe: a harness that keeps CBMC busy for minutes (64-bit multiplier
// equivalence is hard for SAT), so the process tree is alive when killed.
#[cfg(kani)]
mod proofs {
    #[kani::proof]
    fn mul_commutes() {
        let a: u64 = kani::any();
        let b: u64 = kani::any();
        let c: u64 = kani::any();
        assert_eq!(a.wrapping_mul(b).wrapping_mul(c), c.wrapping_mul(b).wrapping_mul(a));
    }
}
