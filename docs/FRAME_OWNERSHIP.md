# Frame Ownership State Machine (MR10)

## Purpose

MR10 introduces a frame ownership state machine that tracks the lifecycle of
every UMEM frame. Each frame is in exactly one state at a time, and only legal
transitions are permitted. This prevents use-after-free, double-free, and
ownership confusion in the AF_XDP dataplane.

## Lifecycle Diagram

```
               +------+
               | Free |<------------------+
               +------+                   |
                  |                        |
          allocate_for_fill          release_completion
                  |                        |
                  v                        |
              +--------+            +------------+
      +------>| InFill |            | Completion |
      |       +--------+            +------------+
      |           |                        ^
      |    mark_kernel_owned          complete_tx
      |           |                        |
      |           v                        |
      |       +--------+              +----+
      |       | Kernel |              | Tx |
      |       +--------+              +----+
      |           |                     ^
      |        mark_rx             submit_tx
      |           |                     |
      |           v                     |
      |       +----+               +------+
      |       | Rx |               | User |---+
      |       +----+               +------+   |
      |           |                  ^    |    |
      |       acquire_user           |    +----+
      |           +------------------+   recycle_to_fill
      |                                  (back to InFill)
      +----------------------------------+
```

## States

| State      | Meaning                                                  |
|------------|----------------------------------------------------------|
| Free       | Available for allocation. Present in the free list.      |
| InFill     | Queued for submission to the fill ring.                   |
| Kernel     | Owned by the kernel (submitted via fill, awaiting RX).   |
| Rx         | Received by kernel, available for user consumption.      |
| User       | Owned by userspace for packet processing.                |
| Tx         | Submitted for transmission via the TX ring.              |
| Completion | Transmission completed by kernel, awaiting release.      |

## Legal Transitions

| From       | To         | Method              | Notes                          |
|------------|------------|---------------------|--------------------------------|
| Free       | InFill     | allocate_for_fill   | Pops from free list            |
| InFill     | Kernel     | mark_kernel_owned   |                                |
| Kernel     | Rx         | mark_rx             |                                |
| Rx         | User       | acquire_user        |                                |
| User       | InFill     | recycle_to_fill     | RX recycle without TX          |
| User       | Tx         | submit_tx           | Transmit path                  |
| Tx         | Completion | complete_tx         |                                |
| Completion | Free       | release_completion  | Pushes to free list            |

All other transitions are rejected with `InvalidFrameTransition`.

## Invalid Transition Behavior

- Returns a typed error with the frame index, current state, and attempted state.
- Does NOT mutate the frame's state.
- Does NOT alter the free list.
- Does NOT corrupt ownership tracking.

## Free-List Invariants

- Every index in the free list is in bounds.
- Every index in the free list has state `Free`.
- No index appears twice in the free list.
- A non-Free frame is never in the free list.
- Frames enter the free list only via `release_completion` (Completion -> Free).
- `allocate_for_fill` removes from the free list before transitioning to InFill.
- `recycle_to_fill` (User -> InFill) does NOT add to the free list.

## Why TX Requires Completion Before Free

A transmitted frame cannot be reused until the kernel confirms completion.
Allowing Tx -> Free would risk the kernel still DMA-reading a frame that
userspace has already repurposed. The Completion state provides a safe
handoff point.

## Why User -> InFill Exists

When userspace receives a packet but does not need to transmit a response,
the frame can be recycled directly back to the fill ring without going
through the TX/Completion path. This avoids unnecessary state transitions
and keeps the fill ring populated.

## Error Types

| Error                    | When                                           |
|--------------------------|-------------------------------------------------|
| InvalidFrameIndex        | Frame index out of bounds                       |
| InvalidFrameTransition   | Illegal state transition attempted               |
| FramePoolExhausted       | No free frames available for allocation          |
| FrameOwnershipCorrupt    | Free-list invariant violation detected            |

## Performance

- O(1) state lookup by frame index (Vec-backed).
- O(1) free-frame allocation (VecDeque pop).
- O(1) lifecycle transitions (no heap allocation).
- O(n) invariant check (`assert_no_duplicate_ownership`) for debug/assertion use.
- No atomics, locks, or dynamic dispatch.
- Per-queue friendly: MR12 can create one FramePool per queue.

## What MR10 Does NOT Implement

- No queue loop (MR11).
- No ring polling (MR11).
- No packet RX/TX processing (MR11+).
- No real AF_XDP runtime.
- No UMEM kernel registration.
- No XSK bind.
- No concurrency primitives (single-threaded per-queue design).

## How MR11 Will Use This

MR11 (Queue Lifecycle) will call FramePool methods from the queue loop:
1. `allocate_for_fill` to prepare frames for the fill ring.
2. `mark_kernel_owned` after submitting to fill ring.
3. `mark_rx` when kernel produces RX descriptors.
4. `acquire_user` to process received packets.
5. `recycle_to_fill` or `submit_tx` depending on packet handling.
6. `complete_tx` when kernel completes TX.
7. `release_completion` to return frames to the free pool.

