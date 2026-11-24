// SCRATCH (AET-WO-4300k acks): never committed. Prints Sentinel-03's unacknowledged ledger rows.
use sha2::{Digest, Sha256};
#[test]
fn print_unacknowledged_rows() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(root.join("examples/sentinel-03.json")).unwrap();
    let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let t = aethelgard_ir::solve(&spec, &table, &sha).unwrap();
    let mut l = aethelgard_cli::ledger::build(&t, &table).unwrap();
    let _un = aethelgard_cli::ledger::acknowledge(&mut l, &spec);
    let rows: Vec<_> = l
        .rows
        .iter()
        .filter(|r| r.class.unwitnessed().is_some())
        .collect();
    std::fs::write(
        "/home/forge/.cache/aet-4300k/acks/rows.json",
        serde_json::to_string_pretty(&rows).unwrap(),
    )
    .unwrap();
    let bench: Vec<String> = l
        .rows
        .iter()
        .filter(|r| r.acknowledged == Some(false))
        .map(|r| {
            format!(
                "{} {} {} bench={} addr={:?}",
                r.id,
                r.class.name(),
                r.access,
                aethelgard_cli::ledger::on_the_bench(&l, r),
                r.address
            )
        })
        .collect();
    std::fs::write(
        "/home/forge/.cache/aet-4300k/acks/bench.txt",
        bench.join("\n"),
    )
    .unwrap();
    let all: Vec<String> = l
        .rows
        .iter()
        .filter(|r| {
            r.register.starts_with("RCC.CR")
                || r.register.starts_with("RCC.CFGR")
                || r.register.starts_with("GPIOC.")
                || r.register.starts_with("GPIOE.MODER")
                || r.register.starts_with("GPIOA.PUPDR")
        })
        .map(|r| {
            format!(
                "{} | {} | {} | v={:?} | bench={} | J: {} | N: {}",
                r.id,
                r.class.name(),
                r.access,
                r.value,
                aethelgard_cli::ledger::on_the_bench(&l, r),
                r.judge,
                r.note
            )
        })
        .collect();
    std::fs::write(
        "/home/forge/.cache/aet-4300k/acks/rcc_gpio.txt",
        all.join("\n"),
    )
    .unwrap();
}
