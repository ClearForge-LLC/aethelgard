//! `AET-WO-4300b`, the board crate's serial ports: a console on `USART2`
//! (PA2/PA3, AF7) and four ports on four USARTs, each hand-built from the
//! reference's solved topology. Every value a configuration writes is the one
//! `aethelgard_ir::serial` gives, at RM0433's bit positions; the pins take
//! speed `00`; the firmware sends each pattern before the identity reads,
//! echoes after the boot word, and touches no USART after the echo; and a
//! reference without serial ports generates what it did before.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use aethelgard_ir::serial::{self, Framing, Parity, SerialRole, SolvedSerialPort};
use aethelgard_ir::{PinAssignment, PinTable, SolvedClock, SolvedTopology, Specification};
use aethelgard_svd_hal::board::{SERIAL_LINE, SpeedCheck, generate_with, pin_speeds};
use aethelgard_svd_hal::pin_table;
use sha2::{Digest, Sha256};

fn solved_from(path: &Path) -> (SolvedTopology, PinTable) {
    let bytes = fs::read(path).unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = pin_table(&spec.mcu.part).unwrap();
    (aethelgard_ir::solve(&spec, &table, &sha).unwrap(), table)
}

/// The reference's solved topology with no serial port: any the
/// specification declares, and their assignments, removed.
fn reference() -> (SolvedTopology, PinTable) {
    let (mut solved, table) = solved_from(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json"),
    );
    strip_serial(&mut solved);
    (solved, table)
}

fn strip_serial(solved: &mut SolvedTopology) {
    solved.serial_ports.clear();
    solved
        .assignments
        .retain(|_, a| !serial::PERIPHERALS.contains(&a.peripheral.as_str()));
    // The stored speeds of the pins removed go too (`AET-WO-4400b` D2).
    let kept: Vec<String> = solved
        .assignments
        .keys()
        .chain(solved.gpio.keys())
        .cloned()
        .collect();
    solved.pin_speeds.retain(|net, _| kept.contains(net));
}

/// What the CLI passes: the twin's model table.
fn answers(part: &str) -> bool {
    matches!(aethelgard_twin::model::model(part), Ok(Some(_)))
}

fn framing(baud: u32, parity: Parity, stop_bits: u8, inverted: bool) -> Framing {
    Framing {
        baud,
        data_bits: 8,
        parity,
        stop_bits,
        inverted,
    }
}

/// One port to add: label, peripheral, TX pin, RX pin, AF, role, framing.
type Port = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    u8,
    SerialRole,
    Framing,
);

/// `solved` with `ports` added, in this order, each with its `BRR` solved by
/// `aethelgard_ir::serial` on the reset clock and its nets `<LABEL>_TX` and
/// `<LABEL>_RX` assigned.
fn add_ports(solved: &mut SolvedTopology, ports: &[Port]) {
    insert_ports(solved, ports);
    solved.validate().unwrap();
}

/// [`add_ports`] without the topology's check, for a refusal the check makes.
fn insert_ports(solved: &mut SolvedTopology, ports: &[Port]) {
    let clock = SolvedClock::reset();
    for (i, (label, peripheral, tx, rx, af, role, f)) in ports.iter().enumerate() {
        let kernel = serial::kernel_hz(&clock, peripheral).unwrap();
        let brr = serial::solve_brr(kernel, f.baud).unwrap();
        let up = label.to_ascii_uppercase();
        let (tx_net, rx_net) = (format!("{up}_TX"), format!("{up}_RX"));
        for (net, pin, line) in [(&tx_net, tx, "TX"), (&rx_net, rx, "RX")] {
            // A serial line's stored speed is `00` (`AET-WO-4400b` D2).
            solved.pin_speeds.insert(net.clone(), 0);
            solved.assignments.insert(
                net.clone(),
                PinAssignment {
                    pin: (*pin).into(),
                    peripheral: (*peripheral).into(),
                    signal: format!("{peripheral}_{line}"),
                    alternate_function: *af,
                },
            );
        }
        solved.serial_ports.insert(
            (*label).into(),
            SolvedSerialPort {
                index: u32::try_from(i).unwrap(),
                peripheral: (*peripheral).into(),
                tx: tx_net,
                rx: rx_net,
                role: *role,
                framing: f.clone(),
                negotiated: None,
                // An `rc_crsf` port declares at least the cited CRSF spread.
                peer_deviation_ppm: if *role == SerialRole::RcCrsf {
                    serial::crsf_cited_spread_ppm()
                } else {
                    0
                },
                kernel_hz: kernel,
                brr,
                actual_millibaud: serial::actual_millibaud(kernel, brr),
                error_ppm: serial::error_ppm(kernel, brr, f.baud),
            },
        );
    }
}

