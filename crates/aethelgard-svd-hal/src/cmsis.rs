//! Register facts for the STM32H753: base addresses, register offsets and bit
//! positions, from ST's CMSIS device header `stm32h753xx.h`
//! (`STMicroelectronics/cmsis-device-h7`, Apache-2.0), vendored byte for byte
//! and hash-checked before it is read (ruled on PR #42, R1).
//!
//! Nothing here is a C preprocessor. The parse reads exactly three shapes and
//! refuses anything else it is asked to resolve:
//! - `#define NAME VALUE`, where `VALUE` is a literal (`0x5400UL`, `21U`), another
//!   name, or a sum of those, in parentheses or not;
//! - `typedef struct { … } NAME_TypeDef;` whose every field is a `uint32_t`, an
//!   array of them, or reserved padding of the same type. Offsets come from
//!   field order, and every `Address offset:` comment must agree;
//! - the RCC enable bit of a peripheral, `RCC_<register>_<peripheral>EN_Pos`,
//!   which must name exactly one register;
//! - an interrupt number, the one `<NAME>_IRQn = <decimal>,` line inside
//!   `typedef enum { … } IRQn_Type;` (`AET-WO-2100`);
//! - a memory region's size, which the header states only in the comment of its
//!   base: `#define NAME (…) /*!< Base address of : <N>KB …` (`AET-WO-2100`).

use std::collections::BTreeMap;

pub const REPOSITORY: &str = "https://github.com/STMicroelectronics/cmsis-device-h7";
pub const COMMIT: &str = "8f922cdc7cc6de2344e75ddd657889f4ff761790";
pub const HEADER_FILE: &str = "stm32h753xx.h";
pub const HEADER_SHA256: &str = "4f6def439a9fa94705303fe227d34308048c44bd36263d311aa54cc94f8398f5";
const HEADER_BYTES: &[u8] = include_bytes!("../data/st-cmsis-device-h7/stm32h753xx.h");

/// How deep a chain of names may go before the parse refuses it.
const MAX_DEPTH: usize = 8;

/// One variant per refusal (`INV-06`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegisterError {
    #[error("vendored file `{file}` has sha256 {actual}, not the pinned {expected}")]
    SourceHashMismatch {
        file: &'static str,
        expected: &'static str,
        actual: String,
    },
    #[error("malformed register source: {detail}")]
    MalformedSource { detail: String },
    #[error("the register source defines no `{name}`")]
    Undefined { name: String },
}

impl RegisterError {
    /// The variant name.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::SourceHashMismatch { .. } => "SourceHashMismatch",
            Self::MalformedSource { .. } => "MalformedSource",
            Self::Undefined { .. } => "Undefined",
        }
    }
}

fn malformed<T>(detail: impl Into<String>) -> Result<T, RegisterError> {
    Err(RegisterError::MalformedSource {
        detail: detail.into(),
    })
}

/// One register of a block: its name, byte offset, and length in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub offset: u32,
    pub words: u32,
}

/// A register block's layout, from its `typedef struct`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    pub fields: Vec<Field>,
    /// The block's size in bytes: the end of its last field.
    pub size: u32,
}

impl Layout {
    /// The byte offset of `name`, or of word `index` of an array field.
    pub fn offset(&self, name: &str, index: u32) -> Result<u32, RegisterError> {
        let field = self
            .fields
            .iter()
            .find(|f| f.name == name)
            .ok_or_else(|| RegisterError::Undefined { name: name.into() })?;
        if index >= field.words {
            return malformed(format!(
                "`{name}[{index}]` is past its {} words",
                field.words
            ));
        }
        Ok(field.offset + 4 * index)
    }
}

/// The parsed header: every `#define`, and the source text for structs.
#[derive(Debug)]
pub struct RegisterFacts {
    text: String,
    defines: BTreeMap<String, Vec<String>>,
}

/// The facts from the vendored header, after its hash is checked.
pub fn register_facts() -> Result<RegisterFacts, RegisterError> {
    facts_from(HEADER_BYTES)
}

pub(crate) fn facts_from(bytes: &[u8]) -> Result<RegisterFacts, RegisterError> {
    let actual = crate::sha256_hex(bytes);
    if actual != HEADER_SHA256 {
        return Err(RegisterError::SourceHashMismatch {
            file: HEADER_FILE,
            expected: HEADER_SHA256,
            actual,
        });
    }
    parse(bytes)
}

