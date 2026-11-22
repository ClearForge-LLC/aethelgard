"""AET-WO-1219 scratch reader. Not a crate; reads text, links nothing.

Source side: every serde struct field and enum variant in
crates/aethelgard-ir/src, with file:line, honouring rename, rename_all, alias,
flatten, skip, tag, untagged and deserialize_with. The specification keys (K)
are the fields reachable from `Specification`; the held tables' columns (H)
are the fields reachable from each table's raw root; the solved topology (T)
from `SolvedTopology`. Refusal kinds (R) are the string arms of every
`fn kind` / `fn name` of a refusal enum, checked against the enum's variants.

Data side: every key path of every committed specification (the three
directories versions.rs reads) and of the serde dump (dump.rs), walked along
the schema. Any data key the source side lacks fails the run.

Output: out/inventory.json (every entry) and, with classes.tsv present,
the joined list (LIST.md) and the counts (COUNT.md). An entry with no class,
or a class row with no entry, fails the run.
"""

import argparse
import collections
import json
import pathlib
import re
import sys

# ---------------------------------------------------------------- source --


def strip_comments(text):
    """Blank // and /* */ comments outside string literals, keeping lines."""
    out, i, n = [], 0, len(text)
    while i < n:
        c = text[i]
        if c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            out.append(text[i : j + 1])
            i = j + 1
        elif text.startswith("r#\"", i):
            j = text.index('"#', i + 3)
            out.append(text[i : j + 2])
            i = j + 2
        elif c == "'" and i + 2 < n and (text[i + 2] == "'" or text[i + 1] == "\\"):
            j = text.index("'", i + 2 if text[i + 1] != "\\" else i + 3)
            out.append(text[i : j + 1])
            i = j + 1
        elif text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif text.startswith("/*", i):
            j = text.index("*/", i) + 2
            out.append(re.sub(r"[^\n]", " ", text[i:j]))
            i = j
        else:
            out.append(c)
            i += 1
    return "".join(out)


def match_close(text, i, open_c, close_c):
    depth = 0
    for j in range(i, len(text)):
        if text[j] == open_c:
            depth += 1
        elif text[j] == close_c:
            depth -= 1
            if depth == 0:
                return j
    raise ValueError("unbalanced at %d" % i)


def split_top(s, sep=","):
    parts, depth, cur, instr = [], 0, [], False
    for c in s:
        if c == '"':
            instr = not instr
        if not instr:
            if c in "<([{":
                depth += 1
            elif c in ">)]}":
                depth -= 1
            elif c == sep and depth == 0:
                parts.append("".join(cur))
                cur = []
                continue
        cur.append(c)
    if "".join(cur).strip():
        parts.append("".join(cur))
    return parts


def serde_attrs(attr_texts):
    """Merge every #[serde(...)] into a dict."""
    d = {}
    for a in attr_texts:
        m = re.match(r"\s*serde\s*\((.*)\)\s*$", a, re.S)
        if not m:
            continue
        for part in split_top(m.group(1)):
            part = part.strip()
            kv = re.match(r'(\w+)\s*=\s*"(.*)"$', part, re.S)
            if kv:
                d.setdefault(kv.group(1), []).append(kv.group(2))
            elif re.match(r"\w+$", part):
                d.setdefault(part, []).append(True)
            else:
                d.setdefault("_other", []).append(part)
    return d


def derives(attr_texts):
    out = set()
    for a in attr_texts:
        m = re.match(r"\s*derive\s*\((.*)\)\s*$", a, re.S)
        if m:
            out |= {x.strip().split("::")[-1] for x in m.group(1).split(",")}
    return out


def line_of(text, i):
    return text.count("\n", 0, i) + 1


def rename(name, rule, variant):
    if not rule:
        return name
    words = (
        re.findall(r"[A-Z][a-z0-9]*|[0-9]+", name) if variant else name.split("_")
    )
    words = [w for w in words if w]
    if rule == "snake_case":
        return "_".join(w.lower() for w in words)
    if rule == "lowercase":
        return name.lower() if variant else name
    if rule == "UPPERCASE":
        return name.upper()
    raise ValueError(rule)  # a rule the IR does not use today: teach it here


ITEM = re.compile(r"\b(struct|enum)\s+(\w+)(\s*<[^>{]*>)?\s*(\{|\(|;)")


def parse_items(path, text):
    """Every struct/enum with its attributes, fields or variants."""
    clean = strip_comments(text)
    items = []
    for m in ITEM.finditer(clean):
        # Attributes directly above: walk back over whitespace, `pub(...)`, #[...].
        start = m.start()
        head = clean[:start]
        k = len(head.rstrip())
        vis = re.search(r"(pub(\([^)]*\))?)\s*$", head[:k])
        if vis:
            k = vis.start()
        attrs = []
        while True:
            h = clean[:k].rstrip()
            if not h.endswith("]"):
                break
            # find matching "#["
            depth, j = 0, len(h) - 1
            while j >= 0:
                if h[j] == "]":
                    depth += 1
                elif h[j] == "[":
                    depth -= 1
                    if depth == 0:
                        break
                j -= 1
            if j < 1 or h[j - 1] != "#":
                break
            attrs.insert(0, h[j + 1 : len(h) - 1])
            k = j - 1
        kind, name = m.group(1), m.group(2)
        d = derives(attrs)
        sa = serde_attrs(attrs)
        item = {
            "file": path,
            "name": name,
            "kind": kind,
            "line": line_of(clean, m.start()),
            "derives": sorted(d),
            "serde": sa,
            "cfg_test": any("cfg(test)" in a for a in attrs),
        }
        if m.group(4) == ";":
            item["fields"] = []
        elif m.group(4) == "(":
            close = match_close(clean, m.end() - 1, "(", ")")
            item["tuple"] = [t.strip() for t in split_top(clean[m.end() : close])]
            item["fields"] = []
        else:
            close = match_close(clean, m.end() - 1, "{", "}")
            body = clean[m.end() : close]
            base = m.end()
            if kind == "struct":
                item["fields"] = parse_fields(clean, body, base)
            else:
                item["variants"] = parse_variants(clean, body, base)
        items.append(item)
    return items


def take_attrs(chunk):
    """Leading #[...] attributes of a member chunk, and the rest."""
    attrs, s = [], chunk
    while True:
        t = s.lstrip()
        if not t.startswith("#["):
            return attrs, s
        close = match_close(t, 1, "[", "]")
        attrs.append(t[2:close])
        s = t[close + 1 :]


