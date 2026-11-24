//! `AET-WO-4300d` slice A: the power parts table and the specification's
//! `power`, judged statically. Every refusal is fired by one change to the
//! Sentinel-02 power fixture or, where the change is a datasheet limit, to a
//! copy of the embedded table, and checked by name.

use aethelgard_ir::parts::Held;
use aethelgard_ir::power::{self, Refusal};
use aethelgard_ir::power_parts::{self, POWER_PARTS_JSON, parse_power_parts};
use aethelgard_ir::{IrValidationError as E, Specification};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The sha256 of `data/power-parts.json`. It moves only with the file.
/// `AET-WO-4300k`: schema 12 (rows DMP6023LSS-13, MMSZ5242BT1G,
/// SF-1206F400-2, SF-1206F700-2, TLV803SDBZR and their four documents, the
/// MCU's `bor_falling_min_millivolts`, LMR33630 `cin` enforced).
const TABLE_SHA256: &str = "694fc8f20a48bd4fc0b331d19981837d36c135f0812e58599a21d9d4d8feeeb7";

/// One refusal case: what it changes, the refusal, its id, and the change.
type Case = (&'static str, Refusal, &'static str, fn(&mut Value));

const FIXTURE: &str = include_str!("fixtures/sentinel-02-power.json");

fn full() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}

/// The fixture without its crossing and the 1.8 V IMU behind it: the tree
/// every check but the crossing's passes.
fn base() -> Value {
    let mut v = full();
    v["power"]["crossings"] = json!([]);
    // The translator's and its CS pull-up's loads go with the crossing
    // (`AET-WO-1209` D4): a load naming a part the board lacks is refused.
    v["power"]["loads"]
        .as_array_mut()
        .unwrap()
        .retain(|l| l["reference"] != "U13" && l["reference"] != "R40");
    // The IMU's acknowledgement goes with it: one stands only for a part the
    // board uses (`AET-WO-4210` D-B6). The flash's stays (`AET-WO-4211`).
    v["accept_lifecycle"]
        .as_array_mut()
        .unwrap()
        .retain(|a| a["part"] != "ICM20948");
    for key in ["spi_devices", "spi_buses", "net_constraints"] {
        let list = v[key].as_array().unwrap().clone();
        v[key] = Value::Array(
            list.into_iter()
                .filter(|x| {
                    !(x["reference"] == "U5"
                        || x["id"] == "IMU"
                        || x["net_name"].as_str().is_some_and(|n| n.starts_with("IMU")))
                })
                .collect(),
        );
    }
    v
}

fn spec(v: &Value) -> Specification {
    serde_json::from_value(v.clone()).expect("a specification")
}

fn refused(v: &Value) -> E {
    spec(v).validate().expect_err("refused")
}

fn assert_power(err: &E, kind: Refusal, id: &str) {
    match err {
        E::Power { kind: k, id: i, .. } => {
            assert_eq!((*k, i.as_str()), (kind, id), "{err}");
        }
        other => panic!("not a power refusal: {other}"),
    }
}

/// [`assert_power`], naming the case that failed.
fn assert_case(err: &E, kind: Refusal, id: &str, what: &str) {
    match err {
        E::Power { kind: k, id: i, .. } => assert_eq!((*k, i.as_str()), (kind, id), "{what}"),
        other => panic!("{what}: not a power refusal: {other}"),
    }
}

/// A DC-bias curve for a test table: `(mV, change per mille)` points, the
/// tolerance's and the temperature limit's loss, cited to the part's sheet.
fn curve(doc: &str, points: &[(i64, i64)], tol: i64, temp: i64) -> Value {
    let at = json!({"document": doc, "section": "test only"});
    json!({
        "points": points.iter().map(|(mv, c)| json!({"millivolts": mv, "change_permille": c})).collect::<Vec<_>>(),
        "at": at,
        "tolerance_permille": {"value": tol, "at": at},
        "temperature_permille": {"value": temp, "at": at}
    })
}

const GRM_SHEET: &str = "MURATA_GRM188R71C105KA12";

/// The embedded table as JSON, for a copy with one limit changed.
fn table_json() -> Value {
    serde_json::from_str(POWER_PARTS_JSON).unwrap()
}

fn row<'v>(t: &'v mut Value, part: &str) -> &'v mut Value {
    t["parts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["part"] == part)
        .unwrap()
}

/// The fixture checked against `table`, with the sensor table's own output
/// delays (`K6`: the ICM-20948's tVD.SDO, 59 ns).
fn against(v: &Value, table: &Value) -> Result<(), E> {
    let t = parse_power_parts(&table.to_string()).expect("the changed table parses");
    let sensors = aethelgard_ir::parts::table().unwrap();
    power::check_against(&spec(v), &t, sensors, &power::output_delays(sensors)).map(|_| ())
}

fn table_refusal(mutate: impl FnOnce(&mut Value)) -> String {
    let mut t = table_json();
    mutate(&mut t);
    match parse_power_parts(&t.to_string()) {
        Err(E::PowerPartsTable { reason, .. }) => reason,
        other => panic!("not refused: {other:?}"),
    }
}

