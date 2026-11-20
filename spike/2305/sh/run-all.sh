#!/bin/bash
# AET-WO-2305 §1.1: every run, inside the tools image (sh/img.sh, 8G). Each board's campaign holds its
# undisturbed boot first (spike/2305/scenarios/*-undisturbed.json, no override), then its droops.
set -u
R=/repo/spike/2305/sh/run.sh; S=spike/2305/scenarios
bash $R s01min examples/sentinel-01-minimal.json $S/s01min-undisturbed.json scenarios/sentinel-01/droop-bor3.json scenarios/sentinel-01/droop-tolerated.json
bash $R s01bench examples/sentinel-01.json $S/s01bench-undisturbed.json scenarios/sentinel-01-bench/droop-bor3.json
bash $R dshot600 crates/aethelgard-cli/tests/fixtures/motors-dshot600.json $S/dshot600-undisturbed.json scenarios/motors/dshot600-droop.json
bash $R four-layer crates/aethelgard-cli/tests/fixtures/four-layer.json $S/four-layer-undisturbed.json crates/aethelgard-cli/tests/fixtures/four-layer-droop.json
bash $R spi-only crates/aethelgard-cli/tests/fixtures/spi-only.json $S/spi-only-undisturbed.json crates/aethelgard-cli/tests/fixtures/spi-only-droop-bor3.json
bash $R s03 crates/aethelgard-cli/tests/fixtures/sentinel-03.json $S/s03-undisturbed.json scenarios/sentinel-03/droop-tolerated.json
bash $R s02 crates/aethelgard-cli/tests/fixtures/sentinel-02-shaped.json $S/s02-undisturbed.json $S/s02-droop.json
bash $R logger spike/2305/boards/logger.json $S/logger-undisturbed.json $S/logger-droop-bor3.json
bash $R ext-bmp581 examples/sentinel-01-ext-bmp581.json $S/ext-bmp581-undisturbed.json $S/ext-bmp581-droop-bor3.json
