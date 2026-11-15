# AET-WO-4300d spike probes (deleted at the build)

Shell and Python 3; the datasheets live outside the repository in `~/.cache/aet-4300d/ds/`, held by sha256 (`ds/SHA256SUMS` there, copied here as `SHA256SUMS`).

- `fetch.sh`: fetches each candidate's datasheet and records HTTP status, bytes and sha256 (`fetch.log`).
- `kicadlib.sh`: copies KiCad's symbol and footprint libraries out of the pinned tools image with `podman create` + `podman cp` (the container never starts, so it needs no cgroup and no caps), then lists regulator and translator symbols (`kicadlib.out`).
