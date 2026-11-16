//! The witness ledger (`AET-WO-4230` §1.7, `RUL-20`, contract D9): one row
//! per register field the generated firmware writes or leaves at reset, each
//! classed by what a pass of it is evidence of. The ledger covers register
//! fields only (the architect's ruling on Q-I): what the twin cannot judge
//! that no acknowledgement can name (the scripts' start-up wait, each
//! register a model answers 0x00 with a warning) is [`twin_limits`], a list
//! of its own that never blocks a package.
//!
//! **Acknowledgement (`AET-WO-4230` R1).** Every row of an unwitnessed class
//! (`same-source`, `ignored-by-model`, `silicon-only`) is `acknowledged` or
//! not by the specification's `acknowledge_unwitnessed` entry for its
//! register and class ([`acknowledge`]); `verify --fab-out` refuses a row
//! that is not, and `validate` refuses an entry naming a field the ledger
//! lacks ([`stale`]).
//!
//! The fields come from the generator's own list
//! ([`aethelgard_svd_hal::board::register_fields`]); the classes come from
//! [`TABLE`] here, keyed by (family, register, field, access). A field the
//! table does not know is a hard error ([`LedgerError::Unclassified`]): the
//! ledger never guesses a class. Every `observed` and `ignored-by-model`
//! entry cites the measurement or the Renode source line that shows it
//! ([`cited`]); nothing is `observed` without one.
//!
//! **`independent-rule`, one criterion for every family:** a rule in
//! `aethelgard-ir` that shares no code with the generator, by a different
//! formulation, holds the value; its failure refuses generation, solving or
//! the verdict; and the Kani oracle's expected word is the rule's own
//! (`spi_words`, `i2c_cr1`) or the topology's word that rule held (`TIMINGR`,
//! `BRR`, a timer's `PSC`/`ARR`/`CCRx`). Each row's `note` says which.
//!
//! Where two classes apply, the stronger wins (`independent-rule` >
//! `observed` > the rest) and the note names the other.
//!
//! The Renode sources cited by line are `renode-infrastructure` `066a7f13`,
//! the submodule of Renode `v1.17.0` (`AET-WO-4230` FEEDBACK §2); for the
//! models whose source this work order did not hold (GPIO, I2C, USART, timer)
//! the evidence is the committed FEEDBACK passage that read or measured it.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::LazyLock;

use aethelgard_ir::silicon::{self, BrrFinding, I2cFinding, SpiFinding};
use aethelgard_ir::{
    PeerDeviationSource, PinTable, SolvedTopology, Specification, UnwitnessedClass,
};
use aethelgard_svd_hal::board::{FieldAccess, FieldUse, register_fields};
use aethelgard_svd_hal::cmsis;
use serde::{Serialize, Serializer};

/// What a pass of a row is evidence of (`RUL-20`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    IndependentRule,
    Observed,
    SameSource,
    IgnoredByModel,
    SiliconOnly,
}

impl Class {
    pub const ALL: [Class; 5] = [
        Class::IndependentRule,
        Class::Observed,
        Class::SameSource,
        Class::IgnoredByModel,
        Class::SiliconOnly,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Class::IndependentRule => "independent-rule",
            Class::Observed => "observed",
            Class::SameSource => "same-source",
            Class::IgnoredByModel => "ignored-by-model",
            Class::SiliconOnly => "silicon-only",
        }
    }

    /// What "pass" is evidence of for a row of this class (`RUL-20`).
    pub fn evidence(self) -> &'static str {
        match self {
            Class::IndependentRule => {
                "judged (RUL-20 (1)): a rule in aethelgard-ir that shares no code with the generator holds the value, and the proof's expected word is that rule's"
            }
            Class::Observed => {
                "judged (RUL-20 (2)): the twin's model reacted to the field, as the cited measurement or Renode source line shows"
            }
            Class::SameSource => {
                "consistency, not judgement: only checks generated from the same source as the code check it, and the model does not react to it; only the bench confirms it"
            }
            Class::IgnoredByModel => {
                "consistency, not judgement: the twin's model stores or acts on none of it (cited); only the bench confirms it"
            }
            Class::SiliconOnly => {
                "nothing: no model exists for it; only the bench measures it (RUL-20 (3))"
            }
        }
    }

    /// Whether a pass of it is judgement (`RUL-20` (1) or (2)).
    pub fn judged(self) -> bool {
        matches!(self, Class::IndependentRule | Class::Observed)
    }

    /// The class an acknowledgement names for a row of this class
    /// (`AET-WO-4230` R1); `None` for a judged class, which none needs.
    pub fn unwitnessed(self) -> Option<UnwitnessedClass> {
        match self {
            Class::IndependentRule | Class::Observed => None,
            Class::SameSource => Some(UnwitnessedClass::SameSource),
            Class::IgnoredByModel => Some(UnwitnessedClass::IgnoredByModel),
            Class::SiliconOnly => Some(UnwitnessedClass::SiliconOnly),
        }
    }
}

impl Serialize for Class {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

/// One row of the ledger (contract D9). `value` is `null` where the field's
/// last write is decided at run time (a data byte, an address, a count):
/// D9's `u32` cannot say that, so this is the one shape change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LedgerRow {
    /// `SPI1.CFG1.MBR`; a reset field ends `@reset` (`RCC.D2CCIP1R.SPI123SEL`
    /// is both written and read at reset); a non-register row is
    /// `FAMILY.reference.what`.
    pub id: String,
    pub family: String,
    /// `SPI1.CFG1`; empty for a non-register row.
    pub register: String,
    pub field: String,
    pub pos: u8,
    pub width: u8,
    /// `written`, `reset`, or `none` for a non-register row.
    pub access: &'static str,
    pub value: Option<u32>,
    pub class: Class,
    pub judge: String,
    pub note: String,
    /// For a row of an unwitnessed class ([`Class::unwitnessed`]), whether
    /// the specification acknowledges it (`AET-WO-4230` R1); absent for a
    /// judged row and for a twin limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acknowledged: Option<bool>,
    /// The acknowledging entry's reason, beside `acknowledged: true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The register's bus address, for the bench sheet; `None` for a
    /// non-register row.
    #[serde(skip)]
    pub address: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Counts {
    pub by_class: BTreeMap<&'static str, usize>,
    pub by_family: BTreeMap<String, usize>,
    /// The rows of an unwitnessed class no acknowledgement names
    /// ([`acknowledge`]); every such row before it runs.
    pub unacknowledged: usize,
}

/// The ledger as `verify`'s verdict carries it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Ledger {
    pub rows: Vec<LedgerRow>,
    pub counts: Counts,
    pub total: usize,
    /// For each class, what "pass" is evidence of.
    pub evidence: BTreeMap<&'static str, &'static str>,
}

/// Why no ledger could be built; `verify` refuses with it before any gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerError {
    /// The generator's field list, or the register facts, failed.
    Fields(String),
    /// A field the classification table does not know.
    Unclassified { id: String },
    /// A table entry that claims `observed` or `ignored-by-model` without a
    /// citation, or any entry without a judge.
    Uncited { id: String },
    /// Two rows with one id.
    Duplicate { id: String },
    /// A row an independent rule would class whose value is not the rule's:
    /// a motor-output pin's `PUPDR` field that is not
    /// [`silicon::motor_output_pupd`] (`AET-WO-4230` R5).
    RuleDisagrees {
        id: String,
        value: Option<u32>,
        rule: u32,
    },
}

impl LedgerError {
    pub fn kind(&self) -> &'static str {
        match self {
            LedgerError::Fields(_) => "LedgerFieldsUnavailable",
            LedgerError::Unclassified { .. } => "LedgerFieldUnclassified",
            LedgerError::Uncited { .. } => "LedgerClassUncited",
            LedgerError::Duplicate { .. } => "LedgerRowDuplicate",
            LedgerError::RuleDisagrees { .. } => "LedgerRuleDisagrees",
        }
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            LedgerError::Fields(_) => None,
            LedgerError::Unclassified { id }
            | LedgerError::Uncited { id }
            | LedgerError::Duplicate { id }
            | LedgerError::RuleDisagrees { id, .. } => Some(id),
        }
    }

    pub fn message(&self) -> String {
        match self {
            LedgerError::Fields(m) => format!("the ledger's field list is unavailable: {m}"),
            LedgerError::Unclassified { id } => format!(
                "the generated firmware touches `{id}`, which the witness ledger's classification table does not know: no class is guessed (AET-WO-4230 §1.7, §5.4)"
            ),
            LedgerError::Uncited { id } => format!(
                "the classification entry for `{id}` claims a model's behaviour without citing a measurement or a Renode source line, or names no judge"
            ),
            LedgerError::Duplicate { id } => format!("the ledger has two rows `{id}`"),
            LedgerError::RuleDisagrees { id, value, rule } => format!(
                "the generated firmware leaves `{id}` at {}, not the independent rule's {rule:#b} (aethelgard_ir::silicon::motor_output_pupd): the row cannot be classed independent-rule",
                value.map_or_else(|| "a run-time value".to_owned(), |v| format!("{v:#b}"))
            ),
        }
    }
}

/// One entry of the classification table. `register` and `field` are
/// patterns in which `#` stands for one run of decimal digits (a pin index,
/// a channel, a timer number); every other character is literal.
#[derive(Debug, Clone)]
pub struct Entry {
    pub family: &'static str,
    pub register: &'static str,
    pub field: String,
    pub access: FieldAccess,
    pub class: Class,
    pub judge: String,
    /// The measurement or Renode source line a model's behaviour is cited
    /// to; required for `observed` and `ignored-by-model`.
    pub source: String,
    pub note: String,
}