const CONSOLE: [Port; 1] = [(
    "console",
    "USART2",
    "PA2",
    "PA3",
    7,
    SerialRole::Console,
    Framing {
        baud: 115_200,
        data_bits: 8,
        parity: Parity::None,
        stop_bits: 1,
        inverted: false,
    },
)];

/// The four-port fixture's framings, on four peripherals and pins the pin
/// table gives (AF7 and AF8), none of the reference's.
fn four_ports() -> [Port; 4] {
    [
        (
            "mavlink",
            "USART1",
            "PA9",
            "PA10",
            7,
            SerialRole::Mavlink,
            framing(57_600, Parity::None, 1, false),
        ),
        (
            "gps",
            "USART3",
            "PD8",
            "PD9",
            7,
            SerialRole::Gps,
            framing(115_200, Parity::None, 1, false),
        ),
        (
            "rc_crsf",
            "UART4",
            "PD1",
            "PD0",
            8,
            SerialRole::RcCrsf,
            framing(420_000, Parity::None, 1, false),
        ),
        (
            "rc_sbus",
            "USART6",
            "PC6",
            "PC7",
            7,
            SerialRole::RcSbus,
            framing(100_000, Parity::Even, 2, true),
        ),
    ]
}

fn console() -> (SolvedTopology, PinTable) {
    let (mut solved, table) = reference();
    add_ports(&mut solved, &CONSOLE);
    (solved, table)
}

fn four() -> (SolvedTopology, PinTable) {
    let (mut solved, table) = reference();
    add_ports(&mut solved, &four_ports());
    (solved, table)
}

fn generate(solved: &SolvedTopology, table: &PinTable) -> BTreeMap<String, String> {
    generate_with("serial-board", solved, table, &answers).unwrap()
}

/// The text from `head` to the end of its item (the next line that is `}`).
fn item<'a>(text: &'a str, head: &str) -> &'a str {
    assert_eq!(text.matches(head).count(), 1, "`{head}`");
    let at = text.find(head).unwrap();
    let rest = &text[at..];
    &rest[..rest.find("\n}\n").unwrap()]
}

/// The `r.write` lines of `ty`'s configuration.
fn configuration(board: &str, ty: &str) -> Vec<String> {
    item(board, &format!("impl {ty}<Clocked> {{"))
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("r."))
        .map(str::to_owned)
        .collect()
}

/// RM0433 Rev 9 §48.8 (USART_TypeDef offsets and bit positions), written here
/// from the manual, not read from the header the generator reads.
const BRR: u32 = 0x0C;
const CR2: u32 = 0x04;
const CR1: u32 = 0x00;
const UE: u32 = 1;
const RE: u32 = 1 << 2;
const TE: u32 = 1 << 3;
const PS: u32 = 1 << 9;
const PCE: u32 = 1 << 10;
const M0: u32 = 1 << 12;
const M1: u32 = 1 << 28;
/// FIFO mode (RM0433 Rev 9 §48.8.1; the PM's ruling on adversarial pass 2,
/// H1).
const FIFOEN: u32 = 1 << 29;
const STOP_POS: u32 = 12;
const RXINV: u32 = 1 << 16;
const TXINV: u32 = 1 << 17;

