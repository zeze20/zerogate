################################################################################
# Stage: dev
# Base development image with stable Rust, clang/llvm, and workspace tools.
# Used for: cargo check, cargo test, cargo clippy, cargo fmt, unsafe audit.
################################################################################
FROM ubuntu:24.04 AS dev

ENV DEBIAN_FRONTEND=noninteractive
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH=/usr/local/cargo/bin:$PATH

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    build-essential \
    pkg-config \
    clang \
    llvm \
    libelf-dev \
    zlib1g-dev \
    make \
    bash \
    && rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable
RUN rustup component add rustfmt clippy

# Validate dev toolchain
RUN cargo --version && rustc --version && clang --version | head -1

WORKDIR /workspace
CMD ["/bin/bash"]

################################################################################
# Stage: ebpf
# Extends dev with nightly Rust, bpf-linker, bpftool, and LLVM dev libraries.
# Used for: eBPF cross-compilation and verifier smoke tests.
################################################################################
FROM dev AS ebpf

RUN apt-get update && apt-get install -y --no-install-recommends \
    llvm-dev \
    libclang-dev \
    cmake \
    bpftool \
    iproute2 \
    file \
    sudo \
    && rm -rf /var/lib/apt/lists/*

RUN rustup toolchain install nightly \
    && rustup component add rust-src --toolchain nightly

RUN cargo install bpf-linker

# Validate ebpf toolchain
RUN rustc +nightly --version \
    && command -v bpf-linker \
    && command -v bpftool \
    && bpftool version

WORKDIR /workspace
CMD ["/bin/bash"]
