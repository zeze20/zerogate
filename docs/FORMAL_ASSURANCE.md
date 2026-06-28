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

## MR10.2 — Formal Tooling & Invariant Policy (current)

Standardizes how the formal model is run and documents the error/invariant
policies that future runtime code must follow.

- **Runner:** [`../scripts/run_tla.sh`](../scripts/run_tla.sh) — `quick`
  (N=3, N=4), `extended` (adds N=5), `frame-ownership` (default cfg). It locates
  Java + `tla2tools.jar` and runs **real** TLC; it never fakes a pass.
- **CI:** `.github/workflows/tla.yml` (GitHub Actions) and the `tla_model_check`
  job in `.gitlab-ci.yml` (stage `formal`) both install Java, download the pinned
  `tla2tools.jar`, set `TLA2TOOLS_JAR`, and run `./scripts/run_tla.sh quick` —
  real TLC, no fake-green CI. The GitHub workflow triggers on `pull_request`
  (any base), `workflow_dispatch`, and `push` to `main`.
- **Specs index:** [`../specs/README.md`](../specs/README.md).
- **Error policy:** [`ERROR_POLICY.md`](ERROR_POLICY.md) — panic/fail-fast for
  internal invariant violations vs `Result`/drop/retry/metric for external and
  resource failures.
- **Invariant policy:** [`INVARIANT_POLICY.md`](INVARIANT_POLICY.md) — release
  always-on O(1) memory-safety checks vs debug/test-only O(n) diagnostic scans.
- **Local tooling setup:** [`DEVELOPMENT.md`](DEVELOPMENT.md) — Java +
  `tla2tools.jar` (`TLA2TOOLS_JAR`) for running the model checks locally.
- **MR10.3 contracts / pre-MR11 design docs:**
  [`RING_FRAME_CONTRACT.md`](RING_FRAME_CONTRACT.md),
  [`DESCRIPTOR_VALIDATION.md`](DESCRIPTOR_VALIDATION.md),
  [`QUEUE_LIFECYCLE_MODEL.md`](QUEUE_LIFECYCLE_MODEL.md),
  [`UNSAFE_CONTRACTS.md`](UNSAFE_CONTRACTS.md),
  [`THREAT_MODEL.md`](THREAT_MODEL.md),
  [`MR11_ACCEPTANCE_CRITERIA.md`](MR11_ACCEPTANCE_CRITERIA.md),
  [`MR10_3_NOTES.md`](MR10_3_NOTES.md).

N>=8 / symmetry-reduced checking is **not** implemented; the runner does not
offer it and says so explicitly rather than pretending.

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
