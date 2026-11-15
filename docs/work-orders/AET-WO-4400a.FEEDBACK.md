# AET-WO-4400a FEEDBACK — ArduPilot in its own pinned image, built in CI, and a pinout check read from the linked ELF

**Builder:** Claude (strong tier), worktree `~/aethelgard-4400a`, branch `wo/aet-4400a-ardupilot-image` from `main` `cb44872`.
**Cadence:** build, with one measured stop (§1.4). The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #132); it is not committed here.

Labels:
- **[read]**: read at source, in this repository at `cb44872`, in ArduPilot at `Copter-4.7.2` (`fb773567`), or in Renode 1.17.0.
- **[measured]**: run on Furnace or on GitHub's runner; the cap, `nice` and `-j` are given with each run.
- **[inferred]**: neither; reasoning from the two.

## 1. Predictions (committed before any image build, ArduPilot build, decode or Renode run)

All **[inferred]** from the `4400` spike's measurements (`AET-WO-4400.FEEDBACK.md` §2, §4, §5) unless marked.

**The image.**
- Base `debian:trixie-slim` by index digest (the tools image is trixie too), Python 3.13 from the archive. ArduPilot's waf runs under 3.13 as it did under the spike's 3.14.
- Every one of the nine pip packages has a wheel that 3.13 on x86_64 can install, except `empy==3.3.4`, which is an sdist only. Building it needs `setuptools`, which `--require-hashes` does not cover through pip's build isolation. So `setuptools` is pinned with its hash and installed first, and the rest go in with `--no-build-isolation`.
- Image size: 2.3–2.8 GB uncompressed (1.4 GB clone, 0.69 GB toolchain, about 0.15 GB venv, about 0.1 GB of Debian).
- Cold build on GitHub's runner: 3–6 min, most of it the toolchain download from `firmware.ardupilot.org` and the clone. Warm, from the `gha` cache: 2–4 min, dominated by importing about 2.5 GB of layers.
- The smoke (`MatekH743`, `-j$(nproc)`, 4 vCPU): 331 CPU-s ÷ 4 plus overhead, so **90–150 s**, under `--network none`, and `git status --porcelain` prints nothing.

**The `ardupilot` job.**
- Each of the four boards takes 280 CPU-s on Furnace, so **75–120 s** on the runner from clean.
- **One tree against four clean trees.** All four boards are installed as `Sentinel01`, so building them one after another in one tree reuses `build/Sentinel01`. `hwdef.h` reaches nearly every translation unit, so the reuse saves under 20 %. I expect one container per board and one container for all four to differ by under 10 %, and I will keep whichever is simpler if they do.
- The whole job (image from cache, four boards, building the `aethelgard` binary on the runner, the checks) takes **10–15 min**, under the 20-minute line.
- **D4:** with `GITHUB_ACTIONS=true` in the container, `configure` exits non-zero with `Numeric board ID found (7777)` (`hwdef.py:129-139`, [read] in the spike). With it unset, all four boards configure and build.

**The decode of step G** (all 11 ports; the spike hand-decoded GPIOA only):
- GPIOA: `MODER` `0x0000A801`, `AFRL` `0x55500000`, `AFRH` `0`.
- GPIOB: `MODER` `0x0000A000`, `AFRL` `0x44000000`.
- GPIOD: `MODER` `0x000A0000`, `AFRH` `0x00000077`.
- Every other port: `MODER` `0`, both AFRs `0`. Every unnamed pin is input, including PA13, PA14, PA15, PB3 and PB4, whose RM0433 reset state is alternate (debug).
- `hwdef.h` agrees with the ELF on every port.

**The reverse and package checks need a definition of "configured".**
- I expect the literal "every pin whose `MODER` differs from its reset value" to flag almost every pin. ArduPilot writes input (`00`) to every unnamed pin, while RM0433 resets most pins to analog (`11`). It would also flag ports I, J and K, which mostly do not exist on LQFP144.
- **Predicted definition:** a pin is configured when it is driven, that is output or alternate, and is not at its RM0433 reset `MODER` and AF. Input and analog drive nothing.
- Under it, step G configures exactly the 6 topology pins plus PD8 and PD9.

**The seeds**, as the spike measured them, now from the Rust check:
- `seed-pin`: forward, `I2C_SCL` at PB6 is input, not alternate AF4; reverse, PB8 alternate AF4 named by nothing.
- `seed-af`: forward, PB6 and PB7 are on AF6, solved AF4.
- `seed-pkg`: package, PI0 is an output and is not a pin of `STM32H753ZITx` (LQFP144); reverse also names PI0.

**The ELF reader.** Hand-written, about 200 lines, with no new dependency. The CLI already holds a strict ELF32 header reader (`crates/aethelgard-cli/src/elf.rs`), and `object` would add a dependency tree for about 60 lines of parsing.

**D8, one Renode run of step G's ELF.**
- **The cap may stop it before it starts.** `4210` measured (its F10, bus note 02:40 UTC) that podman 5.7.0 with `--memory 8g --memory-swap 8g` leaves `memory.swap.max` at 8589934592, not 0. The kickoff's rule then stops the run before Renode starts. I expect the same reading.
- **If it runs:** nothing on USART3 within a bounded virtual time. The last PC sits in a ChibiOS or ArduPilot busy-wait on a peripheral Renode's H753 platform models only partly. My guess is the storage or flash driver, or the IWDG. The spike's reading found no blocker before the banner, so this is a weak prediction.

**CI time across the PR.**
- This PR also triggers `tools-image` (`container/**`) and `engine-image` (`crates/**`, `Cargo.*`): 10–14 min and about 4 min, as on recent runs.
- `tools-shard (1)` already takes 17 m 19 s. The new crate's tests are tier 1 and add seconds to `gates`, not to any shard.