#[test]
fn the_embedded_table_is_pinned_by_its_sha256() {
    let hash: String = Sha256::digest(POWER_PARTS_JSON.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(hash, TABLE_SHA256);
}

#[test]
fn the_embedded_table_parses_with_every_document_held() {
    let t = power_parts::table().unwrap();
    let names: Vec<&str> = t.rows().iter().map(|r| r.part.as_str()).collect();
    assert_eq!(
        names,
        [
            "ABM8AIG-16.000MHZ-12-2Z-T3",
            "C0402C180J5GACTU",
            "CRCW0402_Z_0",
            "CRCW0603_F_K",
            "DMP6023LSS-13",
            "GCM188R71H104KA57",
            "GCM188R71H224KA64",
            "GCM32EC71H106KA03",
            "GRM188R71C105KA12",
            "GRM32ER61C226KE20",
            "GRM32ER71A226KE20",
            "IHLP5050FDER8R2M01",
            "LMR33630ADDA",
            "MMSZ5242BT1G",
            "SF-1206F400-2",
            "SF-1206F700-2",
            "STM32H753ZITx",
            "TLV75518PDBV",
            "TLV803SDBZR",
            "TNPW0603_B_E",
            "TXU0304PW",
            "UCZ1H470MCL1GS",
        ]
    );
    assert!(t.pending().is_empty());
    for r in t.rows() {
        let d = t.document(&r.temperature_celsius.at.document).unwrap();
        assert!(
            matches!(&d.held, Held::Sha256(h) if h.len() == 64),
            "{}",
            r.part
        );
    }
}

#[test]
fn the_table_parser_refuses_by_name() {
    let r = table_refusal(|t| t["schema_version"] = json!(1));
    assert!(r.contains("schema_version is 1"), "{r}");
    let r = table_refusal(|t| row(t, "TLV75518PDBV")["kind"]["ldo"]["extra"] = json!(1));
    assert!(r.contains("unknown field"), "{r}");
    let r = table_refusal(|t| {
        t["documents"]["UNUSED"] =
            json!({"title": "t", "revision": "r", "sha256": "0".repeat(64), "pending": null})
    });
    assert!(r.contains("cited by no value"), "{r}");
    let r = table_refusal(|t| {
        row(t, "TLV75518PDBV")["kind"]["ldo"]["vin_millivolts"]["at"]["document"] = json!("NOPE")
    });
    assert!(r.contains("does not hold"), "{r}");
    let r = table_refusal(|t| {
        row(t, "TLV75518PDBV")["kind"]["ldo"]["vin_millivolts"]["min"] = json!(6000)
    });
    assert!(r.contains("its min is above its max"), "{r}");
    let r = table_refusal(|t| {
        row(t, "TLV75518PDBV")["pins"]
            .as_object_mut()
            .unwrap()
            .remove("NC");
    });
    assert!(r.contains("names exactly"), "{r}");
    let r = table_refusal(|t| {
        let mut copy = row(t, "TLV75518PDBV").clone();
        copy["part"] = json!("tlv75518pdbv");
        t["parts"].as_array_mut().unwrap().push(copy);
    });
    assert!(r.contains("twice, in some case"), "{r}");
    let r = table_refusal(|t| row(t, "IHLP5050FDER8R2M01")["kind"]["capacitor"] = json!({}));
    assert!(r.contains("exactly one kind"), "{r}");
    let r = table_refusal(|t| {
        row(t, "TLV75518PDBV")["kind"]["ldo"]["external"][0]["dielectrics"] = json!([])
    });
    assert!(r.contains("allowed dielectrics"), "{r}");
    // A key given twice in `pins` or `documents` is refused (A5).
    let text = POWER_PARTS_JSON.replacen("\"OUT\": \"5\"", "\"OUT\": \"5\", \"OUT\": \"1\"", 1);
    assert!(matches!(
        parse_power_parts(&text),
        Err(E::PowerPartsTable { .. })
    ));
    let first = format!(
        "\"SNVSAN3F\": {{\"title\": \"t\", \"revision\": \"r\", \"sha256\": \"{}\", \"pending\": null}}, \"SNVSAN3F\": {{",
        "0".repeat(64)
    );
    let text = POWER_PARTS_JSON.replacen("\"SNVSAN3F\": {", &first, 1);
    assert!(matches!(
        parse_power_parts(&text),
        Err(E::PowerPartsTable { .. })
    ));
    let r = table_refusal(|t| {
        row(t, "STM32H753ZITx")["kind"]["mcu"]["spi_sck_low_shortfall_ns"]["value"] = json!(-1000)
    });
    assert!(r.contains("below zero"), "{r}");
    // A key given twice in a row is refused, never collapsed.
    let text = POWER_PARTS_JSON.replacen(
        "\"lifecycle\": \"active\",",
        "\"lifecycle\": \"active\", \"lifecycle\": \"eol\",",
        1,
    );
    assert!(matches!(
        parse_power_parts(&text),
        Err(E::PowerPartsTable { .. })
    ));
}

#[test]
fn a_sense_net_is_a_new_net() {
    let mut v = base();
    v["power"]["sense"][0]["net"] = json!("vdd_3v3");
    assert_eq!(refused(&v).kind(), "NetShadowsDomain");
    v["power"]["sense"][0]["net"] = json!("Gnd");
    assert_eq!(refused(&v).kind(), "NetShadowsDomain");
    v["power"]["sense"][0]["net"] = json!("spi_sck");
    assert_eq!(refused(&v).kind(), "DuplicateNetName");
}

#[test]
fn power_is_a_required_key_and_null_declares_none() {
    let mut v = base();
    v["power"] = Value::Null;
    // Without a tree no regulator outputs the supply, so it enters by a
    // connector pin (`AET-WO-4300g`), on a connector of its own: the debug
    // connector's supply pin is the debugger's sense, no source (adversarial A3).
    v["connectors"].as_array_mut().unwrap().push(json!({
        "reference": "J3", "part": "SM04B-GHS-TB", "standard": null,
        "pins": [
            {"pin": "1", "net": "VDD_3V3", "offboard": null},
            {"pin": "2", "net": null, "offboard": null},
            {"pin": "3", "net": null, "offboard": null},
            {"pin": "4", "net": "GND", "offboard": null}
        ]
    }));
    v["mcu"]["supply_source"] = json!({"connector": "J3", "pin": "1"});
    // Without a tree there is no battery rail, so a class naming it names an
    // unknown net (`AET-WO-4300d2` K10).
    assert_eq!(refused(&v).kind(), "ClassUnknownNet");
    v["net_classes"] = json!([]);
    assert_eq!(spec(&v).validate(), Ok(()));
}

#[test]
fn bor_level_is_required_and_bounded() {
    let mut v = base();
    v["mcu"]["bor_level"] = json!(4);
    assert_eq!(refused(&v).kind(), "BorLevelOutOfRange");
}

/// The fixture's tree comes to the numbers computed by hand from the cited
/// limits (FEEDBACK B3): integer arithmetic, each bound rounded against the
/// part. `solve` carries exactly this (`solve_cli`'s power test).
#[test]
fn the_tree_checks_to_its_hand_computed_bounds() {
    let p = power::check(&spec(&base())).unwrap().expect("a tree");
    let d = |id: &str| &p.domains[id];
    assert_eq!(
        (
            d("VDD_5V").vout_min_microvolts,
            d("VDD_5V").vout_max_microvolts
        ),
        (4_818_048, 5_237_403)
    );
    assert_eq!(
        (
            d("VDD_3V3").vout_min_microvolts,
            d("VDD_3V3").vout_max_microvolts
        ),
        (3_144_727, 3_417_204)
    );
    assert_eq!(
        (
            d("VDD_1V8").vout_min_microvolts,
            d("VDD_1V8").vout_max_microvolts
        ),
        (1_773_000, 1_827_000)
    );
    assert_eq!(
        [
            d("VDD_5V").load_milliamps,
            d("VDD_3V3").load_milliamps,
            d("VDD_1V8").load_milliamps
        ],
        // VDD_3V3: the MCU's 658 mA, U12's 20 mA and 1 mA (40 uA rounded
        // up), and the fixture's 35 mA of other sinks (`AET-WO-1303b`): its
        // 37 less the removed crossing's U13 and R40 (`AET-WO-1209` D4)
        [2000, 714, 20]
    );
    assert_eq!(
        (p.input_steady_milliamps, p.input_fault_milliamps),
        (2714, 10_100)
    );
    let pad = |p: &(String, String)| (p.0.clone(), p.1.clone());
    assert_eq!(pad(&d("VDD_5V").source), ("R10".to_owned(), "1".to_owned()));
    assert_eq!(pad(&d("VDD_5V").ret), ("R11".to_owned(), "2".to_owned()));
    assert_eq!(
        pad(&d("VDD_1V8").source),
        ("U12".to_owned(), "5".to_owned())
    );
    assert_eq!(pad(&d("VDD_1V8").ret), ("U12".to_owned(), "2".to_owned()));
    let floors = &d("VDD_3V3").consumers;
    assert_eq!(floors[0], ("STM32H753ZITx (MCU)".to_owned(), 2680));
    // The LDO's input floor is its dropout need, 1827 + 400 mV (A3), and
    // VREF+ is floored where the divided battery still fits under it (D3):
    // 4S × 4350 mV = 17400 mV through R50 100 k / R51 10 k, each CRCW at
    // 10000 + 100 × 65 + 20000 = 36500 ppm, R51 high and R50 low:
    // ⌈17400 mV × 10365 / (10365 + 96350)⌉ = 1690025 µV, 1691 mV (it was
    // 2536 mV at 6S's 26100 mV, `AET-WO-4300k` D14).
    assert!(floors.contains(&("U12 (TLV75518PDBV input)".to_owned(), 2227)));
    assert!(
        floors.contains(&("VREF+ (VBAT_SENSE full scale)".to_owned(), 1691)),
        "{floors:?}"
    );
    assert!(
        p.conditions
            .iter()
            .any(|c| c.contains("not production tested"))
    );
    assert!(p.conditions.iter().any(|c| c.contains("typical only")));
    // A specification with no tree checks to none.
    let mut none = base();
    none["power"] = Value::Null;
    assert_eq!(power::check(&spec(&none)), Ok(None));
}

#[test]
fn the_tree_is_refused_by_name() {
    let cases: Vec<Case> = vec![
        ("no source", Refusal::DomainWithoutSource, "VDD_1V8", |v| {
            v["power"]["regulators"].as_array_mut().unwrap().pop();
        }),
        (
            "two sources",
            Refusal::DomainWithTwoSources,
            "VDD_3V3",
            |v| {
                let mut second = v["power"]["regulators"][2].clone();
                second["reference"] = json!("U14");
                second["output"] = json!("VDD_3V3");
                second["external"] = json!([]);
                v["power"]["regulators"]
                    .as_array_mut()
                    .unwrap()
                    .push(second);
            },
        ),
        ("a loop", Refusal::PowerLoop, "U12", |v| {
            v["power"]["regulators"][2]["input"] = json!("VDD_1V8");
        }),
    ];
    for (what, kind, id, mutate) in cases {
        let mut v = base();
        mutate(&mut v);
        assert_power(&refused(&v), kind, id);
        let _ = what;
    }
}

#[test]
fn each_static_check_refuses_by_name() {
    let cases: Vec<Case> = vec![
        (
            "9S is above VIN 36 V",
            Refusal::BatteryOutsideRegulatorInput,
            "U10",
            |v| {
                v["power"]["battery"]["cells"]["max"] = json!(9);
            },
        ),
        (
            "1 % dividers on the 3.3 V buck",
            Refusal::RegulatorOutsideDomain,
            "U11",
            |v| {
                v["power"]["regulators"][1]["feedback"]["top"]["part"] = json!("CRCW0603_F_K");
                v["power"]["regulators"][1]["feedback"]["bottom"]["part"] = json!("CRCW0603_F_K");
            },
        ),
        (
            "TI's 100 k / 43.2 k, not centred",
            Refusal::RegulatorOutsideDomain,
            "U11",
            |v| {
                v["power"]["regulators"][1]["feedback"]["top"]["ohms"] = json!(100_000);
                v["power"]["regulators"][1]["feedback"]["bottom"]["ohms"] = json!(43_200);
            },
        ),
        (
            "3.1 A on the 5 V buck",
            Refusal::RegulatorOverloaded,
            "U10",
            |v| {
                v["power"]["loads"][0]["milliamps"] = json!(3100);
            },
        ),
        (
            "400 mA through the LDO",
            Refusal::JunctionTooHot,
            "U12",
            |v| {
                v["power"]["loads"][1]["milliamps"] = json!(400);
            },
        ),
        // F1, SF-1206F700-2, is rated -20 °C and is swept with every power
        // part before J1 is judged (`power_in`); J1 alone is below.
        (
            "a -30 C board",
            Refusal::BoardOutsidePartTemperature,
            "F1",
            |v| {
                v["power"]["conditions"]["board_temperature_celsius"]["min"] = json!(-30);
            },
        ),
        // By reference: the second input ceramic C60 moved every later index
        // (`AET-WO-4300k` D14).
        ("no CBOOT", Refusal::ExternalPartMissing, "U10", |v| {
            drop_ext(v, "U10", "C12");
        }),
        (
            "three output capacitors",
            Refusal::ExternalPartOutOfRange,
            "U10",
            |v| {
                drop_ext(v, "U10", "C14");
            },
        ),
        (
            "a 10 V capacitor on 5 V",
            Refusal::ExternalPartOutOfRange,
            "C14",
            |v| {
                ext(v, "U10", "C14")["part"] = json!("GRM32ER71A226KE20");
            },
        ),
        (
            // 16000 mV is under the 4S pack's 4 × 4350 = 17400 mV.
            "a 16 V input capacitor on a 17.4 V battery",
            Refusal::ExternalPartOutOfRange,
            "C10",
            |v| {
                v["power"]["regulators"][0]["external"][0]["part"] = json!("GRM32ER61C226KE20");
            },
        ),
        (
            "a role the row lacks",
            Refusal::ExternalPartOutOfRange,
            "C99",
            |v| {
                v["power"]["regulators"][2]["external"]
                    .as_array_mut()
                    .unwrap()
                    .push(
                        json!({"role": "cboot", "reference": "C99", "part": "GCM188R71H104KA57"}),
                    );
            },
        ),
        // R51 high and R50 low, each 36500 ppm: 100 k / 20 k divides 17400 mV
        // to ⌈17400 × 20730 / (20730 + 96350)⌉ = 3081 mV, under VREF+'s
        // 3135 mV (it refused 6S's 26100 mV); 22 k divides it to
        // ⌈17400 × 22803 / (22803 + 96350)⌉ = 3330 mV, over.
        (
            "a 22 k bottom resistor",
            Refusal::SenseFullScaleExceeded,
            "VBAT_SENSE",
            |v| {
                v["power"]["sense"][0]["bottom"]["ohms"] = json!(22_000);
            },
        ),
        (
            "no reservoir",
            Refusal::SenseSourceImpedance,
            "VBAT_SENSE",
            |v| {
                v["power"]["sense"][0]["reservoir"] = Value::Null;
            },
        ),
        (
            "16 bits needs 524 nF",
            Refusal::SenseSourceImpedance,
            "C50",
            |v| {
                v["power"]["sense"][0]["adc_resolution_bits"] = json!(16);
            },
        ),
        (
            "11 bits has no RAIN row",
            Refusal::PowerValueInvalid,
            "VBAT_SENSE",
            |v| {
                v["power"]["sense"][0]["adc_resolution_bits"] = json!(11);
            },
        ),
        (
            "an unknown regulator",
            Refusal::PowerPartUnknown,
            "U10",
            |v| {
                v["power"]["regulators"][0]["part"] = json!("LMR33630");
            },
        ),
        (
            "a capacitor named as a resistor",
            Refusal::PowerPartUnknown,
            "R10",
            |v| {
                v["power"]["regulators"][0]["feedback"]["top"]["part"] = json!("GRM188R71C105KA12");
            },
        ),
        (
            "an unknown power-in part",
            Refusal::PowerPartUnknown,
            "J1",
            |v| {
                v["power"]["input"]["part"] = json!("XT60PW-M");
            },
        ),
        (
            "a 90 C board, past the MCU's 85 C",
            Refusal::BoardOutsidePartTemperature,
            "mcu",
            |v| {
                v["power"]["conditions"]["board_temperature_celsius"]["max"] = json!(90);
            },
        ),
    ];
    for (what, kind, id, mutate) in cases {
        let mut v = base();
        mutate(&mut v);
        let err = refused(&v);
        match &err {
            E::Power { kind: k, id: i, .. } => {
                assert_eq!((*k, i.as_str()), (kind, id), "{what}: {err}");
            }
            other => panic!("{what}: not a power refusal: {other}"),
        }
    }
    // The XT30PW-M's own -20 °C floor, with the fuse's lowered to -40 °C in
    // a table copy so it is not reached first.
    let mut v = base();
    v["power"]["conditions"]["board_temperature_celsius"]["min"] = json!(-30);
    let mut t = table_json();
    row(&mut t, "SF-1206F700-2")["temperature_celsius"]["min"] = json!(-40);
    assert_power(
        &against(&v, &t).unwrap_err(),
        Refusal::BoardOutsidePartTemperature,
        "J1",
    );
}

#[test]
fn a_datasheet_limit_changed_in_a_table_copy_is_refused_by_name() {
    let cases: Vec<Case> = vec![
        (
            "a 2 us off-time",
            Refusal::BuckHeadroomViolated,
            "U10",
            |t| {
                row(t, "LMR33630ADDA")["kind"]["buck"]["toff_min_max_ns"]["value"] = json!(2000);
            },
        ),
        // tON = VOUT,min / (VIN,max × fSW,max): U10's 4818048 µV / (17400 mV
        // × 460 kHz) is 601.95 ns at 4S, so 500 ns (which refused U10 at
        // 6S's 401 ns) now reaches only U11's 392.9 ns; 700 ns refuses U10,
        // judged first.
        ("a 700 ns on-time", Refusal::OnTimeTooShort, "U10", |t| {
            row(t, "LMR33630ADDA")["kind"]["buck"]["ton_min_max_ns"]["value"] = json!(700);
        }),
        ("a 1.4 V dropout", Refusal::DropoutViolated, "U12", |t| {
            row(t, "TLV75518PDBV")["kind"]["ldo"]["dropout_max_millivolts"]["value"] = json!(1400);
        }),
        (
            "an 11 A high-side limit",
            Refusal::PowerInputOverloaded,
            "J1",
            |t| {
                row(t, "LMR33630ADDA")["kind"]["buck"]["high_side_limit_milliamps"]["max"] =
                    json!(11_000);
                row(t, "IHLP5050FDER8R2M01")["kind"]["inductor"]["isat_typical_milliamps"]["value"] =
                    json!(20_000);
            },
        ),
        (
            "an inductor saturating at 5 A",
            Refusal::ExternalPartOutOfRange,
            "L1",
            |t| {
                row(t, "IHLP5050FDER8R2M01")["kind"]["inductor"]["isat_typical_milliamps"]["value"] =
                    json!(5000);
            },
        ),
        (
            "an X5R output capacitor rated to 80 C",
            Refusal::BoardOutsidePartTemperature,
            "C14",
            |t| {
                row(t, "GRM32ER61C226KE20")["temperature_celsius"]["max"] = json!(80);
            },
        ),
        // The drift life decides a verdict (FEEDBACK, a decision): at the
        // TNPW's 225 000 h row, 0.30 %, the 5 V rail reaches past 5.25 V.
        (
            "TNPW drift at 225 000 h",
            Refusal::RegulatorOutsideDomain,
            "U10",
            |t| {
                row(t, "TNPW0603_B_E")["kind"]["resistor"]["drift_ppm"]["value"] = json!(3000);
            },
        ),
        ("an EOL LDO", Refusal::PowerPartUnknown, "U12", |t| {
            row(t, "TLV75518PDBV")["lifecycle"] = json!("eol");
        }),
        (
            "Equation 5 at 2 uH/V MHz",
            Refusal::ExternalPartOutOfRange,
            "L1",
            |t| {
                row(t, "LMR33630ADDA")["kind"]["buck"]["inductor_min_coefficient"]["value"] =
                    json!(2000);
            },
        ),
        (
            "an MCU whose VDD stops at 3.4 V",
            Refusal::DomainOutsideConsumer,
            "VDD_3V3",
            |t| {
                row(t, "STM32H753ZITx")["kind"]["mcu"]["vdd_millivolts"]["max"] = json!(3400);
            },
        ),
        (
            "a BOR3 floor above the domain",
            Refusal::DomainOutsideConsumer,
            "VDD_3V3",
            |t| {
                let m = &mut row(t, "STM32H753ZITx")["kind"]["mcu"];
                m["bor_falling_max_millivolts"][3]["value"] = json!(3135);
                m["bor_rising_max_millivolts"][3]["value"] = json!(3135);
            },
        ),
    ];
    for (what, kind, id, mutate) in cases {
        let mut t = table_json();
        mutate(&mut t);
        let err = against(&base(), &t).expect_err(what);
        match &err {
            E::Power { kind: k, id: i, .. } => {
                assert_eq!((*k, i.as_str()), (kind, id), "{what}: {err}");
            }
            other => panic!("{what}: not a power refusal: {other}"),
        }
    }
    // The unchanged copy passes: each case above moved only its one limit.
    assert_eq!(against(&base(), &table_json()), Ok(()));
}

#[test]
fn a_thermal_rule_the_board_cannot_take_is_refused_by_1309s_validator() {
    let mut t = table_json();
    row(&mut t, "LMR33630ADDA")["kind"]["buck"]["thermal_vias"]["ring_um"] = json!(100);
    assert_eq!(
        against(&base(), &t).unwrap_err().kind(),
        "ThermalViaRingUnderRules"
    );
}

#[test]
fn the_crossing_is_judged_on_both_sides() {
    // Without the crossing a 1.8 V device on a 3.3 V bus is the one-rail
    // refusal it always was.
    let mut v = full();
    v["power"]["crossings"] = json!([]);
    assert_eq!(refused(&v).kind(), "SupplyMismatch");
    // Swapped sides leave the device off the crossing's B side: the same.
    let mut v = full();
    v["power"]["crossings"][0]["a_domain"] = json!("VDD_1V8");
    v["power"]["crossings"][0]["b_domain"] = json!("VDD_3V3");
    assert_eq!(refused(&v).kind(), "SupplyMismatch");
    let cases: Vec<Case> = vec![
        (
            "a 1 M pull-up at 1 %",
            Refusal::CrossingPullUp,
            "R40",
            |v| {
                v["power"]["crossings"][0]["cs_pull_up"]["ohms"] = json!(1_000_000);
            },
        ),
        (
            "a crossing on the I2C bus",
            Refusal::CrossingInvalid,
            "U15",
            |v| {
                let mut second = v["power"]["crossings"][0].clone();
                second["reference"] = json!("U15");
                second["bus"] = json!("SENSORS");
                second["cs_pull_up"]["reference"] = json!("R41");
                second["external"][0]["reference"] = json!("C42");
                second["external"][1]["reference"] = json!("C43");
                v["power"]["crossings"]
                    .as_array_mut()
                    .unwrap()
                    .insert(0, second);
            },
        ),
        (
            "no B-side bypass",
            Refusal::ExternalPartMissing,
            "U13",
            |v| {
                v["power"]["crossings"][0]["external"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            },
        ),
    ];
    for (what, kind, id, mutate) in cases {
        let mut v = full();
        mutate(&mut v);
        let err = refused(&v);
        match &err {
            E::Power { kind: k, id: i, .. } => {
                assert_eq!((*k, i.as_str()), (kind, id), "{what}: {err}");
            }
            other => panic!("{what}: not a power refusal: {other}"),
        }
    }
}

/// With the translator's bands widened in a table copy, a crossing whose A
/// side is not its bus's domain reaches the crossing's own rule.
#[test]
fn a_crossing_on_the_wrong_domain_is_refused_by_the_crossing_rule() {
    let mut t = table_json();
    row(&mut t, "TXU0304PW")["kind"]["translator"]["vcca_millivolts"]["max"] = json!(5500);
    let mut v = full();
    v["power"]["crossings"][0]["a_domain"] = json!("VDD_5V");
    let err = against(&v, &t).unwrap_err();
    assert_power(&err, Refusal::CrossingDomainMismatch, "U13");
}

/// SCK out 19 + 59 + MISO back 15 + setup 2 = 95 ns: inside 4 MHz's
/// 125 - 2 ns and 5 MHz's 100 - 2, outside 5.3 MHz's 94 - 2 and 7 MHz's
/// 71 - 2, so the crossing caps the IMU bus below the part's own 7 MHz.
#[test]
fn the_miso_round_trip_bounds_the_crossed_bus() {
    assert_eq!(against(&full(), &table_json()), Ok(()));
    let mut v = full();
    v["spi_buses"][1]["speed_hz"] = json!(5_000_000);
    assert_eq!(against(&v, &table_json()), Ok(()));
    for hz in [5_300_000, 7_000_000] {
        v["spi_buses"][1]["speed_hz"] = json!(hz);
        let err = against(&v, &table_json()).unwrap_err();
        assert_power(&err, Refusal::MisoRoundTripTooSlow, "U13");
    }
    let mut t = table_json();
    row(&mut t, "TXU0304PW")["kind"]["translator"]["tmax_a_to_b_bps"]["value"] = json!(7_000_000);
    assert_power(
        &against(&full(), &t).unwrap_err(),
        Refusal::CrossingTooFast,
        "U13",
    );
}

#[test]
fn the_committed_fixture_with_its_crossing_validates_on_the_sensor_tables_own_delay() {
    // `K6`: no table copy, no injected delay.
    let p = power::check(&spec(&full())).unwrap().unwrap();
    assert!(spec(&full()).validate().is_ok());
    let has = |s: &str| p.conditions.iter().any(|c| c.contains(s));
    // F-B1: 19 + 59 + 15 + 2 ns, at 4 MHz.
    assert!(has("the MISO round trip is 95 ns"), "{:?}", p.conditions);
    // The delay's characterization is a condition the verdict names.
    assert!(has("tVD.SDO max 59 ns at 20 pF"), "{:?}", p.conditions);
    assert!(has("characterization of 5 parts"), "{:?}", p.conditions);
}

#[test]
fn a_miso_round_trip_with_no_cited_output_delay_is_refused_not_passed() {
    // The row's value is `null`: a table copy, since the committed one holds it.
    let mut doc: Value = serde_json::from_str(aethelgard_ir::parts::PARTS_JSON).unwrap();
    let imu = doc["parts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["part"] == "ICM20948")
        .unwrap();
    imu["spi_output_delay_max_ns"] = Value::Null;
    imu["citations"]["spi_output_delay"] = Value::Null;
    let sensors = aethelgard_ir::parts::parse_parts(&doc.to_string()).unwrap();
    let t = parse_power_parts(POWER_PARTS_JSON).unwrap();
    let err = power::check_against(
        &spec(&full()),
        &t,
        &sensors,
        &power::output_delays(&sensors),
    )
    .unwrap_err();
    assert_power(&err, Refusal::OutputDelayMissing, "U5");
}

#[test]
fn every_remaining_guard_refuses_by_name() {
    let cases: Vec<Case> = vec![
        (
            "no cells",
            Refusal::PowerValueInvalid,
            "power.battery.cells",
            |v| {
                v["power"]["battery"]["cells"]["min"] = json!(0);
            },
        ),
        ("a 0 ohm resistor", Refusal::PowerValueInvalid, "R10", |v| {
            v["power"]["regulators"][0]["feedback"]["top"]["ohms"] = json!(0);
        }),
        ("a 0 mA load", Refusal::PowerValueInvalid, "VDD_5V", |v| {
            v["power"]["loads"][0]["milliamps"] = json!(0);
        }),
        // `AET-WO-1303b`: a drop limit is 1 to 100000 mV, once per domain
        (
            "a 0 mV drop limit",
            Refusal::PowerValueInvalid,
            "VDD_3V3",
            |v| {
                v["power"]["drop_limits"] =
                    json!([{"domain": "VDD_3V3", "max_millivolts": 0, "source": "s"}]);
            },
        ),
        (
            "a part named twice as a load",
            Refusal::PowerValueInvalid,
            "VDD_3V3",
            |v| {
                let loads = v["power"]["loads"].as_array_mut().unwrap();
                loads.push(
                    json!({"domain": "VDD_3V3", "milliamps": 1, "source": "s", "reference": "U2"}),
                );
                loads.push(
                    json!({"domain": "VDD_3V3", "milliamps": 2, "source": "s", "reference": "U2"}),
                );
            },
        ),
        (
            "a drop limit named twice",
            Refusal::PowerValueInvalid,
            "VDD_3V3",
            |v| {
                v["power"]["drop_limits"] = json!([
                    {"domain": "VDD_3V3", "max_millivolts": 165, "source": "s"},
                    {"domain": "VDD_3V3", "max_millivolts": 160, "source": "s"}
                ]);
            },
        ),
        (
            "a board range upside down",
            Refusal::PowerValueInvalid,
            "power.conditions.board_temperature_celsius",
            |v| {
                v["power"]["conditions"]["board_temperature_celsius"]["min"] = json!(90);
            },
        ),
        (
            "an LDO with a divider",
            Refusal::PowerValueInvalid,
            "U12",
            |v| {
                v["power"]["regulators"][2]["feedback"] =
                    v["power"]["regulators"][0]["feedback"].clone();
                v["power"]["regulators"][2]["feedback"]["top"]["reference"] = json!("R90");
                v["power"]["regulators"][2]["feedback"]["bottom"]["reference"] = json!("R91");
            },
        ),
        (
            "a buck with no divider",
            Refusal::ExternalPartMissing,
            "U10",
            |v| {
                v["power"]["regulators"][0]["feedback"] = Value::Null;
            },
        ),
        (
            "RFBT above 1 M",
            Refusal::ExternalPartOutOfRange,
            "R10",
            |v| {
                v["power"]["regulators"][0]["feedback"]["top"]["ohms"] = json!(1_020_000);
                v["power"]["regulators"][0]["feedback"]["bottom"]["ohms"] = json!(255_000);
            },
        ),
        ("no inductor", Refusal::ExternalPartMissing, "U10", |v| {
            v["power"]["regulators"][0]["external"]
                .as_array_mut()
                .unwrap()
                .pop();
        }),
        (
            "two inductors",
            Refusal::ExternalPartOutOfRange,
            "U10",
            |v| {
                v["power"]["regulators"][0]["external"]
                .as_array_mut()
                .unwrap()
                .push(json!({"role": "inductor", "reference": "L9", "part": "IHLP5050FDER8R2M01"}));
            },
        ),
        (
            "an X7S part where X7R is required",
            Refusal::ExternalPartOutOfRange,
            "C11",
            |v| {
                ext(v, "U10", "C11")["part"] = json!("GCM32EC71H106KA03");
            },
        ),
        (
            "an inductor named as a capacitor",
            Refusal::PowerPartUnknown,
            "C10",
            |v| {
                v["power"]["regulators"][0]["external"][0]["part"] = json!("IHLP5050FDER8R2M01");
            },
        ),
    ];
    for (what, kind, id, mutate) in cases {
        let mut v = base();
        mutate(&mut v);
        let err = refused(&v);
        match &err {
            E::Power { kind: k, id: i, .. } => {
                assert_eq!((*k, i.as_str()), (kind, id), "{what}: {err}");
            }
            other => panic!("{what}: not a power refusal: {other}"),
        }
    }
}

#[test]
fn the_crossing_structure_is_refused_by_name() {
    let cases: Vec<Case> = vec![
        (
            "two crossings on one bus",
            Refusal::CrossingInvalid,
            "U13",
            |v| {
                let mut second = v["power"]["crossings"][0].clone();
                second["reference"] = json!("U15");
                second["cs_pull_up"]["reference"] = json!("R41");
                second["external"][0]["reference"] = json!("C42");
                second["external"][1]["reference"] = json!("C43");
                v["power"]["crossings"].as_array_mut().unwrap().push(second);
            },
        ),
        (
            "two devices behind one chip-select crossing",
            Refusal::CrossingInvalid,
            "U13",
            |v| {
                let mut second = v["spi_devices"][1].clone();
                second["reference"] = json!("U6");
                second["cs"]["net"] = json!("IMU2_CS");
                v["spi_devices"].as_array_mut().unwrap().push(second);
            },
        ),
    ];
    for (what, kind, id, mutate) in cases {
        let mut v = full();
        mutate(&mut v);
        let err = refused(&v);
        match &err {
            E::Power { kind: k, id: i, .. } => {
                assert_eq!((*k, i.as_str()), (kind, id), "{what}: {err}");
            }
            other => panic!("{what}: not a power refusal: {other}"),
        }
    }
    // A translator with too few A-to-B channels for SCK, MOSI and CS.
    let mut t = table_json();
    row(&mut t, "TXU0304PW")["kind"]["translator"]["channels"][2]["direction"] = json!("b_to_a");
    assert_power(
        &against(&full(), &t).unwrap_err(),
        Refusal::CrossingInvalid,
        "U13",
    );
    // A load above the current the LDO's dropout is stated at.
    let mut t = table_json();
    row(&mut t, "TLV75518PDBV")["kind"]["ldo"]["dropout_at_milliamps"] = json!(10);
    assert_power(
        &against(&base(), &t).unwrap_err(),
        Refusal::DropoutViolated,
        "U12",
    );
}

/// The guards the mutation pass found untested (FEEDBACK B6).
#[test]
fn the_mutation_survivors_are_held() {
    // A 0 mV cell.
    let mut v = base();
    v["power"]["battery"]["cell_min_millivolts"]["value"] = json!(0);
    assert_power(&refused(&v), Refusal::PowerValueInvalid, "power.battery");
    // A resistor whose worst error reaches 100 % is refused, never divided by.
    let mut t = table_json();
    // 997 375 + 25 ppm/K x 65 K + 1000 = exactly 1 000 000 ppm.
    row(&mut t, "TNPW0603_B_E")["kind"]["resistor"]["tolerance_ppm"]["value"] = json!(997_375);
    assert_power(
        &against(&base(), &t).unwrap_err(),
        Refusal::RegulatorOutsideDomain,
        "U10",
    );
    // A divider already under RAIN needs no reservoir: 10 k / 1 k is at most
    // about 1.1 k at 12 bits, under 1150 ohms.
    let mut v = base();
    v["power"]["sense"][0]["top"]["ohms"] = json!(10_000);
    v["power"]["sense"][0]["bottom"]["ohms"] = json!(1_000);
    v["power"]["sense"][0]["reservoir"] = Value::Null;
    assert_eq!(spec(&v).validate(), Ok(()));
    // A reservoir rated under the pin's voltage.
    let mut t = table_json();
    let mut low = row(&mut t, "GCM188R71H104KA57").clone();
    low["part"] = json!("LOW_VOLTAGE_100N");
    low["kind"]["capacitor"]["rated_millivolts"]["value"] = json!(2500);
    t["parts"].as_array_mut().unwrap().push(low);
    let mut v = base();
    v["power"]["sense"][0]["reservoir"]["part"] = json!("LOW_VOLTAGE_100N");
    assert_power(
        &against(&v, &t).unwrap_err(),
        Refusal::ExternalPartOutOfRange,
        "C50",
    );
}

/// `AET-WO-1218` (`1217` R3): the battery sense's reservoir is rated for
/// VDDA's maximum, as a pin device's is (`1217` F8). VDD_3V3 is 3300 mV at
/// 50 per mille: 3135 mV to 3465 mV. No held capacitor is rated under 10 V,
/// so the band is reached by a test row.
#[test]
fn the_sense_reservoir_is_rated_for_vddas_maximum() {
    let vdd = aethelgard_ir::VoltageDomain {
        id: "VDD_3V3".into(),
        nominal_millivolts: 3300,
        tolerance_permille: 50,
    };
    let (min, max) = (
        aethelgard_ir::minimum_millivolts(&vdd),
        aethelgard_ir::maximum_millivolts(&vdd),
    );
    assert_eq!((min, max), (3135, 3465));
    let mut v = base();
    v["power"]["sense"][0]["reservoir"]["part"] = json!("BAND_100N");
    for (rated, ok) in [(min, false), (max - 1, false), (max, true)] {
        let mut t = table_json();
        let mut band = row(&mut t, "GCM188R71H104KA57").clone();
        band["part"] = json!("BAND_100N");
        band["kind"]["capacitor"]["rated_millivolts"]["value"] = json!(rated);
        t["parts"].as_array_mut().unwrap().push(band);
        let r = against(&v, &t);
        if ok {
            assert_eq!(r, Ok(()), "{rated}");
        } else {
            let err = r.unwrap_err();
            assert_power(&err, Refusal::ExternalPartOutOfRange, "C50");
            assert!(
                err.to_string().contains("VDDA's maximum of 3465 mV"),
                "{err}"
            );
        }
    }
}

/// `AET-WO-1218` (`1217` R5): the battery sense's declared reservoir is a
/// capacitor row whether or not RAIN needs it. 10 k / 1 k is under RAIN, so
/// before this a resistor row validated and `emit` refused it
/// (`PowerNotDrawable`).
#[test]
fn a_sense_reservoir_rain_does_not_need_is_still_a_capacitor_row() {
    let mut v = base();
    v["power"]["sense"][0]["top"]["ohms"] = json!(10_000);
    v["power"]["sense"][0]["bottom"]["ohms"] = json!(1_000);
    assert_eq!(spec(&v).validate(), Ok(()));
    v["power"]["sense"][0]["reservoir"]["part"] = json!("CRCW0603_F_K");
    let err = refused(&v);
    assert_power(&err, Refusal::PowerPartUnknown, "C50");
    assert!(err.to_string().contains("is not a capacitor row"), "{err}");
}

/// `power::check` is public: on a specification nobody validated, a BOR
/// level past the table is refused, never an index past its end (A19).
#[test]
fn check_on_an_unvalidated_bor_level_refuses_rather_than_panics() {
    let mut v = base();
    v["mcu"]["bor_level"] = json!(7);
    let err = power::check(&spec(&v)).unwrap_err();
    assert_power(&err, Refusal::PowerValueInvalid, "mcu");
}

/// The ruling on PR #147 (comment 6893999770).
#[test]
fn the_build_rulings_hold() {
    // D-B9: a 5 V LMR33630 rail needs a battery minimum of at least 7 V; 2S
    // (6.0 V) is refused, 3S (9.0 V) is the fixture.
    let mut v = base();
    v["power"]["battery"]["cells"]["min"] = json!(2);
    assert_power(&refused(&v), Refusal::BatteryOutsideRegulatorInput, "U10");
    // D-B6: the XT30PW-M is judged at 15 A. Two high-side limits of 7.4 A
    // (14.8 A) pass; of 7.6 A (15.2 A) are refused.
    for (isc, ok) in [(7400, true), (7600, false)] {
        let mut t = table_json();
        row(&mut t, "LMR33630ADDA")["kind"]["buck"]["high_side_limit_milliamps"]["max"] =
            json!(isc);
        let r = against(&base(), &t);
        if ok {
            assert_eq!(r, Ok(()), "{isc}");
        } else {
            assert_power(&r.unwrap_err(), Refusal::PowerInputOverloaded, "J1");
        }
    }
    // D-B3, D-B4, D-B6, D-B7, D-B8: each margin and condition is in the verdict.
    let p = power::check(&spec(&full())).unwrap().unwrap();
    let has = |s: &str| p.conditions.iter().any(|c| c.contains(s));
    assert!(has(
        "a margin of 3.16 times, not guaranteed above it (ruled D-B3)"
    ));
    assert!(has(
        "32768 pF is needed and 100000 pF is declared, a margin of 3.05 times"
    ));
    assert!(has("the 8000 h life, ruled D-B4"));
    assert!(has("rated 15000 mA"));
    assert!(has("XT30PW-M30"));
    assert!(has("at or under 5154639 Hz"));
    assert!(has("X7S, accepted"));
}

// ---- AET-WO-4300d3: the power tree's remaining judgements --------------------

/// A regulator's entry in the fixture, by reference.
fn reg<'v>(v: &'v mut Value, reference: &str) -> &'v mut Value {
    v["power"]["regulators"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["reference"] == reference)
        .unwrap()
}

/// A regulator's external part, by reference.
fn ext<'v>(v: &'v mut Value, regulator: &str, reference: &str) -> &'v mut Value {
    reg(v, regulator)["external"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|e| e["reference"] == reference)
        .unwrap()
}

