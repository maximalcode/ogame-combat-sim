# syntax=docker/dockerfile:1

FROM rust:1.85-bookworm AS build
WORKDIR /src

# Keep dependency compilation cacheable while ensuring the binary is built
# entirely inside the container and never picks up host target output.
COPY Cargo.toml Cargo.lock rustfmt.toml deny.toml ./
COPY combat-api combat-api
COPY combat-core combat-core
COPY combat-types combat-types
COPY combat-fixtures combat-fixtures
COPY combat-cli combat-cli
COPY combat-ogame-api combat-ogame-api
# Keep panic locations useful without embedding the container build root in the
# shipped binary. This also makes runtime-content inspection independent of the
# builder's checkout path.
RUN RUSTFLAGS='--remap-path-prefix=/src=.' cargo build --release -p combat-api --locked

FROM debian:bookworm-slim AS runtime
RUN useradd --system --uid 10001 --create-home --home-dir /nonroot combat
COPY --from=build /src/target/release/combat-api /usr/local/bin/combat-api
USER 10001:10001
ENV PORT=3000 \
    MAX_SIMULATIONS=1000 \
    MAX_CONCURRENT_SIMULATIONS=1 \
    SHUTDOWN_GRACE_SECONDS=10
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/combat-api"]
