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

## Reusable `unsafe` contract template

Every `unsafe` block (current and future) must carry a `// SAFETY:` comment that
addresses **all** of the following. Reviewers should reject an `unsafe` block
that leaves any item unstated.

```text
// SAFETY:
// - caller guarantees:   <what the caller must have established before this call>
// - callee guarantees:   <what this block promises on return / on each path>
// - kernel/NIC/verifier: <assumptions about kernel, NIC, or eBPF verifier state>
// - alignment:           <required alignment of every pointer dereferenced>
// - lifetime:            <how long the referenced memory stays valid; who owns it>
// - aliasing:            <no &mut aliasing; no overlapping &mut/&; provenance>
// - memory ordering:     <ordering/atomics required for ring producer/consumer>
// - failure behavior:    <what happens on the error path; never UB, never silent>
// - test coverage:       <which tests exercise this block, incl. failure paths>
```

Checklist (each item is mandatory):

1. **Caller guarantees** — preconditions the caller must satisfy.
2. **Callee guarantees** — postconditions this block upholds on every path.
3. **Kernel/NIC/verifier assumptions** — e.g. descriptor came from a ring the
   kernel populated; verifier-proven bounds for parser pointer math.
4. **Memory alignment assumptions** — alignment of each dereferenced pointer.
5. **Lifetime assumptions** — validity window and owner of the memory.
6. **Aliasing assumptions** — no `&mut` aliasing; provenance is sound.
7. **Memory ordering assumptions** — acquire/release ordering for ring head/tail
   producer↔consumer handoff (relevant once real rings arrive in MR12c).
8. **Failure behavior** — explicit, non-UB, never a silent fallback or fake
   success (see [`ERROR_POLICY.md`](ERROR_POLICY.md)).
9. **Test coverage** — tests that exercise the block, including failure paths.

General rules that still apply:

- Validate all bounds/alignment **before** the unsafe access (defer to
  [`INVARIANT_POLICY.md`](INVARIANT_POLICY.md) release O(1) checks).
- Do **not** widen the unsafe surface beyond the audited file.
- Do **not** expose raw pointers across a public boundary.
- Do **not** use `unsafe` to bypass the ownership state machine.

> **MR10.3 adds no real `unsafe` AF_XDP code.** This template governs the
> adapters that arrive in MR12.

## Future (MR12) unsafe expectations

Real ring/UMEM adapters will need `unsafe` for mmap'd ring access and descriptor
read/write. They must:

- Keep `unsafe` confined to a small, audited adapter module.
- Treat all kernel-provided descriptors as untrusted (validate per
  [`RING_FRAME_CONTRACT.md`](RING_FRAME_CONTRACT.md)).
- Preserve the atomicity model: no externally-visible publish before validation
  and state transition succeed.
- Never silently fall back from zero-copy to copy-mode.
