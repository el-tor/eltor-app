# =============================================================================
# Stage 1: Build Dependencies
# =============================================================================
FROM rust:1.82-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    build-essential \
    ca-certificates \
    curl \
    wget \
    git \
    autoconf \
    automake \
    libtool \
    zlib1g-dev \
    libevent-dev \
    libscrypt-dev \
    make \
    patch \
    flex \
    bison \
    unzip \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && npm install -g pnpm@9.9.0 \
    && rm -rf /var/lib/apt/lists/*

# =============================================================================
# Stage 2: Build Backend Dependencies
# =============================================================================
FROM builder AS backend-builder

WORKDIR /root/code

# Clone and build git dependencies
RUN git clone https://github.com/el-tor/eltor.git /root/code/eltor && \
    git clone https://github.com/lightning-node-interface/lni.git /root/code/lni && \
    git clone https://github.com/el-tor/libeltor-sys.git /root/code/libeltor-sys && \
    git clone https://github.com/el-tor/libeltor.git /root/code/libeltor && \
    git clone https://github.com/el-tor/eltord.git /root/code/eltord

# Checkout specific branches
# TODO change to master
RUN cd /root/code/eltord && git checkout relay-flows-2 && \
    cd /root/code/lni && git checkout search

# Build libeltor-sys
RUN cd /root/code/libeltor-sys && \
    ./scripts/copy.sh && \
    mkdir -p patches libtor-src/patches && \
    touch patches/.keep libtor-src/patches/.keep && \
    ./scripts/build.sh

# Build eltord
RUN cd /root/code/eltord && cargo build --release

# Copy eltor-app backend and build it
COPY backend /root/code/eltor-app/backend
WORKDIR /root/code/eltor-app/backend
RUN cargo build --release

# =============================================================================
# Stage 3: Build Frontend
# =============================================================================
FROM builder AS frontend-builder

WORKDIR /root/code/eltor-app/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install

# Copy frontend source and build
COPY frontend ./
RUN pnpm run build

# =============================================================================
# Stage 4: Download Phoenix
# =============================================================================
FROM debian:bookworm-slim AS phoenix-downloader

RUN apt-get update && apt-get install -y curl unzip && rm -rf /var/lib/apt/lists/*

# Download phoenixd
RUN ARCH=$(uname -m) && \
    if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then \
        PHOENIXD_URL="https://github.com/ACINQ/phoenixd/releases/download/v0.6.0/phoenixd-0.6.0-linux-arm64.zip"; \
    else \
        PHOENIXD_URL="https://github.com/ACINQ/phoenixd/releases/download/v0.6.0/phoenixd-0.6.0-linux-x64.zip"; \
    fi && \
    curl -L -o /tmp/phoenixd.zip "$PHOENIXD_URL" && \
    unzip /tmp/phoenixd.zip -d /tmp && \
    find /tmp -name "phoenixd" -type f -executable -exec cp {} /usr/local/bin/phoenixd \; && \
    find /tmp -name "phoenix-cli" -type f -executable -exec cp {} /usr/local/bin/phoenix-cli \; && \
    chmod +x /usr/local/bin/phoenixd /usr/local/bin/phoenix-cli

# =============================================================================
# Stage 5: Final Runtime Image
# =============================================================================
FROM debian:bookworm-slim AS runtime

# Install only runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    lsof \
    gettext-base \
    libssl3 \
    libsqlite3-0 \
    nano \
    python3-minimal \
    && rm -rf /var/lib/apt/lists/*

# Install multi-arch libraries for phoenixd if needed
RUN dpkg --add-architecture amd64 && \
    apt-get update && \
    apt-get install -y \
        libc6:amd64 \
        libcrypt1:amd64 \
        libgcc-s1:amd64 \
        libssl3:amd64 \
        zlib1g:amd64 \
        libstdc++6:amd64 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user and group
RUN groupadd -r eltor && useradd -r -g eltor -d /home/eltor -s /bin/bash eltor

# Create application structure with proper ownership
RUN mkdir -p /home/eltor/code/eltor-app/backend/bin \
             /home/eltor/data/logs \
             /home/eltor/data/tor \
             /home/eltor/data/phoenix \
             /home/eltor/.phoenix \
    && chown -R eltor:eltor /home/eltor

# Copy built binaries from previous stages
COPY --from=backend-builder /root/code/eltord/target/release/eltor /home/eltor/code/eltor-app/backend/bin/eltord
COPY --from=backend-builder /root/code/eltor-app/backend/target/release/eltor-backend /home/eltor/code/eltor-app/backend/bin/eltor-backend
COPY --from=phoenix-downloader /usr/local/bin/phoenixd /home/eltor/code/eltor-app/backend/bin/phoenixd
COPY --from=phoenix-downloader /usr/local/bin/phoenix-cli /home/eltor/code/eltor-app/backend/bin/phoenix-cli

# Copy frontend
COPY --from=frontend-builder /root/code/eltor-app/frontend/dist /home/eltor/code/eltor-app/frontend/dist

# Copy configuration files
COPY backend/bin/IP2LOCATION-LITE-DB3.BIN /home/eltor/code/eltor-app/backend/bin/
COPY backend/*.json /home/eltor/code/eltor-app/backend/
COPY backend/bin/torrc.template /home/eltor/code/eltor-app/backend/bin/
COPY backend/bin/torrc.relay.template /home/eltor/code/eltor-app/backend/bin/
COPY backend/run.sh /home/eltor/code/eltor-app/backend/
COPY scripts/start.sh /home/eltor/start.sh
COPY scripts/exports.sh /home/eltor/exports.sh

# Set ownership for all copied files
RUN chown -R eltor:eltor /home/eltor/code \
    && chown eltor:eltor /home/eltor/start.sh /home/eltor/exports.sh

# Set permissions
RUN chmod +x /home/eltor/code/eltor-app/backend/bin/eltord \
             /home/eltor/code/eltor-app/backend/bin/phoenixd \
             /home/eltor/code/eltor-app/backend/bin/phoenix-cli \
             /home/eltor/code/eltor-app/backend/bin/eltor-backend \
             /home/eltor/code/eltor-app/backend/run.sh \
             /home/eltor/start.sh \
             /home/eltor/exports.sh

# Switch to non-root user
USER eltor

# Set working directory
WORKDIR /home/eltor/code/eltor-app

# Expose ports
EXPOSE 5173 8080 9740 18058 9996

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV BACKEND_PORT=8080
ENV FRONTEND_PORT=5173

# Run the application
CMD ["/home/eltor/start.sh"]