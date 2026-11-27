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
    elf::defaults_signatures(&elf.bytes(), None)
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
        elf::defaults_signatures(b"not an elf", None)
            .unwrap_err()
            .kind(),
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

/// Sentinel-03's ROMFS `defaults.parm`, cut from its ELF (1cfc5e02, built in
/// the pinned image from `generate hwdef`'s files, `spike/4412`): 430 bytes
/// of raw deflate, and the 597 they inflate to, which `generate hwdef`
/// wrote for its topology (the same sha256, 14f29607).
const S03_DEFLATE: &[u8] = include_bytes!("fixtures/sentinel-03.defaults.parm.deflate");
const S03_DEFAULTS: &str = include_str!("fixtures/sentinel-03.defaults.parm");

/// A ROMFS `defaults.parm` holding `deflated`, which inflates to `len` bytes.
fn defaults_file(deflated: Vec<u8>, len: usize) -> RomfsFile {
    RomfsFile {
        compressed: deflated.len() as u32,
        decompressed: len as u32,
        contents: deflated,
        ..ROMFS_DEFAULTS
    }
}

/// A board that links `load_defaults_file` and embeds `file` as its ROMFS
/// `defaults.parm`, beside `hwdef.dat`.
fn embedding(file: RomfsFile) -> Elf {
    Elf {
        functions: vec![
            RELOAD_DEFAULTS_FILE.to_owned(),
            function(LOAD_DEFAULTS_FILE),
        ],
        romfs: Some(vec![file, ROMFS_HWDEF]),
        ..board()
    }
}

fn deflated(text: &str) -> RomfsFile {
    defaults_file(
        miniz_oxide::deflate::compress_to_vec(text.as_bytes(), 6),
        text.len(),
    )
}

/// The signatures with `defaults` admitted, each line whole.
fn admitting(elf: &Elf, defaults: &str) -> Result<Vec<String>, &'static str> {
    elf::defaults_signatures(&elf.bytes(), Some(defaults)).map_err(|e| e.kind())
}

/// `AET-WO-4412` (ruled 2026-11-26T17:35Z): the one defaults file admitted
/// is the one `generate hwdef` writes for the topology, byte for byte, and
/// with it `load_defaults_file`, which loads only that file on ChibiOS.
/// Sentinel-03's own, as its ELF embeds it, is admitted; with no file to
/// admit it is refused as `AET-WO-4406` built it.
#[test]
fn sentinel_03s_own_defaults_parm_is_admitted_and_nothing_else() {
    let s03 = embedding(defaults_file(S03_DEFLATE.to_vec(), S03_DEFAULTS.len()));
    assert_eq!(admitting(&s03, S03_DEFAULTS), Ok(vec![]));
    assert!(elf::read_admitting(&s03.bytes(), Some(S03_DEFAULTS)).is_ok());
    assert_eq!(signatures(&s03), Ok(vec![FILE.into(), ROMFS.into()]));
    assert_eq!(
        elf::read(&s03.bytes()).unwrap_err().kind(),
        "ParamDefaultsOverride"
    );
    // Every other signature still refuses beside an admitted file.
    let lua = Elf {
        romfs: Some(vec![
            defaults_file(S03_DEFLATE.to_vec(), S03_DEFAULTS.len()),
            ROMFS_HWDEF,
            ROMFS_ADVLUA,
        ]),
        scripting: Some(scripting_var_info(1.0)),
        ..s03.clone()
    };
    let lua = admitting(&lua, S03_DEFAULTS).unwrap();
    assert_eq!(lua.len(), 2, "{lua:?}");
    assert!(
        lua[0].starts_with(LUA) && lua[1].starts_with(SCR_ENABLE),
        "{lua:?}"
    );
    // `load_defaults_file` with no ROMFS defaults.parm is not admitted.
    let no_file = Elf {
        romfs: Some(vec![ROMFS_HWDEF]),
        ..s03
    };
    assert_eq!(admitting(&no_file, S03_DEFAULTS).map(|f| f.len()), Ok(1));
}

