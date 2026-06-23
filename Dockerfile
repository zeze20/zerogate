FROM ubuntu:24.04

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
    llvm-dev \
    libclang-dev \
    libelf-dev \
    zlib1g-dev \
    make \
    cmake \
    iproute2 \
    iputils-ping \
    bpftool \
    file \
    sudo \
    bash \
    && rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable

RUN rustup toolchain install nightly \
    && rustup component add rustfmt clippy \
    && rustup component add rust-src --toolchain nightly

RUN cargo install bpf-linker

WORKDIR /workspace

CMD ["/bin/bash"]