/// The four writes `port` must make, from `serial.rs`'s `BRR`, `m_bits` and
/// `stop_field`.
fn expected(ty: &str, port: &SolvedSerialPort) -> Vec<String> {
    let f = &port.framing;
    let m = f.m_bits().unwrap();
    let mut cr1 = RE | TE | FIFOEN;
    if m & 1 != 0 {
        cr1 |= M0;
    }
    if m & 2 != 0 {
        cr1 |= M1;
    }
    match f.parity {
        Parity::None => {}
        Parity::Even => cr1 |= PCE,
        Parity::Odd => cr1 |= PCE | PS,
    }
    let mut cr2 = u32::from(f.stop_field().unwrap()) << STOP_POS;
    if f.inverted {
        cr2 |= TXINV | RXINV;
    }
    let w = |offset: u32, value: u32, name: &str| {
        format!("r.write(TOKEN, Block::{ty}, {offset:#04x}, {value:#010x}); // {name}")
    };
    vec![
        w(BRR, port.brr, "BRR"),
        w(CR2, cr2, "CR2"),
        w(CR1, cr1, "CR1"),
        w(CR1, cr1 | UE, "CR1 | UE"),
    ]
}

/// `USART2` → `Usart2`.
fn ty(peripheral: &str) -> String {
    let mut out = String::new();
    for (i, c) in peripheral.chars().enumerate() {
        out.push(if i == 0 { c } else { c.to_ascii_lowercase() });
    }
    out
}

#[test]
fn the_console_writes_exactly_its_solved_values_in_order() {
    let (solved, table) = console();
    let board = &generate(&solved, &table)["src/board.rs"];
    let port = &solved.serial_ports["console"];
    assert_eq!(port.brr, 556);
    let writes = configuration(board, "Usart2");
    assert_eq!(writes, expected("Usart2", port));
    // The literals, worked by hand: 64 MHz / 115200 rounds to 556, 8N1, in
    // FIFO mode.
    assert_eq!(
        writes,
        [
            "r.write(TOKEN, Block::Usart2, 0x0c, 0x0000022c); // BRR",
            "r.write(TOKEN, Block::Usart2, 0x04, 0x00000000); // CR2",
            "r.write(TOKEN, Block::Usart2, 0x00, 0x2000000c); // CR1",
            "r.write(TOKEN, Block::Usart2, 0x00, 0x2000000d); // CR1 | UE",
        ]
    );
}

#[test]
fn four_ports_each_write_their_solved_values() {
    let (solved, table) = four();
    let board = &generate(&solved, &table)["src/board.rs"];
    for (label, port) in &solved.serial_ports {
        let t = ty(&port.peripheral);
        assert_eq!(
            configuration(board, &t),
            expected(&t, port),
            "{label} on {}",
            port.peripheral
        );
    }
    // SBUS, 100 kbaud 8E2 inverted: BRR 640, STOP 10 with TXINV and RXINV, a
    // 9-bit word (M0) with even parity.
    assert_eq!(
        configuration(board, "Usart6"),
        [
            "r.write(TOKEN, Block::Usart6, 0x0c, 0x00000280); // BRR",
            "r.write(TOKEN, Block::Usart6, 0x04, 0x00032000); // CR2",
            "r.write(TOKEN, Block::Usart6, 0x00, 0x2000140c); // CR1",
            "r.write(TOKEN, Block::Usart6, 0x00, 0x2000140d); // CR1 | UE",
        ]
    );
    // USART1 and USART6 are on APB2ENR, UART4 on APB1LENR (RM0433 §8.7.40, 42).
    let rcc = item(board, "impl Rcc {");
    for (f, t, offset, bit) in [
        ("usart1", "Usart1", 0x0f0, 4),
        ("usart6", "Usart6", 0x0f0, 5),
        ("usart3", "Usart3", 0x0e8, 18),
        ("uart4", "Uart4", 0x0e8, 19),
    ] {
        assert!(
            rcc.contains(&format!(
                "pub fn enable_{f}<R: Registers>(&mut self, r: &mut R, _gated: {t}<Gated>, _scale2: Scale2) -> {t}<Clocked> {{\n        \
                 r.modify(TOKEN, Block::Rcc, {offset:#05x}, |v| v | {:#010x});",
                1u32 << bit
            )),
            "{f}"
        );
    }
}

