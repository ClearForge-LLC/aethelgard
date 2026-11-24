# AET-WO-2314 — the measurement (§1.1), and the stop

Measured 2026-11-24 on `72d85eef` (main). Predictions were committed first (`c73d9e94`, `PREDICTIONS.md`).
The twin side ran in the pinned tools image (`container/IMAGE`, `1e0ab105…`) under `~/bin/capped-run` (8 GiB, swap 0),
`--network none`, `nice 10`, one container at a time. The ELFs are CI's own: `ardupilot-image` run 45342089937 on main
`0faaaec3`. `generated-sentinel-02/arducopter.elf` has sha256 `5eace4ca…`, and `plant.py udf` plants `udf #0` at
`0x080360aa`, the same site as `1609`'s. Every number below is from a run whose line is in `out/` or quoted from this
session's runs (`ANSWER 2314:` lines).

**Answers up front.**
1. **(1) is buildable inside the fence with no new verdict field.** All six planted endings give today the same bare
   `PlantedEventUnplaced`. The undisturbed verdict names five of them in its findings. The sixth, a boot word that
   reads `BOOT_MAGIC` but that no hook saw written, has no finding of its own, so the refusal must name it itself.
   The dropped-line count is not in the generated verdict.
2. **(2) can pause before the cap.** A hook on the ELF's four fault handlers that sets `IsHalted` freezes the core at
   HardFault entry (4.392440830 s). `RunFor` still returns at 5 s with every read intact. The planted HardFault peaks
   at 856,180 KiB, against 1,575,296–1,580,632 KiB killed at the cap today. The stacked PC is the planted `udf`'s
   address. ArduPilot's normal 5 s boot enters no fault handler. **Not a stop.**
3. **(3) cannot move R3's peak.** The access-log flood is 20,250,312 `ISR` reads, which the filter already
   discards. `TDR` writes are 600 lines (58,804 bytes). The peak is Renode's own memory, upstream of the CLI's
   filter. No foreign rule reads a `TDR` line, but the generated judge does, so the drop must be foreign-only.
4. **The BMP581 CI failure is unproven, and my prediction's argument was wrong** (P1.3, below). By the numbers, the
   wall bound is the lead: alone the boot takes 4.1 s of the 30 s, and 21.8 s when eight run on 4 CPUs. Memory
   reaches 745 MB of 1,536 MiB, and the log 10 KB of 8 MiB. The build names the cause: its first CI run and the
   10-run shard-11 replay are the instruments.

---

## (1) The causes reaching `word_of`

**The plants** (`out/plants.diff`, a scratch test in `campaign_tier1.rs`, reverted). The campaign's tier-1 stub ran
`droop-bor3` with the undisturbed boot planted six ways. Each ending was then run again under `simulate` with the same
stub, to read the verdict that `boot(None)` returns.

| Plant | Campaign today (exit 2) | The undisturbed verdict's own end |
|---|---|---|
| memory bound (`common::HOLD_PAST_CAP`) | bare refusal | `memory_cap_exceeded`: "passed the 1536 MiB cap"; `memory` {peak 1,740,704, bound 1,572,864} |
| log flood (160,000 lines past 8 MiB) | bare refusal | `renode_log_flood` (info): "passed 8 MiB", **no count**; 7 downstream errors (`*_unwitnessed`, `serial_*`) |
| fault word `0xDEADBEEF` | bare refusal | `boot_word_missing`: "reads 0xdeadbeef, not 0x2100b007" |
| park code `BOOT_PLL1_NOT_LOCKED` | bare refusal | `boot_word_missing`: "… 0x2100b0f5 …: boot_pll1_not_locked, RCC_CR.PLL1RDY did not rise within the firmware's bound" |
| no write (word reads 0) | bare refusal | `boot_word_missing`: "reads 0x00000000" |
| **`BOOT_MAGIC` read, no hook line** | bare refusal | **no finding names it**: the word reads `0x2100b007`, `boot_word_ns` is null, and only `serial_echo_missing` mentions the hook. On a board with no serial port, nothing does |

The wall bound reaches the same arm as memory (`main.rs:2220-2228`, `renode::bounded`): `wall_bound_exceeded`,
"30 s". So does `cpu_aborted`. Neither was planted here: the wall takes 30 s, and the boot's tier-1 test already
covers both.

