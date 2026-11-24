#!/usr/bin/env python3 -I
"""Enumerate optional-ish keys from the IR sources, with JSON paths and file:line.
Optional-ish: Option<..>, #[serde(default..)], skip_serializing_if, Vec<..> (may be empty), BTreeMap/HashMap.
Output: JSON list of {path, struct, field, type, attrs, file, line, kind}."""
import re, sys, json, glob, os
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '../../..'))
SRC = os.path.join(ROOT, 'crates/aethelgard-ir/src')
structs = {}   # name -> list of fields
enums = {}
for f in sorted(glob.glob(SRC + '/*.rs')):
    lines = open(f).read().split('\n')
    i = 0
    rel = os.path.relpath(f, ROOT)
    while i < len(lines):
        m = re.match(r'pub struct (\w+)\s*\{\s*$', lines[i]) or re.match(r'struct (\w+)\s*\{\s*$', lines[i])
        if m:
            name = m.group(1)
            # struct-level attrs
            j = i - 1; sattrs = []
            while j >= 0 and (lines[j].startswith('#[') or lines[j].startswith('///') or lines[j].strip()==''):
                if lines[j].startswith('#['): sattrs.append(lines[j])
                j -= 1
            fields = []; attrs = []; i += 1
            while i < len(lines) and not lines[i].startswith('}'):
                l = lines[i]; s = l.strip()
                if s.startswith('#['):
                    # may span lines
                    a = s
                    while a.count('(') > a.count(')') or a.count('[') > a.count(']'):
                        i += 1; a += ' ' + lines[i].strip()
                    attrs.append(a)
                elif re.match(r'(pub(\([^)]*\))?\s+)?(r#)?\w+\s*:\s*.+,\s*$', s) and not s.startswith('//'):
                    mm = re.match(r'(?:pub(?:\([^)]*\))?\s+)?(?:r#)?(\w+)\s*:\s*(.+),\s*$', s)
                    fields.append(dict(field=mm.group(1), type=mm.group(2), attrs=attrs, file=rel, line=i+1))
                    attrs = []
                elif s.startswith('///') or s == '' or s.startswith('//'):
                    pass
                else:
                    attrs = []
                i += 1
            structs[(rel,name)] = dict(fields=fields, sattrs=sattrs, file=rel)
        mm = re.match(r'pub enum (\w+)', lines[i]) if i < len(lines) else None
        if mm: enums[mm.group(1)] = rel
        i += 1

def inner(t):
    t = re.sub(r'\b(?:crate|super|self)::','',t); t = re.sub(r'\b[a-z_0-9]+::','',t).strip(); opt = vec = mp = False
    while True:
        m = re.match(r'(Option|Vec|Box|BTreeMap<String,|HashMap<String,|BTreeSet|\[)\s*<?(.*?)>?\]?$', t)
        mm = re.match(r'Option<(.*)>$', t)
        if mm: opt = True; t = mm.group(1).strip(); continue
        mm = re.match(r'Vec<(.*)>$', t)
        if mm: vec = True; t = mm.group(1).strip(); continue
        mm = re.match(r'Box<(.*)>$', t)
        if mm: t = mm.group(1).strip(); continue
        mm = re.match(r'(?:BTreeMap|HashMap)<\s*\w+\s*,\s*(.*)>$', t)
        if mm: mp = True; t = mm.group(1).strip(); continue
        mm = re.match(r'(?:BTreeSet|HashSet)<(.*)>$', t)
        if mm: vec = True; t = mm.group(1).strip(); continue
        mm = re.match(r'\[(.*);\s*\w+\]$', t)
        if mm: t = mm.group(1).strip(); continue
        break
    return t, opt, vec, mp

out = []
seen_struct_paths = {}
def lookup(name, ctxfile):
    if (ctxfile,name) in structs: return (ctxfile,name)
    c=[k for k in structs if k[1]==name]
    return c[0] if len(c)==1 else (c[0] if c else None)
def walk(skey, path, depth=0):
    if skey is None or depth > 8: return
    sname = skey[1]
    for fd in structs[skey]['fields']:
        t, opt, vec, mp = inner(fd['type'])
        name = fd['field']
        for a in fd['attrs']:
            m = re.search(r'rename\s*=\s*"(\w+)"', a)
            if m: name = m.group(1)
        flat = any('flatten' in a for a in fd['attrs'])
        p = path + ('' if flat else ('.' if path else '') + name)
        sd = any('serde(default' in a or ', default' in a or 'default =' in a or 'default)' in a for a in fd['attrs'])
        sk = any('skip_serializing_if' in a for a in fd['attrs'])
        kinds = []
        if opt: kinds.append('Option')
        if sd: kinds.append('serde-default')
        if sk: kinds.append('skip_serializing_if')
        if vec: kinds.append('Vec/Set(may be empty)')
        if mp: kinds.append('Map')
        jp = p + ('[]' if vec else ('{}' if mp else ''))
        if kinds:
            out.append(dict(path=jp, struct=sname, field=fd['field'], type=fd['type'], kinds=kinds,
                            attrs=fd['attrs'], file=fd['file'], line=fd['line']))
        if lookup(t, skey[0]):
            walk(lookup(t, skey[0]), jp, depth+1)
        elif t in enums:
            out.append(dict(path=jp+'  (enum '+t+')', struct=sname, field=fd['field'], type=t, kinds=['enum-not-walked'], attrs=[], file=fd['file'], line=fd['line'])) if False else None
walk(('crates/aethelgard-ir/src/lib.rs','Specification'), '')
json.dump(out, sys.stdout, indent=1)