/// INV-02 in the text: the configuration takes the token by reference and
/// exactly its two solved pins, each in its function, from `Clocked`; it reads
/// nothing; sending and receiving exist only once it is configured.
#[test]
fn the_configuration_takes_the_clock_token_and_only_its_own_pins() {
    let (solved, table) = console();
    let board = &generate(&solved, &table)["src/board.rs"];
    let start = item(board, "impl Usart2<Clocked> {");
    assert!(start.contains(
        "        _clock: &ResetClock,\n        \
         _tx: PA2<AlternateFunction<7>>,\n        \
         _rx: PA3<AlternateFunction<7>>,\n    \
         ) -> Usart2<Enabled<()>> {"
    ));
    assert!(!start.contains("r.read") && !start.contains("r.modify"));
    assert!(!start.contains("fn send") && !start.contains("fn receive"));
    let enabled = item(board, "impl Usart2<Enabled<()>> {");
    assert!(enabled.contains("pub fn send<R: Registers>"));
    assert!(enabled.contains("pub fn receive<R: Registers>"));
    // TXE is ISR bit 7, RXNE bit 5; TDR is at 0x28, RDR at 0x24.
    // `send` waits its own bound: no proof takes a smaller one, so no
    // `send_within` claims one does (adversarial F13).
    assert!(enabled.contains("usart_wait(r, Block::Usart2, 0x00000080, USART_WAIT_SPINS)"));
    assert!(!board.contains("send_within"));
    assert!(enabled.contains("r.write(TOKEN, Block::Usart2, 0x28, u32::from(byte));"));
    assert!(enabled.contains("usart_wait(r, Block::Usart2, 0x00000020, polls)"));
    assert!(enabled.contains("r.read(TOKEN, Block::Usart2, 0x24)"));
    assert!(board.contains("            Block::Usart2 => 0x40004400,"));
    assert_eq!(board.matches("ResetClock { _private: () }").count(), 1);
    // The clock check covers the USART kernel clock selections.
    let rcc = item(board, "impl Rcc {");
    assert!(rcc.contains("        let d2ccip2r = r.read(TOKEN, Block::Rcc, 0x054);\n"));
    assert!(rcc.contains("&& d2ccip2r & 0x0000303f == 0 {"));
    // No timer: no `Running`.
    assert!(!board.contains("pub struct Running;"));
}

/// The USART pins take `00` with their own reason, AF7, push-pull, no pull.
#[test]
fn serial_pins_take_00_with_their_own_reason() {
    let (solved, table) = four();
    let speeds = pin_speeds(&solved, &table).unwrap();
    let serial: Vec<_> = speeds.iter().filter(|p| p.signal.contains("ART")).collect();
    assert_eq!(serial.len(), 8);
    for p in serial {
        assert_eq!(p.ospeedr, 0, "{}", p.pin);
        assert_eq!(
            p.check,
            SpeedCheck::OutOfScope {
                reason: SERIAL_LINE
            }
        );
    }
    let (solved, table) = console();
    let board = &generate(&solved, &table)["src/board.rs"];
    let pa2 = item(board, "impl PA2<Unconfigured> {");
    assert!(pa2.contains(
        "pub fn into_usart2_tx<R: Registers>(self, r: &mut R) -> PA2<AlternateFunction<7>> {"
    ));
    // OSPEEDR 00, AFRL AF7, MODER AF; push-pull is the port's whole OTYPER
    // and no pull its whole PUPDR (PA2 bits 5:4 `00`, the debug pulls kept),
    // which `split` writes and no pin touches (`AET-WO-4230` D5, R5).
    assert!(!pa2.contains("Block::GpioA, 0x04"), "{pa2}");
    assert!(!pa2.contains("Block::GpioA, 0x0c"), "{pa2}");
    let split = item(board, "impl GpioA<Clocked> {");
    assert!(split.contains("        r.write(TOKEN, Block::GpioA, 0x04, 0x00000000);\n"));
    assert!(split.contains("        r.write(TOKEN, Block::GpioA, 0x0c, 0x64000000);\n"));
    for line in [
        "r.modify(TOKEN, Block::GpioA, 0x08, |v| v & !0x00000030);",
        "r.modify(TOKEN, Block::GpioA, 0x20, |v| (v & !0x00000f00) | 0x00000700);",
        "r.modify(TOKEN, Block::GpioA, 0x00, |v| (v & !0x00000030) | 0x00000020);",
    ] {
        assert!(pa2.contains(line), "{line}\n{pa2}");
    }
    assert!(
        item(board, "impl PA3<Unconfigured> {")
            .contains("r.modify(TOKEN, Block::GpioA, 0x08, |v| v & !0x000000c0);")
    );
}

