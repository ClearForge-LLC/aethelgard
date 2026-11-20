# AET-WO-4240c — measured (2026-11-20)

## §1.1 The null corpus (`kit/emit-sweep.sh`, `kit/same.sh`)

Baseline: `8df353ee` from `git archive`, built apart (`~/.cache/aet-4240c/target-base`). 29 specifications:
24 emit (72 files), 5 refuse on both (exit 2). Mask: none, stdout included (the out directory normalised).

| Branch commit | What it adds | Result |
|---|---|---|
| `13c67282` | the two vendored files | IDENTICAL, 72 files, 29 specifications |
| `f7bbb1e0` | + own_nets and the row's symbol id (`440d68ab`), the HSE group (`87136367`), the router | IDENTICAL, 72 files, 29 specifications |

## §1.2 Vendoring

Both files' git blob shas equal GitLab's files-API `blob_id` at ref `10.0.7` (P3 holds). `Crystal:Crystal_GND24`
is a 404 upstream; the symbol is `Device:Crystal_GND24` (P4 holds).

## §1.4 `Cs`: the 16 MHz crystal fixture on the real part

The fixture is `examples/sentinel-01-minimal.json` with `mcu.hse` = `4240a`'s crystal declaration (ABM8AIG-16, caps
18 pF + 18 pF, pull 30 ppm), emitted at library level (the CLI refuses it `HseNotGenerated` until `4240b`).

Routed: no via on either oscillator net. The crystal is 3.6 mm (pad 3 to PH1) and 5.3 mm (pad 1 to PH0) from
the MCU's pads, copper edge to copper edge.

`extract`'s bound (`oscillator_lines` + `bound`), copper only, from above:

| Net | Bound fF | Trace | Pads | Vias | Length | Pads on it |
|---|---|---|---|---|---|---|
| `U1_OSC_IN` | 7,473 | 5,118 | 2,355 | 0 | 10.75 mm | 3 (U1.23, Y1.1, C20.1) |
| `U1_OSC_OUT` | 6,016 | 3,661 | 2,355 | 0 | 6.875 mm | 3 (U1.24, Y1.3, C21.1) |

**P7 was wrong:** the bound is 6–7.5 pF, not under 1 pF. It is Dirichlet's shell bound at `S` = the declared
0.2 mm clearance, an upper bound by construction and far from the typical value; it gives no lower bound.

**The pin term:** DS12117 Rev 10 gives `CIO` 5 pF **typical**, no minimum, no maximum (Table 49 p. 137, rev Y;
Table 146 p. 241, rev V). P6 holds: no held source brackets `Cs`.

**Provisional:** KiCad has not yet run on this board; the bound's conditions assume a DRC-clean board.

**What the crystal sees.** `extract` bounds each net with every other conductor at 0, the other oscillator net
included, so each figure holds that net's stray to ground **and** its mutual capacitance `Cm` to the other
oscillator net. The ground share adds to its capacitor; `Cm` sits across the crystal and adds whole:
`load = series(C1 + s1, C2 + s2) + Cm`, with `s1 + Cm ≤ 7,473 fF` and `s2 + Cm ≤ 6,016 fF` (copper) and the pins'
`CIO` to ground [inferred: the circuit, not a cited formula; AN2867 Rev 10 lumps one `Cs`, uncited]. **The split
between ground and mutual is unknown.** With `C1 = C2 = 18 pF`:

| Strays | Load, least | Load, most (the split that maximises it) | C giving CL 12 at the most |
|---|---|---|---|
| copper bound + CIO 5 pF typical per pin | 9.00 pF (no stray) | 17.87 pF (`Cm` = 6.016 pF) | 6.28 pF |
| copper bound, no pin | 9.00 pF | 15.37 pF (`Cm` = 6.016 pF) | 11.28 pF |

So the load at the declared 18 pF lies somewhere in 9.0–17.9 pF against the row's CL 12 pF, and the value that
would give CL 12 lies anywhere from about 6 pF to 24 pF (24 pF with no stray at all). No held source narrows
either. The declaration is not changed: a finding.