/// Whether `e` may be used: every entry names its judge, and every
/// `observed` or `ignored-by-model` entry cites a FEEDBACK measurement or a
/// source line (`file.cs L<n>`).
pub fn cited(e: &Entry) -> bool {
    let cites = |s: &str| {
        s.contains("FEEDBACK")
            || s.split(".cs L")
                .skip(1)
                .any(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
    };
    !e.judge.trim().is_empty()
        && match e.class {
            Class::Observed | Class::IgnoredByModel => cites(&e.source),
            _ => true,
        }
}

/// `#` matches one or more digits; everything else literally.
fn matches(pattern: &str, name: &str) -> bool {
    let (p, n) = (pattern.as_bytes(), name.as_bytes());
    let (mut i, mut j) = (0, 0);
    while i < p.len() {
        if p[i] == b'#' {
            let start = j;
            while j < n.len() && n[j].is_ascii_digit() {
                j += 1;
            }
            if j == start {
                return false;
            }
        } else if j < n.len() && p[i] == n[j] {
            j += 1;
        } else {
            return false;
        }
        i += 1;
    }
    j == n.len()
}

/// The entry for one field, or the refusal.
pub fn classify<'t>(
    table: &'t [Entry],
    family: &str,
    register: &str,
    field: &str,
    access: FieldAccess,
    id: &str,
) -> Result<&'t Entry, LedgerError> {
    let e = table
        .iter()
        .find(|e| {
            e.family == family
                && e.access == access
                && matches(e.register, register)
                && matches(&e.field, field)
        })
        .ok_or_else(|| LedgerError::Unclassified { id: id.to_owned() })?;
    if !cited(e) {
        return Err(LedgerError::Uncited { id: id.to_owned() });
    }
    Ok(e)
}

// ------------------------------------------------------------ the table

const RCC_CS: &str = "STM32H7_RCC.cs";
const SPI_CS: &str = "STM32H7_SPI.cs";

/// The judge of a field only the generated oracle checks.
const ORACLE_ONLY: &str = "the generated oracle only";

const SPI_RULE: &str = "aethelgard_ir::silicon::spi_words, held by silicon::check_spi before generation (svd-hal check_silicon); the proof's expected CR1, CFG1 and CFG2 are the rule's words (board.rs oracle_spi)";
const SELECT_RULE: &str = "aethelgard_ir::silicon::spi_words (kernel_select): the whole select word, the solved controllers' codes and every other field at reset, held by silicon::check_spi; the proof's expected word is the rule's";
const I2C_CR1_RULE: &str = "aethelgard_ir::silicon::i2c_cr1, held by silicon::check_i2c before generation; the proof's expected CR1 is the rule's word (and the rule's word with PE for the enable)";
const TIMINGR_RULE: &str = "aethelgard_ir::silicon::check_timingr (through check_i2c), a decode and inequalities at HSI's worst case, which refuses generation when it fails; the proof's expected word is the topology's, which that rule held";
const BRR_RULE: &str = "aethelgard_ir::silicon::check_brr (AET-WO-4230 R3, the architect's ruling on Q-H): 2*|BRR*baud - kernel| <= baud and 16 <= BRR <= 0xFFFF in exact integers, sharing nothing with serial::solve_brr; generation refuses a BRR it does not hold (svd-hal check_silicon, SiliconRuleFails) and the verdict's silicon findings judge every USART again; the proof's expected BRR is the topology's, which that rule held";
const MOTOR_PUPD_RULE: &str = "aethelgard_ir::silicon::motor_output_pupd (AET-WO-4230 R5; its source is the architect's ruling on PR #161, comment 6911424608, Q-G: pull-down `10` on every motor-output pin in the first GPIO init, whole word); the proof's expected PUPDR takes this pin's field from the rule (svd-hal board.rs oracle_pupdr), and the ledger refuses a value other than the rule's (LedgerRuleDisagrees)";
const TIM_RULE: &str = "aethelgard_ir::motor::check_silicon (RUL-20 (1)'s own example: silicon's ARR + 1), a static finding that fails the twin's verdict (renode.rs static_motor_findings); the proof's expected word is the topology's, which that rule judged";

/// Each RCC enable gate the generator can write (`GPIOA`–`GPIOK`, `I2C1`–`I2C4`,
/// `SPI1`–`SPI6`, every USART/UART of `aethelgard_ir::serial::PERIPHERALS`,
/// every timer of `aethelgard_ir::motor::TIMERS`), its register, the line of
/// its flag in `STM32H7_RCC.cs`, and the measurement that shows the
/// platform registers its peripheral with the RCC, if one is held.
const GATES: [(&str, &str, u32, Option<&str>); 41] = [
    ("GPIOAEN", "AHB4ENR", 792, Some(GATE_REF)),
    ("GPIOBEN", "AHB4ENR", 795, Some(GATE_REF)),
    ("GPIOCEN", "AHB4ENR", 798, None),
    ("GPIODEN", "AHB4ENR", 801, None),
    ("GPIOEEN", "AHB4ENR", 804, None),
    ("GPIOFEN", "AHB4ENR", 807, None),
    ("GPIOGEN", "AHB4ENR", 810, None),
    ("GPIOHEN", "AHB4ENR", 813, None),
    ("GPIOIEN", "AHB4ENR", 816, None),
    ("GPIOJEN", "AHB4ENR", 819, None),
    ("GPIOKEN", "AHB4ENR", 822, None),
    ("TIM2EN", "APB1LENR", 856, None),
    ("TIM3EN", "APB1LENR", 859, None),
    ("TIM4EN", "APB1LENR", 862, None),
    ("TIM5EN", "APB1LENR", 865, None),
    ("TIM12EN", "APB1LENR", 874, None),
    ("TIM13EN", "APB1LENR", 877, None),
    ("TIM14EN", "APB1LENR", 880, None),
    ("SPI2EN", "APB1LENR", 887, Some(GATE_SPI)),
    ("SPI3EN", "APB1LENR", 890, Some(GATE_SPI)),
    ("USART2EN", "APB1LENR", 896, Some(GATE_PORTS)),
    ("USART3EN", "APB1LENR", 899, Some(GATE_PORTS)),
    ("UART4EN", "APB1LENR", 902, Some(GATE_PORTS)),
    ("UART5EN", "APB1LENR", 905, None),
    ("I2C1EN", "APB1LENR", 908, Some(GATE_REF)),
    ("I2C2EN", "APB1LENR", 911, None),
    ("I2C3EN", "APB1LENR", 914, None),
    ("UART7EN", "APB1LENR", 925, None),
    ("UART8EN", "APB1LENR", 928, None),
    ("TIM1EN", "APB2ENR", 952, Some(GATE_TIM)),
    ("TIM8EN", "APB2ENR", 955, None),
    ("USART1EN", "APB2ENR", 959, Some(GATE_REF)),
    ("USART6EN", "APB2ENR", 962, None),
    ("SPI1EN", "APB2ENR", 966, Some(GATE_SPI)),
    ("SPI4EN", "APB2ENR", 969, Some(GATE_SPI)),
    ("TIM15EN", "APB2ENR", 973, None),
    ("TIM16EN", "APB2ENR", 976, None),
    ("TIM17EN", "APB2ENR", 979, None),
    ("SPI5EN", "APB2ENR", 983, Some(GATE_SPI)),
    ("SPI6EN", "APB4ENR", 1014, Some(GATE_SPI)),
    ("I2C4EN", "APB4ENR", 1018, None),
];
const GATE_REF: &str = "AET-WO-3302 bench sheet B2's Renode column (AHB4ENR 0x3, APB1LENR 0x00200000, APB2ENR 0x1000, read back after the reference's boot; USART1EN by AET-WO-4300b FEEDBACK P2.1's image): the bit reads back only for a peripheral the platform registers";
const GATE_PORTS: &str =
    "AET-WO-4300b FEEDBACK P2.2 (four ports' images read back after the boot, measured)";
const GATE_TIM: &str = "AET-WO-4300a FEEDBACK (the motor fixtures' register image, every enable register judged in full after the boot)";
const GATE_SPI: &str = "the emitted platform registers each solved SPI it adds with `rcc RegistrationPeripheralAlias.SPIn` (aethelgard-twin platform, boards/sentinel-01/sentinel-01.repl)";

#[allow(clippy::too_many_arguments)]
fn e(
    family: &'static str,
    register: &'static str,
    field: &str,
    access: FieldAccess,
    class: Class,
    judge: &str,
    source: &str,
    note: &str,
) -> Entry {
    Entry {
        family,
        register,
        field: field.to_owned(),
        access,
        class,
        judge: judge.to_owned(),
        source: source.to_owned(),
        note: note.to_owned(),
    }
}

/// The classification table (WO §1.7). Order matters only where two
/// patterns could match one field; none here do.
pub static TABLE: LazyLock<Vec<Entry>> = LazyLock::new(table);

