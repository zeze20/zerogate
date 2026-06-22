# Continuous Integration (CI)

ZeroGate uses GitLab CI as the primary continuous integration system.

The pipeline is split into focused stages so failures are easy to locate and security checks are enforced independently.

---

## Pipeline Stages

- format
- lint
- test
- security
- ebpf-build
- docs

---

## Jobs

### cargo_fmt

Runs:

```bash
cargo fmt --all -- --check
```

Purpose:

Ensures consistent Rust formatting across the workspace.

---

### cargo_clippy

Runs:

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Purpose:

Enforces strict lint rules. All warnings are treated as errors.

---

### cargo_check

Runs:

```bash
cargo check --workspace
```

Purpose:

Ensures the entire workspace compiles successfully.

---

### cargo_test

Runs:

```bash
cargo test --workspace
```

Purpose:

Runs all unit and integration tests.

---

### unsafe_audit

Runs:

```bash
./scripts/audit_no_unsafe.sh
```

Purpose:

Enforces unsafe confinement rules.

Unsafe is ONLY allowed in:

- zerogate-ebpf/src/parser.rs
- zerogate-agent/src/umem.rs
- zerogate-agent/src/sys.rs

Any unsafe usage outside these files will fail CI.

---

### dependency_audit

This is currently a placeholder.

Future MR will introduce:

- cargo-audit or cargo-deny
- vulnerability scanning

Currently marked as `allow_failure`.

---

### ebpf_build

Runs:

```bash
./scripts/build_ebpf.sh
```

Purpose:

Builds the eBPF program for `bpfel-unknown-none` using Rust nightly and `-Z build-std=core`.

Currently marked as `allow_failure` until GitLab runners have nightly, rust-src, and bpfel support fully prepared.

Artifacts:

- `target/bpfel-unknown-none/` (expires in 1 week)

---

### ebpf_verifier_load

Runs:

```bash
./scripts/verify_ebpf_load.sh
```

Purpose:

Attempts to load the eBPF object using `bpftool` to invoke the Linux BPF verifier.

This job is **manual** because it requires:

- Linux environment
- Root or CAP_BPF/CAP_NET_ADMIN capabilities
- `bpftool` installed
- Mounted bpffs

It is not triggered automatically on shared non-privileged runners.

Artifacts:

- `artifacts/verifier.log` (expires in 1 week)

---

### docs_smoke

Checks that required docs exist:

```bash
README.md
docs/ARCHITECTURE.md
docs/SECURITY_INVARIANTS.md
docs/CI.md
```

---

## Unsafe Policy

ZeroGate strictly limits unsafe usage.

Allowed files:

- parser.rs (eBPF boundary)
- umem.rs (AF_XDP memory boundary)
- sys.rs (syscall/FFI boundary)

All other files must remain safe.

---

## Negative Test (Unsafe Enforcement)

To verify the audit script works:

Temporarily add the following to a non-allowed file:

```rust
unsafe { core::hint::unreachable_unchecked() }
```

Run:

```bash
./scripts/audit_no_unsafe.sh
```

Expected result:

- Script fails
- Reports file and line number

After test:

Remove the injected unsafe block.

---

## Current Limitations

- Dependency auditing is not yet enforced
- eBPF build is not yet implemented
- Verifier/load test is not yet included
- AF_XDP runtime is still a scaffold

These will be implemented in future merge requests.

---

## Summary

This CI pipeline ensures:

- consistent formatting
- strict linting
- test correctness
- enforced unsafe boundaries
- presence of critical documentation

It forms the foundation for secure and verifiable development of ZeroGate.
