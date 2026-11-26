# AET-WO-1919 predictions, committed before the measurement

Written 2026-11-26 at about 02:30Z, on `main` `649e81a8`. No CI job timing, probe line or Furnace run of this work order had been read yet.

**What I had already seen:**
- the work order's figures: 70.4 min for `bench-package-s03` in CI after `1918`'s split (run 45620143376), against a 90-minute limit; 16 min on Furnace (`4300k`);
- `spike/1918/results.md`: the harness table (pre-split and split at 80), CI's `tim1` at 399 s on an EPYC 7763, Sentinel-03's boots at a median of 11.0 s on that CPU, and boots up to 20.8–23.6 s under shard load;
- the code: `simulate --platform` runs Renode through `Invocation::run`, with no bound (`main.rs`, "Only a boot is bounded"); `verify` runs each gate as its own subprocess, one after another, and logs no gate time; `ci/probe.py` logs each kani-driver's harness, VmHWM and lifetime, and each Renode's lifetime and argv;
- a grep for the one-line literal `"frames": []`: one hit, in `boot_tier1.rs`, inside an input refused for `input_version: 2`.

## (1) `bench-package-s03`

| # | Prediction |
|---|---|
| P1 | **Steps on CI** (slowest runner): image pull 1–4 min; CLI and bridge build 5–10 min; `verify --against-fab` 52–62 min; the judge step under 1 min; the rest under 1 min |
| P2 | **`prove`** is 55–75 % of `verify`'s step on CI, that is 30–45 min, across 26 harnesses run one after another |
| P3 | **Inside one prove gate,** compiling the crate with Kani costs 10–30 s and is repeated for each of the 26 harnesses (5–13 min in all). CBMC plus kani-driver's parse is the rest |
| P4 | **The slowest harness by wall time on CI** is one of `every_valid_sequence_tim1_0` / `_1`, at 220–400 s each. The six split parts of `tim1`, `spi1` and `spi2` take more than 40 % of `prove`'s time |
| P5 | **The gates beside `prove`** take 12–25 min on CI: the campaign of 11 scenarios 4–8 min, the firmware build 2–5 min, emit with ERC and DRC 3–7 min, extract and attribute 1–3 min |
| P6 | **Runner CPUs:** every `bench-package-s03` job measured ran on an AMD EPYC (7763, 9V74 or 9V45). The 7763 is the slowest. The job's wall time varies by more than 15 % across runs |
| P7 | **On Furnace,** `verify` of Sentinel-03 takes 30–50 min in one container, depending on the box's load. `prove` is again the largest share |
| P8 | **The job's own CLI build repeats `tools-build`'s work:** the same image, the same lock file, from cold, 5–9 min |
| P9 | **Options:** no single cache or build reuse brings the job under 45 min on the slowest runner. Proving the harnesses in 3–4 parallel jobs, then composing, reaches 30–42 min. Composing prove results changes what `verify` derives, so it is ruled first |

## (2) Platform loads

| # | Prediction |
|---|---|
| P10 | **Every load on CI** (`simulate --platform`, and every tier-2 test that loads) takes at most 10 s, with a median of 1–4 s, on every CPU. **No load reaches 24 s** (within 5× of 120 s), apart from run 45709090931's hung one |
| P11 | **On Furnace,** every load takes at most 4 s |
| P12 | **Loads are 10–40 of all the Renode runs** in one full CI run |
| P13 | **No CI test loads through the MCP `simulate` tool** with Renode present. Its load is measured on Furnace only |
| P14 | **Renode is also run unbounded outside a load:** `renode --version` before each simulate, and `doctor`'s probes. Each takes under 5 s |

## (3) An empty input

| # | Prediction |
|---|---|
| P15 | **No committed input, archive or test relies on `"frames": []` being accepted.** The one literal in `boot_tier1.rs` is refused for its version first, and stays refused with the same kind if the version check comes before the empty check |
| P16 | **"An empty input by another name"** cannot reach a run. A frame's `mavlink` must parse as one whole listed MAVLink frame, so no frame has nothing to send. But an input of heartbeats only, which arms nothing, is accepted today |

## 4411

| # | Prediction |
|---|---|
| P17 | **`4411`'s pushed branch** (`de132429`, its spike record) holds no Sentinel-03 with the module, so it gives nothing to measure. Its build adds one UART harness of under 2 GiB and under 60 s |
