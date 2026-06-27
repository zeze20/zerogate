# Threat Model (Pre-MR11 Design Doc)

> **Status: design doc, evolving.** A first-pass threat model for the ZeroGate
> dataplane. It will be expanded in MR17 (security hardening / TCB). MR10.2 does
> not change runtime behavior.

## Assets

- Memory safety of the userspace agent (UMEM frames, rings, free list).
- Integrity of frame ownership accounting.
- Correct policy enforcement (PASS/DROP/REDIRECT).
- Availability of the dataplane under hostile traffic.

## Trust boundaries

- **Untrusted:** the network (packet contents, sizes, malformed frames) and any
  descriptor whose origin is the kernel/NIC RX path — treated as untrusted input
  to be validated.
- **Assumed-correct (TCB, not proven by ZeroGate):** Rust/LLVM/Cargo,
  bpf-linker/clang, Linux kernel + eBPF verifier + AF_XDP, NIC driver/hardware/
  DMA, container runtime, Java/TLC tooling. See `FORMAL_ASSURANCE.md`.

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

## Non-goals (current)

- Proving kernel/hardware/compiler correctness.
- Side-channel resistance.
- Production key management (deferred to MR18 KMS).
- Multi-tenant isolation guarantees beyond per-queue design (MR13).

## Future work

MR17 will formalize the trusted computing base
(`TRUSTED_COMPUTING_BASE.md`), assumptions (`ASSUMPTIONS.md`), and proof
boundaries (`PROOF_BOUNDARIES.md`), and conduct a hardening audit.