/// A regulator without one external part, by reference.
fn drop_ext(v: &mut Value, regulator: &str, reference: &str) {
    let list = reg(v, regulator)["external"].as_array_mut().unwrap();
    let before = list.len();
    list.retain(|e| e["reference"] != reference);
    assert_eq!(list.len() + 1, before, "{regulator} holds {reference}");
}

/// The fixture's verdict conditions, checked against `table`.
fn conditions(v: &Value, table: &Value) -> Vec<String> {
    let t = parse_power_parts(&table.to_string()).expect("the changed table parses");
    let sensors = aethelgard_ir::parts::table().unwrap();
    power::check_against(&spec(v), &t, sensors, &power::output_delays(sensors))
        .unwrap()
        .unwrap()
        .conditions
}

/// A table with test-only TVS rows: the Littelfuse SMBJ28A and SMBJ26A as
/// read (p. 2, 06/03/20 revision, from a mirror, so never pinned), an
/// invented part that passes, to prove the clamp path, and an invented part
/// standing off under the 4S pack's 17.4 V (`AET-WO-4300k` D14), whose clamp
/// alone would pass.
fn tvs_table() -> Value {
    let mut t = table_json();
    t["documents"]["SMBJ"] = json!({"title": "Littelfuse SMBJ series (test only)",
        "revision": "06/03/20",
        "sha256": "4e7ea0ef4a74de638b88fb027adc9750d9a4c71a0a162c125032205a939e6b71",
        "pending": null});
    let tvs = |part: &str, standoff: i64, clamp: i64| {
        let at = json!({"document": "SMBJ", "section": "p. 2, electrical characteristics"});
        json!({"part": part, "lifecycle": "unread", "symbol": "Device:D_TVS",
            "footprint": "Diode_SMD:D_SMB", "pins": {},
            "temperature_celsius": {"min": -65, "max": 150, "at": at},
            "kind": {"tvs": {"standoff_millivolts": {"value": standoff, "at": at},
                "clamp_max_millivolts": {"value": clamp, "at": at},
                "clamp_condition": "at IPP, 10/1000 us"}}})
    };
    let parts = t["parts"].as_array_mut().unwrap();
    parts.push(tvs("SMBJ28A", 28000, 45400));
    parts.push(tvs("SMBJ26A", 26000, 42100));
    parts.push(tvs("TVS_TEST_PASS", 27000, 37000));
    parts.push(tvs("TVS_TEST_LOW_STANDOFF", 17000, 37000));
    t
}

