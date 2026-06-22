# BPF Map Access And Synchronization

## Overview

MR7 adds userspace BPF map access and policy/session synchronization scaffolding to `zerogate-agent`.

The agent manages three BPF maps that bridge userspace policy decisions and kernel-side eBPF/XDP program behavior:

| Map | Key | Value | Purpose |
|-----|-----|-------|---------|
| **POLICY** | `PacketMeta` | `PolicyAction` | Per-flow XDP action (PASS/DROP/REDIRECT) |
| **SESSIONS** | `SessionKey` | `SessionValue` | Admitted sessions mapped to XSK queue indices |
| **XSK_MAP** | `u32` (queue ID) | `i32` (socket FD) | AF_XDP socket registration for XDP_REDIRECT |

## Architecture

```
PolicySnapshot
    |
    v
BpfMapManager<B>
    |
    +-- set_policy(meta, action)     --> B::upsert_policy()
    +-- remove_policy(meta)          --> B::remove_policy()
    +-- admit_session(id, xsk_idx)   --> B::upsert_session()
    +-- revoke_session(id)           --> B::remove_session()
    |
    B = InMemoryMapBackend   (test/dev)
    B = AyaMapBackend        (future kernel backend)
```

### Trait-Based Design

Map access is abstracted behind three traits:

- `PolicyMapWriter` — upsert/remove policy entries
- `SessionMapWriter` — upsert/remove session entries
- `XskMapWriter` — upsert/remove XSK socket entries

This allows unit tests to use `InMemoryMapBackend` (HashMap-backed) while production will use a kernel/Aya backend that writes to real BPF maps.

### InMemoryMapBackend

The in-memory backend is **test/development only**. It does NOT write to real kernel BPF maps.

Behavior:
- Upsert inserts or replaces deterministically.
- Remove is idempotent — removing a missing key is a no-op.
- No panics, no unwrap, no unsafe.

### Kernel Backend

Real kernel map binding requires the Aya loader to expose loaded map handles. Until the loader is fully integrated, the kernel backend is not implemented. Methods return `NotImplemented` errors.

## ABI Types

All map keys and values use `#[repr(C, packed)]` types from `zerogate-common`:

| Type | Size | Fields |
|------|------|--------|
| `PacketMeta` | 16 | src_ip, dst_ip, src_port, dst_port, protocol, _reserved |
| `PolicyAction` | 1 | action (PASS=0, DROP=1, REDIRECT=2) |
| `SessionKey` | 8 | session_id |
| `SessionValue` | 4 | xsk_index |

ABI layout must remain stable for map key/value types. Layout tests enforce size and alignment invariants.

## PolicySnapshot

`PolicySnapshot` is a pure data type representing a point-in-time collection of policies and sessions. It can be applied atomically to a `BpfMapManager`:

```rust
let snap = PolicySnapshot::empty()
    .with_policy(meta, PolicyAction { action: PolicyAction::DROP })
    .with_session(SessionKey { session_id: 42 }, SessionValue { xsk_index: 1 });

apply_policy_snapshot(&mut manager, &snap)?;
```

## What Is NOT Implemented

- AF_XDP socket creation
- UMEM allocation/registration
- XSK socket binding or real XSK_MAP FD registration
- RX/TX/FILL/COMPLETION rings
- Queue worker loop
- Real kernel/Aya map backend
- Policy signing/verification (future KMS work)
- Async runtime or concurrency primitives

## Future Work

- Kernel/Aya map backend once the loader exposes map handles
- Batched map updates for atomic snapshot swaps
- Policy generation tracking
- Per-queue XSK registration via AF_XDP
- Hot-reload policy updates