#[allow(clippy::too_many_lines)]
fn table() -> Vec<Entry> {
    use Class::{
        IgnoredByModel as Ign, IndependentRule as Rule, Observed as Obs, SameSource as Same,
        SiliconOnly as Si,
    };
    use FieldAccess::{Reset as R, Written as W};
    let mut t = Vec::new();

    // ---- RCC: the kernel-clock select words, written whole.
    let d2_tags = format!(
        "{RCC_CS} L403–418: every RCC_D2CCIP1R field is a tag (written value dropped, reads 0); measured, AET-WO-4230 FEEDBACK §2"
    );
    for (field, what) in [
        (
            "SPI#SEL",
            "the solved controllers' kernel code (per_ck `100`, hsi_ker_ck `011`)",
        ),
        ("SAI#SEL", "at reset inside the rule's whole word"),
        ("SPDIFSEL", "at reset inside the rule's whole word"),
        ("DFSDM#SEL", "at reset inside the rule's whole word"),
        ("FDCANSEL", "at reset inside the rule's whole word"),
        ("SWPSEL", "at reset inside the rule's whole word"),
    ] {
        t.push(e(
            "RCC",
            "D2CCIP1R",
            field,
            W,
            Rule,
            SELECT_RULE,
            &d2_tags,
            &format!("{what}; also ignored-by-model: {d2_tags}"),
        ));
    }
    let d3_tags =
        format!("{RCC_CS} L432–444: every RCC_D3CCIPR field is a tag (SPI6SEL L443, I2C4SEL L435)");
    for field in [
        "SPI#SEL",
        "I#C#SEL",
        "LPUART#SEL",
        "LPTIM#SEL",
        "ADCSEL",
        "SAI#ASEL",
        "SAI#BSEL",
    ] {
        t.push(e(
            "RCC",
            "D3CCIPR",
            field,
            W,
            Rule,
            SELECT_RULE,
            &d3_tags,
            &format!(
                "written with SPI6 only; every field but SPI6SEL at reset inside the rule's word; also ignored-by-model: {d3_tags} (and Renode's SPI6SEL is 2 bits where RM0433's is 3, AET-WO-4230 B0)"
            ),
        ));
    }
    // ---- RCC: each enable gate, a read-modify-write of its bit.
    for (field, register, line, registered) in GATES {
        let flag = format!(
            "{RCC_CS} L{line} (the flag) and L1353–1365 (a write calls sysbus.SetPeripheralEnabled on the registered peripheral)"
        );
        let entry = match registered {
            Some(m) => e(
                "RCC",
                register,
                field,
                W,
                Obs,
                &format!("the model enables or disables the peripheral on the bus: {flag}"),
                &format!("{flag}; {m}"),
                &format!(
                    "the platform registers the peripheral: {m}; also judged in full by the twin's register image (aethelgard_twin::boot::expected, from the topology)"
                ),
            ),
            None => e(
                "RCC",
                register,
                field,
                W,
                Same,
                ORACLE_ONLY,
                "",
                &format!(
                    "the model acts on it only for a peripheral the platform registers with the RCC ({flag}; L1355–1358 otherwise logs and drops it), and no held measurement shows this one registered; the twin's register image judges the whole enable register from the topology (consistency)"
                ),
            ),
        };
        t.push(entry);
    }
    // ---- RCC: the reset fields `ResetClock` reads before any write.
    for (register, field, line, what) in [
        (
            "CR",
            "HSIDIV",
            "148",
            "HSI's divider; the model's clock follows it (L1546)",
        ),
        ("CFGR", "SWS", "226", "reads the SW the model was given"),
        (
            "CFGR",
            "TIMPRE",
            "231",
            "the timer clock multiplier (L1490–1494)",
        ),
        ("D1CFGR", "HPRE", "242", "a divider the model computes from"),
        (
            "D1CFGR",
            "D#PPRE",
            "243",
            "a divider the model computes from",
        ),
        (
            "D1CFGR",
            "D#CPRE",
            "245",
            "a divider the model computes from",
        ),
        (
            "D2CFGR",
            "D#PPRE#",
            "254, L256",
            "a divider the model computes from",
        ),
        (
            "D3CFGR",
            "D#PPRE",
            "264",
            "a divider the model computes from",
        ),
    ] {
        let src = format!(
            "{RCC_CS} L{line}; AET-WO-4300a FEEDBACK §0 (stored fields, each starting at 0)"
        );
        t.push(e(
            "RCC",
            register,
            field,
            R,
            Obs,
            &format!("the model holds the field at its reset value and acts on it: {src}"),
            &src,
            &format!(
                "{what}; the firmware's ResetClock reads it and parks if it is not at reset, so a boot in the twin judges the read"
            ),
        ));
    }
    for (register, field, line) in [
        ("D1CCIPR", "CKPERSEL", "401"),
        ("D2CCIP1R", "SPI#SEL", "408, L410"),
        ("D2CCIP2R", "USART#SEL", "420–421"),
        ("D2CCIP2R", "I#C#SEL", "425"),
        ("D3CCIPR", "I#C#SEL", "435"),
        ("D3CCIPR", "SPI#SEL", "443"),
    ] {
        let src = format!(
            "{RCC_CS} L{line}: a tag, read 0 whatever was written (AET-WO-4230 FEEDBACK §2, measured for CKPERSEL, SPI123SEL, SPI45SEL, I2C123SEL)"
        );
        t.push(e(
            "RCC",
            register,
            field,
            R,
            Ign,
            &src,
            &src,
            "the kernel clock each independent rule derives assumes this reset code (silicon::spi_kernel_hz, i2c_kernel_hz, serial::kernel_hz); the twin reads 0 whatever silicon holds, so ResetClock's read of it is never judged",
        ));
    }

    // ---- RCC: `D2CKRDY`, a dependency the firmware never polls (the
    // architect's ruling on Q-F; RM0433 Rev 9 §8.5.10 step c, p. 372).
    let d2 = format!("{RCC_CS} L175: a tag, read 0 whatever silicon holds");
    t.push(e(
        "RCC",
        "CR",
        "D2CKRDY",
        R,
        Ign,
        &d2,
        &d2,
        "RM0433 Rev 9 §8.7.2 (p. 384): set by hardware once the D2 domain's clocks are available, 0 after reset; §8.5.10 step c asks the firmware to read it until 1 after enabling a D2 peripheral, which the firmware does not (a wait on the twin's 0 would park every boot), so silicon is taken to have set it: a bench row (3302)",
    ));

    // ---- CORE: the FPU enable in the generated `Reset`.
    t.push(e(
        "CORE",
        "CPACR",
        "CP#",
        W,
        Same,
        "none independent: written by the generated Reset's assembly (svd-hal board/firmware.rs), outside every Kani harness, and judged by no aethelgard-ir rule",
        "",
        "found by the tier-2 completeness test (AET-WO-4230 §5.4), not by svd-hal's register_fields; whether Renode's core faults a float instruction with CPACR clear is not held, so no reaction is claimed",
    ));

    // ---- FLASH: the brown-out level.
    t.push(e(
        "FLASH",
        "OPTSR_CUR",
        "BOR_LEV",
        R,
        Si,
        "the boot record's word 1 against topology.bor_level (BrownOutLevelNotProgrammed, contract D10); the twin presents the declared level itself (AET-WO-4230 FEEDBACK §2 m1), so only the bench shows the option byte was programmed",
        "",
        "set by the declaration and the factory step in FAB-NOTES, a read-modify-write of the word as read, the only correct procedure (AET-WO-4230 R8, the architect's ruling on Q-J); the whole word differs above bit 15 among the three values on record, each at BOR_LEV 00: the bench's 0x1BC6_AAF0 (AET-WO-3302, NUCLEO-H753ZI rev V, 2026-11-16 05:00 UTC), RM0433 Rev 9 Table 21's 0x0006_AAF0 and Renode's 0x0406_AAF0 (plus the level, as the twin presents it), visible to a firmware that reads all of it",
    ));

    // ---- GPIO.
    t.push(e(
        "GPIO",
        "OTYPER",
        "OT#",
        W,
        Ign,
        "OTYPER is not stored: AET-WO-4300b FEEDBACK §10 (pass 3, all-ones written and read back, measured); the twin lists every configured port's OTYPER as unobservable",
        "AET-WO-4300b FEEDBACK §10, measured",
        "written whole from the pins' peripherals (contract D5, B0): the proof's expected word is computed in svd-hal (oracle_otyper), not by an aethelgard-ir rule",
    ));
    let pin_routing = "AET-WO-4300 FEEDBACK §0: STM32_GPIOPort forwards a timer channel's level to a pin only while its MODER is AF and its AFR selects that function; Renode's USART has no pin (AET-WO-4300b FEEDBACK §0) and a BSRR write drives a pin whatever MODER says (AET-WO-4200 FEEDBACK)";
    for (register, field) in [("MODER", "MODER#"), ("AFRL", "AFSEL#"), ("AFRH", "AFSEL#")] {
        t.push(e(
            "GPIO",
            register,
            field,
            W,
            Same,
            ORACLE_ONLY,
            "",
            &format!(
                "the model acts on it only for a timer channel's pin ({pin_routing}); the table cannot see the pin's role, so the weaker class; the twin's register image and serial pin judge compare it with the topology (consistency)"
            ),
        ));
    }
    for (register, field, what) in [
        (
            "OSPEEDR",
            "OSPEED#",
            "no edge rate is modelled; `check speeds` judges the chosen speed's fmax with svd-hal's own table (SpeedCheck), not an aethelgard-ir rule",
        ),
        (
            "PUPDR",
            "PUPD#",
            "no pull is modelled; written whole per port by `split` (AET-WO-4230 R5), every pin's field at its role's pull or its reset value, and a motor-output pin's field is independent-rule instead (silicon::motor_output_pupd, see motor_pupd)",
        ),
    ] {
        t.push(e(
            "GPIO",
            register,
            field,
            W,
            Same,
            ORACLE_ONLY,
            "",
            &format!(
                "stored (only OTYPER is unstored among a port's registers, AET-WO-4300b FEEDBACK §10) with no reaction known: {what}; STM32_GPIOPort.cs is not held here"
            ),
        ));
    }
    for field in ["BS#", "BR#"] {
        t.push(e(
            "GPIO",
            "BSRR",
            field,
            W,
            Obs,
            "a BSRR write sets each pin's connection whatever MODER says (AET-WO-4200 FEEDBACK, STM32_GPIOPort.cs read; AET-WO-4300 FEEDBACK §0): a chip select drives the SPI multiplexer, and the device answers only while selected (AET-WO-4210 F11)",
            "AET-WO-4200 FEEDBACK (STM32_GPIOPort.cs read), AET-WO-4300 FEEDBACK §0",
            "a run-time value: a chip select's set or reset",
        ));
    }

    // ---- I2C.
    let i2c_unstored = "AET-WO-4300b FEEDBACK §10 (pass 3, measured): DNF, ANFOFF, TXDMAEN, SBC, WUPEN, GCEN, SMBHEN, SMBDEN, ALERTEN, PECEN are not stored";
    for field in [
        "TXIE",
        "RXIE",
        "ADDRIE",
        "NACKIE",
        "STOPIE",
        "TCIE",
        "ERRIE",
        "RXDMAEN",
        "NOSTRETCH",
    ] {
        t.push(e(
            "I2C",
            "CR1",
            field,
            W,
            Rule,
            I2C_CR1_RULE,
            "",
            "stored by the model (not among the unstored bits of AET-WO-4300b FEEDBACK §10); STM32F7_I2C.cs is not held, so no reaction is claimed",
        ));
    }
    for field in [
        "DNF", "ANFOFF", "TXDMAEN", "SBC", "WUPEN", "GCEN", "SMBHEN", "SMBDEN", "ALERTEN", "PECEN",
    ] {
        t.push(e(
            "I2C",
            "CR1",
            field,
            W,
            Rule,
            I2C_CR1_RULE,
            i2c_unstored,
            &format!("also ignored-by-model: {i2c_unstored}"),
        ));
    }
    t.push(e(
        "I2C",
        "CR1",
        "PE",
        W,
        Same,
        ORACLE_ONLY,
        "",
        "the rule's CR1 has PE clear and the enable adds it, so no aethelgard-ir rule decides PE = 1; stored (AET-WO-4300b FEEDBACK §10), and STM32F7_I2C.cs is not held, so no reaction is claimed",
    ));
    for field in ["SCLL", "SCLH", "SDADEL", "SCLDEL", "PRESC"] {
        t.push(e(
            "I2C",
            "TIMINGR",
            field,
            W,
            Rule,
            TIMINGR_RULE,
            "AET-WO-4300b FEEDBACK §10 (all of TIMINGR not stored, measured)",
            "also ignored-by-model: TIMINGR is not stored (AET-WO-4300b FEEDBACK §10, measured), so the twin cannot tell any SCL timing; the twin lists it as unobservable",
        ));
    }
    let i2c_read = "the device's model answered the firmware's read at its address, with the register pointer written and one byte read back (AET-WO-4220 FEEDBACK P1.3, measured: I2C 0x76 register 0xD0 read 0x5A; the model logged `write 1 byte(s), pointer 0xD0` and `read 1 byte(s)`)";
    for (field, what) in [
        ("SADD", "the address: only the device at it answered"),
        (
            "RD_WRN",
            "the direction: a pointer written, then a byte read",
        ),
        ("NBYTES", "the count: one byte each way"),
        ("START", "a transfer started"),
    ] {
        t.push(e(
            "I2C",
            "CR2",
            field,
            W,
            Obs,
            i2c_read,
            "AET-WO-4220 FEEDBACK P1.3, measured",
            &format!("{what}; a run-time value"),
        ));
    }
    for field in [
        "AUTOEND", "ADD#", "HEAD#R", "STOP", "NACK", "RELOAD", "PECBYTE",
    ] {
        t.push(e(
            "I2C",
            "CR2",
            field,
            W,
            Same,
            ORACLE_ONLY,
            "",
            "STM32F7_I2C.cs is not held and no measurement isolates it: no reaction is claimed",
        ));
    }
    t.push(e(
        "I2C",
        "TXDR",
        "TXDATA",
        W,
        Obs,
        i2c_read,
        "AET-WO-4220 FEEDBACK P1.3, measured",
        "the pointer byte reached the device's model; a run-time value",
    ));
    for field in [
        "ADDRCF", "NACKCF", "STOPCF", "BERRCF", "ARLOCF", "OVRCF", "PECCF", "TIMOUTCF", "ALERTCF",
    ] {
        t.push(e(
            "I2C",
            "ICR",
            field,
            W,
            Same,
            ORACLE_ONLY,
            "",
            "write-1-to-clear (RM0433 Rev 9 §47.9.8, p. 2015); STM32F7_I2C.cs is not held: no reaction is claimed",
        ));
    }

    // ---- SPI: CR1, CFG1, CFG2 written whole from the rule's words.
    for (register, field, note) in [
        (
            "CR1",
            "SPE",
            format!(
                "also observed: {SPI_CS} L92–105 (SPE starts a transfer or clears the FIFOs and TSIZE)"
            ),
        ),
        (
            "CR1",
            "CSTART",
            format!("also observed: {SPI_CS} L108–115 (CSTART starts the transfer)"),
        ),
        (
            "CR1",
            "CSUSP",
            format!("{SPI_CS} L116 (stored, set-only; 0 here)"),
        ),
        ("CR1", "IOLOCK", format!("{SPI_CS} L133 (stored)")),
        (
            "CR1",
            "SSI",
            format!(
                "also ignored-by-model: {SPI_CS} L129, a tag; measured, AET-WO-4230 FEEDBACK §2 (SSI 0x1000 reads 0)"
            ),
        ),
        (
            "CR1",
            "MASRX",
            format!("also ignored-by-model: {SPI_CS} L107, a tag"),
        ),
        (
            "CR1",
            "HDDIR",
            format!("also ignored-by-model: {SPI_CS} L128, a tag"),
        ),
        (
            "CR1",
            "CRC#_#",
            format!("also ignored-by-model: {SPI_CS} L130, a tag"),
        ),
        (
            "CR1",
            "RCRCINI",
            format!("also ignored-by-model: {SPI_CS} L131, a tag"),
        ),
        (
            "CR1",
            "TCRCINI",
            format!("also ignored-by-model: {SPI_CS} L132, a tag"),
        ),
        (
            "CFG1",
            "DSIZE",
            format!("also observed: {SPI_CS} L152 (the frame size the transfer uses)"),
        ),
        ("CFG1", "RXDMAEN", format!("{SPI_CS} L162 (stored)")),
        ("CFG1", "TXDMAEN", format!("{SPI_CS} L164 (stored)")),
        (
            "CFG1",
            "MBR",
            format!(
                "also ignored-by-model: {SPI_CS} L169, a tag; measured, AET-WO-4230 FEEDBACK §2 (MBR 7 reads 0); the twin renders no SCK rate"
            ),
        ),
        (
            "CFG1",
            "FTHLV",
            format!("also ignored-by-model: {SPI_CS} L153, a tag"),
        ),
        (
            "CFG1",
            "UDRCFG",
            format!("also ignored-by-model: {SPI_CS} L159, a tag"),
        ),
        (
            "CFG1",
            "UDRDET",
            format!("also ignored-by-model: {SPI_CS} L160, a tag"),
        ),
        (
            "CFG1",
            "CRCSIZE",
            format!("also ignored-by-model: {SPI_CS} L165, a tag"),
        ),
        (
            "CFG1",
            "CRCEN",
            format!("also ignored-by-model: {SPI_CS} L167, a tag"),
        ),
        (
            "CFG2",
            "MASTER",
            format!(
                "also ignored-by-model as behaviour: {SPI_CS} L188 always reads 1 (only master mode is modelled), while silicon clears MASTER on a mode fault (AET-WO-3302 bench, first silicon: CFG2 read 0 with SSM = 0; AET-WO-4230 FEEDBACK notes); see SR.MODF"
            ),
        ),
        (
            "CFG2",
            "LSBFRST",
            format!("also observed: {SPI_CS} L195 (the byte order the transfer uses)"),
        ),
        (
            "CFG2",
            "SSM",
            format!(
                "also ignored-by-model: {SPI_CS} L198, a tag; measured, AET-WO-4230 FEEDBACK §2 (SSM reads 0)"
            ),
        ),
        (
            "CFG2",
            "MSSI",
            format!("also ignored-by-model: {SPI_CS} L175, a tag"),
        ),
        (
            "CFG2",
            "MIDI",
            format!("also ignored-by-model: {SPI_CS} L176, a tag"),
        ),
        (
            "CFG2",
            "IOSWP",
            format!("also ignored-by-model: {SPI_CS} L183, a tag"),
        ),
        (
            "CFG2",
            "COMM",
            format!("also ignored-by-model: {SPI_CS} L185, a tag"),
        ),
        (
            "CFG2",
            "SP",
            format!("also ignored-by-model: {SPI_CS} L186, a tag"),
        ),
        (
            "CFG2",
            "CPHA",
            format!("also ignored-by-model: {SPI_CS} L196, a tag"),
        ),
        (
            "CFG2",
            "CPOL",
            format!("also ignored-by-model: {SPI_CS} L197, a tag"),
        ),
        (
            "CFG2",
            "SSIOP",
            format!("also ignored-by-model: {SPI_CS} L200, a tag"),
        ),
        (
            "CFG2",
            "SSOE",
            format!("also ignored-by-model: {SPI_CS} L201, a tag"),
        ),
        (
            "CFG2",
            "SSOM",
            format!("also ignored-by-model: {SPI_CS} L202, a tag"),
        ),
        (
            "CFG2",
            "AFCNTR",
            format!(
                "AET-WO-4230 R4 (the architect's ruling on Q-D; RM0433 Rev 9 §50.11.4): 1 on every master, so the controller keeps driving its pins between transfers (ST's HAL MasterKeepIOState); the rule's CFG2 carries it, and a generator that clears it fails the proof (R2-gen's AFCNTR mutant); also ignored-by-model: {SPI_CS} L203, a tag"
            ),
        ),
    ] {
        t.push(e("SPI", register, field, W, Rule, SPI_RULE, "", &note));
    }
    t.push(e(
        "SPI",
        "CR2",
        "TSIZE",
        W,
        Obs,
        &format!("{SPI_CS} L146 and L376–385: the transfer counts TSIZE packets and ends at it"),
        &format!("{SPI_CS} L146"),
        "a run-time value",
    ));
    t.push(e(
        "SPI",
        "CR2",
        "TSER",
        W,
        Ign,
        &format!("{SPI_CS} L149: a tag"),
        &format!("{SPI_CS} L149"),
        "",
    ));
    t.push(e(
        "SPI",
        "TXDR",
        "TXDR",
        W,
        Obs,
        &format!("{SPI_CS} L277–281: a write is queued and transmitted to the selected device"),
        &format!("{SPI_CS} L277"),
        "a run-time value (one-byte writes)",
    ));
    t.push(e(
        "SPI",
        "IFCR",
        "EOTC",
        W,
        Obs,
        &format!("{SPI_CS} L251–257: EOTC resets the transmission state"),
        &format!("{SPI_CS} L251"),
        "",
    ));
    t.push(e(
        "SPI",
        "IFCR",
        "SUSPC",
        W,
        Obs,
        &format!("{SPI_CS} L267–273: SUSPC clears SUSP"),
        &format!("{SPI_CS} L267"),
        "written 0",
    ));
    for (field, line) in [
        ("TXTFC", 258),
        ("UDRC", 259),
        ("OVRC", 260),
        ("CRCEC", 261),
        ("TIFREC", 262),
        ("MODFC", 263),
        ("TSERFC", 266),
    ] {
        let src = format!("{SPI_CS} L{line}: a tag");
        t.push(e("SPI", "IFCR", field, W, Ign, &src, &src, "written 0"));
    }
    t.push(e(
        "SPI",
        "SR",
        "MODF",
        R,
        Ign,
        &format!(
            "{SPI_CS} L238: MODF is a tag, so the model raises no mode fault, and CFG2.MASTER always reads 1 (L188)"
        ),
        &format!("{SPI_CS} L238, L188"),
        "the bring-up depends on MODF staying 0: on first silicon (AET-WO-3302 bench, SSM = 0) the first SPE raised it (SR 0x0000_1202) and cleared MASTER, demoting the controller to a disabled slave; the twin cannot show it. SSM | SSI before SPE is proved by Kani (AET-WO-4230 B1's MODF guard)",
    ));

    // ---- USART.
    for field in ["DIV_FRACTION", "DIV_MANTISSA"] {
        t.push(e(
            "USART",
            "BRR",
            field,
            W,
            Rule,
            BRR_RULE,
            "",
            "check_brr holds the nearest whole USARTDIV, so a BRR one off fails it; serial::check_silicon also judges the worst-case receiver tolerance (HSI's extremes and the peer) at solve (SerialBaudTolerance). Stored (16 bits, STM32F7_USART.cs L210 per aethelgard-twin boot.rs) and read back by the twin's register image; the model renders no wire time (AET-WO-4300b FEEDBACK §10)",
        ));
    }
    let usart_src = "AET-WO-4300b FEEDBACK §0 (STM32F7_USART.cs read): a TDR write reaches the backend only through TransmitCharacter, when TE and UE are set; UARTBase.cs L80–92: an injected byte is queued only when RE and UE are set";
    for (field, what) in [
        (
            "UE",
            "measured: with UE clear nothing is captured (AET-WO-4300b FEEDBACK P2.3)",
        ),
        ("TE", "the transmitter"),
        ("RE", "the receiver: the echo needs it"),
    ] {
        t.push(e("USART", "CR1", field, W, Obs, usart_src, usart_src, what));
    }
    let usart_stored = "stored (AET-WO-4300b FEEDBACK §0: UE, RE, TE, RXNEIE, TCIE, TXEIE, PEIE, PS, PCE, OVER8, RTOIE) with no reaction claimed; the twin's register image compares it with the topology (aethelgard_twin::boot::expected), which is consistency";
    for field in [
        "RXNEIE_RXFNEIE",
        "TCIE",
        "TXEIE_TXFNFIE",
        "PEIE",
        "PS",
        "PCE",
        "OVER#",
        "RTOIE",
    ] {
        t.push(e(
            "USART",
            "CR1",
            field,
            W,
            Same,
            ORACLE_ONLY,
            "",
            usart_stored,
        ));
    }
    let usart_unstored = "not among the bits the model stores (AET-WO-4300b FEEDBACK §0); M0 and M1 tags measured (P4.1); FIFOEN in bits the model reserves (measured, AET-WO-4300b fix 2 f1)";
    for field in [
        "UESM", "IDLEIE", "WAKE", "M#", "MME", "CMIE", "DEDT", "DEAT", "EOBIE", "FIFOEN", "TXFEIE",
        "RXFFIE",
    ] {
        t.push(e(
            "USART",
            "CR1",
            field,
            W,
            Ign,
            usart_unstored,
            "AET-WO-4300b FEEDBACK §0, P4.1, measured",
            "the serial judge reads M0, M1 and FIFOEN only in the words the access log shows written (consistency)",
        ));
    }
    t.push(e(
        "USART",
        "CR2",
        "STOP",
        W,
        Same,
        ORACLE_ONLY,
        "",
        "stored (the only stored CR2 field, AET-WO-4300b FEEDBACK §0) with no reaction claimed; the twin's register image compares it with the topology (consistency)",
    ));
    for field in [
        "SLVEN", "DIS_NSS", "ADDM#", "LBDL", "LBDIE", "LBCL", "CPHA", "CPOL", "CLKEN", "LINEN",
        "SWAP", "RXINV", "TXINV", "DATAINV", "MSBFIRST", "ABREN", "ABRMODE", "RTOEN", "ADD",
    ] {
        t.push(e(
            "USART",
            "CR2",
            field,
            W,
            Ign,
            "STOP is the only CR2 field the model stores (AET-WO-4300b FEEDBACK §0); RXINV and TXINV are tags, measured (P4.1)",
            "AET-WO-4300b FEEDBACK §0, P4.1, measured",
            "",
        ));
    }
    t.push(e(
        "USART",
        "TDR",
        "TDR",
        W,
        Obs,
        "the bytes written reach the capture (AET-WO-4300b FEEDBACK P2.1, measured)",
        "AET-WO-4300b FEEDBACK P2.1, measured",
        "a run-time value",
    ));

    // ---- TIM: every register written whole by a timer's start.
    let tim_model = "AET-WO-4300a FEEDBACK §0 (STM32_Timer.cs read)";
    for (register, field, what) in [
        (
            "PSC",
            "PSC",
            "also observed: stored as the divider (STM32_Timer.cs per AET-WO-4300a FEEDBACK §0)",
        ),
        (
            "ARR",
            "ARR",
            "also observed: the update limit (STM32_Timer.cs L59 per AET-WO-4300a FEEDBACK §0)",
        ),
        (
            "CCR#",
            "CCR#",
            "also observed: the compare limit that ends the pulse (STM32_Timer.cs L105–124 per AET-WO-4300a FEEDBACK §0)",
        ),
    ] {
        t.push(e("TIM", register, field, W, Rule, TIM_RULE, "", what));
    }
    t.push(e(
        "TIM",
        "CR1",
        "CEN",
        W,
        Obs,
        &format!("CEN starts the counter at 0: {tim_model}, STM32_Timer.cs L143–151"),
        tim_model,
        "",
    ));
    t.push(e(
        "TIM",
        "CR1",
        "OPM",
        W,
        Obs,
        &format!("OPM stops the counter at the first update: {tim_model}, STM32_Timer.cs L57"),
        tim_model,
        "written 0",
    ));
    for field in ["ARPE", "UDIS", "URS", "DIR", "CMS"] {
        t.push(e(
            "TIM",
            "CR1",
            field,
            W,
            Same,
            ORACLE_ONLY,
            "",
            "stored (bits 0–7, STM32_Timer.cs L143–165 per aethelgard-twin boot.rs) with no reaction claimed; the twin's register image compares it with the topology (consistency)",
        ));
    }
    for field in ["CKD", "UIFREMAP"] {
        t.push(e(
            "TIM",
            "CR1",
            field,
            W,
            Ign,
            "outside the bits the model stores (CKD a tag; CR1 stored bits 0–7): AET-WO-4300a FEEDBACK §0",
            "AET-WO-4300a FEEDBACK §0",
            "",
        ));
    }
    t.push(e(
        "TIM",
        "CCMR#",
        "OC#M[#:#]",
        W,
        Obs,
        "PWM mode 1 sets the output at the update and clears it at the compare: AET-WO-4300a FEEDBACK §0 (STM32_Timer.cs L50–83, L105–124)",
        "AET-WO-4300a FEEDBACK §0",
        "",
    ));
    t.push(e(
        "TIM",
        "CCMR#",
        "OC#M[#]",
        W,
        Ign,
        "OCxM is 3 bits in the model; bit 3 is reserved there (AET-WO-4300a FEEDBACK §0)",
        "AET-WO-4300a FEEDBACK §0",
        "",
    ));
    for field in ["OC#PE", "OC#FE", "OC#CE"] {
        t.push(e(
            "TIM",
            "CCMR#",
            field,
            W,
            Ign,
            "a tag (AET-WO-4300a FEEDBACK §0); the twin lists OCxPE as unobservable",
            "AET-WO-4300a FEEDBACK §0",
            "",
        ));
    }
    t.push(e(
        "TIM",
        "CCMR#",
        "CC#S",
        W,
        Same,
        ORACLE_ONLY,
        "",
        "stored (STM32_Timer.cs L425–445 per aethelgard-twin boot.rs) with no reaction claimed; the twin's register image compares it with the topology (consistency)",
    ));
    for field in ["CC#E", "CC#P", "CC#NP"] {
        t.push(e(
            "TIM",
            "CCER",
            field,
            W,
            Same,
            ORACLE_ONLY,
            "",
            "stored (aethelgard-twin boot.rs: CCxE, CCxP and CCxNP stored) with no reaction claimed; the twin's register image compares it with the topology (consistency)",
        ));
    }
    t.push(e(
        "TIM",
        "CCER",
        "CC#NE",
        W,
        Ign,
        "a tag (AET-WO-4300a FEEDBACK §0)",
        "AET-WO-4300a FEEDBACK §0",
        "",
    ));
    for field in [
        "MOE", "AOE", "BKE", "BKP", "BKF", "BK#E", "BK#P", "BK#F", "DTG", "LOCK", "OSSI", "OSSR",
    ] {
        t.push(e(
            "TIM",
            "BDTR",
            field,
            W,
            Ign,
            "every BDTR field is a tag, MOE included (AET-WO-4300a FEEDBACK §0, STM32_Timer.cs L377–386); the twin lists MOE and BKE as unobservable",
            "AET-WO-4300a FEEDBACK §0",
            "",
        ));
    }
    t.push(e(
        "TIM",
        "EGR",
        "UG",
        W,
        Obs,
        "UG zeroes the counter: AET-WO-4300a FEEDBACK §0 (STM32_Timer.cs L265–295)",
        "AET-WO-4300a FEEDBACK §0",
        "it does not touch a channel's output in the model",
    ));
    for field in ["CC#G", "COMG", "TG", "BG", "B#G"] {
        t.push(e(
            "TIM",
            "EGR",
            field,
            W,
            Same,
            ORACLE_ONLY,
            "",
            "written 0; no measurement or held source line shows the model's handling",
        ));
    }
    t
}