/// The input damped by a clamp alone, as D2: the fixture's D1 is the
/// protection's zener (`AET-WO-4300k` D14).
fn with_clamp(part: &str) -> Value {
    let mut v = base();
    v["power"]["input"]["hot_plug"] = json!([{"reference": "D2", "part": part, "side": "board"}]);
    v
}

#[test]
fn the_fixture_passes_every_new_check_and_names_each_condition() {
    assert_eq!(spec(&full()).validate().map(|_| ()), Ok(()));
    let c = conditions(&full(), &table_json());
    let has = |s: &str| c.iter().any(|x| x.contains(s));
    // §1.2, `AET-WO-4300k` (a): the copper is measured on the drawn board,
    // so the verdict names the least area that clears. U10, 1170 mW on an
    // 85 °C board: 85000 + ⌈1170 × 36.0⌉ = 127120 m°C at 10 cm², over
    // 125 °C; 85000 + 1170 × 30.0 = 120100 at 15 cm², under. U11, 550 mW:
    // 85000 + 550 × 42.0 = 108100 at the figure's first point, 7 cm².
    assert!(has(
        "U10: its junction is screened, not guaranteed: it needs at least 15 cm² of copper per joined layer"
    ));
    assert!(has(
        "U11: its junction is screened, not guaranteed: it needs at least 7 cm² of copper per joined layer"
    ));
    // The screen at the fixture's former 20 cm²: 1170 mW × 28.0 °C/W is
    // 32760 mK over 85 °C, 117760 m°C, 7240 mK of margin.
    let s = power::junction_screen(&spec(&full()), "U10", 2000).unwrap();
    assert!(
        s.contains("is 117760 m°C on a 85 °C board, 7240 mK under 125 °C"),
        "{s}"
    );
    assert!(has("outside the θJA figure's 4-layer board"));
    // §1.3: the LDO's 1 µF at 50 % against 0.47 µF; the buck's named (D2).
    assert!(has("`cin` holds 500000 pF effective at 3465 mV"));
    assert!(has("a margin of 1.06 times, typical, not guaranteed"));
    // The buck's 10 µF input minimum is effective and, since `AET-WO-4300k`
    // D14, enforced, with two ceramics per buck at 4S. Murata's curve at
    // 17.4 V, between 15 V's -172 and 17.5 V's -228: -172 + ⌊-56 × 2400 /
    // 2500⌋ = -172 - 54 = -226; 10 µF x 0.774 x 0.9 (tolerance) x 0.78
    // (X7S) = 5433480 pF each, 10866960 pF for two, 1.08 times 10 µF. The
    // named shortfall (one ceramic, `report`) is held in
    // `a_bias_lie_and_a_short_effective_capacitance_are_refused`.
    assert!(has(
        "U10: `cin` holds 10866960 pF effective at 17400 mV under DC bias"
    ));
    assert!(has(
        "5433480 pF, its rated 10000000 pF times its maker's curve at 17400 mV, -226 per mille interpolated"
    ));
    assert!(has(
        "sha256 8b0b6cfffc8a1bc96152d2d1b6167f71bc9c6cde89d2b37fad060d888b6ce73b"
    ));
    assert!(has("its tolerance's -100 per mille"));
    assert!(has("its temperature limit's -220 per mille"));
    assert!(has("it is screened against the 10000000 pF minimum"));
    assert!(has("a margin of 1.08 times, typical, not guaranteed"));
    assert!(has(
        "by the declared derating, a placeholder until its maker's curve is held"
    ));
    assert!(has("a declared derating, not a maker's figure"));
    // §1.1, §1.4, D5. With no clamp, the ring is bounded at twice the
    // pack (ruled D7): 2 × 17400 = 34800 mV, under the 38000 mV VIN maximum.
    assert!(has(
        "rings VIN to at most twice the battery's 17400 mV, 34800 mV, under VIN's absolute maximum of 38000 mV"
    ));
    assert!(has(
        "damps the input's ring through its ESR, at most 250 mOhm"
    ));
    assert!(has("VBOR3's rising maximum of 2780 mV is judged"));
    // `AET-WO-1315` §1.5: the board draws the hot-plug parts and each control
    // as declared, so no condition says it does not.
    assert!(!has("(FEEDBACK D5)"));
    assert!(!has("does not draw"));
}

