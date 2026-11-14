import re,sys,hashlib
t=open(sys.argv[1]).read()
# footprint (at x y [rot]) directly after (footprint "..." (layer ...)
pos=sorted(re.findall(r'\(footprint\s+"[^"]*"[^\n]*\n(?:[^\n]*\n){0,6}?\s*\(at ([-\d.]+) ([-\d.]+)(?: ([-\d.]+))?\)',t))
print(hashlib.sha256(repr(pos).encode()).hexdigest()[:12], len(pos))
