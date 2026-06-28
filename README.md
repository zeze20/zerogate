# ZeroGate

__The Mathematically Verified, Zero-Copy eBPF Gateway for Zero-Trust Networks.__

ZeroGate is a next-generation infrastructure component engineered for extreme throughput and absolute data privacy. By bypassing the standard Linux network stack via hardware-sympathetic eBPF/XDP and AF_XDP, it delivers million-packets-per-second (Mpps) filtering without suffocating the CPU. 

It is built specifically for highly regulated environments (Defense, Finance, Telecom) where microsecond latency, absolute memory safety, and cryptographic privacy are non-negotiable.

## Core Capabilities

- __Line-Rate DDoS Mitigation:__ Drops malicious or unauthorized UDP packets directly at the Network Interface Card (NIC) at wire speed, before they allocate any kernel memory.
- __Microsecond Fast-Path Routing:__ Routes valid sessions directly to userspace, completely eliminating kernel-to-userspace context switches and cache-line bouncing.
- __Privacy by Design (Zero-Trust):__ Ensures that no unauthorized entity can penetrate the network perimeter. Every single packet is cryptographically verified against a strict session map.
- __Zero-Downtime Operations:__ Seamlessly updates routing rules and scales via Kubernetes without dropping a single active packet, utilizing BPFfs map pinning.

## Cutting-Edge Technologies

ZeroGate integrates the absolute pinnacle of modern systems programming and cryptography:

- __Rust:__ The foundation of the userspace agent, providing fearless concurrency and memory safety.
- __eBPF & AF_XDP:__ Kernel-level programmability for zero-copy packet processing (`XDP_ZEROCOPY`) and lock-free per-CPU telemetry.
- __Verus (Formal Verification):__ The core parsing logic is mathematically proven. The compiler guarantees the absence of out-of-bounds reads and buffer overflows, resulting in zero runtime panics.
- __Post-Quantum Cryptography (ML-KEM-768):__ Protects handshakes against "Harvest Now, Decrypt Later" quantum attacks.
- __FROST & MPC (Multi-Party Computation):__ Threshold signing over isolated Tokio tasks ensures the master private key never materializes in memory, offering military-grade cryptographic privacy.

## ZeroGate Enterprise

While the fast-path data plane is open-source, the enterprise control plane offers carrier-grade orchestration for mission-critical infrastructure:

- __Distributed Key Management System (KMS):__ Powered by FROST enclaves.
- __Cloud-Native Productionization:__ NUMA-aware Kubernetes DaemonSets, strict CPU core pinning, and native Prometheus observability for real-time Mpps metrics.
- __Advanced Threat Intelligence:__ Dynamic eBPF map updates based on real-time threat analysis.

_(For enterprise licensing, architectural audits, and PoC requests, please contact the maintainers)._

## Architecture Deep Dive

1. __Ingress (Hardware):__ Packet hits the NIC Rx queues.
2. __eBPF/XDP Hook (Kernel):__ The XDP program (`zerogate-ebpf`) inspects the custom `ChunkHdr` and evaluates the `session_id` strictly against the lock-free BPF `HashMap`.
3. __Action Routing:__ 
   - __Invalid Session:__ Executed as `XDP_DROP` (Instant, CPU-efficient rejection).
   - __Valid Session:__ Executed as `XDP_REDIRECT` via `XSK_MAP`.
4. __Userspace Processing (`zerogate-agent`):__ The Rust agent absorbs the packet via AF_XDP Rx rings pinned to the exact IRQ CPU core, ensuring L1/L2 cache locality and maximum performance.

## Quick Start

### Prerequisites

- Ubuntu/Debian (or WSL 2) with a modern Linux Kernel (5.15+)
- Rust Nightly toolchain
- `bpf-linker` installed globally (`cargo install bpf-linker`)

### Development & Formal Tooling

For the local toolchain — Rust nightly checks plus the Java + `tla2tools.jar`
setup required to run the TLA+ model checks (`./scripts/run_tla.sh`) — see
[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

### Docker Development

For a reproducible Linux development environment with Rust, nightly, LLVM, and bpf-linker pre-installed, see [docs/DOCKER.md](docs/DOCKER.md).

### Build & Deploy

Compile the eBPF ELF object and the highly optimized userspace agent:

```bash
# Build the workspace in release mode for maximum throughput
cargo build --workspace --release

# Note: Running the AF_XDP agent requires CAP_BPF, CAP_NET_ADMIN, and CAP_IPC_LOCK capabilities.
License
This project is licensed under the Apache License 2.0 - providing explicit patent protection for enterprise adopters.