def parse_fields(clean, body, base):
    fields, cursor = [], 0
    for chunk in split_top(body):
        off = body.index(chunk, cursor)
        cursor = off + len(chunk)
        attrs, rest = take_attrs(chunk)
        m = re.match(r"\s*(pub(\([^)]*\))?\s+)?(r#)?(\w+)\s*:\s*(.*)$", rest, re.S)
        if not m:
            continue
        name_pos = off + chunk.index(rest) + rest.index(m.group(4))
        fields.append(
            {
                "name": m.group(4),
                "type": " ".join(m.group(5).split()),
                "line": line_of(clean, base + name_pos),
                "serde": serde_attrs(attrs),
            }
        )
    return fields


def parse_variants(clean, body, base):
    variants, cursor = [], 0
    for chunk in split_top(body):
        off = body.index(chunk, cursor)
        cursor = off + len(chunk)
        attrs, rest = take_attrs(chunk)
        m = re.match(r"\s*(\w+)\s*(\{|\(|=|$)", rest, re.S)
        if not m:
            continue
        name_pos = off + chunk.index(rest) + rest.index(m.group(1))
        v = {
            "name": m.group(1),
            "line": line_of(clean, base + name_pos),
            "serde": serde_attrs(attrs),
        }
        if m.group(2) == "{":
            o = rest.index("{")
            c = match_close(rest, o, "{", "}")
            v["fields"] = parse_fields(clean, rest[o + 1 : c], base + off + chunk.index(rest) + o + 1)
        elif m.group(2) == "(":
            o = rest.index("(")
            c = match_close(rest, o, "(", ")")
            v["tuple"] = [t.strip() for t in split_top(rest[o + 1 : c])]
        variants.append(v)
    return variants


# ------------------------------------------------------------ type model --

WRAP_SEQ = {"Vec", "BTreeSet", "HashSet", "VecDeque"}
WRAP_MAP = {"BTreeMap", "HashMap"}
WRAP_TRANSPARENT = {"Box", "Option", "Rc", "Arc", "Cow"}
PRIMS = {
    "String", "str", "bool", "char", "u8", "u16", "u32", "u64", "u128", "usize",
    "i8", "i16", "i32", "i64", "i128", "isize", "f32", "f64", "Value",
}


def parse_type(t):
    """('prim', name) | ('opt', T) | ('seq', T) | ('map', V) | ('item', path) | ('tuple', [..])."""
    t = t.strip().lstrip("&").strip()
    t = re.sub(r"^'static\s+", "", t)
    if t.startswith("[") and t.endswith("]"):
        inner = split_top(t[1:-1], ";")[0]
        return ("seq", parse_type(inner))
    if t.startswith("(") and t.endswith(")"):
        return ("tuple", [parse_type(x) for x in split_top(t[1:-1])])
    m = re.match(r"([\w:]+)\s*(<(.*)>)?$", t, re.S)
    if not m:
        return ("prim", t)
    head = m.group(1).split("::")[-1]
    args = split_top(m.group(3)) if m.group(3) else []
    args = [a for a in args if not a.strip().startswith("'")]
    if head == "Option":
        return ("opt", parse_type(args[0]))
    if head in WRAP_TRANSPARENT:
        return parse_type(args[-1])
    if head in WRAP_SEQ:
        return ("seq", parse_type(args[0]))
    if head in WRAP_MAP:
        return ("map", parse_type(args[1]))
    if head in PRIMS:
        return ("prim", head)
    return ("item", m.group(1))


class Crate:
    def __init__(self, src_dir):
        self.items = []
        self.uses = {}
        self.aliases = {}
        for p in sorted(src_dir.glob("*.rs")):
            text = p.read_text()
            rel = "crates/aethelgard-ir/src/" + p.name
            self.items += parse_items(rel, text)
            self.uses[rel] = {}
            # Every `use crate::mod::{A, B}` name, to its module.
            for m in re.finditer(r"\buse\s+crate::(\w+)::\{([^}]*)\}", text):
                for n in re.findall(r"\b(\w+)\b", m.group(2)):
                    self.uses[rel][n] = m.group(1)
            for m in re.finditer(r"\buse\s+crate::(\w+)::(\w+)\s*;", text):
                self.uses[rel][m.group(2)] = m.group(1)
            # Module aliases: `use crate::power_parts::{self as pp, ..}`, `use crate::x as y`.
            al = {}
            for m in re.finditer(r"\buse\s+crate::(\w+)::\{[^}]*\bself\s+as\s+(\w+)", text):
                al[m.group(2)] = m.group(1)
            for m in re.finditer(r"\buse\s+crate::(\w+)\s+as\s+(\w+)\s*;", text):
                al[m.group(2)] = m.group(1)
            self.aliases[rel] = al
        self.by_name = collections.defaultdict(list)
        for it in self.items:
            self.by_name[it["name"]].append(it)

    def resolve(self, ref, from_file):
        parts = ref.split("::")
        name = parts[-1]
        cands = [i for i in self.by_name.get(name, []) if not i["cfg_test"]]
        if not cands:
            return None
        if len(parts) > 1 and parts[-2] not in ("crate", "self", "super"):
            mod = self.aliases.get(from_file, {}).get(parts[-2], parts[-2])
            f = "crates/aethelgard-ir/src/%s.rs" % mod
            got = [i for i in cands if i["file"] == f]
            if got:
                return got[0]
        if len(parts) > 1 and parts[-2] == "crate":
            got = [i for i in cands if i["file"].endswith("/lib.rs")]
            if got:
                return got[0]
        same = [i for i in cands if i["file"] == from_file]
        if same:
            return same[0]
        mod = self.uses.get(from_file, {}).get(name)
        if mod:
            got = [i for i in cands if i["file"].endswith("/%s.rs" % mod)]
            if got:
                return got[0]
        lib = [i for i in cands if i["file"].endswith("/lib.rs")]
        if lib and from_file.endswith("/lib.rs"):
            return lib[0]
        if len(cands) == 1:
            return cands[0]
        if lib:
            return lib[0]
        raise SystemExit("ambiguous type %s from %s: %s" % (ref, from_file, [c["file"] for c in cands]))

    def get(self, name, file):
        for i in self.items:
            if i["name"] == name and i["file"].endswith(file):
                return i
        raise SystemExit("no item %s in %s" % (name, file))


# The wire type serde actually reads where a hand-written Deserialize routes
# through another derive (lib.rs:1040 Mcu -> McuWire; lib.rs:1241
# SolvedTopology -> SolvedWire). Idle and Esc (motor.rs:123, :190) read the
# same externally tagged shape their Serialize derive writes, so their derive
# is the model.
WIRE = {("Mcu", "lib.rs"): ("McuWire", "lib.rs"), ("SolvedTopology", "lib.rs"): ("SolvedWire", "lib.rs")}


def wire(crate, item):
    w = WIRE.get((item["name"], item["file"].split("/")[-1]))
    return crate.get(*w) if w else item


def field_json(f, struct_item):
    if "rename" in f["serde"]:
        return f["serde"]["rename"][0]
    return rename(f["name"], (struct_item["serde"].get("rename_all") or [None])[0], False)


