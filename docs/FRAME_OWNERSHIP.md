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
