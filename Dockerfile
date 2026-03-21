# Lattice dev container
# Rust 1.84 on Debian Bookworm + Node 20 + Tauri Linux dependencies
FROM rust:1.84-bookworm

# Install Node 20 via NodeSource
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs

# Tauri Linux dependencies (WebKit2GTK + build tools)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libwebkit2gtk-4.1-dev \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    patchelf \
    libxdo-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    curl \
    wget \
    file \
    pkg-config \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Tauri CLI
RUN cargo install tauri-cli --version "^2"

# Install cargo tools
RUN rustup component add rustfmt clippy

WORKDIR /workspace

# Pre-cache dependencies by copying manifests first
COPY Cargo.toml Cargo.lock* ./
COPY crates/ crates/
RUN cargo fetch

CMD ["bash"]