def variant_json(v, enum_item):
    if "rename" in v["serde"]:
        return v["serde"]["rename"][0]
    return rename(v["name"], (enum_item["serde"].get("rename_all") or [None])[0], True)


def skipped(f):
    s = f["serde"]
    return "skip" in s or "skip_deserializing" in s


class Schema:
    """The key tree serde reads from one root, and the entries on it."""

    def __init__(self, crate, unit):
        self.crate = crate
        self.unit = unit
        self.entries = collections.OrderedDict()  # id -> entry

    def eid(self, item, member):
        return "%s:%s.%s" % (item["file"].split("/")[-1][:-3], item["name"], member)

    def entry(self, item, member, line, json_name, extra):
        i = self.eid(item, member)
        e = self.entries.get(i)
        if not e:
            e = {
                "unit": self.unit,
                "id": i,
                "type_name": item["name"],
                "member": member,
                "json": json_name,
                "at": "%s:%d" % (item["file"], line),
                "paths": [],
                "variants": collections.OrderedDict(),
                **extra,
            }
            self.entries[i] = e
        return e

    # node: compiled view of a type for walking data.
    def node(self, t, from_file, stack=()):
        kind = t[0]
        if kind == "prim":
            if t[1] == "Value":
                UNRESOLVED.add((self.unit, "Value", from_file))
            return {"k": "prim", "t": t[1]}
        if kind == "opt":
            return {"k": "opt", "of": self.node(t[1], from_file, stack)}
        if kind == "seq":
            return {"k": "seq", "of": self.node(t[1], from_file, stack)}
        if kind == "map":
            return {"k": "map", "of": self.node(t[1], from_file, stack)}
        if kind == "tuple":
            return {"k": "tuple", "of": [self.node(x, from_file, stack) for x in t[1]]}
        item = self.crate.resolve(t[1], from_file)
        if item is None:
            UNRESOLVED.add((self.unit, t[1], from_file))
            return {"k": "prim", "t": t[1], "unresolved": True}
        REACHED.add((item["file"], item["name"]))
        item = wire(self.crate, item)
        REACHED.add((item["file"], item["name"]))
        key = (item["file"], item["name"])
        if key in stack:
            return {"k": "ref", "item": item}
        return {"k": "item", "item": item, "stack": stack + (key,)}


def walk_schema(sch, node, path, holder):
    """Enumerate every key path the schema admits. `holder` is the entry whose
    value this node is (variants are recorded on it)."""
    k = node["k"]
    if k in ("prim", "ref"):
        return
    if k in ("opt",):
        return walk_schema(sch, node["of"], path, holder)
    if k == "seq":
        return walk_schema(sch, node["of"], path + "[]", holder)
    if k == "map":
        return walk_schema(sch, node["of"], path + ".*", holder)
    if k == "tuple":
        for x in node["of"]:
            walk_schema(sch, x, path, holder)
        return
    item = node["item"]
    stack = node["stack"]
    if item["kind"] == "struct":
        if item.get("tuple"):
            # newtype or tuple struct: serde reads the inner value(s).
            for t in item["tuple"]:
                walk_schema(sch, sch.node(parse_type(t), item["file"], stack), path, holder)
            return
        for f in item["fields"]:
            if skipped(f):
                continue
            t = parse_type(f["type"])
            sub = sch.node(t, item["file"], stack)
            if "flatten" in f["serde"]:
                walk_schema(sch, sub, path, holder)
                continue
            j = field_json(f, item)
            fp = (path + "." if path else "") + j
            e = sch.entry(
                item, f["name"], f["line"], j,
                {
                    "rust_type": f["type"],
                    "optional_on_wire": t[0] == "opt" and "deserialize_with" not in f["serde"]
                    or "default" in f["serde"],
                    "serde": {a: v for a, v in f["serde"].items()},
                    "aliases": f["serde"].get("alias", []),
                },
            )
            if fp not in e["paths"]:
                e["paths"].append(fp)
            for a in f["serde"].get("alias", []):
                ap = (path + "." if path else "") + a
                if ap not in e["paths"]:
                    e["paths"].append(ap)
            walk_schema(sch, sub, fp, e)
        return
    # enum
    tag = (item["serde"].get("tag") or [None])[0]
    untagged = "untagged" in item["serde"]
    if tag:
        te = sch.entry(item, "<tag %s>" % tag, item["line"], tag, {"rust_type": "tag", "serde": {}})
        tp = (path + "." if path else "") + tag
        if tp not in te["paths"]:
            te["paths"].append(tp)
    for v in item["variants"]:
        if "skip" in v["serde"] or "skip_deserializing" in v["serde"]:
            continue
        vj = variant_json(v, item)
        if holder is not None:
            holder["variants"].setdefault(
                vj, {"rust": "%s::%s" % (item["name"], v["name"]), "at": "%s:%d" % (item["file"], v["line"]), "used": []}
            )
        if tag:
            te["variants"].setdefault(vj, {"rust": "%s::%s" % (item["name"], v["name"]), "at": "%s:%d" % (item["file"], v["line"]), "used": []})
        payload_path = path if (tag or untagged) else (path + "." if path else "") + vj
        if v.get("fields"):
            fake = dict(item, kind="struct", fields=v["fields"], tuple=None, serde={"rename_all": item["serde"].get("rename_all_fields", [])})
            walk_schema(sch, {"k": "item", "item": fake, "stack": stack}, payload_path, holder)
        elif v.get("tuple"):
            for t in v["tuple"]:
                walk_schema(sch, sch.node(parse_type(t), item["file"], stack), payload_path, holder)


# --------------------------------------------------------------- data walk --


