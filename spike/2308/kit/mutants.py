#!/usr/bin/env python3
"""AET-WO-2308 mutants: one per guard this work order adds or changes.

Each mutant is (id, file, old, new, cargo test args). The tree is a copy
(TREE), never the worktree. A mutant is applied by rewriting its file (a new
mtime, so cargo rebuilds; the rsync-mtime lesson) and reverted the same way.
It is KILLED when its tests fail and the build succeeds, BUILD when the
build breaks (rewrite the mutant), SURVIVED when its tests pass.

Usage: mutants.py <tree> <target dir> <out.json> [id ...]
Tier-2 mutants (marked) run only inside the tools image (img.sh).
"""
import json
import subprocess
import sys
import time

CLI = "crates/aethelgard-cli/src"
EDA = "crates/aethelgard-eda/src"
BIN = ["-p", "aethelgard-cli", "--bin", "aethelgard"]
LIB = ["-p", "aethelgard-cli", "--lib"]

MUTANTS = [
    ("M1-group-key", f"{EDA}/fab.rs", "let key = (value, name, note);",
     "let key = (value, name, String::new());", ["-p", "aethelgard-eda", "--lib", "--", "fab::"]),
    ("M2-note-column", f"{EDA}/fab.rs", "            csv_field(note, false),",
     "            csv_field(\"\", false),", ["-p", "aethelgard-eda", "--lib", "--", "fab::"]),
    ("M3-noted-filter", f"{EDA}/fab.rs", ".filter(|[.., note]| !note.is_empty())",
     ".filter(|[.., _note]| true)", ["-p", "aethelgard-eda", "--lib", "--", "fab::"]),
    ("M4-section-none", f"{CLI}/fab.rs", "    if parts.is_empty() {\n        return Ok(String::new());\n    }",
     "    if !parts.is_empty() {\n        return Ok(String::new());\n    }", BIN + ["--", "fab::tests::the_notes_list"]),
    ("M5-standin-refusal", f"{CLI}/verify.rs", "if let Some((reference, _)) = drawn.first() {",
     "if let Some((reference, _)) = drawn.first().filter(|_| false) {",
     ["-p", "aethelgard-cli", "--test", "fab_tier1", "--", "a_drawn_twin_stand_in"]),
    ("M6-standin-predicate", f"{CLI}/verify.rs", ".filter(|(_, part)| table.get(part).is_some_and(|p| p.stand_in))",
     ".filter(|(_, part)| table.get(part).is_some_and(|p| p.stand_in && false))",
     ["-p", "aethelgard-cli", "--test", "fab_tier1", "--", "a_drawn_twin_stand_in"]),
    ("M7-renode-agree", f"{CLI}/verify.rs", "Some(b) if *b != named => {",
     "Some(b) if *b != named && false => {", BIN + ["--", "the_campaign_names_the_renode"]),
    ("M8-renode-missing", f"{CLI}/verify.rs",
     "            .ok_or_else(|| {\n                fail(format!(\n                    \"the archive of run {} names no Renode build\",\n                    run.run_id\n                ))\n            })?;",
     "            .unwrap_or_default();", BIN + ["--", "the_campaign_names_the_renode"]),
    ("M9-against-walk", f"{CLI}/tree.rs", "            && entries == 0\n",
     "            && entries == usize::MAX\n", BIN + ["--", "tree::tests"]),
    ("M17-against-top", f"{CLI}/tree.rs", "            && !rel.is_empty()\n", "",
     BIN + ["--", "tree::tests"]),
    ("M10-against-callsite", f"{CLI}/verify.rs", "tree::against(dir, &files, &[&format!(\"{BOARD_DIR}/target\")])",
     "tree::diff(dir, &files, &[&format!(\"{BOARD_DIR}/target\")])",
     ["-p", "aethelgard-cli", "--test", "fab_tier1", "--", "a_tree_that_differs"]),
    ("M11-exit-fab-mode", f"{CLI}/verify.rs", "    Ok(if report.ok { 0 } else { EXIT_FAIL })\n}",
     "    Ok(if report.ok || fab_mode.is_some() { 0 } else { EXIT_FAIL })\n}", BIN + ["--", "the_exit_code_is"]),
    ("M12-exit-no-fab", f"{CLI}/verify.rs", "    Ok(if report.ok { 0 } else { EXIT_FAIL })\n}",
     "    Ok(if report.ok || fab_mode.is_none() { 0 } else { EXIT_FAIL })\n}", BIN + ["--", "the_exit_code_is"]),
    ("M13-elf-binding", f"{CLI}/fab.rs", "    if tree::sha256(&elf) != b.elf_sha256 {",
     "    if tree::sha256(&elf) != b.elf_sha256 && false {", BIN + ["--", "a_swapped_elf"]),
    ("M14-board-binding", f"{CLI}/fab.rs", "    if input.sha256 != b.board_sha256 {",
     "    if input.sha256 != b.board_sha256 && false {", BIN + ["--", "a_swapped_elf"]),
    ("M15-ledger-class", f"{CLI}/ledger.rs", '("CR", "HSIKERON", Ign,', '("CR", "HSIKERON", Rule,',
     LIB + ["--", "ledger::tests"]),
    ("M16-ledger-pllsrc", f"{CLI}/ledger.rs", "            pll_rule,\n            &pll_stored,\n            &format!(\"{word}, {what}: PLLSRC's bits",
     "            &format!(\"{held}\"),\n            &pll_stored,\n            &format!(\"{word}, {what}: PLLSRC's bits", LIB + ["--", "ledger::tests"]),
    # Tier 2 (the tools image): the call sites only a full verify reaches.
    ("T1-section-callsite", f"{CLI}/fab.rs", "    let notes = notes + &noted(b.project, b.bom)?;\n",
     "    let notes = notes + \"\";\n", ["-p", "aethelgard-cli", "--features", "with-tools", "--test", "verify_2k2_tier2", "--", "verify_fab_out"]),
    ("T2-campaign-version", f"{CLI}/verify.rs", "    g.tool_version = Some(campaign_renode(Path::new(&p.archive), &out)?);\n",
     "", ["-p", "aethelgard-cli", "--features", "with-tools", "--test", "verify_2k2_tier2", "--", "verify_binds"]),
    ("T3-against-fab-callsite", f"{CLI}/verify.rs", "let differences = tree::against(dir, &package, &[]).map_err(crate::stopped)?;",
     "let differences = tree::diff(dir, &package, &[]).map_err(crate::stopped)?;",
     ["-p", "aethelgard-cli", "--features", "with-tools", "--test", "against_fab_tier2", "--", "verify_against_fab_names_every"]),
]


