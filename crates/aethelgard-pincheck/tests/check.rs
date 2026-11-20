//! Tier 1: the reader, the cross-check and the three directions, on synthetic
//! ELFs (`AET-WO-4400a` §1.4). The real ArduPilot ELFs are judged in the
//! `pinout` CI job, never committed.

mod common;

use aethelgard_pincheck::{AllowList, Mode, Refusal, elf, header, judge};
use common::{Elf, allow, lqfp_like, step_g_defaults, step_g_words, topology};

fn elf_kind(e: &Elf) -> &'static str {
    elf::read(&e.bytes()).unwrap_err().kind()
}

/// The findings as `check kind pin net decoded`.
fn findings(words: [[u32; 7]; 11], allow: &AllowList) -> Vec<String> {
    let config = elf::read(
        &Elf {
            words,
            ..Elf::default()
        }
        .bytes(),
    )
    .unwrap();
    let j = judge(&topology(), &lqfp_like(), &config, None, allow).unwrap();
    assert_eq!(j.ok, j.findings.is_empty());
    j.findings
        .iter()
        .map(|f| {
            format!(
                "{} {} {} {} {}",
                f.check,
                f.kind,
                f.pin,
                f.net.as_deref().unwrap_or("-"),
                f.decoded
            )
        })
        .collect()
}

#[test]
fn the_decoder_reads_the_spikes_hand_decode() {
    let config = elf::read(&Elf::default().bytes()).unwrap();
    assert_eq!(
        config.ports[0].moder, 0x0000_A801,
        "GPIOA MODER, AET-WO-4400 FEEDBACK §5"
    );
    assert_eq!(config.ports[0].afrl, 0x5550_0000);
    let pin = |n| aethelgard_pincheck::PinId::parse(n).unwrap();
    assert_eq!(config.pin(pin("PA0")), (Mode::Output, 0));
    assert_eq!(config.pin(pin("PA5")), (Mode::Alternate, 5));
    assert_eq!(config.pin(pin("PA13")), (Mode::Input, 0));
    assert_eq!(config.pin(pin("PD9")), (Mode::Alternate, 7));
}

#[test]
fn step_g_passes_with_its_declared_allow_list() {
    assert_eq!(findings(step_g_words(), &allow()), Vec::<String>::new());
    let config = elf::read(&Elf::default().bytes()).unwrap();
    let j = judge(&topology(), &lqfp_like(), &config, None, &allow()).unwrap();
    let named: Vec<String> = j
        .configured
        .iter()
        .map(|c| format!("{} {}", c.pin, c.named_by.as_deref().unwrap_or("-")))
        .collect();
    assert_eq!(
        named,
        [
            "PA0 topology FLASH_CS",
            "PA5 topology SPI_SCK",
            "PA6 topology SPI_MISO",
            "PA7 topology SPI_MOSI",
            "PB6 topology I2C_SCL",
            "PB7 topology I2C_SDA",
            "PD8 allow-list",
            "PD9 allow-list"
        ]
    );
}

#[test]
fn without_the_allow_list_the_usart_is_named_unnamed() {
    assert_eq!(
        findings(step_g_words(), &AllowList::default()),
        [
            "reverse UnnamedPin PD8 - alternate AF7",
            "reverse UnnamedPin PD9 - alternate AF7"
        ]
    );
}

#[test]
fn each_seed_fails_by_name() {
    // seed-pin: I2C1_SCL on PB8 (AF4) instead of PB6.
    let mut w = step_g_words();
    w[1][0] = 0x0002_8000;
    w[1][5] = 0x4000_0000;
    w[1][6] = 0x0000_0004;
    assert_eq!(
        findings(w, &allow()),
        [
            "forward NotAlternate PB6 I2C_SCL input",
            "reverse UnnamedPin PB8 - alternate AF4"
        ]
    );
    // seed-af: PB6/PB7 on AF6 (I2C4).
    let mut w = step_g_words();
    w[1][5] = 0x6600_0000;
    assert_eq!(
        findings(w, &allow()),
        [
            "forward WrongAlternateFunction PB6 I2C_SCL alternate AF6",
            "forward WrongAlternateFunction PB7 I2C_SDA alternate AF6"
        ]
    );
    // seed-pkg: PI0 an output; LQFP144 has no port I pins in this table.
    let mut w = step_g_words();
    w[8][0] = 0x0000_0001;
    assert_eq!(
        findings(w, &allow()),
        [
            "reverse UnnamedPin PI0 - output",
            "package NotInPackage PI0 - output"
        ]
    );
}

