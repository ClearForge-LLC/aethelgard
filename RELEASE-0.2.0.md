# Releasing 0.2.0: Scotty's checklist

Run this from your own workstation, never Furnace, with your own crates.io token. `docs/releasing.md` is the procedure behind each step, and `AET-WO-1801` prepared it. Each command is followed by what it should print.

## 0. Before you start

- **The commit.**
  1. Run `git switch main && git pull && git status`. It should say `nothing to commit, working tree clean`.
  2. Run `git log -1 --format=%H`. It should name the commit CI passed on: the 0.2.0 bump commit (`AET-WO-1801`), or a later one.
  3. Run `grep -m1 '^version' Cargo.toml`. It should print `version = "0.2.0"`.
- **The token,** made on crates.io under Account Settings, API Tokens:
  - scopes `publish-new` and `publish-update` (three crates are new: `aethelgard-evolve`, `aethelgard-pincheck`, `aethelgard-renode-models`);
  - crate scope `aethelgard-*`;
  - a short expiry.
- **Log in.** Run `cargo login` and paste the token at the prompt, never as an argument. It should print `Login token for crates-io saved`.
- **The whole release, without uploading.** Run it in a fresh cargo home and target directory, so that no earlier dry run's cached 0.2.0 can stand in for today's (cargo treats a name and version as immutable; `AET-WO-1801` §9 met exactly this):
  ```
  d=$(mktemp -d); CARGO_HOME="$d/home" CARGO_TARGET_DIR="$d/target" cargo publish --workspace --dry-run --locked; echo "exit $?"; rm -rf "$d"
  ```
  It needs no token and downloads the dependencies again. For each of the ten crates it should print `Uploading aethelgard-<name> v0.2.0` and then `warning: aborting upload due to dry run`, and it should exit 0. *(Run on Furnace at the bump with `main` merged, 2026-11-17, in this hermetic form: exit 0 for all ten, with no token.)*

## 1. Publish, in this order

Before each crate from step 4 on, check that its dependencies are live:
`curl -s -o /dev/null -w '%{http_code}\n' https://crates.io/api/v1/crates/<dependency>/0.2.0` should print `200`.

| # | Command | Needs live first |
|---|---|---|
| 1 | `cargo publish -p aethelgard-ir --locked` | none |
| 2 | `cargo publish -p aethelgard-kicad-lib --locked` | none |
| 3 | `cargo publish -p aethelgard-renode-models --locked` | none |
| 4 | `cargo publish -p aethelgard-svd-hal --locked` | 1 |
| 5 | `cargo publish -p aethelgard-eda --locked` | 1, 2 |
| 6 | `cargo publish -p aethelgard-twin --locked` | 1, 3, 4 |
| 7 | `cargo publish -p aethelgard-chaos --locked` | none |
| 8 | `cargo publish -p aethelgard-evolve --locked` | 4, 7 |
| 9 | `cargo publish -p aethelgard-pincheck --locked` | 1 |
| 10 | `cargo publish -p aethelgard-cli --locked` | 1, 4, 5, 6, 7, 8, 9 |

Each should print, in turn:
- `Packaging aethelgard-<name> v0.2.0`, `Verifying …`, `Uploading …`;
- `Uploaded aethelgard-<name> v0.2.0 to registry crates-io`;
- `Waiting for aethelgard-<name> v0.2.0 to be available at registry crates-io`;
- `Published aethelgard-<name> v0.2.0 at registry crates-io`.

**Dry runs.** On Furnace, 2026-11-17, with the bump applied: `cargo publish -p <crate> --dry-run --locked` exited 0 for steps 1, 2 and 3. Steps 4–10 cannot be dry-run one at a time until their dependencies are live, because the dry run resolves them on crates.io. The workspace dry run in step 0 covers all ten. To dry-run one of them anyway, add `--dry-run` once the dependencies in its row answer 200.

**If the first publish of a new name (steps 3, 8, 9) is refused for a rate limit:** nothing was uploaded. Wait the time it names, then run the same command again. This happened once at 0.1.0.

**If a publish times out waiting, or the next one cannot find a dependency:** the index is lagging. Wait until the `curl` check answers `200`, then run the same command again. Never publish a crate twice. See `docs/releasing.md`, "If crates.io's index lags".

## 2. Verify the release

In a clean directory, outside this repository's target directory:

```
cargo install aethelgard-cli --version 0.2.0 --locked --root /tmp/aethelgard-0.2.0
/tmp/aethelgard-0.2.0/bin/aethelgard --version
```

The second command should print `aethelgard 0.2.0`. Then, from a checkout of `main`:

```
/tmp/aethelgard-0.2.0/bin/aethelgard validate < examples/sentinel-01-minimal.json; echo "exit $?"
```