/// The parse, without the hash check, so tests can feed it synthetic headers.
pub(crate) fn parse(bytes: &[u8]) -> Result<RegisterFacts, RegisterError> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return malformed(format!("`{HEADER_FILE}` is not UTF-8"));
    };
    let mut defines: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("#define") else {
            continue;
        };
        if !rest.starts_with([' ', '\t']) {
            continue;
        }
        let rest = strip_comment(rest).trim();
        let (name, value) = match rest.find(|c: char| c.is_whitespace()) {
            Some(i) => (&rest[..i], rest[i..].trim()),
            None => (rest, ""),
        };
        // Function-like macros (`NAME(x)`) are not values; they are left out.
        if name.contains('(') {
            continue;
        }
        defines
            .entry(name.to_owned())
            .or_default()
            .push(value.to_owned());
    }
    Ok(RegisterFacts {
        text: text.to_owned(),
        defines,
    })
}

fn strip_comment(s: &str) -> &str {
    let s = match s.find("/*") {
        Some(i) => &s[..i],
        None => s,
    };
    match s.find("//") {
        Some(i) => &s[..i],
        None => s,
    }
}

/// A literal: hex or decimal digits, then an optional `U`, `UL` or `ULL`.
fn literal(term: &str) -> Option<u64> {
    let digits = term
        .strip_suffix("ULL")
        .or_else(|| term.strip_suffix("UL"))
        .or_else(|| term.strip_suffix('U'))
        .unwrap_or(term);
    if let Some(hex) = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
    {
        if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        return u64::from_str_radix(hex, 16).ok();
    }
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

fn identifier(term: &str) -> bool {
    let mut bytes = term.bytes();
    matches!(bytes.next(), Some(b) if b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Strips one pair of parentheses that encloses the whole expression.
fn unwrap_parens(s: &str) -> &str {
    let s = s.trim();
    if !(s.starts_with('(') && s.ends_with(')')) {
        return s;
    }
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && i != s.len() - 1 {
                    return s;
                }
            }
            _ => {}
        }
    }
    &s[1..s.len() - 1]
}

impl RegisterFacts {
    /// The value of macro `name`: a literal, a name, or a sum of those.
    pub fn value(&self, name: &str) -> Result<u32, RegisterError> {
        let v = self.resolve(name, 0)?;
        u32::try_from(v).or_else(|_| malformed(format!("`{name}` is {v:#x}, wider than 32 bits")))
    }

    fn resolve(&self, name: &str, depth: usize) -> Result<u64, RegisterError> {
        if depth > MAX_DEPTH {
            return malformed(format!("`{name}` is more than {MAX_DEPTH} names deep"));
        }
        let values = self
            .defines
            .get(name)
            .ok_or_else(|| RegisterError::Undefined { name: name.into() })?;
        let [value] = values.as_slice() else {
            return malformed(format!("`{name}` is defined {} times", values.len()));
        };
        let mut expr = value.as_str();
        loop {
            let inner = unwrap_parens(expr);
            if inner == expr {
                break;
            }
            expr = inner;
        }
        if expr.is_empty() {
            return malformed(format!("`{name}` has no value"));
        }
        let mut sum: u64 = 0;
        for term in expr.split('+') {
            let term = unwrap_parens(term);
            let v = if let Some(v) = literal(term) {
                v
            } else if identifier(term) {
                self.resolve(term, depth + 1)?
            } else {
                return malformed(format!(
                    "`{name}` is `{value}`, which is not a literal, a name or a sum of them"
                ));
            };
            sum = sum
                .checked_add(v)
                .ok_or_else(|| RegisterError::MalformedSource {
                    detail: format!("`{name}` overflows"),
                })?;
        }
        Ok(sum)
    }

    /// The base address of peripheral `instance`, from `<instance>_BASE`.
    pub fn base(&self, instance: &str) -> Result<u32, RegisterError> {
        self.value(&format!("{instance}_BASE"))
    }

