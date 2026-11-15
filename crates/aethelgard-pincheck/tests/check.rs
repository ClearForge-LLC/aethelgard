//! Tier 1: the reader, the cross-check and the three directions, on synthetic
//! ELFs (`AET-WO-4400a` §1.4). The real ArduPilot ELFs are judged in the
//! `pinout` CI job, never committed.

mod common;

use aethelgard_pincheck::{AllowList, Mode, Refusal, elf, header, judge};
use common::{Elf, allow, lqfp_like, step_g_words, topology};

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
