# AET-WO-4400i — A foreign firmware hears MAVLink: a declared, timed input on its port, the receive request wired, the input in the trace, and the sensors' rest declared

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4400i-foreign-hears-mavlink`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, the foreign judge. Written 2026-11-24. **Cadence:** a measurement, a stop, then the build.

**Grounds:**
- **`AET-WO-4400h`** (#393, merged as the record), D3 in `spike/4400h/RESULTS.md` §5. ArduPilot armed over MAVLink, with no defaults file, no skip and no force, but only in a replay outside the CLI, with three local changes:
  - **the receive request:** `usart1`'s `ReceiveDmaRequest -> dmamux1@41` (`patches/usart1-rx-request.diff`). Without it, no byte is read;
  - **IDLE is a tag**, so frames were padded to ArduPilot's 64-byte receive buffer;
  - **two stimuli:** a barometer with a constant reading, and an IMU whose witness stimulus puts 1 g on y ("Leaning").
- **Northstar §3.1** (ratified 2026-11-24): the first application is an agent-flown drone. An agent commands it over MAVLink, and the paths an armed, commanded autopilot takes are the ones the twin must answer for first.
- **Tviburi:** `TVI-WO-1303` (merged) acknowledges request 41 by name because no model raises it. `TVI-WO-1305` (the USART guard, written beside this one) is the consumer row's other half. The seam is Tviburi's records read by hash (northstar §3.1, independence).
- `INV-04` (an injected input must replay from the trace), `INV-06` and `INV-07` (cite them; do not restate them).

> **What this is:**
> - **(1) Measure first, with predictions committed:**
>   - how the generated firmware's serial injection works today (its record, its trace kind), and what a foreign firmware lacks;
>   - each serial port's receive request line on the emitted platform, against DMAMUX1's table, and whether emitting it moves any committed verdict;
>   - IDLE:
>     - what ArduPilot's receive path does without it;
>     - whether the padding is a sound stand-in or hides a fault;
>     - what an overlay that raises it would cost;
>   - the two stimuli: where the 1 g on y comes from (the parts table's witness stimulus), and what "at rest" must be for each sensor (cited: 1 g on the axis the board's orientation names, and a pressure the barometer's datasheet allows);
>   - **the window.** Does the arm fit inside the CLI's 5 s window on Sentinel-03 (`4400h` (1): keep 5 s)? If not, say how long it needs. The window's cost is `AET-WO-4400j`'s.
> - **(2) The build** (after the ruling):
>   - **A declared input:** a file of timed frames for a named port, hashed into the verdict. `simulate --foreign` sends it at its stated virtual times. The input is recorded in the trace, so replay reproduces it with no file (`INV-04`; a trace version bump).
>   - **The receive request lines** emitted for each declared serial port, from DMAMUX1's table, cited.
>   - **IDLE,** as ruled from (1): an overlay that raises it as silicon does (cited), or the padding kept, with the gap named in the verdict.
>   - **Declared rest stimuli:** a board at rest by its declared orientation, never by a constant the judge invents (`INV-07`). A witness stimulus that is not at rest is named.
>   - **An armed verdict:** with a declared input that sets parameters and arms, `simulate --foreign` reports the armed state as a witness, read from ArduPilot's own heartbeat, never from the input that asked for it.
> - **Each refusal comes with a test, and each test with a mutant it kills.**
>
> It is NOT:
> - the window's cost or the IMU log flood (`AET-WO-4400j`, after this one);
> - DShot over MAVLink's out-of-memory (`4400h` D4 a2, a row);
> - a reboot of a foreign firmware (`4400h` D4 a, a row);
> - a flight model or a SITL bridge (a row);
> - Tviburi's USART guard;
> - the radio module (`AET-WO-4411`).

## 1. Build

1. **Measure** (1). **Stop** with:
   - the measurement, each answer from a run;
   - the input file's format and the trace record, as proposed;
   - the IDLE choice;
   - the rest stimuli, cited;
   - whether the arm fits the window.
2. **Build** (2) to the ruling. Merge main before READY.

## 2. Acceptance

- **Sentinel-03's unmodified ArduPilot,** under `simulate --foreign` with a declared input of `PARAM_SET`s and an arm, reports **armed**, witnessed from its own heartbeat. Its replay from the trace alone reproduces the run.
  - If the window is too short, the stop says so, and the acceptance is the replay outside the bounds with the gap named.
- **No forced arm, no skipped check, no defaults file.** A planted input carrying the force code is refused by name. So is one whose frames do not parse as MAVLink.
- **A planted input** that sets a parameter ArduPilot rejects is reported as rejected, from ArduPilot's own `PARAM_VALUE` or `STATUSTEXT`.
- **Every committed verdict** is unchanged, or each change is named and ruled.
- **The CLAUDE.md gates pass, and CI is green,** with every CI guard run locally first.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-cli/src/renode.rs` and `renode/foreign.rs` (the input, the trace record and the armed witness);
  - the platform emitter (the receive request lines and the IDLE overlay);
  - `crates/aethelgard-twin` (stimuli and the trace record);
  - the trace format (a version bump);
  - their tests and goldens;
  - `ci/foreign-judge.sh` (added cases only);
  - `spike/4400i/`, `docs/work-orders/AET-WO-4400i.*`.
- **Not:**
  - the IR's specification keys (a new key is a stop: the input is a run's file, not the board's);
  - `fab/`;
  - Tviburi;
  - the steering documents.
- **Running beside you:** `AET-WO-4300k` and `AET-WO-4411`. Add, never rewrite, and merge main before READY.
- One PR, left unmerged. Write `~/.cache/aet-4400i/pr-title-body.md`.

## 5. Adversarial pass, full

- **An arm the judge believes because it asked for it:** a witness read from the input instead of the heartbeat, or a heartbeat forged by the input's own bytes echoed back.
- **An input the replay cannot reproduce:** a frame sent at a time the trace rounds, or one whose bytes the trace does not hold.
- **A rest that is not rest:** a stimulus that passes pre-arm on a board whose declared orientation says otherwise.

## 7. Flag-and-stop conditions

- **After §1.1,** with the measurement.
- **A committed verdict moves.** Stop with the diff.
- **The arm needs a forced or skipped check, or a defaults file.** Stop with the check.
