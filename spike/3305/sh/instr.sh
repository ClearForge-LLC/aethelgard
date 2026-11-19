#!/bin/bash
# Inside the image, REPO=instr: a scratch CLI that prints the last boot's Renode log (RAWLOG) to stderr;
# the two two-bme runs again (held-low-sda on the reference firmware, baseline on the wrong-address mutant).
set -u
cd /repo && cargo build -q --locked -p aethelgard-cli || exit 9
B=/cache/target-instr/debug; A=$B/aethelgard; export PATH=$B:$PATH; cp /cache/target/debug/aethelgard-bridge $B/
O=/cache/m/two-bme; mkdir -p $O/raw; cd $O/sc
$A campaign --scenarios held-low-sda.json --firmware $O/fw/ref.elf --archive $O/raw/ref > $O/raw/held.jsonl 2> $O/raw/held.err; echo "held rc=$?"
$A campaign --scenarios wrong-address.json --firmware $O/fw/mut.elf --archive $O/raw/mut > $O/raw/mut.jsonl 2> $O/raw/mut.err; echo "mut rc=$?"
