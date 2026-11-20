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

**What the crystal sees** with `C1 = C2 = 18 pF`, each net's stray `s` adding to its capacitor
(`CL = (C1+s1)(C2+s2)/(C1+C2+s1+s2)`, the per-net form [inferred]; AN2867 Rev 10's lumped `Cs` is the series of the
two strays when `C1 = C2`):

| Strays per net | Load at 18 pF | C for CL 12 pF |
|---|---|---|
| copper bound + CIO typical (12.47 / 11.02 pF) | 14.86 pF | 12.3 pF |
| copper bound, no pin (7.47 / 6.02 pF) | 12.36 pF | 17.3 pF |
| no copper, CIO typical (5 / 5 pF) | 11.50 pF | 19.0 pF |
| nothing (0 / 0) | 9.00 pF | 24.0 pF |

So the declared 18 pF fits CL 12 only for a stray near 6 pF per net, which no held source bounds. P8 (a mismatch
at about 15 pF) holds only at the top of the bracket. Not changed: a finding.