#[test]
fn a_gpio_that_is_not_an_output_and_an_af_on_an_input_pin_fail() {
    let mut w = step_g_words();
    w[0][0] = 0x0000_A800; // PA0 input
    w[1][0] = 0x0000_8000; // PB6 input, though AFRL still says AF4
    assert_eq!(
        findings(w, &allow()),
        [
            "forward NotOutput PA0 FLASH_CS input",
            "forward NotAlternate PB6 I2C_SCL input"
        ]
    );
}

#[test]
fn reset_states_are_not_configured_but_driving_a_debug_pin_is() {
    // Every pin analog, PA13-15 and PB3-4 alternate AF0: RM0433's reset values.
    let mut w = step_g_words();
    w[0][0] |= 0xA800_0000;
    w[1][0] |= 0x0000_0280;
    for port in [2, 4, 5, 6, 7, 8, 9, 10] {
        w[port][0] = 0xFFFF_FFFF;
    }
    w[3][0] = 0xFFFA_FFFF; // port D analog but for PD8 and PD9
    assert_eq!(findings(w, &allow()), Vec::<String>::new());
    // PA13 on AF1 is not its reset state.
    w[0][6] = 0x0010_0000;
    assert_eq!(
        findings(w, &allow()),
        ["reverse UnnamedPin PA13 - alternate AF1"]
    );
}

#[test]
fn an_allowed_pin_must_be_exactly_as_declared() {
    let mut w = step_g_words();
    w[3][6] = 0x0000_0078; // PD8 AF8
    assert_eq!(
        findings(w, &allow()),
        ["reverse AllowedPinDiffers PD8 - alternate AF8"]
    );
    let mut w = step_g_words();
    w[3][0] = 0x0008_0000; // PD8 input
    assert_eq!(
        findings(w, &allow()),
        ["reverse AllowedPinDiffers PD8 - input"]
    );
}

#[test]
fn the_allow_list_cannot_hide_a_topology_pin() {
    let hide = br#"{"allow":{"PB6":{"mode":"alternate","alternate_function":6,"why":"hide"}}}"#;
    let config = elf::read(&Elf::default().bytes()).unwrap();
    let list = AllowList::parse(hide).unwrap();
    let err = judge(&topology(), &lqfp_like(), &config, None, &list).unwrap_err();
    assert_eq!(err.kind(), "AllowNamesTopologyPin");
}

#[test]
fn a_malformed_allow_list_is_refused() {
    for bad in [
        &br#"{"allow":{"PD8":{"mode":"alternate","why":"no af"}}}"#[..],
        br#"{"allow":{"PD8":{"mode":"output","alternate_function":7,"why":"af on output"}}}"#,
        br#"{"allow":{"PD8":{"mode":"input","why":"input is never configured"}}}"#,
        br#"{"allow":{"PD8":{"mode":"output","why":" "}}}"#,
        br#"{"allow":{"PD08":{"mode":"output","why":"not canonical"}}}"#,
        br#"{"allow":{"PL1":{"mode":"output","why":"no port L"}}}"#,
        br#"{"allow":{"PD8":{"mode":"output","why":"a"},"PD8":{"mode":"output","why":"b"}}}"#,
        br#"{"allow":{},"extra":1}"#,
        br#"{"allow":{"PD8":{"mode":"alternate","alternate_function":16,"why":"af 16"}}}"#,
    ] {
        let err = AllowList::parse(bad).unwrap_err();
        assert_eq!(
            err.kind(),
            "MalformedAllowList",
            "{}",
            String::from_utf8_lossy(bad)
        );
    }
}

