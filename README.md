# ZeroGate

ZeroGate is an experimental Rust workspace for an eBPF/XDP and AF_XDP networking engine.

This repository is currently an architecture scaffold. It contains shared ABI types, packet-header definitions, parser scaffolding, frame-ownership models, userspace agent scaffolding, Verus-oriented model scaffolding, and KMS placeholders.

It is not yet a production-grade zero-copy dataplane.

## Maturity Status

### Implemented in this repository

- Workspace layout for:
  - `zerogate-common`
  - `zerogate-ebpf`
  - `zerogate-agent`
  - `zerogate-verus`
  - `zerogate-kms`
- Shared ABI and packet-header definition scaffolding.
- eBPF parser and XDP pipeline scaffolding.
- Frame ownership state-machine scaffolding.
- Verus-oriented model scaffolding for ABI, parser, frame, and ring invariants.
- KMS traits and development placeholders.
- Architecture and security-invariant documentation scaffolding.

### Not yet production implemented

- Real Linux AF_XDP socket creation and bind lifecycle.
- Real Linux UMEM registration for AF_XDP.
- Real RX/TX/FILL/COMPLETION ring mmap and polling.
- Runtime XSK_MAP update from the userspace loader path.
- CI build of the eBPF program for `bpfel-unknown-none`.
- Privileged eBPF verifier/load smoke test.
- Production cryptography.
- Post-quantum cryptography, including ML-KEM.
- FROST threshold signatures or MPC.
- Kubernetes deployment integration.
- Prometheus metrics integration.
- Performance claims such as Mpps throughput or line-rate forwarding.
- Completed formal verification claims.

## Security Position

ZeroGate should be treated as a work-in-progress security-sensitive systems project.

The current goal is to make invariants explicit, auditable, and testable before claiming production readiness.

Important invariants under active development include:

- Shared ABI layout stability.
- Parser bounds checking before packet reads.
- Strict confinement of `unsafe` to audited modules.
- UMEM frame single-ownership.
- No TX-frame recycling before completion.
- Deterministic policy decisions.
- Keeping KMS and cryptographic material outside the dataplane hot path.

For more detail see:

- `docs/ARCHITECTURE.md`
- `docs/SECURITY_INVARIANTS.md`

## Architecture Summary

- **zerogate-common** defines constants, endian helpers, packet headers, and shared ABI types.
- **zerogate-ebpf** contains the XDP/eBPF parser and map scaffolding.
- **zerogate-agent** contains the userspace AF_XDP runtime scaffolding.
- **zerogate-verus** contains model scaffolding intended to align with runtime invariants.
- **zerogate-kms** contains policy-signing and key-loading boundaries. Current cryptographic implementations are development placeholders only.

## Quick Start

### Prerequisites

- Rust toolchain suitable for the workspace.
- Linux is required for eventual AF_XDP and eBPF verifier/load testing.
- Privileged eBPF/AF_XDP tests require appropriate Linux capabilities such as:
  - `CAP_BPF`
  - `CAP_NET_ADMIN`
  - `CAP_IPC_LOCK` or suitable resource limits for locked UMEM memory

### Build And Check

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```

The eBPF build and privileged verifier/load path are planned hardening work and must not be assumed complete until the corresponding CI jobs and scripts exist.

## Production Readiness

This repository must not be described as production-ready until, at minimum:

- The real Linux AF_XDP socket, UMEM, and ring path is implemented and tested.
- The eBPF program builds for `bpfel-unknown-none` in CI.
- A privileged verifier/load smoke test exists and passes on a Linux runner.
- ABI layout tests pass.
- Parser malformed-packet corpus tests pass.
- Frame lifecycle and fake-ring integration tests pass.
- The unsafe audit script passes in CI.
- Documentation matches the implemented code.

## License

This project is licensed under the Apache License 2.0.
