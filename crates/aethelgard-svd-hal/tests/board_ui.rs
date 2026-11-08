//! Gate clause 3 (`INV-02`): firmware that misuses the generated board crate
//! does not compile, and a valid driver does. Snapshots are regenerated with
//! `TRYBUILD=overwrite` whenever the board crate is (spike F7).

#[test]
fn misuse_does_not_compile_and_the_driver_does() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
    t.compile_fail("tests/ui/fail/*.rs");
}
