# AET-WO-1211c adversarial pass (fresh Opus reviewer, against e0c7fae3), as handed back
Saved by the builder: the reviewer's Write was refused for subagents, so the handback text is the report.
Probes: ~/.cache/aet-1211c/adv/probes.patch (adv_* tests); Renode kit and outputs: ~/.cache/aet-1211c/adv/rn/.
- F1 should-fix: replay never joins kind 26 `level` / kind 37 to kind 24's idle, kind 26's anchor or edges; forged level PASS (adv_r1-r3, archived). base() fixture idles at a pulse.
- F2 should-fix: output witnessed while firmware sets the other level before MODER (second BSRR word; ODR write) - Renode mutants twoset, odr.
- F3 should-fix: ADC witness at idle 0 V judges nothing (DR reset 0); noconv mutant passes; off-idle discriminates.
- F4 should-fix/decision: gpio gate counts an input item as judged on the twin's own stimulus (D5's reason).
- F5 note: replay checks no pin across kinds (33/35 vs 24/27/CS).
- F6 note: replay accepts a mismatch whose reading equals the declaration.
- F7 note: plan takes direction from Declared, not SolvedGpio; bits fallback unwrap_or.
- F8 should-fix: null sweep stale at adeb3acd and never runs verify (reviewer found no move at library level, adv_n, 27 specs).
- F9 note: idle-high settled window can be shorter than one period.
- F10 note: classify calls a refused hold Baseline.
- F11 note: no boot -> gpio/adc gates vanish (fails closed via simulate); nSLEEP counted in gpio and bridge.
- F12 note: an ODR write clobbers a driven input (G4).
Could not break: pin_held by another name (19 names); droop reset re-applies stimuli (D2g, real BOR reset); G2 pull never judged; no move without a pin device; ADC1 gate; INV-07 except F4.
