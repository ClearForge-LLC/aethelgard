// The image ships only libngspice.so.0 (no libngspice-dev, so no unversioned
// libngspice.so and no header: AET-WO-2101 F1, ruled L3), so the binary links
// the versioned soname. The FFI is declared by hand in src/ffi.rs against the
// vendored header.
//
// It also computes the bridge's source digest (AET-WO-2200 F15), which the
// binary prints as the `source_sha256` of its identity line and the CLI
// recomputes from `include_bytes!` of the same files. The recipe, exactly:
//
//   SHA-256 over, in this order, the files `Cargo.toml`, `Cargo.lock`,
//   `build.rs`, then every regular file of `src/` sorted by file name (byte
//   order). For each file, feed: its path relative to the crate root (for
//   example `src/main.rs`, with `/`), one 0x00 byte, the file's length as
//   8 bytes little-endian, then the file's bytes.
//
// The digest is lower-case hex, handed to the crate as the environment
// variable `AETHELGARD_BRIDGE_SOURCE_SHA256`. The vendored header is not in it.
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rustc-link-arg=-l:libngspice.so.0");
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let mut files: Vec<String> = ["Cargo.toml", "Cargo.lock", "build.rs"]
        .map(str::to_owned)
        .to_vec();
    let mut src: Vec<String> = Vec::new();
    for entry in fs::read_dir(root.join("src")).expect("read src/") {
        let entry = entry.expect("read src/ entry");
        if entry.file_type().expect("file type").is_file() {
            src.push(entry.file_name().into_string().expect("UTF-8 file name"));
        }
    }
    src.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    files.extend(src.into_iter().map(|n| format!("src/{n}")));

    let mut hash = Sha256::new();
    for rel in &files {
        let bytes = fs::read(Path::new(&root).join(rel)).expect("read a source file");
        hash.update(rel.as_bytes());
        hash.update(&[0]);
        hash.update(&(bytes.len() as u64).to_le_bytes());
        hash.update(&bytes);
        println!("cargo:rerun-if-changed={rel}");
    }
    println!("cargo:rerun-if-changed=src");
    println!(
        "cargo:rustc-env=AETHELGARD_BRIDGE_SOURCE_SHA256={}",
        hash.finish_hex()
    );
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256, std only (a build script may use no dependency).
struct Sha256 {
    h: [u32; 8],
    block: Vec<u8>,
    len: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            h: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            block: Vec::new(),
            len: 0,
        }
    }

    fn update(&mut self, data: &[u8]) {
        self.len += data.len() as u64;
        self.block.extend_from_slice(data);
        let full = self.block.len() / 64 * 64;
        let rest = self.block.split_off(full);
        for chunk in self.block.as_chunks::<64>().0 {
            compress(&mut self.h, chunk);
        }
        self.block = rest;
    }

    fn finish_hex(mut self) -> String {
        let bits = self.len * 8;
        let mut pad = vec![0x80u8];
        while (self.block.len() + pad.len()) % 64 != 56 {
            pad.push(0);
        }
        pad.extend_from_slice(&bits.to_be_bytes());
        let len = self.len;
        self.update(&pad);
        self.len = len;
        self.h.iter().map(|w| format!("{w:08x}")).collect()
    }
}

fn compress(h: &mut [u32; 8], chunk: &[u8]) {
    let mut w = [0u32; 64];
    for (i, b) in chunk.as_chunks::<4>().0.iter().enumerate() {
        w[i] = u32::from_be_bytes(*b);
    }
    for i in 16..64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
    }
    let mut v = *h;
    for i in 0..64 {
        let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
        let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
        let t1 = v[7]
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(K[i])
            .wrapping_add(w[i]);
        let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
        let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
        let t2 = s0.wrapping_add(maj);
        v = [
            t1.wrapping_add(t2),
            v[0],
            v[1],
            v[2],
            v[3].wrapping_add(t1),
            v[4],
            v[5],
            v[6],
        ];
    }
    for (a, b) in h.iter_mut().zip(v) {
        *a = a.wrapping_add(b);
    }
}
