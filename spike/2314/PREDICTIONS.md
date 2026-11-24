# AET-WO-2314 §1.1 — predictions, committed before any measurement

Written 2026-11-24 on `72d85eef` (main), from reading the code, the CI logs and `spike/1609`, before any run.

## (1) The four causes reaching `word_of`

`campaign.rs:1224` reads `verdict.boot.boot_word_ns` only. That is `None` when the hook saw no write of
`BOOT_MAGIC` over a non-magic word (`renode.rs:2523-2527`), when hooks were not read (dropped lines,
`renode.rs:2515`), or when the boot was killed at a bound (`main.rs:2220-2228`: no hooks are read).

- **P1.1** Each cause, planted under the campaign's tier-1 stub, gives today the same refusal text,
  "the undisturbed boot wrote no boot word to place it from", exit 2, with no cause in it:
  (a) memory bound (`common::HOLD_PAST_CAP`); (b) a log flood past 8 MiB; (c) a word other than
  `BOOT_MAGIC` written (an unnamed fault word, `0xDEADBEEF`); (d) a firmware parked on a named code
  (`BOOT_PLL1_NOT_LOCKED`, `0x2100B0F5`); (e) no write at all, the core halted.
- **P1.2** The undisturbed boot's own verdict (the one `boot(None)` returns) already names each:
  (a) `memory_cap_exceeded` with "1536 MiB" (the wall bound's `wall_bound_exceeded`, "30 s", reaches the
  same arm); (b) `renode_log_flood` (info) with the 8 MiB budget, **but not the dropped count**, which is
  in `run::Output` only, never in the generated verdict (the foreign verdict has `log_lines_dropped`);
  (c) `boot_word_missing` reading `0xdeadbeef`; (d) `boot_word_missing` with "boot_pll1_not_locked";
  (e) `boot_word_missing` reading the reset word, plus `cpu_halted` or a PC finding.
  So the fix needs no new verdict field: the campaign carries the undisturbed verdict's findings (and its
  `memory` report) into the refusal. The dropped count needs `main.rs` (outside the fence) or a verdict
  field (a stop): a decision.
- **P1.3** The CI failures of `bor_recovery::the_bmp581_extension_recovers_after_its_settle` are **not
  the wall bound.** Read from CI's own stamps (job 138183630071): the test's firmware build line is at
  1795490050.48 and its FAILED line at 1795490060.34, 9.9 s later; the boot ran under 10 s, a third of the
  30 s bound. The same holds in job 138173503045. My lean for the cause: the memory cap (the only other
  bound, and the one that moves with a runner's state), then a flood; least likely a fault word (the ELF is
  byte-identical to the passing runs). Furnace will likely not reproduce it at 4 CPUs (4410: 287/0).

## (2) A hook on the four fault vectors

- **P2.1** Handlers from the ELF's vector table at `FLASH_BANK1_BASE` (`plan.vector_table`, which the
  plan writes to VTOR): entries 3–6 (HardFault, MemManage, BusFault, UsageFault). On the CI
  `sentinel-02` ELF they are four distinct handlers (ChibiOS/ArduPilot define each), with HardFault's
  near `0x08000340` as in `spike/1609`.
- **P2.2** On ArduPilot's normal 5 s boot no fault handler is entered (0 hook lines): ArduPilot's boot
  takes no fault on the twin (`1609` control: ICSR 0x800, CFSR/HFSR 0).
- **P2.3** On `1609`'s planted `udf` the HardFault hook fires once at about 4.392441 s (20.27 µs after
  U5's data witness), before the flood: the access log's 20 M lines come after it, from CrashCatcher's
  polling. A hook that stops the core there (halting the CPU, or pausing the emulation) ends the
  flood, so the run never nears the 1.5 GiB cap: peak RSS about the control's 845,000 KiB.
- **P2.4** Which stop works is uncertain: `IsHalted = True` may be undone by a pending interrupt
  (WFI semantics); a monitor-level pause from inside a CPU hook may deadlock `RunFor`. I predict a
  halt that holds through the window's end is possible from the hook (IsHalted, re-asserted on each
  entry, or the hook itself as a self-loop), and `RunFor` then completes with the reads intact.
- **P2.5** The faulting PC (stacked at SP+24 on the exception frame, MSP or PSP by EXC_RETURN bit 2) can
  be read in the hook; the vector by IPSR (xPSR & 0x1FF) or by which handler address fired.

## (3) TDR writes in the foreign filter

- **P3.1** No foreign rule reads a USART `TDR` access line: `judge_serial` reads writes in
  `CR1..CR2+4` only; the locked, error and unhandled-write rules read other line shapes; no rule matches
  `TransmitData` or offset 0x28.
- **P3.2** Dropping them changes **no** foreign verdict's findings on the positive case (568 console
  writes in 5 s, `2311` A5) and **no** peak RSS: the filter runs in the CLI on Renode's stdout, while the
  peak in R3 is Renode's own (VmHWM). So R3's peak is unchanged by (3) within noise (±2 %); (2) is what
  moves it.

## (4) The panic hook's address against the ELF's call sites

- **P4.1** `AP_HAL::panic` has tens to low hundreds of direct `bl` call sites in ArduPilot (format
  strings across libraries). A decoy is a different function named `_ZN6AP_HAL5panicEPKcz`; the real
  panic is reached by `bl` from many sites. Counting `bl` targets is a full scan of `.text`
  (about 1.5 MB of Thumb-2) for 32-bit `bl` encodings: milliseconds in Rust, but "sure" is the hard
  part: a decoy can also be called from many sites, so a call-site count proves the symbol is called,
  not that it is the panic. I predict it is cheap but not sure: it stays a row, with the numbers.
