# AET-WO-4411: the specification version bump for `radio_modules`, run after
# #390 (which takes 30). Reads the current version from lib.rs, bumps it by
# one, appends the versions table's row, and moves every committed
# specification's top-level schema_version (a JSON object with `project_name`
# and `mcu`) from the old version to the new, editing only that one line.
# Usage: python3 -I spike/4411/build/bump.py <repo>
import json
import pathlib
import re
import subprocess
import sys

repo = pathlib.Path(sys.argv[1])
lib = repo / "crates/aethelgard-ir/src/lib.rs"
s = lib.read_text()
m = re.search(r"pub const SPECIFICATION_SCHEMA_VERSION: u32 = (\d+);", s)
old = int(m.group(1))
new = old + 1
lib.write_text(s.replace(m.group(0), f"pub const SPECIFICATION_SCHEMA_VERSION: u32 = {new};"))

versions = repo / "crates/aethelgard-ir/src/versions.rs"
v = versions.read_text()
anchor = "\n];\n\n/// A row with nothing removed;"
assert v.count(anchor) == 1
rules = [
    "RadioPortUndeclared", "RadioPortRole", "RadioPortShared", "RadioModuleTwice",
    "RadioPartUnknown", "RadioBoardLayers", "RadioSupplyUnknown", "RadioSupplyOutOfRange",
    "RadioSupplyNotMcu", "RadioSupplyUnsourced", "RadioEnableNotPulled", "RadioStrapUndeclared",
    "RadioStrapUnknown", "RadioStrapPullTo", "RadioStrapUndriven", "RadioStrapWrongLevel",
    "RadioDecouplingUndeclared", "RadioPassivePartUnknown", "RadioPullTooWeak",
    "RadioEnableTooFast", "RadioDecouplingShort", "RadioTableMalformed",
]
row = (
    f"\n    row(\n        {new},\n        \"AET-WO-4411\",\n        &[],\n        &[\"radio_modules\"],\n        &[],\n        &[\n"
    + "".join(f"            \"{r}\",\n" for r in rules)
    + "        ],\n    ),"
)
versions.write_text(v.replace(anchor, row + anchor))

files = subprocess.run(["git", "-C", str(repo), "ls-files", "*.json"], capture_output=True, text=True).stdout.split()
moved = []
for f in files:
    p = repo / f
    text = p.read_text()
    try:
        doc = json.loads(text)
    except ValueError:
        continue
    if not (isinstance(doc, dict) and "project_name" in doc and "mcu" in doc and doc.get("schema_version") == old):
        continue
    pat = re.compile(r'("schema_version"\s*:\s*)' + str(old) + r"\b")
    found = pat.findall(text)
    if len(found) != 1:
        sys.exit(f"{f}: {len(found)} schema_version lines")
    p.write_text(pat.sub(lambda mm: mm.group(1) + str(new), text, count=1))
    moved.append(f)
print(f"{old} -> {new}: {len(moved)} specifications")
for f in moved:
    print(" ", f)