class Walk:
    def __init__(self, sch):
        self.sch = sch
        self.unknown = []
        self.seen_paths = collections.Counter()
        self.seen_entries = collections.defaultdict(set)
        self.seen_variants = collections.defaultdict(set)

    def fits(self, node, v):
        """Would this value read as this node, ignoring deep errors?"""
        sub = Walk(self.sch)
        sub.value(node, v, "", None, "?")
        return not sub.unknown

    def value(self, node, v, path, holder, src):
        k = node["k"]
        if k == "prim" or k == "ref":
            if k == "ref":
                self.value({"k": "item", "item": node["item"], "stack": ()}, v, path, holder, src)
            return
        if k == "opt":
            if v is None:
                return
            return self.value(node["of"], v, path, holder, src)
        if k == "seq":
            if isinstance(v, list):
                for x in v:
                    self.value(node["of"], x, path + "[]", holder, src)
            return
        if k == "map":
            if isinstance(v, dict):
                for x in v.values():
                    self.value(node["of"], x, path + ".*", holder, src)
            return
        if k == "tuple":
            if isinstance(v, list):
                for n, x in zip(node["of"], v):
                    self.value(n, x, path, holder, src)
            return
        item = node["item"]
        stack = node.get("stack", ())
        if item["kind"] == "struct":
            if item.get("tuple"):
                ts = item["tuple"]
                if len(ts) == 1:
                    self.value(self.sch.node(parse_type(ts[0]), item["file"], stack), v, path, holder, src)
                return
            if not isinstance(v, dict):
                return
            self.struct(item, item["fields"], stack, v, path, src)
            return
        self.enum(item, stack, v, path, holder, src)

    def struct(self, item, fields, stack, v, path, src):
        names = {}
        flat = []
        for f in fields:
            if skipped(f):
                continue
            if "flatten" in f["serde"]:
                flat.append(f)
                continue
            names[field_json(f, item)] = f
            for a in f["serde"].get("alias", []):
                names[a] = f
        for key, x in v.items():
            f = names.get(key)
            fp = (path + "." if path else "") + key
            if f is None:
                if flat:
                    continue  # flattened children take the rest; checked below
                self.unknown.append((src, fp))
                continue
            eid = self.sch.eid(item, f["name"])
            e = self.sch.entries.get(eid)
            self.seen_paths[fp] += 1
            self.seen_entries[eid].add(src)
            self.value(self.sch.node(parse_type(f["type"]), item["file"], stack), x, fp, e, src)
        for f in flat:
            rest = {k: x for k, x in v.items() if k not in names}
            self.value(self.sch.node(parse_type(f["type"]), item["file"], stack), rest, path, None, src)

    def enum(self, item, stack, v, path, holder, src):
        tag = (item["serde"].get("tag") or [None])[0]
        untagged = "untagged" in item["serde"]
        vs = [x for x in item["variants"] if "skip" not in x["serde"]]
        def note(vj):
            if holder is not None:
                self.seen_variants[holder["id"]].add((vj, src))
        if untagged:
            for x in vs:
                node = self.variant_node(item, x, stack)
                if node is not None and self.fits(node, v):
                    note(variant_json(x, item))
                    self.value(node, v, path, holder, src)
                    return
            self.unknown.append((src, path + " (no untagged variant fits)"))
            return
        if tag:
            if not isinstance(v, dict) or tag not in v:
                self.unknown.append((src, path + " (no tag)"))
                return
            vj = v[tag]
            te = self.sch.eid(item, "<tag %s>" % tag)
            self.seen_entries[te].add(src)
            self.seen_variants[te].add((vj, src))
            self.seen_paths[(path + "." if path else "") + tag] += 1
            x = [x for x in vs if variant_json(x, item) == vj]
            if not x:
                self.unknown.append((src, path + " (tag %s)" % vj))
                return
            rest = {k: y for k, y in v.items() if k != tag}
            node = self.variant_node(item, x[0], stack)
            if node is not None:
                self.value(node, rest, path, holder, src)
            return
        if isinstance(v, str):
            x = [x for x in vs if variant_json(x, item) == v]
            if not x:
                self.unknown.append((src, "%s = %r (no such variant)" % (path, v)))
            else:
                note(v)
            return
        if isinstance(v, dict) and len(v) == 1:
            (vj, inner), = v.items()
            x = [x for x in vs if variant_json(x, item) == vj]
            if not x:
                self.unknown.append((src, "%s.%s (no such variant)" % (path, vj)))
                return
            note(vj)
            vp = (path + "." if path else "") + vj
            self.seen_paths[vp] += 1
            node = self.variant_node(item, x[0], stack)
            if node is not None:
                self.value(node, inner, vp, holder, src)

    def variant_node(self, item, x, stack):
        if x.get("fields"):
            fake = dict(item, kind="struct", fields=x["fields"], tuple=None, serde={})
            return {"k": "item", "item": fake, "stack": stack}
        if x.get("tuple"):
            if len(x["tuple"]) == 1:
                return self.sch.node(parse_type(x["tuple"][0]), item["file"], stack)
            return {"k": "tuple", "of": [self.sch.node(parse_type(t), item["file"], stack) for t in x["tuple"]]}
        return {"k": "prim", "t": "unit"}


# ------------------------------------------------------------- refusals --


def refusal_kinds(src_dir):
    """Every refusal-kind string: the `=> "Name"` arms of each refusal enum's
    `fn kind` or `fn name`, checked against that enum's variants."""
    enums = {
        "lib.rs": "IrValidationError",
        "clock.rs": "HseRefusal",
        "motor.rs": "PwmRefusal",
        "pin_devices.rs": "PinKindOfRefusal",
        "power.rs": "Refusal",
        "solve.rs": "SolveError",
    }
    out, problems = [], []
    for fname, ename in enums.items():
        text = (src_dir / fname).read_text()
        clean = strip_comments(text)
        items = [i for i in parse_items("crates/aethelgard-ir/src/" + fname, text) if i["name"] == ename and i["kind"] == "enum"]
        if len(items) != 1:
            problems.append("%s: %s enum not found once" % (fname, ename))
            continue
        en = items[0]
        m = re.search(r"impl\s+%s\s*\{" % ename, clean)
        body_start = m.end() - 1
        body = clean[body_start : match_close(clean, body_start, "{", "}")]
        fm = re.search(r"fn\s+(kind|name)\s*\(", body)
        fstart = body.index("{", fm.end())
        fbody = body[fstart : match_close(body, fstart, "{", "}")]
        arms = {}
        # Each arm: the patterns before `=>` (one or more `|`-joined) and its value.
        inner = fbody[fbody.index("match"):]
        inner = inner[inner.index("{") + 1:]
        for am in re.finditer(r"((?:\|?\s*(?:Self|K|%s)::\w+(?:\s*\{[^}]*\}|\s*\([^)]*\))?\s*)+)=>\s*(\{\s*\"(\w+)\"\s*\}|\"(\w+)\"|[^,]+),?" % ename, inner):
            for vname in re.findall(r"(?:Self|K|%s)::(\w+)" % ename, am.group(1)):
                arms[vname] = am.group(3) or am.group(4) or ("<delegates: %s>" % am.group(2).strip())
        for v in en["variants"]:
            got = arms.get(v["name"])
            if got is None:
                problems.append("%s::%s has no arm in fn %s" % (ename, v["name"], fm.group(1)))
                continue
            if got.startswith("<delegates"):
                continue  # a wrapper: its kinds are its inner enum's
            out.append(
                {
                    "unit": "R",
                    "id": "R:%s" % got,
                    "type_name": ename,
                    "member": v["name"],
                    "json": got,
                    "at": "crates/aethelgard-ir/src/%s:%d" % (fname, v["line"]),
                    "variant_differs": got != v["name"],
                }
            )
        # Kinds a variant chooses by a nested match (lib.rs PinSpeedUnserved,
        # solve.rs PinSpeed): every `=> "Name"` literal of the fn not yet read.
        have = {e["json"] for e in out if e["type_name"] == ename}
        for lm in re.finditer(r'=>\s*\{?\s*"([A-Z]\w+)"', fbody):
            if lm.group(1) in have:
                continue
            have.add(lm.group(1))
            pos = body_start + fstart + lm.start()
            out.append(
                {
                    "unit": "R",
                    "id": "R:%s" % lm.group(1),
                    "type_name": ename + "(nested match)",
                    "member": lm.group(1),
                    "json": lm.group(1),
                    "at": "crates/aethelgard-ir/src/%s:%d" % (fname, line_of(clean, pos)),
                    "variant_differs": True,
                }
            )
    seen = collections.Counter(e["json"] for e in out)
    dup = [k for k, n in seen.items() if n > 1]
    # The same string from two enums is one kind on the wire.
    merged = collections.OrderedDict()
    for e in out:
        if e["id"] in merged:
            merged[e["id"]]["at"] += ", " + e["at"].split("/")[-1]
            merged[e["id"]]["type_name"] += "+" + e["type_name"]
        else:
            merged[e["id"]] = e
    return list(merged.values()), problems, dup


