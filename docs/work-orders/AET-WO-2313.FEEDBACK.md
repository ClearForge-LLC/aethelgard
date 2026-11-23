# AET-WO-2313 — FEEDBACK

Base: `origin/main` 9b246a6a. **Status: STOPPED at §1.1 by §7's first condition, before any code change.**

## 1. Measure first (§1.1): every place the bound is stated or relied on

Searched with `git grep` for `1 GiB`, `GiB`, `1024 * 1024`, `1024*1024`, `1048576`, `1_048_576`, `1,048,576`,
`1073741`, `1 << 20/30`, `memory_cap`, `MEMORY_CAP_FINDING`, `rss_kib`, `--rss-kib`, `bound_kib`,
`share_permille`, `1024 MiB`, `head -c`, `900`/`‰` across `crates/`, `ci/`, `.github/`, `container/`,
`scripts/`, READMEs and `docs/`. `spike/**`, `ci/fixtures/**` and past FEEDBACK files are dated records and
are not moved.

### 1.1 Code: where the cap is enforced or reported

| Site | What | Bound it runs under |
|---|---|---|
| `cli/src/renode.rs:210–222` | `BOOT_BOUNDS = {30 s, 1024 * 1024 KiB}` and its doc comment | the constant |
| `cli/src/main.rs:2096`, `:2098` | `simulate --firmware` and `--foreign` boots (`run_bounded_filtered`) | `BOOT_BOUNDS` |
| `cli/src/main.rs:2036`, `:2094` → `analog.rs:736–756` `analog::boot` | a droop scenario's boot: **the bridge owns Renode as its child** (`--renode`, D1), so the reaped tree is bridge + Renode | `BOOT_BOUNDS` |
| `cli/src/analog.rs:365` (`measure`), `:454` (`rail`), `:567` (`chain`) | the bridge alone (ngspice) | `BOOT_BOUNDS` (shared) |
| `cli/src/doctor.rs:409` | the bridge's `--version` probe | `BOOT_BOUNDS` (shared) |
| `cli/src/main.rs:2236` | the verdict's `memory` record: `bound_kib` = `BOOT_BOUNDS.rss_kib`, for every boot including the droop boot | reads the constant |
| `cli/src/renode.rs:310–315` | the finding's text: "passed the {rss_kib/1024} MiB cap" | reads the constant |
| `cli/src/run.rs:398, 545, 996, 1096` | the reaper: `--rss-kib N` argument, `peak > b.rss_kib` | generic, no literal |
| `cli/src/verdict.rs:140–144` | `share_permille = peak * 1000 / bound_kib` | generic, no literal |

No second literal of the bound in any script, workflow, `container/` file or test helper. The only other
`1048576` is `routing_tier1.rs:107` (`RUST_MIN_STACK`), unrelated. The `900‰` in roadmap L1056 is
`AET-WO-4400g` §1.4's measurement stop, not code.

### 1.2 Tests that plant past the cap or pin its value

| Test | Plant / assertion | Measured |
|---|---|---|
| `boot_tier1.rs:1005–1027` `a_boot_past_the_memory_cap_fails_and_names_it` (tier 1) | `HOG` = `x=$(yes \| head -c 1300000000); sleep 60`; asserts `memory_cap_exceeded`, `peak > bound_kib` | **the tree holds 2,543,124 KiB** (sampled every 0.1 s on Furnace; bash's command substitution keeps about two copies). It is already past 1.5 GiB, so **it passes at both caps unchanged and proves nothing about the new one** (§5's second item) |
| `boot_tier1.rs:1030` `an_aborted_machine_killed_at_a_bound_is_cpu_aborted` (tier 1) | the same `HOG`, killed at a bound, reported `cpu_aborted` | as above |
| `campaign_tier1.rs:2735–2762` `a_droop_boot_killed_at_a_bound_is_a_verdict_and_replays` (tier 1) | the same hog through the droop boot; expects `brownout_unrecovered`, `memory_cap_exceeded` | as above, about 2.5 GB |
| `twin_tier2.rs:2278–2284` (tier 2) | `bound_kib == 1_048_576` and `share_permille == peak * 1000 / 1_048_576` | pins the old value literally |
| `twin_tier2.rs:2622–2628` (tier 2) | asserts `memory_cap_exceeded` is **absent** on the m2 DMA mutant | not weakened by a higher cap |
| `chaos/tests/campaign.rs:3273` | the finding name only | no value |

