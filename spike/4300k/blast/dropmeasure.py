#!/usr/bin/env python3
# Part 2(A): in-memory copies of the two power specs with supply_tier1's stackup minima and a
# drop limit of 5 % of nominal on ONE regulated domain at a time; emit once, extract per domain.
import json,subprocess,os
A='/home/forge/aethelgard-4300k/target/debug/aethelgard'
O='/home/forge/.cache/aet-4300k/blast/drop'
specs={'sentinel-03':'crates/aethelgard-cli/tests/fixtures/sentinel-03.json','sentinel-02-power':'crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json'}
for n,f in specs.items():
    s=json.load(open('/home/forge/aethelgard-4300k/'+f)); st=s['board']['stackup']
    st["outer_copper_min_um"]={"value":30,"source":"test: 1 oz foil plus plating, at least"}
    st["plane_copper_min_um"]={"value":15,"source":"test: 1/2 oz foil, at least"}
    st["plating_min_um"]={"value":18,"source":"test: the fabricator's average hole plating"}
    st["copper_temperature_max_c"]={"value":125,"source":"test: the load's junction maximum"}
    dom={d['id']:d['nominal_millivolts'] for d in s['voltage_domains']}
    d=f'{O}/{n}'; os.makedirs(d,exist_ok=True)
    for reg in sorted({r['output'] for r in s['power']['regulators']}):
        s['power']['drop_limits']=[{"domain":reg,"max_millivolts":dom[reg]*50//1000,"source":"test: 5 % of nominal"}]
        sp=f'{d}/spec-{reg}.json'; json.dump(s,open(sp,'w'),indent=1)
        b=f'{d}/emit/{s["project_name"]}.kicad_pcb'
        if not os.path.exists(b):
            r=subprocess.run([A,'emit','--file',sp,'--out',d+'/emit'],capture_output=True,text=True)
            print(n,'emit rc',r.returncode,r.stdout[:300] if r.returncode else '',flush=True)
            if r.returncode: break
        r=subprocess.run([A,'extract','--board',b,'--spec',sp],capture_output=True,text=True)
        open(f'{d}/extract-{reg}.out','w').write(r.stdout)
        print(n,reg,'limit',s['power']['drop_limits'][0]['max_millivolts'],'mV extract rc',r.returncode,flush=True)
