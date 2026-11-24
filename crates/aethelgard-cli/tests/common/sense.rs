//! `AET-WO-4410` Q11–Q12 (ruled 2026-11-24): a test whose purpose is not the
//! battery sense keeps the sense where it measured it, on the ADC3 pin the
//! solver chose before S2's ADC1-first rule.

use serde_json::{Value, json};

/// `v` with every ADC1-capable pin no other net of its solution holds
/// declared in `mcu.reserved_pins`, so the sense falls back to the free ADC3
/// pin of lowest position, as before S2. Reserving only the ADC1 pin the
/// sense took would move it to the next ADC1 pin (measured: PC0 reserved
/// gives PC1 on Sentinel-03), not back. A pin another net holds is left
/// alone, so no other net moves.
pub fn keep_off_adc1(mut v: Value) -> Value {
    let spec: aethelgard_ir::Specification = serde_json::from_value(v.clone()).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    let senses: Vec<&String> = solved.power.iter().flat_map(|p| p.sense.keys()).collect();
    let held: Vec<&str> = solved
        .assignments
        .values()
        .map(|a| a.pin.as_str())
        .chain(solved.gpio.values().map(|g| g.pin.as_str()))
        .chain(
            solved
                .adc_inputs
                .iter()
                .filter(|(net, _)| !senses.contains(net))
                .map(|(_, a)| a.pin.as_str()),
        )
        .collect();
    let reserved = v["mcu"]["reserved_pins"].as_array_mut().unwrap();
    for (name, pin) in &table.pins {
        let adc1 = pin.adc_channels.iter().any(|c| c.starts_with("ADC1_"));
        let declared = reserved.iter().any(|r| r["pin"] == name.as_str());
        if adc1 && !declared && !held.contains(&name.as_str()) {
            reserved.push(json!({"pin": name, "reason": "test: the sense kept off ADC1, where this test measured it (AET-WO-4410 Q11-Q12)"}));
        }
    }
    v
}