## Formal Specification — MR10.1

### 1. Purpose

The TLA+ model in [`formal/tla/FrameOwnership.tla`](../formal/tla/FrameOwnership.tla)
specifies the abstract ownership lifecycle of UMEM frames. It allows the MR10
ownership invariants to be **formally specified and model-checked** with TLC
over a finite frame set.

This is a model of the *abstract state machine*. It does not verify the Rust
implementation line-by-line, and it does not claim that ZeroGate is bug-free or
fully formally verified. The claim is precisely:

> The MR10 frame ownership lifecycle invariants are formally specified and
> model-checkable with TLA+.

### 2. Rust-to-TLA+ Mapping

| Rust | TLA+ |
|------|------|
| `FrameState::Free` | `"Free"` |
| `FrameState::InFill` | `"InFill"` |
| `FrameState::Kernel` | `"Kernel"` |
| `FrameState::Rx` | `"Rx"` |
| `FrameState::User` | `"User"` |
| `FrameState::Tx` | `"Tx"` |
| `FrameState::Completion` | `"Completion"` |
| `FramePool.states` (Vec) | `state` function (`Frames -> FrameStates`) |
| `FramePool.free_list` (VecDeque) | `free` set |
| `FrameState::can_transition_to` | `CanTransition(from, to)` |
| `allocate_for_fill()` | `AllocateForFill` |
| `mark_kernel_owned()` | `MarkKernelOwned` |
| `mark_rx()` | `MarkRx` |
| `acquire_user()` | `AcquireUser` |
| `recycle_to_fill()` | `RecycleToFill` |
| `submit_tx()` | `SubmitTx` |
| `complete_tx()` | `CompleteTx` |
| `release_completion()` | `ReleaseCompletion` |

**Free-list abstraction:** The Rust implementation backs the free list with a
`VecDeque` (stack/queue). The TLA+ model abstracts it as a **set** of free
frames. Allocation order is intentionally abstracted away — set-based reasoning
is sufficient for ownership correctness (no duplicates, no non-Free frame in the
free list, free set matches Free state). The Rust uniqueness property is checked
at runtime by `assert_no_duplicate_ownership`.

### 3. Checked Invariants

- `TypeOK` — variables stay within their domains.
- `FrameInExactlyOneState` — every frame has exactly one state.
- `FreeListOnlyFree` — every frame in `free` has state `Free`.
- `FreeListMatchesState` — `free` is exactly the set of `Free` frames.
- `NonFreeNotInFree` — a non-Free frame is never in `free`.
- `TxNotFreeBeforeCompletion` — a `Tx` frame is never in `free`.
- `UserNotFree`, `RxNotFree`, `InFillNotFree`, `KernelNotFree`,
  `CompletionNotFree` — per-state exclusion from the free set.
- `OwnershipConsistent` — conjunction of all the above.

### 4. What This Model Checks

- Only legal lifecycle transitions are reachable (illegal transitions are not
  encoded in `Next`, so they cannot occur).
- Free-list correctness: `free` always equals the set of `Free` frames.
- Ownership consistency: no frame is `Free` while in
  `Tx`/`User`/`Rx`/`Kernel`/`InFill`/`Completion`.
- `Tx` frames cannot be returned directly to `Free` — they must pass through
  `Completion` first.
- No duplicate ownership and no allocate-twice / release-twice within the model.

### 5. What This Model Does NOT Check

- Does NOT verify Linux kernel correctness.
- Does NOT verify the eBPF verifier's correctness.
- Does NOT verify AF_XDP runtime correctness.
- Does NOT verify NIC/DMA/hardware correctness.
- Does NOT verify Rust compiler correctness.
- Does NOT verify the actual Rust implementation code line-by-line.
- Does NOT replace runtime tests or the unsafe audit.

### 6. How to Run Model Checking

TLA+ tooling (`tla2tools.jar`) is not bundled in the repo. Download it from
<https://github.com/tlaplus/tlaplus/releases> and run with a Java 11+ runtime:

```bash
java -cp tla2tools.jar tlc2.TLC \
  -config formal/tla/FrameOwnership.cfg \
  formal/tla/FrameOwnership.tla
```

The configuration uses `N = 3` frames by default. For deeper exploration, edit
the `CONSTANTS N` value in `FrameOwnership.cfg` to `N = 4`.

### 7. Model Checking Result

Last run with TLC 2.19 (tla2tools 1.7.4), Java 21:

| N | States generated | Distinct states | Depth | Result |
|---|------------------|-----------------|-------|--------|
| 3 | 1177 | 343 | 19 | PASS — no invariant violations |
| 4 | 10977 | 2401 | 25 | PASS — no invariant violations |
