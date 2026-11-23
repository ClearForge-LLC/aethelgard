/// AET-WO-2309 adversarial item 2 probes.
#[test]
fn probe2309_item2() {
    let base = run("witnessed", true, true, vec![]);
    // M2's witness replaced by a second of M1's.
    let mut r = base.clone();
    for x in &mut r {
        if matches!(x, Record::MotorWitness { label, .. } if label == "M2") {
            *x = witness("M1", "gpioPortA.pa8obs", "witnessed");
        }
    }
    eprintln!("PROBE2309 duplicate for dropped: {:?}", check_run(&r).map(|_| ()));
    // Both witnesses dropped, the boot word read but wrong, its finding named: the work order's scope.
    let mut r = run("witnessed", true, false, vec![("boot_word_missing", "error", 1)]);
    r.retain(|x| !matches!(x, Record::MotorWitness { .. }));
    eprintln!("PROBE2309 read, unbooted, no witness: {:?}", check_run(&r).map(|_| ()));
    let good = dshot_run(Some(DSHOT_LEAD), Some((DSHOT_LEAD + 2_000_000, 0)), vec![]);
    eprintln!("PROBE2309 dshot base: {:?}", check_run(&good).map(|_| ()));
    let mut r = good.clone();
    r.retain(|x| !matches!(x, Record::MotorWitness { .. } | Record::JudgeStop { .. }));
    eprintln!("PROBE2309 dshot, witnesses and stop dropped: {:?}", check_run(&r).map(|_| ()));
}
