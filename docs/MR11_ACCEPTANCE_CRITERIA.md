# MR11 Acceptance Criteria

> **Status: acceptance checklist for a future MR.** Defined in MR10.3 so MR11
> (Single `QueueContext` Lifecycle over Fake Rings) has an objective merge bar.
> MR11 must not merge unless every item below holds. See
> [`QUEUE_LIFECYCLE_MODEL.md`](QUEUE_LIFECYCLE_MODEL.md),
> [`RING_FRAME_CONTRACT.md`](RING_FRAME_CONTRACT.md), and
> [`DESCRIPTOR_VALIDATION.md`](DESCRIPTOR_VALIDATION.md).

## Structure / ownership

1. `QueueContext` is **generic over ring traits** (`FILL: FillRing`,
   `RX: RxRing`, `TX: TxRing`, `COMP: CompletionRing`).
2. `QueueContext` **exclusively owns** its `FramePool`.
3. **No global/shared `FramePool`** exists; no `Arc<Mutex<FramePool>>` on the hot
   path.

## Lifecycle / data paths (over fake rings)

4. Fill bootstrap works over fake rings.
5. Fill bootstrap **ring-full path produces no orphan frame**.
6. RX descriptor validation exists (per [`DESCRIPTOR_VALIDATION.md`](DESCRIPTOR_VALIDATION.md)).
7. RX consume path validates `InFill -> RxReady -> UserOwned`.
8. `UserOwned` recycle path is safe.
9. `UserOwned -> TxSubmitted` TX path is safe.
10. TX submit failure **pre-commit produces no orphan frame**.
11. Completion reap returns a `TxSubmitted` frame to `Free`.
12. Completion **without** a prior `TxSubmitted` **fails-fast** where it implies
    invariant corruption.

## Error classification

13. `FramePoolExhausted` is `Result`/drop/metric, **not** panic.
14. Double-free / double-ownership / invalid transition **panic** (fail-fast).

## Fault-injection tests (fake rings)

15. Fake-ring **reserve failure** is tested.
16. Fake-ring **write failure before publish** is tested.
17. Fake-ring **commit failure before publish** is tested.
18. **Duplicate RX descriptor** is tested.
19. **Duplicate completion** is tested.
20. **Descriptor crossing frame boundary** is tested.
21. **No-orphan-frame invariant** is tested (O(n) check after each operation).

## Performance / discipline

22. O(n) scans are **not** in the release hot path.
23. Hot path has **no per-packet allocation or log formatting**.
24. Real AF_XDP remains `NotImplemented` unless fully implemented later.

---

These map directly onto the contracts in MR10.3: items 4–12 to the transition
table and atomicity model in `RING_FRAME_CONTRACT.md`; items 6, 18–20 to the
matrix in `DESCRIPTOR_VALIDATION.md`; items 13–14 to `ERROR_POLICY.md`; items
21–22 to the no-orphan invariant and `INVARIANT_POLICY.md`.
