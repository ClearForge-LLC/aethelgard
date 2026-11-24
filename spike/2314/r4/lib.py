import struct, sys, time, collections
NAME='_ZN6AP_HAL5panicEPKcz'
class Elf:
    def __init__(s,path):
        s.path=path; d=s.d=open(path,'rb').read()
        (s.shoff,)=struct.unpack_from('<I',d,0x20); s.phoff,=struct.unpack_from('<I',d,0x1c)
        s.phentsize,s.phnum,s.shentsize,s.shnum,s.shstrndx=struct.unpack_from('<HHHHH',d,0x2a)
        s.ph=[struct.unpack_from('<8I',d,s.phoff+i*s.phentsize) for i in range(s.phnum)]
        s.sh=[struct.unpack_from('<10I',d,s.shoff+i*s.shentsize) for i in range(s.shnum)]
        s.syms=[]; s.symoff=[]
        for i,h in enumerate(s.sh):
            if h[1]==2:
                strsec=s.sh[h[6]]
                for j in range(h[5]//16):
                    off=h[4]+j*16
                    n,v,sz,info,oth,shn=struct.unpack_from('<IIIBBH',d,off)
                    e=d.index(b'\0',strsec[4]+n)
                    s.syms.append(dict(name=d[strsec[4]+n:e].decode('latin1'),value=v,size=sz,type=info&15,bind=info>>4,shn=shn,off=off))
        # exec loads: PT_LOAD with PF_X
        s.segs=[(p[3],d[p[1]:p[1]+p[4]]) for p in s.ph if p[0]==1 and p[6]&1]  # (vaddr, bytes)
        s.loads=[(p[3],d[p[1]:p[1]+p[4]]) for p in s.ph if p[0]==1 and p[4]>0]
    def byname(s,n): return [x for x in s.syms if x['name']==n]
def decode_branch(hw1,hw2):
    """return (kind, offset) for BL/B.W(T4) else None"""
    if hw1&0xF800!=0xF000: return None
    t=hw2&0xD000
    if t not in (0xD000,0x9000): return None
    S=hw1>>10&1; J1=hw2>>13&1; J2=hw2>>11&1
    I1=(~(J1^S))&1; I2=(~(J2^S))&1
    imm=(S<<24)|(I1<<23)|(I2<<22)|((hw1&0x3ff)<<12)|((hw2&0x7ff)<<1)
    if S: imm-=1<<25
    return ('BL' if t==0xD000 else 'B.W', imm)
def naive_scan(elf):
    """every halfword offset; returns dict target-> list of (addr,kind)"""
    out=collections.defaultdict(list)
    for va,b in elf.segs:
        n=len(b)
        for o in range(0,n-3,2):
            hw1=b[o]|b[o+1]<<8
            if hw1&0xF800!=0xF000: continue
            hw2=b[o+2]|b[o+3]<<8
            r=decode_branch(hw1,hw2)
            if r: out[(va+o+4+r[1])&0xFFFFFFFF].append((va+o,r[0]))
    return out
