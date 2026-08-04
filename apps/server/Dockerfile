FROM rust:1.85-bookworm AS builder
WORKDIR /build
COPY apps/server/Cargo.toml apps/server/Cargo.lock* ./
COPY apps/server/src ./src
COPY packages/content/releases/evil-hunter-1.411/building-registry.json /packages/content/releases/evil-hunter-1.411/building-registry.json
COPY packages/content/releases/evil-hunter-1.411/ordinary-hunting-monster-map.json /packages/content/releases/evil-hunter-1.411/ordinary-hunting-monster-map.json
COPY packages/content/releases/evil-hunter-1.411/experience-runtime-catalog.json /packages/content/releases/evil-hunter-1.411/experience-runtime-catalog.json
COPY packages/content/releases/evil-hunter-1.411/gear-catalog.json /packages/content/releases/evil-hunter-1.411/gear-catalog.json
COPY packages/content/releases/evil-hunter-1.411/hunter-assets.json /packages/content/releases/evil-hunter-1.411/hunter-assets.json
COPY packages/content/releases/evil-hunter-1.411/monster-material-market-catalog.json /packages/content/releases/evil-hunter-1.411/monster-material-market-catalog.json
COPY packages/content/releases/evil-hunter-1.411/monster-runtime-catalog.json /packages/content/releases/evil-hunter-1.411/monster-runtime-catalog.json
COPY packages/content/releases/evil-hunter-rebuild-v1/weapon-core-catalog.json /packages/content/releases/evil-hunter-rebuild-v1/weapon-core-catalog.json
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN useradd --create-home --uid 10001 game && apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates wget \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/evil-hunter-server /usr/local/bin/evil-hunter-server
USER game
EXPOSE 8080
ENTRYPOINT ["evil-hunter-server"]
