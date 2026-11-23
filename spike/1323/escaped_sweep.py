"""AET-WO-1323 adversarial A1: append a sweep of 72 escaped-pair boards to
route.rs's tests (pitch 0.4/0.35/0.3 mm, pads 0.2/0.15 mm, /P and /N 2-4 pads
apart, neighbours or not, at index 2 or 0), optionally with `prune` disabled.
Usage: python3 -I escaped_sweep.py <route.rs> [noprune]. Never committed applied."""
import sys
p = sys.argv[1]
s = open(p).read()
if len(sys.argv) > 2:
    a = "        let mut any = false;\n        loop {\n            let mut cut = false;"
    assert s.count(a) == 1
    s = s.replace(a, "        if true {\n            return false;\n        }\n" + a)
assert s.endswith("}\n")
s = s[:-2] + '''
    #[test]
    #[ignore]
    fn spike_escaped_pair_sweep() {
        for pitch in [400_000i64, 350_000, 300_000] {
            for w in [200_000i64, 150_000] {
                for gap in [2usize, 3, 4] {
                    for nb in [false, true] {
                        for idx in [2usize, 0] {
                            let facing = |k: usize| -> Option<String> {
                                if k == idx { Some("/P".into()) } else if k == idx + gap { Some("/N".into()) } else if nb { Some(format!("/S{k}")) } else { None }
                            };
                            let soic = |reference: &str, cx: i64, left: bool| {
                                let mut pads = Vec::new();
                                for col in [-1i64, 1] {
                                    for k in 0..8usize {
                                        let y = 6 * MM + (k as i64) * pitch - 7 * pitch / 2;
                                        let x = cx + col * 2 * MM;
                                        let net = if (col == 1) == left { facing(k) } else { None };
                                        let number = (pads.len() + 1).to_string();
                                        pads.push(pad(&number, net.as_deref(), rect(x - MM / 2, y - w / 2, x + MM / 2, y + w / 2)));
                                    }
                                }
                                Part { rotation: 0, reference: reference.into(), pads, courtyard: Vec::new(), keepouts: Vec::new() }
                            };
                            let mut stack = crate::stack::reference();
                            stack.pairs.push(crate::stack::Pair { name: "X".into(), nets: ["/P".into(), "/N".into()], gap_nm: 300_000, width_nm: 200_000 });
                            let board = Board { outline: rect(0, 0, 24 * MM, 12 * MM), parts: vec![soic("A", 5 * MM, true), soic("B", 19 * MM, false)], stack };
                            let r = route(&board);
                            println!("{pitch} {w} {gap} {nb} {idx}: {}", match &r { Ok(_) => "ok".to_string(), Err(e) => format!("{} {}", e.net, e.why.chars().take(110).collect::<String>()) });
                        }
                    }
                }
            }
        }
    }
}
'''
open(p, "w").write(s)
