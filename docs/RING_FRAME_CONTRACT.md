# Ring/Frame Contract (Pre-MR11 Design Doc)

> **Status: MR10.3 contract, not yet implemented in the runtime.** This is the
> implementation-ready ring↔frame ownership contract and atomicity model that
> MR11 must satisfy. No code in this MR implements queue lifecycle behavior.
> Where it constrains future behavior, it defers to
> [`ERROR_POLICY.md`](ERROR_POLICY.md) and
> [`INVARIANT_POLICY.md`](INVARIANT_POLICY.md). Companion docs:
> [`DESCRIPTOR_VALIDATION.md`](DESCRIPTOR_VALIDATION.md) and
> [`MR11_ACCEPTANCE_CRITERIA.md`](MR11_ACCEPTANCE_CRITERIA.md).

## Runtime frame states (MR10, implemented)

The implemented `FrameState` (see [`FRAME_OWNERSHIP.md`](FRAME_OWNERSHIP.md)):

- `Free`
- `InFill`
- `Kernel` (kernel-owned after fill submission)
- `Rx`
- `User`
- `Tx`
- `Completion`

For the queue runtime, the same states are sometimes described with
runtime-oriented names. The mapping intended for MR11:

| Runtime concept | MR10 `FrameState` |
|-----------------|-------------------|
| Free | `Free` |
| InFill | `InFill` |
| RxReady | `Rx` |
| UserOwned | `User` |
| TxSubmitted | `Tx` |
| CompletionSeen | `Completion` |

### `Kernel` / `KernelOwned` and the DMA boundary (TLA+ abstract state)

`Kernel` (`KernelOwned` in model terms) represents a frame handed to the kernel
via the fill ring and not yet returned as an RX descriptor.

- `KernelOwned` **may exist in TLA+** as an explicit observational state.
- It is **not necessarily a directly observable Rust runtime state**: Rust never
  watches the actual DMA write. We only know a frame is kernel-owned because we
  submitted it to FILL and have not seen it return.
- The kernel DMA boundary is enforced **by protocol, not by observation**: a
  frame in `InFill`/`Kernel` is off-limits to userspace until an RX descriptor
  for it is observed. `InFill` means **userspace must not read or write the
  packet bytes**.

### Decision: `CompletionSeen` is optional at runtime

The safety-critical requirement on the TX→reuse path is that completion handling
**atomically** (a) validates the frame is currently `TxSubmitted` and (b)
releases it to `Free`. A distinct runtime `CompletionSeen` state is a
**debugging/observability aid**, not a safety requirement.

- **TLA+:** `CompletionSeen` is retained as an abstract state so the model can
  assert a TX frame is never freed without passing through completion
  (`TxNotFreeBeforeCompletion`).
- **Runtime (MR10, implemented):** the `FrameState` enum currently includes
  `Completion`, so today the runtime models `TxSubmitted -> Completion -> Free`.
  This MR does **not** change that.
- **Runtime (MR11 decision):** the runtime is *permitted* to collapse this into a
  single atomic `TxSubmitted -> Free` transition on completion observation,
  because the safety property is the atomic validate-then-release, not the
  intermediate state. Whether MR11 keeps `Completion` for observability or
  collapses it is an MR11 implementation choice; either satisfies this contract
  provided the transition validates `TxSubmitted` and fails-fast otherwise.

## Required transitions