// ------------------------------------------------------------ the twin's limits

/// The registers each model of a scripted part answers 0x00 with a warning,
/// one row per set, as the repository lists them: (part, what, source).
/// Renode's stock BME280, TMP117 and flash models are not held, so no list
/// exists for them and none is added.
const MODEL_SETS: [(&str, &str, &str); 5] = [
    (
        "ICM20948",
        "0x06.CLKSEL",
        "a tag in Renode's stock model: the script's configure write 0x06 = 0x01 warns `Tags: CLKSEL (0x1)` and is not stored (AET-WO-4210 FEEDBACK P1.3, measured; aethelgard-twin model.rs ICM_WRITE_WARNINGS)",
    ),
    (
        "ICM42688P",
        "bank1:0x0F,0x10",
        "factory-trimmed `0xXX` values with no datasheet fact: read 0x00, writes ignored, a warning (AET-WO-4220 FEEDBACK D-B7; models/renode/ICM42688P.cs L174, L242)",
    ),
    (
        "ICM42688P",
        "bank2:0x3B",
        "a factory-trimmed `0xXX` value with no datasheet fact: read 0x00, writes ignored, a warning (AET-WO-4220 FEEDBACK D-B7; models/renode/ICM42688P.cs L174, L242)",
    ),
    (
        "ICM42688P",
        "bank3,bank4",
        "banks 3 and 4 are not modelled: every access reads 0x00 with a warning (AET-WO-4220 FEEDBACK D-B7; models/renode/ICM42688P.cs L172–175)",
    ),
    (
        "BMP581",
        "reserved",
        "every reserved address reads 0x00, ignores writes and warns (AET-WO-4220 FEEDBACK D-B7; models/renode/BMP581.cs L193, L221)",
    ),
];