#[test]
fn every_malformed_elf_is_refused_by_name() {
    let bytes = Elf::default().bytes();
    assert_eq!(elf::read(&bytes[..40]).unwrap_err().kind(), "Truncated");
    assert_eq!(
        elf::read(&bytes[..bytes.len() - 1]).unwrap_err().kind(),
        "Truncated"
    );
    assert_eq!(
        elf_kind(&Elf {
            class: 2,
            ..Elf::default()
        }),
        "WrongClass"
    );
    assert_eq!(
        elf_kind(&Elf {
            data: 2,
            ..Elf::default()
        }),
        "WrongByteOrder"
    );
    assert_eq!(
        elf_kind(&Elf {
            machine: 62,
            ..Elf::default()
        }),
        "WrongMachine"
    );
    assert_eq!(
        elf_kind(&Elf {
            sym_size: 304,
            ..Elf::default()
        }),
        "SymbolSize"
    );
    assert_eq!(
        elf_kind(&Elf {
            duplicate: true,
            ..Elf::default()
        }),
        "SymbolDuplicated"
    );
    assert_eq!(
        elf_kind(&Elf {
            function: true,
            ..Elf::default()
        }),
        "SymbolNotObject"
    );
    assert_eq!(
        elf_kind(&Elf {
            shndx: 0,
            ..Elf::default()
        }),
        "SymbolOutsideSection"
    );
    assert_eq!(
        elf_kind(&Elf {
            shndx: 2,
            ..Elf::default()
        }),
        "SymbolOutsideSection"
    );
    assert_eq!(
        elf_kind(&Elf {
            sym_value: common::ADDR + 4,
            ..Elf::default()
        }),
        "SymbolOutsideSection"
    );
    // The section table says .rodata is elsewhere in the file than the PT_LOAD
    // that is flashed: refused, never read from either.
    let lie = Elf {
        section_offset: Some(0),
        ..Elf::default()
    };
    assert_eq!(elf_kind(&lie), "SectionTableDisagrees");
    assert_eq!(
        elf_kind(&Elf {
            etype: 4,
            ..Elf::default()
        }),
        "WrongType"
    );
    // Adversarial A2, A5: what reset reads must be what was flashed there.
    let moved = Elf {
        paddr: Some(common::ADDR + 0x1000),
        ..Elf::default()
    };
    assert_eq!(elf_kind(&moved), "SymbolNotInPlace");
    assert_eq!(
        elf_kind(&Elf {
            overlay: true,
            ..Elf::default()
        }),
        "SegmentsOverlap"
    );
    assert_eq!(
        elf_kind(&Elf {
            writable: true,
            ..Elf::default()
        }),
        "SymbolOutsideSection"
    );
    let mut renamed = bytes.clone();
    let at = renamed
        .windows(19)
        .position(|w| w == b"gpio_default_config")
        .unwrap();
    renamed[at] = b'G';
    assert_eq!(elf::read(&renamed).unwrap_err().kind(), "SymbolMissing");
    let mut not_elf = bytes;
    not_elf[0] = 0;
    assert_eq!(elf::read(&not_elf).unwrap_err().kind(), "NotElf");
}

#[test]
fn the_header_cross_check_agrees_and_refuses_any_disagreement() {
    let words = step_g_words();
    let config = elf::read(&Elf::default().bytes()).unwrap();
    let text = common::header(&words);
    let h = header::read(&text).unwrap();
    let j = judge(&topology(), &lqfp_like(), &config, Some(&h), &allow()).unwrap();
    assert!(j.ok);
    assert_eq!(j.decodes, "elf and hwdef.h agree");

    let mut other = words;
    other[1][5] = 0x6600_0000;
    let h = header::read(&common::header(&other)).unwrap();
    let err = judge(&topology(), &lqfp_like(), &config, Some(&h), &allow()).unwrap_err();
    assert_eq!(err.kind(), "DecodesDisagree");
    assert_eq!(err.id().as_deref(), Some("GPIOB_AFRL"));
    assert!(matches!(
        err,
        Refusal::DecodesDisagree {
            elf: 0x4400_0000,
            header: 0x6600_0000,
            ..
        }
    ));
}