/// The firmware's order: the reset clock or park (first, before any write:
/// `AET-WO-4230` D6), the clock gate, the pins, the configuration, the
/// pattern, the identity reads, the boot word, the echo, the park; and after
/// the echo nothing names a USART.
#[test]
fn the_console_firmware_runs_in_the_contract_s_order() {
    let (solved, table) = console();
    let main = &generate(&solved, &table)["firmware/src/main.rs"];
    let at = |s: &str| main.find(s).unwrap_or_else(|| panic!("{s}\n{main}"));
    let order = [
        at("    let Ok(clock) = rcc.reset_clock(&mut r, &scale2) else {\n"),
        at("    let usart2 = rcc.enable_usart2(&mut r, usart2, scale2);\n"),
        at("    let pa2 = gpioa.pa2.into_usart2_tx(&mut r);\n"),
        at("    let pa3 = gpioa.pa3.into_usart2_rx(&mut r);\n"),
        at("    let mut usart2 = usart2.configure(&mut r, &clock, pa2, pa3);\n"),
        at(
            "    for &byte in [0x63, 0x6f, 0x6e, 0x73, 0x6f, 0x6c, 0x65].iter().chain(PATTERN_TAIL.iter()) {\n        \
             if usart2.send(&mut r, byte).is_err() {\n",
        ),
        at("record(0,"),
        at(".write_volatile(BOOT_MAGIC)"),
        at("    for _ in 0..FRAME_LEN {\n        \
            let Some(byte) = usart2.receive(&mut r, ECHO_POLLS) else {\n"),
        at("    // No sleep:"),
    ];
    assert!(order.is_sorted(), "{order:?}");
    assert!(main.rfind("record(").unwrap() < order[7]);
    assert_eq!(main.matches("rcc.reset_clock(").count(), 1);
    assert_eq!(main.matches("usart2.send(").count(), 2);
    let after_echo = &main[main.rfind("usart2.").unwrap()..];
    let park = &after_echo[after_echo.find("    // No sleep:").unwrap()..];
    assert!(!park.contains("usart") && !park.contains("Usart"), "{park}");
    assert!(main.contains("const PATTERN_TAIL: [u8; 5] = [0x00, 0xfe, 0x7f, 0x80, 0xff];\n"));
    assert!(main.contains("const FRAME_LEN: u32 = 8;\n"));
    assert!(main.contains(&format!(
        "const ECHO_POLLS: u32 = {};\n",
        aethelgard_svd_hal::board::ECHO_POLLS
    )));
}

/// Four ports: configured, then their patterns, then echoed, each in the
/// specification's order (not the peripherals' name order).
#[test]
fn four_ports_go_in_the_specification_s_order() {
    let (solved, table) = four();
    let main = &generate(&solved, &table)["firmware/src/main.rs"];
    let at = |s: &str| main.find(s).unwrap_or_else(|| panic!("{s}\n{main}"));
    let mut order = Vec::new();
    for f in ["usart1", "usart3", "uart4", "usart6"] {
        order.push(at(&format!(
            "    let mut {f} = {f}.configure(&mut r, &clock, "
        )));
    }
    for f in ["usart1", "usart3", "uart4", "usart6"] {
        order.push(at(&format!(
            "        if {f}.send(&mut r, byte).is_err() {{\n"
        )));
    }
    order.push(at(".write_volatile(BOOT_MAGIC)"));
    for f in ["usart1", "usart3", "uart4", "usart6"] {
        order.push(at(&format!(
            "        let Some(byte) = {f}.receive(&mut r, ECHO_POLLS) else {{\n"
        )));
    }
    assert!(order.is_sorted(), "{order:?}");
}

