FROM rust:1.85-bookworm AS builder
WORKDIR /build
COPY apps/server/Cargo.toml apps/server/Cargo.lock* ./
COPY apps/server/src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN useradd --create-home --uid 10001 game && apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates wget \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/evil-hunter-server /usr/local/bin/evil-hunter-server
USER game
EXPOSE 8080
ENTRYPOINT ["evil-hunter-server"]
