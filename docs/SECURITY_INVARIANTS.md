# ZeroGate Security Invariants

This document lists the security invariants that ZeroGate enforces at every layer.

## 1. No Double Ownership

**Invariant**: A UMEM frame exists in exactly one state at any time.

**Enforcement**:
- `frame_pool.rs` tracks per-frame state; every transition is validated.
- Illegal transitions return `FramePoolError` in release mode.
- Debug assertions fire on invalid transitions in development.
- The Verus model (`frame_state.rs`) proves single-ownership by construction.

## 2. No Unbounded Pointer Arithmetic

**Invariant**: All packet pointer arithmetic is bounded by explicit checks.

**Enforcement**:
- Every packet read in `parser.rs` follows the pattern:
  ```
  if data + offset + size_of::<T>() > data_end { return None; }
  ```
- The offset accumulator is monotonically increasing.
- The Verus model (`parser_model.rs`) proves `read_valid(packet_len, offset, size)` holds before every access.

## 3. No Packet Dereference Before Bounds Check

**Invariant**: No raw pointer dereference occurs before verifying the access is within `[data, data_end)`.

**Enforcement**:
- `parser::read_at<T>()` is the single entry point for all packet reads.
- The bounds check is the first operation in the function.
- The eBPF verifier independently verifies this at program load time.

## 4. No Loose ABI Pointers

**Invariant**: No ABI-shared struct contains raw pointers, references, `usize`, `Vec`, `String`, `Box`, trait objects, or dynamically sized types.

**Enforcement**:
- All shared types are `#[repr(C)]` or `#[repr(C, packed)]`.
- Only fixed-width integers: `u8`, `u16`, `u32`, `u64`.
- Explicit padding fields for alignment.
- Compile-time `assert!` on every type's `size_of`.
- The Verus model (`abi_model.rs`) independently checks sizes and alignment.

## 5. No Hidden Global State in Pure Decision Logic

**Invariant**: Packet classification decisions are pure functions of their inputs.

**Enforcement**:
- `policy::evaluate_packet()` takes a `&[u8]` and returns `PacketAction` with no side effects.
- The XDP program's `process_packet()` is `#[inline(always)]` and reads only from packet buffer and BPF maps.
- The Verus model (`parser_model.rs`) proves decision determinism: same inputs → same output.

## 6. No TX Frame Recycling Before Completion

**Invariant**: A frame submitted for TX must not be recycled until the kernel confirms transmission via the COMPLETION ring.

**Enforcement**:
- The state machine requires `User → Tx → Completion → Free` before a frame can re-enter the pool.
- `User → InFill` (direct recycle) is only allowed for frames NOT submitted for TX.
- A frame in `Tx` state cannot transition to `Free` or `InFill` — only to `Completion`.
- The Verus ring model (`ring_model.rs`) proves a frame index appears in at most one ring at a time.

## 7. Ring Capacity Bounds

**Invariant**: No ring's occupancy exceeds its configured capacity.

**Enforcement**:
- `RingModel::enqueue()` returns `Err(Full)` when capacity is reached.
- Ring sizes are power-of-two and configured at initialization.
- The Verus ring model proves `within_capacity()` holds after every operation.

## 8. No Duplicate Frame in Any Ring

**Invariant**: A frame index appears at most once in any single ring, and in at most one ring across all rings.

**Enforcement**:
- `RingModel` tracks a `HashSet` of active frame indices; duplicate insertion returns `Err(Duplicate)`.
- `frame_in_at_most_one_ring()` verifies cross-ring uniqueness.
- The Verus ring model proves this invariant through lifecycle simulation tests.

## 9. Unsafe Confinement

**Invariant**: `unsafe` code exists only in designated modules.

**Enforcement**:
- `scripts/audit_no_unsafe.sh` scans all `.rs` files and fails CI if `unsafe` appears outside:
  - `zerogate-ebpf/src/parser.rs`
  - `zerogate-agent/src/umem.rs`
  - `zerogate-agent/src/sys.rs`
- Every `unsafe` block has a SAFETY comment documenting provenance, bounds, alignment, lifetime, and aliasing.

## 10. No Key Material in eBPF

**Invariant**: Cryptographic keys never cross into the eBPF program or BPF maps.

**Enforcement**:
- `zerogate-kms` signs policies; `zerogate-agent` loads pre-validated compact entries.
- BPF maps contain only `SessionKey` (u64) and `PolicyAction` (u8) — no key material.
- The data-plane hot path uses integer comparisons only.
