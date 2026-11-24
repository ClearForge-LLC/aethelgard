# AET-WO-4400h — ArduPilot armed in the twin: how long a window holds, how ArduPilot arms there with no defaults file, and what arming reaches

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4400h-armed-window`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** P4, the foreign judge. Written 2026-11-24. **Cadence:** a spike that ends in a stop. Nothing outside `spike/4400h/` is committed; the build is a later work order, written from the stop.

**Grounds:**
- **Tviburi's `OQ-03`** (Tviburi `architecture.md`, the `OQ-03` row): the cut of paths that must refuse before the first silicon re-check waits for "a longer or armed ArduPilot window".
  - In ArduPilot's 5 s window (`TVI-WO-1150` (v)), the 120 TIM1/TIM2 motor rows are "not reached in the window", not "never".
  - `TVI-WO-1303` measured that USART1's receive over DMA never moves a byte in the twin.
- **Scotty, 2026-11-24:** the drone is meant to be flown by an agent, not a pilot: commands over MAVLink, with ArduPilot's own failsafes keeping authority. The paths an armed, commanded ArduPilot takes are the ones that fly.
- **`AET-WO-4400g`** (#238): ArduPilot runs at 1.00000× in a 5 s window. **`AET-WO-4400f` F1:** RSS grows with wall time. **`AET-WO-2313`:** Renode's cap is 1,572,864 KiB.
- **`AET-WO-4406`** (#289): `check pinout` refuses an ELF that applies parameter defaults at boot (`ParamDefaultsOverride`). So arming cannot come from a baked-in `defaults.parm`. Parameters must arrive at run time, as an agent would send them.
- **`TVI-WO-1302` FEEDBACK §7:** AP-m runs 50 Hz PWM, not DShot, because `MOT_PWM_TYPE` is at its default.
- **`AET-WO-2314`** is building `foreign_fault` in `renode/foreign.rs`. Measure on main plus your own local patches; commit none of them.
- `INV-04` and `INV-06` (cite them; do not restate them).

> **What this is, all measured, predictions committed first:**
> - **(1) The window's cost.** Run `simulate --foreign` on `sentinel-02-own` at 5, 10, 20 and 30 s of virtual time (a local patch to the bound, recorded as a patch in `spike/4400h/`). Record:
>   - peak RSS, with and without `2311`'s access log;
>   - wall time, and the host load during the run.
>
>   Propose the longest window that keeps the margin `4400g` §1.4 required (a run over 900‰ of the cap stops it).
> - **(2) Arming, with no defaults file.** Inject MAVLink into the twin over the board's MAVLink port (say which, and how the twin injects into a foreign firmware's USART today, or what it lacks):
>   - `PARAM_SET` for the fewest parameters arming needs;
>   - then `MAV_CMD_COMPONENT_ARM_DISARM`.
>
>   Record:
>   - every pre-arm check that fails, each named by its own message and classed as: a twin model gap (a sensor reads constant, no GPS model), a parameter, or a real fault;
>   - whether a parameter set over MAVLink persists, and what the stock flash controller does with the write (bank 2: a direct input to `TVI-WO-1304`).
>   - **Prefer checks that pass to checks disabled, and a disabled check to a forced arm.** ArduPilot's force-arm flag is the last resort. If it is used, it is named in every record, never a default.
> - **(3) What arming reaches.** The access log armed against unarmed, in Tviburi's queue format (as `TVI-WO-1150` (v) produced `queue-ap.tsv`):
>   - which TIM1/TIM2 and DMA rows are reached;
>   - which peripherals newly start;
>   - whether `MOT_PWM_TYPE` set to DShot moves timer DMA.
>
>   Hand the file to Tviburi through the bus.
> - **(4) A commanded step, as far as the twin can say.** After arming, a GUIDED-mode command (a takeoff target): do the motor outputs change?
>   - Name what the twin cannot say: there is no vehicle dynamics model, so the IMU and barometer do not respond to thrust.
>   - Flight behaviour belongs to a flight simulator (ArduPilot's SITL), not this twin. Say what a bridge between the two would need, as a row, and build nothing.
>
> It is NOT:
> - a flight or physics model;
> - a `defaults.parm` emitter (any route around `ParamDefaultsOverride` is a stop);
> - a change to any bound's committed value;
> - any change to Tviburi.

## 1. Build

1. **Measure** (1)–(4). Commit the predictions in `spike/4400h/` first, then the records, the patches and the queue file.
2. **Stop** with:
   - the window table and the proposed bound;
   - the arming path, its injected bytes and its failed checks, each classed;
   - the armed queue file;
   - (4)'s answer;
   - the build work order you would write, with its fence.

## 2. Acceptance (of the spike)

- **Each number** comes from a run whose command and record are in `spike/4400h/`.
- **The armed queue file** is in Tviburi's format and names its ELF, board and window.
- **No file outside `spike/4400h/` and `docs/work-orders/AET-WO-4400h.*` changes.** The CLAUDE.md gates pass on the branch, and CI is green.

## 3. Fences

- **Fence:** `spike/4400h/`, `docs/work-orders/AET-WO-4400h.*`.
- **Not:**
  - `crates/` (local patches only, kept as patch files);
  - `renode/foreign.rs` (`AET-WO-2314`'s);
  - the IR;
  - `ci/`;
  - the steering documents.
- **Running beside you:** `AET-WO-4300k`, `2314` and `3309`. Your runs share Furnace: check `free -g` before each long run, use one container at a time, and use `~/bin/capped-run`.
- One PR, left unmerged. Write `~/.cache/aet-4400h/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **An arm that is not one:** ArduPilot reports armed while its outputs stay disabled, or a forced arm recorded as a passing check.
- **A window that holds only on a quiet host:** a run near the cap under the load of three other builders.
- **A queue row that is the injection's own traffic,** not ArduPilot's.

## 7. Flag-and-stop conditions

- **After §1,** with the measurement (the WO's one stop).
- **The cap fails before 10 s.** Stop with the curve.
- **Arming needs a defaults file or a rebuilt ELF.** Stop with the reason.
