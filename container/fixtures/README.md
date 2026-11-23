# Tool-calibration inputs

`container/smoke.sh` runs each tool in the image on these known inputs and checks the known answer:

- `drc-violation.kicad_pcb`: two tracks on nets `NET_A` and `NET_B`, 0.05 mm apart against KiCad's default 0.2 mm clearance. `kicad-cli pcb drc --exit-code-violations` must exit 5 and report one `clearance` violation.
- `drc-clean.kicad_pcb`: the same board outline with no tracks. The same command must exit 0.
- `rc-step.cir`: a 1 kΩ resistor into a 1 µF capacitor driven by a step. ngspice must measure a 10–90 % rise within 2 % of 2.2·R·C = 2.2 ms.
- `kani-smoke/`: a crate outside the workspace with two Kani harnesses. `add_in_range` must verify, and `add_overflows` must fail on its seeded overflow.
- `erc-clean.kicad_sch`: a schematic with nothing on it. `kicad-cli sch erc --exit-code-violations` must exit 0.
- `erc-dangling-label.kicad_sch`: one local label connected to nothing. The same command must exit 5 and report exactly one `label_dangling` error.
- `kani-compile-error/`: a crate that does not compile. `aethelgard prove` must report a `compile_error`.
- `bridge-request.json`: the request `aethelgard simulate --firmware` writes for the reference specification's `I2C_SCL` line at 2.2 kΩ (`AET-WO-2200`). `container/engine-smoke.sh` runs `aethelgard-bridge` on it and checks the `AET-WO-2101` S2 rise time, 186.401 ns.
- `renode-missing-include.repl`: a platform whose only line includes a file Renode does not have. `aethelgard simulate` must report a `platform_load_error`.

`container/smoke.sh` uses the KiCad files and `kani-smoke`; the `aethelgard` tier-2 tests (`--features with-tools`) use all of them.

These files are tool-calibration inputs, not board artifacts. No specification produces them, and `INV-01` (`docs/northstar.md` §4) governs only artifacts generated from a specification, so a builder should not flag-and-stop on them.

## The engine image

`ghcr.io/clearforge-llc/aethelgard` is the pinned tools image plus the `aethelgard` binary (`AET-WO-1501`) and, beside it, `aethelgard-bridge` (`AET-WO-2200`), published from `main` as `:<commit sha>` only, never `latest`. Pull and run it by digest, which the `engine-image` run summary prints for each publish: `docker run --rm -i --network none -v "$PWD:/repo:ro" -w /repo ghcr.io/clearforge-llc/aethelgard@sha256:<digest> aethelgard validate --file examples/sentinel-01-minimal.json`.

## The pin check's fixtures

`check-pins/` holds Dockerfiles that `container/check-pins.sh` must refuse (`AET-WO-4401`). The forms are the four that 4400a's A7 left open (`git submodule update --remote`, `git fetch` and `git pull`, `pipx`, a `pip install` without `--require-hashes`), a venv whose pip is unpinned, and the ways the adversarial pass tried to slip a form past the check. Each `# expect:` line names one refusal. `container/test-check-pins.sh` requires exactly those refusals and no others, and requires the three real Dockerfiles to pass. `copy-script.sh` is never run: it is the script that `copy-script.Dockerfile` copies in. `AET-WO-4407` adds:

- the `# syntax=` spellings BuildKit reads (case, spacing, `//`, after a shebang) and a byte-order mark;
- an `apt-get update` with no snapshot source, or in a stage that skips it, and the ways around the source (a second one written, `APT_CONFIG`, a missing file);
- a fullwidth digit in a `FROM` digest.

`snapshot.sources` is the snapshot source the apt fixtures copy. A fixture in a directory of its own (`apt-moving-sources/`, `apt-two-dates/`) has its own `*.sources` beside it. The test runs every fixture under `C` and under `en_US.UTF-8`.

## The GCC source pin's fixtures

`gcc-source/` holds pairs of a `GCC-SOURCE` pin and a `manifest.json` that `container/check-gcc-source.sh` must refuse (`AET-WO-4403`). Four malformed pins prove that it delegates the pin's form to `check-ardupilot-pin.sh`. Two pins disagree with their manifest. The rest are manifests whose pin matches their bytes but whose shape is not the mirror's. `container/test-check-gcc-source.sh` names each fixture's one refusal and requires the committed pin and manifest to pass. The generator was a one-off; each fixture is plain JSON, and its pin is `sha256sum` of its manifest.

## The GPL-2.0-only Debian source tables' fixtures

`debian-src/` holds tables that `container/debian-src.py check` must refuse (`AET-WO-4403` D5), cut from two real packages of `ARDUPILOT-DEBIAN-GPL2.tsv` (`base-passwd`, `hostname`). `ok.tsv` must pass. The rest each break one rule: the form of a row or field, the order, exactly one `.dsc` per package, the derived tag, or the derived manifest digest. `container/test-debian-src.sh` names each fixture's one refusal and requires both committed tables to pass.
