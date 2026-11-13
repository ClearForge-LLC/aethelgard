#!/bin/bash
# F15: AntShell rewrites its whole in-memory history after every command. The same 1 µs × 10 ms
# lock-step, commands as separate lines (the default) and as one repeated line (ONE_LINE=1).
set -eu
export TMAX_NS=1000
L=/cache/target/release/lockstep
$L lockstep bor3 1000 10000000 /cache/p26/oneline-off
ONE_LINE=1 $L lockstep bor3 1000 10000000 /cache/p26/oneline-on
ONE_LINE=1 $L lockstep bor3 1000 30000000 /cache/p26/oneline-on-30ms
for d in oneline-off oneline-on; do sha256sum /cache/p26/$d/trace-*/trace.cbor; done
