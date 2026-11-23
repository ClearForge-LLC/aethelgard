# AET-WO-4300j — mutants for the adversarial-pass fixes

Each mutant was run against `cargo test -p aethelgard-ir --test motor the_topology_refuses_a_pull`.

| id | guard (F1, the topology reader) | mutation | killed |
|---|---|---|---|
| F1a | the link is an identifier | the `plain(link)` check removed | yes |
| F1b | the pull's references are distinct from every other | the link left out of `references_distinct` | yes |
| F1c | floor ≤ ceiling | the clause removed | **no**: it was implied by the containment check, so the clause was deleted |
| F1d | `ohms` inside the window | the clause removed | yes |
| F1e | floor ≥ 1 | the clause removed | yes |
