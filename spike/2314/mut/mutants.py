"""AET-WO-2314: one mutant per guard this work order adds, each run against the suite its test lives in
(host, CARGO_TARGET_DIR given). Each mutant replaces one exact text (it must occur once); the file is restored
with `git checkout --` (a fresh mtime, so cargo rebuilds: spike/2313's lesson). A mutant is killed when its
suite fails. python3 -I mutants.py [ID...]"""
import subprocess, sys, os
REPO = "/home/forge/aethelgard-2314"
C = "crates/aethelgard-cli/src/"
T_CAMPAIGN = ["--test", "campaign_tier1", "an_undisturbed_boot_with_no_boot_word_names_its_end"]
T_BOOT = ["--test", "boot_tier1", "foreign"]
T_UNIT = ["--bin", "aethelgard", "foreign::tests"]
M = [
    ("C1", C + "campaign.rs", '.filter(|f| f.severity != "info" || f.kind == "renode_log_flood")', '.filter(|f| f.severity != "info")', T_CAMPAIGN),
    ("C2", C + "campaign.rs", "if let Some(m) = v.memory.as_ref().filter(|_| bounded) {", "if let Some(m) = v.memory.as_ref().filter(|_| false) {", T_CAMPAIGN),
    ("C3", C + "campaign.rs", "    if !flood\n", "    if true\n", T_CAMPAIGN),
    ("C4", C + "campaign.rs", "        end.push(format!(\n            \"its boot word reads", "        end.truncate(end.len());\n        drop(format!(\n            \"its boot word reads", T_CAMPAIGN),
    ("C5", C + "campaign.rs", "to place it from: {end}\"", "to place it from\"", T_CAMPAIGN),
    ("C6", C + "campaign.rs", "let mut end: Vec<String> = v\n        .findings\n        .iter()", "let mut end: Vec<String> = v\n        .findings\n        .iter()\n        .take(0)", T_CAMPAIGN),
    ("F1", C + "renode/foreign.rs", "if word == 0 || !executable", "if !executable", T_BOOT),
    ("F2", C + "renode/foreign.rs", "if word == 0 || !executable.iter().any(|&(s, e)| (s..e).contains(&address)) {", "if word == 0 {", T_BOOT),
    ("F3", C + "renode/foreign.rs", "ok = v in ({vectors});", "ok = True;", T_UNIT),
    ("F4", C + "renode/foreign.rs", "f + 24))); ok and setattr(self, 'IsHalted', True)", "f + 24)))", T_UNIT),
    ("F5", C + "renode/foreign.rs", "lr = r(self.LR); f = r(self.OtherSP) if lr & 4 else r(self.SP);", "lr = r(self.LR); f = r(self.SP);", T_UNIT),
    ("F6", C + "renode/foreign.rs", "if f.next().is_some() || !handlers.contains(&(n, handler)) {", "if f.next().is_some() {", T_BOOT),
    ("F7", C + "renode/foreign.rs", "        PANIC_MARK,\n        FAULT_MARK,\n", "        PANIC_MARK,\n", T_UNIT),
    ("F8", C + "renode/foreign.rs", "    c.extend(fault_hooks(faults));\n", "", T_BOOT),
    ("F9", C + "renode/foreign.rs", "            .map_or(\"a fault\", |v| v.1);", "            .map_or(\"a fault\", |_| \"HardFault\");", T_BOOT),
    ("F10", C + "renode/foreign.rs", "    if let Some((n, stacked, ns)) = lines.iter().find_map(|line| fault_entry(line, faults)) {", "    if let Some((n, stacked, ns)) = lines.iter().find_map(|line| fault_entry(line, faults)).filter(|_| false) {", T_BOOT),
    ("D1", C + "renode/foreign.rs", "    discard: unread_access_line,\n", "", T_UNIT),
    ("D2", C + "renode/foreign.rs", "                a.write\n                    && usart_tdr()", "                usart_tdr()", T_UNIT),
    ("D3", C + "renode/foreign.rs", "usart_tdr().is_some_and(|tdr| a.offset & !3 == tdr)", "usart_tdr().is_some_and(|tdr| a.offset == tdr)", T_UNIT),
    ("D4", C + "renode/foreign.rs", "                    && usarts().contains(&a.source.as_str())\n", "\n", T_UNIT),
]
want = set(sys.argv[1:])
env = dict(os.environ)
for mid, path, old, new, test in M:
    if want and mid not in want:
        continue
    full = os.path.join(REPO, path)
    s = open(full).read()
    assert s.count(old) == 1, f"{mid}: the text occurs {s.count(old)} times"
    open(full, "w").write(s.replace(old, new))
    try:
        r = subprocess.run(["nice", "-n", "10", "cargo", "test", "-q", "-p", "aethelgard-cli"] + test,
                           cwd=REPO, env=env, capture_output=True, text=True)
        out = r.stdout + r.stderr
        compiled = "error[" not in out
        print(f"MUTANT {mid}: {'KILLED' if r.returncode != 0 else 'SURVIVED'}"
              f"{'' if compiled else ' (did not compile)'} ({path}; {' '.join(test)})", flush=True)
    finally:
        subprocess.run(["git", "checkout", "--", path], cwd=REPO, check=True)
        os.utime(full)
