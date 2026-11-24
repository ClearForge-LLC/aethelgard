//! The vendored KiCad 10.0.7 symbols and footprints (the bytes sit in
//! `aethelgard-kicad-lib`, ruled D1 on PR #40): each file's sha256 is checked before any is used, and
//! symbols come out flattened, renamed to their `lib:name` id and without
//! `ki_fp_filters` (D3: KiCad 10's parity check asks for that field on a
//! footprint, where no property of that name satisfies it).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use sha2::{Digest, Sha256};

use crate::sexpr::{self, Sx};

/// Every file's path in `aethelgard-kicad-lib`'s `FILES` (KiCad's, under its
/// `data/kicad-10.0.7/`, and our own, under `data/aethelgard/`), and the
/// sha256 its bytes must have. The pins live
/// here, in the consumer: a data crate that pinned its own files would be
/// checking itself (AET-WO-1304).
const PINS: [(&str, &str); 60] = [
    (
        "footprints/Capacitor_SMD.pretty/CP_Elec_8x10.kicad_mod",
        "03187d2ed4dca904a6c0379cff9fe3a9fd0e1139b59fb9669db42160ca40cc3f",
    ),
    (
        "footprints/Capacitor_SMD.pretty/C_0402_1005Metric.kicad_mod",
        "0403382fc4583ed510b461b1fa4a36dfaec6f4c0d9b1a67e6b0027837a54e1b5",
    ),
    (
        "footprints/Capacitor_SMD.pretty/C_0603_1608Metric.kicad_mod",
        "fe0dbfefbb181a0466f93a8de52d84ba7b00fcd9acdbb69575f4128a0af4e405",
    ),
    (
        "footprints/Capacitor_SMD.pretty/C_1210_3225Metric.kicad_mod",
        "e1dae516e079b70c4736d001fb81bfb4ef5f9c2a37c8175223a44f83a04ff75f",
    ),
    (
        "footprints/Connector_AMASS.pretty/AMASS_XT30PW-M_1x02_P2.50mm_Horizontal.kicad_mod",
        "07c4d13ce29915abf8fa0a76cc167ada9eeb5515e5e00edf1b18d3dcbcb6bbfd",
    ),
    (
        "footprints/Crystal.pretty/Crystal_SMD_3225-4Pin_3.2x2.5mm.kicad_mod",
        "7648bca15399a8bd7856b5da6a1d2bc3a87049dc63958f8e8a3ab7409941a8e4",
    ),
    (
        "footprints/Inductor_SMD.pretty/L_Vishay_IHLP-5050.kicad_mod",
        "2da846ed03a3bdcae6ce3e8ef3c7d8e670947704310d2df2e7f19e5927de92d8",
    ),
    (
        "footprints/Package_LGA.pretty/Bosch_LGA-8_2.5x2.5mm_P0.65mm_ClockwisePinNumbering.kicad_mod",
        "14464b7a437e7efb0fa9947b89a7d3825242f3ed82d649fe2ca45fad5d3dda86",
    ),
    (
        "footprints/Package_QFP.pretty/LQFP-144_20x20mm_P0.5mm.kicad_mod",
        "c56afb80b84306b4ae3639fdd270987fc47b46ee531bbae036cc2ee25f2cf165",
    ),
    (
        "footprints/Package_SO.pretty/HTSSOP-16-1EP_4.4x5mm_P0.65mm_EP3.4x5mm_Mask2.46x2.31mm.kicad_mod",
        "f8b340db7496c435d9ae3726b9a80f37546b1278a4a8079abd0a6327104d4053",
    ),
    (
        "footprints/Package_SO.pretty/TSSOP-14_4.4x5mm_P0.65mm.kicad_mod",
        "e4bf82cea6d3b3259ddf42469b1407628ad1181a28ae6f3f340190c0cca31ccd",
    ),
    (
        "footprints/Package_SO.pretty/Texas_HSOP-8-1EP_3.9x4.9mm_P1.27mm.kicad_mod",
        "a2de8e28dd9f6a17dc2592de4c8500ac34f85d12e9e90e9c6499dc9de4937d91",
    ),
    (
        "footprints/Package_SON.pretty/WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm.kicad_mod",
        "5b7efafe23ba0a80faf2aa5786e7ceed526eb3802b254bc28577f1fc06abb633",
    ),
    (
        "footprints/Package_SO.pretty/SOIC-8_5.3x5.3mm_P1.27mm.kicad_mod",
        "cc86a7083c9df1c1bf808dfa469e913d446455fd7ab7a7941e18681b80e8b9b5",
    ),
    (
        "footprints/Package_TO_SOT_SMD.pretty/SOT-23-5.kicad_mod",
        "455a3f7c3e5eb5b8847eaa5df23e18651e78ab9f75afd09a0883758a0d901761",
    ),
    (
        "footprints/Resistor_SMD.pretty/R_0402_1005Metric.kicad_mod",
        "e05c7605248c220836f642ed1f526133edf0374acdfd5bb2631e58b4e377acfb",
    ),
    (
        "footprints/Connector_JST.pretty/JST_GH_SM04B-GHS-TB_1x04-1MP_P1.25mm_Horizontal.kicad_mod",
        "d28e139f60db2bef02272b37d5075e87f3e804aef0b442d0fd81a3f7cfe07ebe",
    ),
    (
        "footprints/Connector_JST.pretty/JST_GH_SM06B-GHS-TB_1x06-1MP_P1.25mm_Horizontal.kicad_mod",
        "d1855caf77704056d0af0775660442c74af40b18ae3e40bd244db5997fdc072f",
    ),
    (
        "footprints/Connector_JST.pretty/JST_GH_SM10B-GHS-TB_1x10-1MP_P1.25mm_Horizontal.kicad_mod",
        "5767bc29104c4797f67b898b1db76f1e1afb5ac65f432afaabda2c56769c7da7",
    ),
    (
        "footprints/Connector_JST.pretty/JST_SH_SM06B-SRSS-TB_1x06-1MP_P1.00mm_Horizontal.kicad_mod",
        "090e9bd7350aed224a938b922cc7d49020aa5235cc7e647a82cc3c08f37e881e",
    ),
    (
        "footprints/Resistor_SMD.pretty/R_0603_1608Metric.kicad_mod",
        "7190ac4a00125b807e54129ef0d87d87f2a658eeb74d025a7028203419b09f23",
    ),
    (
        "footprints/Sensor_Motion.pretty/InvenSense_QFN-24_3x3mm_P0.4mm.kicad_mod",
        "f8bb811410e1c549caa7667ad203ffc67fed319ae1957a41922c297ffb7cdffa",
    ),
    (
        "symbols/Connector_Generic.kicad_symdir/Conn_01x02.kicad_sym",
        "0fa71012f16ff9329d578174b763d6aee9b64f3b5112db4416a0cf5df7491fdc",
    ),
    (
        "symbols/Connector_Generic_MountingPin.kicad_symdir/Conn_01x04_MountingPin.kicad_sym",
        "370c2845e6ad1e61ea1b6b5c6a0784dea59f4aedc6d16eedda5ea3108c5105ac",
    ),
    (
        "symbols/Connector_Generic_MountingPin.kicad_symdir/Conn_01x06_MountingPin.kicad_sym",
        "43b18535b89d110da0b5750f4aeff7ed5c1ccb7def712d79451b292f55ce3c53",
    ),
    (
        "symbols/Connector_Generic_MountingPin.kicad_symdir/Conn_01x10_MountingPin.kicad_sym",
        "29f8c6d312d1a9a13225066d864c58b4e1e08d001360fb5bcc196a08be14caba",
    ),
    (
        "symbols/Device.kicad_symdir/C.kicad_sym",
        "533df5a016c224a83701e2e66b767864f39fc44c972a2a4f821987f1fd99573e",
    ),
    (
        "symbols/Device.kicad_symdir/C_Polarized.kicad_sym",
        "e0371fff515df9a22bc5e6d82a2164943da1d389e2ccf0d220ff03d7d77cc87a",
    ),
    (
        "symbols/Device.kicad_symdir/Crystal_GND24.kicad_sym",
        "84b419f45f564b391ab81579eb3c84f301ffad816f7488f98a13129a16073b3b",
    ),
    (
        "symbols/Device.kicad_symdir/L.kicad_sym",
        "ea666fa611b3d896b47a0d12a8f2975511f696384535d3cc85bd039fb9259042",
    ),
    (
        "symbols/Device.kicad_symdir/R.kicad_sym",
        "db1dd3eb60dbcc9e239d107e2d123ee253aefdd841186076e9df39444d4e11c7",
    ),
    (
        "symbols/Logic_LevelTranslator.kicad_symdir/TXU0304PW.kicad_sym",
        "d3be2fad2f944f89603c948bb9d1ea8014e874da669406029c11e878411f6a7f",
    ),
    (
        "symbols/MCU_ST_STM32H7.kicad_symdir/STM32H753ZITx.kicad_sym",
        "257de01b80c04c8157fd1b617b5d045d9c8a4cf516ad9e1e23156f2ebe716e6b",
    ),
    (
        "symbols/Memory_Flash.kicad_symdir/W25Q32JVSS.kicad_sym",
        "49133cae4075c838798a68067881f10d0836a3316f7de485f334d92d71569550",
    ),
    (
        "symbols/RF_Module.kicad_symdir/ESP32-C3-WROOM-02.kicad_sym",
        "2508dfe1fc72b61b3957d854397bb4d8afe4da28a5bd2a1c1c30acaf53718460",
    ),
    (
        "symbols/Regulator_Linear.kicad_symdir/TLV70012_SOT23-5.kicad_sym",
        "f2d21a5bfe88c6651ffc16609c9184099c9063c47c05dcf89a78a2418ff64bda",
    ),
    (
        "symbols/Regulator_Linear.kicad_symdir/TLV75518PDBV.kicad_sym",
        "9a211c51a0f05e322fbeaad08cfaa300bc94c667266076d55da8982760bd3bef",
    ),
    (
        "symbols/Regulator_Switching.kicad_symdir/LMR33630ADDA.kicad_sym",
        "e4c9f73ab9ca38fcf757672e936c8a4f27bba0ec196086b1bf6e0144339e3795",
    ),
    (
        "symbols/Regulator_Switching.kicad_symdir/LMR33640ADDA.kicad_sym",
        "888e4dedeb4d2c14d3fad15bc15d911bd8c6289ee25621ec372e46a41b6510b4",
    ),
    (
        "symbols/Sensor_Motion.kicad_symdir/ICM-20948.kicad_sym",
        "6a92444c8a1137c38a8b9e080edb82872deea8f3f9f5d324e9c487d2ad6862d4",
    ),
    (
        "symbols/power.kicad_symdir/+3V3.kicad_sym",
        "7a07c7939a39de227db5a8ed79e8be88d1b1e60b3d9b5dd8afc2f4be41ad2d57",
    ),
    (
        "symbols/power.kicad_symdir/GND.kicad_sym",
        "2505811766064acf321cd3aae4e72ef96d35da64c1ed4ca8d6a24271c9cf7734",
    ),
    (
        "symbols/power.kicad_symdir/PWR_FLAG.kicad_sym",
        "5308c8c77530d5ba458ca658925119069cb6cb8ad6fbf424d88afb1fdb4e3ef1",
    ),
    (
        "symbols/Sensor.kicad_symdir/BME280.kicad_sym",
        "d194dfcae6dc11d852b349df938c2b95f08ef803bababdf2792ee2bec11cb55a",
    ),
    (
        "symbols/Sensor_Temperature.kicad_symdir/TMP102xxDRL.kicad_sym",
        "ba51c59e932af56e44ae300fa8712a53ae1396d3af3f9ef7c609e665037b453f",
    ),
    (
        "symbols/Sensor_Temperature.kicad_symdir/TMP117xxDRV.kicad_sym",
        "1d1a1427556c4e89fc7d30fcbd418120ebbe9c0b774cabda08843ac5f6742848",
    ),
    // Our own (`AET-WO-4220b`, MIT OR Apache-2.0): `data/aethelgard/`.
    (
        "footprints/Aethelgard_Driver.pretty/TI_HTSSOP-16-1EP_4.4x5mm_P0.65mm_EP3.4x5mm_Mask3.3x3.3mm.kicad_mod",
        "58fc3611cf81a68c368c884a0df4ee9923cd01d7376e418b94e6a045175a4336",
    ),
    // Derived from KiCad's (`AET-WO-4411` D4), so CC-BY-SA-4.0, not MIT OR Apache-2.0.
    (
        "footprints/Aethelgard_RF.pretty/ESP32-C3-WROOM-02.kicad_mod",
        "11f61c326e92d6f4abf8af609faca83f4837c4b5c6b6603a26020e337db99c7c",
    ),
    (
        "footprints/Aethelgard_Sensor.pretty/Bosch_LGA-10_2x2mm_P0.5mm.kicad_mod",
        "5ee1738db4ee82b091bd24536bf2e74d8bfc6b4bc16b31cf5d3bcefcc05c2608",
    ),
    (
        "footprints/Aethelgard_Sensor.pretty/TDK_LGA-14_3x2.5mm_P0.5mm.kicad_mod",
        "56a4ce03c78f1409732c4d3783174b9865882608d6d0e1e4d218e15dc250ba82",
    ),
    (
        "symbols/Aethelgard_Driver.kicad_symdir/DRV8833PWP.kicad_sym",
        "510be06771e3e06104817bba6966dce2e810a9f4c0d39b7e989d226d4c4c8f12",
    ),
    (
        "symbols/Aethelgard_Driver.kicad_symdir/DRV8876PWP.kicad_sym",
        "d0588348a5307e3130d908f7327a4752a21d97f2a3ab972484f7b471c3175338",
    ),
    (
        "symbols/Aethelgard_Sensor.kicad_symdir/BMP581.kicad_sym",
        "1d85734054b68821e489173e838f6aeb31afe42c186b15ef0e93eb27781f666b",
    ),
    (
        "symbols/Aethelgard_Sensor.kicad_symdir/ICM-42688-P.kicad_sym",
        "782551acb98cd228716af64e10aeac3ed0dcf65aba4130af92179062011c127a",
    ),
    (
        "aethelgard/README.md",
        "11710f4d4ea07e9eee683d47e495df45684389a24f86d418ee6db6c114bb9d88",
    ),
    (
        "footprints/Button_Switch_SMD.pretty/SW_SPST_PTS810.kicad_mod",
        "0c23d0140164aadb05dfa4a8e393e4311a75aa7c543552af4d0959f114bb6ed6",
    ),
    (
        "footprints/TestPoint.pretty/TestPoint_Pad_D1.0mm.kicad_mod",
        "6d602c576b4ab0de29fa4cc096f419a62e828ba45ceee30c1263d86b420b2b12",
    ),
    (
        "symbols/Connector.kicad_symdir/TestPoint.kicad_sym",
        "e983b0d2c450bc8c517b5bd83873bbe060f9e11e71dbb07951765a1b54f39a8e",
    ),
    (
        "symbols/Switch.kicad_symdir/SW_Push.kicad_sym",
        "650beb4ee58eda52928926b1eb0ad8b11dedaacc9ad1334bb4d3a920c38c9814",
    ),
    (
        "LICENSE.md",
        "45d2bce75e5a4208f5afb01b8fb2c406e700371c4fe2b5f5cd5c443d46db4d8f",
    ),
];

