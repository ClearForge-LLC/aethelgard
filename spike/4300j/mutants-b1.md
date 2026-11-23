# AET-WO-4300j Phase B1: mutants of the guards this build adds or changes in svd-hal, hwdef and the ledger

Run by `spike/4300j/mutants_b1.py` in a copy of the tree (B2 edits the worktree at the same time): each mutant applied as exact text, its killing test run alone, every file restored byte for byte. B5, B6 and B15 mutate Phase A's `silicon::implied_pupd` (not edited by B1) to show that the oracle, `check pinout` and the ledger each read it.

| id | guard | mutation | killing test | killed |
|---|---|---|---|---|
| B1 | svd-hal generator: `open` writes `10` (`motor_pull`) | `open` gives `00` | `aethelgard-svd-hal motor_outputs::each_motor_pin_s_pupdr_follows_its_fitment_in_the_generator_and_the_oracle` | yes |
| B2 | svd-hal generator: `fitted` writes `00` (`motor_pull`) | `fitted` gives `10` | `aethelgard-svd-hal motor_outputs::each_motor_pin_s_pupdr_follows_its_fitment_in_the_generator_and_the_oracle` | yes |
| B3 | svd-hal generator: a motor pin's pull comes from its fitment | the timer role's pull instead (4300i) | `aethelgard-svd-hal motor_outputs::each_motor_pin_s_pupdr_follows_its_fitment_in_the_generator_and_the_oracle` | yes |
| B4 | svd-hal oracle: a motor pin's expected field from `implied_pupd` | the oracle holds every motor pin `00` | `aethelgard-svd-hal motor_outputs::each_motor_pin_s_pupdr_follows_its_fitment_in_the_generator_and_the_oracle` | yes |
| B5 | the oracle's rule: `implied_pupd` `open` gives `10` (Phase A's, read by the oracle) | `open` gives `00` | `aethelgard-svd-hal motor_outputs::each_motor_pin_s_pupdr_follows_its_fitment_in_the_generator_and_the_oracle` | yes |
| B6 | pincheck `WrongPull` on an `open` motor pin (by `implied_pupd`) | as B5 | `aethelgard-pincheck check::an_open_motor_pin_with_no_internal_pull_is_refused_as_wrong_pull` | yes |
| B7 | hwdef pull token: `open` writes `PULLDOWN` | `open` writes `FLOATING` | `aethelgard-hwdef generate::each_motor_line_s_pull_follows_its_fitment` | yes |
| B8 | hwdef pull token: `fitted` writes `FLOATING` | `fitted` writes `PULLDOWN` | `aethelgard-hwdef generate::each_motor_line_s_pull_follows_its_fitment` | yes |
| B9 | hwdef comment line: `fitted` names the link | the link left out | `aethelgard-hwdef generate::a_comment_line_above_each_motor_line_names_its_fitment_and_bound` | yes |
| B10 | hwdef comment line: `open` names the bound | ceiling written as the floor | `aethelgard-hwdef generate::a_comment_line_above_each_motor_line_names_its_fitment_and_bound` | yes |
| B11 | hwdef comment line: one above each motor line | no comment line | `aethelgard-hwdef generate::a_comment_line_above_each_motor_line_names_its_fitment_and_bound` | yes |
| B12 | hwdef render: a comment line sets no width | comment lines widen every line | `aethelgard-hwdef generate::a_comment_line_above_each_motor_line_names_its_fitment_and_bound` | yes (second run; see below) |
| B13 | ledger rule: `MOTOR_PUPD_RULE` names `open`, `10` | the `open` clause dropped | `aethelgard-cli ledger_tier1::a_motor_output_s_pin_pull_follows_its_fitment_by_the_independent_rule` | yes |
| B14 | ledger note: an `open` row names its fitment and link `DNP` | an `open` row written as `fitted` | `aethelgard-cli ledger_tier1::a_motor_output_s_pin_pull_follows_its_fitment_by_the_independent_rule` | yes |
| B15 | ledger rule value: an `open` pin's rule is `10` (by `implied_pupd`) | as B5 | `aethelgard-cli ledger_tier1::a_motor_output_s_pin_pull_follows_its_fitment_by_the_independent_rule` | yes |

B12 survived the first run: the test held only that every sourced line shares one width, which a comment line that
set the width also gives. The test now holds that width to the longest sourced line's own length; B9 to B12 were run
again (`mutants_b1.py <copy> B9 B10 B11 B12`) and all four were killed. 15 mutants, 15 killed.
