import sys,collections,struct,shutil; sys.path.insert(0,'/home/forge/.cache/aet-2314/r4')
from lib import *
from q3 import insns,r0_source,cstr
SRC='/home/forge/.cache/aet-2314/ap/control.elf'
def table(e):
    T=collections.defaultdict(lambda:[0,0])  # target -> [bl sites, sites w/ literal str in r0]
    for nm,L in insns(e):
        for k,(a,sz,h1,h2) in enumerate(L):
            if sz==4:
                r=decode_branch(h1,h2)
                if r:
                    t=(a+4+r[1])&0xFFFFFFFF
                    T[t][0]+=1
                    if r[0]=='BL':
                        kind,v=r0_source(e,L,k)
                        if v and cstr(e,v): T[t][1]+=1
    return T
def check(path,label,T=None):
    e=Elf(path); s=e.byname(NAME)
    pa=s[0]['value']&~1; T=table(e)
    n,lit=T[pa]
    # in-FUNC?
    f=[x for x in e.syms if x['type']==2 and x['value']&~1==pa]
    print(f'{label}: hook addr {pa:#x} callers={n} literal-r0={lit} funcs-at-addr={[x["name"][:40] for x in f]}')
    return e,T
def patch(dst,newaddr_val,rename=None):
    shutil.copy(SRC,dst); d=bytearray(open(dst,'rb').read()); e=Elf(SRC); x=e.byname(NAME)[0]
    struct.pack_into('<I',d,x['off']+4,newaddr_val)
    struct.pack_into('<I',d,x['off']+8,0) if False else None
    open(dst,'wb').write(d)
if __name__=='__main__':
    e=Elf(SRC); T=table(e)
    pa=e.byname(NAME)[0]['value']&~1; print('real panic',T[pa])
    funcs={}
    for x in e.syms:
        if x['type']==2 and x['size']>0: funcs.setdefault(x['value']&~1,x)
    ge=[(t,c) for t,c in T.items() if t in funcs and c[0]>=T[pa][0]]
    print('functions with >= real panic site count (56):',len(ge))
    ge_lit=[(t,c) for t,c in ge if c[1]==c[0]]
    print(' of those, ALL BL callers pass literal str in r0:',[(hex(t),c,funcs[t]['name'][:40]) for t,c in ge_lit])
    ge_any=[(t,c) for t,c in ge if c[1]>=0.5*c[0]]
    print(' of those, >=50% literal:',[(hex(t),c,funcs[t]['name'][:40]) for t,c in ge_any])
    # any function with >=10 callers & all literal
    big=[(t,c,funcs[t]['name'][:50]) for t,c in T.items() if t in funcs and c[0]>=10 and c[1]==c[0]]
    print('functions >=10 callers, all literal r0:',len(big),[(hex(t),c,n) for t,c,n in big])
    one=[(t,c,funcs[t]['name'][:50]) for t,c in T.items() if t in funcs and c[0]>=1 and c[1]==c[0]]
    print('functions >=1 caller & all callers literal r0:',len(one),'of',sum(1 for t in T if t in funcs),'called funcs')
    zero=[t for t in funcs if t not in T]
    print('zero-caller funcs:',len(zero))
    import json; json.dump({'zero':zero[:5],'big':[(t,c,n) for t,c,n in big],'one':[(t,c,n) for t,c,n in one][:20]},open('/home/forge/.cache/aet-2314/r4/q4.json','w'))
    # sizes: panic body
    base=e.segs[0][0]; b=e.segs[0][1]
    body=b[pa-base:pa-base+76]; print('panic body hw:',' '.join(f'{body[i]|body[i+1]<<8:04x}' for i in range(0,76,2)))