/// A pinned file's path, sha256 and the bytes the data crate holds for it.
type Vendored = (&'static str, &'static str, &'static [u8]);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LibraryError {
    #[error("vendored `{path}` has sha256 {actual}, not {expected}: a corrupted build")]
    HashMismatch {
        path: &'static str,
        expected: &'static str,
        actual: String,
    },
    #[error("pinned `{path}` is absent from aethelgard-kicad-lib: a corrupted build")]
    MissingFile { path: &'static str },
    #[error("aethelgard-kicad-lib holds `{path}`, which is not pinned: a corrupted build")]
    UnpinnedFile { path: &'static str },
    #[error("vendored `{0}` is not a KiCad S-expression: {1}")]
    Unparseable(&'static str, sexpr::SexprError),
    #[error("no vendored symbol `{0}`")]
    NoSymbol(String),
    #[error("no vendored footprint `{0}`")]
    NoFootprint(String),
    #[error("symbol `{0}` extends past a depth of 4")]
    ExtendsTooDeep(String),
}

/// The pinned files with their bytes, or the first refusal: a pinned path the
/// data crate lacks, a file it holds that is not pinned, or a hash mismatch.
/// Nothing is served unless every file passes.
fn verify(
    pins: &[(&'static str, &'static str)],
    files: &[(&'static str, &'static [u8])],
) -> Result<Vec<Vendored>, LibraryError> {
    let mut out = Vec::with_capacity(pins.len());
    for &(path, expected) in pins {
        let &(_, bytes) = files
            .iter()
            .find(|(p, _)| *p == path)
            .ok_or(LibraryError::MissingFile { path })?;
        let actual: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if actual != expected {
            return Err(LibraryError::HashMismatch {
                path,
                expected,
                actual,
            });
        }
        out.push((path, expected, bytes));
    }
    if let Some(&(path, _)) = files
        .iter()
        .find(|(p, _)| !pins.iter().any(|(q, _)| q == p))
    {
        return Err(LibraryError::UnpinnedFile { path });
    }
    Ok(out)
}

/// The checked files, or the first refusal; checked once.
fn checked() -> Result<&'static [Vendored], LibraryError> {
    static CHECK: OnceLock<Result<Vec<Vendored>, LibraryError>> = OnceLock::new();
    CHECK
        .get_or_init(|| verify(&PINS, &aethelgard_kicad_lib::FILES))
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

fn file(path: &str) -> Result<Option<(&'static str, &'static [u8])>, LibraryError> {
    Ok(checked()?
        .iter()
        .find(|(p, _, _)| *p == path)
        .map(|(p, _, b)| (*p, *b)))
}

/// One symbol file's top-level `(symbol "name" …)`.
fn raw_symbol(lib: &str, name: &str) -> Result<Sx, LibraryError> {
    let path = format!("symbols/{lib}.kicad_symdir/{name}.kicad_sym");
    let (path, bytes) =
        file(&path)?.ok_or_else(|| LibraryError::NoSymbol(format!("{lib}:{name}")))?;
    let text = std::str::from_utf8(bytes)
        .map_err(|_| LibraryError::Unparseable(path, sexpr::SexprError::NotUtf8(0)))?;
    let lib_file = sexpr::parse(text).map_err(|e| LibraryError::Unparseable(path, e))?;
    lib_file
        .children("symbol")
        .find(|s| s.arg(1) == Some(name))
        .cloned()
        .ok_or_else(|| LibraryError::NoSymbol(format!("{lib}:{name}")))
}

/// The flags a derived symbol may restate; its own value wins.
const FLAGS: [&str; 8] = [
    "power",
    "pin_names",
    "pin_numbers",
    "exclude_from_sim",
    "in_bom",
    "on_board",
    "embedded_fonts",
    "duplicate_pin_numbers_are_jumpers",
];

fn flatten(lib: &str, name: &str, depth: usize) -> Result<Sx, LibraryError> {
    if depth > 4 {
        return Err(LibraryError::ExtendsTooDeep(format!("{lib}:{name}")));
    }
    let sym = raw_symbol(lib, name)?;
    let Some(parent_name) = sym
        .child("extends")
        .and_then(|e| e.arg(1))
        .map(str::to_owned)
    else {
        return Ok(sym);
    };
    let parent = flatten(lib, &parent_name, depth + 1)?;
    let own: Vec<&Sx> = sym.items()[2..]
        .iter()
        .filter(|i| i.head() != Some("extends"))
        .collect();
    let overridden = |item: &Sx| {
        own.iter().any(|o| {
            o.head() == item.head() && (item.head() != Some("property") || o.arg(1) == item.arg(1))
        })
    };
    let mut out = vec![Sx::atom("symbol"), Sx::str(name)];
    out.extend(own.iter().map(|o| (*o).clone()));
    for item in &parent.items()[2..] {
        let head = item.head().unwrap_or("");
        if (head == "property" || FLAGS.contains(&head)) && overridden(item) {
            continue;
        }
        let mut item = item.clone();
        if head == "symbol"
            && let Some(rest) = item
                .arg(1)
                .and_then(|s| s.strip_prefix(parent_name.as_str()))
                .map(str::to_owned)
            && let Some(items) = item.items_mut()
        {
            items[1] = Sx::str(format!("{name}{rest}"));
        }
        out.push(item);
    }
    Ok(Sx::List(out))
}

/// Whether a symbol's `(property …)` is a private one: a library note
/// (`(property private "KLC_S3.3" …)` in `Device:Crystal_GND24`) that stays
/// in the library symbol and is drawn on no instance or footprint
/// (`AET-WO-4240c`; checked by ERC and the DRC gate's schematic parity).
pub fn private(property: &Sx) -> bool {
    matches!(property.items().get(1), Some(Sx::Atom(a)) if a == "private")
}

/// `lib:name`, flattened and renamed as a schematic's `lib_symbols` embeds
/// it, without `ki_fp_filters`.
pub fn symbol(lib_id: &str) -> Result<Sx, LibraryError> {
    let (lib, name) = lib_id
        .split_once(':')
        .ok_or_else(|| LibraryError::NoSymbol(lib_id.to_owned()))?;
    let mut sym = flatten(lib, name, 0)?;
    if let Some(items) = sym.items_mut() {
        items[1] = Sx::str(lib_id);
        items.retain(|i| !(i.head() == Some("property") && i.arg(1) == Some("ki_fp_filters")));
    }
    Ok(sym)
}

/// A footprint, `lib:name`, as its `.kicad_mod` holds it.
pub fn footprint(lib_id: &str) -> Result<Sx, LibraryError> {
    let (lib, name) = lib_id
        .split_once(':')
        .ok_or_else(|| LibraryError::NoFootprint(lib_id.to_owned()))?;
    let path = format!("footprints/{lib}.pretty/{name}.kicad_mod");
    let (path, bytes) = file(&path)?.ok_or_else(|| LibraryError::NoFootprint(lib_id.to_owned()))?;
    let text = std::str::from_utf8(bytes)
        .map_err(|_| LibraryError::Unparseable(path, sexpr::SexprError::NotUtf8(0)))?;
    sexpr::parse(text).map_err(|e| LibraryError::Unparseable(path, e))
}

/// Every vendored file the gate writes into its scratch directory, by its
/// path relative to a `lib/` directory there, the licence included.
pub fn files() -> Result<Vec<(&'static str, &'static [u8])>, LibraryError> {
    Ok(checked()?.iter().map(|(p, _, b)| (*p, *b)).collect())
}

/// `(sym-lib-table, fp-lib-table)`, project-local, naming every vendored
/// library under `lib_dir` (an absolute path the gate chose). The gate
/// writes both beside the copy it judges (ruled D3).
pub fn tables(lib_dir: &str) -> Result<(String, String), LibraryError> {
    let mut sym = BTreeMap::new();
    let mut fp = BTreeMap::new();
    for (path, _, _) in checked()? {
        let mut parts = path.split('/');
        match (parts.next(), parts.next()) {
            (Some("symbols"), Some(dir)) => {
                if let Some(lib) = dir.strip_suffix(".kicad_symdir") {
                    sym.insert(lib, format!("{lib_dir}/symbols/{dir}"));
                }
            }
            (Some("footprints"), Some(dir)) => {
                if let Some(lib) = dir.strip_suffix(".pretty") {
                    fp.insert(lib, format!("{lib_dir}/footprints/{dir}"));
                }
            }
            _ => {}
        }
    }
    let table = |kind: &str, libs: &BTreeMap<&str, String>| {
        let mut out = format!("({kind}\n\t(version 7)\n");
        for (name, uri) in libs {
            out.push_str(&format!(
                "\t(lib (name {}) (type \"KiCad\") (uri {}) (options \"\") (descr \"\"))\n",
                sexpr::quote(name),
                sexpr::quote(uri)
            ));
        }
        out.push_str(")\n");
        out
    };
    Ok((table("sym_lib_table", &sym), table("fp_lib_table", &fp)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vendored_file_matches_its_hash() {
        assert_eq!(files().unwrap().len(), PINS.len());
        assert_eq!(PINS.len(), aethelgard_kicad_lib::FILES.len());
    }

    #[test]
    fn a_file_that_does_not_match_its_hash_is_refused() {
        let (path, expected) = PINS[0];
        let tampered = [(path, &b"(footprint)"[..])];
        assert!(matches!(
            verify(&PINS[..1], &tampered),
            Err(LibraryError::HashMismatch { path: p, expected: e, .. }) if p == path && e == expected
        ));
    }

    #[test]
    fn a_pinned_file_the_data_crate_lacks_is_refused() {
        let held = &aethelgard_kicad_lib::FILES[1..];
        assert!(matches!(
            verify(&PINS, held),
            Err(LibraryError::MissingFile { path }) if path == PINS[0].0
        ));
    }

    #[test]
    fn a_file_the_data_crate_holds_unpinned_is_refused() {
        let mut held = aethelgard_kicad_lib::FILES.to_vec();
        held.push(("footprints/Extra.pretty/X.kicad_mod", &b"(footprint)"[..]));
        assert!(matches!(
            verify(&PINS, &held),
            Err(LibraryError::UnpinnedFile { path }) if path == "footprints/Extra.pretty/X.kicad_mod"
        ));
    }

    #[test]
    fn a_derived_symbol_is_flattened_and_keeps_its_own_fields() {
        let tmp117 = symbol("Sensor_Temperature:TMP117xxDRV").unwrap();
        assert_eq!(tmp117.arg(1), Some("Sensor_Temperature:TMP117xxDRV"));
        assert!(tmp117.child("extends").is_none());
        assert_eq!(
            tmp117.property("Footprint"),
            Some("Package_SON:WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm")
        );
        assert_eq!(tmp117.property("Value"), Some("TMP117xxDRV"));
        let subs: Vec<&str> = tmp117.children("symbol").filter_map(|s| s.arg(1)).collect();
        assert!(
            subs.iter().all(|s| s.starts_with("TMP117xxDRV_")),
            "{subs:?}"
        );
        assert_eq!(crate::schematic::pins(&tmp117).unwrap().len(), 6);
    }

    /// The power tree's symbols (AET-WO-4300d2) resolve, with `extends`
    /// flattened, and carry the pin numbers the datasheet pinouts name.
    #[test]
    fn the_power_symbols_resolve_with_their_pin_numbers() {
        for (id, numbers) in [
            (
                "Regulator_Switching:LMR33630ADDA",
                &["1", "2", "3", "4", "5", "6", "7", "8", "9"][..],
            ),
            ("Regulator_Linear:TLV75518PDBV", &["1", "2", "3", "4", "5"]),
            (
                "Logic_LevelTranslator:TXU0304PW",
                &[
                    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14",
                ],
            ),
            (
                "Sensor_Motion:ICM-20948",
                &[
                    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14",
                    "15", "16", "17", "18", "19", "20", "21", "22", "23", "24",
                ],
            ),
            ("Device:L", &["1", "2"]),
            ("Connector_Generic:Conn_01x02", &["1", "2"]),
        ] {
            let sym = symbol(id).unwrap();
            assert!(sym.child("extends").is_none(), "{id}");
            let mut got: Vec<String> = crate::schematic::pins(&sym)
                .unwrap()
                .into_iter()
                .map(|p| p.number)
                .collect();
            got.sort_by_key(|n| n.parse::<u32>().unwrap_or(u32::MAX));
            assert_eq!(got, numbers, "{id}");
        }
    }

    #[test]
    fn ki_fp_filters_is_dropped_from_every_symbol() {
        for id in [
            "Device:R",
            "Device:C",
            "Sensor:BME280",
            "Sensor_Temperature:TMP117xxDRV",
            "Memory_Flash:W25Q32JVSS",
            "MCU_ST_STM32H7:STM32H753ZITx",
        ] {
            let s = symbol(id).unwrap();
            assert_eq!(s.property("ki_fp_filters"), None, "{id}");
            assert!(s.property("Reference").is_some(), "{id}");
        }
    }

    #[test]
    fn footprints_parse_and_carry_no_local_clearance() {
        for &(path, _, bytes) in checked().unwrap() {
            if path.starts_with("footprints/") {
                let text = std::str::from_utf8(bytes).unwrap();
                let fp = sexpr::parse(text).unwrap();
                assert_eq!(fp.head(), Some("footprint"));
                // A clearance only inside a keepout zone's `connect_pads`
                // (`RUL-03`, amended for `AET-WO-4300d2` D2 (a)).
                let keepout_clearances: usize = fp
                    .children("zone")
                    .filter(|z| z.child("keepout").is_some())
                    .filter_map(|z| z.child("connect_pads"))
                    .map(|c| c.children("clearance").count())
                    .sum();
                assert_eq!(
                    text.matches("(clearance").count(),
                    keepout_clearances,
                    "{path}"
                );
            }
        }
    }

    #[test]
    fn the_tables_name_every_vendored_library() {
        let (sym, fp) = tables("/s/lib").unwrap();
        assert!(sym.contains(
            "(lib (name \"Device\") (type \"KiCad\") (uri \"/s/lib/symbols/Device.kicad_symdir\")"
        ));
        assert!(fp.contains("(uri \"/s/lib/footprints/Package_QFP.pretty\")"));
        // Our own library (`AET-WO-4220b`) beside KiCad's, by one name.
        assert!(sym.contains(
            "(lib (name \"Aethelgard_Sensor\") (type \"KiCad\") (uri \"/s/lib/symbols/Aethelgard_Sensor.kicad_symdir\")"
        ));
        assert!(fp.contains("(uri \"/s/lib/footprints/Aethelgard_Sensor.pretty\")"));
        // `AET-WO-4411`: the radio module's symbol library and our derived footprint library.
        assert!(sym.contains("(uri \"/s/lib/symbols/RF_Module.kicad_symdir\")"));
        assert!(fp.contains("(uri \"/s/lib/footprints/Aethelgard_RF.pretty\")"));
        assert_eq!(sym.matches("(lib ").count(), 17);
        // `AET-WO-4240c`: the crystal's footprint library.
        assert!(fp.contains("(uri \"/s/lib/footprints/Crystal.pretty\")"));
        assert_eq!(fp.matches("(lib ").count(), 17);
        assert!(sexpr::parse(&sym).is_ok() && sexpr::parse(&fp).is_ok());
    }
}
