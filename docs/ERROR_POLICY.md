# ZeroGate Error Policy — Panic vs Result

This document defines when ZeroGate **panics / fails fast** versus when it
returns a **`Result`** (or drops/retries/records a metric). It is policy for
current and future code (queue lifecycle, ring adapters, etc.). MR10.2 documents
the policy; it does not change existing runtime behavior.

## Core principle

> **A memory-safety / ownership invariant violation is a bug in ZeroGate's own
> logic → panic / fail-fast.**
>
> **An external, resource, or packet-level failure is an expected condition →
> `Result` (drop / retry / metric), never memory corruption.**

The distinction is *whose fault it is*:

- If reaching a state is **impossible** given correct internal logic, it means
  our accounting is already corrupt. Continuing could cause use-after-free /
  double-free / data corruption. **Fail fast** so the bug is loud and contained.
- If a failure is caused by **the outside world** (a full ring, an exhausted
  pool, a malformed packet, an unsupported kernel feature), it is a normal
  operating condition. **Return a typed error** and let the caller drop/retry/
  account for it. State must remain consistent.

A corollary used throughout: **on any `Result::Err`, observable state
(`FramePool` states, free list, ring indices) must be unchanged** — the
operation is atomic from the API's perspective.

## Panic / fail-fast (internal invariant violations)

These indicate ZeroGate's own accounting is corrupt. They must not be "handled"
by limping along.

| Condition | Why fail-fast |
|-----------|---------------|
| `FrameOwnershipCorrupt` | Ownership accounting is inconsistent; any further frame op is unsafe |
| `DoubleOwnership` | A frame is owned by two parties → classic UAF/corruption source |
| `DoubleFree` | A frame released twice → free-list corruption / aliasing |
| `InvalidTransition` (internally driven) | Code attempted an illegal state move it should never request |
| `ImpossibleState` | Reached a state unreachable under correct logic |
| `FreeListCapacityExceeded` | Pushing beyond `frame_count` means accounting overflow, not resource exhaustion |
| Ring/Frame impossible consistency violation | e.g. a committed frame in a state that contradicts the ring it sits in |

Note on the current code: MR10's public `FramePool` API returns
`InvalidFrameTransition` as a **`Result`** when a *caller* requests an illegal
transition (defensive, state unchanged). The **panic** policy above applies to
conditions that can only arise from *internal* corruption (e.g. a
debug/diagnostic scan finding duplicate ownership, or a future free-list push
that would exceed capacity). See
[`INVARIANT_POLICY.md`](INVARIANT_POLICY.md) for which checks run in release vs
debug.

## Result / drop / retry / metric (external & resource failures)

These are expected and must leave state consistent.

| Condition | Typical handling |
|-----------|------------------|
| `FramePoolExhausted` | No free frames right now → drop/backpressure; retry later |
| `RingFull` | Producer ring has no slots → retry next tick / drop with metric |
| `RingEmpty` | Consumer ring has nothing → normal poll result, not an error event |
| `ReserveFailure` (before any state transition) | Could not reserve a slot → no-op, state unchanged |
| `MalformedPacket` | Bad input from the wire → drop + metric |
| `InvalidDescriptor` | Descriptor failed validation → drop (see nuance below) |
| `InvalidDescriptorAddr` | addr outside UMEM / not frame-aligned → drop |
| `InvalidDescriptorLen` | `len == 0` or `len > usable_frame_size` → drop |
| `DescriptorCrossesFrameBoundary` | `addr + len` exceeds the frame → drop |
| `TxUnavailable` | TX path not ready → backpressure/drop |
| `PolicyUnavailable` | Depending on mode: fail-closed (drop) or fail-open per config |
| `BpfMapUnavailable` | Map backend not present → `NotImplemented`/error, no fake success |
| `XskBindUnavailable` | Real XSK bind not implemented → `NotImplemented` |
| `UMemRegistrationFailure` | Kernel UMEM registration failed/unimpl → error |
| `XdpProgramLoadFailure` | eBPF load failed → error |
| `KernelFeatureUnsupported` | Required kernel feature missing → error, documented |

## The important nuance: same surface, different cause

Some ring/descriptor conditions are **external bad input** in one context and an
**impossible internal invariant** in another. The rule:

- **Descriptor came from outside** (kernel RX, a peer): treat validation failure
  as **external** → `Result`/drop. We cannot trust external descriptors, so a
  bad one is expected, not a bug in us.
- **Descriptor/transition is internally generated** and still inconsistent
  (e.g. we built a descriptor that crosses a frame boundary, or we expected a
  frame to be in state X and it is in state Y): that is an **internal invariant
  violation** → fail-fast.

Concretely for descriptors:

- If a **`frame_id` cannot be derived** from a descriptor (addr not mappable),
  do **not** attempt to recycle an unknown frame → drop as external error.
- If a **`frame_id` is derivable** and its **expected state matches**, a safe
  recycle/drop policy may apply.
- If the **expected-state check fails** for an internally-tracked frame, that is
  an invariant violation → fail-fast.

## Boundary behavior (no fake success)

For systems not yet implemented (real XSK bind, UMEM kernel registration, eBPF
load, BPF map syscalls), ZeroGate returns explicit `NotImplemented`-style errors
rather than pretending to succeed or silently falling back to fake behavior.
Silent fallback (including zero-copy → copy-mode) is forbidden unless explicitly
designed and documented.

## Quick decision checklist

1. Could this state arise if all ZeroGate logic is correct? **No → panic.**
2. Is the failure caused by the outside world (ring/pool/packet/kernel)?
   **Yes → `Result` + drop/retry/metric.**
3. On `Err`, is observable state unchanged? **It must be.**
4. Are we tempted to fake success or silently fall back? **Never.**
