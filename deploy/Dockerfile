# Stage 1: Chef
FROM lukemathwalker/cargo-chef:latest-rust-slim-trixie AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin api --bin scraper

# Stage 2: Runtime
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binaries from builder
COPY --from=builder /app/target/release/api /usr/local/bin/gw2shinies-api
COPY --from=builder /app/target/release/scraper /usr/local/bin/gw2shinies-scraper

# Default environment variables
ENV SURREAL_DB_URI=localhost:8000

# The user can override the command to run either the api or the scraper
# Default to api
CMD ["gw2shinies-api"]
