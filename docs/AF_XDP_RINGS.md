# AF_XDP Rings

This document describes the AF_XDP ring architecture as modeled by ZeroGate.

---

## AF_XDP Socket Role

An AF_XDP socket (XSK) provides zero-copy packet I/O between the kernel and userspace. Each XSK is bound to a specific NIC queue and shares a UMEM region with the kernel. Communication between kernel and userspace occurs through four shared rings.

---

## UMEM Relationship

Rings exchange **descriptors** that reference frames in UMEM by byte offset (`UmemAddr`). Descriptors are not raw pointers — they are validated UMEM offsets. This ensures:

- No out-of-bounds memory access.
- No unaligned frame access.
- No pointer confusion between kernel and userspace address spaces.

---

## Descriptor Model

A ring descriptor (`RingDesc`) contains:

| Field     | Type      | Meaning                                   |
|-----------|-----------|-------------------------------------------|
| `addr`    | `UmemAddr`| Byte offset into UMEM (frame-aligned)     |
| `len`     | `u32`     | Packet/data length                        |
| `options` | `u32`     | Reserved for future use                   |

### Why addr is a UMEM offset

The descriptor `addr` is a byte offset into the shared UMEM region, not a raw pointer. This is intentional:

- Kernel and userspace may have different virtual address mappings.
- Offsets are position-independent and safe to validate.
- Raw pointers would be meaningless across address spaces.

### Why descriptor validation matters

Before any descriptor is used, it must be validated:

- `addr` must be within UMEM bounds (`< total_size`).
- `addr` must be frame-aligned (`addr % frame_size == 0`).
- `len` must be greater than 0 (strict policy).
- `len` must not exceed `frame_size`.

Invalid descriptors are rejected with structured errors.

---

## Ring Roles

### FILL Ring

**Direction:** Userspace -> Kernel

Userspace provides empty frame addresses to the kernel via the FILL ring. The kernel uses these frames to store incoming packets.

Future integration will transition frame ownership: `Free -> InFill -> Kernel`.

### RX Ring

**Direction:** Kernel -> Userspace

The kernel returns received packet descriptors through the RX ring. Each descriptor identifies the frame and the received packet length.

Future integration will transition frame ownership: `Kernel -> Rx -> User`.

### TX Ring

**Direction:** Userspace -> Kernel

Userspace submits frames for transmission via the TX ring. Each descriptor identifies the frame and the data length to transmit.

Future integration will transition frame ownership: `User -> Tx`.

### COMPLETION Ring

**Direction:** Kernel -> Userspace

The kernel returns transmitted frame descriptors through the COMPLETION ring, indicating transmission is complete and the frame can be reused.

Future integration will transition frame ownership: `Tx -> Completion -> Free`.

---

## What MR9 Implements

- XSK configuration validation (`XskConfig::validate()`).
- XSK handle lifecycle scaffold (`XskHandle` with `Created`/`Bound`/`Closed` states).
- Ring configuration validation (`RingConfig::validate()`).
- Ring descriptor type and validation against UMEM (`RingDesc::validate_for_umem()`).
- Safe ring traits (`FillRing`, `RxRing`, `TxRing`, `CompletionRing`).
- Fake/test ring implementations for deterministic host-side testing.
- Structured error types for XSK, ring, and descriptor failures.
- `bind()` returns `NotImplemented` — no fake kernel success.

---

## What MR9 Does NOT Implement

- Queue worker dataplane loop.
- Live RX polling or TX processing.
- Real XSK socket bind or XSK_MAP registration.
- Kernel ring mmap.
- Frame ownership state machine integration.
- Packet forwarding or routing.
- Live NIC traffic processing.
- Production AF_XDP performance validation.

---

## Future MRs

| MR    | Scope                                         |
|-------|-----------------------------------------------|
| MR10+ | Frame ownership state machine                 |
| MR10+ | Real XSK socket creation and bind             |
| MR10+ | Kernel ring mmap                              |
| MR10+ | Queue worker dataplane loop                   |
| MR10+ | FILL/RX/TX/COMPLETION integration             |
| MR10+ | Live NIC traffic and performance validation   |

---

## Configuration

### XskConfig

| Field            | Constraint                        |
|------------------|-----------------------------------|
| `interface_name` | Non-empty string                  |
| `queue_id`       | Valid NIC queue index              |
| `frame_count`    | > 0, power of two                 |
| `frame_size`     | 2048 or 4096                      |
| `force_copy`     | Boolean (copy vs zero-copy mode)  |

### RingConfig

| Field             | Constraint          |
|-------------------|---------------------|
| `fill_size`       | > 0, power of two   |
| `rx_size`         | > 0, power of two   |
| `tx_size`         | > 0, power of two   |
| `completion_size` | > 0, power of two   |

Default ring sizes are 2048 entries each (conservative).
