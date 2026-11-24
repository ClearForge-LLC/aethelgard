import sys,json,struct,shutil; sys.path.insert(0,'/home/forge/.cache/aet-2314/r4')
from q4 import *
J=json.load(open('/home/forge/.cache/aet-2314/r4/q4.json'))
R='/home/forge/.cache/aet-2314/r4/'
e0=Elf(SRC); T0=table(e0); pa=e0.byname(NAME)[0]['value']&~1
ranking=sorted(((c[0],t) for t,c in T0.items() if c[1]==c[0] and c[0]>=1),reverse=True)
def verdicts(path,label):
    e=Elf(path); T=table(e); s=e.byname(NAME)
    addr=s[0]['value']&~1 if s else None; n,l=T[addr] if addr in T else (0,0)
    # rule A: N>0 ; rule B: N>=1 and all literal ; rule C: hook addr == target of most-called all-literal-r0 function
    best=max(((c[0],t) for t,c in T.items() if c[1]==c[0] and c[0]>=1))[1]
    print(f'{label}: hook={addr:#x} N={n} lit={l} | A(N>0)={n>0} B(N>0&all-lit)={n>0 and n==l} C(hook==top all-lit callee {best:#x})={addr==best}')
zero=J['zero'][0]; print('zero-caller target',hex(zero))
d=R+'a-zero.elf'; shutil.copy(SRC,d); b=bytearray(open(d,'rb').read()); x=e0.byname(NAME)[0]
struct.pack_into('<I',b,x['off']+4,zero|1); open(d,'wb').write(b); verdicts(d,'(a) decoy=zero-caller func')
top=0x80c0394
d=R+'b-top.elf'; shutil.copy(SRC,d); b=bytearray(open(d,'rb').read()); struct.pack_into('<I',b,x['off']+4,top|1); open(d,'wb').write(b); verdicts(d,'(b1) decoy=most-called func (702)')
assertf=0x81099e8
d=R+'b-assert.elf'; shutil.copy(SRC,d); b=bytearray(open(d,'rb').read()); struct.pack_into('<I',b,x['off']+4,assertf|1); open(d,'wb').write(b); verdicts(d,'(b2) decoy=__assert_func (14, all literal)')
# (c): rename real panic (flip last char of its strtab name), put the panic name on a 1-caller literal function
one=[o for o in J['one'] if o[1][0]==1][0]; print('one-caller literal func',one)
e=e0; strsec=[h for h in e.sh if h[1]==3]
# find name offset in symtab entry
nameoff=struct.unpack_from('<I',e.d,x['off'])[0]
sh=e.sh[[h for h in e.sh if h[1]==2][0][6]]
d=R+'c-renamed.elf'; shutil.copy(SRC,d); b=bytearray(open(d,'rb').read())
# rename real panic: change 'z' (last char) to 'y'
b[sh[4]+nameoff+len(NAME)-1]=ord('y')
# find the one-caller function's symtab entry; point its name at the original panic string by copying: add: set a *different* symbol's name offset to the string of panic name? panic string now altered, so make a second sym name -> use old string bytes: rewrite an existing symbol's st_name to point at nameoff of another existing copy? simpler: swap st_value
open(d,'wb').write(b)
# rename back approach: instead of renaming, swap values: symbol NAME -> one-caller func (real panic then nameless in symtab)
d2=R+'c-swap.elf'; shutil.copy(SRC,d2); b=bytearray(open(d2,'rb').read()); struct.pack_into('<I',b,x['off']+4,one[0]|1)
struct.pack_into('<I',b,x['off']+8,0) 
open(d2,'wb').write(b); verdicts(d2,'(c) panic name on 1-caller literal func (real panic still in image, nameless)')
