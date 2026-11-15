//! Gate clause 3 (`INV-02`): firmware that misuses the generated board crate
//! does not compile, and a valid driver does. Snapshots are regenerated with
//! `TRYBUILD=overwrite` whenever the board crate is (spike F7). The `timer_*`
//! cases and `valid_timer_driver` include the generated `motors-pwm` board by
//! path (`tests/ui/motors-pwm/board.rs`, `AET-WO-4300a`).

#[test]
fn misuse_does_not_compile_and_the_driver_does() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
    t.compile_fail("tests/ui/fail/*.rs");
}
