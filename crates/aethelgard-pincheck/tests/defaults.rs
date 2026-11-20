//! Tier 1: an ELF that can apply parameter defaults at boot is refused by
//! name (`ParamDefaultsOverride`, `AET-WO-4406`), with every signature it
//! shows. The shapes are cut from the measured builds (`spike/4406` §2.1):
//! every committed and CI-built board defines `reload_defaults_file` and
//! lists `hwdef.dat` in ROMFS, and passes; `4405`'s two A1 builds each show
//! two signatures; and each signature alone is refused.

mod common;

use aethelgard_pincheck::elf::{self, LOAD_DEFAULTS_FILE, LOAD_EMBEDDED_PARAM_DEFAULTS};
use common::{
    ADVAPJ_PARAM_DEFAULTS, Elf, RELOAD_DEFAULTS_FILE, ROMFS_ADVLUA, ROMFS_DEFAULTS, ROMFS_HWDEF,
    RomfsFile, VarRow, scripting_var_info,
};

fn function(name: &[u8]) -> String {
    String::from_utf8(name.to_vec()).unwrap()
}

/// A board as every measured one is: `reload_defaults_file` linked, ROMFS
/// holding `hwdef.dat`, scripting compiled in with `SCR_ENABLE` 0.
fn board() -> Elf {
    Elf {
        functions: vec![RELOAD_DEFAULTS_FILE.to_owned()],
        romfs: Some(vec![ROMFS_HWDEF]),
        scripting: Some(scripting_var_info(0.0)),
        ..Elf::default()
    }
}

/// The signatures `elf` shows, each line up to its first `: `, or the
/// refusal's kind.
fn signatures(elf: &Elf) -> Result<Vec<String>, &'static str> {
    elf::defaults_signatures(&elf.bytes())
        .map(|found| {
            found
                .iter()
                .map(|s| {
                    s.split_once(": ")
                        .map_or(s.as_str(), |(head, _)| head)
                        .to_owned()
                })
                .collect()
        })
        .map_err(|e| e.kind())
}

const FILE: &str = "AP_Param::load_defaults_file is linked";
const EMBEDDED: &str = "AP_Param::load_embedded_param_defaults is linked";
const DATA: &str =
    "AP_Param::param_defaults_data holds 67 bytes of defaults (AP_Param.cpp:1618-1622)";
const ROMFS: &str = "ROMFS lists defaults.parm, which ChibiOS loads at boot as @ROMFS/defaults.parm (AP_HAL_ChibiOS/Util.h:43-45, AP_Param.cpp:1647-1666)";
const LUA: &str = "ROMFS lists scripts/adv.lua, a Lua script ArduPilot runs from @ROMFS/scripts when scripting is enabled, and a script can set parameters (boards.py:669-672, lua_scripts.cpp:548-550)";
const SCR_ENABLE: &str = "AP_Scripting::var_info's ENABLE (SCR_ENABLE) defaults to 1";

/// Every measured board's shape passes: `reload_defaults_file` is linked in
/// every build and is not a signature, and a ROMFS of `hwdef.dat` lists no
/// defaults. So does an ELF with no ROMFS symbol, as a board without ROMFS
/// links (`advnoromfs`, measured). A zero-size ROMFS, never measured, is
/// refused rather than read as empty.
#[test]
fn a_board_as_every_measured_one_passes() {
    assert_eq!(signatures(&board()), Ok(vec![]));
    assert!(elf::read(&board().bytes()).is_ok());
    assert_eq!(signatures(&Elf::default()), Ok(vec![]));
    let empty = Elf {
        romfs: Some(vec![]),
        ..board()
    };
    assert_eq!(signatures(&empty), Err("SymbolSize"));
}