/// Each device whose script the firmware runs: the twin has a model for its
/// part (the composition's `answers`) and the part has a script.
fn scripted(topology: &SolvedTopology) -> Vec<(String, String)> {
    let answers = |part: &str| aethelgard_twin::model::model(part).is_ok_and(|m| m.is_some());
    topology
        .i2c_devices_in_order()
        .into_iter()
        .map(|(r, d)| (r.to_owned(), d.part.clone()))
        .chain(
            topology
                .spi_devices_in_order()
                .into_iter()
                .map(|(r, d)| (r.to_owned(), d.part.clone())),
        )
        .filter(|(_, part)| answers(part))
        .collect()
}

fn wait_rows(topology: &SolvedTopology) -> Result<Vec<LedgerRow>, LedgerError> {
    let parts = aethelgard_ir::parts::table().map_err(|e| LedgerError::Fields(e.to_string()))?;
    let settles: Vec<(String, String, u32)> = scripted(topology)
        .into_iter()
        .filter_map(|(reference, part)| {
            let settle = parts
                .get(&part)
                .and_then(|p| p.script.as_ref())
                .and_then(|s| s.settle_us)
                .filter(|&s| s > 0)?;
            Some((reference, part, settle))
        })
        .collect();
    // The bring-up waits once, for the largest (`AET-WO-4230` R6).
    let wait = settles.iter().map(|s| s.2).max().unwrap_or(0);
    let mut out = Vec::new();
    for (reference, part, settle) in settles {
        let Some(script) = parts.get(&part).and_then(|p| p.script.as_ref()) else {
            continue;
        };
        let condition = script.start_up.required_settle().map_or_else(
            || "no start-up is printed (a stand-in)".to_owned(),
            |(us, c)| format!("{c} ({us} µs)"),
        );
        out.push(LedgerRow {
            id: format!("WAIT.{reference}"),
            family: "WAIT".into(),
            register: String::new(),
            field: reference.clone(),
            pos: 0,
            width: 0,
            access: "none",
            value: Some(settle),
            class: Class::SiliconOnly,
            judge: format!(
                "aethelgard_ir::parts at load (SettleBelowStartUp, WO §1.9): {part}'s settle_us {settle} µs, {condition}"
            ),
            note: format!(
                "the bring-up waits once, after every scripted device's configuration and before any data read, for the largest settle_us among them ({wait} µs), which covers this one's because the devices power up together (AET-WO-4230 R6, the architect's ruling on Q-E); the twin runs the spin about 1.92 times long and models no instruction timing (AET-WO-4220 F2, D-B5), so only the bench shows the wait suffices; start-up: {}",
                script.start_up.condition
            ),
            acknowledged: None,
            reason: None,
            address: None,
        });
    }
    Ok(out)
}