/// A board with motor outputs and a port reads the reset clock once, before
/// the port's configuration, and its echo never touches a timer.
#[test]
fn a_board_with_timers_and_a_port_reads_the_clock_once_first() {
    let (mut solved, table) = solved_from(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../aethelgard-cli/tests/fixtures/motors-pwm.json"),
    );
    strip_serial(&mut solved);
    add_ports(&mut solved, &CONSOLE);
    let files = generate(&solved, &table);
    let main = &files["firmware/src/main.rs"];
    assert_eq!(main.matches("rcc.reset_clock(").count(), 1);
    let clock = main.find("rcc.reset_clock(").unwrap();
    let configure = main.find("usart2.configure(").unwrap();
    let start = main.find("tim1.start_outputs(").unwrap();
    let boot = main.find(".write_volatile(BOOT_MAGIC)").unwrap();
    assert!(clock < configure && configure < start && start < boot);
    let echo = &main[boot..];
    assert!(!echo.contains("tim1") && !echo.contains("Tim1"), "{echo}");
    let board = &files["src/board.rs"];
    assert!(board.contains("pub struct Running;"));
    assert!(board.contains(
        "/// The clock tree is the reset clock this board's values were solved for: HSI\n"
    ));
}

#[test]
fn regeneration_is_byte_identical() {
    for (solved, table) in [console(), four()] {
        assert_eq!(generate(&solved, &table), generate(&solved, &table));
    }
}

/// Each refusal, by name. Since `aethelgard-ir` judges `serial_ports` in
/// `SolvedTopology::validate` (step `validate_solved`), every case here is the
/// topology's refusal before the generator looks, named with the id the
/// check reports; the generator's own refusals stay behind it.
#[test]
fn what_the_generator_cannot_write_is_refused() {
    // The refusal's name, and for a topology the check refuses, the check's
    // own name and the id it names.
    let kind = |solved: &SolvedTopology, table: &PinTable| match generate_with(
        "b", solved, table, &answers,
    )
    .unwrap_err()
    {
        aethelgard_svd_hal::board::BoardError::InvalidTopology(e) => {
            format!("InvalidTopology({} {})", e.kind(), e.id().unwrap_or("-"))
        }
        e => e.kind().to_owned(),
    };
    // LPUART1: refused by name, whatever its pins. The topology's own
    // serial check refuses it first, since `aethelgard-ir` judges
    // `serial_ports` in `SolvedTopology::validate`: the port's first net (in
    // name order) is on USART2, not on the port's LPUART1.
    let (mut solved, table) = console();
    solved.serial_ports.get_mut("console").unwrap().peripheral = "LPUART1".into();
    assert_eq!(
        kind(&solved, &table),
        "InvalidTopology(SerialPortMismatch CONSOLE_RX)"
    );
    // Two ports on one USART.
    let (mut solved, table) = reference();
    let mut two = four_ports();
    (two[1].1, two[1].2, two[1].3, two[1].4) = ("USART1", "PB14", "PB15", 4);
    insert_ports(&mut solved, &two);
    // The topology's serial check refuses it first, naming the second port
    // (in label order) on USART1.
    assert_eq!(
        kind(&solved, &table),
        "InvalidTopology(SerialPortMismatch mavlink)"
    );
    // A TX on another USART's signal.
    let (mut solved, table) = console();
    let a = solved.assignments.get_mut("CONSOLE_TX").unwrap();
    a.pin = "PA9".into();
    a.peripheral = "USART1".into();
    a.signal = "USART1_TX".into();
    assert_eq!(
        kind(&solved, &table),
        "InvalidTopology(SerialPortMismatch CONSOLE_TX)"
    );
    // The port's TX and RX nets swapped, each now on the other's signal.
    let (mut solved, table) = console();
    let port = solved.serial_ports.get_mut("console").unwrap();
    (port.tx, port.rx) = (port.rx.clone(), port.tx.clone());
    assert_eq!(
        kind(&solved, &table),
        "InvalidTopology(SerialPortMismatch console)"
    );
    // A port's TX net on another peripheral's signal: I2C1's SDA, moved from
    // its own net.
    let (mut solved, table) = console();
    let sda = solved
        .assignments
        .iter()
        .find(|(_, a)| a.signal == "I2C1_SDA")
        .map(|(n, _)| n.clone())
        .unwrap();
    let line = solved.analog_lines.remove(&sda).unwrap();
    solved.analog_lines.insert("CONSOLE_TX".into(), line);
    let sda = solved.assignments.remove(&sda).unwrap();
    solved.assignments.insert("CONSOLE_TX".into(), sda);
    assert_eq!(
        kind(&solved, &table),
        "InvalidTopology(SerialPortMismatch console)"
    );
    // A port whose RX net is not assigned.
    let (mut solved, table) = console();
    solved.assignments.remove("CONSOLE_RX");
    assert_eq!(
        kind(&solved, &table),
        "InvalidTopology(SerialPortMismatch console)"
    );
    // One net for TX and RX.
    let (mut solved, table) = console();
    solved.serial_ports.get_mut("console").unwrap().rx = "CONSOLE_TX".into();
    assert_eq!(
        kind(&solved, &table),
        "InvalidTopology(SerialPortMismatch console)"
    );
    // Three stop bits, and a BRR past 16 bits.
    let (mut solved, table) = console();
    solved
        .serial_ports
        .get_mut("console")
        .unwrap()
        .framing
        .stop_bits = 3;
    assert_eq!(
        kind(&solved, &table),
        "InvalidTopology(SerialPortMismatch console)"
    );
    let (mut solved, table) = console();
    solved.serial_ports.get_mut("console").unwrap().brr = 0x1_0000;
    assert_eq!(
        kind(&solved, &table),
        "InvalidTopology(SerialPortMismatch console)"
    );
}

