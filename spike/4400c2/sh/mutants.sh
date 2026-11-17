#!/bin/bash
# Inside the tools image: the targeted adversarial pass's mutants, one per guard 4400c2 adds, each
# run against the twin crate's tests (lib). Each file is restored from a copy taken first.
set -u
T=/repo/crates/aethelgard-twin
B=/cache/mut-backup; rm -rf $B; mkdir -p $B; cp $T/data/dmamux1-spi-rx.tsv $T/src/platform.rs $B/
restore() { cp $B/dmamux1-spi-rx.tsv $T/data/; cp $B/platform.rs $T/src/; }
run() { local name=$1; shift; "$@"; local out; out=$(cargo test --locked --offline -p aethelgard-twin --lib 2>&1)
  local f; f=$(grep -E '^test .* FAILED$' <<< "$out" | sed -E 's/^test (.*) \.\.\. FAILED$/\1/' | tr '\n' ' ')
  if grep -q '^test result: ok' <<< "$out"; then echo "SURVIVED $name"; else echo "KILLED $name by: ${f:-$(grep -m1 -E 'error' <<< "$out")}"; fi
  restore; }
run M1-swap-37-39 sed -i 's/\t37$/\tXX/; s/\t39$/\t37/; s/\tXX$/\t39/' $T/data/dmamux1-spi-rx.tsv
run M2-citation-removed sed -i '/RM0433/d' $T/data/dmamux1-spi-rx.tsv
run M3-spi6-guessed sed -i 's/    if peripheral == "SPI6" {\n        return Ok(None);/X/; s/return Ok(None);/return Ok(Some(11));/' $T/src/platform.rs
run M4-line-dropped sed -i 's/                format!("    DMARecieve -> dmamux1@{n}\\n"),/                { let _ = n; String::new() },/' $T/src/platform.rs
run M5-parser-loose sed -i 's/if !n.bytes().all(|b| b.is_ascii_digit()) {/if false {/' $T/src/platform.rs