It should print one JSON line starting `{"ok":true,"project_name":"sentinel-01",`, then `exit 0`. *(The same `validate`, run on Furnace from `cargo install --path crates/aethelgard-cli` with the bump applied, gave exit 0.)*

## 3. Afterward

- Revoke the token on crates.io.
- Tell the architect. The README's install line is a docs PR after the publish.

## What 0.2.0 can do that 0.1.0 could not

0.1.0 was published on 2026-11-09, after `AET-WO-1800` (merged as `e9b5b34`), and read specification version 1. 0.2.0 reads specification 13 (`crates/aethelgard-ir/src/lib.rs:68`), so a specification written for today's engine cannot be read by 0.1.0.

**Specification and solving**
- A specification declares each I2C bus's speed. `solve` records it and refuses a bus too fast for its rise time (`AET-WO-1205`), and it refuses a pull-up below the UM10204 sink floor or a bus over its capacitance limit (`AET-WO-1203`).
- `validate` refuses what `solve` and `generate` refuse, with the same error. That includes the SPI, motor-timing and serial-baud limits judged against DS12117 (`AET-WO-1206`, `AET-WO-1207`, `AET-WO-1208`, `AET-WO-4230`).
- A specification can describe SPI devices, motor outputs, serial links and off-board connectors (`AET-WO-4200`, `AET-WO-4300a`, `AET-WO-4300b`, `AET-WO-4300f`). `check speeds` picks pin speeds from a datasheet table and the declared load (`AET-WO-4300c`).
- A specification can declare a power tree of cited power parts. The solver judges hot-plug, the buck's junction temperature, capacitor DC bias, the brown-out threshold and control pins (`AET-WO-4300d`, `AET-WO-4300d3`).
- Parts carry lifecycle status, and a part not recommended for new designs must be acknowledged (`AET-WO-4210`, `AET-WO-4211`).

**Board generation**
- Boards are emitted and routed at 2 or 4 layers from the declared stackup (`AET-WO-4100`).
- The router escapes under dense parts, enforces courtyards, judges zones, and supports net classes, vias, coupled pairs, rotation and sub-grid escape (`AET-WO-1307`, `AET-WO-1308`, `AET-WO-1310`). Through-hole pads and thermal-via arrays are emitted and judged (`AET-WO-1309`).
- `aethelgard-kicad-lib` carries the project's own symbols and footprints for the ICM-42688-P and BMP581, cited pin by pin, alongside KiCad's. It is now `CC-BY-SA-4.0 AND (MIT OR Apache-2.0)`. A part's declared pin capacitance (specification 13) is counted by `extract` as declared, never as a maximum (`AET-WO-4220b`).
- The power tree is drawn and routed on the board (`AET-WO-4300d2`). The MCU's support network is drawn, and `check support` judges it from KiCad's netlist (`AET-WO-4300g`).

**Firmware and the twin**
- `generate`, `build` and `verify` write the typestate board crate, build it and prove it (`AET-WO-3301`).
- Generated firmware talks to sensor models in Renode through the analog bridge (`AET-WO-2200`, `AET-WO-2202`, `AET-WO-2203`). The twin ships its own C# models, the ICM-42688-P and BMP581, in the new crate `aethelgard-renode-models` (`AET-WO-4220`).
- Motor outputs and serial links are witnessed in virtual time (`AET-WO-4300a`, `AET-WO-4300b`), and the boot judge resists a firmware that fakes its boot word (`AET-WO-2204`).

**Fault campaigns and evolution**
- `campaign` archives runs as CBOR, and `replay` recomputes a run's hash from the archive alone (`AET-WO-2300`, `AET-WO-2205`, `AET-WO-3201`).
- `attribute` names a failure as electrical, software, none or indeterminate (`AET-WO-3100`). `propose` makes a rules-based fix and re-gates it, in the newly published crate `aethelgard-evolve` (`AET-WO-3200`).

**Verification and fabrication**
- `extract` bounds each signal line's capacitance from the routed board (`AET-WO-1305`).
- `verify --fab-out` writes the fabrication package its verdict names (`AET-WO-3300`).
- Every KiCad file a gate reads goes through one strict reader (`AET-WO-1306`, `AET-WO-1311`), and tools run behind a uid boundary (`AET-WO-1601`, `AET-WO-1603`).
- `check pinout` judges an ArduPilot ELF's pins against the topology, in the new crate `aethelgard-pincheck` (`AET-WO-4400a`, `AET-WO-4401`).

**Images**
- The published tools and ArduPilot images are pinned snapshots, and each carries its written source offer. Its GPL and LGPL sources are mirrored next to it (`AET-WO-4403`, `RUL-21`).

P3's exit gate still waits on bench rows, so this release claims no phase beyond P2 as closed.
