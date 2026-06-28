# MR10.3 Notes — Ring/Frame Contract & Atomicity

> **Status: MR10.3 (docs + tooling).** This MR closes the remaining MR10.2
> tooling gaps (MR10.2.1) and turns the pre-MR11 design docs into precise,
> implementation-ready contracts. **No queue-lifecycle, AF_XDP, or eBPF runtime
> code is implemented.**

## What changed

### Phase 0 — MR10.2.1 tooling closure

- **GitHub Actions** ([`../.github/workflows/tla.yml`](../.github/workflows/tla.yml)):
  real TLC via `./scripts/run_tla.sh quick`; `actions/setup-java` (Temurin 21);
  pinned `tla2tools.jar` download into `TLA2TOOLS_JAR`. Triggers on
  `pull_request` (any base, so stacked PRs run), `workflow_dispatch` (mode
  input), and `push` to `main`. No fake-green.
- **`scripts/run_tla.sh`**: now prefers the `TLA2TOOLS_JAR` env var (legacy
  `TLA_TOOLS_JAR` still honored). `.gitlab-ci.yml` updated to match and to fail
  if the jar download is empty.
- **Local dev docs** ([`DEVELOPMENT.md`](DEVELOPMENT.md)): Java + `tla2tools.jar`
  setup, `TLA2TOOLS_JAR`, and how CI obtains the jar. `tla2tools.jar` is
  intentionally **not vendored**.

### Phase 1 — MR10.3 contracts

- [`RING_FRAME_CONTRACT.md`](RING_FRAME_CONTRACT.md): TLA+ `KernelOwned` vs
  runtime; the **`CompletionSeen` is optional at runtime** decision;
  external-visibility definitions for fill/TX; **no-orphan-frame invariant**;
  **bounded free-list contract**; **fake-ring semantics**; ring-trait direction.
- [`DESCRIPTOR_VALIDATION.md`](DESCRIPTOR_VALIDATION.md): per-case classification
  matrix (panic vs Result, state change, metric, tests).
- [`QUEUE_LIFECYCLE_MODEL.md`](QUEUE_LIFECYCLE_MODEL.md): **QueueContext owns
  FramePool exclusively** hard decision.
- [`UNSAFE_CONTRACTS.md`](UNSAFE_CONTRACTS.md): reusable `unsafe` SAFETY template.
- [`THREAT_MODEL.md`](THREAT_MODEL.md): assets, trust boundaries, attacker
  capabilities, out-of-scope attackers, required security behavior.
- [`MR11_ACCEPTANCE_CRITERIA.md`](MR11_ACCEPTANCE_CRITERIA.md): 24-item MR11
  merge bar.

## Phase 2 decision — no Rust code added

The task permitted *tiny, isolated* Rust scaffolding (ring trait stubs, an error
enum, a `BoundedFreeList` skeleton) **only** if it changed no runtime behavior
and required no broad `FramePool` refactor. **We deliberately added none**, for
these reasons:

- **Unused-item lint risk.** `zerogate-agent` is a binary crate; trait/type
  definitions not yet wired into `QueueContext` would be dead code, and CI runs
  `clippy -D warnings`. Silencing with `#[allow(dead_code)]` would add noise that
  exists only to satisfy the lint — a smell this project avoids.
- **`BoundedFreeList` is not isolated.** The current free list lives inside
  `FramePool`; making it a bounded type touches the pool's alloc/free paths —
  exactly the "broad refactor" the task says to avoid. Better done in MR11 where
  it is exercised.
- **Ring traits belong with their first consumer.** Defining
  `FillRing`/`RxRing`/`TxRing`/`CompletionRing` is most valuable when
  `QueueContext` is generic over them (MR11), so their shape is validated by a
  real user rather than guessed now.

Per the task's final rule — *prefer a clearer contract over more implementation,
and document follow-up when uncertain* — these are specified in the docs above
and left for MR11. Test count stays **233**; no `unsafe` added.

## Follow-ups

- **MR11** — implement `QueueContext` over fake rings to the
  [`MR11_ACCEPTANCE_CRITERIA.md`](MR11_ACCEPTANCE_CRITERIA.md) bar (incl. the
  `BoundedFreeList` and ring traits documented here).
- **MR11.1** — `QueueLifecycle.tla` composing/refining `FrameOwnership.tla`.
- **MR11.2** — fault-injection / property tests over fake rings.
- **MR12a/b/c** — real UMEM register + XSK bind / eBPF load + maps / real ring
  adapter (first real `unsafe` under the [`UNSAFE_CONTRACTS.md`](UNSAFE_CONTRACTS.md)
  template).
