#!/usr/bin/env python3
import json,os,re,collections
O='/home/forge/.cache/aet-4300k/blast/'
p2={r['name']:r for r in json.load(open(O+'part2.json'))}
L=[]
L.append('# AET-WO-4300k blast radius (measured at 521dcd06, debug binary)\n')
L.append('Scratch outputs: `/home/forge/.cache/aet-4300k/blast/<name>/{validate,solve,emit,extract}.{out,err,rc}`. Scripts: `part1.sh`, `part2.py`, `dropmeasure.py`, `mkmd.py` in this directory.\n')
L.append('## Part 1: host baseline, 36 committed specifications\n')
L.append('rc = exit codes validate/solve/emit/extract. Conditions = `conditions` array count printed by extract (identical across boards except where noted); extract `ran` rows: lines=judged count, supply/oscillator/pairs = `off` key when not declared.\n')
L.append('| spec | rc v/s/e/x | refusal kind | extract verdict | conditions | lines judged | findings | emit notes | lifecycle msgs | gates off |')
L.append('|---|---|---|---|---|---|---|---|---|---|')
for n in sorted(os.listdir(O)):
    if not os.path.isdir(O+n) or n in('drop',): continue
    rc=[];kind=''
    obj=[]
    for s in['validate','solve','emit','extract']:
        c=open(f'{O}{n}/{s}.rc').read().strip(); rc.append(c)
        try: o=json.load(open(f'{O}{n}/{s}.out'))
        except Exception: o={}
        obj.append(o)
        if c!='0' and not kind and 'error' in o: kind=s+': '+o['error']['kind']+(' '+str(o['error']['id']) if o['error'].get('id') else '')
    x=obj[3]; ran=x.get('ran',[])
    lines=next((g.get('judged') for g in ran if g['gate']=='extract.lines'),'')
    off=','.join(g['off'] for g in ran if 'off' in g)
    if rc[3]=='1' and not kind: kind='extract: verdict fail (exit 1)'
    L.append(f"| {n} | {'/'.join(rc)} | {kind or '-'} | {x.get('verdict','-')} | {len(x.get('conditions',[])) if x else '-'} | {lines} | {len(x.get('findings',[])) if x else '-'} | {len(obj[2].get('notes',[])) if obj[2] else '-'} | {len(obj[0].get('lifecycle',[])) if obj[0] else '-'} | {off} |")
L.append('')
L.append('Condition templates printed by extract (count of boards): see the sentinel-03 list below; the others are subsets plus per-board `TPn` test-pad lines, the BMP581/ICM42688P declared-capacitance lines, and the oscillator line (hse-crystal-16 only).\n')
x=json.load(open(O+'sentinel-03/extract.out'))
L.append('### sentinel-03 extract `conditions` (verbatim, 15)\n')
for c in x['conditions']: L.append('- '+c)
L.append('\nextract `ran`: `'+json.dumps(x['ran'])+'`; `findings`: `[]`; verdict `pass`.\n')
e=json.load(open(O+'sentinel-03/emit.out'))
L.append('emit `notes` (verbatim):\n')
for c in e['notes']: L.append('- '+c)
L.append('\nemit `capacitance_ladder`: `'+json.dumps(e['capacitance_ladder'])+'`\n')
L.append('validate/solve/emit/extract each print the same one `lifecycle` message (W25Q64JVxIQ NRND, acknowledged in accept_lifecycle); validate prints no "screened"/"not computed" text. Full outputs: `sentinel-03/*.out`.\n')
L.append('### `generate hwdef` on sentinel-03\n')
L.append('`aethelgard solve --file sentinel-03.json > topology.json` (rc 0), then `aethelgard generate hwdef --topology topology.json --out hwdef.dat`: rc 2, stdout `'+open(O+'sentinel-03/hwdef.out').read().strip()+'`. No hwdef.dat written.\n')
L.append('## Part 2: blast radius per rule\n')
L.append('Only two of 36 specifications declare `power`: sentinel-03 and sentinel-02-power. Neither declares `power.drop_limits`. No specification declares any supply stackup minimum (outer_copper_min_um, plane_copper_min_um, plating_min_um, copper_temperature_max_c).\n')
L.append('| spec | A power / drop_limits | B fab_tolerances; fabricator(s) in stackup sources | C battery doubled vs VIN abs max | D bor_level / MCU-rail devices vs falling min | E power.input | F loads per regulated rail |')
L.append('|---|---|---|---|---|---|---|')
for n,r in p2.items():
    d=r['D']
    if len(d)>260: d=d[:200]+' ... '+d[d.find('REFUSED'):][:200]+'...'
    L.append(f"| {n} | {r['A']} | {r['B']} | {r['C']} | {d} | {r['E']} | {r['F']} |")
L.append('')
open('/home/forge/aethelgard-4300k/spike/4300k/blast/BLAST.md','w').write('\n'.join(L))
