# AET-WO-4300j — mutants for the adversarial-pass fixes

Each mutant was run against `cargo test -p aethelgard-ir --test motor the_topology_refuses_a_pull`.

| id | guard (F1, the topology reader) | mutation | killed |
|---|---|---|---|
| F1a | the link is an identifier | the `plain(link)` check removed | yes |
| F1b | the pull's references are distinct from every other | the link left out of `references_distinct` | yes |
| F1c | floor ≤ ceiling | the clause removed | **no**: it was implied by the containment check, so the clause was deleted |
| F1d | `ohms` inside the window | the clause removed | yes |
| F1e | floor ≥ 1 | the clause removed | yes |

## F2 and F3 (ruled 2026-11-23T12:25Z)

Each mutant was run against `cargo test -p aethelgard-ir --test motor --test offboard`.

| id | guard | mutation | killed |
|---|---|---|---|
| F2a | ESC VIH on the peer rail's maximum | the board's rail instead | yes |
| F2b | ESC VIL on the peer rail's minimum | the board's rail instead | yes |
| F3a | the low corner clears the floor | the bare value against the floor | yes |
| F3b | the high corner clears the ceiling | the bare value against the ceiling | yes |
| F3c | the band is tolerance + drift | tolerance alone | yes |
| R1 | the peer tolerance is below 1000 permille | `> 1000` | yes |
