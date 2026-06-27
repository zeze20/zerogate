# Unsafe Contracts (Pre-MR11 Design Doc)

> **Status: design doc.** Documents the current `unsafe` boundary and the
> contract future `unsafe` (e.g. real ring/UMEM adapters in MR12) must satisfy.
> MR10.2 does not add or change any `unsafe` code.

## Current policy

`unsafe` is forbidden except in an explicit allow-list enforced by
[`../scripts/audit_no_unsafe.sh`](../scripts/audit_no_unsafe.sh) in CI:

| File | Why unsafe is allowed |
|------|----------------------|
| `zerogate-ebpf/src/parser.rs` | Packet pointer access under the eBPF verifier's bounds model |
| `zerogate-agent/src/umem.rs` | UMEM buffer allocation |
| `zerogate-agent/src/sys.rs` | Low-level system calls / FFI |

All other modules — including `frame.rs`, `rings.rs`, `xsk.rs`, `maps.rs` — must
be `unsafe`-free. No public API exposes raw pointers.

## Contract for any allowed `unsafe`

Each `unsafe` block must:

1. Have a `// SAFETY:` comment stating the invariant it relies on.
2. Validate all bounds/alignment **before** the unsafe access (defer to
   [`INVARIANT_POLICY.md`](INVARIANT_POLICY.md) release O(1) checks).
3. Not widen the unsafe surface beyond the audited file.
4. Not expose raw pointers across a public boundary.
5. Not be used to bypass the ownership state machine.

## Future (MR12) unsafe expectations

Real ring/UMEM adapters will need `unsafe` for mmap'd ring access and descriptor
read/write. They must:

- Keep `unsafe` confined to a small, audited adapter module.
- Treat all kernel-provided descriptors as untrusted (validate per
  [`RING_FRAME_CONTRACT.md`](RING_FRAME_CONTRACT.md)).
- Preserve the atomicity model: no externally-visible publish before validation
  and state transition succeed.
- Never silently fall back from zero-copy to copy-mode.
