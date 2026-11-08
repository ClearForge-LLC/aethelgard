// The image ships only /usr/lib/x86_64-linux-gnu/libngspice.so.0 (no libngspice-dev:
// no unversioned libngspice.so, no header), so link by the versioned soname.
fn main() {
    println!("cargo:rustc-link-arg=-l:libngspice.so.0");
    println!("cargo:rerun-if-changed=build.rs");
}
