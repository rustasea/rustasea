# ExampleApp — Docker dev image (laravel/sail parity).
#
# Multi-stage cargo-chef build: `planner` records the dependency recipe,
# `builder` cooks it into a cached layer, then only the app source invalidates
# the final `cargo build`. The runtime stage is a slim Debian image running the
# `example-app` binary as a non-root user. All settings come from the
# environment at runtime — nothing is baked in.

# --- Chef base -------------------------------------------------------------
FROM lukemathwalker/cargo-chef:latest-rust-1.90-bookworm AS chef
WORKDIR /app

# --- Planner: capture the dependency recipe --------------------------------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# --- Builder: cook dependencies, then build the binary ---------------------
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin example-app

# --- Dev: hot-reloading stage for the compose dev override -----------------
FROM builder AS dev
RUN cargo install cargo-watch --locked
EXPOSE 8000
CMD ["cargo", "watch", "-x", "run"]

# --- Runtime: slim image with only what the binary reads -------------------
FROM debian:bookworm-slim AS runtime
# `libsqlite3-0` backs the sqlx sqlite driver (system-linked, not bundled) and
# `ca-certificates` lets rustls validate TLS roots.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

# Non-root runtime user (uid 1000) matching the compose bind-mount owner.
RUN useradd --create-home --uid 1000 appuser

WORKDIR /app
COPY --from=builder /app/target/release/example-app /usr/local/bin/example-app
# Runtime inputs the binary reads from the working directory.
COPY --from=builder /app/config ./config
COPY --from=builder /app/storage ./storage
RUN mkdir -p storage/logs storage/framework storage/app/public \
    && chown -R appuser:appuser /app

USER appuser
ENV APP_ENV=local
EXPOSE 8000
# Liveness probe: the app has no HTTP health route yet, so check the port.
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD bash -c 'exec 3<>/dev/tcp/127.0.0.1/8000' || exit 1

CMD ["example-app"]
