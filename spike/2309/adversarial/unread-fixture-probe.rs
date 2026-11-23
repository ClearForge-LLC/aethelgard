#[test]
fn probe2309_unread_fixture_is_refused_by_kind_10s_own_rule_and_the_writer() {
    let unread = good_with_boot(|w, n, pc| {
        w.observed = None;
        *n = None;
        *pc = None;
    });
    let boot = unread
        .iter()
        .find(|r| matches!(r, Record::BootObserved { .. }))
        .unwrap();
    eprintln!("PROBE2309 check: {:?}", boot.check());
    let refused = append_one("probe2309-unread", boot.clone());
    eprintln!("PROBE2309 writer: {refused:?}");
    // good(): its records each pass their own field rules (archivable).
    for r in good() {
        assert!(r.check().is_ok(), "{r:?}");
    }
}
