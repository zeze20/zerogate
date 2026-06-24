# AF_XDP UMEM

## What Is UMEM?

In the AF_XDP architecture, UMEM (User Memory) is a contiguous region of memory shared between the kernel and userspace for zero-copy packet processing. The kernel writes received packets directly into UMEM frames, and userspace reads them without copying.

UMEM is divided into fixed-size **frames**. Each frame holds exactly one packet buffer. Frame sizes are typically 2048 or 4096 bytes, matching common MTU sizes and kernel page granularity.

## Frame Identity

Frames are identified by two representations:

- **FrameIndex** (`u32`): ordinal position of the frame within the UMEM region (0-based).
- **UmemAddr** (`u64`): byte offset from the start of the UMEM region.

The relationship is:

```
offset = frame_index * frame_size
```

This identity mapping is invertible: given a frame-aligned offset, the frame index can be recovered by integer division.

## Why Offset Alignment Matters

AF_XDP hardware and kernel subsystems assume frame-aligned addresses. An unaligned offset would point into the middle of a frame, corrupting packet boundaries. All offset values must satisfy:

```
offset % frame_size == 0
```

## Why Offset Bounds Matter

An offset outside `[0, total_size)` would access memory beyond the allocated UMEM region, causing undefined behavior at the kernel level. All offsets must satisfy:

```
offset < frame_count * frame_size
```

## Why Raw Pointers Are Not Exposed

The UMEM public API exposes only `FrameIndex` and `UmemAddr` — typed wrappers for indices and offsets. Raw pointers are never part of the public interface because:

- They bypass frame boundary validation.
- They bypass ownership tracking (future frame pool integration).
- They enable use-after-free patterns in multi-consumer scenarios.
- The UMEM boundary must remain auditable without tracing pointer provenance.

## What MR8 Implements

- `UmemConfig` — validated configuration (frame_count, frame_size, headroom).
- `UmemRegion` — allocation boundary backed by safe `Vec<u8>`.
- Frame index validation against `frame_count`.
- UMEM offset validation against `total_size` and frame alignment.
- `FrameIndex` <-> `UmemAddr` conversion with checked arithmetic.
- Structured error types for all invalid inputs.
- Comprehensive host-safe unit tests (no root, no NIC, no kernel).

## What MR8 Does NOT Implement

- Kernel UMEM registration (`setsockopt(XDP_UMEM_REG)`).
- XSK socket creation or binding.
- AF_XDP `bind()`.
- RX ring, TX ring, FILL ring, or COMPLETION ring.
- Queue worker loop or packet processing.
- Frame ownership state machine (free, in-fill, kernel-owned, user-owned).
- Real NIC interaction.
- Performance benchmarks.

## Future MRs

| MR | Scope |
|----|-------|
| MR9+ | XSK socket creation |
| Future | Ring mmap (RX/TX/FILL/COMPLETION) |
| Future | Frame ownership integration (frame_pool.rs) |
| Future | Queue dataplane loop |
| Future | Kernel UMEM registration |
| Future | AF_XDP performance validation on bare metal |

## Configuration Invariants

| Parameter | Constraint |
|-----------|-----------|
| `frame_count` | > 0, power of two |
| `frame_size` | 2048 or 4096 |
| `headroom` | < frame_size |
| `total_size` | frame_count * frame_size, must not overflow `usize` |
