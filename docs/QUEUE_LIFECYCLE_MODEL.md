# Queue Lifecycle Model (Pre-MR11 Design Doc)

> **Status: MR10.3 design doc, not implemented.** Forward-looking design for MR11
> (Single `QueueContext` Lifecycle over Fake Rings) and MR11.1
> (`QueueLifecycle.tla`). No code in this MR implements queue lifecycle.

## Scope (future MR11)

`QueueContext` will drive a single AF_XDP-style queue over **fake rings**:

- `QueueContext` create / start / stop / drop.
- `QueueContext` **exclusively owns** its `FramePool` (no global sharing, no
  `Arc<Mutex<FramePool>>` in the hot path).
- Fill bootstrap (seed the fill ring from `Free` frames).
- RX consume (`InFill`/`Kernel` → `RxReady`).
- `RxReady -> UserOwned`.
- `UserOwned` recycle path (`-> InFill`).
- `UserOwned -> TxSubmitted`.
- Completion reap (`TxSubmitted -> Free`; `CompletionSeen` is optional at
  runtime — see [`RING_FRAME_CONTRACT.md`](RING_FRAME_CONTRACT.md)).
- Fake-ring reserve/commit/failure tests.
- All public error paths preserve `FramePool` invariants.
- Real AF_XDP remains `NotImplemented`.

## Out of scope for MR11

Real UMEM kernel registration, real XSK bind, real eBPF load, map syscall
integration, multi-queue, CPU pinning, production metrics, kernel-matrix
testing, privileged integration tests.

## Ownership model (hard decision)

These are **decided**, not open questions, and bind MR11/MR12:

- `QueueContext` **owns `FramePool` exclusively**.
- `FramePool` is **not globally shared**.
- `Arc<Mutex<FramePool>>` is **forbidden in any packet hot path**.
- MR11 is **single-threaded per `QueueContext`**.
- MR12 multi-queue must use **one of**: a **per-queue `FramePool`**, or **explicit
  disjoint frame ranges / UMEM slices**.
- **Cross-queue frame handoff is forbidden** until an explicit protocol is
  designed and reviewed.
- A shared global `FramePool` is **out of scope** for the dataplane hot path.

Rationale: exclusive, single-threaded ownership eliminates lock contention,
false sharing, and ambiguous ownership domains on the hot path, and keeps the
ownership state machine (MR10) reasoning local to one queue.

## Lifecycle states (context level)

```text
Created --start--> Running --stop--> Stopped --drop--> (dropped)
```

- `start` bootstraps the fill ring.
- `Running` performs RX consume / process / TX submit / completion reap.
- `stop` ceases polling; frames retain their ownership states.
- `drop` releases resources; on drop, ownership accounting must still be
  consistent (debug/test asserts may verify this).

## Relationship to the formal model

`QueueLifecycle.tla` (MR11.1) should **compose with or refine**
`FrameOwnership.tla`: the queue actions must only induce legal frame transitions,
so ownership invariants (`FreeListMatchesState`, `TxNotFreeBeforeCompletion`,
etc.) continue to hold under ring operations. Liveness/fairness (e.g. submitted
TX frames are eventually freed) becomes meaningful here and will be considered
then.

## See also

- [`RING_FRAME_CONTRACT.md`](RING_FRAME_CONTRACT.md) — ring↔frame contract and
  atomicity.
- [`ERROR_POLICY.md`](ERROR_POLICY.md), [`INVARIANT_POLICY.md`](INVARIANT_POLICY.md).
- [`../specs/README.md`](../specs/README.md).
