# Security Invariants

This document records the security invariants enforced across ZeroGate subsystems.

---

## Unsafe Confinement

Unsafe code is allowed only in:

- `zerogate-ebpf/src/parser.rs` — eBPF pointer boundary access.
- `zerogate-agent/src/umem.rs` — UMEM memory boundary (currently safe Vec-based; reserved for future page-aligned allocation).
- `zerogate-agent/src/sys.rs` — syscall/FFI boundary (`libc::if_nametoindex`).

All other files must remain safe. The `scripts/audit_no_unsafe.sh` script enforces this in CI.

---

## UMEM Memory Invariants

The following invariants are enforced by `UmemConfig::validate()` and `UmemRegion` methods:

1. **frame_count** must be greater than zero and a power of two.
2. **frame_size** must be 2048 or 4096.
3. **total_size** (`frame_count * frame_size`) must not overflow `usize`.
4. **headroom** must be strictly less than `frame_size`.
5. **Frame indices** must be in `[0, frame_count)`. Any index `>= frame_count` is rejected with a structured error.
6. **UMEM offsets** must be in `[0, total_size)`. An offset `>= total_size` is rejected.
7. **UMEM offsets** must be frame-aligned (`offset % frame_size == 0`). Unaligned offsets are rejected.
8. **No raw pointer exposure** through the UMEM public API. Frame identity is expressed only through `FrameIndex` and `UmemAddr`.
9. **No packet buffer reuse** without ownership validation. Frame ownership tracking is deferred to future MRs but must be enforced before production packet access.
10. **No AF_XDP registration claim** without real kernel registration. The current allocation is `Vec`-backed and does not claim kernel UMEM registration.

---

## BPF Map Invariants

1. Map keys and values use ABI-stable types from `zerogate-common` (`#[repr(C, packed)]`).
2. The `InMemoryMapBackend` is test-only and does not claim kernel map success.
3. Map update/delete operations return structured errors, never panic.
4. No raw pointers are exposed in the map synchronization API.

---

## AF_XDP Ring Invariants

1. **Descriptors must point inside UMEM.** Any `addr >= total_size` is rejected.
2. **Descriptors must be frame-aligned** where required (`addr % frame_size == 0`).
3. **Descriptor length must not exceed frame_size.** Oversized descriptors are rejected.
4. **Descriptor length must be greater than zero** (strict policy in MR9).
5. **Ring capacity must be enforced.** Submission beyond capacity returns `RingFull`.
6. **No descriptor submission bypasses validation.** All public ring paths validate descriptors.
7. **Fake rings are test-only.** They do not model kernel ring memory and must not be used in production.
8. **Future frame ownership integration** must prevent duplicate ownership of the same frame.
9. **Future TX frames must not be recycled** before the kernel signals completion.
10. **No raw ring memory or raw pointers** are exposed through ring trait APIs.

---

## eBPF Parser Invariants

1. All pointer arithmetic in the parser is bounds-checked against `data_end`.
2. The parser operates within the eBPF verifier's static analysis constraints.
3. Unsafe is confined to `parser.rs` for required eBPF pointer access patterns.
