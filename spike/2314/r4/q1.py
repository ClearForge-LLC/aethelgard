import sys; sys.path.insert(0,'/home/forge/.cache/aet-2314/r4')
from lib import *
import collections
for p in sys.argv[1:]:
    e=Elf(p); print(p, len(e.syms),'syms; segs',[(hex(a),len(b)) for a,b in e.segs],'loads',[(hex(a),len(b)) for a,b in e.loads])
    for x in e.byname(NAME): print(' ',x)
    for x in e.byname(NAME):
        print('  same addr:',[ (y['name'],y['type']) for y in e.syms if y['value']&~1==x['value']&~1 and y['type']!=0 or (y['value']&~1==x['value']&~1 and y['name'][0]!='$')])
    print(' mapping syms $d:',sum(1 for y in e.syms if y['name']=='$d' or y['name'].startswith('$d.')), 'FUNC:',sum(1 for y in e.syms if y['type']==2))
