# AET-WO-2302b FEEDBACK — attribution for the twin's new findings, the dead window code removed

Branch `wo/aet-2302b-attribution`, cut from `main` at `ecc6be9` (#175, `AET-WO-2302`, merged). The work order is on the docs branch (`claude/argon-repo-docs-synthesis-rjrb0d`, docs PR #198) until it merges; it is the source of truth. Every claim is labelled **measured**, **read at source** or **inferred**.

## §1 The classification, before any rule (WO §1.1)

Each of the ten kinds `AET-WO-2302` added is classed by what produces it in the twin, with the record that decides it.
- **Electrical** is the board, a part or the specification's declaration.
- **Software** is the firmware's own behaviour: its register writes and its timing.
- **Harness** is the judge's or the run's own bound, never a fault of the device under test.

Read at source for every row: `cli/src/renode.rs` (where the judge emits each kind), `cli/src/anchor.rs` (the hooks and the judge's actions), `chaos/src/record.rs` (kinds 10, 26, 29, 30) and `chaos/src/campaign.rs` (what replay binds).

| Kind | Class | What produces it in the twin | Evidence record that decides it | Where the records cannot decide |
|---|---|---|---|---|
| `motor_anchor_unobserved` | **software** | Edges on a motor pin, with no `CEN` rise by the CPU in the boot: the firmware toggled the pin itself (GPIO, `BSRR`), or started the timer in a way the CPU's stores do not show (DMA). Every CPU store width is hooked (word, half-word, byte, and `strd`/`stm`, measured in `2302`'s pass 2). No board fault drives a motor pin in the twin. | Kind 26: `cen_rises == 0`, `anchor_ns` absent, `rises + falls > 0`; and the verdict (kind 14). | None. |
| `motor_anchor_repeated` | **software** | A second `CEN` rise in one boot: the firmware wrote `CEN` again, by restart or clear-and-set. | Kind 26: `cen_rises >= 2`, which replay binds. | None. |
| `motor_anchor_out_of_order` | **software** when booted | The firmware's own code order: `CEN` before its last device step's witness, or after its boot word. | Kind 26: `anchor_ns`; kind 10: the boot word set; the verdict. | The device steps' instants are not in the trace, only the verdict says the anchor preceded one. When **no boot word** was set, the order "after the boot word" cannot be judged, and a board fault (a droop's reset) can also cost the boot word. Then it is **unexplained by name**, missing the boot word (kind 10), never guessed. |
| `motor_window_past_run` | **harness** | The anchor plus `DSHOT_WINDOW_NS` passes the run's end, `MAX_VIRTUAL_NS`. A firmware that starts its outputs at 99 ms is not wrong on silicon; the twin's run is too short to witness it. | Kind 26: `anchor_ns + DSHOT_WINDOW_NS > ` kind 10's run, the verdict. | The bound to name: `MAX_VIRTUAL_NS`, `DSHOT_WINDOW_NS`. |
| `motor_edge_before_anchor` | **software** | Edges before the timer's anchor in its boot: the firmware drove the pin before `start_outputs`. | Kind 26: `edges_before > 0`, which replay binds. | None. |
| `motor_edge_after_window` | **software** when the stop ran | Edges after the judge's stop cleared `CEN`: the firmware restarted the timer or drove the pin after the window. | Kind 26: `edges_after > 0`; kind 30: the stop present, with `cr1_after` showing `CEN` clear. | If the stop did not run or left `CEN` set (`judge_stop_missing`), the edges may be the timer still running. Then the edges are the harness's (below), not the firmware's. |
| `judge_stop_missing` | **harness** | The judge's own act did not run in the boot it reads, or left `CEN` set. Its cause is the judge's action (stale after a core lag, or not scheduled), never the device. | Kind 30 absent for an anchored DShot timer, or `cr1_after` with `CEN` set; the verdict. | The bound to name: the judge's stop at anchor + `DSHOT_WINDOW_NS`. |
| `judge_stop_mismatch` | **software** (a call against the work order's example: **for the architect**) | The judge's read-back of the timer's solved registers at the window's end differs from the solved values. The judge only reads them. The firmware wrote the timer inside its window: a `CCR`, `ARR` or `PSC` changed. It is `register_mismatch`'s evidence at another instant, which the software rules already own. | Kind 30: `read_back` entries off kind 11's expected values (bound by replay since `2302` A2-1). | None. The work order lists `judge_stop_*` among the harness examples. By its evidence, only `judge_stop_missing` is harness: a mismatch is the device's register state. I class it software and flag it rather than follow the example against the record. |
| `serial_wire_past_run` | **harness** | The summed wire time of the frames and echoes, from the boot word, passes the run's end. That is the twin's run bound, not a fault: the same firmware with an earlier boot word passes (measured in `2302` A2-3). | Kind 29: `injected_ns`; kind 27: the framing; the verdict (bound by replay since A2-5). | The bound to name: `MAX_VIRTUAL_NS` against the wire time. |
| `serial_injection_out_of_order` | **harness** | The judge's own injection chain went in out of order. The injections are the judge's act, and `2302`'s pass 2 read it as effectively unreachable (defence in depth). | Kind 29: `injected_ns` across the ports; the verdict. | The bound to name: the judge's sequential injection (`AET-WO-2302` D4). |

**Consequences for the rules (WO §1.2):**
- One software rule for each software kind: six. `motor_anchor_out_of_order` is attributed only when booted, and `motor_edge_after_window` only when the stop ran and cleared `CEN`.
- An explicit refusal for the four harness kinds, naming each bound. It never attributes them to either half.
- Where the records cannot decide, the kind is reported unexplained by name, with its missing evidence (`INV-06`), never plain `I1`.
- No kind is electrical. In the twin, nothing a board declares produces any of the ten. A board fault reaches them only through a lost boot word, which the rules above refuse to read as either half.
- **§7's stop does not fire.** Each kind's evidence is in the trace or the verdict.