def all_kind_literals(src_dir):
    """Every `=> "UpperCamel"` in src but versions.rs: what versions.rs's own
    test calls a kind this crate returns."""
    got = set()
    for p in src_dir.glob("*.rs"):
        if p.name == "versions.rs":
            continue
        got |= set(re.findall(r'=>\s*"([A-Z][A-Za-z0-9]+)"', strip_comments(p.read_text())))
    return got


# ------------------------------------------------------------- versions --


def version_rows(src_dir):
    text = strip_comments((src_dir / "versions.rs").read_text())
    start = text.index("pub const CHANGES")
    body = text[text.index("[", text.index("=", start)) : text.index("];", start)]
    rows = []
    for m in re.finditer(r"\brow\(", body):
        o = m.end() - 1
        args = split_top(body[o + 1 : match_close(body, o, "(", ")")])
        n = int(args[0].strip().replace("_", ""))
        lit = lambda a: re.findall(r'"([^"]+)"', a)
        repl = [(x, y) for x, y in re.findall(r'path:\s*"([^"]+)",\s*by:\s*"([^"]+)"', args[4])]
        rows.append({"version": n, "wo": lit(args[1])[0], "requires": lit(args[2]), "optional": lit(args[3]), "replaced": repl, "rules": lit(args[5])})
    for m in re.finditer(r"VersionChange\s*\{", body):
        raise SystemExit("versions.rs: a row not built by row(): read it by hand")
    return rows


def version_of_path(path, rows):
    """The version that introduced `path`: the row naming the longest prefix
    of it (a container a row added brings its keys; a rename's `by` counts),
    the later row on a tie; else 1."""
    norm = path.replace(".*", "")
    hits = [(len(p), r["version"], r["wo"], p) for r in rows
            for p in r["requires"] + r["optional"] + [y for _, y in r["replaced"]]
            if norm == p or norm.startswith(p + ".") or norm.startswith(p + "[]")]
    if not hits:
        return (1, "v1", None)
    _, v, wo, p = max(hits)
    return (v, wo, p)


# ------------------------------------------------------------ citations --

DOC = r"(?:DS|RM|AN|ES|UM|DB)-?\d{3,5}|DS-009|SNVS\w+|SBVS\w+|SLVS\w+|SCES\w+|SLLS\w+|IPC-\d+\w*|JESD\w+"
PAGE = re.compile(r"\b(?:pp?\.)\s*\d+(?:\s*[–-]\s*\d+)?")


def cite_source_item(text, line):
    """The citation of a source item: its doc comment (the `///` block above
    `line`, or above the first item of its group) and its value, as written.
    Returns (document, page or None)."""
    lines = text.split("\n")
    j = line - 2
    # A group of items shares the doc block above its first (silicon.rs
    # CR1_SPE, CR1_CSTART, CR1_SSI): walk up over the group, to a blank line.
    while j >= 0 and lines[j].strip() and not lines[j].strip().startswith(("///", "#[", "}")):
        j -= 1
    block = []
    while j >= 0 and (lines[j].strip().startswith("///") or lines[j].strip().startswith("#[")):
        block.insert(0, lines[j].strip().lstrip("/").strip())
        j -= 1
    k = line - 1
    while k < len(lines) and not lines[k].rstrip().endswith(";"):
        k += 1
    blob = " ".join(block + lines[line - 1 : k + 1])
    docs = re.findall(DOC, blob)
    m = PAGE.search(blob)
    if not docs:
        return ("the module's document", m.group(0)) if m else (None, None)
    return ", ".join(dict.fromkeys(docs)), (m.group(0) if m else None)


def cite_table(table):
    """Every citation object in a held data file ({document, section} or
    {document, ..., section}): how many, how many name a page."""
    found = []
    def walk(v):
        if isinstance(v, dict):
            if "document" in v and isinstance(v.get("document"), str):
                found.append(v)
            for x in v.values():
                walk(x)
        elif isinstance(v, list):
            for x in v:
                walk(x)
    walk(table)
    paged = [c for c in found if PAGE.search(json.dumps(c))]
    return len(found), len(paged), sorted({c["document"] for c in found})


# ----------------------------------------------------------------- main --


# Each held data file: its envelope, and the row types its rows are parsed as
# one by one from a RawValue (parts.rs:1582, :1604; power_parts.rs:2491).
TABLE_ROOTS = [
    ("H:parts.json", "crates/aethelgard-ir/data/parts.json",
     [("RawTable", "parts.rs", None), ("RawPart", "parts.rs", "parts"), ("RawPinPart", "pin_parts.rs", "pin_parts")]),
    ("H:power-parts.json", "crates/aethelgard-ir/data/power-parts.json",
     [("RawTable", "power_parts.rs", None), ("RawRow", "power_parts.rs", "parts")]),
    ("H:output-speeds.json", "crates/aethelgard-ir/data/output-speeds.json",
     [("RawTable", "speeds.rs", None)]),
]

# A file key serde reads but never writes back, by design: McuWire's
# version-19 `hse_hz`, read as `"hse": null` at 0 (lib.rs:1034-1040).
READ_ONLY_PATHS = {"mcu.hse_hz"}


REACHED = set()
UNRESOLVED = set()

# A type the walk cannot see into, and why that hides no key.
EXPLAINED_UNRESOLVED = {
    ("Hc", "serde_json::value::RawValue", "crates/aethelgard-ir/src/parts.rs"):
        "parts.json `parts[]`/`pin_parts[]`: each row is parsed on its own as RawPart / RawPinPart (parts.rs:1582, :1604), walked here as their own roots",
    ("Hc", "serde_json::value::RawValue", "crates/aethelgard-ir/src/power_parts.rs"):
        "power-parts.json `parts[]`: each row is parsed on its own as RawRow (power_parts.rs:2491), walked here as its own root",
    ("Hc", "Value", "crates/aethelgard-ir/src/pin_parts.rs"):
        "RawPinPart.identity: any value but null is refused (pin_parts.rs:432), so no key lives under it",
}

