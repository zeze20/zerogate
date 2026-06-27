# Formal Assurance Roadmap

This document describes ZeroGate's formal assurance strategy. It is a roadmap,
not a certification. Nothing here claims that ZeroGate is bug-free or fully
formally verified.

## Honest Claims

What we **can** claim today:

- The MR10 frame ownership lifecycle invariants are **formally specified and
  model-checkable** with TLA+ (see [`FRAME_OWNERSHIP.md`](FRAME_OWNERSHIP.md)).

What we explicitly do **not** claim:

- ZeroGate is not "bug-free".
- ZeroGate is not "fully formally verified".
- Not "all bugs are mathematically impossible".

## MR10.1 — Frame Ownership TLA+ Specification (current)

- Files: `formal/tla/FrameOwnership.tla`, `formal/tla/FrameOwnership.cfg`.
- Models the abstract frame ownership state machine from MR10.
- Checks lifecycle legality, free-list correctness, and ownership consistency
  over a finite frame set with TLC.
- Abstracts the free list as a set; allocation order is not modeled.

## Future Work

The following are planned future assurance efforts. They are not implemented and
are listed only to give direction.

- **MR11 — `QueueLifecycle.tla`**: model the queue loop interactions (fill/RX/TX/
  completion ring submission and reaping) on top of the ownership state machine.
- **MR12 — `MultiQueueIsolation.tla`**: model per-queue `FramePool` instances and
  prove cross-queue frame isolation.
- **MR15 — Verus alignment**: align selected Rust functions with Verus proofs to
  connect the abstract model to the implementation.
- **MR16 — Fuzzing**: differential and property fuzzing of the runtime state
  machine against the model.
- **MR17 — TCB and assumptions**: document the trusted computing base and the
  explicit assumptions each assurance artifact relies on.
- **MR21 — Assurance case**: consolidate evidence into a structured assurance
  case.

## Scope Boundaries

Formal models in this repository check abstract designs. They do not verify the
Linux kernel, the eBPF verifier, the AF_XDP runtime, hardware, or the Rust
compiler, and they do not replace runtime tests or the unsafe audit.