**The mapping I planted** (the WO's four words): *bound* = `memory_cap_exceeded` / `wall_bound_exceeded` /
`cpu_aborted`; *dropped lines* = `renode_log_flood`; *fault word* = any word other than `BOOT_MAGIC`; *park code* =
a fault word that `pwr_boot_word`/`hse_boot_word` name (`BOOT_PLL1_NOT_LOCKED` is the FEEDBACK's own example). Two
more were planted because the code has them: *no write*, and *magic with no hook*.

**P1.1 held. P1.2 held, with one miss:** I had not predicted the sixth ending, which no finding names.

**P1.3 was wrong in its argument.** I read job 138183630071's "firmware build" line at 1795490050.48 as the BMP581
test's start. That line is printed by another test's helper (`ANSWER 2100:`). `battery_sag::firmware` prints nothing.
**The test's span, derived from libtest's own stamps.** The binary runs 9 tests on 4 threads, started in name order as
each thread frees. Each "running for over 60 seconds" line lands 60.0 s after the start this derives: 8 of 8 lines
check within 0.03 s.

| Job (commit) | Started (after) | Ended | Span |
|---|---|---|---|
| 138183630071 (09c2ff8d) | …005.35 (`a_step_sent…` ok) | FAILED …060.34 | **55.0 s** |
| 138173503045 (e39c84c5) | …401.66 (`an_echo…` ok) | FAILED …459.20 | **57.5 s** |
| 138219678879 (046fdbfa, same packing, passed) | …762.03 (`a_firmware…` ok) | ok …858.20 | 96.2 s |

The failure comes at the campaign's first boot (`boot(None)`), after `generate` and the firmware's `build`. A
55–57.5 s span is what a build of about 25 s plus a boot killed at the 30 s wall bound would give. So the wall bound
fits the stamps; it is still not proven.

**The BMP581's margins, measured** (`sh/bmp.sh`, `sh/contend.sh`; `examples/sentinel-01-ext-bmp581.json`, the
undisturbed boot as the campaign's `boot(None)` runs it, the whole 100 ms window):

| Condition | Runs | Wall s | Boot word ns | Peak RSS KiB (cap 1,572,864) | Log bytes (budget 8,388,608) |
|---|---|---|---|---|---|
| alone | 4 | 4.0–4.2 | 23,127,440 (all) | 515,220–516,860 | 10,465 (10 lines) |
| 4 at once, `taskset -c 0-3` | 4 | 11.5–12.1 | 23,127,440 | 618,660–745,256 | — |
| 8 at once, `taskset -c 0-3` | 8 | 20.5–21.9 | 23,127,440 | 616,796–663,492 | — |

- Wall time scales with load: about 2.7 s per concurrent boot on four of Furnace's cores. The other causes stay far
  from their bounds: memory at 47 % at worst, the log at 0.1 %. The word is the same in every run, so a fault word is
  out.
- **CI history** (`ci.yml`, the last 60 runs): shard 11 failed only on `4410`'s branch, at e39c84c5 and 09c2ff8d
  (×2), 02:24–02:50Z. Their `shard.tsv` (288 tests) is byte-identical (`cmp`, sha256 `24495afd…`) to 046fdbfa's, which passed. Every main run passed.
- So the same packing gave 3 fails and 1 pass. Neither packing alone nor the code decides it; the runner's load does.
  That fits the wall bound, and memory less well.
- The shard-11 replay (that `shard.tsv` at 4 CPUs until a failure or 10 passes) runs in the build, after (1). On
  today's code a failure prints the same bare refusal CI already printed.

## (2) A hook on the four fault vectors

**The handlers** (the ELF's table at `FLASH_BANK1_BASE`, `plan.vector_table`, Thumb bit cleared; `measure.py`):
HardFault `0x08000340` (`HardFault_Handler`), MemManage `0x080c02bc`, BusFault `0x080c02b4`, UsageFault `0x080c02b8`.
All four are distinct, and none is shared with another vector of the 166 read (P2.1 held).

**What a CPU hook can do** (`measure.py probe`): Renode's `cpu` exposes `IsHalted`, `Pause`, `RequestPause`,
`OtherSP`, `IsUsingProcessSP`, `VectorTableOffset` and `Bus`. `ReadDoubleWordFromBus` is protected and refused, so
reads go through `self.Bus.ReadDoubleWord`.

**The runs** (`sh/measure.sh`: the CLI's own captured monitor script plus the hooks; `sh/after.py`; VmHWM polled
every 0.1 s):

| Run | Exit | Wall s | Peak KiB | Hook lines | After the hook |
|---|---|---|---|---|---|
| control, CLI (`simulate --foreign`) | 0 pass | — | 867,156 | — | — |
| control + halting hooks | 0 | 9.4 | 930,800 | **0** | VTOR `0x08000000`, live entries = ELF's, CFSR 0, HFSR 0 |
| udf, CLI today (×2) | 1 fail | 25.0, 25.3 | 1,575,296, 1,580,632 | — | `memory_cap_exceeded` only, nothing read |
| udf, Renode alone, no hook, access log | 0 | 53.3 | **3,036,176** | — | 2.35 GB of stdout |
| udf + logging hook, no access log | 0 | 15.5 | 1,227,316 | 1 | — |
| udf, probe hook, no access log | 0 | 30.8 | 1,613,120 | 1 | — |
| **udf + halting hook, access log** (4 runs) | **0** | **8.2–8.9** | **856,180–872,564** | **1** | 0 firmware lines after; markers and every read intact; the same hook line each time |
| noimu ELF (`aa9aecd3…`) + halting hooks | 0 | 8.6 | 885,436 | **0** | VTOR, entries = ELF's; CFSR, HFSR 0 |
| four-port ELF (`da2b1976…`, its own capture, CI's derived spec) + halting hooks | 0 | 7.7 | 683,640 | **0** | VTOR, entries = ELF's; CFSR, HFSR 0; its CLI run passes (662,796 KiB) |

- **The hook's line:** `[00:00:04.392440830] … AETHELGARD-FAULT HardFault pc=8000340 icsr=803 lr=ffffffed sp=30000600
  osp=30027310 stacked=80360aa vtor=8000000 n=72565519`. The stacked PC, read from the PSP frame (EXC_RETURN bit 2
  set) at +24, is `0x080360aa`, **the planted `udf`'s address** (P2.5 held).
- **The halt holds** (P2.4 held, `IsHalted` alone):
  - The instruction count at the run's end is `0x453430F` = 72,565,519, the hook's own count.
  - The PC stays at `0x8000340`, and ICSR reads `0x803`.
  - `RunFor` returned at 5 s with BEGIN, END and every read present.
  - No interrupt un-halted the core. Pause and emulation pause were not needed, and were not run.
- **Without the access log, the HardFault still passes the cap on one run of two** (1,613,120 KiB in 30.8 s;
  1,227,316 KiB in 15.5 s). `1609` measured 1,101,724 KiB. So the halt, not the log, is what keeps a faulted run under
  the cap.
- **Coverage:** only HardFault was exercised: a `udf` with UsageFault disabled escalates to it. MemManage, BusFault
  and UsageFault are hooked the same way but never entered. Renode answers an unmapped read with a log line, not
  a bus error, so a BusFault plant may not be reachable in the twin. The three CI ELFs whose normal boot `foreign-judge.sh`
  runs enter no handler. Its other cases are these ELFs with a panic call or a float default patched; none was run
  with the hooks.
- **What co-occurs:** the foreign judge keeps `core_in_exception` (ICSR.VECTACTIVE 3 at the end, `foreign.rs:1547`).
  A halted run therefore also fails `core_in_exception`. Every witness came before the fault (U5's data at
  4.392420560 s), so no `*_unwitnessed` is expected.

## (3) TDR in the foreign filter

**The udf run's access lines** (`sh/hist.sh`, Renode alone with the access log):

| Source, op, register | Lines | Bytes |
|---|---|---|
| usart1 ReadUInt32 0x1C (ISR) | 20,250,312 | 2,349,036,192 |
| usart1 WriteByte 0x28 (TDR) | 523 | 51,112 |
| usart1 WriteUInt32 0x28 (TDR) | 77 | 7,692 |
| every other USART line | 43 | 4,532 |
| non-access log lines | 7,194 | 961,181 |

- The flood is `ISR` polling. `is_unjudged_access` already discards it in the CLI, uncounted.
- Renode's peak comes from Renode printing those lines, before any filter. So dropping `TDR` (0.0025 % of the
  bytes) cannot move R3's peak (P3.2 held).
- The before-and-after in the build will show the same `memory_cap_exceeded` both ways without (2), and about
  856 MB with (2).

**Readers of `TDR`** (a read-only sub-search, spot-checked at `serial.rs:1053`, `1237` and the one user of
`FOREIGN_LOG_FILTER`, `main.rs:2180`):
- **Foreign judge:** none.
  - `judge_serial` → `judge_options` takes only writes in `cr1..cr2+4` (0x0..0x8, `serial.rs:981`).
  - `error_lines`, `locked`, erase, device lines and `unhandled_write_key` read other shapes (`[ERROR]`,
    `[WARNING]`, hook lines).
- **Generated judge:** **yes.** `judge_writes` ("TDR before UE"), `tx_at_send` and `judge_pins` ("at the first TDR
  write") read the access-log `TDR` line through `access_boots`. So the drop must be a **foreign-only discard**,
  beside `unjudged_access_line`. `is_unjudged_access` stays as it is.
- **By another name:**
  - The only foreign effect is the budget: fewer kept bytes, so a flooding firmware passes 8 MiB later.
  - No committed test pins `log_kept_bytes`: `boot_tier1.rs:3006` asserts `> 0`, and `ci/foreign-judge.sh:87` asserts
    `< budget`.
  - One theoretical adjacency: `error_lines` checks the line before an `[ERROR]` for a CFG2 hook line. Removing a
    `TDR` line between the two could exempt that error. The `ISR` discard already has the same property.
P3.1 held.

## (4) The panic hook against the ELF's call sites

A read-only sub-measurement in Python (`r4/`; run from `~/.cache/aet-2314/r4`, where its paths point). I re-ran
`q2.py` on `control.elf` and it reproduced the counts below.

| | sentinel-02 (control) | four-port | sentinel-02-noimu |
|---|---|---|---|
| panic symbol, size | `0x080c033c`, 76 | `0x080becdc`, 76 | `0x080becb0`, 76 |
| symbols of that name / others at that address | 1 / 0 | 1 / 0 | 1 / 0 |
| direct `BL` callers (every-halfword scan = precise sweep) | 56 | 54 | 54 |
| `B.W` tail calls; words equal to `panic\|1` | 0; 0 | 0; 0 | 0; 0 |
| callers passing a string literal in r0 (`ldr` from the pool) | 56/56 | 54/54 | 54/54 |
| scan time, Python (1.3 MB of code) | 0.04 s naive, 0.10 s sweep | — | — |

**Decoys,** made by rewriting the symbol's `st_value` in copies of control.elf (`r4/q5.py`). Three candidate rules:
- A: the hook address has at least one caller.
- B: A, and every caller passes a literal.
- C: the hook address is the most-called function whose callers all pass a literal.

| Decoy | Callers | A | B | C |
|---|---|---|---|---|
| a function with no caller | 0 | caught | caught | caught |
| the most-called function (`0x080c0394`) | 702 | passes | caught | caught |
| `__assert_func` | 14, all literal | passes | passes | caught |
| the name moved to a one-caller literal function, real panic left nameless | 1 | passes | passes | caught |

- 55 functions have at least as many `BL` sites as the panic. 3,813 of 11,022 FUNC symbols have no direct caller.
- Only the panic and `__assert_func` are all-literal with ten or more callers: 56 against 14 on control, 54 against 14
  on the others.

**P4.1 held.** The check is cheap: well under a millisecond per MB in Rust, and no Renode. It is **not sure**:
- Rule C is a heuristic over the judged ELF's own code (INV-07).
- A firmware built to defeat it passes:
  - a decoy given more literal-passing callers than the panic;
  - a panic reached only through `BLX`, a vtable or a tail `B.W`;
  - a panic inlined as a loop.
- Rules A and B catch only careless decoys.
- So it stays a row unless the ruling takes rule C as "closes the symtab-only decoy", with the residual named.

---

## Decisions for the ruling

**D1. The refusal's carried end (1).** `place` is reached with no word from two call sites, the droop's `boot(None)`
and the battery sag's, where the run's own boot is the undisturbed one (`campaign.rs:1229-1245`). Both carry the
end, and the plants cover both: the six endings under the droop, and at least one under a sag scenario. When the
undisturbed boot yields no boot word, `PlantedEventUnplaced` appends
that boot's end, taken from the verdict `boot(None)` returned:
- every non-info finding, as `kind: message`;
- `renode_log_flood` when present;
- for a bound, the `memory` report (peak against bound);
- when none of those applies and the word reads `BOOT_MAGIC`, its own sentence: "its boot word reads 0x2100b007 at the
  run's end, but its hook saw no write of it".

So two endings at once are both named. It is campaign.rs only, with no new verdict field and no change to any
committed verdict. One planted test per ending (six), each with a mutant.
- **The dropped count:** not in the generated verdict (P1.2).
  - (a) Name the flood and its 8 MiB budget, with no count, all in the fence. **Lean.**
  - (b) Pass `Output::dropped_log_lines` out of `main.rs`'s `simulate` beside the verdict. That is outside the fence.
  - (c) Put the count in `renode_log_flood`'s message. That changes the archived verdict bytes run to run
    (`AET-WO-2301` F10), so no.

**D2. `foreign_fault` (2), built as measured.**
- **The hooks:** at the four handlers read from the ELF's table at `plan.vector_table`, Thumb bit cleared.
  - Each logs a CPU `[WARNING]` mark with the vector, the stacked PC (PSP frame when EXC_RETURN bit 2 is set, else
    MSP, at +24), ICSR and VTOR, then sets `IsHalted`.
  - It fires only when `ICSR.VECTACTIVE` equals the vector's number. That way a handler reached by a call, or through
    a moved table that routes it elsewhere, never fails a run falsely.
- **The finding:** `foreign_fault`, naming the vector, the PC and the entry's virtual time.
- **A table the hooks cannot use:** an entry that is 0, outside every executable `PT_LOAD`, or the table itself
  outside the loaded image. The four CI ELFs have none, and four distinct handlers. Tier-1 fixture ELFs, which are
  tiny, probably will have them.
  - (i) A named refusal before the boot (`ForeignFaultVectorUnusable`, naming the entry), as `1609`'s missing panic
    symbol is. It is fail-closed, and the fixtures need a table.
  - (ii) Hook only the usable entries, and name the unusable ones as `unhooked` in the rule row's text and in the
    finding-less report. **Lean (i)** for a missing table or an entry outside the image.
  - Entries shared by several vectors are hooked once, and the vector is named from `ICSR.VECTACTIVE`, not from the
    address (measured: ICSR reads `0x803` in the hook).
- **Where the evidence goes:**
  - (a) In the finding's message only, plus one `RULES` row. **Lean,** because it adds no report field.
  - (b) A `boot.fault` report beside `boot.panic`, as `1609` did. It is a new key in the foreign verdict, so I ask
    first, since the WO stops on a new verdict field.
- **Co-occurring:** `core_in_exception`, kept, since both are true.
- **Its log mark** joins `FOREIGN_LOG_FILTER.keep`, as `PANIC_MARK` did.
- **Tests:**
  - tier-1: the hook's text and the mark's parse, as `panic_hook` has;
  - a stub run with the mark, failing by name;
  - CI: `ci/foreign-judge.sh` adds the planted-udf case, which wants `foreign_fault` naming HardFault and
    `0x080360aa`, and not `memory_cap_exceeded`. This needs the plant in the script (`plant.py`'s udf, two bytes
    found by symbol) or a committed plant ELF. **Ask:** plant at CI time (the script gains a ~40-line Python plant,
    added case only) or skip the CI case and keep the tier-2 evidence here.
- **Residuals, named in the row:**
  - a firmware that takes and handles faults deliberately fails;
  - a VTOR moved at run time to a table whose handlers are elsewhere is not hooked, and falls back to today's
    judging. The live VTOR at the end and the four live entries equalled the ELF's on both runs measured.

**D3. TDR (3).** A foreign-only `discard`: today's `unjudged_access_line`, or a USART write (any width) to its `TDR`
offset from the header. `LOG_FILTER` and `is_unjudged_access` stay untouched, because the generated judge reads those
lines.
- **Tests:**
  - the foreign filter drops `TDR` writes at each width;
  - it keeps `CR1`, `CR2`, `BRR` and the next word (`0x2C`);
  - `LOG_FILTER` still keeps `TDR`.
- **The peak "after":** measured in the build. Predicted: unchanged without (2), about 856 MB with it.

**D4. R4.** Cheap but not sure: lean **a row**, with the numbers above. If the ruling wants rule C built, it is a
`ForeignPanicNotCalled`-style refusal, with its margin and residual named.

**D5. The BMP581 (1, second half).**
- Build (1) first.
- Then replay shard 11's failing `shard.tsv` (288 tests, from run 45312081588) at `taskset -c 0-3`, until a failure or
  10 passes, and report the cause the refusal now names.
- The branch's first CI run is a second instrument.
- If it is the wall bound, I propose its fix as a ruling, with these numbers:
  - alone, 4.1 s;
  - 8 at once on 4 CPUs, 21.8 s;
  - against the 30 s bound.