| Transition | Trigger | Authorized component | Pre | Post | Failure behavior | Metric/log | Test cases |
|-----------|---------|----------------------|-----|------|------------------|-----------|-----------|
| `Free -> InFill` | Fill bootstrap / refill | QueueContext (producer) | frame `Free`, fill slot reserved | frame `InFill`, removed from free list | reserve fail → no-op `Result`; wrong state → fail-fast | inc `fill.submitted` | alloc from empty pool; alloc all |
| `InFill -> RxReady` | RX descriptor observed for frame | QueueContext (RX consumer) | frame `InFill` | frame `Rx` | unknown frame_id → drop; wrong state → fail-fast | inc `rx.received` | rx for valid frame; rx for non-InFill (must fail-fast) |
| `RxReady -> UserOwned` | App acquires for processing | QueueContext / app | frame `Rx` | frame `User` | wrong state → fail-fast | — | acquire valid; acquire non-Rx |
| `UserOwned -> InFill` | RX recycle (no TX) | QueueContext | frame `User` | frame `InFill`; **not** pushed to free | wrong state → fail-fast | inc `frame.recycled` | recycle path; recycle non-User |
| `UserOwned -> TxSubmitted` | App submits for transmit | QueueContext (producer) | frame `User`, TX slot reserved | frame `Tx` | reserve fail → no-op `Result`; wrong state → fail-fast | inc `tx.submitted` | submit valid; submit when TX full |
| `TxSubmitted -> CompletionSeen` | Completion descriptor observed | QueueContext (completion consumer) | frame `Tx` | frame `Completion` | unknown frame_id → drop; wrong state → fail-fast | inc `tx.completed` | completion for valid; completion w/o submit (fail-fast) |
| `CompletionSeen -> Free` | Release after completion | QueueContext | frame `Completion` | frame `Free`; pushed to free **exactly once** | double release → fail-fast | inc `frame.freed` | release valid; double release (fail-fast) |

## Invalid transitions (must fail-fast)

These imply internal inconsistency, not external input:

- `Free -> TxSubmitted`
- `InFill -> UserOwned` without an RX descriptor
- `TxSubmitted -> UserOwned`
- `CompletionSeen` without a previous `TxSubmitted`
- `DoubleFree`
- double TX submit of the same frame
- duplicate RX descriptor for a frame already `Rx`/`User` (when it implies
  internal inconsistency)

## Key ownership rule

> A frame submitted to **FILL** is **not** accessible by userspace until an
> **RX descriptor for that frame is observed**. Between fill submission and RX
> observation the frame is kernel-owned (`Kernel`); touching its buffer in that
> window is a use-after-handoff bug.

## Ring operation atomicity model

Producer-side flow (FILL, TX), in order:

1. **reserve** ring slot
2. **validate** descriptor/frame
3. **transition** FramePool state
4. **write** descriptor
5. **commit/publish**
6. **rollback** only if failure occurs **before** external visibility

Rules:

- If **reserve fails**, FramePool state must **not** change.
- If **validation fails**, FramePool state must **not** change.
- If **transition fails** due to an impossible state, **panic / fail-fast**.
- If **descriptor write fails before publish**, rollback **may** be allowed.
- After **commit/publish succeeds**, the frame **may be externally visible**.
- After **commit/publish succeeds**, rollback is **forbidden / unsafe**.
- Fake rings **may** simulate a commit failure **before** external visibility
  (to test rollback paths).
- Real AF_XDP commit semantics are mapped later in **MR12c**. Do **not** assume
  commit-after-publish rollback is safe.

### Defining "external visibility"

"External visibility" is the point after which another party (the kernel/NIC)
may consume the descriptor, so userspace can no longer safely reclaim ownership:

- **Fill ring:** visibility begins when the fill descriptor is committed/published
  such that **the kernel may consume it** (start a DMA into that frame).
- **TX ring:** visibility begins when the TX descriptor is committed/published
  such that **the kernel/NIC may consume it** for transmit.

After publish, userspace **must not assume it can roll back ownership**. The
FramePool transition (step 3) therefore happens *before* the descriptor write and
commit, and a rollback is only legal in the window between transition and commit
(steps 3→5), never after step 5.

## Descriptor validation contract

An accepted descriptor must satisfy **all** of:

- `addr` is inside UMEM
- `addr` maps to exactly one frame
- frame-index conversion cannot overflow
- `len > 0`
- `len <= usable_frame_size`
- `addr + len` cannot overflow
- `addr + len <= frame_start + frame_size`
- headroom/offset behavior is explicit
- multi-buffer is **out of scope** unless explicitly implemented
- jumbo / multi-frame packet handling is **out of scope**

Nuance (see [`ERROR_POLICY.md`](ERROR_POLICY.md)):