fn model_rows(topology: &SolvedTopology) -> Vec<LedgerRow> {
    let mut out = Vec::new();
    for (reference, part) in scripted(topology) {
        for (_, what, source) in MODEL_SETS.iter().filter(|m| m.0 == part) {
            out.push(LedgerRow {
                id: format!("MODEL.{reference}.{what}"),
                family: "MODEL".into(),
                register: String::new(),
                field: format!("{reference} {what}"),
                pos: 0,
                width: 0,
                access: "none",
                value: Some(0),
                class: Class::IgnoredByModel,
                judge: (*source).to_owned(),
                note: format!("{part}: what silicon answers there is not modelled"),
                acknowledged: None,
                reason: None,
                address: None,
            });
        }
    }
    out
}

// ------------------------------------------------------------ the ledger

fn access_name(a: FieldAccess) -> &'static str {
    match a {
        FieldAccess::Written => "written",
        FieldAccess::Reset => "reset",
    }
}

fn id_of(f: &FieldUse) -> String {
    match f.access {
        FieldAccess::Written => f.id(),
        FieldAccess::Reset => format!("{}@reset", f.id()),
    }
}

/// The ledger's register rows, one per field `fields` lists (and `SR.MODF`
/// of each SPI controller), each classed by `table`.
pub fn register_rows(fields: &[FieldUse], table: &[Entry]) -> Result<Vec<LedgerRow>, LedgerError> {
    let facts = cmsis::register_facts().map_err(|e| LedgerError::Fields(e.to_string()))?;
    // The generated `Reset` enables the FPU before `main` (svd-hal
    // board/firmware.rs: `ldr r0, =0xE000ED88`, `orr r1, r1, #0x00F00000`,
    // `str`): a read-modify-write of `CPACR`'s CP10 and CP11 (bits 21:20 and
    // 23:22) to full access, `0b11`. `register_fields` lists only the board
    // crate's register writes, so the tier-2 completeness test found this one
    // missing (AET-WO-4230 §5.4).
    let mut uses: Vec<FieldUse> = [("CP10", 20u8), ("CP11", 22)]
        .into_iter()
        .map(|(field, pos)| FieldUse {
            family: "CORE".into(),
            register: "SCB.CPACR".into(),
            offset: CPACR_OFFSET,
            field: field.into(),
            pos,
            width: 2,
            access: FieldAccess::Written,
            value: Some(0b11),
            whole_word: false,
        })
        .collect();
    uses.extend_from_slice(fields);
    // The controllers' mode-fault flag, which the bring-up depends on staying
    // at reset (the architect's first-silicon note).
    let spis: Vec<String> = fields
        .iter()
        .filter(|f| f.family == "SPI")
        .filter_map(|f| f.register.split_once('.').map(|(i, _)| i.to_owned()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    for spi in spis {
        let (pos, width) = facts
            .field("SPI_SR_MODF")
            .map_err(|e| LedgerError::Fields(e.to_string()))?;
        let offset = facts
            .layout("SPI")
            .and_then(|l| l.offset("SR", 0))
            .map_err(|e| LedgerError::Fields(e.to_string()))?;
        uses.push(FieldUse {
            family: "SPI".into(),
            register: format!("{spi}.SR"),
            offset,
            field: "MODF".into(),
            pos: u8::try_from(pos).unwrap_or(u8::MAX),
            width: u8::try_from(width).unwrap_or(u8::MAX),
            access: FieldAccess::Reset,
            value: Some(0),
            whole_word: false,
        });
    }
    let mut out = Vec::new();
    for f in &uses {
        let id = id_of(f);
        let (instance, register) = f
            .register
            .split_once('.')
            .ok_or_else(|| LedgerError::Unclassified { id: id.clone() })?;
        let e = classify(table, &f.family, register, &f.field, f.access, &id)?;
        let address = base(&facts, instance).map(|b| b + f.offset);
        out.push(LedgerRow {
            id,
            family: f.family.clone(),
            register: f.register.clone(),
            field: f.field.clone(),
            pos: f.pos,
            width: f.width,
            access: access_name(f.access),
            value: f.value,
            class: e.class,
            judge: e.judge.clone(),
            note: e.note.clone(),
            acknowledged: None,
            reason: None,
            address,
        });
    }
    Ok(out)
}

/// The system control block's base and `CPACR`'s offset in it: the address
/// the generated `Reset` writes, `0xE000ED88` (ST's device header does not
/// define the core's blocks; CMSIS's `core_cm7.h` is not vendored).
const SCB_BASE: u32 = 0xE000_ED00;
const CPACR_OFFSET: u32 = 0x88;

/// The bus address of `instance`'s register block: the flash controller's is
/// `FLASH_R_BASE`, not the memory at `FLASH_BASE` (svd-hal board.rs).
pub fn base(facts: &cmsis::RegisterFacts, instance: &str) -> Option<u32> {
    if instance == "SCB" {
        return Some(SCB_BASE);
    }
    let name = if instance == "FLASH" {
        "FLASH_R"
    } else {
        instance
    };
    facts.base(name).ok()
}

/// Counts and evidence over `rows`; refuses two rows with one id.
pub fn ledger_of(rows: Vec<LedgerRow>) -> Result<Ledger, LedgerError> {
    let mut seen = std::collections::BTreeSet::new();
    for r in &rows {
        if !seen.insert(r.id.as_str()) {
            return Err(LedgerError::Duplicate { id: r.id.clone() });
        }
    }
    let mut by_class: BTreeMap<&'static str, usize> =
        Class::ALL.iter().map(|c| (c.name(), 0)).collect();
    let mut by_family: BTreeMap<String, usize> = BTreeMap::new();
    for r in &rows {
        *by_class.entry(r.class.name()).or_default() += 1;
        *by_family.entry(r.family.clone()).or_default() += 1;
    }
    let unacknowledged = rows
        .iter()
        .filter(|r| r.class.unwitnessed().is_some() && r.acknowledged != Some(true))
        .count();
    Ok(Ledger {
        total: rows.len(),
        rows,
        counts: Counts {
            by_class,
            by_family,
            unacknowledged,
        },
        evidence: Class::ALL
            .iter()
            .map(|c| (c.name(), c.evidence()))
            .collect(),
    })
}

/// The witness ledger of the firmware `generate` writes for `topology`:
/// register fields only (the architect's ruling on Q-I), none acknowledged
/// yet ([`acknowledge`]).
pub fn build(topology: &SolvedTopology, table: &PinTable) -> Result<Ledger, LedgerError> {
    let fields =
        register_fields(topology, table).map_err(|e| LedgerError::Fields(e.to_string()))?;
    let mut rows = register_rows(&fields, &TABLE)?;
    motor_pupd(&mut rows, topology)?;
    ledger_of(rows)
}

/// What the twin cannot judge outside the register fields (the architect's
/// ruling on Q-I): each scripted device's start-up wait (`WAIT`,
/// `silicon-only`) and each register a part's model answers 0x00 with a
/// warning (`MODEL`, `ignored-by-model`). Never acknowledged, never blocking
/// a package; no `PIN_C` row (Q-I).
pub fn twin_limits(topology: &SolvedTopology) -> Result<Vec<LedgerRow>, LedgerError> {
    let mut rows = wait_rows(topology)?;
    rows.extend(model_rows(topology));
    Ok(rows)
}

/// `AET-WO-4230` R5: each motor-output pin's `PUPDR` field is the
/// independent rule's ([`silicon::motor_output_pupd`]), not the table's
/// same-source; any other value is refused (`LedgerRuleDisagrees`). The pin
/// is the assignment of the output's net (`RUL-16`), never a role the table
/// cannot see.
pub fn motor_pupd(rows: &mut [LedgerRow], topology: &SolvedTopology) -> Result<(), LedgerError> {
    let rule = silicon::motor_output_pupd();
    for net in topology.motor_outputs.keys() {
        let pin = topology
            .assignments
            .get(net)
            .map(|a| a.pin.as_str())
            .ok_or_else(|| LedgerError::Fields(format!("motor output `{net}` has no pin")))?;
        let (port, index) = pin
            .strip_prefix('P')
            .and_then(|p| p.split_at_checked(1))
            .filter(|(port, n)| {
                port.bytes().all(|b| b.is_ascii_uppercase())
                    && !n.is_empty()
                    && n.bytes().all(|b| b.is_ascii_digit())
            })
            .ok_or_else(|| LedgerError::Fields(format!("motor output `{net}`'s pin `{pin}`")))?;
        let (register, field) = (format!("GPIO{port}.PUPDR"), format!("PUPD{index}"));
        let row = rows
            .iter_mut()
            .find(|r| r.access == "written" && r.register == register && r.field == field)
            .ok_or_else(|| LedgerError::Unclassified {
                id: format!("{register}.{field}"),
            })?;
        if row.value != Some(rule) {
            return Err(LedgerError::RuleDisagrees {
                id: row.id.clone(),
                value: row.value,
                rule,
            });
        }
        row.class = Class::IndependentRule;
        row.judge = MOTOR_PUPD_RULE.to_owned();
        row.note = format!(
            "motor output `{net}` on {pin}: pulled down from the first GPIO init until start_outputs, while the timer does not yet drive it; the table's class for any other pin's field is same-source (no pull is modelled)"
        );
    }
    Ok(())
}

// ------------------------------------------------------------ acknowledgement

/// Marks each row of an unwitnessed class acknowledged or not by `spec`
/// (`AET-WO-4230` R1): acknowledged exactly when the entry for its register
/// and class names its field, with that entry's reason. A field the
/// generator newly writes in an acknowledged register is therefore not
/// acknowledged. Nothing is dropped. Returns the ids of the rows left
/// unacknowledged, in the ledger's order.
pub fn acknowledge(ledger: &mut Ledger, spec: &Specification) -> Vec<String> {
    let mut missing = Vec::new();
    for r in &mut ledger.rows {
        let Some(class) = r.class.unwitnessed() else {
            continue;
        };
        let entry = spec
            .unwitnessed_acknowledgement(&r.register, class)
            .filter(|a| a.fields.contains(&r.field));
        r.acknowledged = Some(entry.is_some());
        r.reason = entry.map(|a| a.reason.clone());
        if entry.is_none() {
            missing.push(r.id.clone());
        }
    }
    ledger.counts.unacknowledged = missing.len();
    missing
}

/// An acknowledgement that names what the ledger lacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stale {
    /// Its index in `acknowledge_unwitnessed`.
    pub index: usize,
    pub register: String,
    pub class: &'static str,
    /// Each field it names that the ledger has no row of for that register
    /// and class: all of them when the ledger lacks the register.
    pub fields: Vec<String>,
}

impl Stale {
    pub const KIND: &'static str = "UnwitnessedAcknowledgementStale";

    /// The entry's JSON path.
    pub fn path(&self) -> String {
        format!("acknowledge_unwitnessed[{}].fields", self.index)
    }

    pub fn message(&self) -> String {
        format!(
            "`acknowledge_unwitnessed` acknowledges {} of register `{}` as `{}`, which the witness ledger of this specification's firmware has no row of: an acknowledgement names exactly the ledger's fields (AET-WO-4230 R1)",
            self.fields
                .iter()
                .map(|f| format!("`{f}`"))
                .collect::<Vec<_>>()
                .join(", "),
            self.register,
            self.class
        )
    }
}

/// The first entry of `spec.acknowledge_unwitnessed` naming a field the
/// ledger has no row of for its register and class (a register the ledger
/// lacks included), in document order (`AET-WO-4230` R1).
pub fn stale(ledger: &Ledger, spec: &Specification) -> Option<Stale> {
    spec.acknowledge_unwitnessed
        .iter()
        .enumerate()
        .find_map(|(index, a)| {
            let class = a.class_kind()?;
            let fields: Vec<String> = a
                .fields
                .iter()
                .filter(|f| {
                    !ledger.rows.iter().any(|r| {
                        r.register == a.register
                            && r.class.unwitnessed() == Some(class)
                            && r.field == **f
                    })
                })
                .cloned()
                .collect();
            (!fields.is_empty()).then(|| Stale {
                index,
                register: a.register.clone(),
                class: class.name(),
                fields,
            })
        })
}

/// One serial port whose peer deviation is declared, not cited
/// (`AET-WO-4230` R2, the architect's ruling on Q-A): the verdict names
/// each with its figure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeclaredPeer {
    pub port: String,
    pub peer_deviation_ppm: u32,
}