    /// The layout of `<kind>_TypeDef`, such as `GPIO` or `RCC`.
    pub fn layout(&self, kind: &str) -> Result<Layout, RegisterError> {
        let name = format!("{kind}_TypeDef");
        let close = format!("}} {name};");
        let lines: Vec<&str> = self.text.lines().collect();
        let ends: Vec<usize> = (0..lines.len())
            .filter(|&i| lines[i].trim() == close)
            .collect();
        let [end] = ends.as_slice() else {
            return if ends.is_empty() {
                Err(RegisterError::Undefined { name })
            } else {
                malformed(format!("`{name}` is defined {} times", ends.len()))
            };
        };
        let start = (0..*end)
            .rev()
            .find(|&i| lines[i].trim() == "typedef struct")
            .ok_or_else(|| RegisterError::MalformedSource {
                detail: format!("`{name}` has no `typedef struct`"),
            })?;
        if lines.get(start + 1).map(|l| l.trim()) != Some("{") {
            return malformed(format!(
                "`{name}`: `typedef struct` is not followed by `{{`"
            ));
        }
        let mut fields = Vec::new();
        let mut offset: u32 = 0;
        for line in &lines[start + 2..*end] {
            let code = strip_comment(line).trim();
            if code.is_empty() {
                continue;
            }
            let field = struct_field(code, &name)?;
            if let Some(stated) = stated_offset(line)
                && stated != offset
            {
                return malformed(format!(
                    "`{name}.{}` is at {offset:#x} by field order, but its comment says {stated:#x}",
                    field.0
                ));
            }
            let words = field.1;
            fields.push(Field {
                name: field.0,
                offset,
                words,
            });
            offset = words
                .checked_mul(4)
                .and_then(|b| offset.checked_add(b))
                .ok_or_else(|| RegisterError::MalformedSource {
                    detail: format!("`{name}` overflows"),
                })?;
        }
        if fields.is_empty() {
            return malformed(format!("`{name}` has no fields"));
        }
        Ok(Layout {
            fields,
            size: offset,
        })
    }

    /// The RCC register and bit that gate `peripheral`'s clock: the one
    /// `RCC_<register>_<peripheral>EN_Pos` that exists.
    pub fn enable_bit(&self, peripheral: &str) -> Result<(String, u32), RegisterError> {
        let suffix = format!("_{peripheral}EN_Pos");
        let found: Vec<&str> = self
            .defines
            .keys()
            .filter_map(|k| k.strip_prefix("RCC_")?.strip_suffix(suffix.as_str()))
            .filter(|reg| identifier(reg) && reg.ends_with("ENR"))
            .collect();
        let [register] = found.as_slice() else {
            return if found.is_empty() {
                Err(RegisterError::Undefined {
                    name: format!("RCC_*{suffix}"),
                })
            } else {
                malformed(format!(
                    "`{peripheral}`'s clock enable is in {} registers: {}",
                    found.len(),
                    found.join(", ")
                ))
            };
        };
        let bit = self.value(&format!("RCC_{register}{suffix}"))?;
        if bit > 31 {
            return malformed(format!("`{peripheral}`'s enable bit is {bit}"));
        }
        Ok(((*register).to_owned(), bit))
    }

    /// A bit position macro that must be under 32.
    pub fn position(&self, name: &str) -> Result<u32, RegisterError> {
        let v = self.value(name)?;
        if v > 31 {
            return malformed(format!("`{name}` is {v}, not a bit position"));
        }
        Ok(v)
    }

    /// The interrupt number of `name` (`SPI1` for `SPI1_IRQn`, `I2C1_EV` for
    /// `I2C1_EV_IRQn`): its one line inside the `IRQn_Type` enum.
    pub fn irq(&self, name: &str) -> Result<u32, RegisterError> {
        let Some(start) = self.text.find("typedef enum") else {
            return Err(RegisterError::Undefined {
                name: "IRQn_Type".into(),
            });
        };
        let Some(len) = self.text[start..].find("} IRQn_Type;") else {
            return Err(RegisterError::Undefined {
                name: "IRQn_Type".into(),
            });
        };
        let body = &self.text[start..start + len];
        let wanted = format!("{name}_IRQn");
        let mut found = Vec::new();
        for line in body.lines() {
            let code = strip_comment(line).trim();
            let Some((lhs, rhs)) = code.split_once('=') else {
                continue;
            };
            if lhs.trim() != wanted {
                continue;
            }
            let value = rhs.trim().strip_suffix(',').unwrap_or(rhs.trim()).trim();
            match value.parse::<u32>() {
                Ok(v) if value.bytes().all(|b| b.is_ascii_digit()) => found.push(v),
                _ => return malformed(format!("`{wanted}` is `{value}`, not a decimal number")),
            }
        }
        match found.as_slice() {
            [v] => Ok(*v),
            [] => Err(RegisterError::Undefined { name: wanted }),
            _ => malformed(format!("`{wanted}` is in the enum {} times", found.len())),
        }
    }

