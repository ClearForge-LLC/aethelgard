import re,sys
# Hostnames seen leaving the process: TLS SNI and DNS query names, from an
# `strace -f -s300 -e trace=connect,sendto,sendmmsg,sendmsg` log; plus connect() peers.
t=open(sys.argv[1],errors='replace').read()
names=set()
for m in re.finditer(r'(?:sendto|sendmsg|sendmmsg)\(\d+, .*?"((?:[^"\\]|\\.)*)"',t):
    b=bytes(m.group(1),'latin1').decode('unicode_escape').encode('latin1')
    for n in re.findall(rb'([a-z0-9-]{1,63}(?:\.[a-z0-9-]{1,63})+)',b.lower()):
        if re.search(rb'\.(com|org|net|io|dev|cloud)$',n): names.add(n.decode())
    # DNS wire labels
    j=0;labels=[]
    for mm in re.finditer(rb'((?:[\x01-\x3f][a-z0-9-]+)+)\x00\x00[\x01\x1c]\x00\x01',b.lower()):
        s=mm.group(1);i=0;parts=[]
        while i<len(s): l=s[i];parts.append(s[i+1:i+1+l].decode());i+=l+1
        names.add('dns:'+'.'.join(parts))
ips=sorted(set(re.findall(r'sin6?_port=htons\((\d+)\), (?:sin_addr=inet_addr\("|sin6_flowinfo=[^,]*, inet_pton\(AF_INET6, ")([0-9a-f.:]+)',t)))
print('\n'.join(sorted(names)));print(ips)