/// Every port of `spec` whose `peer_deviation_source` is `declared`, in the
/// specification's order.
pub fn declared_peers(spec: &Specification) -> Vec<DeclaredPeer> {
    spec.serial_ports
        .iter()
        .filter(|p| p.peer_deviation_source == PeerDeviationSource::Declared)
        .map(|p| DeclaredPeer {
            port: p.label.clone(),
            peer_deviation_ppm: p.peer_deviation_ppm,
        })
        .collect()
}

// ------------------------------------------------------------ silicon findings

/// Every SPI and I2C controller's independent rule and every USART's
/// `BRR` rule, as the verdict carries them (`AET-WO-4230` §1.7, R3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Silicon {
    pub spi: Vec<SpiFinding>,
    pub i2c: Vec<I2cFinding>,
    pub usart: Vec<UsartFinding>,
    pub holds: bool,
}

/// [`silicon::check_brr`] for one serial port's USART.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsartFinding {
    pub port: String,
    pub peripheral: String,
    #[serde(flatten)]
    pub brr: BrrFinding,
}

pub fn silicon(topology: &SolvedTopology) -> Silicon {
    let spi: Vec<SpiFinding> = topology
        .spi_controllers
        .iter()
        .map(|(p, c)| silicon::check_spi(p, c))
        .collect();
    let i2c: Vec<I2cFinding> = topology
        .i2c_timings
        .iter()
        .map(|(p, t)| silicon::check_i2c(p, t))
        .collect();
    let usart: Vec<UsartFinding> = topology
        .serial_ports
        .iter()
        .map(|(port, p)| UsartFinding {
            port: port.clone(),
            peripheral: p.peripheral.clone(),
            brr: silicon::check_brr(p.kernel_hz, p.framing.baud, p.brr),
        })
        .collect();
    let holds = spi.iter().all(|f| f.holds)
        && i2c.iter().all(|f| f.holds)
        && usart.iter().all(|f| f.brr.holds);
    Silicon {
        spi,
        i2c,
        usart,
        holds,
    }
}

