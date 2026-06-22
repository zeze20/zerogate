# eBPF Loader And XDP Attach Manager

## Overview

MR6 adds a userspace eBPF loader and XDP attach manager scaffold to `zerogate-agent`.

The loader manages the control-plane lifecycle of the eBPF/XDP program:

1. Validate configuration (object path, interface, program name, attach mode).
2. Load the compiled eBPF object into the kernel.
3. Attach the XDP program to a network interface.
4. Detach the XDP program on shutdown.

The loader is separate from the AF_XDP dataplane. It does not implement UMEM allocation, XSK socket creation, or ring lifecycle.

## Architecture

```
AgentConfig
    |
    v
EbpfConfig --> EbpfManager
                  |
                  +--> validate_config()
                  +--> load()          [Created -> Loaded]
                  +--> attach_xdp()    [Loaded -> Attached]
                  +--> detach_xdp()    [Attached -> Detached]
                  +--> open_policy_map()   [MR7]
                  +--> open_session_map()  [MR7]
                  +--> open_xsk_map()      [MR7]
```

### State Machine

```
Created --> Loaded --> Attached --> Detached
```

Invalid transitions return `InvalidEbpfState` errors:

- `attach_xdp()` before `load()` returns error.
- `detach_xdp()` before `attach_xdp()` returns error.
- `load()` in any state other than `Created` returns error.

## XDP Attach Modes

| Mode | Flag | Description |
|------|------|-------------|
| SKB/Generic | `skb` | Works on all interfaces, lowest performance |
| Driver/Native | `driver` | Requires driver support, better performance |
| Hardware/Offload | `hardware` | Requires NIC firmware support |

Default mode: `skb`

## Usage

```bash
zerogate-agent \
  --object /path/to/zerogate-ebpf.o \
  --iface eth0 \
  --program-name zerogate_xdp \
  --xdp-mode skb
```

### CLI Options

| Option | Required | Default | Description |
|--------|----------|---------|-------------|
| `--object` | Yes | — | Path to eBPF object file |
| `--iface` | Yes | — | Network interface name |
| `--program-name` | No | `zerogate_xdp` | XDP program name |
| `--xdp-mode` | No | `skb` | Attach mode: skb, driver, hardware |

## Requirements

Loading and attaching requires:

- Linux (eBPF is a Linux kernel subsystem)
- Root or `CAP_BPF` + `CAP_NET_ADMIN` capabilities
- A valid eBPF object file built for `bpfel-unknown-none`
- The target network interface must exist

Unit tests do NOT require root, Linux, a real NIC, or a real eBPF object.

## Map Access

BPF map access methods are placeholders returning `NotImplemented`:

- `open_policy_map()` — planned for MR7
- `open_session_map()` — planned for MR7
- `open_xsk_map()` — planned for MR7

## Failure Modes

| Error | Cause |
|-------|-------|
| `InvalidConfig` | Empty object path, interface name, or program name |
| `BpfLoadFailed` | Object file missing, invalid ELF, or kernel rejection |
| `XdpAttachFailed` | Interface not found, permission denied, or driver incompatibility |
| `XdpDetachFailed` | Detach syscall failure |
| `InterfaceResolveFailed` | Interface does not exist or name contains NUL |
| `UnsupportedPlatform` | Running on non-Linux OS |
| `NotImplemented` | Feature planned for future MR |

## Current Limitations

- Real Aya-based eBPF loading is scaffolded but not yet integrated (requires a valid eBPF object built on Linux).
- AF_XDP runtime (UMEM, XSK, rings) is not implemented.
- Policy/session/XSK map synchronization is deferred to MR7.
- The loader has been tested with host-safe unit tests only; real attach/detach testing requires a privileged Linux runner.
