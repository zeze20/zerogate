//! ZeroGate eBPF/XDP program.
//!
//! When built for `bpfel-unknown-none`, this compiles as a no_std eBPF program.
//! When built for the host target, it compiles as a normal binary for testing.

#![cfg_attr(target_arch = "bpf", no_std)]
#![cfg_attr(target_arch = "bpf", no_main)]

pub mod parser;
pub mod xdp;

#[cfg(target_arch = "bpf")]
mod ebpf_entry {
    use core::panic::PanicInfo;

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        loop {}
    }
}

#[cfg(not(target_arch = "bpf"))]
fn main() {
    println!("zerogate-ebpf: XDP parser scaffold (host mode)");
    println!("Build for bpfel-unknown-none to produce an eBPF artifact.");
}
