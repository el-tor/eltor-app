# =============================================================================
# Stage 1: Build Dependencies
# =============================================================================
FROM rustlang/rust:nightly-slim AS builder

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

# Clone and build lni dependency (needed for backend)
RUN git clone https://github.com/lightning-node-interface/lni.git /root/code/lni

# Copy eltor-app backend and build it
COPY backend /root/code/eltor-app/backend
WORKDIR /root/code/eltor-app/backend
RUN cargo build --release

# =============================================================================
# Stage 3: Download eltord and Arti
# =============================================================================
FROM builder AS eltord-downloader

WORKDIR /app

# Copy download scripts
COPY scripts/download-eltord.sh /app/scripts/download-eltord.sh
COPY scripts/download-arti.sh /app/scripts/download-arti.sh
RUN chmod +x /app/scripts/*.sh

# Create backend bin directory
RUN mkdir -p /app/backend/bin

# Download eltord binary (script places it in /app/backend/bin automatically)
RUN /app/scripts/download-eltord.sh

# Download and build Arti binary (script places it in /app/backend/bin automatically)
RUN /app/scripts/download-arti.sh

# Verify both binaries are present and executable
RUN ls -la /app/backend/bin/ && \
    /app/backend/bin/eltord --version && \
    /app/backend/bin/arti --version

# =============================================================================
# Stage 4: Download Phoenix
# =============================================================================
FROM builder AS phoenix-downloader

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
# Stage 4: Final Runtime Image
# =============================================================================
FROM rustlang/rust:nightly-slim AS runtime

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

# Create non-root user and group with UID 1000, and a shared data group
RUN groupadd -r -g 1000 user && \
    groupadd -r -g 2000 datagroup && \
    useradd -r -u 1000 -g 1000 -G datagroup -d /home/user -s /bin/bash user && \
    usermod -a -G datagroup root

# Create application structure with shared group ownership
RUN mkdir -p /home/user/code/eltor-app/backend/bin \
             /home/user/code/eltor-app/backend/bin/data \
             /home/user/code/eltor-app/data \
             /home/user/.eltor \
             /home/user/.tor \
             /home/user/data/logs \
             /home/user/data/tor/client \
             /home/user/data/tor-relay/client \
             /home/user/data/phoenix \
             /home/user/.phoenix \
    && chown -R user:datagroup /home/user \
    && chmod -R g+rwx /home/user/data 

# Copy built binaries from previous stages
COPY --from=eltord-downloader /app/backend/bin/eltord /home/user/code/eltor-app/backend/bin/eltord
COPY --from=eltord-downloader /app/backend/bin/arti /home/user/code/eltor-app/backend/bin/arti
COPY --from=backend-builder /root/code/eltor-app/backend/target/release/eltor-backend /home/user/code/eltor-app/backend/bin/eltor-backend
COPY --from=phoenix-downloader /usr/local/bin/phoenixd /home/user/code/eltor-app/backend/bin/phoenixd
COPY --from=phoenix-downloader /usr/local/bin/phoenix-cli /home/user/code/eltor-app/backend/bin/phoenix-cli

# Copy pre-built frontend
COPY frontend/dist /home/user/code/eltor-app/frontend/dist

# Copy configuration files
COPY backend/bin/IP2LOCATION-LITE-DB3.BIN /home/user/code/eltor-app/backend/bin/
COPY backend/*.json /home/user/code/eltor-app/backend/
COPY backend/bin/torrc.template /home/user/code/eltor-app/backend/bin/
COPY backend/bin/torrc.relay.template /home/user/code/eltor-app/backend/bin/
COPY backend/run.sh /home/user/code/eltor-app/backend/
COPY scripts/start.sh /home/user/start.sh

# Set ownership for all copied files (use shared group)
RUN chown -R user:datagroup /home/user/code \
    && chown user:datagroup /home/user/start.sh \
    && chmod -R g+rw /home/user/code

# Set permissions
RUN chmod +x /home/user/code/eltor-app/backend/bin/eltord \
             /home/user/code/eltor-app/backend/bin/arti \
             /home/user/code/eltor-app/backend/bin/phoenixd \
             /home/user/code/eltor-app/backend/bin/phoenix-cli \
             /home/user/code/eltor-app/backend/bin/eltor-backend \
             /home/user/code/eltor-app/backend/run.sh \
             /home/user/start.sh

# Switch to non-root user
USER user

# Set working directory
WORKDIR /home/user/code/eltor-app

# Expose ports (now using environment variables)
# Note: Frontend is now served by the backend, so only backend port is needed
EXPOSE 5174 9740 18058 18068 18057 18067 9996 9997 7781 18048 18049

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${BACKEND_PORT:-5174}/health || exit 1

# Set environment variables with defaults for Docker
ENV RUST_LOG=info
ENV BACKEND_PORT=5174
ENV BIND_ADDRESS=0.0.0.0
ENV BACKEND_URL=http://localhost:5174
# FRONTEND_PORT no longer needed as frontend is served by backend

# Run the application
CMD ["/home/user/start.sh"]