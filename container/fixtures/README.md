# Tool-calibration inputs

`container/smoke.sh` runs each tool in the image on these known inputs and checks the known answer:

- `drc-violation.kicad_pcb`: two tracks on nets `NET_A` and `NET_B`, 0.05 mm apart against KiCad's default 0.2 mm clearance. `kicad-cli pcb drc --exit-code-violations` must exit 5 and report one `clearance` violation.
- `drc-clean.kicad_pcb`: the same board outline with no tracks. The same command must exit 0.
- `rc-step.cir`: a 1 kΩ resistor into a 1 µF capacitor driven by a step. ngspice must measure a 10–90 % rise within 2 % of 2.2·R·C = 2.2 ms.
- `kani-smoke/`: a crate outside the workspace with two Kani harnesses. `add_in_range` must verify, and `add_overflows` must fail on its seeded overflow.

These files are tool-calibration inputs, not board artifacts. No specification produces them, and `INV-01` (`docs/northstar.md` §4) governs only artifacts generated from a specification, so a builder should not flag-and-stop on them.
