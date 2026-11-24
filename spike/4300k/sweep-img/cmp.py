import json,os,re,sys
O='/home/forge/.cache/aet-4300k/sweep-img/out'; idx=json.load(open('/home/forge/.cache/aet-4300k/sweep-img/index.json'))
IDK=('part','role','pin','net','ic','rule','code','type','description','id','name','check','domain','ref')
SKIP=re.compile(r'(^|_)(sha256|path)$|^path$|elapsed|duration|gap_um|^pads$|_um$|^distance|^pcb_|^schematic$|^input$')
def load(d,g):
    p=f'{O}/{d}/{g}.out'
    if not os.path.exists(p): return None
    return [json.loads(l) for l in open(p) if l.strip().startswith('{')]
def flat(v,p,out):
    if isinstance(v,dict):
        for k,x in v.items():
            if SKIP.search(k): continue
            flat(x,p+'.'+k,out)
    elif isinstance(v,list):
        seen={}
        for i,x in enumerate(v):
            if isinstance(x,dict):
                key=','.join(str(x[k])[:60] for k in ('role','part','pin','net','ic','code','type','id','name','check','domain') if k in x) or str(i)
                if isinstance(x.get('rule'),str) and not key: key=x['rule'][:60]
            else: key=str(x)[:100]
            seen[key]=seen.get(key,0)+1
            flat(x,f'{p}[{key}#{seen[key]}]',out)
        out[p+'#len']=len(v)
    else:
        out[p]=v
def facts(d,g):
    r=load(d,g)
    if r is None: return None
    o={}
    for i,x in enumerate(r): flat(x,f'{g}:{i}',o)
    return o
def rc(d,g):
    p=f'{O}/{d}/{g}.rc'; return open(p).read().strip() if os.path.exists(p) else '-'
base={'sentinel-03':'base-sentinel-03','sentinel-03+gated':'base-gated'}
B={c:{g:facts(b,g) for g in('erc','drc','support')} for c,b in base.items()}
res=[]
for e in idx:
    d=e['id']; row=dict(e); row['rc']={g:rc(d,g) for g in('validate','emit','erc','drc','support')}; row['less']={}; row['new']={}
    for g in('erc','drc','support'):
        f=facts(d,g); b=B[e['carrier']][g]
        if f is None: row['less'][g]=['NOT RUN (emit rc %s)'%row['rc']['emit']]; continue
        row['less'][g]=[f'gone {k} = {str(v)[:160]}' for k,v in b.items() if k not in f]+[f'chg {k}: {str(v)[:100]} -> {str(f[k])[:100]}' for k,v in b.items() if k in f and f[k]!=v]
        row['new'][g]=[f'new {k} = {str(v)[:160]}' for k,v in f.items() if k not in b]
    res.append(row)
json.dump(res,open('/home/forge/.cache/aet-4300k/sweep-img/cmp.json','w'),indent=1)
for r in res:
    print(r['id'],r['carrier'],r['path'],r['variant'],r['rc'],{g:len(r['less'][g]) for g in r['less']},{g:len(r['new'].get(g,[])) for g in r['new']})
