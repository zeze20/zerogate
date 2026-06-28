# Descriptor Validation Contract (MR10.3)

> **Status: MR10.3 contract, not yet implemented in the runtime.** This refines
> the descriptor rules in [`RING_FRAME_CONTRACT.md`](RING_FRAME_CONTRACT.md) into
> a per-case classification MR11 must satisfy. Classification follows
> [`ERROR_POLICY.md`](ERROR_POLICY.md): external/malformed input → `Result`/drop;
> impossible internal state → panic/fail-fast.

Validation applies at four points: **RX descriptor**, **TX submit**, **completion
frame id**, and **fill submit**.

## Acceptance conditions

Where applicable, an accepted descriptor/frame must satisfy **all** of:

- `addr` is inside UMEM
- `addr` maps to exactly **one known** frame
- frame-index conversion cannot overflow
- `len > 0`
- `len <= usable_frame_size`
- `addr + len` cannot overflow
- `addr + len <= frame_start + frame_size`
- headroom/offset behavior is explicit
- multi-buffer packets are **out of scope**
- jumbo / multi-frame packets are **out of scope**

## The deciding question: can `frame_id` be derived?

Invalid-descriptor behavior depends on whether a frame can be identified:

- **Case A — `frame_id` cannot be derived.** Do **not** recycle an unknown
  frame. Return a `Result` error, (future) metric/log, **no `FramePool`
  transition**. Recycling an unknown frame id would itself corrupt ownership.
- **Case B — `frame_id` derived but expected-state mismatch.** A tracked frame in
  the wrong state for the operation is a potential **internal invariant
  violation** → **panic/fail-fast** when the state is impossible under correct
  logic.
- **Case C — `frame_id` derived and expected state valid.** Drop/recycle via the
  documented safe transition path may proceed.

## Classification matrix

| # | Case | Panic or Result | State change? | Metric/log | Required tests |
|---|------|-----------------|---------------|------------|----------------|
| 1 | Invalid `addr` (outside UMEM) | `Result` (external) | none | `desc.invalid_addr` | addr below UMEM; addr ≥ total size; addr == total size |
| 2 | Invalid `len` (`0` or `> usable_frame_size`) | `Result` | none | `desc.invalid_len` | len==0; len==frame_size+1 |
| 3 | `addr + len` overflow | `Result` | none | `desc.overflow` | addr near `usize::MAX`; len forcing wrap |
| 4 | Crosses frame boundary (`addr+len > frame_start+frame_size`) | `Result` | none | `desc.crosses_boundary` | valid addr, len spilling into next frame |
| 5 | Unknown frame id (Case A) | `Result` | none | `desc.unknown_frame` | addr not mapping to any tracked frame |
| 6 | Duplicate RX descriptor for a frame already `Rx`/`User` | **Panic** if it implies impossible internal state; else `Result` drop | none | `rx.duplicate` | duplicate RX for `Rx` frame; for `User` frame |
| 7 | Expected-state mismatch on a tracked frame (Case B) | **Panic** | none (process aborts) | fatal log | transition attempted from wrong state |
| 8 | Duplicate completion for a frame already `Free`/`Completion` | **Panic** (implies double-free) | none (aborts) | fatal log | completion replayed for freed frame |
| 9 | Completion without a prior `TxSubmitted` | **Panic** (impossible) | none (aborts) | fatal log | completion for `Free`/`User` frame |

Notes:

- Rows 1–5 are **external/malformed input** at the kernel/network boundary →
  `Result`/drop/metric, never panic. They must **not** mutate `FramePool` state.
- Rows 7–9 are **impossible internal states** under correct logic → fail-fast.
  Reaching them means ZeroGate's own accounting is corrupt, which is exactly the
  class of bug that must not be papered over (see [`ERROR_POLICY.md`](ERROR_POLICY.md)).
- Row 6 is the **nuanced** one: a duplicate RX descriptor can be a hostile/buggy
  kernel-side event (treat as drop) **or** evidence the frame was double-tracked
  (impossible → panic). MR11 must decide per derivable state and document which
  branch it took; the safe default is to fail-fast only when the duplicate
  implies two owners for one frame.

## Relationship to existing validation

MR9 already implements descriptor *bounds* validation
(`RingDesc::validate_for_umem` → `UmemRegion::validate_offset`): it rejects OOB,
`addr == total_size`, unaligned, `len == 0`, and `len > frame_size`. Rows 1–4
build on that existing single source of truth. MR10.3 adds the **frame-id
derivation** and **expected-state** dimensions (rows 5–9), which only become
live once the queue lifecycle (MR11) tracks per-frame ownership across rings.
