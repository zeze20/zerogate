# Queue Lifecycle Model (Pre-MR11 Design Doc)

> **Status: design doc, not implemented.** Forward-looking design for MR11
> (Single `QueueContext` Lifecycle over Fake Rings) and MR11.1
> (`QueueLifecycle.tla`). No code in MR10.2 implements this.

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
- Completion reap (`TxSubmitted -> CompletionSeen -> Free`).
- Fake-ring reserve/commit/failure tests.
- All public error paths preserve `FramePool` invariants.
- Real AF_XDP remains `NotImplemented`.

## Out of scope for MR11

Real UMEM kernel registration, real XSK bind, real eBPF load, map syscall
integration, multi-queue, CPU pinning, production metrics, kernel-matrix
testing, privileged integration tests.

## Ownership model

- One `QueueContext` owns one `FramePool` (single-threaded per queue).
- MR12/MR13 will create **one `FramePool` per queue** for multi-queue isolation;
  no cross-queue frame sharing.
- No `Arc<Mutex<...>>` around `FramePool` on the packet hot path.

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
