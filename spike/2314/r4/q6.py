import sys,time; sys.path.insert(0,'/home/forge/.cache/aet-2314/r4')
from q4 import *
for p in sys.argv[1:]:
    e=Elf(p); t=time.time(); T=table(e); dt=time.time()-t
    pa=e.byname(NAME)[0]['value']&~1
    fn={}
    for x in e.syms:
        if x['type']==2: fn.setdefault(x['value']&~1,x['name'])
    rk=sorted(((c[0],t_) for t_,c in T.items() if c[1]==c[0] and c[0]>=1),reverse=True)[:3]
    print(p.split('/')[-3:], 'panic',T[pa],'top all-lit:',[(n,hex(a),fn.get(a,'?')[:30]) for n,a in rk],f'full table {dt:.1f}s')
