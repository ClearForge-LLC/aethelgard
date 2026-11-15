#!/bin/bash
# Copies KiCad 10.0.7's libraries out of the pinned tools image without starting a container
# (podman create + podman cp: no process runs, so no cgroup and no caps), into ~/.cache/aet-4300d/kicad.
set -eu
REPO=$(cd "$(dirname "$0")/../.." && pwd)
IMG=$(cat "$REPO/container/IMAGE")
OUT=$HOME/.cache/aet-4300d/kicad
export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
c=$(podman create "$IMG" true)
trap 'podman rm -f "$c" >/dev/null' EXIT
for f in symbols/Regulator_Switching.kicad_sym symbols/Regulator_Linear.kicad_sym \
         symbols/Logic_LevelTranslator.kicad_sym symbols/Device.kicad_sym symbols/Amplifier_Current.kicad_sym \
         footprints/Package_SO.pretty footprints/Package_TO_SOT_SMD.pretty footprints/Package_SON.pretty \
         footprints/Package_DFN_QFN.pretty footprints/Inductor_SMD.pretty footprints/Capacitor_SMD.pretty \
         footprints/Resistor_SMD.pretty; do
  mkdir -p "$OUT/$(dirname "$f")"
  podman cp "$c:/usr/share/kicad/$f" "$OUT/$f"
done
podman cp "$c:/usr/share/kicad/symbols/LICENSE.md" "$OUT/symbols/LICENSE.md" 2>/dev/null || echo "no symbols/LICENSE.md"
podman cp "$c:/usr/share/kicad/footprints/LICENSE.md" "$OUT/footprints/LICENSE.md" 2>/dev/null || echo "no footprints/LICENSE.md"
echo "image $IMG"