A plant between the caps that the reaper must not kill: `dd if=/dev/zero bs=1300M count=1 | sleep N` holds
**1,350,928 KiB** (dd blocks writing to the sleeping reader with its buffer resident), inside
1,048,576–1,572,864 KiB.

### 1.3 Prose that states the bound

In the fence (to move):
- `crates/aethelgard-cli/README.md` L333 (the bounds paragraph: **1 GiB**, `"bound_kib":1048576`), L351
  (renode_log_flood "at 98.5 % of the 1 GiB memory bound": a measurement, about 1,032,800 KiB, which is
  about 656‰ of the new bound), L407 ("the 1 GiB bound is a ruling of its own"), L414 (the list: "the 30 s
  and 1 GiB bounds"), L484 (the bridge "with the same 30 s and 1 GiB bounds" as Renode: false once the
  constant is split).
- `cli/src/renode/foreign.rs` L24–30 (the comment) and **L181, a verdict string** ("30 s, 1 GiB, an aborted
  machine"; the rule table's row, so not a comment).
- `boot_tier1.rs:1005` ("About 1.3 GB … past the 1 GiB cap").
- `crates/aethelgard-chaos/README.md` L429: the finding name only, no number.

**Steering documents (§7's first condition), named and not edited:**
- `docs/architecture.md` L563: `RUL-13` itself ("a 1 GiB resident-memory cap … about 2.5× its peak";
  "the 30 s / 1 GiB bound also applies to the bridge"). The work order says the architect amends it; on
  `origin/main` 9b246a6a it still reads 1 GiB, and `docs/work-orders/AET-WO-2313.md` is not on main.
- `docs/architecture.md` L713: the 2026-11-08 amendments-log row ("a 30 s wall bound and a 1 GiB resident
  cap on Renode").
- `docs/roadmap.md` L184: `AET-WO-2100`'s record ("The boot is bounded at 30 s and 1 GiB").
- `docs/roadmap.md` §7.1 rows L982 (Sentinel-02's 63–65 % of the 1 GiB bound), L1056 (the foreign judge's
  1 GiB cap killed a baseline under load), L1154 (Tviburi `1301`: "the 1 GiB bound is judged in CI or on
  unslowed runs only"), L1158 (the HardFault under `2311`'s access log).

### 1.4 A verdict that depends on 1 GiB (§7's second condition): checked, none found

`AET-WO-1609`'s RESULTS (PR #376, `spike/1609/RESULTS.md`) measured its `udf` HardFault plant at
**2,589,980 KiB with `2311`'s access log and 1,101,724 KiB without it**. The second sits between the caps
(700‰ of 1.5 GiB). Its record of the same run without the memory bound: it ends in HardFault, ICSR `0x803`,
so `core_in_exception` names it. So under 1.5 GiB that shape still fails, by name rather than by the cap; the
shipped configuration (access log on each declared USART) still passes the new cap and fails
`memory_cap_exceeded`. The healthy control peaks at 844,492 KiB. No test asserts a real firmware fails by
the memory cap alone. I read this as no weakening; the architect may read 1609's "would name" (from ICSR,
not a judged run) otherwise.

## 2. Decisions needed

- **D1, the droop boot's bound.** The work order keeps "the bridge's 1 GiB (`cli/src/analog.rs`)", and the
  kickoff says split the constant if the bridge shares it. Three of the bridge's four runs are the bridge
  alone (`measure`, `rail`, `chain`; and doctor's probe). The fourth, `analog::boot`, is a **Renode boot
  whose parent is the bridge**, and the verdict reports its memory against `BOOT_BOUNDS` (`main.rs:2236`)
  and names `BOOT_BOUNDS` in its finding. **Lean: (a) the droop boot takes Renode's 1.5 GiB**, and a new
  `analog::BRIDGE_BOUNDS = {30 s, 1 GiB}` bounds `measure`, `rail`, `chain` and doctor's probe; the droop
  hog test moves past 1.5 GiB. (b) The droop boot keeps 1 GiB: then `main.rs:2236` and `renode.rs:314` must
  take the bound actually enforced, or the verdict misreports it.
- **D2, the steering-document lines** in §1.3: yours to amend or leave as dated records. L1154 and L982 read
  as live rows the ruling answers.
- **Fence:** the split touches `analog.rs` (three call sites and the new constant) and `doctor.rs:409`,
  outside the listed fence; `foreign.rs:181` is a verdict string. Both need the architect's yes.
