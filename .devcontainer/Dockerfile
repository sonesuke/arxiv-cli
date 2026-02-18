FROM rust:1-slim-bookworm

# Set UTF-8 locale
ENV LANG=C.UTF-8 \
    LANGUAGE=C.UTF-8 \
    LC_ALL=C.UTF-8

# Install Node.js and other dependencies as root
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
    curl \
    git \
    pkg-config \
    libssl-dev \
    ripgrep \
    tmux \
    vim \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/*

# Install Node.js LTS manually
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y nodejs \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/*

# Ensure pre-installed tools are always in PATH even if CARGO_HOME is overridden at runtime
ENV PATH=/usr/local/cargo/bin:$PATH

# Install cargo-binstall and other rust tools as root (installs to /usr/local/cargo/bin)
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && rustup component add rustfmt clippy llvm-tools-preview \
    && cargo binstall -y cargo-audit cargo-llvm-cov
