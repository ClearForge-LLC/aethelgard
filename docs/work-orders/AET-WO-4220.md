# AET-WO-4220 — Sensor models of our own: current flight parts the twin can witness

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4220-own-models`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass at the build.
**Phase:** P4. Ruled by Scotty on 2026-11-15, after the `4210` spike (PR #131, D1): every part the twin models and ArduPilot drives is EOL. **Cadence: spike (§1), then STOP for the architect's ruling, then build (§2).**

**Grounds:**
- **The twin's IMU and barometer models (Renode 1.17.0) are all EOL parts:** ICM-20948 and ICP-101xx (`4210` F3). The build in progress uses them as acknowledged stand-ins, refused for fabrication unless accepted by name.
- **Aethelgard's purpose is to match firmware to the hardware a developer actually has.** That means old parts, current parts and add-ons, simulated before anything is bought. A sensor the twin cannot model is a sensor the tool cannot vouch for.
- **`RUL-12` as amended on 2026-11-15:** Renode peripheral models are C#, because that is the language Renode compiles at load. They live under `models/renode/`, are pinned by sha256, and never hold engine logic.
- **What `4210` measured:** what a model must log so the witness can judge an ordered transaction script, with per-read windows, declared warnings, a judge-set stimulus and an exact value. A model of our own is written to that contract from the start.

> **What this is:** Renode models of a current flight IMU and a current barometer, written by this project, loaded by the pinned Renode, driven by ArduPilot's own probe and init, and witnessed by the `4210` script machinery. The aim is that Sentinel-02 can fly parts that are in production.
>
> It is NOT:
> - a general sensor library (two parts, then a rule for adding more);
> - sensor physics beyond a judge-set stimulus;
> - a fork of Renode.

## 1. Spike

Probes under `spike/4220/`. **Predictions go into FEEDBACK before any run.** Run every container with `~/bin/capped-run`.

**S1. Can the pinned Renode load our C#?** In the pinned tools image, under `--network none`:
- load a minimal C# peripheral from a mounted file (`include @…cs`, or the mechanism the platform supports);
- attach it behind the SPI multiplexer and on an I2C bus;
- read one register.

Record: load time, compiler errors on a deliberate fault, and where the compiled assembly goes. Then propose how the emitted platform names the model file and pins its sha256, so a changed file is refused.

**S2. The parts, by evidence.** For each candidate IMU (for example ICM-42688-P, ICM-45686, BMI088, BMI270) and barometer (for example BMP390, DPS310, ICP-20100, MS5611), record:
- lifecycle, from the vendor page held by sha256;
- the datasheet, held by sha256, or listed for Scotty if the host refuses Furnace;
- the ArduPilot driver at `fb773567` and the registers it touches at probe and init;
- the I/O voltage, which matters for `4300d`'s 1.8 V question;
- KiCad 10.0.7 symbol and footprint availability.

**Recommend one IMU and one barometer, with a reason.**

**S3. The model contract.** For the recommended parts, propose the register map the model implements:
- **the minimum:** the identity, every register ArduPilot writes at init, and the data registers;
- what it logs per transaction, in the form `4210`'s witness reads (step kinds, Noisy, bytes in the line);
- how a judge-set stimulus becomes register values through the datasheet's scale;
- **data-ready:** a model of our own can raise it from the sensor's own sampling, not from a firmware store. Say how, and what that would let `RUL-19` (5) judge;
- **FIFO:** ArduPilot reads these IMUs through their FIFO. Say whether the model implements it now or the bring-up script reads the data registers, and what ArduPilot-in-the-twin would later need.

**S4. Tests.** Propose how a model is tested:
- the C# itself, against the datasheet's register defaults;
- through the twin (tier 2), with the `4210` witness;
- against ArduPilot's probe sequence, read at source.

Say what is mutated in the adversarial pass.

**S5. Upstream.** Read Renode's contribution path and licence. Propose whether and when the models are offered upstream. That is a public act, so it is Scotty's call.

**Stop** with:
- the table;
- the recommended parts;
- the model contract;
- the load mechanism with its pin;
- the decisions needed.

## 2. Build (after the ruling; expected, and the ruling confirms)

1. **`models/renode/<Part>.cs`** for each ruled part, each pinned by sha256 in the emitted platform. A changed model is refused.
2. **`parts.json` rows** for the parts (lifecycle `active`, cited), with each transaction script.
3. **A Sentinel-02-shaped fixture** with the IMU and barometer, witnessed end to end with no EOL acknowledgement needed.
4. **Tests at each level** from S4, and a full adversarial pass.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-02`, `INV-06`, `INV-07`; `RUL-05`, `RUL-11`, `RUL-12` (as amended), `RUL-19`.
- **The spike fence:** `spike/4220/` and FEEDBACK only. Datasheets go in `~/.cache/aet-4220/ds/`.
- **Other builders:** read `~/.cache/aet-bus/WHO.md`.
  - The `4210` build owns the witness's script machinery: tell it, through the mailbox, what your model will log.
  - `4300d` owns the 1.8 V question.
- One PR, left unmerged. Write `~/.cache/aet-4220/pr-title-body.md`. **Push and revoke by the FUR-WO-0301 F7 route.**

## 7. Flag-and-stop conditions

- **The pinned Renode cannot load a C# file offline.** That is a ruling: a Renode version bump, or a prebuilt assembly.
- **No current IMU or barometer has both a fetchable datasheet and an ArduPilot driver.** List the URLs for Scotty.