#[test]
fn a_malformed_header_is_refused_by_macro() {
    let text = common::header(&step_g_words());
    let refused = |t: &str| header::read(t).unwrap_err();
    let missing = text.replace("#define VAL_GPIOK_AFRH", "#define VAL_GPIOK_AFRX");
    assert_eq!(refused(&missing).id().as_deref(), Some("VAL_GPIOK_AFRH"));
    let twice = format!("{text}#undef VAL_GPIOA_MODER\n");
    assert_eq!(refused(&twice).id().as_deref(), Some("VAL_GPIOA_MODER"));
    let short = text.replacen("PIN_MODE_INPUT(15U)", "PIN_MODE_INPUT(14U)", 1);
    assert_eq!(refused(&short).kind(), "MalformedHeader");
    let odd = text.replacen("PIN_MODE_INPUT(1U)", "PIN_MODE_INPUT(1U) | 0x100", 1);
    assert_eq!(refused(&odd).kind(), "MalformedHeader");
    let wrong_half = text.replacen("PIN_AFIO_AF(8U,", "PIN_AFIO_AF(7U,", 1);
    assert_eq!(refused(&wrong_half).kind(), "MalformedHeader");
}

/// Review 2, A15.1: each symbol's name is compared in at most
/// `SYMBOL.len() + 1` bytes, so 400,000 symbols naming one 8 MiB name read in
/// moments (a scan to the NUL per symbol was about 3 × 10^12 steps); a name
/// that only starts with the symbol's is not it; and a string table that does
/// not end in NUL is refused, since no name in it is then known terminated.
#[test]
fn a_crowd_of_long_names_reads_in_bounded_time() {
    let start = std::time::Instant::now();
    let config = elf::read(
        &Elf {
            crowd: Some((400_000, 8 << 20)),
            ..Elf::default()
        }
        .bytes(),
    );
    assert!(config.is_ok(), "{:?}", config.err().map(|e| e.kind()));
    assert!(start.elapsed().as_secs() < 30, "{:?}", start.elapsed());
    assert_eq!(
        elf_kind(&Elf {
            unterminated: true,
            ..Elf::default()
        }),
        "MalformedElf"
    );
}

/// AET-WO-1603 §1.4: the reader sees what the preprocessor sees. `#if 0` and
/// `#if 1` are evaluated exactly: a definition in a dead branch is no
/// definition, one in a live branch is. Any other conditional around one of
/// the 33 macros is refused, `HwdefConditionalUnsupported`, never read
/// through, whatever spelling the directive takes.
#[test]
fn a_definition_inside_a_conditional_is_evaluated_exactly_or_refused() {
    let text = common::header(&step_g_words());
    let real = header::read(&text).unwrap();
    let start = text.find("#define VAL_GPIOA_MODER").unwrap();
    let end = start + text[start..].find("\n\n").unwrap() + 1;
    let (head, def, tail) = (&text[..start], &text[start..end], &text[end..]);
    let wrapped = |open: &str, close: &str| format!("{head}{open}\n{def}{close}\n{tail}");
    let refused = |t: &str| header::read(t).unwrap_err();

    // Dead: the only definition is gone.
    let dead = refused(&wrapped("#if 0", "#endif"));
    assert_eq!(dead.kind(), "MalformedHeader");
    assert_eq!(dead.id().as_deref(), Some("VAL_GPIOA_MODER"));
    // Live, exactly: `#if 1`, the `#else` of `#if 0`, an `#elif 1` after it.
    for (open, close) in [
        ("#if 1", "#endif"),
        ("#if 0\n#else", "#endif"),
        ("#if 0\n#elif 1", "#endif"),
        ("#if 0\n#elif 0\n#else", "#endif"),
    ] {
        assert_eq!(header::read(&wrapped(open, close)).unwrap(), real, "{open}");
    }
    // A decoy in a dead branch beside the real definition is not a second one.
    let decoy = format!(
        "#if 0\n{}#endif\n{text}",
        def.replace("INPUT(0U)", "OUTPUT(0U)")
    );
    assert_eq!(header::read(&decoy).unwrap(), real);
    // An `#elif 0` is dead even after a branch the reader cannot evaluate.
    let elif0 = format!(
        "#ifdef X\n#elif 0\n{}#endif\n{text}",
        def.replace("INPUT(0U)", "OUTPUT(0U)")
    );
    assert_eq!(header::read(&elif0).unwrap(), real);
    // Dead after a taken branch, whatever its own condition says.
    let after = refused(&wrapped("#if 1\n#else", "#endif"));
    assert_eq!(after.id().as_deref(), Some("VAL_GPIOA_MODER"));
    assert_eq!(after.kind(), "MalformedHeader");

    // Anything else around one of the 33 is refused by name.
    for (open, groups) in [
        ("#ifdef STM32H7", 1),
        ("#ifndef STM32H7", 1),
        ("#if defined(STM32H7)", 1),
        ("#if 2", 1),
        ("#if (1)", 1),
        ("#if 0x1", 1),
        ("#if 0\n#elif STM32H7", 1),
        ("#if 0\n#elifdef X", 1),
        ("#ifdef A\n#else", 1),
        ("# if X", 1),
        ("#\tifdef X", 1),
        ("  #  ifndef X", 1),
        ("%:ifdef X", 1),
        ("#if 1\n#if X", 2),
        ("#if X\n#if 1", 2),
    ] {
        let e = refused(&wrapped(open, &vec!["#endif"; groups].join("\n")));
        assert_eq!(e.kind(), "HwdefConditionalUnsupported", "{open}: {e}");
        assert_eq!(e.id().as_deref(), Some("VAL_GPIOA_MODER"), "{open}");
    }
    // A region that holds none of the 33 is not read, so not refused: the
    // shape of ArduPilot's generated header (step-g L7-9, L180-184, L1447-1455).
    let others = format!(
        "#ifndef TRUE\n#define TRUE 1\n#endif\n\
         #if defined(STM32_I2C_I2C1_RX_DMA_STREAM) && defined(STM32_I2C_I2C1_TX_DMA_STREAM)\n#define HAL_I2C1_CONFIG {{ 1 }}\n#else\n#define HAL_I2C1_CONFIG {{ 0 }}\n#endif\n\
         {text}#ifndef AP_TERRAIN_AVAILABLE\n#define AP_TERRAIN_AVAILABLE 0\n#endif\n#if AP_TERRAIN_AVAILABLE\n#ifndef D\n#define D \"x\"\n#endif\n#endif\n"
    );
    assert_eq!(header::read(&others).unwrap(), real);
}