- If a **`frame_id` cannot be derived**, do not recycle an unknown frame → drop.
- If a **`frame_id` is derivable** and the expected state matches, safe
  recycle/drop may apply.
- An **expected-state mismatch** for a tracked frame is an invariant violation →
  fail-fast.

The full per-case classification (panic vs Result, state change vs no-op,
metric, required tests) lives in
[`DESCRIPTOR_VALIDATION.md`](DESCRIPTOR_VALIDATION.md).

## No-orphan-frame invariant

> At **every public API boundary**, every frame must be accounted for in
> **exactly one** ownership location/state.

Valid accounting locations/states: `Free`, `InFill`, `RxReady` (`Rx`),
`UserOwned` (`User`), `TxSubmitted` (`Tx`), and `CompletionSeen` (`Completion`,
if used). A pre-commit, rollbackable reservation counts as part of the
producer-side transition (see atomicity model), not as a separate location.

No frame may be:

- absent from all accounting (an **orphan**),
- present in more than one accounting location (double accounting),
- marked `InFill` without being committed to the fill ring **or** held in a
  pre-commit rollbackable reservation,
- marked `TxSubmitted` without being committed to the TX ring **or** held in a
  pre-commit rollbackable reservation.

Enforcement split (see [`INVARIANT_POLICY.md`](INVARIANT_POLICY.md)):

- **Release, O(1), always-on:** the per-frame expected-state guard on every
  transition, and the bounded free-list capacity guard (below) — these prevent
  the *creation* of an orphan/double-accounted frame.
- **Debug/test, O(n):** a full "all frames accounted for exactly once" scan over
  the pool (no frame Free-but-also-tracked, no frame missing). This is an **MR11
  acceptance criterion** (run after each fake-ring operation in tests), not a
  hot-path check. See [`MR11_ACCEPTANCE_CRITERIA.md`](MR11_ACCEPTANCE_CRITERIA.md).

## Bounded free list contract (future requirement)

The free list backing `Free` frames must be **truly bounded**, not merely
`VecDeque::with_capacity(frame_count)`:

- `with_capacity` alone is **insufficient** — it permits growth past capacity.
- `push` when `len == capacity` must **panic / fail-fast**.
- Capacity-exceeded means **internal accounting corruption** (a frame was freed
  that wasn't owned, or freed twice), **not** normal resource exhaustion.
- Duplicate free entries must be caught in **debug/test diagnostics** (O(n) scan).
- The **O(1) capacity guard is release always-on**; the **O(n) duplicate scan is
  debug/test only**.

Contrast with `FramePoolExhausted` (an empty free list on alloc), which is a
**resource** condition → `Result`/drop/metric, never a panic.

> Implementing a dedicated `BoundedFreeList` type is deferred: it is only
> appropriate if it can be added without a broad `FramePool` refactor. MR10.3
> documents the contract; see Phase 2 note in
> [`MR10_3_NOTES.md`](MR10_3_NOTES.md).

## Fake ring semantics (MR11 requirement, document only)

MR11's fake rings must **not** be oversimplified push/pop queues. They must be
able to exercise every real-ring failure mode so the QueueContext invariants are
genuinely tested. Required capabilities:

- reserve / write descriptor / commit / rollback-before-commit / poll
- completion injection
- ring full / ring empty
- reserve failure
- write failure before publish
- commit failure before publish
- duplicate descriptor injection
- stale/duplicate completion injection
- invalid descriptor injection

## Future ring trait direction (document only)

Rings have different semantics, so prefer **separate traits** over one generic
`RingProvider` (a single generic trait would hide those semantic differences):

- `FillRing`, `TxRing` — **producer-side**, reservation semantics.
- `RxRing`, `CompletionRing` — **consumer-side**, polling semantics.

MR11's `QueueContext` should be generic over these from the start, so that the
real AF_XDP adapters in MR12c slot in without a refactor:

```text
QueueContext<FILL, RX, TX, COMP>
where FILL: FillRing, RX: RxRing, TX: TxRing, COMP: CompletionRing
```

Not implemented in MR10.3 (see Phase 2 decision in
[`MR10_3_NOTES.md`](MR10_3_NOTES.md)).
