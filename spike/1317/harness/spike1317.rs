//! AET-WO-1317 spike harness (scratch copy only): parts-level removal, trace
//! and knock-outs over the bounded placer.
use std::collections::BTreeMap;

use aethelgard_ir::{PinTable, SolvedTopology, Specification};

use crate::pcb::{self, knob};
use crate::schematic::{self, EmitError, Part};

pub struct Case {
    pub class: String,
    pub w_nm: i64,
    pub h_nm: i64,
    pub message: String,
    pub trace: Vec<String>,
    /// reference -> "x,y,rot"
    pub placed: BTreeMap<String, String>,
}

#[derive(Clone, Default)]
pub struct Knobs {
    pub no_k: bool,
    pub no_m: bool,
    pub no_s: bool,
    pub order: Option<Vec<String>>,
    pub salt: u64,
    pub rounds: Option<usize>,
}

fn parts_of(
    spec: &Specification,
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<(Vec<Part>, crate::stack::Stack), EmitError> {
    spec.validate().map_err(EmitError::InvalidSpecification)?;
    let (parts, _) = schematic::design(spec, solved, table)?;
    let stack = crate::stack::Stack::of(spec).map_err(EmitError::FabToleranceUndeclared)?;
    Ok((parts, stack))
}

fn is_mcu(p: &Part) -> bool {
    p.lib_id.starts_with("MCU_")
}

/// Each removal: a non-MCU part with every part it owns.
/// With `closure`, also every part left with a label net no remaining part
/// shares (R5, U2's SDO strap), repeatedly.
pub fn removals(spec: &Specification, solved: &SolvedTopology, table: &PinTable, closure: bool) -> Vec<Vec<String>> {
    let (parts, _) = parts_of(spec, solved, table).unwrap();
    parts
        .iter()
        .filter(|p| !is_mcu(p))
        .map(|p| {
            let mut g = vec![p.reference.clone()];
            g.extend(
                parts
                    .iter()
                    .filter(|q| q.owner.as_ref().is_some_and(|o| o.ic == p.reference))
                    .map(|q| q.reference.clone()),
            );
            while closure {
                let left: Vec<&Part> = parts.iter().filter(|q| !g.contains(&q.reference)).collect();
                let nets: Vec<(String, Vec<String>)> = left
                    .iter()
                    .map(|q| (q.reference.clone(), pcb::pin_nets(q).unwrap().into_values().collect()))
                    .collect();
                let orphan = nets.iter().find(|(r, ns)| {
                    let label: Vec<&String> = ns.iter().filter(|n| n.starts_with('/')).collect();
                    !label.is_empty()
                        && label.iter().all(|n| {
                            nets.iter().filter(|(o, ons)| o != r && ons.contains(n)).count() == 0
                        })
                        && !left.iter().any(|q| &q.reference == r && is_mcu(q))
                });
                match orphan {
                    Some((r, _)) => g.push(r.clone()),
                    None => break,
                }
            }
            g
        })
        .collect()
}

/// reference -> a name-free identity (symbol, footprint, pin nets, owner's
/// reference for ICs is kept: IC references were not renumbered).
pub fn identities(spec: &Specification, solved: &SolvedTopology, table: &PinTable) -> BTreeMap<String, String> {
    let (parts, _) = parts_of(spec, solved, table).unwrap();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = BTreeMap::new();
    for p in &parts {
        let nets = pcb::pin_nets(p).unwrap();
        let owner = p.owner.as_ref().map(|o| {
            format!(
                "{}:{}:{}",
                o.ic,
                o.decouples,
                o.anchor.as_ref().map(|a| format!("{:?}", a.pads)).unwrap_or_default()
            )
        });
        let base = format!("{}|{}|{:?}|{:?}", p.lib_id, p.footprint, nets, owner);
        let n = seen.entry(base.clone()).or_default();
        *n += 1;
        out.insert(p.reference.clone(), format!("{base}#{n}"));
    }
    out
}

pub fn run(
    spec: &Specification,
    solved: &SolvedTopology,
    table: &PinTable,
    remove: &[String],
    trace: bool,
    knobs: &Knobs,
) -> Case {
    let (mut parts, stack) = match parts_of(spec, solved, table) {
        Ok(x) => x,
        Err(e) => {
            return Case {
                class: format!("design:{}", e.kind()),
                w_nm: 0,
                h_nm: 0,
                message: e.to_string(),
                trace: vec![],
                placed: BTreeMap::new(),
            };
        }
    };
    parts.retain(|p| !remove.contains(&p.reference));
    if let Ok(list) = std::env::var("UNANCHOR") {
        for p in parts.iter_mut().filter(|p| list.split(',').any(|r| r == p.reference)) {
            p.owner = None;
        }
    }
    let noreserve: Vec<String> = std::env::var("NORESERVE").map(|l| l.split(',').map(String::from).collect()).unwrap_or_default();
    knob::NORESERVE.with(|n| *n.borrow_mut() = noreserve);
    knob::NO_K.with(|c| c.set(knobs.no_k));
    knob::NO_M.with(|c| c.set(knobs.no_m));
    knob::NO_S.with(|c| c.set(knobs.no_s));
    knob::ORDER.with(|o| *o.borrow_mut() = knobs.order.clone());
    knob::SALT.with(|c| c.set(knobs.salt));
    knob::ROUNDS.with(|c| c.set(knobs.rounds));
    knob::TRACE.with(|t| *t.borrow_mut() = trace.then(Vec::new));
    let bound = spec.board.max_outline_um;
    let pairs: Vec<[String; 2]> = stack.pairs.iter().map(|p| p.nets.clone()).collect();
    let result = (|| {
        if let Some(b) = bound {
            pcb::bound_holds(&parts, b)?;
        }
        let items = pcb::items(&parts)?;
        pcb::layout_of(&items, &[], &pairs, bound)
    })();
    let trace = knob::TRACE.with(|t| t.borrow_mut().take()).unwrap_or_default();
    knob::NO_K.with(|c| c.set(false));
    knob::NO_M.with(|c| c.set(false));
    knob::NO_S.with(|c| c.set(false));
    knob::ORDER.with(|o| *o.borrow_mut() = None);
    knob::SALT.with(|c| c.set(0));
    knob::ROUNDS.with(|c| c.set(None));
    match result {
        Ok(l) => Case {
            class: "fit".into(),
            w_nm: l.outline.x1 - l.outline.x0,
            h_nm: l.outline.y1 - l.outline.y0,
            message: String::new(),
            trace,
            placed: l
                .placed
                .iter()
                .map(|p| (p.reference.clone(), format!("{},{},{}", p.origin.0, p.origin.1, p.rotation)))
                .collect(),
        },
        Err(e) => {
            let (w, h) = match &e {
                EmitError::OutlineUnreachable { reached_um_w, reached_um_h, .. } => {
                    (i64::from(*reached_um_w) * 1000, i64::from(*reached_um_h) * 1000)
                }
                _ => (0, 0),
            };
            Case {
                class: e.kind().to_string(),
                w_nm: w,
                h_nm: h,
                message: e.to_string(),
                trace,
                placed: BTreeMap::new(),
            }
        }
    }
}

/// Today's placement; on a refusal, each alternative in turn, the first that
/// fits winning. Returns the case and which alternative ("-" for today's).
pub fn run_fallback(
    spec: &Specification,
    solved: &SolvedTopology,
    table: &PinTable,
    remove: &[String],
    base: &Knobs,
    alts: &[Knobs],
    names: &[String],
) -> (Case, String) {
    let first = run(spec, solved, table, remove, false, base);
    if first.class == "fit" || !first.class.starts_with("Outline") && first.class != "CompactionUnbounded" {
        return (first, "-".into());
    }
    for (k, n) in alts.iter().zip(names) {
        let c = run(spec, solved, table, remove, false, k);
        if c.class == "fit" {
            return (c, n.clone());
        }
    }
    (first, "none".into())
}