# Serde types read only as another type's wire form, and so in no unit of
# their own: SolvedWire and GpioWire are SolvedTopology's read side
# (lib.rs:1241); T walks its Serialize derive, the form `solve` prints.
WIRE_ONLY = {("crates/aethelgard-ir/src/lib.rs", "SolvedWire"), ("crates/aethelgard-ir/src/lib.rs", "GpioWire")}


def committed_specs(repo):
    """Every tracked JSON file (git ls-files), the WO's "every committed
    specification"; the caller keeps those with an `mcu` object."""
    import subprocess
    out = subprocess.run(["git", "-C", str(repo), "ls-files", "*.json"], check=True, capture_output=True, text=True).stdout
    return [repo / x for x in out.split()]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", required=True)
    ap.add_argument("--dump", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--src", help="the IR's src/ (default: the repository's); a mutant copy for the reader's own mutation test")
    a = ap.parse_args()
    repo = pathlib.Path(a.repo).resolve()
    src = pathlib.Path(a.src) if a.src else repo / "crates/aethelgard-ir/src"
    out = pathlib.Path(a.out)
    out.mkdir(exist_ok=True)
    crate = Crate(src)
    fail = []

    # K: from Specification.
    spec_item = crate.get("Specification", "lib.rs")
    K = Schema(crate, "K")
    walk_schema(K, {"k": "item", "item": spec_item, "stack": (("x", "Specification"),)}, "", None)

    # Committed specifications: version from the file; current = SPECIFICATION_SCHEMA_VERSION.
    cur = int(re.search(r"SPECIFICATION_SCHEMA_VERSION:\s*u32\s*=\s*(\d+)", (src / "lib.rs").read_text()).group(1))
    dump = {}
    for line in pathlib.Path(a.dump).read_text().splitlines():
        r = json.loads(line)
        dump[r["file"]] = r
    files_walk = Walk(K)
    dump_walk = Walk(K)
    spec_files, skipped_files = [], []
    for p in committed_specs(repo):
        try:
            v = json.loads(p.read_text())
        except (ValueError, UnicodeDecodeError):
            continue  # not a specification: not JSON text
        rel = str(p.relative_to(repo))
        is_spec = isinstance(v, dict) and isinstance(v.get("mcu"), dict)
        if not is_spec:
            continue  # not a specification: no `mcu` object
        d = dump.get(str(p))
        ver = v.get("schema_version")
        if ver != cur:
            skipped_files.append((rel, "schema_version %s, not %d: serde refuses an older file by its version, so it is not walked" % (ver, cur)))
            continue
        if d is None or "dump" not in d:
            skipped_files.append((rel, "serde refuses it: %s" % (d or {}).get("error")))
            continue
        spec_files.append(rel)
        files_walk.value({"k": "item", "item": spec_item, "stack": (("x", "Specification"),)}, v, "", None, rel)
        dump_walk.value({"k": "item", "item": spec_item, "stack": (("x", "Specification"),)}, d["dump"], "", None, rel)
    if len(spec_files) < 20:
        fail.append("only %d current committed specifications (versions.rs asserts >= 20)" % len(spec_files))
    for src_, p in files_walk.unknown:
        fail.append("FILE KEY NOT IN SOURCE: %s %s" % (src_, p))
    for src_, p in dump_walk.unknown:
        fail.append("DUMP KEY NOT IN SOURCE: %s %s" % (src_, p))
    # Every dump path the source lacks is caught above; every file path must be a dump path too.
    file_paths, dump_paths = set(files_walk.seen_paths), set(dump_walk.seen_paths)
    for p in sorted(file_paths - dump_paths - READ_ONLY_PATHS):
        fail.append("FILE PATH SERDE DOES NOT WRITE BACK: %s" % p)
    for e in K.entries.values():
        e["in_files"] = len(files_walk.seen_entries.get(e["id"], ()))
        e["in_dump"] = len(dump_walk.seen_entries.get(e["id"], ()))
        for vj, info in e["variants"].items():
            info["used"] = sorted({s for (x, s) in files_walk.seen_variants.get(e["id"], ()) if x == vj})
    # Source-side keys no file writes.
    unseen_k = [e for e in K.entries.values() if e["in_files"] == 0]
    unused_variants = [(e["id"], vj) for e in K.entries.values() for vj, i in e["variants"].items() if not i["used"]]

    # Versions.
    rows = version_rows(src)
    if rows[-1]["version"] != cur:
        fail.append("versions.rs last row %d, current %d" % (rows[-1]["version"], cur))
    for e in K.entries.values():
        vs = [version_of_path(p, rows) for p in e["paths"]]
        v0 = min(vs, key=lambda x: x[0])
        e["version"], e["version_wo"], e["version_by"] = v0
    # Every path a row names must be a source path (or its prefix).
    allp = {p.replace(".*", "") for e in K.entries.values() for p in e["paths"]}
    for r in rows:
        for p in r["requires"] + r["optional"] + [y for _, y in r["replaced"]]:
            if p not in allp and not any(q.startswith(p + ".") or q.startswith(p + "[]") for q in allp):
                fail.append("versions.rs v%d names %s, which no source path is" % (r["version"], p))

    # R.
    R, rproblems, rdup = refusal_kinds(src)
    fail += ["REFUSAL: " + x for x in rproblems]
    lits = all_kind_literals(src)
    rnames = {e["json"] for e in R}
    extra_lits = sorted(lits - rnames)
    for r in rows:
        for k in r["rules"]:
            if k not in rnames:
                fail.append("versions.rs v%d rule %s is not a refusal kind read here" % (r["version"], k))
    rule_version = {}
    for r in rows:
        for k in r["rules"]:
            rule_version.setdefault(k, r["version"])
    for e in R:
        e["rule_row"] = rule_version.get(e["json"])

    # H: data tables' columns, walked against their own data.
    H = []
    H_cols = []
    for hid, data, roots in TABLE_ROOTS:
        table = json.loads((repo / data).read_text())
        sch = None
        for root, f, key in roots:
            it = crate.get(root, f)
            if sch is None:
                sch = Schema(crate, "Hc")
                first = it
            base = key or ""
            node = {"k": "item", "item": it, "stack": ((it["file"], root),)}
            walk_schema(sch, node if key is None else {"k": "seq", "of": node}, base, None)
        w = Walk(sch)
        for root, f, key in roots:
            it = crate.get(root, f)
            node = {"k": "item", "item": it, "stack": ((it["file"], root),)}
            if key is None:
                w.value(node, table, "", None, data)
            else:
                for row in table[key]:
                    w.value(node, row, key + "[]", None, data)
        it = first
        for s_, p in w.unknown:
            fail.append("TABLE KEY NOT IN SOURCE: %s %s" % (s_, p))
        for e in sch.entries.values():
            e["table"] = hid
            e["in_files"] = len(w.seen_entries.get(e["id"], ()))
            for vj, info in e["variants"].items():
                info["used"] = sorted({s for (x, s) in w.seen_variants.get(e["id"], ()) if x == vj})
            e["id"] = hid + "|" + e["id"]
            H_cols.append(e)
        n_c, n_p, docs = cite_table(table)
        H.append({"unit": "H", "id": hid, "type_name": first["name"], "member": "", "json": data, "at": it["file"] + ":%d" % it["line"],
                  "cites": "%d row citations, %d name a page; documents: %s" % (n_c, n_p, ", ".join(docs))})

    # H in source: every const or static item outside `#[cfg(test)]` (RULES
    # amendment 1: a scalar is a one-row table).
    for p in sorted(src.glob("*.rs")):
        text = p.read_text()
        clean = strip_comments(text)
        m = re.search(r"#\[cfg\(test\)\]\s*mod\s+\w+\s*\{", clean)
        cut = m.start() if m else len(clean)
        for cm in re.finditer(r"^[ \t]*(pub(\([^)]*\))?\s+)?(const|static)\s+([A-Z][A-Z0-9_]*)\s*:\s*([^=]+)=", clean[:cut], re.M):
            end = cm.end()
            depth, j = 0, end
            while j < len(clean):
                c = clean[j]
                if c in "([{":
                    depth += 1
                elif c in ")]}":
                    depth -= 1
                elif c == ";" and depth == 0:
                    break
                j += 1
            value = " ".join(clean[end:j].split())
            H.append(
                {
                    "unit": "H",
                    "id": "H:%s:%s" % (p.name[:-3], cm.group(4)),
                    "type_name": cm.group(3),
                    "member": cm.group(4),
                    "json": " ".join(cm.group(5).split()),
                    "at": "crates/aethelgard-ir/src/%s:%d" % (p.name, line_of(clean, cm.start(4))),
                    "value": value[:300] + ("…" if len(value) > 300 else ""),
                }
            )
            d, pg = cite_source_item(text, line_of(clean, cm.start(4)))
            H[-1]["cites"] = ("%s, %s" % (d, pg)) if d and pg else ("%s, no page cited" % d if d else "no citation")

    # H: the pin table's schema (lib.rs PinTable). Its rows are built outside
    # this crate (aethelgard-svd-hal::pin_table, from a vendored source), so
    # its columns are walked with no data.
    pt = crate.get("PinTable", "lib.rs")
    ptsch = Schema(crate, "Hc")
    REACHED.add((pt["file"], "PinTable"))
    walk_schema(ptsch, {"k": "item", "item": pt, "stack": ((pt["file"], "PinTable"),)}, "", None)
    for e in ptsch.entries.values():
        e["table"] = "H:pin-table"
        e["in_files"] = None
        e["id"] = "H:pin-table|" + e["id"]
        H_cols.append(e)
    H.append({"unit": "H", "id": "H:pin-table", "type_name": "PinTable", "member": "", "json": "rows built by aethelgard-svd-hal::pin_table (out of scope)", "at": pt["file"] + ":%d" % pt["line"],
              "cites": "no page cited: rows are built from a vendored source, cited by repository, commit and sha256 (PinSource)"})

    # T: the solved topology, from its Serialize derive (what solve prints),
    # then every other serde type no walk reached, each as its own root.
    st = crate.get("SolvedTopology", "lib.rs")
    REACHED.add((st["file"], "SolvedTopology"))
    T = Schema(crate, "T")
    walk_schema(T, {"k": "item", "item": st, "stack": ((st["file"], "SolvedTopology"),)}, "", None)
    REACHED.add((spec_item["file"], "Specification"))
    REACHED.update(WIRE_ONLY)
    for root, f, key in [r for _, _, rr in TABLE_ROOTS for r in rr]:
        REACHED.add((crate.get(root, f)["file"], root))
    orphans = []
    while True:
        more = [i for i in crate.items
                if not i["cfg_test"] and ({"Serialize", "Deserialize"} & set(i["derives"]))
                and (i["file"], i["name"]) not in REACHED]
        if not more:
            break
        for i in more:
            REACHED.add((i["file"], i["name"]))
            orphans.append({"type": i["name"], "at": "%s:%d" % (i["file"], i["line"]), "derives": i["derives"]})
            if "Deserialize" in i["derives"] and "Serialize" not in i["derives"]:
                fail.append("A DESERIALIZE TYPE NO WALK REACHES: %s %s:%d" % (i["name"], i["file"], i["line"]))
            walk_schema(T, {"k": "item", "item": i, "stack": ((i["file"], i["name"]),)}, i["name"], None)
    for u in sorted(UNRESOLVED):
        if u not in EXPLAINED_UNRESOLVED:
            fail.append("UNRESOLVED TYPE (its keys would be invisible): unit %s, %s from %s" % u)

    inv = {
        "current_version": cur,
        "spec_files": spec_files,
        "skipped_files": skipped_files,
        "K": list(K.entries.values()),
        "R": R,
        "R_duplicate_strings": rdup,
        "R_literal_arms_not_refusals": extra_lits,
        "H": H,
        "H_columns": H_cols,
        # RULES §1: T is what the specification does not reach.
        "T": [e for e in T.entries.values() if e["id"] not in K.entries],
        "T_shared_with_K": [e for e in T.entries if e in K.entries],
        "T_roots_beyond_solved_topology": orphans,
        "unresolved_explained": [dict(zip(("unit", "type", "from"), u), why=EXPLAINED_UNRESOLVED[u]) for u in sorted(UNRESOLVED) if u in EXPLAINED_UNRESOLVED],
        "K_unseen_in_files": [e["id"] for e in unseen_k],
        "K_unused_variants": unused_variants,
        "version_rows": rows,
    }
    (out / "inventory.json").write_text(json.dumps(inv, indent=1, sort_keys=False) + "\n")
    print("current version %d; %d committed current specifications; %d files set aside" % (cur, len(spec_files), len(skipped_files)))
    print("K %d  R %d  H %d (data files 3 + pin table, columns %d; source items %d)  T %d" % (len(K.entries), len(R), len(H), len(H_cols), len(H) - 4, len(inv["T"])))
    print("K paths seen in files %d, in dump %d; K entries no file writes: %d; enum values no file uses: %d" % (len(file_paths), len(dump_paths), len(unseen_k), len(unused_variants)))
    if fail:
        print("FAIL")
        for x in fail:
            print("  " + x)
        sys.exit(1)
    print("reader: every data key is a source key")
    cls = pathlib.Path(a.out).parent / "classes.tsv"
    if cls.exists():
        join(inv, cls, pathlib.Path(a.out).parent)


