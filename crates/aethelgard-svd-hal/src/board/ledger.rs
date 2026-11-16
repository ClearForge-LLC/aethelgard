//! The witness ledger's field list (`AET-WO-4230` §1.7, contract D9): every
//! register field the generated firmware writes, and every reset field it
//! depends on, each with its place, its width and its value. The CLI classes
//! each row (`verify`'s ledger); this module only enumerates.
//!
//! What is listed:
//! - a whole-word write lists every field of the register, by ST's CMSIS
//!   name ([`RegisterFacts::fields_of`]), as the word sets it;
//! - a read-modify-write lists only the fields it changes;
//! - the reset fields are [`Rcc::reset_clock`]'s (the clock tree and the
//!   kernel-clock selects the firmware leaves at reset) and `BOR_LEV`, which
//!   the firmware reads from `FLASH_OPTSR_CUR` and never writes.
//!
//! The writes listed are every one the board crate can make on the bring-up
//! path the reference firmware takes (each port's split and pins, each
//! controller's start and transfer, each USART's configuration and send, each
//! timer's start), whether or not a given firmware calls the transfer: a row
//! too many is a row to classify, a row missing is a written bit no row covers
//! (WO §5.4). `tests/register_fields.rs` holds the list to the generated
//! crate's own text.
//!
//! [`Rcc::reset_clock`]: crate::board

use std::collections::BTreeMap;

use aethelgard_ir::{PinTable, SolvedTopology};

use super::{
    BoardError, Facts, I2C_MODES, Kind, Plan, Role, clock_fields, port_pupdr, spi_code,
    timer_writes, usart_values,
};
use crate::cmsis::{self, RegisterError, RegisterFacts};

/// Whether the firmware writes a field or depends on its reset value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldAccess {
    Written,
    Reset,
}

/// One field the generated firmware writes or depends on at reset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldUse {
    /// The register's family: `RCC`, `FLASH`, `GPIO`, `I2C`, `SPI`, `USART`,
    /// `TIM`.
    pub family: String,
    /// The instance and register: `SPI1.CFG1`, `GPIOA.AFRL`, `RCC.D2CCIP1R`.
    pub register: String,
    /// The register's byte offset in its block.
    pub offset: u32,
    /// ST's CMSIS name of the field: `MBR`, `MODE5`, `SPI123SEL`. A field
    /// whose bits are not contiguous is one row per run, named
    /// `OC1M[2:0]`, `OC1M[3]` by the field's own bit numbers.
    pub field: String,
    pub pos: u8,
    pub width: u8,
    pub access: FieldAccess,
    /// For a written field, the value the bring-up leaves in it: the last
    /// write's (`PE` 1; `SPE` 0, as every transfer leaves it). `None` where
    /// the last write's value is decided at run time: a data byte, an
    /// address, a count, a chip select's set or reset bit. For a reset field,
    /// its reset value; for `BOR_LEV`, the level the topology declares, which
    /// the option bytes must hold and the judge compares.
    pub value: Option<u32>,
    /// Whether the write is of the whole word; `false` for a field changed by
    /// a read-modify-write, and for a reset field.
    pub whole_word: bool,
}

impl FieldUse {
    /// `SPI1.CFG1.MBR`.
    pub fn id(&self) -> String {
        format!("{}.{}", self.register, self.field)
    }
}

/// One write the board crate makes, as the ledger sees it.
#[derive(Debug, Clone)]
enum Write {
    /// The whole word: `known` is the mask of the bits whose value the
    /// generator knows (the rest are decided at run time), `word` their value.
    Whole { word: u32, known: u32 },
    /// A read-modify-write of the bits of `mask`, to `value`.
    Changed { mask: u32, value: u32 },
}

/// A register the firmware writes: its family, instance, name, CMSIS prefix,
/// offset, and its writes in program order.
#[derive(Debug, Clone)]
struct Register {
    family: &'static str,
    instance: String,
    name: String,
    prefix: String,
    offset: u32,
    writes: Vec<Write>,
}