/// One message per controller whose rule does not hold, naming it; empty
/// exactly when `s.holds`.
pub fn silicon_failures(s: &Silicon) -> Vec<String> {
    let flags = |pairs: &[(&str, bool)]| -> String {
        pairs
            .iter()
            .filter(|(_, ok)| !ok)
            .map(|(n, _)| *n)
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut out: Vec<String> = s
        .spi
        .iter()
        .filter(|f| !f.holds)
        .map(|f| {
            format!(
                "{}: aethelgard_ir::silicon::check_spi does not hold ({} false; kernel {} Hz, MBR {}, declared {} Hz)",
                f.peripheral,
                flags(&[
                    ("kernel_ok", f.kernel_ok),
                    ("mbr_ok", f.mbr_ok),
                    ("achieved_ok", f.achieved_ok)
                ]),
                f.kernel_hz,
                f.mbr,
                f.declared_hz
            )
        })
        .collect();
    out.extend(s.i2c.iter().filter(|f| !f.holds).map(|f| {
        let words: Vec<String> = f
            .timingr
            .iter()
            .filter(|w| !w.holds)
            .map(|w| format!("{} TIMINGR {:#010x}", w.mode, w.timingr))
            .collect();
        format!(
            "{}: aethelgard_ir::silicon::check_i2c does not hold ({}{}{})",
            f.peripheral,
            flags(&[
                ("kernel_ok", f.kernel_ok),
                ("cr1_ok", f.cr1_ok),
                ("modes_ok", f.modes_ok)
            ]),
            if words.is_empty() { "" } else { "; " },
            words.join(", ")
        )
    }));
    out.extend(s.usart.iter().filter(|f| !f.brr.holds).map(|f| {
        format!(
            "{} ({}): aethelgard_ir::silicon::check_brr does not hold (BRR {} at {} baud on a {} Hz kernel: nearest {}, in range {})",
            f.peripheral, f.port, f.brr.brr, f.brr.baud, f.brr.kernel_hz, f.brr.nearest, f.brr.in_range
        )
    }));
    out
}

// ------------------------------------------------------------ the bench sheet

/// Registers the bench cannot read back: each field is write-only (`w`).
/// `GPIOx_BSRR` (RM0433 Rev 9 §11.4.7) and `I2C_ICR` (§47.9.8, p. 2015).
/// `SPI_IFCR` and `TIMx_EGR` are written to clear or to trigger an event, and
/// the bench sheet leaves them out as well: what they did is read in the
/// status register, not in them.
const WRITE_ONLY: [&str; 4] = ["BSRR", "ICR", "IFCR", "EGR"];

/// Whether a row belongs on the bench sheet: neither `independent-rule` nor
/// `observed`, a register row with a known value, in a register the bench
/// can read. The bench reads after the boot, so a reset row of a field the
/// firmware then writes is left out (its written row stands for it).
pub fn on_the_bench(ledger: &Ledger, r: &LedgerRow) -> bool {
    let rewritten = r.access == "reset"
        && ledger
            .rows
            .iter()
            .any(|w| w.access == "written" && w.register == r.register && w.field == r.field);
    !rewritten
        && !r.class.judged()
        && r.address.is_some()
        && r.value.is_some()
        && !r
            .register
            .split_once('.')
            .is_some_and(|(_, reg)| WRITE_ONLY.contains(&reg))
}

/// The bench sheet's register rows (WO §1.7), as markdown in the shape of
/// `AET-WO-3302.bench.md` B7/B8: register and field, address, mask, expected
/// value, the command that reads it, why the twin cannot tell, and the
/// columns the bench fills in.
pub fn bench_rows(ledger: &Ledger) -> String {
    let mut out = String::from(
        "| Register (`FIELD`) | Address | Mask | Expected | Read | Why the twin cannot tell | Bench (raw) | Bench & mask | Agree? |\n|---|---|---|---|---|---|---|---|---|\n",
    );
    for r in ledger.rows.iter().filter(|r| on_the_bench(ledger, r)) {
        let (Some(address), Some(value)) = (r.address, r.value) else {
            continue;
        };
        let mask = if r.width >= 32 {
            u32::MAX
        } else {
            ((1u32 << r.width) - 1) << r.pos
        };
        let why = match r.class {
            Class::IgnoredByModel => "the model stores or acts on none of it",
            Class::SiliconOnly => "no model: the twin presents the declared value",
            _ => "no independent rule judges it, and the model does not react",
        };
        let _ = writeln!(
            out,
            "| {} (`{}`) | `{address:#010x}` | `{mask:#010x}` | `{:#010x}` | `probe-rs read --chip STM32H753ZITx b32 {address:#010x} 1` | {why} | | | |",
            r.register,
            r.field,
            value << r.pos
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pattern_matches_a_digit_run_and_nothing_else() {
        assert!(matches("OT#", "OT0"));
        assert!(matches("OT#", "OT15"));
        assert!(!matches("OT#", "OT"));
        assert!(!matches("OT#", "OTX"));
        assert!(matches("OC#M[#:#]", "OC1M[2:0]"));
        assert!(!matches("OC#M[#:#]", "OC1M[3]"));
        assert!(matches("CRC#_#", "CRC33_17"));
        assert!(!matches("SPI#SEL", "SPI123SELX"));
    }

    #[test]
    fn every_entry_is_cited_and_no_two_entries_overlap() {
        for e in TABLE.iter() {
            assert!(cited(e), "{} {} {}", e.family, e.register, e.field);
        }
        // An entry's own pattern, its `#` replaced by a digit, matches only it.
        for (i, a) in TABLE.iter().enumerate() {
            let reg = a.register.replace('#', "1");
            let field = a.field.replace('#', "1");
            let hits: Vec<usize> = TABLE
                .iter()
                .enumerate()
                .filter(|(_, b)| {
                    b.family == a.family
                        && b.access == a.access
                        && matches(b.register, &reg)
                        && matches(&b.field, &field)
                })
                .map(|(j, _)| j)
                .collect();
            assert_eq!(hits, vec![i], "{} {} {}", a.family, a.register, a.field);
        }
    }

    #[test]
    fn observed_or_ignored_without_a_citation_is_refused() {
        for class in [Class::Observed, Class::IgnoredByModel] {
            let bare = e(
                "SPI",
                "CR1",
                "SPE",
                FieldAccess::Written,
                class,
                "a judge",
                "",
                "",
            );
            assert!(!cited(&bare), "{class:?}");
            let line = e(
                "SPI",
                "CR1",
                "SPE",
                FieldAccess::Written,
                class,
                "j",
                "STM32H7_SPI.cs L92",
                "",
            );
            assert!(cited(&line));
            let fb = e(
                "SPI",
                "CR1",
                "SPE",
                FieldAccess::Written,
                class,
                "j",
                "AET-WO-4300b FEEDBACK §10",
                "",
            );
            assert!(cited(&fb));
            let no_line = e(
                "SPI",
                "CR1",
                "SPE",
                FieldAccess::Written,
                class,
                "j",
                "STM32H7_SPI.cs L",
                "",
            );
            assert!(!cited(&no_line));
            let table = [bare];
            assert_eq!(
                classify(&table, "SPI", "CR1", "SPE", FieldAccess::Written, "x").unwrap_err(),
                LedgerError::Uncited { id: "x".into() }
            );
        }
        let no_judge = e(
            "SPI",
            "CR1",
            "SPE",
            FieldAccess::Written,
            Class::SameSource,
            " ",
            "",
            "",
        );
        assert!(!cited(&no_judge));
    }

    #[test]
    fn a_field_the_table_does_not_know_is_refused_by_name() {
        let err = classify(
            &TABLE,
            "SPI",
            "CFG1",
            "BPASS",
            FieldAccess::Written,
            "SPI1.CFG1.BPASS",
        )
        .unwrap_err();
        assert_eq!(
            err,
            LedgerError::Unclassified {
                id: "SPI1.CFG1.BPASS".into()
            }
        );
        assert_eq!(err.kind(), "LedgerFieldUnclassified");
        // The access is part of the key: a written select is the rule's, the
        // same field at reset a tag.
        assert_eq!(
            classify(
                &TABLE,
                "RCC",
                "D2CCIP1R",
                "SPI123SEL",
                FieldAccess::Written,
                "a"
            )
            .unwrap()
            .class,
            Class::IndependentRule
        );
        assert_eq!(
            classify(
                &TABLE,
                "RCC",
                "D2CCIP1R",
                "SPI123SEL",
                FieldAccess::Reset,
                "a"
            )
            .unwrap()
            .class,
            Class::IgnoredByModel
        );
        // A gate no generator emits (MDMA) is unknown.
        assert!(
            classify(
                &TABLE,
                "RCC",
                "AHB3ENR",
                "MDMAEN",
                FieldAccess::Written,
                "m"
            )
            .is_err()
        );
    }

    #[test]
    fn two_rows_with_one_id_are_refused() {
        let row = LedgerRow {
            id: "X.Y.Z".into(),
            family: "X".into(),
            register: "X.Y".into(),
            field: "Z".into(),
            pos: 0,
            width: 1,
            access: "written",
            value: Some(0),
            class: Class::SameSource,
            judge: ORACLE_ONLY.into(),
            note: String::new(),
            acknowledged: None,
            reason: None,
            address: None,
        };
        assert_eq!(
            ledger_of(vec![row.clone(), row]).unwrap_err(),
            LedgerError::Duplicate { id: "X.Y.Z".into() }
        );
    }
}
