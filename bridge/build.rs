// The image ships only libngspice.so.0 (no libngspice-dev, so no unversioned
// libngspice.so and no header: AET-WO-2101 F1, ruled L3), so the binary links
// the versioned soname. The FFI is declared by hand in src/ffi.rs against the
// vendored header.
fn main() {
    println!("cargo:rustc-link-arg=-l:libngspice.so.0");
    println!("cargo:rerun-if-changed=build.rs");
}
