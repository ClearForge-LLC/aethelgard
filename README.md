# Aethelgard

Generates a circuit board and the firmware that drives it from one specification, and refuses to call either finished until the pair has survived simulated electrical and software faults together, before any board is manufactured.

**Status:** phases P0 to P2 are complete. P3's work has merged, and its exit gate waits on the bench rows that need the sensors (first silicon ran on 2026-11-16). P4's flight-controller work is under way. P5, a second chip family and firmware ecosystem judged as one combination (`docs/northstar.md` §3.1), waits on its trigger. **Use the engine image by its digest, as the Quickstart below does: it is the supported interface.** The crates.io release is 0.2.0 (2026-11-17), all ten crates: `cargo install aethelgard-cli --version 0.2.0 --locked` installs `aethelgard`, which reads specification 13. Like every release it is a pinned snapshot with no update promise (`RUL-21`), and `main` moves past it. From one specification, `aethelgard` does five things:

- solves the pin assignment;
- emits a schematic and a placed, routed board that pass KiCad's ERC and DRC;
- generates a typestate board crate whose register effects Kani proves (`aethelgard generate` writes it; `aethelgard verify` proves the generated harness);
- boots generated firmware in Renode;
- serves all of this over MCP.

P2 added analog co-simulation, fault campaigns, an archive that replays, and `attribute`, which names the electrical or software half a failure belongs to. **Its rise times are computed by ngspice from the specification's declared pull-up and capacitance bound, not measured on the routed board** (`docs/architecture.md` `RUL-19`); P3 added `extract`, an upper bound on each I2C line's capacitance read from the routed board and judged against that declared bound, as a gate of `aethelgard verify`; `verify --fab-out` writes the reference probe's fabrication package only when every gate passes, and its verdict names every file's hash. No board has been made from it; the bench comparison waits for the development board. The code is licensed `MIT OR Apache-2.0` (see `LICENSE-MIT` and `LICENSE-APACHE`). KiCad's vendored symbols and footprints, in `crates/aethelgard-kicad-lib`, stay `CC-BY-SA-4.0`. Boards emitted from them carry no obligation.

## Where to go

| I want to know | Read |
|---|---|
| Why this exists and what must stay true | [`docs/northstar.md`](docs/northstar.md) |
| What it is and how it is shaped | [`docs/architecture.md`](docs/architecture.md) |
| What order it gets built in, and what "done" means | [`docs/roadmap.md`](docs/roadmap.md) |
| The planning conversation these were synthesized from | [`docs/inputs/argon-session/`](docs/inputs/argon-session/) |

## Quickstart

With only Docker, `bash`, `curl` and `sha256sum`: this block fetches the pinned reference specification, pulls the engine image by digest, and shows the reference board pass ERC and DRC and a doubly-assigned pin refused by name. CI runs exactly this block (`.github/workflows/quickstart.yml`), so it cannot drift. The block is one subshell with `set -euo pipefail`, so pasted into a terminal it stops at the first failed check and never closes your shell.

<!-- quickstart:begin -->
```bash
(
set -euo pipefail
IMAGE=ghcr.io/clearforge-llc/aethelgard@sha256:59e9f0aa2e1ee6ab48aa12855b659fae7ca9a5e9a43046f9d1c1cede9ffdce62
SPEC=https://raw.githubusercontent.com/ClearForge-LLC/aethelgard/10806528ce75ab9b17fdf09f5eda360ae78c7cf8/examples/sentinel-01-minimal.json

# The reference specification, checked against its hash.
curl -fsSL -o spec.json "$SPEC"
echo "03a82a043d459930e254adf5ef24bc5899efe2f5507d29e1c717f315c7bfe7ea  spec.json" | sha256sum -c -

docker pull -q "$IMAGE"
# HOME is the run directory, so the CLI's scratch is private to you (AET-WO-1601).
ae() { docker run --rm --user "$(id -u):$(id -g)" -e HOME=/work -v "$PWD:/work" -w /work "$IMAGE" aethelgard "$@"; }

# Emit the schematic, bill of materials and routed board (exit 0).
emit=$(ae emit --file spec.json --out out); echo "$emit"
grep -q '"ok":true' <<< "$emit"

# ERC on the schematic: exit 0 and a pass verdict.
erc=$(ae check erc --schematic out/sentinel-01.kicad_sch); echo "$erc"
grep -q '"verdict":"pass"' <<< "$erc"

# DRC on the board, judged by the rules the specification declares (board.rules), with schematic parity.
drc=$(ae check drc --board out/sentinel-01.kicad_pcb --spec spec.json --schematic out/sentinel-01.kicad_sch); echo "$drc"
grep -q '"verdict":"pass"' <<< "$drc"
grep -q '"findings":\[\]' <<< "$drc"

# Claim one pin twice (the I2C data line and the flash chip select, both on PB9). solve refuses: exit 2.
docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/work" -w /work "$IMAGE" python3 -I -c '
import json
s = json.load(open("spec.json"))
s["i2c_buses"][0]["sda"]["pin"] = "PB9"
s["spi_devices"][0]["cs"]["pin"] = "PB9"
json.dump(s, open("double.json", "w"))'
rc=0; refused=$(ae solve --file double.json) || rc=$?; echo "$refused"
[ "$rc" -eq 2 ]
grep -q '"kind":"PinClaimedTwice"' <<< "$refused"
grep -q '"claimants":\["i2c_buses\[0\].sda","spi_devices\[0\].cs"\]' <<< "$refused"
echo "quickstart ok"
)
```
<!-- quickstart:end -->

## Running it

**From crates.io, with no image:** `cargo install aethelgard-cli --version 0.2.0 --locked`. `validate` and `solve` are pure Rust and need no external tool; the release reads specification 13:

```bash
aethelgard validate < examples/sentinel-01-minimal.json   # {"ok":true,...}, exit 0
```

Everything that runs KiCad, Renode, ngspice or Kani (`check erc`, `check drc`, the twin, `prove`, `verify`) needs those tools, which the image above carries, and runs them only inside the CLI's own sandbox; without them the CLI refuses with a named kind and a nonzero exit (the codes are in [`crates/aethelgard-cli/README.md`](crates/aethelgard-cli/README.md)). For a checked board, use the image.

From a clone:

Check a specification. It prints one line of JSON and exits 0 when the specification is valid, nonzero otherwise; the codes are in [`crates/aethelgard-cli/README.md`](crates/aethelgard-cli/README.md).

```bash
cargo run -q -p aethelgard-cli -- validate < examples/sentinel-01-minimal.json
```

---

This README points; it does not define. The invariants, the rulings, and the phases each have exactly one home above, and a copy here would be a fourth place for the same fact to drift.
