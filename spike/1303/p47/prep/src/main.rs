//! Usage: p47-prep <spec.json> <board.kicad_pcb> <out_dir>
//! Writes out_dir/input.kicad_pcb, input.kicad_pro, input.kicad_dru as the
//! DRC gate prepares them (no library tables: plotting needs none).
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let spec: aethelgard_ir::Specification =
        serde_json::from_str(&std::fs::read_to_string(&a[1]).unwrap()).unwrap();
    spec.validate().unwrap();
    let text = std::fs::read_to_string(&a[2]).unwrap();
    let tree = aethelgard_eda::board::read(&text).unwrap();
    let board =
        aethelgard_eda::board::prepare_read(&tree, &text, aethelgard_eda::rules::SETUP).unwrap();
    let rules = aethelgard_eda::rules::Rules::from_specification(&spec);
    let out = std::path::Path::new(&a[3]);
    std::fs::create_dir_all(out).unwrap();
    std::fs::write(out.join("input.kicad_pcb"), board).unwrap();
    std::fs::write(out.join("input.kicad_pro"), rules.project("input")).unwrap();
    std::fs::write(out.join("input.kicad_dru"), rules.custom_rules()).unwrap();
}
