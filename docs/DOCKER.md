# Docker Development Environment

## Purpose

Docker provides a reproducible Linux development environment for ZeroGate.

It standardizes the full toolchain: Rust stable + nightly, `rust-src`, clang, LLVM, `libelf`, `bpf-linker`, and `bpftool`. This helps avoid Windows/WSL dependency drift and supports:

- MR5 eBPF build and verifier scripts
- MR8+ Linux-only development work
- Consistent CI-equivalent local checks

## Services

### dev

Normal Rust development environment.

- **Non-privileged** — no special Linux capabilities
- Used for: `cargo fmt`, `cargo check`, `cargo clippy`, `cargo test`, `./scripts/audit_no_unsafe.sh`
- Interactive shell for exploratory development

### ebpf

eBPF artifact build environment.

- **Non-privileged** — does not load programs into the kernel
- Runs `./scripts/build_ebpf.sh`
- Builds the `bpfel-unknown-none` artifact using Rust nightly and `-Z build-std=core`
- Does not attach XDP to any interface

### verifier

Privileged BPF verifier/load smoke test environment.

- **Privileged** — requires elevated kernel access
- Runs `./scripts/verify_ebpf_load.sh`
- Attempts `bpftool` verifier/load against the built eBPF object
- Mounts `/sys/fs/bpf` from the host
- **Not for ordinary development** — use only for intentional verifier testing

## Commands

### Helper Script

```bash
./scripts/docker_dev.sh build      # Build the dev Docker image
./scripts/docker_dev.sh shell      # Open interactive dev shell
./scripts/docker_dev.sh check      # Run fmt + check + clippy + test + unsafe audit
./scripts/docker_dev.sh ebpf       # Build eBPF artifact
./scripts/docker_dev.sh verifier   # Run privileged verifier/load smoke test
./scripts/docker_dev.sh clean      # Stop and remove containers
```

### Raw Docker Compose Commands

```bash
docker compose build dev
docker compose run --rm dev
docker compose run --rm dev bash -lc 'cargo test --workspace'
docker compose run --rm ebpf
docker compose run --rm verifier
docker compose down
```

## Security Notes

- `dev` is intentionally **not privileged**. It cannot load BPF programs or attach XDP.
- `ebpf` is intentionally **not privileged**. It only compiles the eBPF artifact.
- `verifier` is **privileged** because `bpftool prog load` requires kernel BPF access (`CAP_BPF`, `CAP_NET_ADMIN`).
- Privileged containers must be used intentionally and not for ordinary development.
- Docker does not replace production hardening or bare-metal security validation.

## Limitations

- Docker is **not** a replacement for bare-metal AF_XDP performance tests. Container networking adds overhead that does not reflect real NIC line-rate performance.
- Docker does not prove line-rate packet processing capability.
- Docker may not expose a real NIC suitable for AF_XDP benchmarking.
- Verifier success depends on host kernel capabilities (kernel version, BPF subsystem config).
- Windows/WSL Docker may still differ from a dedicated Linux runner in certain kernel behaviors.

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Docker not installed | Install Docker Desktop or Docker Engine for your platform |
| `docker compose` unavailable | Ensure Docker Compose v2 is installed (`docker compose version`) |
| `bpftool` missing in container | Rebuild the image: `docker compose build dev` |
| `bpf-linker` build failure | Check LLVM/clang versions; `bpf-linker` requires LLVM dev libraries |
| Permission denied for verifier | The verifier service requires `privileged: true` and host kernel access |
| `/sys/fs/bpf` missing | Mount bpffs on the host: `sudo mount -t bpf bpf /sys/fs/bpf` |
| eBPF build fails due to toolchain | Ensure nightly + rust-src are installed: `rustup component add rust-src --toolchain nightly` |
| `linux-tools-generic` unavailable | The Dockerfile uses `bpftool` directly; `linux-tools-generic` is omitted because it may not be available on all Ubuntu versions |

## Package Notes

`linux-tools-generic` is omitted from the Dockerfile because it depends on the specific kernel version of the host and may not be available or installable inside a container. The `bpftool` package is installed directly instead.