# ----------------------------------------------------------------- join --

CLASSES = {"V", "N", "S", "D"}
NOT_HELD = "not held"


def join(inv, cls_path, here):
    """Every entry classed exactly once, by RULES.md; LIST.md and COUNT.md."""
    rows, fail = {}, []
    for n, line in enumerate(cls_path.read_text().splitlines()[1:], 2):
        if not line.strip():
            continue
        f = line.split("\t")
        if len(f) != 6:
            fail.append("classes.tsv:%d: %d columns, not 6" % (n, len(f)))
            continue
        i, c, flag, tags, at, why = f
        if i in rows:
            fail.append("classes.tsv:%d: %s classed twice" % (n, i))
        if c not in CLASSES or flag not in ("", "?"):
            fail.append("classes.tsv:%d: class %r flag %r" % (n, c, flag))
        rows[i] = {"class": c, "flag": flag, "tags": tags, "value_at": at, "why": why}
    units = [("K", inv["K"]), ("R", inv["R"]), ("H", inv["H"]), ("Hc", inv["H_columns"]), ("T", inv["T"])]
    ids = set()
    for u, es in units:
        for e in es:
            ids.add(e["id"])
            r = rows.get(e["id"])
            if r is None:
                fail.append("UNCLASSED %s %s" % (u, e["id"]))
                continue
            if r["class"] == "D" and u not in ("H",):
                fail.append("D is for a held table or source item only (rule 1): %s" % e["id"])
            e.update(r)
            e["rp2350"] = NOT_HELD if r["class"] in ("N", "S") else ""
    for i in sorted(set(rows) - ids):
        fail.append("CLASSED BUT NO SUCH ENTRY: %s" % i)
    # No second-chip fact in any reason (RULES §2: judged from ST's side alone).
    for i, r in rows.items():
        if re.search(r"(?i)rp2[0-9]{3}|raspberry|pico\b|esp32|nrf5|atsam|kinetis|i\.mx|lpc[0-9]|gd32|ch32", r["why"]):
            fail.append("A SECOND-CHIP FACT IN A REASON: %s: %s" % (i, r["why"]))
    if fail:
        print("JOIN FAIL")
        for x in fail:
            print("  " + x)
        sys.exit(1)
    write_list(inv, units, here)
    write_count(inv, units, here)
    print("join: %d entries classed; LIST.md and COUNT.md written" % len(ids))


