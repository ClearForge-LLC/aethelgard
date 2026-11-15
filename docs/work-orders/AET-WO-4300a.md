# AET-WO-4300a — Motor outputs: timer channels solved, generated, and witnessed in virtual time

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4300a-motor-outputs`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, the second piece of `AET-WO-4300`'s split (ruled on PR #123). **Cadence: build, with one measured stop (§1.2).** It starts after `4300c` merges, because both change the board crate's pin configuration.

**Grounds, all from the `4300` spike (`AET-WO-4300.FEEDBACK.md`, measured), on `main`:**
- **`STM32_Timer` drives a channel's pin** through the alternate-function route.
- **With virtual timestamps,** an edge the timer causes is stamped exactly, in 10 ns steps. A line the CPU causes carries the start of its 100 µs quantum. The CLI logs host time today (`log-timestamps-type = Host`).
- **The model's conventions** (`architecture.md` §2.21):
  - the period is `ARR` ticks, where silicon counts `ARR + 1`;
  - the first edge comes one period late;
  - `MOE` and `CCxP` are tags;
  - the clock is the platform's fixed 250 MHz.
- **DShot600's bit timing is resolvable;** DMA cannot drive a timer, so a frame's content is static-only.
- **2100 F24:** cold-write every `RCC_*ENR`. Timers sit in already-cold registers, but the fix is owned here.

**Ruled (`4300` D2, D3, D4):**
- **D2:** the CLI's composition moves to virtual timestamps, with the quantum stated as a `--config` key. Every existing witness keeps its verdicts.
- **D3:** a static check judges silicon (`PSC`/`ARR`/`CCR` from the clock tree, with `ARR + 1`). The twin judges the model's conventions, each named beside the judge with its `STM32_Timer.cs` line. The two are never blended into one tolerance.
- **D4:** the timer witness has three parts:
  - observer edges at the model's predicted stamps, exact to 10 ns;
  - the full TIM register read-back against the solved values;
  - at least one witnessed edge off the stated quantum grid.

  A solved timing whose edges all fall on the grid is judged static-only, and the judge says so.

> **What this is:** a specification declares motor outputs; the solver assigns timer channels and pins; the board crate and generated firmware configure them; the twin witnesses each output's period and pulse; and one static fault (`motor_output_dead`) renders and attributes.
>
> It is NOT:
> - DShot frame content (static only: the DMA configuration as registers);
> - ESC protocol beyond timing;
> - ArduPilot (`4400`).

## 1. Build

1. **The specification gains `motor_outputs`:**
   - each output's label;
   - its pin, or `null` for the solver to choose;
   - its protocol, `pwm`, `dshot300` or `dshot600`;
   - for PWM, its rate in Hz and its pulse range in µs.

   Every key is required (`INV-06`). The specification takes the next free `schema_version`, once.
2. **The solver assigns timer channels** from the pin and AF data the solver already reads, so that outputs sharing a timer share its rate. A rate conflict on one timer is refused by name.
   - The `SolvedTopology` carries each output's timer, channel, pin and AF, plus the solved `PSC`, `ARR` and `CCR` from the declared clock tree (`RUL-16`).
   - **The clock-tree assumption** the generator uses today must be a declared or solved fact. If it is not, measure first; that is §1.2.
3. **The board crate and generated firmware.** A timer-output typestate, PWM mode 1, and the configuration written in full (`MOE` included, for silicon). The generated bring-up firmware starts each output at its idle pulse (PWM) or a zero throttle bit pattern (DShot timing only). A Kani harness proves the register writes match the solved values.
4. **The twin and the judge.**
   - `Miscellaneous.LED` observers (or the cheapest pin observer the spike named) on each output pin, with `logLevel -1`.
   - D2's composition change.
   - D4's three-part witness, plus the TIM register read-back.
   - Kinds for the timer check and its witness: take the next trace version once.
   - Replay joins each record to its solved output, as `4200`'s kind 23 does for SPI.
5. **`motor_output_dead`,** one output's pin on the wrong AF, so no edges:
   - a seeded case attributed electrical or software as the evidence allows (state which, and why);
   - its trace records, replay and rule.
6. **2100 F24:** cold-write every `RCC_*ENR` the header defines, and judge each in full.
7. **A test fixture:** a 4-output PWM specification, and a 4-output DShot600 one, with every gate passing in the image.

**§1.2 Measure first, and STOP if it is large: the clock tree.** Read how `svd-hal` and the generator choose the system clock today: HSI 64 MHz, or a PLL. The static check needs the timer kernel clock as a fact.
- **If it is fixed and written in one place,** declare it in the solved topology and proceed.
- **If the generator has no clock-tree model,** stop with what a minimal one needs: the oscillator, the PLL, and the `APB1`/`APB2` timer clocks. That is a ruling, because `4400`'s `hwdef.dat` needs the oscillator too.

## 2. Acceptance

- **Witnessed in the twin:** each fixture's outputs, at the solved period and pulse, with at least one edge off the grid. A store-made edge fails, and so does the coincident `UG`-plus-`BSRR` forgery, which this work order measures first (`4300` D4).
- **The reference's verdicts are unchanged** apart from the named consequences of the virtual-time composition, measured against `main`'s binaries as `4200` did.
- **`motor_output_dead`** attributes as its manifest says.
- **The CLAUDE.md gates pass.** CI is green with every shard under 20 minutes, refitting by `ci/shard.sh`'s rule if needed. `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01` to `INV-07`; `RUL-12`, `RUL-14`, `RUL-16`, `RUL-19`.
- **Fence:**
  - `aethelgard-ir` (the spec and the solver);
  - `aethelgard-svd-hal` (timers, `RCC`);
  - `aethelgard-twin` (observers);
  - the CLI (composition, judge, campaign);
  - `aethelgard-chaos` (records, rules);
  - `aethelgard-evolve` (the rule);
  - the seeded set, fixtures and tests;
  - `ci/`;
  - FEEDBACK.
- One PR, left unmerged. Write `~/.cache/aet-4300a/pr-title-body.md`. **Push and revoke by the FUR-WO-0301 F7 route.**

## 5. Adversarial pass, full

1. **Forge an output:** `BSRR` edges, a `UG`-plus-`BSRR` pair in one quantum, the timer run at another rate with the claim kept.
2. **Make a correct output fail:** prescaler rounding, `ARR` versus `ARR + 1`, the late first edge, a rate at the edge of the grid.
3. **Break the solver:** two outputs at different rates on one timer, an output on a pin with no timer AF, a timer shared with an SPI or I2C pin.
4. **Make `motor_output_dead` attribute wrongly.**

## 7. Flag-and-stop conditions

- The generator has no clock-tree model (§1.2).
- The virtual-time composition changes any existing witness's verdict.
