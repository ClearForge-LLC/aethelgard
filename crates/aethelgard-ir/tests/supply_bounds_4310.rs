//! `AET-WO-4310` (3): the supply bounds, loaded. (3b) each battery-fed
//! regulator's headroom counts the pack's declared series resistance beside
//! the FET's and the fuse's (`2304b` K6).

use aethelgard_ir::power::{self, Refusal};
use serde_json::{Value, json};

#[path = "common/power_table.rs"]
mod power_table;
use power_table::{assert_power, refused, sentinel_03, spec};

fn conditions(v: &Value) -> Vec<String> {
    power::check(&spec(v))
        .expect("checks")
        .expect("a tree")
        .conditions
        .into_iter()
        .collect()
}

#[test]
fn the_headroom_counts_the_packs_declared_series_with_its_source() {
    let c = conditions(&sentinel_03());
    let q1 = c
        .iter()
        .find(|t| t.starts_with("Q1: the input path's drop is"))
        .expect("Q1's drop");
    assert!(
        q1.contains("the input path's drop is 84 mV at 1916 mA")
            && q1.contains("and the pack's series 1000 µΩ (J4 XT30PW-M"),
        "{q1}"
    );
    assert!(
        c.iter().any(|t| t.contains("the input path's drop, 84 mV")),
        "each buck's headroom note carries the drop"
    );
    let screened = power::check(&spec(&sentinel_03()))
        .unwrap()
        .unwrap()
        .screened;
    assert!(!screened.contains_key("headroom_pack_series_undeclared:J4"));
}

#[test]
fn a_pack_series_that_eats_a_bucks_headroom_is_refused_and_its_clean_twin_passes() {
    // 1.5 Ω ahead of the board: about 3 V at the branch's current, past U10's
    // 2.98 V of margin at the pack's 9 V minimum, while the FET's gate still
    // has a stated RDS(on).
    let mut v = sentinel_03();
    v["power"]["battery"]["series_milliohms"]["value"] = json!(1500);
    assert_power(&refused(&v), Refusal::BuckHeadroomViolated, "U10");
    let mut twin = sentinel_03();
    twin["power"]["battery"]["series_milliohms"]["value"] = json!(1);
    spec(&twin).validate().expect("the declared 1 mΩ passes");
}

#[test]
fn an_undeclared_pack_series_is_named_never_taken_as_zero_silently() {
    let mut v = sentinel_03();
    v["power"]["battery"]["series_milliohms"] = Value::Null;
    let c = power::check(&spec(&v)).expect("checks").expect("a tree");
    assert!(
        c.screened
            .contains_key("headroom_pack_series_undeclared:J4"),
        "{:?}",
        c.screened.keys()
    );
}

#[test]
fn a_pack_series_that_starves_the_fets_gate_is_refused_before_the_headroom() {
    // 2.5 Ω: the gate's drive, the pack's minimum less the pack's and the
    // fuse's drop, falls under every |VGS| DMP6023LSS states an RDS(on) at.
    let mut v = sentinel_03();
    v["power"]["battery"]["series_milliohms"]["value"] = json!(2500);
    let e = refused(&v);
    assert_power(&e, Refusal::ProtectionPartOutOfRange, "Q1");
    assert!(
        e.to_string().contains("gate drive at the pack's minimum"),
        "{e}"
    );
}
