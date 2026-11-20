#!/usr/bin/env python3
"""AET-WO-4241 §1.1 survey: for every committed specification, which devices
have a part row with an identity, and which of those the twin models
(`renode` not null in crates/aethelgard-ir/data/parts.json; model::kind).
Run from the repository root: python3 -I spike/4241/sh/survey.py"""
import json, subprocess
table = {r['part']: r for r in json.load(open('crates/aethelgard-ir/data/parts.json'))['parts']}
files = subprocess.run(['git', 'ls-files', '*.json'], capture_output=True, text=True, check=True).stdout.split()
rows, specs = [], 0
for f in sorted(files):
    if '/package/' in f or f.endswith(('topology.json', 'solve.json')):
        continue
    try:
        s = json.load(open(f))
    except Exception:
        continue
    if not (isinstance(s, dict) and 'schema_version' in s and 'mcu' in s):
        continue
    specs += 1
    devs = [(d['reference'], d['part'], 'i2c', d.get('bus')) for d in s.get('i2c_devices') or []]
    devs += [(d['reference'], d['part'], 'spi', d.get('bus') or d.get('controller')) for d in s.get('spi_devices') or []]
    for ref, part, kind, bus in devs:
        row = table.get(part)
        ident = bool(row and row.get('identity'))
        modelled = bool(row and row.get('renode'))
        rows.append((f, ref, part, kind, bus, ident, modelled))
print(f'specifications: {specs}')
print('| specification | device | part | bus | identity | modelled |')
print('|---|---|---|---|---|---|')
for f, ref, part, kind, bus, ident, modelled in rows:
    print(f'| `{f}` | {ref} | {part} | {kind} {bus} | {"yes" if ident else "no"} | {"yes" if modelled else "**no**"} |')
un = [r for r in rows if r[5] and not r[6]]
print(f'\ndevices: {len(rows)}; with identity: {sum(r[5] for r in rows)}; with identity and unmodelled: {len(un)}')
print('unmodelled parts with identity:', sorted({r[2] for r in un}))
print('specifications with one:', len({r[0] for r in un}))
print('unmodelled devices without identity (no row):', sorted({(r[2]) for r in rows if not r[5]}))
