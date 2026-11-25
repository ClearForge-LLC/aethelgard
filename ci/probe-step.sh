#!/bin/bash
# AET-WO-1918 (1): the Kani peak and boot probe around a job's tests or verify, in every job that proves harnesses
# (tools-shard, bench-package, bench-package-s03; ci/test-ceiling.sh pins the three).
#   bash ci/probe-step.sh start                  the runner's CPU, disk and memory, then ci/probe.py from the host: each
#                                                Kani harness's kani-driver and cbmc peak, and each Renode boot's wall time
#   bash ci/probe-step.sh judge [--require-peak] stop the probe, print the boots, and judge every peak against
#                                                ci/kani-ceiling (ci/ceiling.sh); a bench job requires a peak
set -u
cd "$(dirname "$0")/.." || exit 1
log=$RUNNER_TEMP/probe.log stop=$RUNNER_TEMP/probe.stop err=$RUNNER_TEMP/probe.err
case ${1:-} in
  start)
    lscpu | grep -E '^(Model name|CPU\(s\)|Thread|Core|Socket)'; nproc; df -h /; free -g
    nohup python3 -I ci/probe.py "$log" "$stop" > "$err" 2>&1 &
    ;;
  judge)
    touch "$stop"; sleep 1
    df -h /; free -g
    grep '^PROBE boot' "$log" || true
    cat "$err" 2>/dev/null || true
    bash ci/ceiling.sh "$log" ci/kani-ceiling ${2:+"$2"}
    ;;
  *) echo "usage: probe-step.sh start | judge [--require-peak]" >&2; exit 2 ;;
esac
