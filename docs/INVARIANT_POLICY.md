# ZeroGate Invariant Policy — Release vs Debug/Test

This document defines which invariants run **always (release)** and which run
**only in debug/test or diagnostic modes**. The guiding constraint is that the
packet hot path must stay fast while memory-safety corruption is still caught.
MR10.2 documents the policy; it does not change existing runtime behavior.

## Principle

> **Release always-on invariants** must be **O(1)** and strictly
> memory-safety-critical: they prevent corruption.
>
> **Debug/test-only invariants** may be **O(n)**: they provide diagnostics and
> full-consistency checks, and must never run in the release hot path.

Release checks exist to **prevent memory-safety corruption**, not to provide full
diagnostics. Diagnostics belong in debug builds, tests, and explicit diagnostic
modes.

## Release always-on invariants (O(1), memory-safety critical)

These are cheap, local checks that gate every relevant operation in all builds:

- **`frame_id` bounds check** — index is within `0..frame_count`.
- **expected-state transition check** — the frame is in the exact state the
  transition requires before mutating it.
- **descriptor addr within UMEM** — `addr` maps inside the registered region.
- **descriptor len valid** — `len > 0` and `len <= usable_frame_size`.
- **descriptor frame-boundary check** — `addr + len` stays within one frame and
  does not overflow.
- **bounded free-list capacity guard** — pushing when `len == capacity` is an
  accounting-corruption signal (see below); the guard is O(1) and always on.
- **ring index bounds** (when rings exist) — producer/consumer indices stay in
  range.
- **no commit with invalid reservation** (when rings exist) — never publish a
  slot that was not validly reserved.

These are the checks that, if removed, could allow use-after-free, double-free,
out-of-bounds access, or aliasing. They stay in release.

## Debug/test-only invariants (O(n), diagnostics)

These give strong whole-structure guarantees but cost O(n) (or worse) and must
not run on the release hot path. Run them in tests, debug builds
(`debug_assert!`), or explicit diagnostic entry points:

- **full duplicate-ownership scan** — no frame owned in two places
  (today: `FramePool::assert_no_duplicate_ownership`).
- **duplicate free-list-entry scan** — no index appears twice in the free list.
- **all-frames-accounted-for scan** — every frame is in exactly one place.
- **no-frame-in-two-rings global scan** — cross-ring consistency.
- **all-queue-states-internally-consistent scan** — full queue invariant sweep.
- **full pool-consistency scan** — `free` set equals the set of `Free` frames
  across the whole pool (the runtime analogue of the TLA+ `FreeListMatchesState`
  invariant).

## Hot-path rule

- **O(n) invariant scans must not be called from the packet hot path in
  release.** They may be invoked manually, in tests, or under a diagnostic flag.
- Release per-operation cost must remain O(1).
- Prefer `debug_assert!`/`cfg(debug_assertions)`/`cfg(test)` for O(n) checks so
  they vanish from release builds.

## Bounded free list (future requirement, documented not yet enforced)

Recorded here so MR11+ implements it correctly:

- `VecDeque::with_capacity(frame_count)` is **not sufficient** — capacity is a
  hint, not a hard bound; a `VecDeque` can still grow.
- The free list must be **bounded**: pushing when `len == capacity` must
  **panic / fail-fast** (this is `FreeListCapacityExceeded`).
- Capacity-exceeded means **internal accounting corruption**, not normal
  resource exhaustion — it is a fail-fast condition, not a `Result`.
- **Duplicate free entries** must be caught in **debug/test diagnostics** (the
  O(n) scan above), not in the release hot path.
- The **O(1) capacity guard** is **release always-on**.

This is a forward-looking requirement; MR10.2 does not modify `FramePool`.

## Relationship to the formal model

The TLA+ invariants in `formal/tla/FrameOwnership.tla` are the *exhaustively
checked* specification of these properties over a finite model:

| TLA+ invariant | Runtime analogue | Where enforced |
|----------------|------------------|----------------|
| `TypeOK`, `FrameInExactlyOneState` | type system + `frame_id` bounds | release O(1) |
| expected-state for each action | expected-state transition check | release O(1) |
| `FreeListMatchesState` | full pool-consistency scan | debug/test O(n) |
| `NonFreeNotInFree`, per-state `*NotFree` | free-list / state coupling | partly O(1) guards, full check debug/test |
| `OwnershipConsistent` | duplicate-ownership + pool scan | debug/test O(n) |

Model checking covers the *design*; release O(1) guards prevent corruption at
runtime; debug/test O(n) scans catch accounting drift during development.

## See also

- [`ERROR_POLICY.md`](ERROR_POLICY.md) — panic vs `Result` classification.
- [`FRAME_OWNERSHIP.md`](FRAME_OWNERSHIP.md) — the ownership state machine.
- [`../specs/README.md`](../specs/README.md) — running the formal model.
