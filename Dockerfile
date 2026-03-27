# Stage 1: Build the Rust daemon binary
FROM rust:1.88-bookworm AS rust-builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates/branchdeck-core/Cargo.toml crates/branchdeck-core/Cargo.toml
COPY crates/branchdeck-daemon/Cargo.toml crates/branchdeck-daemon/Cargo.toml

# Create stub src-tauri Cargo.toml to satisfy workspace member
# (we don't build the desktop crate in Docker)
RUN mkdir -p src-tauri/src && \
    echo '[package]\nname = "branchdeck-desktop"\nversion = "0.1.0"\nedition = "2021"\n\n[lib]\npath = "src/lib.rs"' > src-tauri/Cargo.toml && \
    echo '' > src-tauri/src/lib.rs

# Create stub source files for dependency caching
RUN mkdir -p crates/branchdeck-core/src && \
    echo 'pub mod models { pub mod workflow; pub mod run; } pub mod services { pub mod activity_store; pub mod event_bus; } pub mod util;' > crates/branchdeck-core/src/lib.rs && \
    mkdir -p crates/branchdeck-daemon/src && \
    echo 'fn main() {}' > crates/branchdeck-daemon/src/main.rs

# Pre-build dependencies (cached layer)
RUN cargo build --release --package branchdeck-daemon 2>/dev/null || true

# Copy actual source code
COPY crates/ crates/

# Build the real daemon binary
RUN cargo build --release --package branchdeck-daemon


# Stage 2: Build the SolidJS frontend
FROM oven/bun:1 AS frontend-builder

WORKDIR /build

# Copy package files for dependency caching
COPY package.json bun.lock ./
COPY sidecar/package.json sidecar/package.json

RUN bun install --frozen-lockfile

# Copy frontend source and build
COPY index.html tsconfig.json tsconfig.node.json vite.config.ts biome.json ./
COPY src/ src/
COPY public/ public/

RUN bun run build


# Stage 3: Runtime image
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js (for agent-bridge / sidecar) and Bun (package manager)
RUN curl -fsSL https://deb.nodesource.com/setup_22.x | bash - && \
    apt-get install -y --no-install-recommends nodejs unzip && \
    rm -rf /var/lib/apt/lists/* && \
    curl -fsSL https://bun.sh/install | bash && \
    ln -s /root/.bun/bin/bun /usr/local/bin/bun

# Install GitHub CLI
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
    | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
    > /etc/apt/sources.list.d/github-cli.list && \
    apt-get update && apt-get install -y --no-install-recommends gh && \
    rm -rf /var/lib/apt/lists/*

# Copy daemon binary
COPY --from=rust-builder /build/target/release/branchdeck-daemon /usr/local/bin/branchdeck-daemon

# Copy frontend static build
COPY --from=frontend-builder /build/dist /opt/branchdeck/dist

# Copy sidecar (agent-bridge)
COPY sidecar/ /opt/branchdeck/sidecar/
RUN cd /opt/branchdeck/sidecar && bun install --production

# Create non-root user
RUN groupadd -r branchdeck && useradd -r -g branchdeck -d /home/branchdeck -s /bin/bash branchdeck

# Create volume mount points
RUN mkdir -p /repos /config /home/branchdeck/.config/branchdeck && \
    chown -R branchdeck:branchdeck /repos /config /home/branchdeck

# Environment variables
ENV BRANCHDECK_PORT=13371
ENV BRANCHDECK_BIND=0.0.0.0
ENV BRANCHDECK_STATIC_DIR=/opt/branchdeck/dist
# Required at runtime (not set here — passed by user):
# ENV GITHUB_TOKEN=
# ENV ANTHROPIC_API_KEY=

EXPOSE 13371

VOLUME ["/repos", "/config"]

USER branchdeck

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:13371/api/health || exit 1

CMD ["branchdeck-daemon", "serve"]
