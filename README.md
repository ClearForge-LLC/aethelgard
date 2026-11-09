# Aethelgard

Generates a circuit board and the firmware that drives it from one specification, and refuses to call either finished until the pair has survived simulated electrical and software faults together, before any board is manufactured.

**Status:** phase P1 of 4 is complete, and the crates are published: `cargo install aethelgard-cli`. From one specification, `aethelgard` does five things:

- solves the pin assignment;
- emits a schematic and a placed, routed board that pass KiCad's ERC and DRC;
- generates a typestate board crate whose register effects Kani proves;
- boots generated firmware in Renode;
- serves all of this over MCP.

P2 has started: analog co-simulation and fault injection. The code is licensed `MIT OR Apache-2.0` (see `LICENSE-MIT` and `LICENSE-APACHE`). KiCad's vendored symbols and footprints, in `crates/aethelgard-kicad-lib`, stay `CC-BY-SA-4.0`. Boards emitted from them carry no obligation.

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
IMAGE=ghcr.io/clearforge-llc/aethelgard@sha256:4c440a2d9e8ebcfbfc5ac49c0d10c7ca3b705a232e0c8895d1e98e0c7764234f
SPEC=https://raw.githubusercontent.com/ClearForge-LLC/aethelgard/3d13f4922b0adddb2f761c3a814c53fcfec61ac8/examples/sentinel-01-minimal.json

# The reference specification, checked against its hash.
curl -fsSL -o spec.json "$SPEC"
echo "acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1  spec.json" | sha256sum -c -

docker pull -q "$IMAGE"
ae() { docker run --rm --user "$(id -u):$(id -g)" -v "$PWD:/work" -w /work "$IMAGE" aethelgard "$@"; }

# Emit the schematic, bill of materials and routed board (exit 0).
emit=$(ae emit --file spec.json --out out); echo "$emit"
grep -q '"ok":true' <<< "$emit"

# ERC on the schematic: exit 0 and a pass verdict.
erc=$(ae check erc --schematic out/sentinel-01.kicad_sch); echo "$erc"
grep -q '"verdict":"pass"' <<< "$erc"

# DRC on the board, judged by the rules the specification implies, with schematic parity.
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

Check a specification. It prints one line of JSON and exits 0 when the specification is valid, nonzero otherwise; the codes are in [`crates/aethelgard-cli/README.md`](crates/aethelgard-cli/README.md).

```bash
cargo run -q -p aethelgard-cli -- validate < examples/sentinel-01-minimal.json
```

---

This README points; it does not define. The invariants, the rulings, and the phases each have exactly one home above, and a copy here would be a fourth place for the same fact to drift.