#[test]
fn an_undamped_battery_input_is_refused_by_name() {
    let cases: Vec<Case> = vec![
        ("no part", Refusal::BatteryInputUndamped, "J1", |v| {
            v["power"]["input"]["hot_plug"] = json!([]);
        }),
        (
            "the bulk on the pack's side",
            Refusal::BatteryInputUndamped,
            "C1",
            |v| {
                v["power"]["input"]["hot_plug"][0]["side"] = json!("pack");
            },
        ),
        (
            "a ceramic as the bulk",
            Refusal::PowerPartUnknown,
            "C1",
            |v| {
                v["power"]["input"]["hot_plug"][0]["part"] = json!("GCM32EC71H106KA03");
            },
        ),
        (
            "three bulks, 141 µF",
            Refusal::BatteryInputUndamped,
            "J1",
            |v| {
                let list = v["power"]["input"]["hot_plug"].as_array_mut().unwrap();
                for r in ["C2", "C3"] {
                    list.push(json!({"reference": r, "part": "UCZ1H470MCL1GS", "side": "board"}));
                }
            },
        ),
    ];
    for (what, kind, id, mutate) in cases {
        let mut v = base();
        mutate(&mut v);
        assert_case(&refused(&v), kind, id, what);
    }
    // Two bulks (94 µF) are inside TI's 20 to 100 µF. On the real fuse they
    // melt it first (`AET-WO-4300k` (10)): 94 µF + the four 10 µF input
    // ceramics + the two 220 nF, all rated, is 134440000 pF; C·V²/(2·R_min)
    // with R_min = 7500 µΩ × 0.75 = 5625 µΩ is ⌈134440000 × 17400² /
    // (2 × 5625 × 10⁶)⌉ = 3618050 mA²s, at or above SF-1206F700-2's
    // 3250000. With its melting I²t raised past the bound in a table copy,
    // the bulk judge alone decides, and passes.
    let mut v = base();
    v["power"]["input"]["hot_plug"]
        .as_array_mut()
        .unwrap()
        .push(json!({"reference": "C2", "part": "UCZ1H470MCL1GS", "side": "board"}));
    let err = refused(&v);
    assert_power(&err, Refusal::FuseMeltsOnHotPlug, "F1");
    assert!(err.to_string().contains("puts up to 3618050 mA²s"), "{err}");
    let mut t = table_json();
    row(&mut t, "SF-1206F700-2")["kind"]["fuse"]["melting_i2t_milliamp2_seconds"]["value"] =
        json!(3_618_051);
    assert_eq!(against(&v, &t), Ok(()));
    // A bulk rated at the battery's maximum (4S × 4350 mV), or holding
    // 19 µF, is refused.
    for (field, value, id) in [
        ("rated_millivolts", 17400, "C1"),
        ("rated_picofarads", 19_000_000, "J1"),
    ] {
        let mut t = table_json();
        row(&mut t, "UCZ1H470MCL1GS")["kind"]["bulk_capacitor"][field]["value"] = json!(value);
        assert_power(
            &against(&base(), &t).unwrap_err(),
            Refusal::BatteryInputUndamped,
            id,
        );
    }
    // The bulk's lifecycle and temperature are judged as every part's are.
    let mut t = table_json();
    row(&mut t, "UCZ1H470MCL1GS")["temperature_celsius"]["max"] = json!(80);
    assert_power(
        &against(&base(), &t).unwrap_err(),
        Refusal::BoardOutsidePartTemperature,
        "C1",
    );
}

#[test]
fn a_clamp_is_judged_by_its_stand_off_and_its_clamp() {
    // SMBJ28A clamps at 45.4 V, above the LMR33630's 38 V. SMBJ26A stood
    // off 26.0 V, at or below 6S's 26.1 V pack; at 4S (4 × 4350 = 17400 mV)
    // its stand-off clears, and its 42.1 V clamp, above 38 V, refuses it.
    // A stand-off at or below the pack: the invented 17.0 V part.
    for (part, says) in [
        ("SMBJ28A", "clamps at up to 45400 mV"),
        ("SMBJ26A", "clamps at up to 42100 mV"),
        (
            "TVS_TEST_LOW_STANDOFF",
            "stands off 17000 mV, not above the battery's maximum of 17400 mV",
        ),
    ] {
        let err = against(&with_clamp(part), &tvs_table()).unwrap_err();
        assert_power(&err, Refusal::BatteryInputUndamped, "D2");
        assert!(err.to_string().contains(says), "{part}: {err}");
    }
    assert_eq!(against(&with_clamp("TVS_TEST_PASS"), &tvs_table()), Ok(()));
    let c = conditions(&with_clamp("TVS_TEST_PASS"), &tvs_table());
    assert!(
        c.iter()
            .any(|x| x.contains("clamps at up to 37000 mV at IPP"))
    );
    // Each battery-fed regulator's own ceiling decides: with the buck's
    // lowered to 36.999 V, the 37 V clamp is refused.
    let mut t = tvs_table();
    row(&mut t, "LMR33630ADDA")["kind"]["buck"]["vin_abs_max_millivolts"]["value"] = json!(36_999);
    assert_power(
        &against(&with_clamp("TVS_TEST_PASS"), &t).unwrap_err(),
        Refusal::BatteryInputUndamped,
        "D2",
    );
    // A TVS row clamping at or below its own stand-off does not parse.
    let r = table_refusal(|t| {
        *t = tvs_table();
        row(t, "SMBJ28A")["kind"]["tvs"]["clamp_max_millivolts"]["value"] = json!(28000);
    });
    assert!(r.contains("not above the stand-off"), "{r}");
}

#[test]
fn a_hot_buck_is_refused_by_name() {
    let cases: Vec<Case> = vec![
        (
            "no declaration",
            Refusal::BuckJunctionOverLimit,
            "U10",
            |v| {
                reg(v, "U10")["junction"] = Value::Null;
            },
        ),
        // 2000 mW at the figure's lowest θ, 25.0 °C/W (58 cm²), is 85000 +
        // 50000 m°C: no area clears it.
        ("2 W of loss", Refusal::BuckJunctionOverLimit, "U10", |v| {
            reg(v, "U10")["junction"]["loss_milliwatts"]["value"] = json!(2000);
        }),
        (
            "a junction on the LDO",
            Refusal::PowerValueInvalid,
            "U12",
            |v| {
                let j = reg(v, "U10")["junction"].clone();
                reg(v, "U12")["junction"] = j;
            },
        ),
        (
            "an ambient above the part's range",
            Refusal::BoardOutsidePartTemperature,
            "mcu",
            |v| {
                v["power"]["conditions"]["board_temperature_celsius"]["max"] = json!(126);
            },
        ),
        ("no loss", Refusal::PowerValueInvalid, "U10", |v| {
            reg(v, "U10")["junction"]["loss_milliwatts"]["value"] = json!(0);
        }),
    ];
    for (what, kind, id, mutate) in cases {
        let mut v = base();
        mutate(&mut v);
        assert_case(&refused(&v), kind, id, what);
    }
    // The copper is measured on the drawn board and screened there
    // (`AET-WO-4300k` (a)). 10 cm², 1000 mm², at 36.0 °C/W: 85000 + 1170 ×
    // 36.0 = 127120 m°C, over 125 °C. 6 cm², 600 mm², is below the
    // figure's first point, 7 cm².
    let screen = |v: &Value, r: &str, mm2: u64| power::junction_screen(&spec(v), r, mm2);
    let err = screen(&base(), "U10", 1000).unwrap_err();
    assert_case(
        &err,
        Refusal::BuckJunctionOverLimit,
        "U10",
        "10 cm², at 36 °C/W",
    );
    assert!(err.to_string().contains("127120 m°C"), "{err}");
    let err = screen(&base(), "U11", 600).unwrap_err();
    assert_case(
        &err,
        Refusal::BuckJunctionOverLimit,
        "U11",
        "below the curve",
    );
    assert!(
        err.to_string()
            .contains("under the θJA figure's smallest area, 7 cm²"),
        "{err}"
    );
    // A loss bound with no source is refused by the shape.
    let mut v = base();
    reg(&mut v, "U10")["junction"]["loss_milliwatts"]["source"] = json!("");
    assert!(
        matches!(refused(&v), E::InvalidSource { .. }),
        "{}",
        refused(&v)
    );
    // The limit is inclusive and exact, at 20 cm² (2000 mm², 28.0 °C/W):
    // 1428 mW rises 39984 mK to 124984 m°C, and 1429 mW rises 40012 mK,
    // past 125 °C.
    for (loss, ok) in [(1428, true), (1429, false)] {
        let mut v = base();
        reg(&mut v, "U10")["junction"]["loss_milliwatts"]["value"] = json!(loss);
        let r = screen(&v, "U10", 2000);
        if ok {
            let s = r.unwrap();
            assert!(s.contains("is 124984 m°C"), "{loss}: {s}");
        } else {
            let err = r.unwrap_err();
            assert_power(&err, Refusal::BuckJunctionOverLimit, "U10");
            assert!(err.to_string().contains("125012 m°C"), "{err}");
        }
    }
    // The point at or below the measured area is used: 29 cm², 2900 mm²,
    // reads the 20 cm² point's 28.0 °C/W, not 30 cm²'s.
    let s = screen(&base(), "U10", 2900).unwrap();
    assert!(
        s.contains("θJA 28.0 °C/W, the figure's point at 20 cm² for the 29.00 cm²"),
        "{s}"
    );
    // A θ curve for another package, or one that rises with area, does not parse.
    let r = table_refusal(|t| {
        row(t, "LMR33630ADDA")["kind"]["buck"]["theta_ja"]["package"] = json!("RNX");
    });
    assert!(r.contains("not in that package"), "{r}");
    let r = table_refusal(|t| {
        row(t, "LMR33630ADDA")["kind"]["buck"]["theta_ja"]["points"][1]["decikelvin_per_watt"] =
            json!(430);
    });
    assert!(r.contains("θ falling or level"), "{r}");
}