    /// The size in KiB that the comment of `#define name` states:
    /// `/*!< Base address of : <N>KB …`. The header has no size macro for its
    /// RAM regions, so this is the only place the size is written.
    pub fn region_kib(&self, name: &str) -> Result<u32, RegisterError> {
        let mut found = Vec::new();
        for line in self.text.lines() {
            let Some(rest) = line.trim_start().strip_prefix("#define") else {
                continue;
            };
            if rest.split_whitespace().next() != Some(name) {
                continue;
            }
            let Some(comment) = rest.split_once("/*!<").map(|(_, c)| c.trim_start()) else {
                return malformed(format!("`{name}` has no comment stating its size"));
            };
            let Some(size) = comment.strip_prefix("Base address of : ") else {
                return malformed(format!(
                    "`{name}`'s comment does not start `Base address of : `"
                ));
            };
            let digits: String = size.chars().take_while(char::is_ascii_digit).collect();
            let kib = match (
                digits.parse::<u32>(),
                size[digits.len()..].starts_with("KB "),
            ) {
                (Ok(k), true) if k > 0 => k,
                _ => return malformed(format!("`{name}`'s comment states no `<N>KB ` size")),
            };
            found.push(kib);
        }
        match found.as_slice() {
            [k] => Ok(*k),
            [] => Err(RegisterError::Undefined { name: name.into() }),
            _ => malformed(format!("`{name}` is defined {} times", found.len())),
        }
    }
}

/// `[qualifier] uint32_t NAME;` or `[qualifier] uint32_t NAME[N];`.
fn struct_field(code: &str, typedef: &str) -> Result<(String, u32), RegisterError> {
    let Some(decl) = code.strip_suffix(';') else {
        return malformed(format!("`{typedef}`: `{code}` is not a field"));
    };
    let mut tokens: Vec<&str> = decl.split_whitespace().collect();
    if matches!(
        tokens.first(),
        Some(&("__IO" | "__I" | "__O" | "__IM" | "__OM" | "__IOM"))
    ) {
        tokens.remove(0);
    }
    let [ty, name] = tokens.as_slice() else {
        return malformed(format!("`{typedef}`: `{code}` is not one field"));
    };
    if *ty != "uint32_t" {
        return malformed(format!(
            "`{typedef}`: `{code}` is a `{ty}`, and only `uint32_t` is read"
        ));
    }
    let (name, words) = match name.split_once('[') {
        Some((n, rest)) => {
            let Some(count) = rest.strip_suffix(']').and_then(literal) else {
                return malformed(format!("`{typedef}`: `{code}` has a bad array length"));
            };
            let count = u32::try_from(count)
                .ok()
                .filter(|c| *c > 0)
                .ok_or_else(|| RegisterError::MalformedSource {
                    detail: format!("`{typedef}`: `{code}` has a bad array length"),
                })?;
            (n, count)
        }
        None => (*name, 1),
    };
    if !identifier(name) {
        return malformed(format!("`{typedef}`: `{name}` is not a field name"));
    }
    Ok((name.to_owned(), words))
}

