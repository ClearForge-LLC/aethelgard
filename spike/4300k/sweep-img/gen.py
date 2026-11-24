import json,glob,copy,re,os
R='/home/forge/worktrees/aethelgard/4300k-snap'
src=open(R+'/spike/4300k/sweep/sweep.py').read()
# copy mutation functions verbatim
ns={'json':json,'copy':copy,'re':re,'os':os}
a=src.index('# ---------- path handling'); b=src.index('# ---------- running')
exec(src[a:b],ns)
s03=json.load(open(R+'/crates/aethelgard-cli/tests/fixtures/sentinel-03.json'))
carr={'sentinel-03':s03,'sentinel-03+gated':ns['gated'](s03)}
sel={}
for f in sorted(glob.glob('/home/forge/.cache/aet-4300k/sweep/results*.json')):
    for x in json.load(open(f)):
        if x.get('carrier') in carr and x.get('exits',{}).get('validate')==0:
            sel[(x['carrier'],x['path'],x['variant'])]=x
idx=[{'id':'base-sentinel-03','carrier':'sentinel-03','path':None,'variant':None},{'id':'base-gated','carrier':'sentinel-03+gated','path':None,'variant':None}]
json.dump(carr['sentinel-03'],open('/home/forge/.cache/aet-4300k/sweep-img/v/base-sentinel-03.json','w'))
json.dump(carr['sentinel-03+gated'],open('/home/forge/.cache/aet-4300k/sweep-img/v/base-gated.json','w'))
for i,(k,x) in enumerate(sorted(sel.items())):
    sp=ns['apply'](carr[k[0]],k[1],k[2])
    if sp is None: print('NA',k); continue
    n='v%03d'%i; json.dump(sp,open(f'/home/forge/.cache/aet-4300k/sweep-img/v/{n}.json','w'))
    idx.append({'id':n,'carrier':k[0],'path':k[1],'variant':k[2]})
json.dump(idx,open('/home/forge/.cache/aet-4300k/sweep-img/index.json','w'),indent=1)
print(len(idx))
