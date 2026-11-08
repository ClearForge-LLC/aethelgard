// The image ships only libngspice.so.0 (no libngspice-dev), and a dependency's
// rustc-link-arg does not reach this binary, so link it here too.
fn main() {
    println!("cargo:rustc-link-arg=-l:libngspice.so.0");
    println!("cargo:rerun-if-changed=build.rs");
}