def cargo(tree, target, args):
    cmd = ["cargo", "test", "--target-dir", target, "-j", "6"] + args
    t = time.time()
    p = subprocess.run(cmd, cwd=tree, capture_output=True, text=True)
    return p.returncode, p.stdout + p.stderr, round(time.time() - t, 1)


def main():
    tree, target, out = sys.argv[1:4]
    only = set(sys.argv[4:])
    results = []
    for mid, path, old, new, args in MUTANTS:
        if only and mid not in only:
            continue
        full = f"{tree}/{path}"
        text = open(full).read()
        if text.count(old) != 1:
            results.append({"id": mid, "outcome": "ANCHOR", "count": text.count(old)})
            continue
        open(full, "w").write(text.replace(old, new))
        try:
            code, log, secs = cargo(tree, target, args)
        finally:
            open(full, "w").write(text)
        built = "error[E" not in log and "could not compile" not in log
        outcome = "BUILD" if not built else ("KILLED" if code != 0 else "SURVIVED")
        failed = [l for l in log.splitlines() if l.startswith("test ") and l.endswith("FAILED")]
        results.append({"id": mid, "outcome": outcome, "secs": secs, "failed": failed[:5]})
        print(mid, outcome, secs, failed[:2], flush=True)
    json.dump(results, open(out, "w"), indent=1)


main()
