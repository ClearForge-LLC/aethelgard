#!/usr/bin/env python3
# Part 2 static columns (A declared, B fabricators, C battery, D BOR vs device VCC min, E input, F loads).
import json,subprocess,re
R='/home/forge/aethelgard-4300k/'
rows=[l.split('\t') for l in subprocess.check_output(['bash',R+'spike/4300k/universe.sh'],text=True,cwd=R).splitlines()]
parts={p['part']:p for p in json.load(open(R+'crates/aethelgard-ir/data/parts.json'))['parts']}
pp={p['part']:p for p in json.load(open(R+'crates/aethelgard-ir/data/power-parts.json'))['parts']} if isinstance(json.load(open(R+'crates/aethelgard-ir/data/power-parts.json'))['parts'],list) else json.load(open(R+'crates/aethelgard-ir/data/power-parts.json'))['parts']
BORF={0:1580,1:1950,2:2250,3:2540}
FAB=['JLCPCB','OSH Park','PCBWay','Eurocircuits']
def walk(o,path=''):
    if isinstance(o,dict):
        if isinstance(o.get('source'),str): yield path,o['source']
        for k,v in o.items(): yield from walk(v,path+'.'+k)
    elif isinstance(o,list):
        for i,v in enumerate(o): yield from walk(v,path+f'[{i}]')
def pmin(part):
    for k,v in parts.items():
        if part.startswith(k) or k.startswith(part): return k,v['supply_millivolts']['min']
    return part,None
out=[]
for n,f in rows:
    s=json.load(open(R+f)); m=s['mcu']; st=s['board']['stackup']; p=s.get('power')
    r={'name':n}
    r['A']=('power' if p else 'no power')+(' / drop_limits' if p and p.get('drop_limits') else (' / NO drop_limits' if p else ''))
    fabs=sorted({w for _,t in walk(st) for w in FAB if w.lower() in t.lower()})
    r['B']=('fab_tolerances; ' if 'fab_tolerances' in st else 'no fab_tolerances; ')+('/'.join(fabs) if fabs else 'no fabricator named')+(' (MULTI)' if len(fabs)>1 else '')
    if p:
        b=p['battery']; pack=b['cells']['max']*b['cell_max_millivolts']['value']
        vins=[]
        for g in p['regulators']:
            ab=pp[g['part']]['kind']['buck' if 'buck' in pp[g['part']]['kind'] else list(pp[g['part']]['kind'])[0]].get('vin_abs_max_millivolts',{}).get('value') if g['part'] in pp else None
            vins.append((g['reference'],g['part'],g['input'],ab))
        bad=[v for v in vins if v[2]=='battery' and v[3] is not None and 2*pack>v[3]]
        r['C']=f'pack max {b["cells"]["max"]} x {b["cell_max_millivolts"]["value"]} = {pack} mV, doubled {2*pack} mV; VIN abs max '+', '.join(f'{a} {c} {d}' for a,c,i,d in vins)+('; REFUSED: '+','.join(x[0] for x in bad) if bad else '; ok')
        r['E']='power.input '+p['input']['reference']+' '+p['input']['part']+' hot_plug '+str(len(p['input'].get('hot_plug',[])))
        outs=[g['output'] for g in p['regulators']]
        r['F']=', '.join(f'{o}: {sum(1 for l in p["loads"] if l["domain"]==o)} loads ({sum(l["milliamps"] for l in p["loads"] if l["domain"]==o)} mA)' for o in outs)
    else: r['C']=r['E']=r['F']='-'
    bor=m['bor_level']; fall=BORF[bor]
    devs=[]
    for d in s.get('i2c_devices',[])+s.get('spi_devices',[]):
        if d['supply']==m['supply']:
            k,mn=pmin(d['part']); devs.append((d['reference'],d['part'],mn))
    fails=[d for d in devs if d[2] is not None and d[2]>fall]
    unk=[d for d in devs if d[2] is None]
    r['D']=f'bor_level {bor} (falling min {fall} mV); MCU rail {m["supply"]}; devices '+(', '.join(f'{a} {b} {c}' for a,b,c in devs) or 'none')+('; REFUSED: '+','.join(f'{a}({c}>{fall})' for a,_,c in fails) if fails else '; ok')+('; no VCC min held: '+','.join(f'{a} {b}' for a,b,_ in unk) if unk else '')
    r['_fails']=[d[0] for d in fails]; r['_fp']=[(d[1],d[2]) for d in fails]
    out.append(r)
json.dump(out,open('/home/forge/.cache/aet-4300k/blast/part2.json','w'),indent=1)
for r in out: print(r['name'],'|',r['A'],'|',r['B'],'|',r['C'],'|',r['D'],'|',r['E'],'|',r['F'])
