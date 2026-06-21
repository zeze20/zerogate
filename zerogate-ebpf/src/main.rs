// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! ZeroGate XDP data-plane entry point.
//!
//! No `unsafe` in this file. All unsafe operations are isolated in
//! `parser.rs` — the designated unsafe boundary for this crate.

#![no_std]
#![no_main]

mod maps;
mod parser;
mod xdp;

use aya_ebpf::{bindings::xdp_action, macros::xdp as xdp_macro, programs::XdpContext};

/// Top-level XDP hook. Delegates to the packet pipeline and maps any
/// internal error to `XDP_ABORTED`.
#[xdp_macro]
pub fn zerogate_xdp(ctx: XdpContext) -> u32 {
    xdp::process_packet(&ctx)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