/// Every field the firmware `generate` writes for `solved` touches, then
/// every reset field it depends on (contract D9).
pub fn register_fields(
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<Vec<FieldUse>, BoardError> {
    let plan = Plan::new("register-fields", solved, table)?;
    let f = cmsis::register_facts()?;
    let facts = Facts::new(&plan, &f)?;
    // The mode the firmware starts every I2C peripheral at: the highest the
    // lowest declared speed admits (`generate`, `firmware::highest`).
    let mode = solved
        .i2c_speeds
        .values()
        .min()
        .and_then(|&hz| I2C_MODES.iter().rev().find(|m| m.1 <= hz))
        .map(|m| m.0);
    let mut out = Vec::new();
    for r in registers(&plan, &facts, mode)? {
        out.extend(rows(&f, &r)?);
    }
    out.extend(reset_rows(&plan, &f, solved.bor_level)?);
    Ok(out)
}

/// The written registers, in the order the bring-up first writes each.
fn registers(plan: &Plan, facts: &Facts, mode: Option<&str>) -> Result<Vec<Register>, BoardError> {
    let mut regs: Vec<Register> = Vec::new();
    let mut add = |family: &'static str,
                   instance: &str,
                   name: &str,
                   prefix: &str,
                   offset: u32,
                   write: Write| {
        if let Some(r) = regs
            .iter_mut()
            .find(|r| r.instance == instance && r.offset == offset)
        {
            r.writes.push(write);
        } else {
            regs.push(Register {
                family,
                instance: instance.to_owned(),
                name: name.to_owned(),
                prefix: prefix.to_owned(),
                offset,
                writes: vec![write],
            });
        }
    };
    let whole = |word: u32| Write::Whole { word, known: !0 };
    // The kernel-clock selection, then each clock gate (a read-modify-write
    // of its one bit).
    for s in &facts.kernel_selects {
        let prefix = format!("RCC_{}", s.register);
        add("RCC", "RCC", s.register, &prefix, s.offset, whole(s.word));
    }
    let gates = facts
        .ports
        .values()
        .map(|p| &p.gate)
        .chain(facts.peripherals.values().map(|p| &p.gate))
        .chain(facts.timers.values().map(|t| &t.gate))
        .chain(facts.serials.values().map(|s| &s.gate));
    for g in gates {
        let prefix = format!("RCC_{}", g.register);
        let bit = 1u32 << g.bit;
        add(
            "RCC",
            "RCC",
            &g.register,
            &prefix,
            g.offset,
            Write::Changed {
                mask: bit,
                value: bit,
            },
        );
    }
    // Each port: its whole `OTYPER` and `PUPDR` (`split`), then each pin's
    // fields, as `into_*` writes them, then a GPIO output's `BSRR` set.
    let gp = &facts.gpio;
    let [mw, _, sw, _, aw] = gp.widths;
    let field_mask = |width: u32, pos: u32| ((1u32 << width) - 1) << pos;
    for port in &plan.ports {
        let inst = format!("GPIO{port}");
        let otyper = plan
            .pins
            .iter()
            .filter(|(_, p)| p.port == *port)
            .fold(0u32, |w, (pin, p)| {
                w | (p.policy().1 << facts.pins[pin].otyper)
            });
        add(
            "GPIO",
            &inst,
            "OTYPER",
            "GPIO_OTYPER",
            gp.otyper,
            whole(otyper),
        );
        add(
            "GPIO",
            &inst,
            "PUPDR",
            "GPIO_PUPDR",
            gp.pupdr,
            whole(port_pupdr(plan, facts, *port)),
        );
    }
    for (pin, p) in &plan.pins {
        let inst = format!("GPIO{}", p.port);
        let pf = &facts.pins[pin];
        let (mode, _, ospeed, _) = p.policy();
        let changed = |width: u32, pos: u32, value: u32| Write::Changed {
            mask: field_mask(width, pos),
            value: value << pos,
        };
        add(
            "GPIO",
            &inst,
            "OSPEEDR",
            "GPIO_OSPEEDR",
            gp.ospeedr,
            changed(sw, pf.ospeedr, ospeed),
        );
        if p.role == Role::Output {
            add(
                "GPIO",
                &inst,
                "BSRR",
                "GPIO_BSRR",
                gp.bsrr,
                whole(1u32 << pf.bs),
            );
        } else {
            let (name, prefix) = if pf.afr_index == 0 {
                ("AFRL", "GPIO_AFRL")
            } else {
                ("AFRH", "GPIO_AFRH")
            };
            add(
                "GPIO",
                &inst,
                name,
                prefix,
                gp.afr[pf.afr_index as usize],
                changed(aw, pf.afr, p.af),
            );
        }
        add(
            "GPIO",
            &inst,
            "MODER",
            "GPIO_MODER",
            gp.moder,
            changed(mw, pf.moder, mode),
        );
    }
    // Each I2C and SPI controller: its start, then its transfer.
    for (name, p) in &plan.peripherals {
        let pf = &facts.peripherals[name];
        match p.kind {
            Kind::I2c => {
                let Some(i) = &pf.i2c else { continue };
                let t = &plan.i2c[name];
                let pe = 1u32 << pf.enable;
                add("I2C", name, "CR1", "I2C_CR1", pf.cr1, whole(t.cr1));
                // `init` writes the mode the firmware starts it at.
                let timingr = mode.and_then(|m| t.timingr.get(m)).copied();
                add(
                    "I2C",
                    name,
                    "TIMINGR",
                    "I2C_TIMINGR",
                    pf.config,
                    Write::Whole {
                        word: timingr.unwrap_or(0),
                        known: if timingr.is_some() { !0 } else { 0 },
                    },
                );
                add("I2C", name, "CR1", "I2C_CR1", pf.cr1, whole(t.cr1 | pe));
                // A transfer: `CR2` (address, direction, count, `START`,
                // `AUTOEND` at run time; every other field 0), `TXDR`, `ICR`.
                // The 7-bit address goes to `SADD[7:1]`.
                let runtime = (0x7Fu32 << (i.sadd + 1))
                    | (1 << i.rd_wrn)
                    | (0xFF << i.nbytes)
                    | (1 << i.autoend);
                add(
                    "I2C",
                    name,
                    "CR2",
                    "I2C_CR2",
                    i.cr2,
                    Write::Whole {
                        word: 1 << i.start,
                        known: !runtime,
                    },
                );
                add(
                    "I2C",
                    name,
                    "TXDR",
                    "I2C_TXDR",
                    pf.txdr,
                    Write::Whole {
                        word: 0,
                        known: !0xFF,
                    },
                );
                add(
                    "I2C",
                    name,
                    "ICR",
                    "I2C_ICR",
                    i.icr,
                    whole((1 << i.stopcf) | (1 << i.nackcf)),
                );
            }
            Kind::Spi => {
                let Some(s) = &pf.spi else { continue };
                let code = spi_code(&plan.spi[name], s, pf.enable, pf.master.unwrap_or_default());
                // Every chip select of the bus driven high, then `init`.
                for sel in plan.selects.get(name).into_iter().flatten() {
                    let pin = &plan.pins[&sel.pin];
                    add(
                        "GPIO",
                        &format!("GPIO{}", pin.port),
                        "BSRR",
                        "GPIO_BSRR",
                        gp.bsrr,
                        whole(1u32 << facts.pins[&sel.pin].bs),
                    );
                }
                add("SPI", name, "CR1", "SPI_CR1", pf.cr1, whole(code.cr1_idle));
                add("SPI", name, "CFG1", "SPI_CFG1", s.cfg1, whole(code.cfg1));
                add("SPI", name, "CFG2", "SPI_CFG2", pf.config, whole(code.cfg2));
                if !plan.selects.contains_key(name) {
                    continue;
                }
                // A transfer: `TSIZE` (run time), the device selected, `SPE`,
                // `CSTART`, each frame's byte (one-byte writes of `TXDR`),
                // `EOTC`, `CR1` idle, the device deselected.
                add(
                    "SPI",
                    name,
                    "CR2",
                    "SPI_CR2",
                    s.cr2,
                    Write::Whole {
                        word: 0,
                        known: !(0xFFFF << s.tsize),
                    },
                );
                for sel in &plan.selects[name] {
                    let pin = &plan.pins[&sel.pin];
                    add(
                        "GPIO",
                        &format!("GPIO{}", pin.port),
                        "BSRR",
                        "GPIO_BSRR",
                        gp.bsrr,
                        Write::Whole { word: 0, known: 0 },
                    );
                }
                for word in [code.cr1_enable, code.cr1_start] {
                    add("SPI", name, "CR1", "SPI_CR1", pf.cr1, whole(word));
                }
                add(
                    "SPI",
                    name,
                    "TXDR",
                    "SPI_TXDR",
                    pf.txdr,
                    Write::Whole { word: 0, known: 0 },
                );
                add("SPI", name, "IFCR", "SPI_IFCR", s.ifcr, whole(1 << s.eotc));
                add("SPI", name, "CR1", "SPI_CR1", pf.cr1, whole(code.cr1_idle));
            }
        }
    }
    // Each USART: its configuration (`BRR`, `CR2`, `CR1`, `CR1 | UE`), then
    // each byte it sends.
    if let Some(u) = &facts.usart {
        for (name, sp) in &plan.serials {
            let (brr, cr2, cr1) = usart_values(sp, u);
            add("USART", name, "BRR", "USART_BRR", u.brr, whole(brr));
            add("USART", name, "CR2", "USART_CR2", u.cr2, whole(cr2));
            add("USART", name, "CR1", "USART_CR1", u.cr1, whole(cr1));
            add(
                "USART",
                name,
                "CR1",
                "USART_CR1",
                u.cr1,
                whole(cr1 | (1 << u.ue)),
            );
            add(
                "USART",
                name,
                "TDR",
                "USART_TDR",
                u.tdr,
                Write::Whole {
                    word: 0,
                    known: !0xFF,
                },
            );
        }
    }
    // Each timer's start, every register whole.
    if let Some(tf) = &facts.tim {
        for (n, t) in &plan.timers {
            let inst = format!("TIM{n}");
            for (reg, offset, value) in timer_writes(*n, t, tf) {
                add(
                    "TIM",
                    &inst,
                    reg,
                    &format!("TIM_{reg}"),
                    offset,
                    whole(value),
                );
            }
        }
    }
    Ok(regs)
}

/// The rows of one written register: every CMSIS field of a whole-word write
/// and every field a read-modify-write changes, in position order, each
/// with the value the last write leaves. A written bit no field covers is
/// refused: the ledger would miss it.
fn rows(f: &RegisterFacts, r: &Register) -> Result<Vec<FieldUse>, BoardError> {
    let fields = f.fields_of(&r.prefix)?;
    let mut covered = 0u32;
    let mut whole_word = false;
    let mut chosen: BTreeMap<u32, (String, u32)> = BTreeMap::new();
    for w in &r.writes {
        let touched = match w {
            Write::Whole { .. } => {
                whole_word = true;
                fields.iter().fold(0, |m, (_, _, mask)| m | mask)
            }
            Write::Changed { mask, .. } => *mask,
        };
        for (name, _, mask) in fields.iter().filter(|(_, _, m)| m & touched != 0) {
            for (run_name, run) in runs(name, *mask) {
                chosen.insert(run.trailing_zeros(), (run_name, run));
            }
            covered |= mask;
        }
        let written = match w {
            Write::Whole { word, known } => word & known,
            Write::Changed { mask, value } => mask | value,
        };
        if written & !covered != 0 {
            return Err(BoardError::Register(RegisterError::MalformedSource {
                detail: format!(
                    "{}.{} is written at bits {:#010x}, which no `{}_*` field covers",
                    r.instance,
                    r.name,
                    written & !covered,
                    r.prefix
                ),
            }));
        }
    }
    Ok(chosen
        .into_values()
        .map(|(field, mask)| {
            let pos = mask.trailing_zeros();
            // The last write that touches the field decides its value.
            let value = r.writes.iter().rev().find_map(|w| match w {
                Write::Whole { word, known } => {
                    Some((known & mask == mask).then_some((word & mask) >> pos))
                }
                Write::Changed { mask: m, value } if m & mask != 0 => {
                    Some(Some((value & mask) >> pos))
                }
                Write::Changed { .. } => None,
            });
            FieldUse {
                family: r.family.to_owned(),
                register: format!("{}.{}", r.instance, r.name),
                offset: r.offset,
                field,
                pos: u8::try_from(pos).unwrap_or(u8::MAX),
                width: u8::try_from(mask.count_ones()).unwrap_or(u8::MAX),
                access: FieldAccess::Written,
                value: value.flatten(),
                whole_word,
            }
        })
        .collect())
}

/// A field's mask as its contiguous runs: the field itself when it is one
/// run, else `NAME[hi:lo]` / `NAME[k]` by the field's own bit numbers.
fn runs(name: &str, mask: u32) -> Vec<(String, u32)> {
    let mut out = Vec::new();
    let mut rest = mask;
    // The field's own bit number of each run's first bit, counting only its
    // bits.
    let mut index = 0u32;
    while rest != 0 {
        let start = rest.trailing_zeros();
        let len = (rest >> start).trailing_ones();
        let run = if len == 32 {
            !0
        } else {
            ((1u32 << len) - 1) << start
        };
        out.push((index, len, run));
        index += len;
        rest &= !run;
    }
    if out.len() == 1 {
        return vec![(name.to_owned(), mask)];
    }
    out.into_iter()
        .map(|(lo, len, run)| {
            let label = if len == 1 {
                format!("{name}[{lo}]")
            } else {
                format!("{name}[{}:{lo}]", lo + len - 1)
            };
            (label, run)
        })
        .collect()
}

/// The reset fields: [`clock_fields`]' (`Rcc::reset_clock` reads each and
/// requires its reset value, 0), then `FLASH_OPTSR_CUR.BOR_LEV`, whose value
/// is the declared level.
fn reset_rows(plan: &Plan, f: &RegisterFacts, bor_level: u8) -> Result<Vec<FieldUse>, BoardError> {
    let mut out = Vec::new();
    if plan.needs_clock() {
        let rcc = f.layout("RCC")?;
        for (register, fields) in clock_fields(plan) {
            let offset = rcc.offset(register, 0)?;
            for field in fields {
                let (pos, width) = f.field(&format!("RCC_{register}_{field}"))?;
                out.push(FieldUse {
                    family: "RCC".into(),
                    register: format!("RCC.{register}"),
                    offset,
                    field: field.into(),
                    pos: u8::try_from(pos).unwrap_or(u8::MAX),
                    width: u8::try_from(width).unwrap_or(u8::MAX),
                    access: FieldAccess::Reset,
                    value: Some(0),
                    whole_word: false,
                });
            }
        }
    }
    let (pos, width) = f.field("FLASH_OPTSR_BOR_LEV")?;
    out.push(FieldUse {
        family: "FLASH".into(),
        register: "FLASH.OPTSR_CUR".into(),
        offset: f.layout("FLASH")?.offset("OPTSR_CUR", 0)?,
        field: "BOR_LEV".into(),
        pos: u8::try_from(pos).unwrap_or(u8::MAX),
        width: u8::try_from(width).unwrap_or(u8::MAX),
        access: FieldAccess::Reset,
        value: Some(u32::from(bor_level)),
        whole_word: false,
    });
    Ok(out)
}