/// AET-WO-1603 §1.4 and §5: continuations are spliced and comments removed
/// before a directive is recognised, as GCC does: a directive inside a
/// comment, or carried onto a comment's line by a backslash, is no
/// directive; a definition swallowed by a directive's continuation is no
/// definition; and the structure must balance.
#[test]
fn a_directive_is_recognised_only_where_the_preprocessor_sees_one() {
    let text = common::header(&step_g_words());
    let real = header::read(&text).unwrap();
    let refused = |t: &str| header::read(t).unwrap_err();
    // A name in a comment is no mention; a directive in a line comment is no
    // directive.
    let noted = format!("// VAL_GPIOA_MODER is below. #if 0\n{text}");
    assert_eq!(header::read(&noted).unwrap(), real);
    // In a block comment: no region opens.
    assert_eq!(
        header::read(&format!("/*\n#if 0\n*/\n{text}")).unwrap(),
        real
    );
    // A line comment ended by a backslash swallows the next line: here the
    // `#if 0`, so the `#endif` below has nothing to close.
    let e = refused(&format!("// note \\\n#if 0\n{text}#endif\n"));
    assert_eq!(e.kind(), "MalformedHeader", "{e}");
    // A backslash then blanks, which GCC splices with a warning, is refused
    // wherever it is (4400a adversarial case 73).
    let e = refused(&format!("// note \\ \t\n#if 0\n{text}#endif\n"));
    assert_eq!(e.kind(), "MalformedHeader", "{e}");
    assert!(
        e.to_string().contains("backslash followed by blanks"),
        "{e}"
    );
    let blank = text.replacen("PIN_MODE_INPUT(1U) | \\\n", "PIN_MODE_INPUT(1U) | \\ \n", 1);
    assert_ne!(blank, text);
    assert_eq!(refused(&blank).id().as_deref(), Some("line 4"));
    // A `#define` carried onto an `#ifdef` line by a backslash is part of its
    // condition, so no definition (GCC: extra tokens after `#ifdef`).
    let start = text.find("#define VAL_GPIOA_MODER").unwrap();
    let hidden = format!("{}#ifdef X \\\n{}#endif\n", &text[..start], &text[start..]);
    let e = refused(&hidden);
    assert_eq!(e.kind(), "MalformedHeader", "{e}");
    assert_eq!(e.id().as_deref(), Some("VAL_GPIOA_MODER"));
    // A `#if 0` carried into another definition's value is no directive.
    let carried = format!("#define OTHER 1 \\\n#if 0\n{text}");
    assert_eq!(header::read(&carried).unwrap(), real);
    // A definition only inside a comment is no definition.
    let commented = text.replacen("#define VAL_GPIOA_MODER", "/*\n#define VAL_GPIOA_MODER", 1);
    let commented = commented.replacen(
        "\n\n#define VAL_GPIOA_AFRL",
        "\n*/\n\n#define VAL_GPIOA_AFRL",
        1,
    );
    assert_eq!(refused(&commented).id().as_deref(), Some("VAL_GPIOA_MODER"));
    // Unbalanced or unterminated structure is refused.
    for bad in [
        format!("{text}#if 1\n"),
        format!("{text}#endif\n"),
        format!("{text}#else\n"),
        format!("#if 0\n#else\n#else\n#endif\n{text}"),
        format!("#if 0\n#else\n#elif 1\n#endif\n{text}"),
        format!("{text}/* open"),
        format!("{text}#define LAST 1 \\"),
    ] {
        assert_eq!(refused(&bad).kind(), "MalformedHeader", "{bad:.40}");
    }
    // CRLF line ends are line ends.
    assert_eq!(header::read(&text.replace('\n', "\r\n")).unwrap(), real);
}