/// The files the board generator writes for the reference without serial
/// ports, with its specification's sha256 put back to zeros, each by its
/// sha256. Taken with the serial generator on the commit where
/// `tests/board.rs` still passed against the committed `boards/sentinel-01`
/// of `5833b9c` (no serial port), which that test compares byte for byte, so
/// these are the digests of the generator before any serial code. A reference
/// without serial ports must not move. Re-taken at `AET-WO-4230`, which moves
/// every board on purpose (the reset clock read first and taken by I2C and
/// SPI, `OTYPER` written whole, the enables read back, the SPI `EOT` wait;
/// then the solved SPI and I2C words written whole, the kernel selection, the
/// erratum delay and the brown-out word; in round 2, `CFG2.AFCNTR` and
/// `PUPDR` written whole by `split`). Re-taken at `AET-WO-4211`: the flash is
/// `nrnd`, so each file with a header gains its one `LIFECYCLE NRND` line, and
/// nothing else moves (`Cargo.lock` and `firmware/Cargo.lock` carry no header).
/// Re-taken at `AET-WO-3303b` with `AETHELGARD_PRINT_DIGESTS=1`: the schedule is
/// proved in parts on every board with two or more, so only `src/proofs.rs`
/// moves.
/// Re-taken at `AET-WO-4240v` (after `3303b`), which moves every board on
/// purpose (the supply and VOS Scale 2 first, `Pwr`, `Scale2` taken by
/// `reset_clock`, the PWR proofs): `main.rs`, `board.rs` and `proofs.rs` were
/// `ff451c44…8d2e`, `59f194d1…c792` and `f34b4e0d…c837`; nothing else moved.
/// Re-taken at `AET-WO-4241` (the architect's D3): `main.rs` alone moves, by
/// U3's (the TMP117's) identity read after every modelled read and before
/// the boot word; it was `d26fab5b…d7fe`.
/// Re-taken at `AET-WO-4240b` (`4240v` F2, ruled): every clock enable takes the
/// `Scale2` token, so `board.rs` moves by that signature (and `Scale2`'s
/// `Clone, Copy`, which keeps every ELF's bytes), `main.rs` and `proofs.rs` by
/// the calls; they were `5dad6e44…c2f9`, `8184414c…5e1e` and `e8218507…6608`.
/// Re-taken at `AET-WO-4240e` (D1 (a), D6, ruled): `select_spi_kernels` runs
/// PLL1 from the HSI across the select and returns a `Result`, so `board.rs`
/// moves by the switch and its fault, `main.rs` by the fault arm and its two
/// constants, and `proofs.rs` by the switch's harness and token; they were
/// `975b78d8…bc7f`, `8680571d…5ae8` and `65140d4a…32d0`.
const BEFORE: [(&str, &str); 10] = [
    (
        "Cargo.lock",
        "9f14c1c5149e01c67c7cab3e617a53955afc5e2b3015672b332dd3651da2c63f",
    ),
    (
        "Cargo.toml",
        "f4e5c4dbdb6a816e1efdc99cd8071a1598acef82224b27225b86217a1f9d6169",
    ),
    (
        "firmware/.cargo/config.toml",
        "468c489444b42d98dc223d65af57707b9890a5de8253ac814104eb9e478b5de2",
    ),
    (
        "firmware/Cargo.lock",
        "289681befb5e390564f79f07d413b938c2f119d79bfddd968dc0029ab2d753c1",
    ),
    (
        "firmware/Cargo.toml",
        "b13172b26c76bde6ff2d706dd9804bee9b5835806bd187eebd398b8d45afb97f",
    ),
    (
        "firmware/link.x",
        "f8b01e41ab824305cd250c72742294d94606ec8e46a6f34bb0fb85955c76aa68",
    ),
    (
        "firmware/src/main.rs",
        "e42a3928a5867ab8b032906d2da2dfc2329dd6d2b8f58b7c7a33201faa92cd0f",
    ),
    (
        "src/board.rs",
        "dc9a28abba048c6e0b7e658d4ae78fba862ef6cb8f6b048bac5c2ed17d9015a7",
    ),
    (
        "src/lib.rs",
        "7b8e13544cfa36e35068ca9e05b2106972cb267c2304b0aaecdc3d779f2ba67d",
    ),
    (
        "src/proofs.rs",
        "02750bb8ab83cf7d46ebccefb0844ab1899c5758cf5402bfaeaf936620f94f62",
    ),
];

