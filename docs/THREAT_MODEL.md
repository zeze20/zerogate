# Threat Model (Pre-MR11 Design Doc)

> **Status: design doc, evolving (MR10.3).** A pre-MR11 threat model for the
> ZeroGate dataplane. It will be expanded in MR17 (security hardening / TCB).
> This MR does not change runtime behavior.

## Assets

- **UMEM memory safety** (frames, rings, free list).
- **Frame ownership integrity** (exactly-one-owner accounting).
- **Packet isolation** (one packet's processing cannot corrupt another's frame).
- **Queue isolation** (per-queue `FramePool`; no cross-queue interference).
- **Ring consistency** (descriptors and indices stay within contract).
- **Policy correctness** (PASS/DROP/REDIRECT enforced as configured).
- **Availability** of the dataplane under hostile traffic.

## Trust boundaries

- **Untrusted network packets** — contents, sizes, malformed/adversarial frames.
- **Kernel/userspace boundary** — RX/completion descriptors crossing into the
  agent are validated input, not trusted facts.
- **AF_XDP shared ring memory** — memory shared with the kernel; treated as
  untrusted for read-back.
- **eBPF verifier boundary** — the parser runs under the verifier's bounds model.
- **BPF maps** — policy/session/xsk map contents.
- **Policy / config input** — operator-supplied configuration.
- **Fake rings vs real rings** — fake rings are test-only; real rings (MR12c)
  cross the kernel boundary and carry the real trust assumptions.
- **Assumed-correct (TCB, not proven by ZeroGate):** Rust/LLVM/Cargo,
  bpf-linker/clang, Linux kernel + eBPF verifier + AF_XDP, NIC driver/hardware/
  DMA, container runtime, Java/TLC tooling. See `FORMAL_ASSURANCE.md`.

## Attacker capabilities (in scope)

- Malformed packets (truncated/oversized/adversarial headers).
- Packet flood / high packet rate.
- Resource-exhaustion pressure (drive the pool/rings toward empty/full).
- Parser edge cases (boundary lengths, unusual encapsulation).
- Queue starvation pressure (force backpressure/drop paths).

## Threats and mitigations

| Threat | Mitigation |
|--------|------------|
| Malformed packet → OOB read in parser | bounds-check-before-read in `zerogate-ebpf/parser.rs` |
| Hostile descriptor → OOB UMEM access | descriptor validation contract (addr/len/frame-boundary), see `RING_FRAME_CONTRACT.md` |
| Frame ownership corruption → UAF/double-free | MR10 ownership state machine; fail-fast on impossible states (`ERROR_POLICY.md`) |
| Free-list accounting drift | O(1) capacity guard (release) + O(n) duplicate scans (debug/test), see `INVARIANT_POLICY.md` |
| Silent fake success masking failure | explicit `NotImplemented`; no fake fallback; no fake-green CI |
| Resource exhaustion (DoS) | `Result`-based drop/backpressure for `RingFull`/`FramePoolExhausted`, no corruption |
| Unsafe-code creep | CI unsafe audit confines `unsafe` to allow-listed files (`UNSAFE_CONTRACTS.md`) |

## Required behavior (security invariants)

- Packet errors **must not corrupt ownership** — a bad packet/descriptor is
  dropped, never allowed to mutate frame accounting.
- Ownership corruption **must fail-fast** (panic), never continue on a corrupt
  state.
- Resource exhaustion **must be `Result`/drop/metric**, never a panic or
  corruption.
- **No silent fallback** (including zero-copy → copy-mode).
- **No fake success** — unimplemented paths return `NotImplemented`.

## Non-goals (current)

Out-of-scope attackers (explicitly **not** defended against):

- A **malicious kernel**.
- **Compromised NIC firmware**.
- A **hostile root user**.
- **Physical / DMA attacks**.
- A **malicious hypervisor**.

Also out of scope: proving kernel/hardware/compiler correctness; side-channel
resistance; production key management (deferred to MR18 KMS); multi-tenant
isolation guarantees beyond the per-queue design (MR13).

## Future work

MR17 will formalize the trusted computing base
(`TRUSTED_COMPUTING_BASE.md`), assumptions (`ASSUMPTIONS.md`), and proof
boundaries (`PROOF_BOUNDARIES.md`), and conduct a hardening audit.
