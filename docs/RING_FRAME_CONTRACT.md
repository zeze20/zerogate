# Ring/Frame Contract (Pre-MR11 Design Doc)

> **Status: design doc, not implemented.** This is forward-looking design for
> MR10.3 / MR11. It describes the intended ring↔frame ownership contract and
> atomicity model. No code in this MR implements it. Where it constrains future
> behavior, it defers to [`ERROR_POLICY.md`](ERROR_POLICY.md) and
> [`INVARIANT_POLICY.md`](INVARIANT_POLICY.md).

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

### `Kernel` / `KernelOwned` and the DMA boundary

`Kernel` represents a frame handed to the kernel via the fill ring and not yet
returned as an RX descriptor. The kernel's DMA ownership is **not directly
observable** from Rust — we only know a frame is kernel-owned because we
submitted it and have not seen it come back. The TLA+ model keeps this as an
explicit `Kernel` state; the runtime treats it the same way. `CompletionSeen`
(`Completion`) is needed in both the runtime and the model to enforce that a TX
frame passes through completion before returning to `Free`.

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

Nuance (see [`ERROR_POLICY.md`](ERROR_POLICY.md)):

- If a **`frame_id` cannot be derived**, do not recycle an unknown frame → drop.
- If a **`frame_id` is derivable** and the expected state matches, safe
  recycle/drop may apply.
- An **expected-state mismatch** for a tracked frame is an invariant violation →
  fail-fast.

## Future ring trait direction (document only)

Rings have different semantics, so prefer **separate traits** over one generic
`RingProvider`:

- `FillRing`, `TxRing` — **producer-side**, reservation semantics.
- `RxRing`, `CompletionRing` — **consumer-side**, polling semantics.

MR11's `QueueContext` should ideally be generic from the start:

```text
QueueContext<FILL, RX, TX, COMP>
where FILL: FillRing, RX: RxRing, TX: TxRing, COMP: CompletionRing
```

Not implemented in MR10.2.
