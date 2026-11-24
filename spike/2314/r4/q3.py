import sys,collections,struct,bisect; sys.path.insert(0,'/home/forge/.cache/aet-2314/r4')
from lib import *
from q2 import build,is32
def rd(e,addr,n):
    for va,b in e.loads:
        if va<=addr<va+len(b): return b[addr-va:addr-va+n]
    return None
def cstr(e,addr):
    for va,b in e.loads:
        if va<=addr<va+len(b):
            o=addr-va; j=b.find(b'\0',o); s=b[o:j]
            if 1<=len(s)<200 and all(32<=c<127 or c in(9,10) for c in s): return s.decode()
    return None
def insns(e):
    base,b,funcs,ms=build(e); msa=[m[0] for m in ms]; out=[]  # per function list of (addr,size,hw1,hw2)
    for fa,sz,nm in funcs:
        L=[];a=fa;end=fa+sz
        while a<end:
            i=bisect.bisect_right(msa,a)-1
            if i>=0 and ms[i][1]=='$d': a=msa[i+1] if i+1<len(msa) else end; continue
            o=a-base; hw1=b[o]|b[o+1]<<8
            if is32(hw1): hw2=b[o+2]|b[o+3]<<8; L.append((a,4,hw1,hw2)); a+=4
            else: L.append((a,2,hw1,0)); a+=2
        out.append((nm,L))
    return out
def r0_source(e,L,k):
    """look back from index k for the writer of r0; return pointer value or None"""
    movw=None
    for j in range(k-1,max(k-10,-1),-1):
        a,sz,h1,h2=L[j]
        if sz==2:
            if h1&0xF800==0x4800 and (h1>>8&7)==0:
                la=((a+4)&~3)+(h1&0xFF)*4; v=struct.unpack('<I',rd(e,la,4))[0]; return ('ldr-lit',v)
            if h1&0xF800==0xA000 and (h1>>8&7)==0: return ('adr',((a+4)&~3)+(h1&0xFF)*4)
            if h1&0xFF87==0x4600: return ('mov-from-reg',None)  # mov r0,rX
        else:
            if h1&0xFBF0==0xF240 and (h2>>8&0xF)==0:
                imm=((h1&0xF)<<12)|((h1>>10&1)<<11)|((h2>>12&7)<<8)|(h2&0xFF); movw=imm
                continue_=None
                # find movt
                for m in range(j+1,k):
                    a2,s2,g1,g2=L[m]
                    if s2==4 and g1&0xFBF0==0xF2C0 and (g2>>8&0xF)==0:
                        t=((g1&0xF)<<12)|((g1>>10&1)<<11)|((g2>>12&7)<<8)|(g2&0xFF); return ('movw/movt',(t<<16)|imm)
                return ('movw-only',imm)
            if h1==0xF8DF and (h2>>12)==0:
                off=h2&0xFFF; la=((a+4)&~3)+(off if h1&0x80 else -off); return ('ldr.w-lit',struct.unpack('<I',rd(e,la,4))[0])
            if h1&0xFFF0==0xF2AF or h1&0xFBFF==0xF20F: pass
        # other writes to r0 would break; ignore
    return (None,None)
if __name__=='__main__':
    p=sys.argv[1]; e=Elf(p); pa=e.byname(NAME)[0]['value']&~1
    funcs=insns(e); stats=collections.Counter(); samples=[]; names=collections.Counter(); sites=[]
    for nm,L in funcs:
        for k,(a,sz,h1,h2) in enumerate(L):
            if sz==4:
                r=decode_branch(h1,h2)
                if r and r[0]=='BL' and (a+4+r[1])&0xFFFFFFFF==pa:
                    kind,v=r0_source(e,L,k); s=cstr(e,v) if v else None
                    stats[(kind,'str' if s else 'nostr')]+=1; sites.append((a,nm,kind,v,s))
    print(p,stats)
    pre=lambda n:n[:20]
    for a,nm,kind,v,s in sites[:10]: print(' ',hex(a),nm[:60],kind,hex(v) if v else None,repr(s)[:70])
    # chibios share
    c=collections.Counter()
    for a,nm,*_ in sites:
        c['ChibiOS' if 'ChibiOS' in nm or 'ChibiOS' in str(nm) else 'other']+=1
    print(' name-based:',c)
    import re
    c2=collections.Counter(re.match(r'_ZN(\d+)',nm).group(0)[3:] if False else (nm[:12]) for a,nm,*_ in sites)
    print(' sites with literal ptr:',sum(1 for s in sites if s[4]),'of',len(sites))
    print(' all names:',sorted(set(s[1] for s in sites))[:60])
