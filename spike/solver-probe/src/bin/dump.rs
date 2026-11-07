//! Prints every I2C/SPI signal in the table, one `signal pin af` per line.
fn main() {
    let t = aethelgard_svd_hal::pin_table("STM32H753ZITx").expect("table");
    for (pin, p) in &t.pins {
        for (s, af) in &p.alternate_functions {
            if s.starts_with("I2C") || s.starts_with("SPI") {
                println!("{s} {pin} {af}");
            }
        }
    }
}
