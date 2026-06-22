# eBPF Verifier And Build Infrastructure

## Why Host Tests Are Not Sufficient

Running `cargo test --workspace` on the host proves parser logic correctness in userspace, but it does NOT prove that:

- The eBPF program builds for `bpfel-unknown-none`.
- The resulting ELF object is acceptable to the Linux BPF verifier.
- Packet pointer arithmetic satisfies verifier constraints.
- Stack usage, map access, and helper calls are within BPF limits.
- The program can be loaded by `bpftool` or an Aya-based loader.

## eBPF Build

The eBPF program must be built for the `bpfel-unknown-none` target (little-endian BPF).

### Build command

```bash
./scripts/build_ebpf.sh
```

This script:

- Checks for `rustup` and nightly toolchain.
- Installs `rust-src` component for `-Z build-std=core`.
- Builds `zerogate-ebpf` for `bpfel-unknown-none`.
- Produces artifacts under `target/bpfel-unknown-none/`.

### Requirements

- Rust nightly toolchain (`rustup toolchain install nightly`)
- `rust-src` component (installed automatically by the script)

## Verifier Smoke Test

The verifier smoke test attempts to load the eBPF object using `bpftool`.

### Run command

```bash
sudo ./scripts/verify_ebpf_load.sh
```

This script:

- Verifies Linux environment.
- Checks for `bpftool`.
- Checks for root or required capabilities.
- Runs the eBPF build.
- Searches for the eBPF artifact.
- Attempts `bpftool prog load` to invoke the kernel BPF verifier.
- Writes verifier output to `artifacts/verifier.log`.
- Unloads the program after successful verification.

### Requirements

- Linux (the BPF verifier is a kernel subsystem)
- Root access or capabilities: `CAP_BPF`, `CAP_NET_ADMIN`
- `bpftool` (install via `apt install linux-tools-common linux-tools-generic` or equivalent)
- Mounted bpffs at `/sys/fs/bpf/`

### What the verifier checks

The Linux BPF verifier validates:

- Packet pointer bounds are proven before dereference.
- Map lookups are null-checked before use.
- Stack usage does not exceed 512 bytes.
- No unbounded loops (unless BPF bounded loop support is available).
- No illegal helper function calls.
- All code paths terminate.
- No out-of-bounds memory access.

### What the verifier does NOT prove

- Correctness of packet decision logic.
- AF_XDP runtime behavior (UMEM, rings, frame lifecycle).
- Performance under real NIC traffic.
- Interaction with userspace agent.

## Troubleshooting

### nightly toolchain missing

```
ERROR: Rust nightly toolchain is required.
Install with: rustup toolchain install nightly
```

### bpf-linker missing (if required)

```
ERROR: bpf-linker is required.
Install with: cargo install bpf-linker
```

Note: `bpf-linker` may not be required if using `-Z build-std=core` with the default linker. If the build fails with linker errors, install it.

### bpftool missing

```
ERROR: bpftool is required.
Install with your distro package manager, e.g. apt install linux-tools-common linux-tools-generic
```

### Permission denied

```
ERROR: verifier/load test should be run as root or with required BPF capabilities.
```

Run with `sudo` or grant `CAP_BPF` and `CAP_NET_ADMIN` to the process.

### No eBPF object found

```
ERROR: no eBPF object found under target/bpfel-unknown-none
```

The build may have failed silently or the artifact path may differ. Check the build output and verify `target/bpfel-unknown-none/debug/zerogate-ebpf` exists.

### Verifier rejected program

```
ERROR: BPF verifier rejected the program.
```

Check `artifacts/verifier.log` for the detailed verifier output. Common causes:

- Missing bounds check before packet pointer dereference.
- Invalid BPF program section name (bpftool expects sections like `xdp`, `xdp/prog`).
- Stack overflow (> 512 bytes).
- Unresolved map references.

Note: The current build produces a Rust ELF binary. If `bpftool` rejects it due to missing section annotations, a future MR will introduce proper BPF section naming via Aya or explicit `#[link_section]` attributes.

## Current Limitations

- The verifier/load test requires a privileged Linux runner.
- AF_XDP runtime is not tested by the verifier smoke test.
- The current ELF artifact may not have correct BPF section naming for `bpftool`.
- An Aya-based loader or explicit section annotations may be needed in a future MR.