#[test]
fn a_bias_lie_and_a_short_effective_capacitance_are_refused() {
    let derate = |v: &mut Value, ratio: u32, kept: u32| {
        let d = &mut v["power"]["conditions"]["dc_bias_derating"][0];
        d["ratio_max_permille"] = json!(ratio);
        d["retained_permille"] = json!(kept);
    };
    // A derating above 1, or of nothing, is refused by the shape.
    for (ratio, kept) in [(1000, 1001), (0, 500), (1000, 0)] {
        let mut v = base();
        derate(&mut v, ratio, kept);
        assert_power(
            &refused(&v),
            Refusal::PowerValueInvalid,
            "power.conditions.dc_bias_derating[0]",
        );
    }
    // The LDO's 0.47 µF minimum is strict: 470 per mille holds 470000 pF and
    // is refused; 471 holds 471000 pF and passes.
    for (kept, ok) in [(470, false), (471, true)] {
        let mut v = base();
        derate(&mut v, 1000, kept);
        let r = spec(&v).validate().map(|_| ());
        if ok {
            assert_eq!(r, Ok(()), "{kept}");
        } else {
            assert_power(
                &r.unwrap_err(),
                Refusal::EffectiveCapacitanceBelowMinimum,
                "U12",
            );
        }
    }
    // No derating covers the LDO's input at 217 per mille of its rating (the
    // buck's input reads its maker's curve): the screen cannot be computed,
    // so it refuses.
    let mut v = base();
    derate(&mut v, 200, 500);
    assert_power(
        &refused(&v),
        Refusal::EffectiveCapacitanceBelowMinimum,
        "U12",
    );
    // A held curve is used before the derating, composed with the tolerance
    // and the temperature limit, each factor deciding: at -350 per mille,
    // 1 µF x 0.65 x 0.9 x 0.78 = 456300 pF is refused against 0.47 µF, and
    // without either loss it would pass; at -300, 491400 pF passes.
    for (change, tol, temp, ok) in [
        (-350, 100, 220, false),
        (-350, 0, 220, true),
        (-350, 100, 0, true),
        (-300, 100, 220, true),
    ] {
        let mut t = table_json();
        row(&mut t, "GRM188R71C105KA12")["kind"]["capacitor"]["dc_bias"] =
            curve(GRM_SHEET, &[(0, change), (16_000, change)], tol, temp);
        let r = against(&base(), &t);
        if ok {
            assert_eq!(r, Ok(()), "{change} {tol} {temp}");
        } else {
            assert_power(
                &r.unwrap_err(),
                Refusal::EffectiveCapacitanceBelowMinimum,
                "U12",
            );
        }
    }
    // A curve composing to above the rated value does not parse (a bias lie).
    let r = table_refusal(|t| {
        row(t, "GRM188R71C105KA12")["kind"]["capacitor"]["dc_bias"] =
            curve(GRM_SHEET, &[(0, 200), (16_000, 0)], 0, 0);
    });
    assert!(r.contains("above the rated capacitance"), "{r}");
    // The buck's `cin` rule with its shortfall only reported, as it was
    // before `AET-WO-4300k` D14 enforced it.
    let report = |t: &mut Value| {
        row(t, "LMR33630ADDA")["kind"]["buck"]["external"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|e| e["role"] == "cin")
            .unwrap()["effective_min_picofarads"]["on_shortfall"] = json!("report");
    };
    // Past the curve's last point it is refused by name, never extrapolated,
    // even where the shortfall would only be reported (the buck's input):
    // the curve cut at 15 V, under the 4S pack's 17400 mV (at 6S it was cut
    // at 25 V, under 26100 mV).
    let mut t = table_json();
    report(&mut t);
    row(&mut t, "GCM32EC71H106KA03")["kind"]["capacitor"]["dc_bias"] = curve(
        "MURATA_GCM32EC71H106KA03_DCBIAS",
        &[(0, 3), (15_000, -172)],
        100,
        220,
    );
    let err = against(&base(), &t).unwrap_err();
    assert_power(&err, Refusal::DcBiasBeyondCurve, "U10");
    assert!(err.to_string().contains("which ends at 15000 mV"), "{err}");
    // Ruled D2 (c), enforced since D14: one ceramic, C60 removed, holds
    // 10 µF x (1 - 0.226 at 17.4 V) x 0.9 x 0.78 = 5433480 pF, at or under
    // the 10 µF minimum, and is refused.
    let mut v = base();
    drop_ext(&mut v, "U10", "C60");
    let err = refused(&v);
    assert_power(&err, Refusal::EffectiveCapacitanceBelowMinimum, "U10");
    assert!(
        err.to_string()
            .contains("holds 5433480 pF effective at 17400 mV"),
        "{err}"
    );
    // Where the row only reports it, the shortfall is named, not blocking:
    // 10000000 - 5433480 = 4566520 pF, 4.566 µF short.
    let mut t = table_json();
    report(&mut t);
    let c = conditions(&v, &t);
    let has = |s: &str| c.iter().any(|x| x.contains(s));
    assert!(has(
        "U10: `cin` holds 5.433 µF effective at 17400 mV under DC bias"
    ));
    assert!(has("4.566 µF short of the 10.000 µF minimum"));
    assert!(has("a named shortfall, not blocking"));
    // A declared class-2 derating keeps at most 50 % (ruled on PR #158); a
    // C0G band may keep all of it.
    let mut v = base();
    derate(&mut v, 1000, 501);
    assert_power(
        &refused(&v),
        Refusal::PowerValueInvalid,
        "power.conditions.dc_bias_derating[0]",
    );
    let mut v = base();
    v["power"]["conditions"]["dc_bias_derating"]
        .as_array_mut()
        .unwrap()
        .push(json!({"dielectrics": ["C0G"], "ratio_max_permille": 1000, "retained_permille": 1000, "source": "class 1"}));
    assert_eq!(spec(&v).validate().map(|_| ()), Ok(()));
    v["power"]["conditions"]["dc_bias_derating"][1]["retained_permille"] = json!(1001);
    assert_power(
        &refused(&v),
        Refusal::PowerValueInvalid,
        "power.conditions.dc_bias_derating[1]",
    );
}

#[test]
fn a_floor_between_the_falling_and_rising_maxima_is_refused() {
    // VDD_3V3's minimum is 3135 mV. A rising maximum there is refused; one
    // millivolt below it passes; the falling maximum stays 2680 mV.
    for (rise, ok) in [(3135, false), (3134, true)] {
        let mut t = table_json();
        row(&mut t, "STM32H753ZITx")["kind"]["mcu"]["bor_rising_max_millivolts"][3]["value"] =
            json!(rise);
        let r = against(&base(), &t);
        if ok {
            assert_eq!(r, Ok(()), "{rise}");
        } else {
            assert_power(
                &r.unwrap_err(),
                Refusal::SupplyFloorBelowBorRising,
                "VDD_3V3",
            );
        }
    }
    // A rising maximum below its falling one does not parse.
    let r = table_refusal(|t| {
        row(t, "STM32H753ZITx")["kind"]["mcu"]["bor_rising_max_millivolts"][3]["value"] =
            json!(2600);
    });
    assert!(r.contains("at or above its falling maximum"), "{r}");
}

#[test]
fn an_unwired_control_pin_is_refused_by_name() {
    let cases: Vec<Case> = vec![
        ("EN not declared", Refusal::ControlPinUnwired, "U10", |v| {
            reg(v, "U10")["controls"].as_array_mut().unwrap().remove(0);
        }),
        ("EN left open", Refusal::ControlPinUnwired, "U11", |v| {
            reg(v, "U11")["controls"][0]["strap"] = json!("open");
        }),
        (
            "the LDO's EN to VCCA",
            Refusal::ControlPinUnwired,
            "U12",
            |v| {
                reg(v, "U12")["controls"][0]["strap"] = json!("vcca");
            },
        ),
        ("FB as a control", Refusal::PowerValueInvalid, "U10", |v| {
            reg(v, "U10")["controls"]
                .as_array_mut()
                .unwrap()
                .push(json!({"pin": "FB", "strap": "open", "wire": null}));
        }),
        ("EN twice", Refusal::PowerValueInvalid, "U10", |v| {
            reg(v, "U10")["controls"]
                .as_array_mut()
                .unwrap()
                .push(json!({"pin": "EN", "strap": "input", "wire": null}));
        }),
    ];
    for (what, kind, id, mutate) in cases {
        let mut v = base();
        mutate(&mut v);
        assert_case(&refused(&v), kind, id, what);
    }
    // The translator's OE left open disables the bus.
    let mut v = full();
    v["power"]["crossings"][0]["controls"][0]["strap"] = json!("open");
    assert_power(&refused(&v), Refusal::ControlPinUnwired, "U13");
    // Both or neither of `strap` and `wire` is refused by the shape.
    for wire in [json!({"net": "PG_5V", "role": "a role"}), Value::Null] {
        let mut v = base();
        reg(&mut v, "U10")["controls"][1]["wire"] = wire.clone();
        if wire.is_null() {
            reg(&mut v, "U10")["controls"][1]["strap"] = Value::Null;
        }
        assert_power(
            &refused(&v),
            Refusal::PowerValueInvalid,
            "power.regulators[0].controls[1]",
        );
    }
    // PG wired to a named net with its role passes.
    let mut v = base();
    reg(&mut v, "U10")["controls"][1] = json!({"pin": "PG", "strap": null,
        "wire": {"net": "PG_5V", "role": "read by the MCU as the 5 V rail's power-good"}});
    assert_eq!(spec(&v).validate().map(|_| ()), Ok(()));
    // A wire a row does not allow is refused; a control the row lacks does
    // not parse.
    let mut t = table_json();
    row(&mut t, "LMR33630ADDA")["kind"]["buck"]["controls"][1]["wirable"] = json!(false);
    assert_power(
        &against(&v, &t).unwrap_err(),
        Refusal::ControlPinUnwired,
        "U10",
    );
    let r = table_refusal(|t| {
        row(t, "LMR33630ADDA")["kind"]["buck"]["controls"][0]["pin"] = json!("XX");
    });
    assert!(r.contains("control pins are exactly"), "{r}");
    let r = table_refusal(|t| {
        row(t, "LMR33630ADDA")["kind"]["buck"]["controls"][0]["straps"] = json!(["input", "input"]);
    });
    assert!(r.contains("a strap twice"), "{r}");
}