/// `4405`'s A1 builds, as measured: `advromfs` (a `defaults.parm` beside
/// `hwdef.dat`) links `load_defaults_file` and lists it in ROMFS; `advapj`
/// (`AP_PARAM_MAX_EMBEDDED_PARAM 1024` and `DEFAULT_PARAMETERS`) links
/// `load_embedded_param_defaults` and holds 67 bytes in
/// `param_defaults_data`, cut from its ELF. `check pinout` reads the ELF
/// through `elf::read`, which refuses each by name with both signatures.
#[test]
fn the_a1_builds_are_refused_with_every_signature() {
    let advromfs = Elf {
        functions: vec![
            RELOAD_DEFAULTS_FILE.to_owned(),
            function(LOAD_DEFAULTS_FILE),
        ],
        romfs: Some(vec![ROMFS_DEFAULTS, ROMFS_HWDEF]),
        ..Elf::default()
    };
    let advapj = Elf {
        functions: vec![
            RELOAD_DEFAULTS_FILE.to_owned(),
            function(LOAD_EMBEDDED_PARAM_DEFAULTS),
        ],
        param_defaults: Some(ADVAPJ_PARAM_DEFAULTS.to_vec()),
        ..board()
    };
    assert_eq!(signatures(&advromfs), Ok(vec![FILE.into(), ROMFS.into()]));
    assert_eq!(signatures(&advapj), Ok(vec![EMBEDDED.into(), DATA.into()]));
    for elf in [advromfs, advapj] {
        let refusal = elf::read(&elf.bytes()).unwrap_err();
        assert_eq!(refusal.kind(), "ParamDefaultsOverride");
        assert!(
            refusal
                .to_string()
                .starts_with("the ELF can apply parameter defaults at boot"),
            "{refusal}"
        );
        assert_eq!(refusal.to_string().matches("; ").count(), 1, "{refusal}");
    }
}

/// Each signature alone is enough, and is named.
#[test]
fn each_signature_alone_is_refused() {
    let length_zero = {
        let mut b = ADVAPJ_PARAM_DEFAULTS.to_vec();
        b[18..20].copy_from_slice(&[0, 0]);
        b
    };
    for (elf, want) in [
        (
            Elf {
                functions: vec![function(LOAD_DEFAULTS_FILE)],
                ..board()
            },
            FILE,
        ),
        (
            Elf {
                functions: vec![function(LOAD_EMBEDDED_PARAM_DEFAULTS)],
                ..board()
            },
            EMBEDDED,
        ),
        (
            Elf {
                param_defaults: Some(ADVAPJ_PARAM_DEFAULTS.to_vec()),
                ..board()
            },
            DATA,
        ),
        (
            Elf {
                romfs: Some(vec![ROMFS_HWDEF, ROMFS_DEFAULTS]),
                ..board()
            },
            ROMFS,
        ),
    ] {
        assert_eq!(signatures(&elf), Ok(vec![want.to_owned()]), "{want}");
        assert_eq!(
            elf::read(&elf.bytes()).unwrap_err().kind(),
            "ParamDefaultsOverride",
            "{want}"
        );
    }
    // The region with nothing in it, and no loader linked: the length is
    // what is read.
    let empty_region = Elf {
        param_defaults: Some(length_zero),
        ..board()
    };
    assert_eq!(signatures(&empty_region), Ok(vec![]));
}

/// Each read is refused by name when it is malformed (`INV-06`): a
/// `param_defaults_data` without its magic, whose `max_length` is not its
/// room, whose length runs past `max_length`, or shorter than its header;
/// a ROMFS name with no NUL within its bound.
#[test]
fn every_malformed_read_is_refused_by_name() {
    let with = |f: &dyn Fn(&mut Vec<u8>)| {
        let mut b = ADVAPJ_PARAM_DEFAULTS.to_vec();
        f(&mut b);
        signatures(&Elf {
            param_defaults: Some(b),
            ..board()
        })
    };
    assert_eq!(with(&|b| b[0] = b'p'), Err("MalformedParamDefaults"));
    assert_eq!(with(&|b| b[15] ^= 1), Err("MalformedParamDefaults"));
    assert_eq!(
        with(&|b| b[16..18].copy_from_slice(&1023u16.to_le_bytes())),
        Err("MalformedParamDefaults")
    );
    assert_eq!(
        with(&|b| b[18..20].copy_from_slice(&1025u16.to_le_bytes())),
        Err("MalformedParamDefaults")
    );
    assert_eq!(with(&|b| b.truncate(19)), Err("SymbolSize"));
    let long: &'static str = Box::leak("s".repeat(129).into_boxed_str());
    let romfs = |name: &'static str| {
        signatures(&Elf {
            romfs: Some(vec![RomfsFile {
                name,
                ..ROMFS_HWDEF
            }]),
            ..board()
        })
    };
    assert_eq!(romfs(long), Err("RomfsNameTooLong"));
    assert_eq!(romfs(&long[..128]), Ok(vec![]));
    assert_eq!(
        elf::defaults_signatures(b"not an elf").unwrap_err().kind(),
        "Truncated"
    );
}

