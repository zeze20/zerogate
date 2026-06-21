# ZeroGate Architecture

## Overview

ZeroGate is a zero-copy, formally-verifiable eBPF data plane for zero-trust networks. It uses Linux XDP (eXpress Data Path) for wire-speed packet classification and AF_XDP for zero-copy delivery to userspace.

```
                      ┌─────────────────────────────────┐
                      │         zerogate-kms             │
                      │   (policy signing, key mgmt)     │
                      └──────────┬──────────────────────┘
                                 │ signed policy
                      ┌──────────▼──────────────────────┐
                      │       zerogate-agent             │
                      │  (AF_XDP userspace data plane)   │
                      │                                  │
                      │  ┌─────────────────────────┐     │
                      │  │  Per-Queue RX Loop       │     │
                      │  │  (CPU-pinned thread)     │     │
                      │  │                         │     │
                      │  │  UMEM ←→ Frame Pool     │     │
                      │  │  Fill/RX/TX/Comp Rings  │     │
                      │  └─────────────────────────┘     │
                      └──────────┬──────────────────────┘
                                 │ AF_XDP (zero-copy)
              ┌──────────────────┼──────────────────────┐
              │            KERNEL                        │
              │  ┌───────────────▼────────────────┐      │
              │  │       zerogate-ebpf             │      │
              │  │   (XDP program, BPF maps)       │      │
              │  │                                 │      │
              │  │  Eth → IPv4 → UDP/TCP → Policy  │      │
              │  │  → XDP_PASS / XDP_DROP /        │      │
              │  │    XDP_REDIRECT (AF_XDP)        │      │
              │  └─────────────────────────────────┘      │
              │                                          │
              │          NIC (driver / hardware)         │
              └──────────────────────────────────────────┘
                                 │
                      ┌──────────▼──────────────────────┐
                      │      zerogate-common             │
                      │  (shared ABI types, headers)     │
                      └─────────────────────────────────┘
                      ┌─────────────────────────────────┐
                      │      zerogate-verus              │
                      │  (formal verification model)     │
                      └─────────────────────────────────┘
```

## Zero-Copy Memory Model

### UMEM Region

The UMEM is a page-aligned, contiguous memory region shared between the kernel and userspace via `mmap`. It is divided into fixed-size frames (default: 4 KiB × 4096 = 16 MiB).

```
UMEM Region (16 MiB)
┌───────┬───────┬───────┬───────┬─ ─ ─ ─┬───────┐
│Frame 0│Frame 1│Frame 2│Frame 3│  ...  │Frame N│
│ 4 KiB │ 4 KiB │ 4 KiB │ 4 KiB │       │ 4 KiB │
└───────┴───────┴───────┴───────┴─ ─ ─ ─┴───────┘
```

Frames are addressed by **index** and **UMEM offset**, never by raw pointer. The offset is `index × frame_size`.

### AF_XDP Ring Structure

Four rings connect kernel and userspace:

| Ring       | Direction        | Purpose                                  |
|------------|------------------|------------------------------------------|
| FILL       | User → Kernel    | Submit empty frame addrs for kernel to fill |
| RX         | Kernel → User    | Kernel delivers filled packet descriptors  |
| TX         | User → Kernel    | Submit frames for kernel to transmit       |
| COMPLETION | Kernel → User    | Kernel returns transmitted frame addrs     |

## UMEM Ownership State Machine

Every UMEM frame is in exactly one state at any time. The state machine is the central invariant of the entire data plane.

```
                ┌──────┐
        ┌──────►│ Free │◄─────────────┐
        │       └──┬───┘              │
        │          │                  │
        │          ▼                  │
        │       ┌──────┐              │
        │       │InFill│◄───────┐     │
        │       └──┬───┘        │     │
        │          │            │     │
        │          ▼            │     │
        │       ┌──────┐        │     │
        │       │Kernel│        │     │
        │       └──┬───┘        │     │
        │          │            │     │
        │          ▼            │     │
        │       ┌──────┐        │     │
        │       │  Rx  │        │     │
        │       └──┬───┘        │     │
        │          │            │     │
        │          ▼            │     │
        │       ┌──────┐        │     │
        │       │ User ├────────┘     │
        │       └──┬───┘              │
        │          │                  │
        │          ▼                  │
        │       ┌──────┐              │
        │       │  Tx  │              │
        │       └──┬───┘              │
        │          │                  │
        │          ▼                  │
        │       ┌──────────┐          │
        └───────┤Completion├──────────┘
                └──────────┘
```

### Legal Transitions

| From       | To         | When                                     |
|------------|------------|------------------------------------------|
| Free       | InFill     | Frame allocated for fill ring             |
| InFill     | Kernel     | Fill ring submitted to kernel             |
| Kernel     | Rx         | Packet received (appears in RX ring)      |
| Rx         | User       | Userspace acquires frame for processing   |
| User       | InFill     | Frame recycled back to fill ring          |
| User       | Tx         | Frame submitted for transmission          |
| Tx         | Completion | Kernel completes transmission             |
| Completion | Free       | Frame returned to free pool               |

**No other transitions are allowed.** Debug assertions enforce this in development; structured errors are returned in release.

## eBPF Verifier Rules

The XDP program follows strict rules for Linux eBPF verifier compliance:

1. **Bounds-checked reads**: Every packet dereference is preceded by:
   ```
   if data + offset + size_of::<T>() > data_end { return; }
   ```

2. **Monotonic offset accumulator**: The parse position only increases.

3. **No loops** (except statically bounded).

4. **No heap allocation, panics, or formatting**.

5. **Null-checked map lookups**: Every `bpf_map_lookup_elem` result is checked for NULL.

## Unsafe Boundary

`unsafe` is strictly confined to three files:

| File                         | Reason                                  |
|------------------------------|----------------------------------------|
| `zerogate-ebpf/src/parser.rs` | Packet buffer reads, BPF helper calls  |
| `zerogate-agent/src/umem.rs`  | Page-aligned memory allocation          |
| `zerogate-agent/src/sys.rs`   | `libc::if_nametoindex` FFI call         |

Every `unsafe` block documents:
- Pointer provenance
- Bounds guarantee
- Alignment guarantee (or use of `read_unaligned`)
- Lifetime guarantee
- Aliasing guarantee

The `scripts/audit_no_unsafe.sh` script enforces this boundary in CI.

## Verus Model Boundary

`zerogate-verus` contains a pure formal model that mirrors the runtime:

| Model File        | What It Models                          |
|-------------------|-----------------------------------------|
| `frame_state.rs`  | Frame ownership state machine           |
| `ring_model.rs`   | Ring capacity and uniqueness invariants  |
| `parser_model.rs` | Parser offset bounds and determinism    |
| `abi_model.rs`    | ABI type sizes and alignment            |

The model uses the same names and state values as the runtime implementation but depends on no syscalls, raw pointers, or OS APIs.

## Concurrency and CPU Pinning Model

- **One RX loop per NIC queue**, each running in a dedicated thread.
- **Each thread is pinned to a specific CPU core** via `core_affinity` for cache locality.
- **No shared rings** between threads — each queue owns its own FILL, RX, TX, COMPLETION rings and UMEM.
- **Per-queue statistics** are cache-line padded (64 bytes) to avoid false sharing.
- **Global statistics** are aggregated periodically by the main thread after shutdown.
