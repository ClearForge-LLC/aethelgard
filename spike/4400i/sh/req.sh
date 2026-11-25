#!/bin/bash
# AET-WO-4400i (1) P2: does emitting each declared serial port's receive request line move a committed verdict?
# Run in the tools image with /repo = main's build (LABEL main) or the patched one (LABEL req,
# patches/rx-request-all-declared.diff). Writes /cache/req/LABEL/:
#   verify.json      ci.yml bench-package's own command, `verify ... --against-fab fab/sentinel-01-bench/package`
#   repl.tsv         every committed example's and fixture's emitted platform sha256 (generate)
#   s02.json, s03.json  `simulate --foreign 5000ms` on sentinel-02-own (CI ELF 5eace4ca) and Sentinel-03 (dbf598d8)
set -u
L=$1; O=/cache/req/$L; rm -rf "$O"; mkdir -p "$O/gen"
A=target/debug/aethelgard
for s in examples/*.json crates/aethelgard-cli/tests/fixtures/sentinel-0*.json /cache/boards/s03/sentinel-03-foreign.json; do
  n=$(basename "$s" .json); $A generate --spec "$s" --out "$O/gen/$n" > "$O/gen/$n.json" 2>&1
  r=$(ls "$O/gen/$n"/board/*.repl 2>/dev/null | head -1)
  printf '%s\t%s\n' "$s" "$([ -n "$r" ] && sha256sum < "$r" | cut -c1-64 || echo none)" >> "$O/repl.tsv"
done
$A simulate --platform "$O/gen/sentinel-02-own/board/sentinel-02-own.repl" --spec crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json \
  --firmware /cache/boards/s02/arducopter.elf --foreign 5000ms > "$O/s02.json" 2> "$O/s02.err"; echo "s02 exit $?" >> "$O/exit.txt"
$A simulate --platform "$O/gen/sentinel-03-foreign/board/sentinel-03.repl" --spec /cache/boards/s03/sentinel-03-foreign.json \
  --firmware /cache/boards/s03/arducopter.elf --foreign 5000ms > "$O/s03.json" 2> "$O/s03.err"; echo "s03 exit $?" >> "$O/exit.txt"
$A verify --spec examples/sentinel-01.json \
  --scenarios scenarios/sentinel-01-bench/pullup-3k3.json scenarios/sentinel-01-bench/droop-bor3.json \
  scenarios/sentinel-01-bench/spi-absent-u4.json scenarios/sentinel-01-bench/pullup-2k2.json \
  scenarios/sentinel-01-bench/serial-rx-open-console.json --against-fab fab/sentinel-01-bench/package > "$O/verify.json" 2> "$O/verify.err"
echo "verify exit $?" >> "$O/exit.txt"
cat "$O/exit.txt"
