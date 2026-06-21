// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! ZeroGate XDP data-plane program.
//!
//! When built for the `bpfel-unknown-none` target with aya-ebpf, this crate
//! produces the actual XDP program. When built as a normal binary (workspace
//! member), it serves as a documentation placeholder.
//!
//! To build the XDP program:
//!   cargo +nightly build -Z build-std=core --target bpfel-unknown-none
//!
//! # Architecture
//!
//! All unsafe is confined to `parser.rs`. The XDP pipeline is:
//!   Eth → IPv4 → UDP/TCP → policy map lookup → ZeroGate session validation
//!   → XDP_PASS / XDP_DROP / XDP_REDIRECT (AF_XDP)
//!
//! See `parser.rs`, `maps.rs`, `xdp.rs` for the eBPF implementation.

// The actual XDP modules (parser.rs, maps.rs, xdp.rs) require aya-ebpf
// and only compile for the bpfel target. They are included in the source
// tree but not compiled as part of the workspace build.

fn main() {
    eprintln!("zerogate-ebpf: this binary is a workspace placeholder.");
    eprintln!("Build the actual XDP program with:");
    eprintln!(
        "  cargo +nightly build -Z build-std=core --target bpfel-unknown-none -p zerogate-ebpf"
    );
    eprintln!();
    eprintln!("XDP pipeline: Eth -> IPv4 -> UDP/TCP -> POLICY map -> SESSIONS map -> XDP_REDIRECT");
    eprintln!("Modules: parser.rs (unsafe boundary), maps.rs, xdp.rs");
    std::process::exit(1);
}
