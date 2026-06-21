// SPDX-License-Identifier: GPL-2.0-only OR MIT
//! ZeroGate Key Management Service entry point.

#![allow(dead_code)]

mod keyring;
mod policy_signing;

fn main() {
    println!("zerogate-kms: key management service (placeholder)");
    println!("  policy signing: available");
    println!("  key loading: available");

    // TODO: Implement actual KMS server.
    // The KMS provides:
    // 1. Policy signing — signs compact policy entries for the data plane.
    // 2. Key loading — loads key material for the agent.
    // The data-plane hot path uses compact map values only;
    // no key material is present in eBPF.
}
