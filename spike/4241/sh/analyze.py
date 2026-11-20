#!/usr/bin/env python3
"""AET-WO-4241 §1.1: summarise the kit's runs (~/.cache/aet-4241/m) into out/runs.md.
Usage: python3 -I spike/4241/sh/analyze.py <m-dir>"""
import json, os, re, sys
M = sys.argv[1]
STATUS = {0x22020000: 'ok', 0x22020001: 'timeout_txis', 0x22020002: 'timeout_tc', 0x22020003: 'timeout_rxne',
          0x22020004: 'timeout_stop', 0x22020005: 'timeout_rxp', 0x22020006: 'timeout_eot', 0xFFFFFFFF: 'unset'}
def rd(p, d=''):
    try: return open(p).read().strip()
    except OSError: return d
def finds(v, key, out):
    if isinstance(v, dict):
        for k, x in v.items():
            if k == key: out.append(x)
            finds(x, key, out)
    elif isinstance(v, list):
        for x in v: finds(x, key, out)
    return out
print('# AET-WO-4241 §1.1 runs\n')
for c in sorted(os.listdir(M)):
    if not os.path.isdir(f'{M}/{c}'): continue
    shas = {}
    for tag in ('old', 'new'):
        d = f'{M}/{c}/{tag}'
        if not os.path.isdir(d): continue
        sha = dict((os.path.basename(l.split()[1]), l.split()[0][:16]) for l in rd(f'{d}/sha256').splitlines() if l.strip())
        shas[tag] = sha
        print(f'## {c}/{tag}\n')
        print(f'- generate rc {rd(d+"/generate.rc")}, build rc {rd(d+"/build.rc")}, simulate rc {rd(d+"/simulate.rc")}; elf {sha.get("fw.elf")}, main.rs {sha.get("main.rs")}')
        if rd(d+'/generate.rc') not in ('0', ''):
            print('- generate: `' + rd(d+'/generate.out')[:400].replace('\n', ' ') + '`')
        main = rd(f'{d}/gen/board/firmware/src/main.rs')
        if main:
            calls = re.findall(r'(i2c\d|spi\d)\.transfer\(&mut r, ([^,]+), &\[([^\]]*)\]', main)
            print(f'- reads in main.rs ({len(calls)}): ' + '; '.join(f'{b} {t} [{w}]' for b, t, w in calls))
        try: v = json.load(open(f'{d}/simulate.json'))
        except Exception: v = None
        if v:
            fs = [(f.get('kind'), f.get('severity')) for f in v.get('findings', [])]
            print(f'- verdict **{v.get("verdict")}**; findings: {fs}')
            for f in v.get('findings', []):
                if f.get('severity') != 'info' or f.get('kind') == 'device_unmodelled':
                    print(f'  - `{f.get("kind")}`: {f.get("message", f.get("detail", ""))[:300]}')
            ab = finds(v, 'absent', [])
            if ab: print(f'- `absent` fields in the verdict: {ab}')
        log = rd(f'{d}/boot.log').splitlines()
        uk = [l for l in log if 'Unknown slave' in l]
        tr = [l for l in log if 'Tried to transmit' in l or 'not connected' in l]
        ours = lambda tag: [l[1:20] + ' ' + l.split('AETHELGARD-', 1)[1] for l in log if '] [WARNING] sysbus: AETHELGARD-' + tag in l]
        bw, st = ours('BOOTWORD'), ours('STEP')
        print(f'- boot log lines {len(log)}; `Unknown slave`: {uk}')
        if tr: print(f'- multiplexer unrouted ({len(tr)}): {tr[:4]}{" …" if len(tr) > 4 else ""}')
        print(f'- BOOTWORD: {bw}')
        print(f'- STEP hooks: {st}')
        ram = [l.split() for l in rd(f'{d}/ram.txt').splitlines()]
        words = [(int(a, 16), int(w, 16)) for _, a, w in ram]
        if words:
            base = words[0][0]
            show = [f'+{a-base:#x}={w:#010x}{"("+STATUS[w]+")" if w in STATUS else ""}' for a, w in words[:20] if w != 0xFFFFFFFF or a - base < 8]
            print(f'- RAM from the boot word: {" ".join(show)}')
        print()
    if 'old' in shas and 'new' in shas:
        print(f'**{c}: ELF old == new: {shas["old"].get("fw.elf") == shas["new"].get("fw.elf")}; main.rs equal: {shas["old"].get("main.rs") == shas["new"].get("main.rs")}**\n')