/// `advlua` (adversarial A1, ruled D-c): `SCRIPTING_ENABLE_DEFAULT 1` and a
/// ROMFS script, as measured. It is refused, naming both Lua signatures; and
/// each alone is refused. A file under `scripts` that is not `.lua` is not a
/// script ArduPilot runs (`boards.py:669-672`), and AP_Scripting compiled
/// out (no `AP_Scripting::var_info`, `advnoromfs`) is no signature.
#[test]
fn advlua_is_refused_naming_both_lua_signatures() {
    let advlua = Elf {
        romfs: Some(vec![ROMFS_HWDEF, ROMFS_ADVLUA]),
        scripting: Some(scripting_var_info(1.0)),
        ..board()
    };
    assert_eq!(signatures(&advlua), Ok(vec![LUA.into(), SCR_ENABLE.into()]));
    let refusal = elf::read(&advlua.bytes()).unwrap_err();
    assert_eq!(refusal.kind(), "ParamDefaultsOverride");
    let script = Elf {
        romfs: Some(vec![ROMFS_HWDEF, ROMFS_ADVLUA]),
        ..board()
    };
    let enabled = Elf {
        scripting: Some(scripting_var_info(1.0)),
        ..board()
    };
    assert_eq!(signatures(&script), Ok(vec![LUA.into()]));
    assert_eq!(signatures(&enabled), Ok(vec![SCR_ENABLE.into()]));
    for elf in [script, enabled] {
        assert_eq!(
            elf::read(&elf.bytes()).unwrap_err().kind(),
            "ParamDefaultsOverride"
        );
    }
    let readme = Elf {
        romfs: Some(vec![RomfsFile {
            name: "scripts/README.md",
            ..ROMFS_ADVLUA
        }]),
        ..board()
    };
    assert_eq!(signatures(&readme), Ok(vec![]));
    let compiled_out = Elf {
        scripting: None,
        ..board()
    };
    assert_eq!(signatures(&compiled_out), Ok(vec![]));
}

/// `AP_Scripting::var_info` is read as the other tables are, and its
/// `ENABLE` row must be one `AP_Int8` flagged `AP_PARAM_FLAG_ENABLE` alone:
/// anything else is refused by name, never read as off.
#[test]
fn a_malformed_scripting_table_is_refused_by_name() {
    let kind = |f: &dyn Fn(&mut Vec<VarRow>)| {
        let mut rows = scripting_var_info(0.0);
        f(&mut rows);
        signatures(&Elf {
            scripting: Some(rows),
            ..board()
        })
    };
    for (why, edit) in [
        (
            "no ENABLE",
            &(|r: &mut Vec<VarRow>| r[0].name = "ENABLED".into()) as &dyn Fn(&mut Vec<VarRow>),
        ),
        ("ENABLE twice", &|r: &mut Vec<VarRow>| {
            r[1].name = "ENABLE".into()
        }),
        ("an AP_Int32", &|r: &mut Vec<VarRow>| r[0].kind = 3),
        ("a default pointer", &|r: &mut Vec<VarRow>| {
            r[0].flags = 4 | 1 << 7
        }),
        ("no enable flag", &|r: &mut Vec<VarRow>| r[0].flags = 0),
    ] {
        assert_eq!(kind(edit), Err("MalformedScriptingVarInfo"), "{why}");
    }
    assert_eq!(
        kind(&|r| {
            r.pop();
        }),
        Err("VarInfoUnterminated")
    );
}
