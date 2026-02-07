FROM ghcr.io/astral-sh/uv:python3.12-bookworm-slim

# Install Node.js 20 for the WhatsApp bridge
RUN apt-get update && \
    apt-get install -y --no-install-recommends curl ca-certificates gnupg git && \
    mkdir -p /etc/apt/keyrings && \
    curl -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg && \
    echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_20.x nodistro main" > /etc/apt/sources.list.d/nodesource.list && \
    apt-get update && \
    apt-get install -y --no-install-recommends nodejs && \
    apt-get purge -y gnupg && \
    apt-get autoremove -y && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy files needed for dependency installation
COPY pyproject.toml README.md LICENSE ./
COPY rust/ rust/

# Install Rust toolchain for building Rust extension
RUN apt-get update && \
    apt-get install -y --no-install-recommends build-essential pkg-config libssl-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    . $HOME/.cargo/env

# Create placeholder Python package
RUN mkdir -p debot bridge && touch debot/__init__.py

# Install Python dependencies with Rust extension
ENV PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
RUN . $HOME/.cargo/env && \
    uv pip install --system --no-cache .

# Copy the full source code
COPY debot/ debot/
COPY bridge/ bridge/

# Build the WhatsApp bridge
WORKDIR /app/bridge
RUN npm install && npm run build
WORKDIR /app

# Create config directory
RUN mkdir -p /root/.debot

# Gateway default port
EXPOSE 18790

ENTRYPOINT ["debot"]
CMD ["status"]
