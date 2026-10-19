use aethelgard_ir::{IrValidationError, NetConstraint, TopologyGraph, VoltageDomain};

fn valid() -> TopologyGraph {
    TopologyGraph {
        project_name: "sentinel-01".into(),
        voltage_domains: vec![VoltageDomain {
            id: "VDD_3V3".into(),
            nominal_millivolts: 3300,
            tolerance_permille: 50,
        }],
        net_constraints: vec![NetConstraint {
            net_name: "I2C1_SDA".into(),
            max_capacitance_femtofarads: 400_000,
            max_rise_time_nanoseconds: 300,
            impedance_ohms: None,
        }],
    }
}

fn refusal(mutate: impl FnOnce(&mut TopologyGraph)) -> IrValidationError {
    let mut graph = valid();
    mutate(&mut graph);
    graph.validate().expect_err("mutated graph must be refused")
}

fn second_domain(graph: &mut TopologyGraph, id: &str) {
    let mut domain = graph.voltage_domains[0].clone();
    domain.id = id.into();
    graph.voltage_domains.push(domain);
}

fn second_net(graph: &mut TopologyGraph, name: &str) {
    let mut net = graph.net_constraints[0].clone();
    net.net_name = name.into();
    graph.net_constraints.push(net);
}

#[test]
fn valid_graph_passes() {
    assert_eq!(valid().validate(), Ok(()));
}

#[test]
fn serde_round_trip_of_valid_graph() {
    let mut graph = valid();
    graph.net_constraints[0].impedance_ohms = Some(50);
    let json = serde_json::to_string(&graph).unwrap();
    let back: TopologyGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(back, graph);
    assert_eq!(back.validate(), Ok(()));
}

#[test]
fn refuses_empty_project_name() {
    assert_eq!(
        refusal(|g| g.project_name = String::new()),
        IrValidationError::EmptyProjectName
    );
    assert_eq!(
        refusal(|g| g.project_name = " \t\n".into()),
        IrValidationError::EmptyProjectName
    );
}

#[test]
fn refuses_empty_domain_id() {
    assert_eq!(
        refusal(|g| g.voltage_domains[0].id = "   ".into()),
        IrValidationError::EmptyDomainId { index: 0 }
    );
}

#[test]
fn refuses_duplicate_domain_id_including_whitespace_variants() {
    let expected = IrValidationError::DuplicateDomainId {
        id: "VDD_3V3".into(),
    };
    assert_eq!(refusal(|g| second_domain(g, "VDD_3V3")), expected);
    assert_eq!(refusal(|g| second_domain(g, " VDD_3V3\t")), expected);
}

#[test]
fn refuses_zero_nominal_voltage() {
    let err = refusal(|g| g.voltage_domains[0].nominal_millivolts = 0);
    assert_eq!(
        err,
        IrValidationError::ZeroNominalVoltage {
            id: "VDD_3V3".into()
        }
    );
    assert_eq!(err.kind(), "ZeroNominalVoltage");
    assert_eq!(err.id(), Some("VDD_3V3"));
}

#[test]
fn refuses_tolerance_out_of_range() {
    for value in [0, 501, u16::MAX] {
        assert_eq!(
            refusal(|g| g.voltage_domains[0].tolerance_permille = value),
            IrValidationError::ToleranceOutOfRange {
                id: "VDD_3V3".into(),
                value
            }
        );
    }
    for value in [1, 500] {
        let mut graph = valid();
        graph.voltage_domains[0].tolerance_permille = value;
        assert_eq!(graph.validate(), Ok(()));
    }
}

#[test]
fn refuses_empty_net_name() {
    assert_eq!(
        refusal(|g| g.net_constraints[0].net_name = String::new()),
        IrValidationError::EmptyNetName { index: 0 }
    );
}

#[test]
fn refuses_duplicate_net_name_including_whitespace_variants() {
    let expected = IrValidationError::DuplicateNetName {
        id: "I2C1_SDA".into(),
    };
    assert_eq!(refusal(|g| second_net(g, "I2C1_SDA")), expected);
    assert_eq!(refusal(|g| second_net(g, "I2C1_SDA ")), expected);
}

#[test]
fn refuses_zero_capacitance() {
    assert_eq!(
        refusal(|g| g.net_constraints[0].max_capacitance_femtofarads = 0),
        IrValidationError::ZeroCapacitance {
            id: "I2C1_SDA".into()
        }
    );
}

#[test]
fn refuses_zero_rise_time() {
    assert_eq!(
        refusal(|g| g.net_constraints[0].max_rise_time_nanoseconds = 0),
        IrValidationError::ZeroRiseTime {
            id: "I2C1_SDA".into()
        }
    );
}

#[test]
fn refuses_zero_impedance() {
    assert_eq!(
        refusal(|g| g.net_constraints[0].impedance_ohms = Some(0)),
        IrValidationError::ZeroImpedance {
            id: "I2C1_SDA".into()
        }
    );
}

const VALID_JSON: &str = r#"{"project_name":"p","voltage_domains":[{"id":"V","nominal_millivolts":3300,"tolerance_permille":50}],"net_constraints":[{"net_name":"N","max_capacitance_femtofarads":1,"max_rise_time_nanoseconds":1,"impedance_ohms":null}]}"#;

fn parse_error(json: &str) -> String {
    serde_json::from_str::<TopologyGraph>(json)
        .expect_err("must not deserialize")
        .to_string()
}

#[test]
fn valid_json_parses() {
    serde_json::from_str::<TopologyGraph>(VALID_JSON).unwrap();
}

#[test]
fn unknown_top_level_field_is_a_deserialization_error() {
    let err = parse_error(&VALID_JSON.replacen('{', r#"{"extra":1,"#, 1));
    assert!(err.contains("unknown field `extra`"), "{err}");
}

#[test]
fn unknown_nested_field_is_a_deserialization_error() {
    let err = parse_error(&VALID_JSON.replace(
        r#""tolerance_permille":50"#,
        r#""tolerance_permille":50,"x":1"#,
    ));
    assert!(err.contains("unknown field `x`"), "{err}");

    let err = parse_error(
        &VALID_JSON.replace(r#""impedance_ohms":null"#, r#""impedance_ohms":null,"y":1"#),
    );
    assert!(err.contains("unknown field `y`"), "{err}");
}

#[test]
fn missing_impedance_key_is_a_deserialization_error() {
    let err = parse_error(&VALID_JSON.replace(r#","impedance_ohms":null"#, ""));
    assert!(err.contains("missing field `impedance_ohms`"), "{err}");
}

#[test]
fn duplicate_key_is_a_deserialization_error() {
    let err = parse_error(&VALID_JSON.replacen('{', r#"{"project_name":"q","#, 1));
    assert!(err.contains("duplicate field"), "{err}");
}
