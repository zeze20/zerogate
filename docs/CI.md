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

### ebpf_build_optional_or_required

Runs eBPF build if script exists:

```bash
./scripts/build_ebpf.sh
```

If not:

- It reports that the build path is not implemented
- This is expected before Phase 5

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
