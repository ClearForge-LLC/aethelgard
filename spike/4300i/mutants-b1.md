# AET-WO-4300i phase B1 mutants

One mutant per guard phase B1 adds or changes (no internal motor pull), sized to the diff (CLAUDE.md). `spike/4300i/mutants-b1.py` applies each edit to the committed tree (`8f573af0`), runs the named test with `AETHELGARD_BLESS` unset so that no golden is rewritten, and restores the file with `git checkout`. A mutant counts as killed only when the named test is among the failures. The raw results are in `out/mutants-b1.tsv`.

**Result: 6/6 killed, 0 survived.**

| id | guard | file | mutant (exact edit) | killing test | result |
|---|---|---|---|---|---|
| B1 | svd-hal generator: a timer channel's PUPDR field is 00 (Role::policy) | `crates/aethelgard-svd-hal/src/board.rs` | `Self::TimCh1 \| Self::TimCh2 \| Self::TimCh3 \| Self::TimCh4 => (MODE_AF, 0, 0b00),` -> `Self::TimCh1 \| Self::TimCh2 \| Self::TimCh3 \| Self::TimCh4 => (MODE_AF, 0, 0b10),` | `aethelgard-svd-hal motor_outputs::motor_pins_carry_no_internal_pull_from_the_first_port_init` | killed |
| B2 | svd-hal oracle: a motor pin's expected PUPDR field is implied_pupd's, 00 | `crates/aethelgard-svd-hal/src/board.rs` | `.map(\|(pin, p)\| (pin.clone(), silicon::implied_pupd(solved, &p.net)))` -> `.map(\|(pin, p)\| (pin.clone(), if solved.motor_outputs.contains_key(&p.net) { 0b10 } else { silicon::implied_pupd(solved, &p.net) }))` | `aethelgard-svd-hal motor_outputs::motor_pins_carry_no_internal_pull_from_the_first_port_init` | killed |
| B3 | hwdef: a motor line's pull token is FLOATING | `crates/aethelgard-hwdef/src/lib.rs` | `"{} {} {} PWM({}) {} PUSHPULL FLOATING",` -> `"{} {} {} PWM({}) {} PUSHPULL PULLDOWN",` | `aethelgard-hwdef generate::every_motor_line_is_floating` | killed |
| B4 | ledger: a motor pin written with a pull is refused (LedgerRuleDisagrees) | `crates/aethelgard-cli/src/ledger.rs` | `if row.value != Some(rule) {             return Err(LedgerError::RuleDisagrees {` -> `if false && row.value != Some(rule) {             return Err(LedgerError::RuleDisagrees {` | `aethelgard-cli ledger_tier1::a_motor_output_s_pin_carries_no_internal_pull_by_the_independent_rule` | killed |
| B5 | ledger: the motor pin's rule is implied_pupd (00) | `crates/aethelgard-cli/src/ledger.rs` | `let rule = silicon::implied_pupd(topology, net);         let pin = topology` -> `let rule = 0b10;         let pin = topology` | `aethelgard-cli ledger_tier1::a_motor_output_s_pin_carries_no_internal_pull_by_the_independent_rule` | killed |
| B6 | check pinout: WrongPull on a motor pin pulled down or up | `crates/aethelgard-pincheck/src/lib.rs` | `let want_pupd = aethelgard_ir::silicon::implied_pupd(topology, net);` -> `let want_pupd = if topology.motor_outputs.contains_key(net) { 0b10 } else { aethelgard_ir::silicon::implied_pupd(topology, net) };` | `aethelgard-pincheck check::a_motor_pin_with_an_internal_pull_is_refused_as_wrong_pull` | killed |

B1 and B2 are killed by the hand-derived literal `0x64000000` (port A's debug-port reset pulls, with `00` at PA8..PA11), in the generator's `split` word and in the proofs' assertion. They are not killed by a golden comparison.

Not mutated:

- the removal of `IdleHighNotEmitted` and of the Q9 arms. Their tests (`a_motor_output_that_idles_high_is_emitted_like_any_other`, the svd-hal and ledger idle-high cases) fail if a refusal or a pull-up comes back. A removal has no guard of its own to mutate.
- `css_hold` and `start_outputs`. Neither writes `PUPDR`, before or after B1, so B1 changed no code in them.