/// AET-WO-1603 adversarial A3 and A8: what the reader does not model is
/// refused, never read: a byte outside printable ASCII and tab (a lone CR,
/// which GCC takes as a line end; U+00A0 and U+0085, which Rust trims and GCC
/// keeps), a trigraph, a C++ raw string, `__has_include`'s header-name, and
/// another file or line number (`#include`, `#line`). A comment spanning
/// lines is one space, so what follows its end continues the line it began
/// on: a `#if 0` there is no directive.
#[test]
fn what_the_reader_does_not_model_is_refused() {
    let text = common::header(&step_g_words());
    let real = header::read(&text).unwrap();
    for bad in [
        "// x\r#if 0\n",
        "#define N\u{a0}1\n",
        "#define N\u{85}1\n",
        "// ??/\n",
        "#define S R\"(x)\"\n",
        "#if __has_include(<a.h>)\n#endif\n",
        "#include <hal.h>\n",
        "#line 10\n",
        "#define T 1\x0c\n",
    ] {
        let e = header::read(&format!("{bad}{text}")).unwrap_err();
        assert_eq!(e.kind(), "MalformedHeader", "{bad:?}: {e}");
        assert_eq!(e.id().as_deref(), Some("line 1"), "{bad:?}: {e}");
    }
    let joined = format!("#define X /*\n*/ #if 0\n{text}");
    assert_eq!(header::read(&joined).unwrap(), real);
}

/// `AET-WO-4400b` §1.3: ArduPilot's defaults on a hand-written step G are
/// refused pin by pin: every topology pin's speed (`10`, where `solve` stored
/// `00` or `01`) and the CS's pull-up. The allow-listed USART's pulls are not
/// the topology's to judge.
#[test]
fn ardupilot_s_default_speeds_and_pulls_are_refused_by_name() {
    assert_eq!(
        findings(step_g_defaults(), &allow()),
        [
            "forward WrongSpeed PA0 FLASH_CS OSPEEDR 10",
            "forward WrongPull PA0 FLASH_CS PUPDR 01",
            "forward WrongSpeed PA5 SPI_SCK OSPEEDR 10",
            "forward WrongSpeed PA6 SPI_MISO OSPEEDR 10",
            "forward WrongSpeed PA7 SPI_MOSI OSPEEDR 10",
            "forward WrongSpeed PB6 I2C_SCL OSPEEDR 10",
            "forward WrongSpeed PB7 I2C_SDA OSPEEDR 10",
        ]
    );
}

