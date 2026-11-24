import sys,time,bisect,collections,struct; sys.path.insert(0,'/home/forge/.cache/aet-2314/r4')
from lib import *
def is32(hw): return (hw>>11) in (0x1D,0x1E,0x1F)
def build(e):
    img=bytearray(); base=e.segs[0][0]; b=e.segs[0][1]
    funcs=sorted({(x["value"]&~1):(x["value"]&~1,x["size"],x["name"]) for x in e.syms if x["type"]==2 and x["size"]>0}.values())
    # data ranges from mapping symbols within the image
    ms=sorted((x['value']&~1,x['name'][:2]) for x in e.syms if x['name'] in('$d','$t','$a') or x['name'][:3] in('$d.','$t.','$a.'))
    return base,b,funcs,ms
def sweep(e,use_map):
    base,b,funcs,ms=build(e); msa=[m[0] for m in ms]
    hits=collections.defaultdict(list); instrs=0
    for fa,sz,nm in funcs:
        end=fa+sz; a=fa
        while a<end:
            if use_map:
                i=bisect.bisect_right(msa,a)-1
                if i>=0 and ms[i][1]=='$d':
                    # skip to next mapping symbol
                    a=msa[i+1] if i+1<len(msa) else end
                    continue
            o=a-base
            if o+2>len(b): break
            hw1=b[o]|b[o+1]<<8
            if is32(hw1) and o+4<=len(b):
                hw2=b[o+2]|b[o+3]<<8; instrs+=1
                r=decode_branch(hw1,hw2)
                if r: hits[(a+4+r[1])&0xFFFFFFFF].append((a,r[0],nm))
                a+=4
            else: a+=2; instrs+=1
    return hits,instrs
if __name__=='__main__':
    for p in sys.argv[1:]:
        e=Elf(p); pa=e.byname(NAME)[0]['value']&~1
        t=time.time(); nv=naive_scan(e); t_n=time.time()-t
        t=time.time(); sw,ni=sweep(e,False); t_s=time.time()-t
        t=time.time(); sm,ni2=sweep(e,True); t_m=time.time()-t
        funcranges=sorted((x['value']&~1,(x['value']&~1)+x['size']) for x in e.syms if x['type']==2 and x['size']>0)
        fs=[f[0] for f in funcranges]
        def infunc(a):
            i=bisect.bisect_right(fs,a)-1; return i>=0 and a<funcranges[i][1]
        print('==',p,'image',len(e.segs[0][1]),'bytes; panic',hex(pa))
        for k in ('BL','B.W'):
            nvk=[h for h in nv.get(pa,[]) if h[1]==k]
            swk=[h for h in sw.get(pa,[]) if h[1]==k]; smk=[h for h in sm.get(pa,[]) if h[1]==k]
            nin=sum(infunc(h[0]) for h in nvk)
            print(f' {k}: naive={len(nvk)} (in FUNC range {nin}) sweep-nomap={len(swk)} sweep-with-$d-map={len(smk)}')
        print(f' times: naive {t_n:.2f}s sweep {t_s:.2f}s sweep+map {t_m:.2f}s; instrs decoded {ni2}')
        # false positive: naive hits not in map-sweep
        sset={h[0] for h in sm.get(pa,[])}; nset={h[0] for h in nv.get(pa,[])}
        print(' naive-only (not in precise sweep):',[hex(a) for a in sorted(nset-sset)][:10], 'sweep-only:',[hex(a) for a in sorted(sset-nset)][:5])
        # ranking: call-site counts of BL targets that are FUNC starts
        cnt=collections.Counter()
        for tgt,l in sm.items():
            c=sum(1 for h in l if h[1]=='BL')
            if c: cnt[tgt]=c
        pc=cnt[pa]; ge=sum(1 for t,c in cnt.items() if c>=pc)
        print(f' BL-target funcs total {len(cnt)}; panic BL count {pc}; funcs with >= that many BL sites: {ge} (incl panic); rank {sorted(cnt.values(),reverse=True).index(pc)+1}')
        print(' top5:',[(hex(t),c) for t,c in cnt.most_common(5)])
        # functions with zero callers
        allf=[f[0] for f in funcranges]; zero=sum(1 for f in allf if f not in cnt and f not in {t for t,l in sm.items()})
        print(' FUNC symbols total',len(allf),'with zero direct (BL/B.W) callers',zero)
        # indirect: words
        n=0; img=e.segs[0][1]; base=e.segs[0][0]
        t=time.time()
        wl=[]
        needle=struct.pack('<I',pa|1)
        i=img.find(needle)
        while i>=0:
            if (i%4==0): wl.append(base+i)
            i=img.find(needle,i+1)
        # all loads
        allw=0
        for va,bb in e.loads:
            j=bb.find(needle)
            while j>=0: allw+=1; j=bb.find(needle,j+1)
        print(f' words==panic|1: exec-seg aligned {len(wl)} ; any-alignment all loads {allw} ({time.time()-t:.3f}s)', [hex(x) for x in wl[:5]])