def md(x):
    return str(x).replace("|", "\\|").replace("\n", " ")


def write_list(inv, units, here):
    o = ["# AET-WO-1219 — the list", "",
         "Generated by `reader.py` from `out/inventory.json` and `classes.tsv`; do not edit. Classes by `RULES.md`. "
         "`at` is where the entry is defined; `value at` is where its value set is held or checked. "
         "The RP2350 column is filled for every N and S entry; no RP2350 datasheet is held (FEEDBACK §RP2350), so every cell is \"not held\".", ""]
    heads = {
        "K": ("K — specification keys (%d)", ["id", "paths", "v", "class", "tags", "at", "value at", "why", "RP2350"]),
        "R": ("R — refusal kinds (%d)", ["kind", "class", "tags", "at", "why", "RP2350"]),
        "H": ("H — held tables and source items (%d)", ["id", "class", "tags", "at", "cites", "why", "RP2350"]),
        "Hc": ("H columns — the held data files' and the pin table's columns (%d)", ["id", "paths", "class", "tags", "at", "value at", "why", "RP2350"]),
        "T": ("T — solved topology and other output types, not counted toward acceptance (%d)", ["id", "paths", "class", "tags", "at", "why", "RP2350"]),
    }
    for u, es in units:
        title, cols = heads[u]
        o += ["## " + title % len(es), "", "| " + " | ".join(cols) + " |", "|" + "---|" * len(cols)]
        for e in es:
            cell = {
                "id": "`%s`" % e["id"], "kind": "`%s`" % e["json"],
                "paths": "<br>".join("`%s`" % p for p in e.get("paths", [])[:4]) + (" …" if len(e.get("paths", [])) > 4 else ""),
                "v": e.get("version", ""), "class": e["class"] + e["flag"], "tags": e["tags"],
                "at": "`%s`" % e["at"], "value at": e["value_at"], "why": e["why"], "RP2350": e["rp2350"], "cites": e.get("cites", ""),
            }
            o.append("| " + " | ".join(md(cell[c]) for c in cols) + " |")
            if u == "K" and e.get("variants"):
                vs = ", ".join("`%s`%s" % (v, "" if i["used"] else " (unused)") for v, i in e["variants"].items())
                o.append("| ↳ values | %s |%s" % (md(vs), " |" * (len(cols) - 2)))
        o.append("")
    (here / "LIST.md").write_text("\n".join(o) + "\n")


def write_count(inv, units, here):
    o = ["# AET-WO-1219 — the count", "", "Generated by `reader.py`; do not edit.", ""]
    o += ["## Per class", "", "| unit | V | N | S | D | flagged ? | total |", "|---|---|---|---|---|---|---|"]
    for u, es in units:
        c = collections.Counter(e["class"] for e in es)
        q = sum(1 for e in es if e["flag"])
        o.append("| %s | %d | %d | %d | %d | %d | %d |" % (u, c["V"], c["N"], c["S"], c["D"], q, len(es)))
    o += ["", "## K per version", "",
          "The version that introduced each key: the latest `versions::CHANGES` row naming the key's path or a prefix of it; a key no row names is version 1. "
          "A renamed key takes its rename's version. A key with several paths takes its earliest.", "",
          "| version | work order | V | N | S | total |", "|---|---|---|---|---|---|"]
    byv = collections.defaultdict(list)
    for e in inv["K"]:
        byv[(e["version"], e["version_wo"])].append(e)
    for (v, wo), es in sorted(byv.items()):
        c = collections.Counter(e["class"] for e in es)
        o.append("| %d | %s | %d | %d | %d | %d |" % (v, wo, c["V"], c["N"], c["S"], len(es)))
    o += ["", "## R per version", "",
          "A refusal kind a `versions::CHANGES` row lists under `rules` takes that row's version; every other kind is \"no row\" (a row lists only kinds that judge content an older file could already hold).", "",
          "| version | V | N | S | total |", "|---|---|---|---|---|"]
    byr = collections.defaultdict(list)
    for e in inv["R"]:
        byr[e.get("rule_row") or 0].append(e)
    for v, es in sorted(byr.items()):
        c = collections.Counter(e["class"] for e in es)
        o.append("| %s | %d | %d | %d | %d |" % (v or "no row", c["V"], c["N"], c["S"], len(es)))
    (here / "COUNT.md").write_text("\n".join(o) + "\n")


if __name__ == "__main__":
    main()
