# AET-WO-4241 §1.1 — predictions, batch 2 (written before C9 ran)

C0's `spike/4600/logger-spi-only.json` is refused before generation (`UnsupportedSchemaVersion`: 16, not 19), so
it is no control. C9 replaces it with `4600`'s pair: `spec/sentinel-01-minimal-no-u3.json`, which is
`examples/sentinel-01-minimal.json` with U3 (TMP117) removed. No other field names U3.

1. **C9, old against new:** the ELF and `main.rs` are byte-identical, because no device is unmodelled.
2. **The `4600` pair:** with U3 and without it, the old generator gives byte-identical ELFs (`4600` F19), and
   the new one gives two different ELFs.
3. **C9's verdicts:** `pass` under both, with no `device_unmodelled`.
