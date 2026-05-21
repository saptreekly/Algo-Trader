# Stage 1: Builder
FROM rust:slim AS builder

# Install dependencies for building
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifest files first for caching
COPY Cargo.toml Cargo.lock ./
# Pre-build dependencies to cache them
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Copy source code
COPY src ./src
# Build application
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m trader
USER trader

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/algo-trader /app/algo-trader

CMD ["./algo-trader"]