/// An extra line, and `MOT_PWM_TYPE` contradicting the topology, are each
/// refused, naming the first line that differs; `load_defaults_file` stays a
/// signature beside each. A file that does not inflate to its stated size is
/// refused by name.
#[test]
fn a_defaults_parm_other_than_the_topologys_is_refused_by_its_first_differing_line() {
    let extra = embedding(deflated(&format!("{S03_DEFAULTS}SERIAL1_PROTOCOL 2\n")));
    let found = admitting(&extra, S03_DEFAULTS).unwrap();
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(
        found[0],
        FILE.to_owned() + ": a defaults file is read at boot (AP_Param.cpp:1599-1616, :2406)"
    );
    // An extra line makes it longer than the topology's: refused by its
    // stated size, before it is inflated.
    assert!(
        found[1].starts_with(ROMFS)
            && found[1].ends_with(
                "it is not the one generate hwdef writes for this topology: it states 616 bytes, the topology's is 597"
            ),
        "{found:?}"
    );
    // A line changed within the topology's size is named, as the first that
    // differs.
    let same = embedding(deflated(
        &S03_DEFAULTS.replace("MOT_PWM_TYPE 6\n", "SERVO_RATE 999\n"),
    ));
    let found = admitting(&same, S03_DEFAULTS).unwrap();
    assert!(
        found[1].ends_with("its line 4 is `SERVO_RATE 999`, the topology's `MOT_PWM_TYPE 6`"),
        "{found:?}"
    );
    let pwm = embedding(deflated(
        &S03_DEFAULTS.replace("MOT_PWM_TYPE 6", "MOT_PWM_TYPE 5"),
    ));
    let found = admitting(&pwm, S03_DEFAULTS).unwrap();
    assert!(
        found[1].ends_with("its line 4 is `MOT_PWM_TYPE 5`, the topology's `MOT_PWM_TYPE 6`"),
        "{found:?}"
    );
    for elf in [extra, pwm] {
        assert_eq!(
            elf::read_admitting(&elf.bytes(), Some(S03_DEFAULTS))
                .unwrap_err()
                .kind(),
            "ParamDefaultsOverride"
        );
    }
    // A stated size other than the topology's file's is not that file, and is
    // refused by its size before anything is inflated.
    for stated in [S03_DEFAULTS.len() - 1, S03_DEFAULTS.len() + 1, 57] {
        let elf = embedding(defaults_file(vec![0xff; 40], stated));
        let found = admitting(&elf, S03_DEFAULTS).unwrap();
        assert!(
            found[1].ends_with(&format!(
                "it states {stated} bytes, the topology's is {}",
                S03_DEFAULTS.len()
            )),
            "{found:?}"
        );
    }
    // At the topology's size, contents that do not inflate to exactly it are
    // refused by name: bytes that are not deflate, a shorter file, a longer one.
    let n = S03_DEFAULTS.len();
    for contents in [
        vec![0xff; 40],
        miniz_oxide::deflate::compress_to_vec(&S03_DEFAULTS.as_bytes()[..n - 1], 6),
        miniz_oxide::deflate::compress_to_vec(format!("{S03_DEFAULTS}x").as_bytes(), 6),
    ] {
        let elf = embedding(defaults_file(contents, n));
        assert_eq!(admitting(&elf, S03_DEFAULTS), Err("MalformedRomfs"));
    }
}

/// The peak resident memory of this process (`VmHWM`), in KiB.
fn peak_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find_map(|l| l.strip_prefix("VmHWM:"))
        .and_then(|v| v.trim().trim_end_matches(" kB").trim().parse().ok())
        .unwrap()
}

/// `AET-WO-4412` review, 2026-11-26T23:39Z (1): a decompression bomb, 256 MiB
/// of zeros deflated to well under 1 MiB and stating 256 MiB, is refused by
/// its stated size before it is inflated, so it allocates nothing near its
/// size (INV-06: a refusal by name, never an out-of-memory death).
#[test]
fn a_defaults_parm_bomb_is_refused_by_its_size_before_it_is_inflated() {
    use miniz_oxide::deflate::core::{
        CompressorOxide, TDEFLFlush, TDEFLStatus, compress, create_comp_flags_from_zip_params,
    };
    const STATED: usize = 256 << 20;
    let zeros = vec![0u8; 1 << 20];
    let mut c = CompressorOxide::new(create_comp_flags_from_zip_params(9, -15, 0));
    let (mut bomb, mut chunk) = (Vec::new(), vec![0u8; 1 << 16]);
    for i in 0..STATED / zeros.len() {
        let flush = if i + 1 == STATED / zeros.len() {
            TDEFLFlush::Finish
        } else {
            TDEFLFlush::None
        };
        let mut input: &[u8] = &zeros;
        loop {
            let (status, read, wrote) = compress(&mut c, input, &mut chunk, flush);
            bomb.extend_from_slice(&chunk[..wrote]);
            input = &input[read..];
            if status == TDEFLStatus::Done || (input.is_empty() && wrote < chunk.len()) {
                break;
            }
        }
    }
    assert!(bomb.len() < 1 << 20, "{}", bomb.len());
    let elf = embedding(defaults_file(bomb, STATED)).bytes();
    let before = peak_kib();
    let refusal = elf::read_admitting(&elf, Some(S03_DEFAULTS)).unwrap_err();
    let grew = peak_kib() - before;
    assert_eq!(refusal.kind(), "ParamDefaultsOverride");
    assert!(
        refusal
            .to_string()
            .contains(&format!("it states {STATED} bytes, the topology's is")),
        "{refusal}"
    );
    assert!(grew < 64 << 10, "the bomb grew the peak by {grew} KiB");
}