/// The first hex number after `Address offset:` in a field's comment.
fn stated_offset(line: &str) -> Option<u32> {
    let rest = line.split_once("Address offset:")?.1.trim_start();
    let hex = rest
        .strip_prefix("0x")
        .or_else(|| rest.strip_prefix("0X"))?;
    let end = hex
        .find(|c: char| !c.is_ascii_hexdigit())
        .unwrap_or(hex.len());
    u32::from_str_radix(&hex[..end], 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> RegisterFacts {
        register_facts().unwrap()
    }

    /// The offsets the S1 probe used (ruled on PR #42, R1), read from the
    /// vendored header. The spike checked the same numbers against RM0433 and
    /// `stm32-metapac` 21.0.0.
    #[test]
    fn the_probe_offsets_come_out_of_the_header() {
        let f = facts();
        let rcc = f.layout("RCC").unwrap();
        assert_eq!(rcc.offset("AHB4ENR", 0).unwrap(), 0x0E0);
        assert_eq!(rcc.offset("APB1LENR", 0).unwrap(), 0x0E8);
        assert_eq!(rcc.size, 0x130);
        let gpio = f.layout("GPIO").unwrap();
        assert_eq!(gpio.offset("MODER", 0).unwrap(), 0x00);
        assert_eq!(gpio.offset("AFR", 0).unwrap(), 0x20);
        assert_eq!(gpio.offset("AFR", 1).unwrap(), 0x24);
        assert_eq!(gpio.size, 0x28);
        let i2c = f.layout("I2C").unwrap();
        assert_eq!(i2c.offset("CR1", 0).unwrap(), 0x00);
        assert_eq!(i2c.offset("TIMINGR", 0).unwrap(), 0x10);
        assert_eq!(i2c.offset("TXDR", 0).unwrap(), 0x28);
        assert_eq!(i2c.size, 0x2C);
        // The rest the generator emits for sentinel-01 (adversarial pass: these
        // were read correctly but not pinned).
        let spi = f.layout("SPI").unwrap();
        assert_eq!(spi.offset("CR1", 0).unwrap(), 0x00);
        assert_eq!(spi.offset("CFG2", 0).unwrap(), 0x0C);
        assert_eq!(spi.offset("TXDR", 0).unwrap(), 0x20);
        assert_eq!(spi.size, 0x54);
        assert_eq!(rcc.offset("APB2ENR", 0).unwrap(), 0x0F0);
        assert_eq!(gpio.offset("BSRR", 0).unwrap(), 0x18);
        assert_eq!(f.position("I2C_CR1_PE_Pos").unwrap(), 0);
        assert_eq!(f.position("SPI_CR1_SPE_Pos").unwrap(), 0);
        assert_eq!(f.position("SPI_CFG2_MASTER_Pos").unwrap(), 22);
    }

    #[test]
    fn bases_and_clock_gates() {
        let f = facts();
        assert_eq!(f.base("RCC").unwrap(), 0x5802_4400);
        assert_eq!(f.base("GPIOA").unwrap(), 0x5802_0000);
        assert_eq!(f.base("GPIOB").unwrap(), 0x5802_0400);
        assert_eq!(f.base("I2C1").unwrap(), 0x4000_5400);
        assert_eq!(f.base("SPI1").unwrap(), 0x4001_3000);
        assert_eq!(f.enable_bit("GPIOB").unwrap(), ("AHB4ENR".into(), 1));
        assert_eq!(f.enable_bit("I2C1").unwrap(), ("APB1LENR".into(), 21));
        assert_eq!(f.enable_bit("SPI1").unwrap(), ("APB2ENR".into(), 12));
        // An alias chain: GPIO_MODER_MODE6_Pos is GPIO_MODER_MODER6_Pos.
        assert_eq!(f.position("GPIO_MODER_MODE6_Pos").unwrap(), 12);
        assert_eq!(f.position("GPIO_AFRH_AFSEL8_Pos").unwrap(), 0);
    }

    #[test]
    fn a_changed_header_is_refused_before_it_is_read() {
        let mut bytes = HEADER_BYTES.to_vec();
        bytes.push(b'\n');
        let err = facts_from(&bytes).unwrap_err();
        assert_eq!(err.kind(), "SourceHashMismatch");
    }

    fn synthetic(text: &str) -> RegisterFacts {
        parse(text.as_bytes()).unwrap()
    }

    #[test]
    fn values_outside_the_three_shapes_are_refused() {
        let f = synthetic(
            "#define A (0x10UL)\n#define B (A + 0x4UL)\n#define C (0x1UL << A)\n\
             #define D 1U\n#define D 2U\n#define E F\n#define F E\n#define G\n",
        );
        assert_eq!(f.value("B").unwrap(), 0x14);
        assert_eq!(f.value("C").unwrap_err().kind(), "MalformedSource");
        assert_eq!(f.value("D").unwrap_err().kind(), "MalformedSource");
        assert_eq!(f.value("E").unwrap_err().kind(), "MalformedSource");
        assert_eq!(f.value("G").unwrap_err().kind(), "MalformedSource");
        assert_eq!(f.value("H").unwrap_err().kind(), "Undefined");
        assert_eq!(
            synthetic("#define P (32U)\n")
                .position("P")
                .unwrap_err()
                .kind(),
            "MalformedSource"
        );
    }

    #[test]
    fn a_struct_is_read_from_field_order_and_its_comments_must_agree() {
        let ok = "typedef struct\n{\n  __IO uint32_t A; /*!< Address offset: 0x00 */\n  \
                  uint32_t R[2];\n  __IO uint32_t B; /*!< Address offset: 0x0C */\n} X_TypeDef;\n";
        let layout = synthetic(ok).layout("X").unwrap();
        assert_eq!(layout.offset("B", 0).unwrap(), 0x0C);
        assert_eq!(layout.size, 0x10);
        assert_eq!(layout.offset("R", 2).unwrap_err().kind(), "MalformedSource");
        let disagrees = ok.replace("0x0C", "0x10");
        assert_eq!(
            synthetic(&disagrees).layout("X").unwrap_err().kind(),
            "MalformedSource"
        );
        let narrow = ok.replace("__IO uint32_t B", "__IO uint16_t B");
        assert_eq!(
            synthetic(&narrow).layout("X").unwrap_err().kind(),
            "MalformedSource"
        );
        let twice = format!("{ok}{ok}");
        assert_eq!(
            synthetic(&twice).layout("X").unwrap_err().kind(),
            "MalformedSource"
        );
        assert_eq!(synthetic(ok).layout("Y").unwrap_err().kind(), "Undefined");
    }

    #[test]
    fn a_clock_gate_must_be_in_exactly_one_register() {
        let f = synthetic("#define RCC_AENR_X1EN_Pos (3U)\n#define RCC_BENR_X1EN_Pos (4U)\n");
        assert_eq!(f.enable_bit("X1").unwrap_err().kind(), "MalformedSource");
        assert_eq!(f.enable_bit("X2").unwrap_err().kind(), "Undefined");
    }

    #[test]
    fn interrupts_and_region_sizes_come_out_of_the_header() {
        let f = facts();
        assert_eq!(f.irq("SPI1").unwrap(), 35);
        assert_eq!(f.irq("SPI4").unwrap(), 84);
        assert_eq!(f.irq("I2C1_EV").unwrap(), 31);
        assert_eq!(f.irq("I2C1_ER").unwrap(), 32);
        assert_eq!(f.irq("SPI9").unwrap_err().kind(), "Undefined");
        assert_eq!(f.region_kib("D1_DTCMRAM_BASE").unwrap(), 128);
        assert_eq!(f.value("D1_DTCMRAM_BASE").unwrap(), 0x2000_0000);
        assert_eq!(f.value("FLASH_BANK1_BASE").unwrap(), 0x0800_0000);
        assert_eq!(f.value("FLASH_BANK2_BASE").unwrap(), 0x0810_0000);
        assert_eq!(f.value("FLASH_END").unwrap(), 0x081F_FFFF);
    }

    #[test]
    fn an_interrupt_or_a_size_outside_its_shape_is_refused() {
        let e = |body: &str| synthetic(&format!("typedef enum\n{{\n{body}\n}} IRQn_Type;\n"));
        assert_eq!(e("  A_IRQn = 7, /*!< x */").irq("A").unwrap(), 7);
        assert_eq!(
            e("  A_IRQn = 0x7,").irq("A").unwrap_err().kind(),
            "MalformedSource"
        );
        assert_eq!(
            e("  A_IRQn = -1,").irq("A").unwrap_err().kind(),
            "MalformedSource"
        );
        assert_eq!(
            e("  A_IRQn = 1,\n  A_IRQn = 2,")
                .irq("A")
                .unwrap_err()
                .kind(),
            "MalformedSource"
        );
        // Outside the enum, a matching line is not read.
        let outside = synthetic("  A_IRQn = 7,\ntypedef enum\n{\n} IRQn_Type;\n");
        assert_eq!(outside.irq("A").unwrap_err().kind(), "Undefined");
        assert_eq!(synthetic("").irq("A").unwrap_err().kind(), "Undefined");

        let r = |line: &str| synthetic(line).region_kib("R_BASE");
        assert_eq!(
            r("#define R_BASE (0x1UL) /*!< Base address of : 64KB of RAM */").unwrap(),
            64
        );
        for bad in [
            "#define R_BASE (0x1UL)",
            "#define R_BASE (0x1UL) /*!< Base address of : (up to 1 MB) flash */",
            "#define R_BASE (0x1UL) /*!< Base address of : 0KB x */",
            "#define R_BASE (0x1UL) /*!< Base of : 64KB x */",
            "#define R_BASE (0x1UL) /*!< Base address of : 64KB */\n#define R_BASE (0x2UL) /*!< Base address of : 64KB */",
        ] {
            assert_eq!(r(bad).unwrap_err().kind(), "MalformedSource", "{bad}");
        }
        assert_eq!(
            r("#define R_BASEX (0x1UL)").unwrap_err().kind(),
            "Undefined"
        );
    }
}
