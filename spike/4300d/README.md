# AET-WO-4300d spike probes (deleted at the build)

Shell and Python 3; the datasheets live outside the repository in `~/.cache/aet-4300d/ds/`, held by sha256 (`ds/SHA256SUMS` there, copied here as `SHA256SUMS`).

- `fetch.sh`: fetches each candidate's datasheet and records HTTP status, bytes and sha256 (`fetch.log`).
- `kicadlib.sh`: copies KiCad's symbol and footprint libraries out of the pinned tools image with `podman create` + `podman cp` (the container never starts, so it needs no cgroup and no caps), and `kicadlist.py` lists the candidates' pins, default footprints and pad kinds (`kicadlib.out`).
- `checks.py` → `checks.out`: S3's static checks on the proposed tree (`power-shape.json`), guaranteed datasheet values only, each cited inline.
- `power-shape.json`: S2's proposed `power` section, illustrative.
- `read-bucks.md`, `read-ldo-xlat.md`: two delegated datasheet reads (subagents), every ambiguous table rendered. FEEDBACK names the rows spot-checked by hand.
- `SHA256SUMS`: the held datasheets (`~/.cache/aet-4300d/ds/`).
- `status.log`: product-status reads (URL, page sha256, result).