/// The edges of every new guard, each where the datasheet's comparison
/// decides it (the mutation run's survivors).
#[test]
fn the_new_guards_hold_at_their_edges() {
    // A TVS standing off exactly the 4S pack's 4 × 4350 = 17400 mV is
    // refused by its stand-off alone; one clamping exactly at 38 V passes.
    let mut t = tvs_table();
    row(&mut t, "TVS_TEST_PASS")["kind"]["tvs"]["standoff_millivolts"]["value"] = json!(17_400);
    let err = against(&with_clamp("TVS_TEST_PASS"), &t).unwrap_err();
    assert_power(&err, Refusal::BatteryInputUndamped, "D2");
    assert!(err.to_string().contains("stands off 17400 mV"), "{err}");
    let mut t = tvs_table();
    row(&mut t, "TVS_TEST_PASS")["kind"]["tvs"]["clamp_max_millivolts"]["value"] = json!(38_000);
    assert_eq!(against(&with_clamp("TVS_TEST_PASS"), &t), Ok(()));
    // Bulk at exactly 20 µF, and two at exactly 100 µF, pass the bulk
    // judge. Two at 100 µF melt the real fuse first (`AET-WO-4300k` (10)):
    // 100 µF + the 40.44 µF of input ceramics, rated, is 140440000 pF, and
    // ⌈140440000 × 17400² / (2 × 5625 × 10⁶)⌉ = 3779522 mA²s is at or above
    // SF-1206F700-2's 3250000; so the copy raises the fuse's melting I²t
    // past it. One at 20 µF, 60440000 pF, puts 1626562 mA²s, under it.
    for (pf, parts) in [(20_000_000, 1), (50_000_000, 2)] {
        let mut t = table_json();
        row(&mut t, "UCZ1H470MCL1GS")["kind"]["bulk_capacitor"]["rated_picofarads"]["value"] =
            json!(pf);
        let mut v = base();
        if parts == 2 {
            v["power"]["input"]["hot_plug"]
                .as_array_mut()
                .unwrap()
                .push(json!({"reference": "C2", "part": "UCZ1H470MCL1GS", "side": "board"}));
            let err = against(&v, &t).unwrap_err();
            assert_power(&err, Refusal::FuseMeltsOnHotPlug, "F1");
            assert!(err.to_string().contains("puts up to 3779522 mA²s"), "{err}");
            row(&mut t, "SF-1206F700-2")["kind"]["fuse"]["melting_i2t_milliamp2_seconds"]["value"] =
                json!(3_779_523);
        }
        assert_eq!(against(&v, &t), Ok(()), "{pf} x {parts}");
    }
    // The junction, screened at the copper measured on the drawn board
    // (`AET-WO-4300k` (a)): exactly 125 °C passes (1600 mW × 25.0 °C/W at
    // 58 cm², 5800 mm²: 85000 + 40000 m°C), and the rise rounds up (1001 mW
    // × 26.5 °C/W at 40 cm², 4000 mm², is 26526.5 mK, 26527).
    let mut v = base();
    reg(&mut v, "U10")["junction"]["loss_milliwatts"]["value"] = json!(1600);
    assert_eq!(spec(&v).validate().map(|_| ()), Ok(()));
    let s = power::junction_screen(&spec(&v), "U10", 5800).unwrap();
    assert!(
        s.contains("is 125000 m°C on a 85 °C board, 0 mK under 125 °C"),
        "{s}"
    );
    // `validate` needs only that some area clears: its edge is the figure's
    // lowest θ, so 1601 mW × 25.0 °C/W = 40025 mK, 125025 m°C, is refused.
    let mut over = base();
    reg(&mut over, "U10")["junction"]["loss_milliwatts"]["value"] = json!(1601);
    let err = refused(&over);
    assert_power(&err, Refusal::BuckJunctionOverLimit, "U10");
    assert!(err.to_string().contains("125025 m°C"), "{err}");
    reg(&mut v, "U10")["junction"]["loss_milliwatts"]["value"] = json!(1001);
    let s = power::junction_screen(&spec(&v), "U10", 4000).unwrap();
    assert!(s.contains("is 111527 m°C on a 85 °C board"), "{s}");
    // Declared values past their maxima. The copper is no longer declared
    // (`AET-WO-4300k` (a)): a junction carrying `copper_square_centimetres`,
    // at its former maximum or past it, does not parse.
    for value in [10_000, 10_001] {
        let mut v = base();
        reg(&mut v, "U10")["junction"]["copper_square_centimetres"] =
            json!({"value": value, "source": "s"});
        let e = serde_json::from_value::<Specification>(v).unwrap_err();
        assert!(
            e.to_string()
                .contains("unknown field `copper_square_centimetres`"),
            "{value}: {e}"
        );
    }
    let mut v = base();
    reg(&mut v, "U10")["junction"]["loss_milliwatts"]["value"] = json!(1_000_001);
    assert_power(
        &spec(&v).validate().unwrap_err(),
        Refusal::PowerValueInvalid,
        "U10",
    );
    // A derating band past 1000 per mille is refused.
    let mut v = base();
    v["power"]["conditions"]["dc_bias_derating"][0]["ratio_max_permille"] = json!(1001);
    assert_power(
        &refused(&v),
        Refusal::PowerValueInvalid,
        "power.conditions.dc_bias_derating[0]",
    );
    // The smallest band covering the ratio is used, its edge inclusive: the
    // LDO's input works at 217 per mille, so a band ending at 217 keeping 480
    // per mille clears it, where the band to 1000 keeping 300 would not; its
    // output, at 119 per mille, takes the band to 150.
    let bands = json!([
        {"dielectrics": ["X5R", "X7R", "X7S"], "ratio_max_permille": 1000, "retained_permille": 300, "source": "a wide band"},
        {"dielectrics": ["X7R"], "ratio_max_permille": 217, "retained_permille": 480, "source": "a narrow band"},
        {"dielectrics": ["X7R"], "ratio_max_permille": 150, "retained_permille": 500, "source": "a low band"}
    ]);
    let mut v = base();
    v["power"]["conditions"]["dc_bias_derating"] = bands.clone();
    assert_eq!(spec(&v).validate().map(|_| ()), Ok(()));
    // Bands that make the verdict hang on their order are refused
    // (adversarial F6): two at one ratio, or a narrower one keeping less.
    for (i, field, value) in [
        (2, "ratio_max_permille", 217),
        (1, "retained_permille", 200),
    ] {
        let mut v = base();
        v["power"]["conditions"]["dc_bias_derating"] = bands.clone();
        v["power"]["conditions"]["dc_bias_derating"][i][field] = json!(value);
        assert_power(
            &refused(&v),
            Refusal::PowerValueInvalid,
            &format!("power.conditions.dc_bias_derating[{}]", i.max(1)),
        );
    }
    // A point exactly at the working voltage is used as it is: -480 per mille
    // at 3465 mV keeps 520000 pF, clearing the LDO's input where the declared
    // 400 per mille would not; its output at 1890 mV interpolates to -262.
    let mut v = base();
    v["power"]["conditions"]["dc_bias_derating"][0]["retained_permille"] = json!(400);
    let mut t = table_json();
    row(&mut t, "GRM188R71C105KA12")["kind"]["capacitor"]["dc_bias"] =
        curve(GRM_SHEET, &[(0, 0), (3465, -480), (16_000, -600)], 0, 0);
    assert_eq!(against(&v, &t), Ok(()));
    // An effective minimum on a role with no working voltage cannot clear.
    let mut t = table_json();
    let ext = row(&mut t, "LMR33630ADDA")["kind"]["buck"]["external"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|e| e["role"] == "cboot")
        .unwrap();
    ext["effective_min_picofarads"] = json!({"value": 1, "at": {"document": "SNVSAN3F", "section": "test only"}, "on_shortfall": "enforce"});
    assert_power(
        &against(&base(), &t).unwrap_err(),
        Refusal::EffectiveCapacitanceBelowMinimum,
        "U10",
    );
    // The parser's edges: an odd layer count, a repeated area or bias
    // voltage, and a bias voltage above the rating are refused; a level θ,
    // level capacitance, and a point at the rated voltage or capacitance pass.
    let theta =
        |t: &mut Value| -> Value { row(t, "LMR33630ADDA")["kind"]["buck"]["theta_ja"].clone() };
    for (change, bad) in [
        (json!({"board_copper_layers": 3}), true),
        (
            json!({"points": [{"square_centimetres": 7, "decikelvin_per_watt": 420}, {"square_centimetres": 7, "decikelvin_per_watt": 400}]}),
            true,
        ),
        (
            json!({"points": [{"square_centimetres": 7, "decikelvin_per_watt": 420}, {"square_centimetres": 10, "decikelvin_per_watt": 420}]}),
            false,
        ),
    ] {
        let mut t = table_json();
        let mut th = theta(&mut t);
        for (k, val) in change.as_object().unwrap() {
            th[k] = val.clone();
        }
        row(&mut t, "LMR33630ADDA")["kind"]["buck"]["theta_ja"] = th;
        let parsed = parse_power_parts(&t.to_string());
        assert_eq!(parsed.is_err(), bad, "{change}");
    }
    let bias = |c: Value| {
        let mut t = table_json();
        row(&mut t, "GRM188R71C105KA12")["kind"]["capacitor"]["dc_bias"] = c;
        parse_power_parts(&t.to_string()).is_err()
    };
    let g = |points: &[(i64, i64)], tol: i64, temp: i64| curve(GRM_SHEET, points, tol, temp);
    assert!(bias(g(&[(0, 0), (16_001, -500)], 0, 0)), "past the rating");
    assert!(
        bias(g(&[(0, 0), (8000, -100), (8000, -200)], 0, 0)),
        "a repeated voltage"
    );
    assert!(bias(g(&[(1000, 0), (16_000, -500)], 0, 0)), "not from 0 mV");
    assert!(bias(g(&[(0, 0)], 0, 0)), "one point");
    assert!(bias(g(&[(0, 0), (16_000, -1000)], 0, 0)), "all of it lost");
    assert!(
        bias(g(&[(0, 0), (16_000, -500)], 1000, 0)),
        "a tolerance of 1"
    );
    assert!(bias(g(&[(0, 0), (16_000, -500)], 0, -1)), "a negative loss");
    assert!(
        !bias(g(&[(0, 0), (16_000, -999)], 999, 999)),
        "the edges pass"
    );
    // Past its last point a curve is never extrapolated: a curve to 1 V
    // refuses the LDO's input at 3465 mV by name.
    let mut t = table_json();
    row(&mut t, "GRM188R71C105KA12")["kind"]["capacitor"]["dc_bias"] =
        g(&[(0, 0), (1000, -100)], 0, 0);
    assert_power(
        &against(&base(), &t).unwrap_err(),
        Refusal::DcBiasBeyondCurve,
        "U12",
    );
    // A control's row names exactly its kind's control pins (F7), and a VIN
    // absolute maximum below the recommended maximum does not parse (F12).
    let r = table_refusal(|t| row(t, "LMR33630ADDA")["kind"]["buck"]["controls"] = json!([]));
    assert!(r.contains("control pins are exactly"), "{r}");
    let r = table_refusal(|t| {
        row(t, "LMR33630ADDA")["kind"]["buck"]["vin_abs_max_millivolts"]["value"] = json!(35_999);
    });
    assert!(r.contains("below the recommended VIN maximum"), "{r}");
    let r = table_refusal(|t| {
        row(t, "TLV75518PDBV")["kind"]["ldo"]["vin_abs_max_millivolts"]["value"] = json!(5_499);
    });
    assert!(r.contains("kind.ldo.vin_abs_max_millivolts"), "{r}");
}

/// Adversarial F1 to F4: a wire to a rail, ground or the battery is a strap
/// in disguise and is refused; a wire to any other net is named.
#[test]
fn a_wire_to_a_rail_is_refused_and_every_wire_is_named() {
    for (r, i, net) in [
        ("U10", 0, "GND"),
        ("U10", 1, "vbat"),
        ("U12", 0, "VDD_3V3"),
        ("U11", 1, "battery"),
    ] {
        let mut v = base();
        reg(&mut v, r)["controls"][i] = json!({"pin": if i == 0 { "EN" } else { "PG" },
            "strap": null, "wire": {"net": net, "role": "a role"}});
        assert_power(&refused(&v), Refusal::ControlPinUnwired, r);
    }
    let mut v = full();
    v["power"]["crossings"][0]["controls"][0] =
        json!({"pin": "OE", "strap": null, "wire": {"net": "gnd", "role": "a role"}});
    assert_power(&refused(&v), Refusal::ControlPinUnwired, "U13");
    let mut v = base();
    reg(&mut v, "U10")["controls"][1] = json!({"pin": "PG", "strap": null, "wire": {"net": "PG_5V", "role": "the 5 V rail's power-good"}});
    let c = conditions(&v, &table_json());
    assert!(c.iter().any(|x| x.contains("U10: PG is declared wired to `PG_5V` (the 5 V rail's power-good)")));
    // `AET-WO-1315` §1.5: the board draws a wire, so the condition no longer
    // says it does not; what it does not judge stays.
    assert!(
        c.iter()
            .any(|x| x.ends_with("neither the net's driver nor its level is judged"))
    );
    assert!(!c.iter().any(|x| x.contains("does not draw")));
}

