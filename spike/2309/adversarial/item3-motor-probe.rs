/// AET-WO-2309 adversarial item 3 probe: a boot read whole as unread (kind 10's four all None),
/// its verdict naming the bound, still holding its kind 26 witnesses (read by Renode).
#[test]
fn probe2309_unread_boot_with_motor_witnesses() {
    for keep in [true, false] {
        let mut r = run("witnessed", true, false, vec![("cpu_aborted", "error", 1)]);
        for x in &mut r {
            if let Record::BootObserved { boot_word, instructions_observed, pc, brown_out, flash_access, .. } = x {
                boot_word.observed = None;
                *instructions_observed = None;
                *pc = None;
                brown_out.observed = None;
                *flash_access = None;
            }
        }
        if !keep {
            r.retain(|x| !matches!(x, Record::MotorWitness { .. }));
        }
        for x in &r {
            assert!(x.check().is_ok(), "{x:?}");
        }
        eprintln!("PROBE2309 unread boot, witnesses kept={keep}: {:?}", check_run(&r).map(|_| ()));
    }
}