#[test]
fn a_reference_without_serial_ports_generates_what_it_did_before() {
    let (solved, table) = reference();
    let files = generate_with("sentinel-01-board", &solved, &table, &answers).unwrap();
    let mut now = Vec::new();
    for (path, text) in &files {
        let text = text.replace(&solved.specification_sha256, &"0".repeat(64));
        let digest: String = Sha256::digest(text.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        now.push((path.clone(), digest));
    }
    if std::env::var_os("AETHELGARD_PRINT_DIGESTS").is_some() {
        for (p, d) in &now {
            println!("(\"{p}\", \"{d}\"),");
        }
    }
    assert_eq!(now.len(), BEFORE.len());
    for ((path, digest), (want_path, want)) in now.iter().zip(BEFORE) {
        assert_eq!(path, want_path);
        assert_eq!(digest, want, "{path} changed");
    }
}

/// The trybuild `usart_*` cases include this generated `board.rs` by path; it
/// must be exactly what the generator writes for the four ports.
/// `AETHELGARD_BLESS=1` rewrites it.
#[test]
fn the_serial_ui_board_is_the_generated_one() {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ui/serial-four/board.rs");
    let (solved, table) = four();
    let text = generate(&solved, &table)["src/board.rs"].clone();
    if std::env::var_os("AETHELGARD_BLESS").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &text).unwrap();
    }
    assert!(
        fs::read_to_string(&path).unwrap() == text,
        "{} differs",
        path.display()
    );
}