/// Each drive field, wrong on one pin, is named with what the topology
/// implies: an I2C line push-pull, an SPI line open-drain, a speed one step
/// off either way, and a pull-down where none is implied.
#[test]
fn each_wrong_drive_field_is_refused_by_name() {
    type Case = (fn(&mut [[u32; 7]; 11]), &'static str, &'static str);
    let cases: [Case; 5] = [
        (
            |w| w[1][1] = 0x40,
            "forward WrongOutputType PB7 I2C_SDA push-pull",
            "open-drain",
        ),
        (
            |w| w[0][1] = 1 << 5,
            "forward WrongOutputType PA5 SPI_SCK open-drain",
            "push-pull",
        ),
        (
            |w| w[0][2] = 0x0000_0400,
            "forward WrongSpeed PA7 SPI_MOSI OSPEEDR 00",
            "OSPEEDR 01",
        ),
        (
            |w| w[1][2] = 1 << 12,
            "forward WrongSpeed PB6 I2C_SCL OSPEEDR 01",
            "OSPEEDR 00",
        ),
        (
            |w| w[0][3] = 2 << 12,
            "forward WrongPull PA6 SPI_MISO PUPDR 10",
            "PUPDR 00",
        ),
    ];
    for (edit, want, expected) in cases {
        let mut w = step_g_words();
        edit(&mut w);
        assert_eq!(findings(w, &allow()), [want]);
        let config = elf::read(
            &Elf {
                words: w,
                ..Elf::default()
            }
            .bytes(),
        )
        .unwrap();
        let j = judge(&topology(), &lqfp_like(), &config, None, &allow()).unwrap();
        assert_eq!(j.findings[0].expected, expected, "{want}");
    }
}

/// A topology with no speed for a pin was never validated: refused by name,
/// never judged as `00`.
#[test]
fn a_pin_with_no_stored_speed_is_refused() {
    let mut t = topology();
    t.pin_speeds.remove("SPI_SCK");
    let config = elf::read(&Elf::default().bytes()).unwrap();
    let err = judge(&t, &lqfp_like(), &config, None, &allow()).unwrap_err();
    assert_eq!(
        (err.kind(), err.id().as_deref()),
        ("NoPinSpeed", Some("SPI_SCK"))
    );
}

/// `AET-WO-1211a`: a pin device is a gpio fact the check cannot judge; it is
/// refused by the device's reference and part, never read as a chip select.
#[test]
fn a_pin_device_is_refused_as_not_judged() {
    use aethelgard_ir::pin_devices::{Direction, McuPull, SolvedGpio, SolvedPinDevice};
    let mut t = topology();
    t.pin_devices.insert(
        "U9".into(),
        SolvedPinDevice {
            part: "WAKE_SWITCH".into(),
        },
    );
    t.gpio.insert(
        "WAKE".into(),
        SolvedGpio {
            pin: "PB5".into(),
            direction: Direction::Input,
            initial: None,
            pull: Some(McuPull::Up),
        },
    );
    t.pin_speeds.insert("WAKE".into(), 0);
    t.validate().unwrap();
    let config = elf::read(&Elf::default().bytes()).unwrap();
    let err = judge(&t, &lqfp_like(), &config, None, &allow()).unwrap_err();
    assert_eq!(
        (err.kind(), err.id().as_deref()),
        ("GpioNotJudged", Some("U9"))
    );
    assert!(err.to_string().contains("WAKE_SWITCH"), "{err}");
}

/// `AET-WO-1211a`: a gpio entry that is not a chip select is refused by its
/// net, whatever the topology says about pin devices.
#[test]
fn an_input_gpio_entry_is_refused_as_not_judged() {
    use aethelgard_ir::pin_devices::{Direction, McuPull, SolvedGpio};
    let mut t = topology();
    t.gpio.insert(
        "WAKE".into(),
        SolvedGpio {
            pin: "PB5".into(),
            direction: Direction::Input,
            initial: None,
            pull: Some(McuPull::Up),
        },
    );
    let config = elf::read(&Elf::default().bytes()).unwrap();
    let err = judge(&t, &lqfp_like(), &config, None, &allow()).unwrap_err();
    assert_eq!(
        (err.kind(), err.id().as_deref()),
        ("GpioNotJudged", Some("WAKE"))
    );
}