/// `AET-WO-4400b` D4, D6: the MCU row's flash size and what ArduPilot
/// `Copter-4.7.2` calls the MCU and clocks its SPI peripherals at. Since
/// `AET-WO-4410` (2), for each cited `OSCILLATOR_HZ`: `0`, SPI1-3 on PLL1_Q
/// at 80 MHz and SPI4-6 on PLL2_Q at 72 MHz; `16000000`, 80 MHz and 100 MHz.
#[test]
fn the_mcu_row_holds_its_flash_and_ardupilot_s_names_and_kernels() {
    let t = power_parts::table().unwrap();
    let power_parts::Kind::Mcu(m) = &t.get("STM32H753ZITx").unwrap().kind else {
        panic!("the MCU row");
    };
    assert_eq!(m.flash_kilobytes.value, 2048);
    assert_eq!(m.flash_kilobytes.at.document, "DS12117");
    let a = &m.ardupilot;
    assert_eq!(
        (a.series.as_str(), a.mcu.as_str()),
        ("STM32H7xx", "STM32H753xx")
    );
    let cited: Vec<u32> = a.oscillators.iter().map(|o| o.oscillator_hz).collect();
    assert_eq!(cited, [0, 16_000_000]);
    for (hz, low, high) in [
        (0, 80_000_000, 72_000_000),
        (16_000_000, 80_000_000, 100_000_000),
    ] {
        let o = a.oscillator(hz).unwrap();
        assert_eq!(o.at.document, "AP_STM32H7_MCUCONF", "{hz}");
        let kernels: Vec<(&str, i64)> = o
            .spi_kernel_hz
            .iter()
            .map(|(p, l)| (p.as_str(), l.value))
            .collect();
        assert_eq!(
            kernels,
            [
                ("SPI1", low),
                ("SPI2", low),
                ("SPI3", low),
                ("SPI4", high),
                ("SPI5", high),
                ("SPI6", high)
            ],
            "{hz}"
        );
        for l in o.spi_kernel_hz.values() {
            assert_eq!(l.at.document, "CHIBIOS_HAL_LLD_TYPE1");
            assert!(
                l.at.section.starts_with(&format!("OSCILLATOR_HZ {hz}:")),
                "{}",
                l.at.section
            );
            assert!(matches!(
                t.document(&l.at.document).unwrap().held,
                Held::Sha256(_)
            ));
        }
    }
    assert!(a.oscillator(8_000_000).is_none());
}

/// The MCU row's `kind.mcu` inside a parsed copy of the table.
fn mcu_of(v: &mut Value) -> &mut Value {
    let rows = v["parts"].as_array_mut().unwrap();
    let row = rows
        .iter_mut()
        .find(|r| r["part"] == "STM32H753ZITx")
        .unwrap();
    &mut row["kind"]["mcu"]
}

/// The column's shape, each refused by name: a peripheral missing from or
/// added to an oscillator's `spi_kernel_hz`, a zero kernel, no oscillator,
/// one oscillator cited twice (`AET-WO-4410` (2)), a name ArduPilot cannot
/// read, and a missing `flash_kilobytes`.
#[test]
fn a_malformed_mcu_ardupilot_column_is_refused() {
    type Edit = (fn(&mut Value), &'static str);
    let cases: [Edit; 7] = [
        (
            |m| {
                m["ardupilot"]["oscillators"][0]["spi_kernel_hz"]
                    .as_object_mut()
                    .unwrap()
                    .remove("SPI6");
            },
            "oscillators[0].spi_kernel_hz` is keyed by exactly SPI1 to SPI6",
        ),
        (
            |m| {
                m["ardupilot"]["oscillators"][1]["spi_kernel_hz"]["SPI7"] =
                    m["ardupilot"]["oscillators"][1]["spi_kernel_hz"]["SPI1"].clone()
            },
            "oscillators[16000000].spi_kernel_hz` is keyed by exactly SPI1 to SPI6",
        ),
        (
            |m| m["ardupilot"]["oscillators"][1]["spi_kernel_hz"]["SPI2"]["value"] = 0.into(),
            "oscillators[16000000].spi_kernel_hz.SPI2",
        ),
        (
            |m| m["ardupilot"]["oscillators"] = serde_json::json!([]),
            "cites no oscillator",
        ),
        (
            |m| {
                m["ardupilot"]["oscillators"][1]["oscillator_hz"] = 0.into();
            },
            "cites OSCILLATOR_HZ 0 twice",
        ),
        (
            |m| m["ardupilot"]["mcu"] = "STM32H753 xx".into(),
            "kind.mcu.ardupilot.mcu",
        ),
        (
            |m| {
                m.as_object_mut().unwrap().remove("flash_kilobytes");
            },
            "flash_kilobytes",
        ),
    ];
    for (mutate, field) in cases {
        let mut v: Value = serde_json::from_str(POWER_PARTS_JSON).unwrap();
        mutate(mcu_of(&mut v));
        let err = parse_power_parts(&v.to_string()).unwrap_err();
        assert!(err.to_string().contains(field), "`{field}` not in: {err}");
    }
}

/// `AET-WO-2304b` §1.2: `series_milliohms` is `null` or `{value, source}`,
/// a 0 mΩ path refused, an unsourced one refused; the key is required (the
/// walker in `validate.rs` removes it).
#[test]
fn the_healthy_series_resistance_is_null_or_a_sourced_nonzero_value() {
    let mut v = base();
    v["power"]["battery"]["series_milliohms"] = Value::Null;
    assert_eq!(spec(&v).validate(), Ok(()));
    v["power"]["battery"]["series_milliohms"] = json!({"value": 0, "source": "s"});
    assert_power(
        &refused(&v),
        Refusal::PowerValueInvalid,
        "power.battery.series_milliohms",
    );
    v["power"]["battery"]["series_milliohms"] = json!({"value": 1, "source": ""});
    assert!(
        matches!(refused(&v), E::InvalidSource { ref path } if path == "power.battery.series_milliohms.source")
    );
}

/// `AET-WO-2304b` §1.5: the power fixture's battery side, one derivation
/// from the specification and the table (measured): V_open 3 × 3,000 mV;
/// U10 and U11 at ⌈VOUT max × load⌉ + their declared loss, each with its
/// minimum operating input and its D-B9 regulation floor; C1 at its rating
/// with its ESR maximum, then each battery-fed input ceramic as IR credits it.
#[test]
fn the_battery_side_is_derived_from_the_specification_and_the_table() {
    let side = power::battery_side(&spec(&full())).unwrap().unwrap();
    assert_eq!(side.v_open_microvolts, 9_000_000);
    assert_eq!(side.healthy.as_ref().map(|h| h.value), Some(1));
    let sinks: Vec<(&str, u64, u64, u64)> = side
        .sinks
        .iter()
        .map(|k| {
            (
                k.reference.as_str(),
                k.input_milliwatts,
                k.shutdown_millivolts,
                k.regulation_millivolts,
            )
        })
        .collect();
    assert_eq!(
        sinks,
        [("U10", 11_645, 3_800, 7_000), ("U11", 2_997, 3_800, 3_800)]
    );
    assert_eq!(
        side.sinks.iter().map(|k| k.input_milliwatts).sum::<u64>(),
        14_642
    );
    assert_eq!(side.capacitors[0].reference, "C1");
    assert_eq!(side.capacitors[0].picofarads, 47_000_000);
    assert_eq!(side.capacitors[0].esr_max_milliohms, Some(250));
    assert!(
        side.capacitors[1..]
            .iter()
            .all(|c| c.esr_max_milliohms.is_none())
    );
    // A board with no tree has no battery side.
    let mut none = base();
    none["power"] = Value::Null;
    none["mcu"]["supply_source"] = json!({"connector": "J2", "pin": "1"});
    assert!(matches!(
        power::battery_side(&spec(&none)),
        Ok(None) | Err(_)
    ));
}

/// `AET-WO-2304b` §1.6: the shutdown clause, through a test-only table that
/// validation admits (the `1315` precedent): the 5 V row's regulation input
/// lowered below the part's 3,800 mV minimum operating input. No row held
/// is so; on every real one the regulation floor dominates.
#[test]
fn a_test_only_table_gives_a_sink_whose_shutdown_floor_binds() {
    let mut t = table_json();
    row(&mut t, "LMR33630ADDA")["kind"]["buck"]["regulation_vin_min"][0]["vin_min"]["value"] =
        json!(3000);
    let table = parse_power_parts(&t.to_string()).expect("the changed table parses");
    let sensors = aethelgard_ir::parts::table().unwrap();
    let side = power::battery_side_against(
        &spec(&full()),
        &table,
        sensors,
        &power::output_delays(sensors),
    );
    // Validation itself refuses a battery below a regulation floor only; a
    // floor lowered below the shutdown floor is admitted.
    let side = side.unwrap().unwrap();
    let u10 = &side.sinks[0];
    assert_eq!(
        (u10.shutdown_millivolts, u10.regulation_millivolts),
        (3_800, 3_000)
    );
    assert_eq!(u10.floor_millivolts(), 3_800);
}

/// F3: each regulator's output capacitors, at IR's credited figure (no
/// maker curve is held for these parts; the fixture's 500 per mille
/// derating), measured.
#[test]
fn each_output_capacitor_is_credited_by_ir_s_own_derivation() {
    let out = power::output_capacitance(&spec(&full())).unwrap();
    let by: Vec<(&str, &str, u64)> = out
        .iter()
        .map(|(r, c)| (r.as_str(), c.domain.as_str(), c.picofarads))
        .collect();
    assert_eq!(
        by,
        [
            ("C14", "VDD_5V", 11_000_000),
            ("C15", "VDD_5V", 11_000_000),
            ("C16", "VDD_5V", 11_000_000),
            ("C17", "VDD_5V", 11_000_000),
            ("C24", "VDD_3V3", 11_000_000),
            ("C25", "VDD_3V3", 11_000_000),
            ("C26", "VDD_3V3", 11_000_000),
            ("C27", "VDD_3V3", 11_000_000),
            ("C31", "VDD_1V8", 500_000),
        ]
    );
}

/// The MCU row's `io` (`AET-WO-1211e` A1): each family upper-case and once,
/// stating a term, each above zero, each pin canonical with a structure of
/// a held family on a page, and at least one pin; each refused by name.
#[test]
fn a_malformed_io_row_is_refused() {
    fn io(t: &mut Value) -> &mut Value {
        &mut row(t, "STM32H753ZITx")["kind"]["mcu"]["io"]
    }
    type Case = (&'static str, fn(&mut Value));
    let cases: [Case; 8] = [
        ("named once", |io| io["families"][1]["family"] = json!("FT")),
        ("states neither term", |io| {
            io["families"][1]["above_supply_millivolts"] = Value::Null
        }),
        ("named once", |io| io["families"][1]["family"] = json!("tt")),
        ("not above zero", |io| {
            io["families"][0]["above_supply_millivolts"] = json!(0)
        }),
        ("io.pins.PA16", |io| {
            io["pins"]["PA16"] = json!({"structure": "FT", "page": 68})
        }),
        ("io.pins.PA1", |io| {
            io["pins"]["PA1"]["structure"] = json!("XX_h")
        }),
        ("io.pins.PA1", |io| io["pins"]["PA1"]["page"] = json!(0)),
        ("is empty", |io| io["pins"] = json!({})),
    ];
    for (says, f) in cases {
        let r = table_refusal(|t| f(io(t)));
        assert!(r.contains(says), "`{says}` not in: {r}");
    }
}

/// The held rows: 114 I/O pins, 102 FT and 12 TT, each cited on its page
/// of DS12117 Rev 10 Table 8; in operation (Table 13) FT takes
/// `Min(VDD + 3.6 V, 5499 mV)` and TT `VDD + 0.3 V`.
#[test]
fn the_io_rows_are_the_lqfp144_s_pins() {
    let t = power_parts::table().unwrap();
    let power_parts::Kind::Mcu(m) = &t.get("STM32H753ZITx").unwrap().kind else {
        panic!("an mcu row")
    };
    let family = |f: &str| m.io.pins.values().filter(|(s, _)| s.starts_with(f)).count();
    assert_eq!(
        (m.io.pins.len(), family("FT"), family("TT")),
        (114, 102, 12)
    );
    assert_eq!(
        m.io.tolerated("PA1", 3135).map(|x| (x.0, x.2)),
        Some(("FT_ha", 5499))
    );
    assert_eq!(m.io.tolerated("PA1", 1805).map(|x| x.2), Some(5405));
    assert_eq!(
        m.io.tolerated("PA4", 3135).map(|x| (x.0, x.2)),
        Some(("TT_a", 3435))
    );
    assert!(m.io.pins["PA4"].1.section.starts_with("p. 69, Table 8"));
    assert_eq!(m.io.floor(3135), 3435);
    assert_eq!(m.io.pulls_off_above_supply.value, 300);
}
